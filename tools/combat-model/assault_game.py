"""Attack-and-defence game (t254): how should a world be defended, and how
attacked, for the same points?

Reads an `assault_odds --grid` JSON from the engine: every attack (a warship
mix plus loaded transports, and a plan: siege, bombard-then-blitz or blitz)
against every defence (batteries, armies, a starbase, a guard fleet) of a
world, for up to 8 years. It
builds the two-player zero-sum game, the attacker's payoff being the chance
the world falls, and solves it:

- the Nash equilibrium for each side by linear program, and the game's value;
- each side's mass by component, as a share of its points;
- a second table on the cost exchange, `(defence lost - attack lost) / defence
  budget`;
- a bootstrap over each pairing's trials: how often each strategy is in the
  equilibrium's support, and the value's spread.

Usage:
  uv run --with numpy --with scipy tools/combat-model/assault_game.py GRID.json [--out OUT.json] [--boot 200]
"""
import argparse
import json

import numpy as np

from metagame import SUPPORT, nash, self_check

COST = {'DD': 5, 'CA': 15, 'BB': 45, 'army': 7, 'battery': 20, 'garrison': 2, 'starbase': 50}


PLANS = ('siege', 'bombard-then-blitz', 'blitz')


def attack_key(c):
    return (c['destroyers'], c['cruisers'], c['battleships'], c['armies'], c['plan'])


def defence_key(c):
    return (c['batteries'], c['defenders'], c['starbase'], tuple(c['guard']))


def attack_name(k):
    return f'{k[0]}DD+{k[1]}CA+{k[2]}BB, {k[3]} armies, {k[4]}'


def defence_name(k):
    guard = '+'.join(f'{n}{h}' for n, h in zip(k[3], ('DD', 'CA', 'BB')) if n) or 'none'
    return f'{k[0]} bat, {k[1]} armies, SB{int(k[2])}, guard {guard}'


def attack_points(k):
    return np.array([k[0] * COST['DD'], k[1] * COST['CA'], k[2] * COST['BB'], k[3] * COST['army']], float)


def defence_points(k):
    g = k[3]
    return np.array([k[0] * COST['battery'], k[1] * COST['garrison'], k[2] * COST['starbase'],
                     g[0] * COST['DD'], g[1] * COST['CA'], g[2] * COST['BB']], float)


def mass(x, keys, points):
    p = np.array([points(k) for k in keys])
    return x @ (p / p.sum(1, keepdims=True))


def solve(m):
    """Attacker (row) maximin, defender (column) minimax, and the value."""
    x, v = nash(m)
    y, _ = nash(-m.T)
    return x, y, v


def check():
    self_check()
    # Matching pennies, unbalanced: the attacker plays row 0 a third of the time.
    x, y, v = solve(np.array([[2.0, 0.0], [0.0, 1.0]]))
    assert np.allclose(x, [1 / 3, 2 / 3]) and np.allclose(y, [1 / 3, 2 / 3]) and abs(v - 2 / 3) < 1e-9


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('grid')
    ap.add_argument('--out')
    ap.add_argument('--boot', type=int, default=200)
    ap.add_argument('--seed', type=int, default=1)
    args = ap.parse_args()
    check()

    cases = json.load(open(args.grid))
    attacks = sorted({attack_key(c) for c in cases})
    defences = sorted({defence_key(c) for c in cases})
    ai = {k: i for i, k in enumerate(attacks)}
    di = {k: j for j, k in enumerate(defences)}
    trials = np.zeros((len(attacks), len(defences)))
    captured = np.zeros_like(trials)
    exchange = np.zeros_like(trials)
    for c in cases:
        i, j = ai[attack_key(c)], di[defence_key(c)]
        trials[i, j] = c['trials']
        captured[i, j] = round(c['captured'] * c['trials'])
        exchange[i, j] = c['defence_points_lost'] - c['ship_points_lost'] - c['armies_lost'] * COST['garrison']
    assert (trials > 0).all(), 'every attack should meet every defence'
    budget_d = defence_points(defences[0]).sum()
    budget_a = attack_points(attacks[0]).sum()
    print(f'{args.grid}: {len(attacks)} attacks of about {budget_a:.0f} points against '
          f'{len(defences)} defences of {budget_d:.0f}, {int(trials[0, 0])} trials a pairing')

    report = {'grid': args.grid, 'attacks': [attack_name(k) for k in attacks],
              'defences': [defence_name(k) for k in defences]}
    for label, m in (('capture', captured / trials), ('exchange', exchange / budget_d)):
        x, y, v = solve(m)
        am = mass(x, attacks, attack_points)
        dm = mass(y, defences, defence_points)
        print(f'\n[{label}] value {v:+.3f}')
        pm = [sum(x[i] for i, k in enumerate(attacks) if k[4] == plan) for plan in PLANS]
        print('  attack mass: ' + ', '.join(f'{h} {p:.2f}' for h, p in zip(('DD', 'CA', 'BB', 'troops'), am))
              + '; by plan ' + ', '.join(f'{plan} {p:.2f}' for plan, p in zip(PLANS, pm)))
        for i in np.argsort(-x):
            if x[i] > SUPPORT:
                print(f'    {attack_name(attacks[i]):>32} {x[i]:.3f}')
        print('  defence mass: ' + ', '.join(f'{h} {p:.2f}' for h, p in zip(
            ('batteries', 'armies', 'starbase', 'guard DD', 'guard CA', 'guard BB'), dm)))
        for j in np.argsort(-y):
            if y[j] > SUPPORT:
                print(f'    {defence_name(defences[j]):>40} {y[j]:.3f}')
        report[label] = {'value': v, 'attack': x.round(4).tolist(), 'defence': y.round(4).tolist(),
                         'attack_mass': am.round(4).tolist(), 'plan_mass': [round(p, 4) for p in pm],
                         'defence_mass': dm.round(4).tolist(),
                         'matrix': m.round(4).tolist()}

    if args.boot:
        rng = np.random.default_rng(args.seed)
        values, sa, sd = [], np.zeros(len(attacks)), np.zeros(len(defences))
        p = captured / trials
        for _ in range(args.boot):
            x, y, v = solve(rng.binomial(trials.astype(int), p) / trials)
            values.append(v)
            sa += x > SUPPORT
            sd += y > SUPPORT
        lo, hi = np.percentile(values, [2.5, 97.5])
        print(f'\nbootstrap ({args.boot}x, capture table): value 95% {lo:.3f}-{hi:.3f}')
        for keys, support, name in ((attacks, sa, attack_name), (defences, sd, defence_name)):
            for i in np.argsort(-support):
                if support[i] / args.boot >= 0.1:
                    print(f'  {name(keys[i]):>40} in support {support[i] / args.boot:.0%}')
        report['bootstrap'] = {'replicates': args.boot, 'value_lo': lo, 'value_hi': hi,
                               'attack_support': (sa / args.boot).round(3).tolist(),
                               'defence_support': (sd / args.boot).round(3).tolist()}

    if args.out:
        json.dump(report, open(args.out, 'w'), indent=1)
        print(f'\nwrote {args.out}')


if __name__ == '__main__':
    main()
