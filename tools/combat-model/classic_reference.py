"""Reproduce the formulas and comparisons in the completed-reference review.

Exact ground enumeration uses floating-point probabilities and ideal uniform
draws. Fleet samples reuse the retained classic model, including its bugs.
Neither calculation runs ECMAINT or the Rust engine.
"""
import argparse
from functools import lru_cache
import json
import math
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'classic-model'))
import fsim
from sim import R, STATS


def shot_damage(max_damage, hit_chance, armor):
    return hit_chance * sum(max(0, damage - armor)
                            for damage in range(1, max_damage + 1)) / max_damage


def engagement_multiplier(ratio):
    y = ratio ** (1 / 3.3)
    return min(2., (y + 1.3) / 2 if y > 1.3 else y)


def sine_intercept(speed_ratio):
    if speed_ratio <= 1:
        return 2 * speed_ratio / math.pi
    angle = math.asin(1 / speed_ratio)
    return 1 - 2 * angle / math.pi + 2 * speed_ratio * (1 - math.cos(angle)) / math.pi


@lru_cache(None)
def binomial(n, p):
    return tuple(math.comb(n, k) * p**k * (1-p)**(n-k) for k in range(n+1))


@lru_cache(None)
def ground_fixed(a, d, defender_percent):
    """Solve with one fixed defender probability for this entire battle."""
    if a == 0:
        return 0., 0.
    if d == 0:
        return 1., 0.
    pd = defender_percent / 100
    unchanged = (1-pd)**d * .7**a
    win, rounds = 0., 1.
    for killed_a, p_defender in enumerate(binomial(d, pd)):
        remaining_a = a - killed_a
        if remaining_a <= 0:
            continue
        for killed_d, p_attacker in enumerate(binomial(remaining_a, .3)):
            if killed_a == killed_d == 0:
                continue
            remaining_d = max(0, d-killed_d)
            w, r = ground_fixed(remaining_a, remaining_d, defender_percent)
            probability = p_defender * p_attacker
            win += probability * w
            rounds += probability * r
    return win / (1-unchanged), rounds / (1-unchanged)


def ground(a, d):
    values = [ground_fixed(a, d, p) for p in range(40, 50)]
    return dict(capture=sum(v[0] for v in values)/10,
                rounds=sum(v[1] for v in values)/10)


def fleet_samples(trials, seed):
    cases = [
        ('10 CA vs 10 CA', dict(ca=10), dict(ca=10), False),
        ('15 CA vs 10 CA', dict(ca=15), dict(ca=10), False),
        ('30 CA vs 10 CA', dict(ca=30), dict(ca=10), False),
        ('30 DD vs 10 CA', dict(dd=30), dict(ca=10), False),
        ('90 DD vs 10 BB', dict(dd=90), dict(bb=10), False),
        ('30 CA vs 10 BB', dict(ca=30), dict(bb=10), False),
        ('10 DD vs 1 base', dict(dd=10), {}, True),
    ]
    costs = dict(dd=5, ca=15, bb=45, sb=50)
    speeds = dict(dd=6, ca=5, bb=4)
    rows = []
    for name, a, b, base in cases:
        a = dict(a, roe=10, maxsp=min(speeds[k] for k in a))
        b = dict(b, roe=10, maxsp=min((speeds[k] for k in b), default=1))
        R.seed(seed)
        counts = dict(a_win=0, b_win=0, mutual=0, unresolved=0)
        rounds = losses_a = losses_b = 0
        start_a = sum(a.get(k, 0)*v for k, v in costs.items())
        start_b = sum(b.get(k, 0)*v for k, v in costs.items()) + 50*base
        for _ in range(trials):
            result = fsim.battle(a, b, base=base)
            alive_a = any(result[1][k] for k in costs)
            alive_b = any(result[2][k] for k in costs)
            key = ('unresolved' if alive_a and alive_b else
                   'a_win' if alive_a else 'b_win' if alive_b else 'mutual')
            counts[key] += 1
            rounds += result['rounds']
            losses_a += start_a - sum(result[1][k]*v for k, v in costs.items())
            losses_b += start_b - sum(result[2][k]*v for k, v in costs.items())
        rows.append(dict(case=name, trials=trials, seed=seed, outcomes=counts,
                         mean_rounds=rounds/trials,
                         mean_cost_lost_a=losses_a/trials,
                         mean_cost_lost_b=losses_b/trials))
    return rows


def results(trials, seed):
    return dict(
        method='Formula enumeration; fleet Monte Carlo uses retained fsim.py, ROE10, fresh whole hulls.',
        reference_commit='4825ba5e', engine_validated=False,
        expected_shot_damage=[dict(unit=name, build_cost=cost,
                                  damage_by_armor=[shot_damage(STATS[t][2], STATS[t][5]/100, ar)
                                                   for ar in range(4)])
                              for name, t, cost in [('DD', 1, 5), ('CA', 2, 15),
                                                    ('BB', 3, 45), ('base', 8, 50),
                                                    ('battery', 11, 20)]],
        fleet_engagement=[dict(ratio=r, approximate_firing_value_ratio=engagement_multiplier(r))
                          for r in (1, 1.5, 2, 3, 5, 10, 30)],
        sine_intercept=[dict(speed_ratio=r, probability=sine_intercept(r)) for r in (1, 2, 6)],
        ground=[dict(attackers=a, defenders=d, **ground(a, d))
                for a, d in [(1, 1), (5, 5), (8, 10), (10, 10), (11, 10),
                             (12, 10), (15, 10), (20, 10), (20, 20), (30, 20), (40, 20)]],
        fleet_samples=fleet_samples(trials, seed) if trials else [])


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--fleet-trials', type=int, default=2000)
    parser.add_argument('--seed', type=int, default=20261010)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    if args.fleet_trials < 0:
        parser.error('--fleet-trials must be nonnegative')
    payload = json.dumps(results(args.fleet_trials, args.seed), indent=2) + '\n'
    if args.output is None:
        print(payload, end='')
    else:
        args.output.write_text(payload)
        print(f'Wrote {args.output}')
