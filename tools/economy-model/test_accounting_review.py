import unittest
from fractions import Fraction as F

from accounting_review import RULES, equilibrium, step, trajectory, whole_capacity
from analyze import nc_step


class AccountingTests(unittest.TestCase):
    def test_integer_baseline_matches_retained_helpers(self):
        for k in (10,100,150):
            for p in range(k+1):
                for tax in range(101):
                    for base in (False,True):
                        self.assertEqual(step(p,k,tax,'current',base),nc_step(p,k,tax,base))

    def test_income_carry_leaves_production_unchanged(self):
        for t in range(101):
            self.assertEqual(step(47,100,t,'current')[0],step(47,100,t,'income_carry')[0])
        self.assertEqual(step(100,100,71,'income_carry'),(98,F(6958,100)))

    def test_smooth_matches_exact_linear_recurrence(self):
        rows=trajectory('smooth',100,[50]*10)
        expected=100-99*F(7,8)**10
        self.assertEqual(rows[-1]['production'],expected)
        self.assertEqual(rows[-1]['goods'],sum(r['income'] for r in rows))

    def test_smooth_equilibrium(self):
        for t in range(101):
            p=equilibrium(100,t)
            self.assertEqual(step(p,100,t,'smooth')[0],p)

    def test_bounds_and_rational_results(self):
        for rule in RULES:
            for tax in (0,50,65,71,100):
                p,income=step(F(1),10,tax,rule)
                self.assertIsInstance(p,F)
                self.assertGreaterEqual(p,0)
                self.assertLessEqual(p,10)
                self.assertGreaterEqual(income,0)

    def test_capacity_rounding_does_not_make_final_capacity_unreachable(self):
        self.assertEqual(whole_capacity(F(999,10)),100)
        self.assertEqual(whole_capacity(F(994,10)),99)
        self.assertEqual(whole_capacity(F(999,10),True),500)
        self.assertLessEqual(trajectory('smooth',10,[50]*40)[-1]['order_budget'],10)

    def test_gated_floor_is_strictly_below_65(self):
        self.assertEqual(step(1,10,64,'gated_min1'),(F(2),F(128,100)))
        self.assertEqual(step(1,10,65,'gated_min1'),step(1,10,65,'smooth'))
        self.assertEqual(step(1,10,50,'gated_min1'),step(1,10,50,'smooth_min1'))
        self.assertEqual(step(F(19,2),10,64,'gated_min1')[0],10)

    def test_gated_high_tax_path_matches_smooth(self):
        for tax in (65,66,70,71,100):
            self.assertEqual(trajectory('gated_min1',100,[tax]*20,start=100),
                             trajectory('smooth',100,[tax]*20,start=100))

    def test_gated_recovery_can_repeat_profitable_tax_cycle(self):
        rows=trajectory('gated_min1',100,[75,64,64],start=100)
        self.assertEqual(rows[-1]['production'],100)
        self.assertEqual(rows[-1]['goods'],F(20086,100))
        self.assertGreater(rows[-1]['goods'],3*65)

    def test_head_start_difference_decays_without_new_growth_rule(self):
        for tax in (50,65,70,71):
            one=trajectory('smooth',100,[tax]*10,start=1)
            three=trajectory('smooth',100,[tax]*10,start=3)
            decay=1-F(100-min(tax,95),400)-F(max(0,tax-65),500)
            for n,(a,b) in enumerate(zip(one,three),1):
                self.assertEqual(b['production']-a['production'],2*decay**n)
        self.assertEqual(step(min(2,3),2,50,'smooth')[0],2)

    def test_whole_spending_preserves_fractional_reserve(self):
        goods=F(49,10)
        budget=max(0,min(goods//1,whole_capacity(F(100))))
        self.assertEqual(budget,4)
        self.assertEqual(goods-budget,F(9,10))
        self.assertEqual(max(0,F(-1,2)//1),0)


if __name__=='__main__':
    unittest.main()
