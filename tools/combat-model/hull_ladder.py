"""The hull_mix.py round robin on engine-t198 (no mixed-fleet bonus) for the t200 punch ladders.

Usage: python3 -I -B tools/combat-model/hull_ladder.py tools/combat-model 20000 4 out.json 6
Results (seed 20261016, on bean-void, ROE 6): nc_hull_ladder_roe6.json.
"""
import itertools
import json
import multiprocessing as mp
import sys

sys.path.insert(0, sys.argv[1])
import rules as R  # noqa: E402
import sweep  # noqa: E402
from fleet_model import Model  # noqa: E402

# All 450 build points (DD 5, CA 15, BB 45).
COMPS = {
    '90DD': dict(DD=90),
    '30CA': dict(CA=30),
    '10BB': dict(BB=10),
    '27CA+9DD': dict(CA=27, DD=9),
    '24CA+18DD': dict(CA=24, DD=18),
    '9BB+9DD': dict(BB=9, DD=9),
    '8BB+18DD': dict(BB=8, DD=18),
    '5BB+15CA': dict(BB=5, CA=15),
    '4BB+15CA+9DD': dict(BB=4, CA=15, DD=9),
}
COST = dict(DD=5, CA=15, BB=45)
for name, comp in COMPS.items():
    assert sum(COST[k] * v for k, v in comp.items()) == 450, name

ROE = int(sys.argv[5]) if len(sys.argv) > 5 else 10
VARIANTS = {'now': [], 'A-dd11-ca33': ['units.DD.AS=11', 'units.CA.AS=33'], 'B-dd12-ca33': ['units.DD.AS=12', 'units.CA.AS=33']}


def job(args):
    variant, a, b, trials = args
    rules = R.load_rules('engine-t198')
    units = R.load_units(rules['default_units'])
    R.apply_overrides(rules, units, VARIANTS[variant])
    model = Model(rules, units)
    case = (f'{a} vs {b}', COMPS[a], COMPS[b], (ROE, ROE), '')
    rec = sweep.run_case(model, variant, rules['default_units'], case, trials, 20261016, 0)
    return dict(variant=variant, left=a, right=b, p=rec['p'], rounds=rec['rounds'],
                loss_left=rec['loss_left'], loss_right=rec['loss_right'],
                crippled_left=rec['crippled_left'], crippled_right=rec['crippled_right'],
                ci=rec.get('ci'), seconds=rec['seconds'])


def main():
    trials, workers, out = int(sys.argv[2]), int(sys.argv[3]), sys.argv[4]
    jobs = [(v, a, b, trials) for v in VARIANTS for a, b in itertools.combinations(COMPS, 2)]
    with mp.Pool(workers) as pool:
        results = []
        for rec in pool.imap_unordered(job, jobs):
            results.append(rec)
            print(f"{rec['variant']:9} {rec['left']:>13} vs {rec['right']:<13} "
                  f"L {rec['p']['A']:.3f} R {rec['p']['B']:.3f} ({rec['seconds']}s)", flush=True)
    with open(out, 'w') as fh:
        json.dump(dict(trials=trials, comps=COMPS, results=results), fh, indent=1)


if __name__ == '__main__':
    main()
