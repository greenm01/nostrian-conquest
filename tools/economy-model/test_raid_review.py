import random
import unittest

from raid_review import FLEET, force, planet_hit, bombard, gate, trial
from assault_collateral import solve


class RaidTests(unittest.TestCase):
    def test_delayed_battery_shield_and_third_round_gate(self):
        world=(1,(('BB',1),),10,500,100)
        cleared=planet_hit(world,1000,3,100)
        self.assertEqual(cleared,(0,(('BB',1),),10,500,100))
        self.assertEqual(planet_hit(cleared,220,3,100),(0,(('BB',0),),0,300,98))

    def test_orbital_survivor_damage_is_not_healed(self):
        found=False
        for seed in range(100):
            a,b,passed=gate(random.Random(seed),force(CA=1),force(CA=1))
            if passed and a[1][1]:
                after,world=bombard(random.Random(10),a,(0,(),0,500,100),100)
                self.assertEqual(after[1],a[1])
                self.assertEqual(after[2],(0,0))
                found=True
                break
        self.assertTrue(found)

    def test_lost_gate_never_fires_on_planet(self):
        for seed in range(30):
            row=trial(random.Random(seed),{'CA':1},{'BB':10},(0,(),0,500,100),100)
            if not row['gate_pass']:
                self.assertEqual(row['production_destroyed'],0)
                self.assertEqual(row['goods_destroyed'],0)

    def test_rate_changes_only_goods_with_matched_rolls(self):
        for seed in range(30):
            a=trial(random.Random(seed),{'BB':3},{'SB':1},(3,(),10,500,100),100)
            b=trial(random.Random(seed),{'BB':3},{'SB':1},(3,(),10,500,100),25)
            self.assertGreaterEqual(a.pop('goods_destroyed'),b.pop('goods_destroyed'))
            self.assertEqual(a,b)

    def test_negative_goods_not_forgiven(self):
        self.assertEqual(planet_hit((0,(),0,-10,100),20,3,25),(0,(),0,-10,98))

    def test_unopposed_roll_enumeration_matches_prior_exact_model(self):
        # Enumerate third-shot d10; the first two shots have no industry target.
        from assault_collateral import bombard_as,shot
        for counts in ({'DD':1},{'CA':1},{'BB':1}):
            start=force(**counts)
            losses=[]
            for die in range(10):
                hits,crit=shot(bombard_as(start),1,die)
                after=planet_hit((0,(),0,500,100),hits+crit,3,100)
                losses.append((500-after[3],100-after[4]))
            prior=solve(start,(0,(),0,500,100),'delayed')
            self.assertAlmostEqual(sum(x[0] for x in losses)/10,prior['goods_lost'])
            self.assertAlmostEqual(sum(x[1] for x in losses)/10,prior['production_lost'])


if __name__=='__main__':
    unittest.main()
