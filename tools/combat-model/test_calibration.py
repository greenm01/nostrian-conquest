"""t068 independent checks of exact ground and cohort allocation wrappers."""
import random
import importlib.util
import unittest

import recovery as rt
from ground_calibration import Shared, blitz_landing
from rules import EOTS_X4, UNITS
from rt_campaign import Tracked, SCHEDULES, cohorts_for, fleet, run_case
from fleet_model import Model


class CalibrationTests(unittest.TestCase):
    def test_first_fire_single_army_closed_form(self):
        # Enumerate totals of quarter-point progress independently. The first
        # side to reach four kills its opponent; same-round ties defend.
        states={(0,0):1.}
        win=0.
        for _ in range(4):
            nxt={}
            for (a,d),p in states.items():
                for kd in EOTS_X4:
                    if d+kd>=4:
                        continue
                    for ka in EOTS_X4:
                        if a+ka>=4:
                            win+=p/100
                        else:
                            key=a+ka,d+kd
                            nxt[key]=nxt.get(key,0)+p/100
            states=nxt
        self.assertAlmostEqual(Shared(EOTS_X4,4,first=True).solve(1,1)[0],win)
        self.assertFalse(states)

    @unittest.skipUnless(importlib.util.find_spec('numpy') and importlib.util.find_spec('scipy'),
                         'optional finite-matrix control needs numpy/scipy; see README')
    def test_independent_cap_is_real(self):
        from ground_calibration import independent
        out=independent(1,1,cap=1)(1,1)
        self.assertAlmostEqual(out['capture'],.30*.65)
        self.assertAlmostEqual(out['mutual'],.30*.35)
        self.assertAlmostEqual(out['cap'],.70*.65)
        self.assertEqual(out['rounds'],1.)

    def test_current_price_matches_previous_ground(self):
        from invasion_price import current, softened
        from nc_combat_analysis import ground
        for a in range(5,21):
            self.assertAlmostEqual(current(a,10)[0],ground(a,10)[0])
        # 6 CA guarantee the existing half-garrison softening cap here.
        self.assertAlmostEqual(softened(10,6)[5],1.)

    def test_simultaneous_symmetry(self):
        model=Shared(EOTS_X4,4)
        w,_,mutual,_,_,_=model.solve(10,10)
        self.assertAlmostEqual(2*w+mutual,1.)

    def test_blitz_cover_no_battery_and_unescorted(self):
        self.assertEqual(set(blitz_landing(10,3,0)),{10})
        self.assertAlmostEqual(blitz_landing(10,3,0)[10],1.)
        self.assertAlmostEqual(sum(blitz_landing(20).values()),1.)
        self.assertEqual(set(blitz_landing(10,0,3)),{0})
        self.assertAlmostEqual(blitz_landing(10,0,3)[0],1.)
        # 3 CA vs3 batteries: cover kills all on rolls6..9; no landing fire.
        self.assertAlmostEqual(blitz_landing(10)[10],.4)

    def test_cohort_observer_preserves_battle(self):
        tracked=Tracked(SCHEDULES['class123-sb2'],'pursuit')
        plain=Model(tracked.rules,UNITS)
        for seed in range(50):
            own=cohorts_for(dict(DD=12,CA=6,BB=2))
            enemy=cohorts_for(dict(CA=12))
            # Include previously crippled hulls to test old-vs-fresh allocation.
            own[0]=rt.force(8,[(1,2),(2,2)])
            a0,b0=fleet(own),fleet(enemy)
            a,b=tracked.fight(own,enemy,random.Random(seed),(6,10))
            expected=plain.simulate(a0,b0,random.Random(seed),(6,10))
            self.assertEqual(tuple(a0[0][i]-a[0][i] for i in range(7)),tuple(expected[11:18]))
            self.assertEqual(tuple(b0[0][i]-b[0][i] for i in range(7)),tuple(expected[18:25]))
            self.assertEqual(a[:2],fleet(own)[:2])

    def test_rt_first_return_excludes_damage_year(self):
        for gap in (0,1):
            a=run_case('screen','uniform2',gap,False,'none',20,123)
            b=run_case('screen','class123-sb2',gap,False,'none',20,123)
            self.assertEqual(a['entries'][1],b['entries'][1])
        for gap in (2,3):
            a=run_case('screen','uniform2',gap,False,'none',20,123)
            b=run_case('screen','class123-sb2',gap,False,'none',20,123)
            if gap==2:
                self.assertEqual(b['entries'][1]['class_damaged_fraction']['DD'],0.)
            if gap==3:
                self.assertEqual(a['entries'][1]['damaged_probability_given_alive'],0.)


if __name__=='__main__':
    unittest.main()
