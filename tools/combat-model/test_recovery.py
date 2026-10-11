"""Recovery timing and conservation, independent of combat win odds."""
import unittest
from recovery import (NOMINAL, begin_year, cripple, detach_crippled, end_year,
                      finish_crippled, force, lose_assistance, merge, timeline)


class RecoveryTiming(unittest.TestCase):
    def test_rt_two_observes_two_full_years_after_damage(self):
        self.assertEqual([r['nominal'] for r in timeline(2, False)], [0, 0, 1])

    def test_base_bonus_does_not_heal_fresh_damage_even_for_rt_one(self):
        for rt in range(1, 5):
            state = cripple(begin_year(force(1), at_base=True), 1, rt)
            state, healed = end_year(state)
            self.assertEqual(healed, 0)
            self.assertEqual(state, force(damaged=[(rt, 1)]))

    def test_assistance_timing(self):
        for rt in range(1, 5):
            rows = timeline(rt, True)
            recovery_year = next(r['year_after_damage'] for r in rows if r['nominal'])
            self.assertEqual(recovery_year, (rt + 1) // 2)

    def test_leaving_base_keeps_ordinary_recovery(self):
        state = lose_assistance(begin_year(force(damaged=[(2, 3)]), at_base=True))
        self.assertEqual(end_year(state), (force(damaged=[(1, 3)]), 0))

    def test_late_merge_cannot_give_arrivals_assistance(self):
        resident = begin_year(force(damaged=[(2, 3)]), at_base=True)
        arriving = begin_year(force(damaged=[(2, 4)]), at_base=False)
        state, healed = end_year(merge(resident, arriving))
        self.assertEqual(healed, 3)
        self.assertEqual(state, force(3, [(1, 4)]))

    def test_split_merge_preserves_every_counter_and_eligibility(self):
        state = cripple(begin_year(force(2, [(1, 3), (2, 4)]), at_base=True), 1, 2)
        kept, detached = detach_crippled(state, 3)
        self.assertEqual(merge(kept, detached), state)
        whole, healed = end_year(state)
        a, ha = end_year(kept)
        b, hb = end_year(detached)
        self.assertEqual(merge(a, b), whole)
        self.assertEqual(ha + hb, healed)

    def test_finishing_fresh_hull_leaves_survivors_closest_to_recovery(self):
        state = cripple(force(1, [(2, 1), (1, 1)]), 1, 2)
        self.assertEqual(finish_crippled(state, 1), force(damaged=[(2, 1), (1, 1)]))

    def test_no_counter_reset_without_new_crippling(self):
        state = force(damaged=[(1, 2)])
        self.assertEqual(cripple(state, 0, 3), state)

    def test_fresh_flag_matches_rt_plus_one_for_ordinary_recovery(self):
        for rt in range(1, 5):
            for fresh in (False, True):
                state = cripple(force(1), 1, rt) if fresh else force(damaged=[(rt, 1)])
                result, _ = end_year(state)
                remaining = max(0, rt + int(fresh) - 1)
                expected = force(1) if remaining == 0 else force(damaged=[(remaining, 1)])
                self.assertEqual(result, expected)


if __name__ == '__main__':
    unittest.main()
