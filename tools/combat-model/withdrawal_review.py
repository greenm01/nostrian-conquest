#!/usr/bin/env python3
"""t062: decision-1 baseline, isolated withdrawal controls, and repair bills.

Pursuit probability is a sensitivity input, not a movement simulation.
No preset is claimed to reproduce the engine until t060 is compared.
"""
import argparse
import json
import random
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import rules as R
from fleet_model import Model, force, _draw
from confidence import binomial_interval
from sweep import run_case


BASE = R.variant(R.CURRENT, 'decision1-control',
                 effect=dict(kind='flat', rows=R.EOTS_X4, unit=4), remainder='carry')


class PursuitModel(Model):
    def _leave(self, a, b, wd_a, wd_b, fought, exchange):
        branches = super()._leave(a, b, wd_a, wd_b, fought, exchange)
        probability = self.rules.get('pursuit_probability', 1.0)
        if self.rules['withdrawal'] != 'pursuit' or (wd_a and wd_b):
            return branches
        return ([(1 - probability, fought, ('terminal', a, b, wd_a, wd_b))]
                + [(p * probability, n, state) for p, n, state in branches])


def candidates():
    early = R.variant(BASE, 'after1-pressed', roe='after1')
    escape = R.variant(early, 'after1-unarmed-pressed', unarmed_escape=True)
    out = [BASE, early, escape]
    for probability in (0.0, 0.5, 1.0):
        for screen in ('absolute', 'conditional'):
            out.append(R.variant(escape, f'pursuit-{probability:g}-{screen}',
                                 withdrawal='pursuit', screen=screen,
                                 pursuit_probability=probability))
    return out


CASES = [
    ('10 CA ROE6 vs 15 CA', dict(CA=10), dict(CA=15), (6, 10), ''),
    ('5 CA 10 TT ROE6 vs 10 CA', dict(CA=5, TT=10), dict(CA=10), (6, 10), ''),
    ('10 TT ROE0 vs 3 DD', dict(TT=10), dict(DD=3), (0, 10), ''),
    ('1 CA 10 TT ROE6 vs 3 CA', dict(CA=1, TT=10), dict(CA=3), (6, 10), ''),
    ('5 CA 5 DD ROE6 vs 10 CA', dict(CA=5, DD=5), dict(CA=10), (6, 10), ''),
    ('10 CA ROE6 vs 10 CA ROE6', dict(CA=10), dict(CA=10), (6, 6), ''),
]


def finish(model, a, b, rng):
    """Return terminal hull state to measure repair and repeated contact."""
    r = 1
    exchange = lambda a, b, **kw: model.exchange_sampled(rng, a, b, **kw)
    while True:
        _, nxt = _draw(rng, [(p, (n, state)) for p, n, state in
                             model.step(a, b, r, (10, 10), exchange)])
        if nxt[0] == 'terminal':
            return nxt[1], nxt[2]
        _, a, b, r = nxt


def persistence_trials(trials, seed):
    model = Model(BASE, R.UNITS)
    totals = {k: 0.0 for k in ('first_loss', 'first_crippled', 'repair_bill',
                              'persistent_second_hulls', 'healed_second_hulls')}
    for i in range(trials):
        a, _ = finish(model, force(CA=15), force(CA=10), random.Random(f'{seed}/first/{i}'))
        crippled_cost = model.cost_of(a[1])
        totals['first_loss'] += 225 - model.cost_of(a[0])
        totals['first_crippled'] += crippled_cost
        totals['repair_bill'] += (crippled_cost + 3) // 4
        # Fresh encounter: reset carry, retain or erase completed crippling.
        hurt = (a[0], a[1], (0, 0))
        healed = (a[0], (0,) * len(a[1]), (0, 0))
        for key, state in [('persistent_second_hulls', hurt), ('healed_second_hulls', healed)]:
            survivor, _ = finish(model, state, force(CA=5), random.Random(f'{seed}/second/{i}'))
            totals[key] += sum(survivor[0])
    return {k: value / trials for k, value in totals.items()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--trials', type=int, default=10000)
    parser.add_argument('--seed', type=int, default=20261010)
    args = parser.parse_args()
    path = Path(__file__).with_name('nc_withdrawal_review.json')
    result = dict(seed=args.seed, trials=args.trials, exact_limit=1000,
                  rulesets={r['name']: r for r in candidates()}, units=R.UNITS, battles=[])
    for name, rules in result['rulesets'].items():
        model = PursuitModel(rules, R.UNITS)
        for case in CASES:
            row = run_case(model, 't062-common', 'baseline', case, args.trials, args.seed, 1000)
            row['ruleset'] = name
            if row['method'] == 'sampled':
                count = round(row['viable_left'] * args.trials)
                row['viable_left_ci'] = list(binomial_interval(count, args.trials)[:2])
            result['battles'].append(row)
            path.write_text(json.dumps(result, indent=2) + '\n')
            print(name, case[0], 'TT', round(row['surviving_left'], 3),
                  'half', round(row['viable_left'], 3), 'loss', round(row['loss_left'], 2), flush=True)
    result['persistence'] = persistence_trials(args.trials, args.seed)
    path.write_text(json.dumps(result, indent=2) + '\n')
    print('Persistence', result['persistence'], flush=True)


if __name__ == '__main__':
    main()
