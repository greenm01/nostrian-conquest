"""Break-even counts for the t200 punch ladders on engine-t198 (no mixed-fleet bonus).

Usage: python3 -I -B tools/combat-model/ladder_breakeven.py tools/combat-model 20000 out.json
Results (seed 20261016, on bean-void): nc_ladder_breakeven.json.
"""
import json, multiprocessing as mp, sys
sys.path.insert(0, sys.argv[1])
import rules as R, sweep
from fleet_model import Model

LADDERS = {'now': [], 'A-dd11-ca33': ['units.DD.AS=11', 'units.CA.AS=33'],
           'B-dd12-ca33': ['units.DD.AS=12', 'units.CA.AS=33']}
SERIES = [('CA', range(26, 37, 2), dict(BB=10)),
          ('DD', range(80, 141, 10), dict(CA=30)),
          ('DD', range(80, 141, 10), dict(BB=10))]


def job(a):
    ladder, hull, n, right, roe = a
    rules = R.load_rules('engine-t198'); units = R.load_units(rules['default_units'])
    R.apply_overrides(rules, units, LADDERS[ladder])
    rec = sweep.run_case(Model(rules, units), ladder, 'u', (f'{n}{hull} vs {right} roe{roe[0]}', {hull: n}, right, roe, ''),
                         int(sys.argv[2]), 20261016, 0)
    return dict(ladder=ladder, hull=hull, n=n, right=right, roe=roe, p=rec['p'],
                loss_left=rec['loss_left'], loss_right=rec['loss_right'])


if __name__ == '__main__':
    jobs = [(l, h, n, r, roe) for l in LADDERS for h, ns, r in SERIES for n in ns for roe in ((10, 10), (6, 6))]
    with mp.Pool(4) as pool:
        out = list(pool.imap(job, jobs))
    json.dump(out, open(sys.argv[3], 'w'), indent=1)
