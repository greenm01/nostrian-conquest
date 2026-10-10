"""Small exact checks for important claims, not engine-conformance tests."""
import unittest

import nc_combat_analysis as fleet
import ground
import classic_reference


class FixedRolls:
    def __init__(self,*rolls):
        self.rolls=iter(rolls)

    def randrange(self,limit):
        value=next(self.rolls)
        assert 0<=value<limit
        return value


class CombatModelChecks(unittest.TestCase):
    def test_classic_ground_matches_one_army_closed_form(self):
        # Defender hits first. A surviving attacker wins with probability .3.
        # Both misses repeat the state; average over the battle-level die.
        expected=sum((1-p/100)*.3/(p/100+(1-p/100)*.3)
                     for p in range(40,50))/10
        self.assertAlmostEqual(classic_reference.ground(1,1)['capture'],expected)
        self.assertEqual(classic_reference.ground_fixed(0,5,40),(0.,0.))
        self.assertEqual(classic_reference.ground_fixed(5,0,40),(1.,0.))

    def test_current_tiny_attack_has_exact_ten_percent_free_capital_kill(self):
        mutual=0
        for dd_roll in range(10):
            for bb_roll in range(10):
                result=fleet.battle(fleet.force(dd=1),fleet.force(bb=1),'C',FixedRolls(dd_roll,bb_roll))
                self.assertEqual(result[1],1)
                self.assertEqual(result[0],'mutual' if dd_roll==9 else 'B')
                mutual+=result[0]=='mutual'
        self.assertEqual(mutual,10)

    def test_current_ground_cliff(self):
        self.assertAlmostEqual(fleet.ground(10,10)[0],.08)
        self.assertAlmostEqual(fleet.ground(11,10)[0],.83)

    def test_defender_first_removes_attackers_before_reply(self):
        self.assertEqual(ground.solve(1,10),(0.,1.))

    def test_ground_remainder_can_resolve_subthreshold_volleys(self):
        # A single volley cannot kill a DS10 army: maximum damage is five.
        win,rounds=ground.solve(1,1)
        self.assertGreater(win,0.)
        self.assertLess(win,.5)
        self.assertGreater(rounds,2.)
        self.assertLess(rounds,10.)

    def test_seeded_trials_are_reproducible_and_count_every_outcome(self):
        args=('check',fleet.force(dd=3),fleet.force(ca=1),'X')
        first=fleet.run(*args,N=100)
        self.assertEqual(first,fleet.run(*args,N=100))
        self.assertAlmostEqual(sum(first['outcomes'].values()),1.)


if __name__=='__main__':
    unittest.main()
