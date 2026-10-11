"""t075 controls: approved rules, hull overrides and recovery bookkeeping."""
import random
import unittest

from fleet_model import Model, force, LOST_A, LOST_B
from hull_review import HullTracked, MATRIX, ESCORTS, battle, campaign, rules_for, units_for
from rules import CLASSES, RULESETS, UNITS
from rt_campaign import cohorts_for, fleet
import recovery as rt


class HullReviewTests(unittest.TestCase):
    def test_roe10_warships_match_decision1(self):
        for ds in (5,8):
            current=Model(RULESETS['engine-t060'],units_for(ds))
            approved=Model(rules_for('pursuit'),units_for(ds))
            for _,a,b in MATRIX:
                for seed in range(12):
                    self.assertEqual(current.simulate(force(**a),force(**b),random.Random(seed)),
                                     approved.simulate(force(**a),force(**b),random.Random(seed)))

    def test_escort_budgets_equal_and_values_not_mutated(self):
        self.assertEqual([sum(UNITS[c]['cost']*n for c,n in a.items()) for _,a in ESCORTS],[75]*3)
        units_for(8)
        self.assertEqual(UNITS['DD']['DS'],5)

    def test_changed_ds_cohorts_match_untracked_combat(self):
        for ds in (5,8):
            tracked=HullTracked(ds,'pursuit')
            plain=Model(tracked.rules,units_for(ds))
            for seed in range(30):
                own=cohorts_for(dict(DD=15,CA=5))
                own[0]=rt.force(11,[(1,2),(2,2)])
                enemy=cohorts_for(dict(CA=10))
                a0,b0=fleet(own),fleet(enemy)
                a,b=tracked.fight(own,enemy,random.Random(seed),(6,10))
                expected=plain.simulate(a0,b0,random.Random(seed),(6,10))
                self.assertEqual(a[:2],fleet(own)[:2])
                self.assertEqual(tuple(a0[0][i]-a[0][i] for i in range(len(CLASSES))),tuple(expected[LOST_A]))
                self.assertEqual(tuple(b0[0][i]-b[0][i] for i in range(len(CLASSES))),tuple(expected[LOST_B]))

    def test_three_year_return_is_recovered(self):
        row=campaign('test',dict(DD=15,CA=5),dict(CA=10),8,3,'none',50,75)
        self.assertTrue(all(e['damaged_given_alive'] in (0,None) for e in row['entries']))
        one=campaign('test',dict(DD=15,CA=5),dict(CA=10),8,1,'none',50,75)
        two=campaign('test',dict(DD=15,CA=5),dict(CA=10),8,2,'none',50,75)
        self.assertEqual(one['entries'][1]['damaged_given_alive'],two['entries'][1]['damaged_given_alive'])
        self.assertGreater(one['entries'][1]['damaged_given_alive'],0)

    def test_transport_measure_does_not_count_escorts(self):
        row=battle('unopposed',dict(TT=10),{},8,(6,10),'none',1,5,75)
        self.assertEqual(row['transports_surviving'],10)
        self.assertEqual(row['half_transports_survive'],1)


if __name__=='__main__':
    unittest.main()
