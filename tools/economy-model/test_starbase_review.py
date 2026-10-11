import unittest
from fractions import Fraction as F

from accounting_review import step
from starbase_review import bonus, grow, growth_control, local_build, surge


class StarbaseTests(unittest.TestCase):
    def test_growth_matches_approved_smooth_model(self):
        for p in (F(3),F(501,10),F(100)):
            for tax in range(101):
                for n in (0,1,3):
                    self.assertEqual(grow(p,100,tax,n),step(p,100,tax,'smooth',bool(n)))

    def test_multiple_bases_do_not_stack(self):
        for n in (2,3,10):
            self.assertEqual(grow(F(3),100,50,n),grow(F(3),100,50,1))
            self.assertEqual(surge(500,n),surge(500,1))

    def test_growth_and_income_at_full_production_do_not_change(self):
        self.assertEqual(grow(F(100),100,50,1),(100,50))
        self.assertEqual(bonus(65),0)
        self.assertEqual(bonus(50),F(1,2))
        self.assertEqual(bonus(60),F(1,6))

    def test_finite_growth_return_bounds_payback(self):
        poor=growth_control(20,3,50)
        self.assertEqual(poor['lifetime_extra_income'],F(68,3))
        self.assertIsNone(poor['payback_pass'])
        rich=growth_control(100,3,50)
        self.assertEqual(rich['lifetime_extra_income'],F(388,3))
        self.assertEqual(rich['payback_pass'],8)
        self.assertLess(rich['at'][80]['extra_income'],rich['lifetime_extra_income'])

    def test_imposed_arrival_delay_reduces_return(self):
        now=growth_control(100,3,50)
        later=growth_control(100,3,50,delay=5)
        self.assertEqual(later['lifetime_extra_income'],now['lifetime_extra_income']*F(7,8)**5)
        self.assertEqual(later['at'][5]['extra_income'],0)
        self.assertEqual(later['payback_pass'],22)

    def test_local_construction_commission_and_growth_are_separate_passes(self):
        result=local_build(100,50)
        self.assertEqual((result['order_pass'],result['completion_pass'],result['first_bonus_pass']),(5,6,7))
        self.assertIsNone(local_build(20,50)['order_pass'])

    def test_spending_conserves_cash_and_does_not_multiply_income(self):
        for n in (0,1,3):
            rows=surge(500,n)
            self.assertEqual(sum(r['spent'] for r in rows)+rows[-1]['goods'],750)
            self.assertEqual([r['spent'] for r in surge(50,n)],[50]*5)
        self.assertEqual(surge(500,1)[0]['spent'],500)
        self.assertEqual(surge(500,0)[0]['spent'],100)


if __name__=='__main__':
    unittest.main()
