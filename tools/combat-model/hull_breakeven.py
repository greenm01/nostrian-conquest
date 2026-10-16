"""Break-even counts on engine-t065: how many of one hull match a fixed force.

Usage: python3 -I -B tools/combat-model/hull_breakeven.py tools/combat-model 20000 out.json
Results of that run (seed 20261016, on bean-void) are in nc_hull_breakeven.json.
"""
import json, multiprocessing as mp, sys
sys.path.insert(0, sys.argv[1])
import rules as R, sweep
from fleet_model import Model

SERIES = [('CA', range(28, 41, 2), dict(BB=10)),
          ('DD', range(90, 151, 10), dict(CA=30)),
          ('DD', range(90, 181, 15), dict(BB=10))]

def job(a):
    hull, n, right, roe = a
    rules = R.load_rules('engine-t065'); units = R.load_units(rules['default_units'])
    rec = sweep.run_case(Model(rules, units), 'be', 'u', (f'{n}{hull} vs {right} roe{roe}', {hull: n}, right, roe, ''),
                         int(sys.argv[2]), 20261016, 0)
    return dict(hull=hull, n=n, right=right, roe=roe, p=rec['p'], loss_left=rec['loss_left'], loss_right=rec['loss_right'],
                withdrew_left=rec['withdrew_left'], withdrew_right=rec['withdrew_right'])

if __name__ == '__main__':
    jobs = [(h, n, r, roe) for h, ns, r in SERIES for n in ns for roe in ((10, 10), (6, 6))]
    with mp.Pool(3) as pool:
        out = list(pool.imap(job, jobs))
    json.dump(out, open(sys.argv[3], 'w'), indent=1)
