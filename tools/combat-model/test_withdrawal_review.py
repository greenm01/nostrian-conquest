"""Regression probes for t062's isolated candidates."""
import unittest

import rules as R
from fleet_model import Model, WD_A, force
from withdrawal_review import BASE, PursuitModel


class WithdrawalCandidates(unittest.TestCase):
    def test_zero_as_escape_is_opt_in_even_at_roe_ten(self):
        ordinary = Model(BASE, R.UNITS)
        enabled = Model(R.variant(BASE, 'escape', unarmed_escape=True), R.UNITS)
        self.assertFalse(ordinary.wants_withdrawal(force(TT=2), 30, 0))
        self.assertTrue(enabled.wants_withdrawal(force(TT=2), 30, 10))
        self.assertFalse(enabled.wants_withdrawal(force(), 30, 0))

    def test_unarmed_escape_still_takes_the_compulsory_exchange(self):
        rules = R.variant(BASE, 'escape', roe='after1', unarmed_escape=True, withdrawal='none')
        m = Model(rules, R.UNITS)
        result, _ = m.solve(force(TT=10), force(DD=1), (0, 10))
        self.assertAlmostEqual(result[4], 1)
        self.assertAlmostEqual(result[WD_A], 1)

    def test_pursuit_probability_is_an_exact_mixture(self):
        a, b = force(CA=2, TT=4), force(CA=3)
        rules = R.variant(BASE, 'pursuit', withdrawal='pursuit', pursuit_probability=0.5)
        m = PursuitModel(rules, R.UNITS)
        branches = m._leave(a, b, True, False, 0, m.exchange)
        self.assertAlmostEqual(sum(p for p, _, _ in branches), 1)
        self.assertAlmostEqual(sum(p for p, n, _ in branches if n == 0), 0.5)
        self.assertAlmostEqual(sum(p for p, n, _ in branches if n == 1), 0.5)

    def test_both_withdrawing_sides_do_not_pursue_each_other(self):
        m = PursuitModel(R.variant(BASE, 'pursuit', withdrawal='pursuit'), R.UNITS)
        a, b = force(CA=2), force(CA=3)
        self.assertEqual(m._leave(a, b, True, True, 0, m.exchange),
                         [(1, 0, ('terminal', a, b, True, True))])

    def test_approved_baseline_retains_current_modifiers_and_critical(self):
        m = Model(BASE, R.UNITS)
        mixed = force(CA=1, DD=1)
        self.assertEqual(m.volley(mixed, 30, 2)[:2], (15, 5))
        self.assertEqual(m.volley(mixed, 30, 2, pressed=True)[:2], (8, 2))
        self.assertTrue(m.volley(mixed, 30, 9)[2])
        self.assertEqual(BASE['critical'], 'free-kill')
        self.assertEqual(BASE['base_condition'], 'present')


if __name__ == '__main__':
    unittest.main()
