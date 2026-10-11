"""t072 collateral boundary and exact simple bombardment checks."""
import unittest
from assault_collateral import damage,solve
from fleet_model import force


class AssaultCollateralTests(unittest.TestCase):
    def test_last_battery_shields_dock_for_entire_exchange(self):
        p=(1,(('BB',1),),10,500,100)
        after=damage(p,200,1,'delayed')
        self.assertEqual(after,(0,(('BB',1),),10,500,100))
        self.assertEqual(damage(after,200,2,'delayed'),(0,(('BB',0),),10,500,100))

    def test_last_round_clear_does_not_break_through(self):
        self.assertEqual(damage((1,(),10,500,100),1000,3,'delayed'),(0,(),10,500,100))
        self.assertEqual(damage((0,(),10,500,100),120,3,'delayed'),(0,(),0,300,98))

    def test_dock_cannot_protect_battery(self):
        empty=solve(force(BB=1),(2,(),10,500,100),'delayed')
        dock=solve(force(BB=1),(2,(('BB',1),),10,500,100),'delayed')
        self.assertAlmostEqual(empty['ship_cost_lost'],dock['ship_cost_lost'])
        self.assertAlmostEqual(empty['batteries_lost'],dock['batteries_lost'])

    def test_undefended_economic_damage_closed_form(self):
        # One BB bombard AS135, overwhelming quarter multipliers4,5,6,6,7,7,8,8,8,10.
        # Only third exchange hits the economy; natural9 adds1 raw hit.
        losses=[((135*k+3)//4+(i==9))//10 for i,k in enumerate((4,5,6,6,7,7,8,8,8,10))]
        row=solve(force(BB=1),(0,(),0,500,100),'delayed')
        self.assertAlmostEqual(row['production_lost'],sum(losses)/10)
        self.assertAlmostEqual(row['goods_lost'],500.)
        self.assertEqual(row['ship_cost_lost'],0.)


if __name__=='__main__':
    unittest.main()
