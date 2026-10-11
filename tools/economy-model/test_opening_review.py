import unittest

from opening_review import home_orders,local_dates,peaceful,PRICES,summary


class OpeningTests(unittest.TestCase):
    def test_joined_cash_funds_two_etacs_but_not_two_escorts(self):
        self.assertEqual([r['complete_pass'] for r in home_orders([20,20])],[1,1])
        self.assertEqual([r['complete_pass'] for r in home_orders([35,35])],[1,2])
        self.assertEqual([r['complete_pass'] for r in home_orders([35,35],75)],[1,1])

    def test_first_turn_capital_access_depends_on_opening_cash(self):
        self.assertEqual(home_orders([45],50)[0]['complete_pass'],1)
        self.assertEqual(home_orders([45],35)[0]['complete_pass'],2)
        self.assertEqual(home_orders([50],50)[0]['complete_pass'],1)

    def test_colony_order_completion_and_departure_are_distinct(self):
        row=local_dates(100,45)
        self.assertEqual((row['order_after_pass'],row['complete_pass'],row['first_departure_pass']),(5,6,7))
        self.assertEqual(local_dates(10,5)['order_after_pass'],3)
        self.assertEqual(local_dates(20,20)['order_after_pass'],27)
        self.assertIsNone(local_dates(10,15))

    def test_new_colonies_have_no_income_in_the_founding_pass(self):
        row=next(row for seed in range(100)
                 if (row:=peaceful(seed,worlds=1))['milestones']['first_colony']==5)
        self.assertEqual(row['at'][5]['colonies'],1)
        self.assertEqual(row['at'][5]['colony_income'],0)
        self.assertGreater(row['at'][10]['colony_income'],0)

    def test_etac_price_changes_cash_not_dates_inside_opening_budget(self):
        a=peaceful(880001,policy='rapid4',etac_cost=15)
        b=peaceful(880001,policy='rapid4',etac_cost=25)
        self.assertEqual(a['milestones'],b['milestones'])
        self.assertEqual(a['at'][5]['colonies'],b['at'][5]['colonies'])
        self.assertEqual(a['at'][5]['home_goods']-b['at'][5]['home_goods'],20)

    def test_expansion_spending_is_actually_deducted(self):
        for policy,paid in [('two',0),('rapid4',40),('guarded4',70)]:
            row=peaceful(880001,policy=policy)
            self.assertEqual(row['at'][5]['home_goods'],50+5*50-paid)
            self.assertEqual(row['at'][5]['expansion_paid'],paid)

    def test_missing_milestones_are_reported_as_censored(self):
        result=summary([peaceful(1,worlds=0)])
        self.assertEqual(result['milestones']['first_colony']['reach_fraction'],0)
        self.assertIsNone(result['milestones']['first_colony']['mean'])


if __name__=='__main__':
    unittest.main()
