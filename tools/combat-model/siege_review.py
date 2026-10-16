"""Sieges of ground batteries under the proposed t202 rules, against today's.

Usage: python3 -I -B tools/combat-model/siege_review.py 4000 out.json

Engine formulas (rust/nc-engine/src/maint/combat/exchange.rs and assault.rs):
the planetary ratio CRT, hits rounded up, simultaneous exchanges, three a
year; battery hits are applied before return fire is removed from play;
battery return fire is focus fire (finish a crippled ship for its DS, destroy
a healthy one for twice its DS, leftovers lost); a natural 9 is a critical.
Ships are repaired between years. Today's rules: one-step batteries (DS 20),
bombard weights 6/33/135, the CRT read in quarters. The t202 rules:
two-step batteries (66 to cripple, 66 more to destroy; fire 90 healthy, 45
crippled), plain bombard AS (12/33/90), the ships' fire on the CRT read in twelfths (a
third of today's) and the batteries' return fire on it in full, every
bombarding fleet of an empire at the world together, and
crippled batteries kept while the world is bombarded each year. Hits below
the next battery's cost are lost, as today (decision lzvpk22s).
"""
import json
import random
import sys

CRT_X4 = [
    [0, 1, 2, 3, 4], [1, 2, 3, 4, 5], [1, 2, 4, 5, 6], [2, 3, 4, 5, 6], [2, 3, 4, 6, 7],
    [2, 4, 5, 6, 7], [3, 4, 5, 6, 8], [3, 4, 6, 7, 8], [4, 5, 6, 7, 8], [4, 6, 7, 8, 10],
]
BATTERY_AS = 90
BB = dict(cost=45, ds=110)
RULES = {
    'today': dict(divisor=4, back_divisor=4, steps=1, battery_ds=20, bombard_as=135, keep_crippled=False),
    't202': dict(divisor=12, back_divisor=4, steps=2, battery_ds=66, bombard_as=90, keep_crippled=True),
}


def column(ours, theirs):
    if theirs == 0 or ours >= 3 * theirs:
        return 4
    if 2 * ours < theirs:
        return 0
    if ours < theirs:
        return 1
    if 2 * ours < 3 * theirs:
        return 2
    return 3


def hits(ours, theirs, roll, divisor):
    return -(-ours * CRT_X4[roll][column(ours, theirs)] // divisor)


def year(rules, rng, ships, batteries):
    """One year of bombardment. ships = [healthy, crippled] battleships,
    batteries = [healthy, crippled]."""
    healthy, crippled = ships
    bh, bc = batteries
    for _ in range(3):
        if bh + bc == 0:
            break
        ours = rules['bombard_as'] * healthy + rules['bombard_as'] // 2 * crippled
        if ours == 0:
            break
        theirs = BATTERY_AS * bh + BATTERY_AS // 2 * bc
        roll = rng.randrange(10)
        out = hits(ours, theirs, roll, rules['divisor']) + (roll == 9)
        roll = rng.randrange(10)
        back = hits(theirs, ours, roll, rules['back_divisor'])
        k = min(crippled, back // BB['ds'])
        crippled -= k
        back -= k * BB['ds']
        k = min(healthy, back // (2 * BB['ds']))
        healthy -= k
        if roll == 9:
            if crippled:
                crippled -= 1
            elif healthy:
                healthy -= 1
        ds = rules['battery_ds']
        if rules['steps'] == 1:
            bh -= min(bh, out // ds)
        else:
            k = min(bh, out // ds)
            bh -= k
            bc += k
            out -= k * ds
            if bh == 0:
                bc -= min(bc, out // ds)
    return [healthy, crippled], [bh, bc]


def siege(rules, rng, start, batteries, reinforce=0, years=15):
    """Years until the world's batteries are gone, ship points lost, or None."""
    ships = [start, 0]
    bats = [batteries, 0]
    sent = start
    for y in range(1, years + 1):
        if y > 1 and reinforce:
            ships[0] += reinforce
            sent += reinforce
        ships, bats = year(rules, rng, ships, bats)
        alive = ships[0] + ships[1]
        if bats[0] + bats[1] == 0:
            return y, (sent - alive) * BB['cost']
        if alive == 0 and not reinforce:
            return None, sent * BB['cost']
        ships = [alive, 0]
        if not rules['keep_crippled']:
            bats = [bats[0] + bats[1], 0]
    return None, sent * BB['cost']


CASES = [(b, f, r) for b in (2, 4, 7, 10, 20, 50)
         for f, r in ((1.5, 0), (2.0, 0), (3.0, 0), (2.0, 0.5), (1.0, 1.0))]


def run(rules, trials, seed):
    rng = random.Random(seed)
    out = []
    for batteries, force, reinforce in CASES:
        one = batteries * 20 / BB['cost']
        start, more = max(1, round(force * one)), round(reinforce * one)
        won = years = lost = 0
        for _ in range(trials):
            y, points = siege(rules, rng, start, batteries, more)
            lost += points
            if y is not None:
                won += 1
                years += y
        out.append(dict(batteries=batteries, force=force, reinforce=reinforce, start_bb=start,
                        reinforce_bb=more, cleared=won / trials,
                        years=years / won if won else None,
                        lost_vs_battery_cost=lost / trials / (batteries * 20)))
    return out


def main():
    trials, path = int(sys.argv[1]), sys.argv[2]
    results = {name: run(rules, trials, 20261016) for name, rules in RULES.items()}
    for name, rows in results.items():
        print(f"== {name}")
        for r in rows:
            years = f"{r['years']:.1f}" if r['years'] else '-'
            print(f"  {r['batteries']:2} batteries, {r['force']:.1f}x +{r['reinforce']:.1f}x/yr "
                  f"({r['start_bb']} BB +{r['reinforce_bb']}): cleared {r['cleared']:.2f}, "
                  f"years {years}, lost {r['lost_vs_battery_cost']:.1f}x")
    with open(path, 'w') as fh:
        json.dump(dict(trials=trials, seed=20261016, rules=RULES, results=results), fh, indent=1)


if __name__ == '__main__':
    main()
