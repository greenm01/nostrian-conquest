"""Classic EC's assaults on the cases of nc-fuzz's assault_odds example, beside
the engine's own results (t202).

Usage: python3 -I -B tools/combat-model/assault_classic_compare.py BEFORE.json AFTER.json [trials]

BEFORE and AFTER are assault_odds JSON outputs (the engine before and after
t202; nc_assault_engine_before.json and nc_assault_engine_t202.json). Classic
runs the decoded ECMAINT assault (tools/classic-model/sim.py): Invade (order
7), Blitz (order 8), and Bombard (order 6) each year until the batteries are
gone, then Invade. Each cell is capture rate, ship points lost (battleships
at 45), and production kept.
"""
import json
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'classic-model'))
import sim  # noqa: E402

ORDERS = {'invade': 7, 'blitz': 8}


def attempt(order, fleet, planet):
    out = sim.Assault(dict(fleet, roe=10), planet, order).run()
    fleet = {'bb': out['bb'], 'tl': out['tl']}
    planet = dict(planet, armies=out['armies'], bat=out['bat'], r3=out['r3'], r9=out['r9'], dock=out['dock'])
    return out['captured'], fleet, planet


def classic(case, years=8):
    fleet = {'bb': case['battleships'], 'tl': case['armies']}
    planet = dict(armies=case['defenders'], bat=case['batteries'], r3=100.0, r9=500.0, prod2=100, dock=[0] * 8)
    if case['plan'] in ORDERS:
        captured, fleet, planet = attempt(ORDERS[case['plan']], fleet, planet)
    else:
        for _ in range(years):
            captured, fleet, planet = attempt(6 if planet['bat'] > 0 else 7, fleet, planet)
            if captured or fleet['bb'] == 0 or fleet['tl'] == 0:
                break
    return captured, (case['battleships'] - fleet['bb']) * 45, planet['r3'] / 100


def main():
    before, after = (json.load(open(path)) for path in sys.argv[1:3])
    trials = int(sys.argv[3]) if len(sys.argv) > 3 else 1000
    sim.R.seed(20261016)
    cell = lambda r: f"{r['captured']:.2f} {r['ship_points_lost']:5.1f} {r['production_kept']:.2f}"
    print('case | plan | classic | NC before | NC t202')
    for b, a in zip(before, after):
        runs = [classic(a) for _ in range(trials)]
        c = (sum(r[0] for r in runs) / trials, sum(r[1] for r in runs) / trials, sum(r[2] for r in runs) / trials)
        print(f"{a['batteries']}b {a['defenders']}a vs {a['battleships']}BB+{a['armies']} | {a['plan']} | "
              f"{c[0]:.2f} {c[1]:5.1f} {c[2]:.2f} | {cell(b)} | {cell(a)}")


if __name__ == '__main__':
    main()
