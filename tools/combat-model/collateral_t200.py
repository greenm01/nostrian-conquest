"""assault_collateral.py's exact bombardment model under the t200 hulls
(DD 12/7, CA 33/30, BB 90/110, bombardment weights 6 / 33 / 135, no
mixed-fleet shift), for the cases the engine's planetary tests check.

Usage: python3 -I -B tools/combat-model/collateral_t200.py
"""
import sys
import os

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import assault_collateral as AC  # noqa: E402
from fleet_model import Model, force  # noqa: E402
from rules import RULESETS, load_units, variant  # noqa: E402

AC.UNITS = load_units('t200')
AC.MODEL = Model(variant(RULESETS['engine-t060'], 'planetary-t200', remainder='discard', mixed_shift=0), AC.UNITS)


def bombard_as(side):
    n, c, _ = side
    return sum((n[i] - c[i]) * full + c[i] * half
               for i, full, half in ((AC.DD, 6, 3), (AC.CA, 33, 16), (AC.BB, 135, 67)))


AC.bombard_as = bombard_as

if __name__ == '__main__':
    for name, counts, planet, order in [('CA6-vs-B3-dockedBB', dict(CA=6), (3, (('BB', 1),), 10, 500, 100), 'delayed'),
                                        ('CA1-no-defense', dict(CA=1), (0, (), 0, 5000, 100), 'delayed'),
                                        ('BB1-no-defense', dict(BB=1), (0, (), 0, 5000, 100), 'delayed')]:
        row = AC.solve(force(**counts), planet, order)
        print(name, {k: round(v, 3) for k, v in row.items() if k != 'states'})
