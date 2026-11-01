"""collateral_t202.py's exact bombardment model with the t234 leak and the
t236 limits: in a round that starts with a battery standing,
`round(hits * share)` extra hits land behind the batteries, with
`share = (0.10 + 0.30 * (P - 10) / 140) * (roll + 1) / 5.5`, split 2/5 dock,
1/10 armies and the rest industry; fire at armies and industry strikes at
half strength; and the year's rounds end once no battery stands. With t237
production never falls below 1.0, and with t238 the main fire that passes
the dock splits a sixth to the armies and the rest to industry.

Usage: python3 -I -B tools/combat-model/collateral_t236.py
"""
from functools import lru_cache
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import assault_collateral as AC  # noqa: E402
import collateral_t202 as T  # noqa: E402
from fleet_model import force  # noqa: E402

POTENTIAL = 100  # the combat tests' target world


def hit_armies(armies, hits):
    loss = min(armies, hits // 10)
    return armies - loss, hits - 10 * loss


def hit_industry(goods, production, hits):
    points = hits // 10
    return max(0, goods - 25 * points), max(min(production, 1), production - points)


def leak(hits, roll):
    potential = min(150, max(10, POTENTIAL))
    share = (0.10 + 0.30 * (potential - 10) / 140) * (roll + 1) / 5.5
    return math.floor(hits * share + 0.5)


def damage(planet, hits, roll):
    batteries, dock, armies, goods, production = planet
    extra = leak(hits, roll) if sum(batteries) else 0
    batteries, hits = T.battery_hits(batteries, hits)
    to_dock, to_armies = extra * 2 // 5, extra // 10
    dock, rest = AC.dock_damage(dock, to_dock)
    armies, rest = hit_armies(armies, (to_armies + rest) // 2)
    goods, production = hit_industry(goods, production, (extra - to_dock - to_armies) // 2 + rest)
    if sum(batteries):
        return batteries, dock, armies, goods, production
    dock, hits = AC.dock_damage(dock, hits)
    hits //= 2
    armies, rest = hit_armies(armies, hits // 6)
    goods, production = hit_industry(goods, production, hits - hits // 6 + rest)
    return batteries, dock, armies, goods, production


def solve(start, planet):
    @lru_cache(None)
    def battle(side, world, r):
        if r > 3 or T.bombard_as(side) == 0:
            return (T.MODEL.cost_of(side[0]), sum(world[0]), sum(T.UNITS[c]['cost'] * n for c, n in world[1]),
                    world[2], world[3], world[4])
        ours, theirs = T.bombard_as(side), T.battery_as(world[0])
        result = [0.] * 6
        for die in range(10):
            hits, crit = T.shot(ours, max(1, theirs), die)
            after = damage(world, hits + crit, die)
            if theirs:
                returns = [T.shot(theirs, max(1, ours), d) for d in range(10)]
            else:
                returns = [(0, False)] * 10
            for back, critical in returns:
                surviving = T.MODEL.apply(side, 0, back, critical)[0][1] if back or critical else side
                row = battle(surviving, after, 4 if sum(after[0]) == 0 else r + 1)
                result = [x + y / 100 for x, y in zip(result, row)]
        return tuple(result)

    result = battle(start, planet, 1)
    initial = (T.MODEL.cost_of(start[0]), sum(planet[0]), sum(T.UNITS[c]['cost'] * n for c, n in planet[1]),
               planet[2], planet[3], planet[4])
    names = ('ship_cost_lost', 'batteries_lost', 'dock_cost_lost', 'armies_lost', 'goods_lost', 'production_lost')
    return dict(zip(names, (a - b for a, b in zip(initial, result))))


if __name__ == '__main__':
    for name, counts, planet in [('BB4-B2', dict(BB=4), ((2, 0), (('BB', 1),), 10, 500, 100)),
                                 ('CA1-no-defense', dict(CA=1), ((0, 0), (), 0, 5000, 100)),
                                 ('BB1-no-defense', dict(BB=1), ((0, 0), (), 0, 5000, 100))]:
        row = solve(force(**counts), planet)
        print(name, {k: round(v, 3) for k, v in row.items()})
