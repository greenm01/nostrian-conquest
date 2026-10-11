from fractions import Fraction as F
import unittest

from scorch_review import yearly,capture,recover,control


class ScorchTests(unittest.TestCase):
    def test_twenty_rolls_and_mean(self):
        values=[yearly(F(100),500,100,d)[0] for d in range(20)]
        self.assertEqual((min(values),max(values),sum(values)/20),(30,49,F(79,2)))

    def test_goods_branch_boundary(self):
        self.assertEqual(yearly(F(100),50,100,0)[1],5)
        self.assertEqual(yearly(F(100),100,100,0)[1],10)
        self.assertAlmostEqual(yearly(F(100),500,100,0)[1],500**.5)

    def test_floor_after_capture_and_no_potential_creation(self):
        self.assertEqual(capture(F(1),10,100),(1,5))
        self.assertEqual(yearly(F(0),0,0,0),(0,0))
        self.assertEqual(capture(F(0),0,0),(0,0))

    def test_destruction_preserves_debt(self):
        for g in (-100,-1,0):
            self.assertEqual(yearly(F(100),g,100,0)[1],g)
            self.assertEqual(capture(F(100),g,100)[1],g)

    def test_recovery_matches_direct_passes(self):
        from accounting_review import step
        for converted in (False,True):
            p=F(79,4)
            income=F(0)
            for year in range(1,21):
                if converted and year<=2:
                    continue
                p,y=step(p,100,50,'smooth')
                income+=y
            expected,end,_=recover(F(79,4),100,conversion=converted)
            self.assertEqual((expected,end),(income,p))

    def test_one_annual_scorch_then_capture(self):
        row=control(100,100,500,1,True)
        self.assertEqual(row['production_mean'],F(79,4))
        self.assertEqual((row['production_min'],row['production_max']),(15,F(49,2)))
        self.assertAlmostEqual(row['goods_left'],500**.5/2)


if __name__=='__main__':
    unittest.main()
