"""t255 experiment: search combat settings for a fleet-mix game with counters.

Each candidate runs `combat_odds --grid` (branch `t255-combat-search`, whose
`--tuning` sets the engine's combat settings for that process) at ROE 10 and
ROE 6, and scores the two mix games with `metagame.py`:

- counter: how far even the best single mix can be beaten by its worst
  matchup, counting a matchup only past two standard errors,
  `-max_i min_j (M[i][j] + 2 se_ij)` (positive means every mix has a real
  counter), capped at COUNTER_CAP so the search doesn't chase pure guessing
  games;
- diversity: the smallest hull's share of points in the Nash equilibrium;
- a small penalty for moving hull AS/DS away from today's values.

Usage (Optuna study kept in SQLite, so a run resumes):
  uv run --with numpy --with scipy --with optuna tools/combat-model/counter_search.py \
      --bin rust/target/release/examples/combat_odds --dir DIR --trials 300
"""
import argparse
import json
import math
import os
import subprocess
import sys
import tempfile

import numpy as np
import optuna

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import metagame as G  # noqa: E402

BASE_AS = (12, 33, 90)
BASE_DS = (7, 30, 110)
HULLS = ('DD', 'CA', 'BB')
COUNTER_CAP = 0.25


def tuning_string(p):
    attack = '/'.join(str(round(b * p[f'as_{h}'])) for b, h in zip(BASE_AS, HULLS))
    defense = '/'.join(str(round(b * p[f'ds_{h}'])) for b, h in zip(BASE_DS, HULLS))
    first = '/'.join(p[f'first_{h}'] for h in HULLS)
    bonus = '/'.join(f"{p[f'bonus_{h}']:.3f}" for h in HULLS)
    return f'per_hull=1,as={attack},ds={defense},exp={p["exp"]:.3f},first={first},bonus={bonus}'


def run_grid(binary, tuning, roe, trials, budget, step):
    with tempfile.NamedTemporaryFile(suffix='.json', delete=False) as f:
        path = f.name
    try:
        subprocess.run([binary, '--grid', str(budget), '--step', str(step), '--trials', str(trials),
                        '--roe', str(roe), '--tuning', tuning, '--json', path],
                       check=True, stdout=subprocess.DEVNULL)
        mixes, counts, budget, _ = G.load(path)
    finally:
        os.unlink(path)
    win, _ = G.tables(counts, budget)
    x, _ = G.nash(win)
    n = counts[..., :5].sum(-1)
    p_win, p_loss = counts[..., 0] / n, counts[..., 1] / n
    se = np.sqrt(np.maximum(p_win + p_loss - (p_win - p_loss) ** 2, 1e-9) / n)
    off = win + 2 * se + np.diag(np.full(len(mixes), np.inf))
    counter = -float(np.max(off.min(axis=1)))
    mass = G.hull_mass(x, mixes)
    support = [[G.name(mixes[i]), round(float(x[i]), 3)] for i in np.argsort(-x) if x[i] > G.SUPPORT]
    return counter, float(mass.min()), mass.round(3).tolist(), support


def score(results, params):
    parts = [min(c, COUNTER_CAP) + 0.5 * min(d, 0.2) for c, d, *_ in results]
    drift = sum(abs(math.log(params[f'{k}_{h}'])) for k in ('as', 'ds') for h in HULLS)
    return float(np.mean(parts)) - 0.05 * drift


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('--bin', required=True)
    ap.add_argument('--dir', required=True)
    ap.add_argument('--trials', type=int, default=300)
    ap.add_argument('--grid-trials', type=int, default=400)
    ap.add_argument('--budget', type=int, default=450)
    ap.add_argument('--step', type=int, default=90)
    args = ap.parse_args()
    os.makedirs(args.dir, exist_ok=True)

    def objective(trial):
        p = {}
        for h in HULLS:
            p[f'first_{h}'] = trial.suggest_categorical(f'first_{h}', ['-', 'DD', 'CA', 'BB'])
            p[f'bonus_{h}'] = trial.suggest_float(f'bonus_{h}', 1.0, 3.0)
            p[f'as_{h}'] = trial.suggest_float(f'as_{h}', 0.8, 1.25, log=True)
            p[f'ds_{h}'] = trial.suggest_float(f'ds_{h}', 0.8, 1.25, log=True)
        p['exp'] = trial.suggest_float('exp', 2.5, 4.5)
        tuning = tuning_string(p)
        results = [run_grid(args.bin, tuning, roe, args.grid_trials, args.budget, args.step) for roe in (10, 6)]
        for roe, (counter, diversity, mass, support) in zip((10, 6), results):
            trial.set_user_attr(f'roe{roe}', {'counter': counter, 'diversity': diversity,
                                               'hull_mass': mass, 'support': support[:6]})
        trial.set_user_attr('tuning', tuning)
        return score(results, p)

    study = optuna.create_study(study_name='t255', direction='maximize',
                                storage=f'sqlite:///{args.dir}/study.db', load_if_exists=True,
                                sampler=optuna.samplers.TPESampler(seed=1, multivariate=True))
    if not study.trials:
        # Today's rules (per-hull fire, no bonus) as the first point.
        base = {f'{k}_{h}': v for h in HULLS for k, v in (('first', '-'), ('bonus', 1.0), ('as', 1.0), ('ds', 1.0))}
        study.enqueue_trial({**base, 'exp': 3.3})
    study.optimize(objective, n_trials=args.trials - len(study.trials))

    best = sorted((t for t in study.trials if t.value is not None), key=lambda t: -t.value)[:10]
    summary = [{'value': t.value, 'number': t.number, **t.user_attrs} for t in best]
    json.dump(summary, open(f'{args.dir}/best.json', 'w'), indent=1)
    for t in best[:5]:
        print(f'{t.number:4} {t.value:+.3f} {t.user_attrs["tuning"]}')
        for roe in (10, 6):
            a = t.user_attrs[f'roe{roe}']
            print(f'       ROE {roe}: counter {a["counter"]:+.3f} hull mass {a["hull_mass"]} support {a["support"][:3]}')
    try:
        print('importance:', optuna.importance.get_param_importances(study))
    except Exception as error:  # needs enough finished trials
        print('importance unavailable:', error)


if __name__ == '__main__':
    main()
