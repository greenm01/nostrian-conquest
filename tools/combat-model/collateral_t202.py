"""assault_collateral.py's exact bombardment model under the t202 rules:
the t200 hulls, plain bombard AS (12 / 33 / 90, half crippled), two-step
batteries (66 hits to cripple, 66 more to destroy; 90 AS, 45 crippled), and
the ships' orbital fire and the batteries' return fire both on the
planetary table in full (quarters), hits rounded up. The
batteries shield the dock and the armies (the `delayed` order, as today).

Usage: python3 -I -B tools/combat-model/collateral_t202.py
"""
from collections import Counter
from functools import lru_cache
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import assault_collateral as AC  # noqa: E402
from fleet_model import Model, base_column, force  # noqa: E402
from rules import CRT_X4, RULESETS, load_units, variant  # noqa: E402

UNITS = load_units('t200')
AC.UNITS = UNITS
MODEL = Model(variant(RULESETS['engine-t060'], 'planetary-t202', remainder='discard', mixed_shift=0), UNITS)
DD, CA, BB = 0, 1, 2
BATTERY_DS = 66


def bombard_as(side):
    n, c, _ = side
    return sum((n[i] - c[i]) * full + c[i] * (full // 2) for i, full in ((DD, 12), (CA, 33), (BB, 90)))


def battery_as(batteries):
    healthy, crippled = batteries
    return 90 * healthy + 45 * crippled


def shot(attack, defense, roll, divisor=4):
    if not attack:
        return 0, False
    return -(-attack * CRT_X4[roll][base_column(attack, defense)] // divisor), roll == 9


def battery_hits(batteries, hits):
    healthy, crippled = batteries
    k = min(healthy, hits // BATTERY_DS)
    healthy -= k
    crippled += k
    hits -= k * BATTERY_DS
    if healthy == 0:
        destroyed = min(crippled, hits // BATTERY_DS)
        crippled -= destroyed
        hits -= destroyed * BATTERY_DS
    return (healthy, crippled), hits


def damage(planet, hits, round_):
    batteries, dock, armies, goods, production = planet
    batteries, hits = battery_hits(batteries, hits)
    if sum(batteries):
        return batteries, dock, armies, goods, production
    dock, hits = AC.dock_damage(dock, hits)
    loss = min(armies, hits // 10)
    armies -= loss
    hits -= 10 * loss
    production = max(0, production - hits // 10)
    goods = max(0, goods - 100 * (hits // 10))
    return batteries, dock, armies, goods, production


def solve(start, planet):
    @lru_cache(None)
    def battle(side, world, r):
        if r > 3 or bombard_as(side) == 0:
            return (MODEL.cost_of(side[0]), sum(world[0]), sum(UNITS[c]['cost'] * n for c, n in world[1]),
                    world[2], world[3], world[4], float(any(side[0])), world[0][1])
        ours, theirs = bombard_as(side), battery_as(world[0])
        shots = Counter(shot(ours, max(1, theirs), die) for die in range(10))
        returns = Counter(shot(theirs, max(1, ours), die) for die in range(10))
        result = [0.] * 8
        for (hits, crit), ka in shots.items():
            after = damage(world, hits + crit, r)
            for (back, critical), kd in returns.items():
                surviving = MODEL.apply(side, 0, back, critical)[0][1]
                row = battle(surviving, after, r + 1)
                result = [x + ka * kd * y / 100 for x, y in zip(result, row)]
        return tuple(result)

    result = battle(start, planet, 1)
    initial = (MODEL.cost_of(start[0]), sum(planet[0]), sum(UNITS[c]['cost'] * n for c, n in planet[1]),
               planet[2], planet[3], planet[4])
    names = ('ship_cost_lost', 'batteries_lost', 'dock_cost_lost', 'armies_lost', 'goods_lost', 'production_lost')
    return dict(zip(names, (a - b for a, b in zip(initial, result))), survival=result[6],
                batteries_crippled=result[7])


if __name__ == '__main__':
    for name, counts, planet in [('BB4-B2', dict(BB=4), ((2, 0), (('BB', 1),), 10, 500, 100)),
                                 ('CA6-vs-B3-dockedBB', dict(CA=6), ((3, 0), (('BB', 1),), 10, 500, 100)),
                                 ('CA1-no-defense', dict(CA=1), ((0, 0), (), 0, 5000, 100)),
                                 ('BB1-no-defense', dict(BB=1), ((0, 0), (), 0, 5000, 100))]:
        row = solve(force(**counts), planet)
        print(name, {k: round(v, 3) for k, v in row.items()})
