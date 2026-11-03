"""Fleet-mix meta-game (t253): which equal-cost DD/CA/BB mixes win?

Reads a `combat_odds --grid` JSON from the engine (every ordered pair of
mixes at one budget), builds the symmetric zero-sum game between mixes,
and solves it:

- payoff `M[i][j] = P(i wins) - P(j wins)`, both seatings pooled, a trial
  without a battle counting as a draw; a second table uses the cost
  exchange, `(cost j lost - cost i lost) / budget`;
- the Nash equilibrium by linear program, and its mass by hull (point share);
- mixes that beat every other mix (dominant), and the best responses to one
  mix (default: the AI's DD/CA mix nearest half-and-half by points);
- a bootstrap over each pair's trials: how often each mix is in the
  equilibrium's support, and the spread of the hull masses;
- optionally alpha-Rank (open_spiel) over the win table.

Usage:
  uv run --with numpy --with scipy tools/combat-model/metagame.py GRID.json [--out OUT.json]
      [--boot 200] [--versus 18DD+6CA+0BB] [--alpharank]
  (add `--with open_spiel --with matplotlib --with networkx` for --alpharank)
"""
import argparse
import json

import numpy as np
from scipy.optimize import linprog

COST = np.array([5, 15, 45])  # DD, CA, BB
HULLS = ('DD', 'CA', 'BB')
SUPPORT = 1e-3


def nash(m):
    """Maximin strategy of the row player of zero-sum matrix `m`, and the value."""
    n, k = m.shape
    # Variables x_1..x_n, v; maximise v subject to x^T m[:, j] >= v for every j.
    c = np.zeros(n + 1)
    c[-1] = -1
    a_ub = np.hstack([-m.T, np.ones((k, 1))])
    a_eq = np.hstack([np.ones((1, n)), np.zeros((1, 1))])
    bounds = [(0, None)] * n + [(None, None)]
    res = linprog(c, A_ub=a_ub, b_ub=np.zeros(k), A_eq=a_eq, b_eq=[1], bounds=bounds, method='highs')
    assert res.success, res.message
    x = np.clip(res.x[:n], 0, None)
    return x / x.sum(), res.x[-1]


def self_check():
    rps = np.array([[0, -1, 1], [1, 0, -1], [-1, 1, 0]], float)
    x, v = nash(rps)
    assert np.allclose(x, 1 / 3, atol=1e-6) and abs(v) < 1e-9, (x, v)


def load(path):
    data = json.load(open(path))
    cases = data['cases']
    mixes = sorted({tuple(c['left_ships']) for c in cases}, key=lambda s: (s[2], s[1], s[0]))
    index = {s: i for i, s in enumerate(mixes)}
    n = len(mixes)
    # counts[i][j] = (i wins, j wins, mutual, stalemate, no battle, i cost lost, j cost lost)
    counts = np.zeros((n, n, 7))
    for c in cases:
        i, j = index[tuple(c['left_ships'])], index[tuple(c['right_ships'])]
        k = c['counts']
        b = c['battles']
        counts[i, j] += (k['left'], k['right'], k['mutual'], k['stalemate'], c['trials'] - b,
                         c['loss_left'] * b, c['loss_right'] * b)
        counts[j, i] += (k['right'], k['left'], k['mutual'], k['stalemate'], c['trials'] - b,
                         c['loss_right'] * b, c['loss_left'] * b)
    budget = int(np.dot(mixes[0], COST))
    return mixes, counts, budget, data


def tables(counts, budget):
    trials = counts[..., :5].sum(-1)
    win = (counts[..., 0] - counts[..., 1]) / trials
    exchange = (counts[..., 6] - counts[..., 5]) / trials / budget
    np.fill_diagonal(win, 0)
    np.fill_diagonal(exchange, 0)
    return win, exchange


def resample(counts, rng):
    """One bootstrap replicate: redraw each pair's outcomes, keeping its mean losses."""
    out = counts.copy()
    n = counts.shape[0]
    for i in range(n):
        for j in range(n):
            k = counts[i, j, :5]
            total = int(k.sum())
            out[i, j, :5] = rng.multinomial(total, k / total)
            battles = total - k[4]
            new_battles = total - out[i, j, 4]
            if battles:
                out[i, j, 5:] = counts[i, j, 5:] / battles * new_battles
    return out


def hull_mass(x, mixes):
    points = np.array([np.array(s) * COST for s in mixes], float)
    shares = points / points.sum(1, keepdims=True)
    return x @ shares


