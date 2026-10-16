"""Starbase DS check for t200 (DD 12/7, CA 33/30, BB 90/110, no mixed-fleet bonus).

Usage: python3 -I -B tools/combat-model/sb_check.py tools/combat-model 20000 out.json
Results (seed 20261016, on bean-void): nc_sb_check.json.
"""
import json, multiprocessing as mp, sys
sys.path.insert(0, sys.argv[1])
import rules as R, sweep
from fleet_model import Model

ATTACKS = {'10DD': dict(DD=10), '20DD': dict(DD=20), '4CA': dict(CA=4), '7CA': dict(CA=7),
           '1BB': dict(BB=1), '2BB': dict(BB=2), '3BB': dict(BB=3), '4BB': dict(BB=4)}


def job(a):
    sb_ds, name, roe = a
    rules = R.load_rules('engine-t198'); units = R.load_units(rules['default_units'])
    R.apply_overrides(rules, units, [f'units.SB.DS={sb_ds}'])
    rec = sweep.run_case(Model(rules, units), 'sb', 'u', (f'{name} vs 1 SB', ATTACKS[name], dict(SB=1), (roe, 10), ''),
                         int(sys.argv[2]), 20261016, 0)
    return dict(sb_ds=sb_ds, attack=name, roe=roe, p=rec['p'], loss_left=rec['loss_left'], loss_right=rec['loss_right'])


if __name__ == '__main__':
    jobs = [(ds, n, roe) for ds in (120, 130, 135) for n in ATTACKS for roe in (10, 6)]
    with mp.Pool(6) as pool:
        out = list(pool.imap(job, jobs))
    json.dump(out, open(sys.argv[3], 'w'), indent=1)
