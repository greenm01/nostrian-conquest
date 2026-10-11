import unittest
from fractions import Fraction as F

from conquest_review import captured, conversion, industry, bombard_control


class ConquestTests(unittest.TestCase):
    def test_mature_conversion_calendar(self):
        for schedule,expected in [('zero_zero_full',[0,0,50,50]),
                                  ('zero_half_full',[0,25,50,50])]:
            rows=captured(100,100,50,schedule,years=4)
            self.assertEqual([r['income'] for r in rows],expected)

    def test_half_growth_and_full_emergency_penalty(self):
        self.assertEqual(conversion(F(50),100,50,1,'zero_half_full'),
                         (F(425,8),F(425,32)))
        self.assertEqual(conversion(F(100),100,70,1,'zero_half_full'),
                         (F(99),F(693,20)))

    def test_old_money_and_paid_work_survive_conversion(self):
        rows=captured(100,100,50,'zero_zero_full',goods=50,owed=12,years=1)
        self.assertEqual((rows[0]['paid'],rows[0]['owed'],rows[0]['goods']),(12,0,38))

    def test_new_income_cannot_fund_same_pass_build(self):
        for schedule,completion in [('zero_zero_full',4),('zero_half_full',3)]:
            rows=captured(100,100,50,schedule,owed=12,years=4)
            self.assertEqual(next(r['pass_number'] for r in rows if not r['owed']),completion)

    def test_debt_fraction_and_capacity_conservation(self):
        for goods in (F(-10),F(9,2),F(200)):
            rows=captured(10,10,50,'zero_zero_full',goods=goods,owed=12,years=6)
            self.assertEqual(rows[-1]['goods'],goods+rows[-1]['income_total']-sum(r['paid'] for r in rows))
            self.assertTrue(all(r['paid']<=10 for r in rows))
            self.assertEqual(rows[0]['paid'],max(0,min(10,goods//1)))

    def test_recapture_does_not_mint_income(self):
        rows=captured(100,100,50,'zero_half_full',years=5,recapture_after=(1,2,3,4))
        self.assertEqual(sum(r['income'] for r in rows),0)

    def test_industry_remainders_and_debt(self):
        self.assertEqual(industry(100,500,[9,9,9]),(100,500))
        self.assertEqual(industry(100,500,[30]),(97,200))
        self.assertEqual(industry(F(1,2),F(-10),[100]),(0,-10))
        self.assertEqual(industry(100,F(1005,10),[10]),(99,F(1,2)))

    def test_zero_potential_never_recovers(self):
        rows=captured(0,0,50,'zero_zero_full')
        self.assertTrue(all(r['production']==0 and r['income']==0 for r in rows))
        recoverable=captured(100,0,50,'zero_zero_full')
        self.assertEqual(recoverable[2]['production'],F(25,2))

    def test_revenue_loss_matches_geometric_sum(self):
        row=bombard_control(100,500,[10])
        for n,r in row['at'].items():
            self.assertEqual(r['revenue_lost'],F(7,2)*(1-F(7,8)**n))


if __name__ == '__main__':
    unittest.main()
