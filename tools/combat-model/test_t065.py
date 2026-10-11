"""t065 one-contact pursuit law and selected-unit comparison regressions."""
import math
import unittest
from unittest.mock import patch
import io
import json

import rules as R
from fleet_model import Model, force
from sweep import compare_engine


class T065Tests(unittest.TestCase):
    def setUp(self):
        self.rules=R.load_rules('engine-t065')
        self.model=Model(self.rules,R.load_units(self.rules['default_units']))

    def test_default_hull_table(self):
        self.assertEqual(self.model.DS[0],8)
        self.assertEqual(R.load_units('baseline')['DD']['DS'],5)

    def test_pursuit_integrated_speed_law(self):
        self.assertEqual(self.model.pursuit_probability(force(DD=1),force(CA=1)),1)
        self.assertAlmostEqual(self.model.pursuit_probability(force(CA=1),force(CA=1)),2/math.pi)
        self.assertAlmostEqual(self.model.pursuit_probability(force(BB=1),force(DD=1)),4/(3*math.pi))
        self.assertEqual(self.model.pursuit_probability(force(SB=1),force(CA=1)),0)
        self.assertEqual(self.model.pursuit_probability(force(TT=1),force(CA=1)),0)

    def test_slowest_surviving_class_and_no_cripple_speed_penalty(self):
        a=force(DD=1,BB=1)
        self.assertAlmostEqual(self.model.pursuit_probability(a,force(CA=1)),8/(5*math.pi))
        crippled=(a[0],a[0],a[2])
        self.assertEqual(self.model.pursuit_probability(a,force(CA=1)),self.model.pursuit_probability(crippled,force(CA=1)))
        self.assertEqual(self.model.pursuit_probability(force(DD=1),force(CA=1)),1)

    def test_failed_catch_has_no_extra_exchange_and_carry_stays_on_catch(self):
        a=force(CA=1)
        a=(a[0],a[1],(7,9))
        b=force(CA=2)
        calls=[]
        def exchange(a,b,**kw):
            calls.append((a,b,kw))
            return [(1.,(a,b))]
        branches=self.model._leave(a,b,True,False,0,exchange)
        self.assertAlmostEqual(sum(p for p,_,_ in branches),1)
        self.assertEqual([n for _,n,_ in branches],[1,0])
        self.assertEqual(calls[0][0][2],(7,9))
        self.assertEqual(calls[0][2],dict(expose_a=False,expose_b=False))

    def test_both_leave_without_pursuit_and_bases_never_leave(self):
        def no_exchange(*args,**kwargs):
            self.fail('mutual withdrawal must not fire')
        a,b=force(CA=1),force(CA=1)
        self.assertEqual(self.model._leave(a,b,True,True,0,no_exchange),[(1.,0,('terminal',a,b,True,True))])
        self.assertFalse(self.model.wants_withdrawal(force(SB=1),999,0))
        self.assertTrue(self.model.wants_withdrawal(force(TT=1),30,0))

    def test_comparison_uses_ds8_row(self):
        record=dict(ruleset='engine-t065',units='dd-10-8-5',case='probe',method='exact',left=dict(DD=1),right=dict(CA=1),loss_left=5,loss_right=0,p=dict(A=0,B=1,mutual=0,stalemate=0))
        engine=dict(cases=[dict(case='probe',battles=100,counts=dict(left=0,right=100,mutual=0,stalemate=0),loss_left=5,loss_right=0,years_per_battle=1)])
        with patch('builtins.open',return_value=io.StringIO(json.dumps(engine))):
            self.assertEqual(compare_engine('unused',[record],[],'engine-t065')[0]['agree'],'yes')


if __name__=='__main__':
    unittest.main()
