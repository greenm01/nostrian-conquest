import itertools
import random
import unittest

from fleet_model import force
from participation_review import MODEL, apportion, bonus_shot, transport_losses, willing, withdrawals


class ParticipationTests(unittest.TestCase):
    def test_t200_hulls_under_t206_rules(self):
        self.assertEqual((MODEL.AS[0], MODEL.DS[0]), (12, 7))
        self.assertEqual(MODEL.EXE_FIN[:3], [4, 15, 55])
        self.assertEqual(MODEL.line[:3], [2, 1, 0])
        self.assertTrue(MODEL.heavy_line(force(CA=10)[0], force(CA=5)[0]))
        self.assertFalse(MODEL.heavy_line(force(CA=9)[0], force(CA=5)[0]))
        self.assertFalse(MODEL.heavy_line(force(DD=90)[0], force(DD=1)[0]))

    def test_admission_uses_assembled_power_and_excludes_roe_zero(self):
        self.assertEqual(willing([180,180],[6,6],300,True), [0,1])
        self.assertEqual(willing([180,180],[6,6],300,False), [])
        self.assertEqual(willing([180,180],[6,0],300,True), [])

    def test_both_sides_leave_and_no_rejoining(self):
        self.assertEqual(withdrawals(([300],[300]),([5],[5]))['remaining'], [[],[]])
        result = withdrawals(([100,100],[200]),([5,10],[5]))
        self.assertEqual(result['remaining'], [[1],[]])
        self.assertEqual(result['iterations'][0]['after'], [[1],[]])

    def test_unarmed_escape_and_base_stays(self):
        result = withdrawals(([0],[100]),([10],[6]),bases=(100,0))
        self.assertEqual(result['remaining'], [[],[0]])
        self.assertEqual(result['iterations'][-1]['power'], [100,100])

    def test_allocation_conserves_and_bounds_losses(self):
        self.assertEqual(apportion([8,2],3,random.Random(1)), [2,1])
        for counts in itertools.product(range(4), repeat=3):
            for losses in range(sum(counts)+1):
                result = apportion(counts,losses,random.Random(1))
                self.assertEqual(sum(result),losses)
                self.assertTrue(all(0 <= x <= n for x,n in zip(result,counts)))

    def test_cutoff_tie_can_award_either_fleet(self):
        class Pick:
            def __init__(self, choice):
                self.choice = choice
            def sample(self, candidates, count):
                self.asserted = (candidates, count)
                return [self.choice]
        for choice in (0, 1):
            rng = Pick(choice)
            result = apportion([1,1],1,rng)
            self.assertEqual(result[choice],1)
            self.assertEqual(rng.asserted,([0,1],1))

    def test_tie_replays_with_same_seed_and_untied_awards_draw_nothing(self):
        first, second = random.Random(73), random.Random(73)
        results = [apportion([1,1,1],2,first) for _ in range(10)]
        self.assertEqual(results,[apportion([1,1,1],2,second) for _ in range(10)])
        before = first.getstate()
        self.assertEqual(apportion([8,2],3,first),[2,1])
        self.assertEqual(before,first.getstate())

    def test_splitting_loaded_transports_does_not_save_cargo(self):
        together = transport_losses([6],[4],4,random.Random(1))
        split = transport_losses([6,0],[0,4],4,random.Random(1))
        self.assertEqual(sum(together['loaded']),sum(split['loaded']))
        self.assertEqual(split,dict(loaded=[4,0],empty=[0,0]))

    def test_surprise_preserves_natural_nine_and_caps_total_shift(self):
        side, enemy = force(CA=5, DD=5, SB=1), force(CA=3)
        plain, swarm = force(CA=10), force(DD=10)
        for roll in range(10):
            # starbase and heavy line already fill the cap
            self.assertEqual(bonus_shot(MODEL,side,enemy,roll,1),MODEL.volley(side,99,roll,enemy=enemy))
            self.assertEqual(bonus_shot(MODEL,plain,swarm,roll,1)[:2],MODEL.volley(plain,120,min(9,roll+1),enemy=swarm)[:2])
            self.assertEqual(bonus_shot(MODEL,plain,swarm,roll,1)[2],roll==9)

    def test_no_surprise_volley_matches_model(self):
        enemy = force(CA=10)
        for side in (force(CA=10),force(DD=5,CA=5),force(SB=1)):
            for roll in range(10):
                self.assertEqual(bonus_shot(MODEL,side,enemy,roll,0),MODEL.volley(side,330,roll,enemy=enemy))


if __name__ == '__main__':
    unittest.main()
