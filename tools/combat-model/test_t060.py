"""Implemented-rule probes that leave CRT-era evidence unchanged."""
import io
import json
import unittest
from unittest.mock import patch
import rules as R
import sweep
from fleet_model import Model, force


class ImplementedFleetRules(unittest.TestCase):
    def test_both_sides_withdrawing_before_round_four_still_exchange_once(self):
        m = Model(R.load_rules('engine-t060'), R.UNITS)
        a, b = force(CA=3), force(CA=3)
        calls = []
        def exchange(a, b, **kw):
            calls.append(kw)
            return [(1.0, (a, b))]
        branches = m.step(a, b, 4, (0, 0), exchange)
        self.assertEqual(calls, [dict(pressed=True)])
        self.assertEqual(branches, [(1.0, 1, ('terminal', a, b, True, True))])

    def test_critical_class_order_is_fixed_and_ignores_screen(self):
        units = R.load_units('baseline')
        units['DD']['DS'] = 999
        m = Model(R.load_rules('engine-t060'), units)
        _, side = m.apply(force(DD=1, SC=1, CA=1), 0, 0, True)[0]
        self.assertEqual(side[0][R.CLASSES.index('DD')], 0)
        _, side = m.apply(force(SC=1, CA=1), 0, 0, True)[0]
        self.assertEqual(side[0][R.CLASSES.index('SC')], 0)
        self.assertEqual(side[0][R.CLASSES.index('CA')], 1)

    def test_withdrawal_uses_unmodified_flat_table(self):
        m = Model(R.load_rules('engine-t060'), R.UNITS)
        side = force(DD=1, CA=1, SB=1)
        self.assertEqual(sum(m.volley(side, 100, 1)[:2]), 70)
        self.assertEqual(sum(m.volley(side, 100, 1, pressed=True)[:2]), 35)
        self.assertFalse(m.volley(side, 100, 8)[2])
        self.assertTrue(m.volley(side, 100, 9)[2])

    def test_comparison_selects_named_preset(self):
        record = dict(ruleset='engine-t060', units='baseline', case='probe', method='exact',
                      left=dict(CA=1), right=dict(CA=1), loss_left=15, loss_right=0,
                      p=dict(A=0, B=1, mutual=0, stalemate=0))
        engine = dict(cases=[dict(case='probe', battles=100, counts=dict(left=0, right=100,
                      mutual=0, stalemate=0), loss_left=15, loss_right=0, years_per_battle=1)])
        with patch('builtins.open', return_value=io.StringIO(json.dumps(engine))):
            [row] = sweep.compare_engine('unused', [record], [], 'engine-t060')
        self.assertEqual(row['agree'], 'yes')
        with patch('builtins.open', return_value=io.StringIO(json.dumps(engine))):
            [row] = sweep.compare_engine('unused', [record], [])
        self.assertEqual(row['agree'], 'no model record')


if __name__ == '__main__':
    unittest.main()