def name(s):
    return f'{s[0]}DD+{s[1]}CA+{s[2]}BB'


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('grid')
    ap.add_argument('--out')
    ap.add_argument('--boot', type=int, default=200)
    ap.add_argument('--seed', type=int, default=1)
    ap.add_argument('--versus')
    ap.add_argument('--alpharank', action='store_true')
    args = ap.parse_args()
    self_check()

    mixes, counts, budget, data = load(args.grid)
    win, exchange = tables(counts, budget)
    roe = data['cases'][0].get('roe')
    print(f'{args.grid}: {len(mixes)} mixes at {budget} points, ROE {roe}, '
          f'{int(counts[0, 1, :5].sum())} trials a pairing (both seatings)')

    # Seat check: the left seat's margin, over mirror matches and over all pairs.
    margin = lambda c: (c['counts']['left'] - c['counts']['right']) / c['trials']
    mirror = [margin(c) for c in data['cases'] if c['left_ships'] == c['right_ships']]
    print(f'left-seat margin: mirrors mean {np.mean(mirror):+.3f} (worst {max(mirror, key=abs):+.3f}), '
          f'all pairs mean {np.mean([margin(c) for c in data["cases"]]):+.3f}')

    report = {'grid': args.grid, 'budget': budget, 'roe': roe, 'mixes': [name(s) for s in mixes]}
    for label, m in (('win', win), ('exchange', exchange)):
        x, v = nash(m)
        mass = hull_mass(x, mixes)
        print(f'\n[{label}] Nash value {v:+.4f}; hull mass ' +
              ', '.join(f'{h} {p:.2f}' for h, p in zip(HULLS, mass)))
        for i in np.argsort(-x):
            if x[i] > SUPPORT:
                print(f'  {name(mixes[i]):>16} {x[i]:.3f}')
        dominant = [name(mixes[i]) for i in range(len(mixes))
                    if all(m[i, j] > 0 for j in range(len(mixes)) if j != i)]
        print(f'  beats every other mix: {", ".join(dominant) or "none"}')
        report[label] = {'nash': x.round(4).tolist(), 'value': v, 'hull_mass': mass.round(4).tolist(),
                         'dominant': dominant, 'matrix': m.round(4).tolist()}

    # Best responses to one mix.
    if args.versus:
        target = mixes.index(tuple(int(p[:-2]) for p in args.versus.split('+')))
    else:
        target = min(range(len(mixes)), key=lambda i: (mixes[i][2] > 0,
                     abs(mixes[i][0] * 5 - mixes[i][1] * 15)))
    order = np.argsort(-win[:, target])
    print(f'\nbest responses to {name(mixes[target])} (win margin):')
    for i in order[:5]:
        print(f'  {name(mixes[i]):>16} {win[i, target]:+.3f}')
    report['versus'] = {'mix': name(mixes[target]),
                        'best': [[name(mixes[i]), round(win[i, target], 4)] for i in order[:5]]}

    if args.boot:
        rng = np.random.default_rng(args.seed)
        support = np.zeros(len(mixes))
        masses = []
        for _ in range(args.boot):
            w, _ = tables(resample(counts, rng), budget)
            x, _ = nash(w)
            support += x > SUPPORT
            masses.append(hull_mass(x, mixes))
        masses = np.array(masses)
        lo, hi = np.percentile(masses, [2.5, 97.5], axis=0)
        print(f'\nbootstrap ({args.boot}x, win table): hull mass 95% ' +
              ', '.join(f'{h} {a:.2f}-{b:.2f}' for h, a, b in zip(HULLS, lo, hi)))
        for i in np.argsort(-support):
            if support[i] / args.boot >= 0.1:
                print(f'  {name(mixes[i]):>16} in support {support[i] / args.boot:.0%}')
        report['bootstrap'] = {'replicates': args.boot, 'support': (support / args.boot).round(3).tolist(),
                               'hull_mass_lo': lo.round(4).tolist(), 'hull_mass_hi': hi.round(4).tolist()}

    if args.alpharank:
        from open_spiel.python.egt import alpharank
        # Single population; payoff is the win share in [0, 1].
        pi = alpharank.compute([(1 + win) / 2], alpha=1e2, m=50, use_inf_alpha=True, verbose=False)[2]
        print('\nalpha-Rank (infinite alpha):')
        for i in np.argsort(-pi)[:8]:
            if pi[i] > SUPPORT:
                print(f'  {name(mixes[i]):>16} {pi[i]:.3f}')
        report['alpharank'] = np.asarray(pi).round(4).tolist()

    if args.out:
        json.dump(report, open(args.out, 'w'), indent=1)
        print(f'\nwrote {args.out}')


if __name__ == '__main__':
    main()
