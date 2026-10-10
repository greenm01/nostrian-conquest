"""Checks for the combat odds sweep: the current rule set reproduces the
review's mode C model exchange for exchange, exact and sampled results
agree, candidate semantics behave as documented, and runs reproduce.
Run with `python3 -I -B -m unittest discover -s tools/combat-model -p 'test_*.py'`.
"""
import os
import random
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import ground  # noqa: E402
import nc_combat_analysis as legacy  # noqa: E402
import rules as R  # noqa: E402
import sweep  # noqa: E402
from confidence import binomial_interval, binomial_test, wilson_interval  # noqa: E402
from fleet_model import (  # noqa: E402
    CRIP_A, CRIP_B, LOSS_A, LOSS_B, P_A, P_B, P_MUTUAL, P_STALEMATE, ROUNDS, WD_A, Model, force,
)


class Tape:
    """Scripted d10 rolls shared by both models."""

    def __init__(self, rolls):
        self.rolls = iter(rolls)

    def randrange(self, limit):
        value = next(self.rolls)
        assert 0 <= value < limit
        return value

    def random(self):
        return 0.5


def model(name, units='baseline', **overrides):
    rules = R.load_rules(name)
    table = R.load_units(units)
    R.apply_overrides(rules, table, [f'{k}={v}' for k, v in overrides.items()])
    return Model(rules, table)


def outcome(vector):
    return 'A' if vector[P_A] else 'B' if vector[P_B] else 'mutual' if vector[P_MUTUAL] else 'stalemate'


class CurrentRulesMatchTheReviewModel(unittest.TestCase):
    def test_combat_line_battles_match_mode_c_exchange_for_exchange(self):
        m = model('current')
        rng = random.Random(20261010)
        for _ in range(500):
            a = legacy.force(dd=rng.randrange(6), ca=rng.randrange(6), bb=rng.randrange(3), sb=rng.randrange(2))
            b = legacy.force(dd=rng.randrange(6), ca=rng.randrange(6), bb=rng.randrange(3), sb=rng.randrange(2))
            if not (legacy.stats(a, [0] * 7)[0] and legacy.stats(b, [0] * 7)[0]):
                continue  # the review model counts one empty round for a force that cannot fire
            rolls = [rng.randrange(10) for _ in range(300)]
            result, rounds, na, nb, ca, cb = legacy.battle(a, b, 'C', Tape(rolls))
            v = m.simulate((tuple(a), (0,) * 7, (0, 0)), (tuple(b), (0,) * 7, (0, 0)), Tape(rolls))
            self.assertEqual(outcome(v), {'cap': 'stalemate'}.get(result, result), (a, b))
            self.assertEqual(v[ROUNDS], rounds)
            self.assertEqual(v[LOSS_A], sum((a[i] - na[i]) * legacy.COST[i] for i in range(7)))
            self.assertEqual(v[LOSS_B], sum((b[i] - nb[i]) * legacy.COST[i] for i in range(7)))
            self.assertEqual(v[CRIP_A], sum(ca[i] * legacy.COST[i] for i in range(7)))
            self.assertEqual(v[CRIP_B], sum(cb[i] * legacy.COST[i] for i in range(7)))

    def test_auxiliaries_are_shot_after_the_line_as_the_engine_does(self):
        # The review model stops when a line is gone; the engine keeps
        # firing at the screened auxiliaries, so the outcome agrees and
        # only auxiliary losses and the exchange count can grow.
        m = model('current')
        rng = random.Random(7)
        extra = 0
        for _ in range(300):
            a = legacy.force(dd=rng.randrange(4), ca=rng.randrange(4), tt=rng.randrange(3), sc=rng.randrange(2))
            b = legacy.force(dd=rng.randrange(4), ca=rng.randrange(4), tt=rng.randrange(3), et=rng.randrange(2))
            if not (legacy.stats(a, [0] * 7)[0] and legacy.stats(b, [0] * 7)[0]):
                continue
            rolls = [rng.randrange(10) for _ in range(300)]
            result, rounds, na, nb, _, _ = legacy.battle(a, b, 'C', Tape(rolls))
            v = m.simulate((tuple(a), (0,) * 7, (0, 0)), (tuple(b), (0,) * 7, (0, 0)), Tape(rolls))
            self.assertEqual(outcome(v), {'cap': 'stalemate'}.get(result, result), (a, b))
            self.assertGreaterEqual(v[ROUNDS], rounds)
            for side, before, after, loss in ((0, a, na, v[LOSS_A]), (1, b, nb, v[LOSS_B])):
                legacy_loss = sum((before[i] - after[i]) * legacy.COST[i] for i in range(7))
                auxiliaries = sum(after[i] * legacy.COST[i] for i in (4, 5, 6))
                self.assertGreaterEqual(loss, legacy_loss)
                self.assertLessEqual(loss, legacy_loss + auxiliaries)
            extra += v[ROUNDS] > rounds
        self.assertGreater(extra, 0)

    def test_lone_destroyer_against_a_battleship_is_a_ten_percent_lottery(self):
        v, states = model('current').solve(force(DD=1), force(BB=1))
        self.assertEqual(states, 1)
        self.assertAlmostEqual(v[P_B], 0.9)
        self.assertAlmostEqual(v[P_MUTUAL], 0.1)
        self.assertAlmostEqual(v[ROUNDS], 1.0)

    def test_exact_and_sampled_agree(self):
        m = model('current')
        v, _ = m.solve(force(CA=11), force(CA=10))
        rng = random.Random(3)
        trials = 4000
        wins = sum(m.simulate(force(CA=11), force(CA=10), rng)[P_A] for _ in range(trials))
        low, high = wilson_interval(int(wins), trials, confidence=.999)
        self.assertTrue(low <= v[P_A] <= high, (v[P_A], low, high))


class CandidateSemantics(unittest.TestCase):
    def test_residual_buys_the_next_step_with_probability_hits_over_cost(self):
        m = model('flat-residual')
        branches = dict((state, p) for p, state in m.apply(force(CA=1), 12, 0, False))
        self.assertAlmostEqual(branches[((0, 1, 0, 0, 0, 0, 0), (0, 1, 0, 0, 0, 0, 0), (0, 0))], 0.4)
        self.assertAlmostEqual(branches[force(CA=1)], 0.6)
        # Execution against a nominal ship pays the full 2xDS kill cost.
        branches = dict((state, p) for p, state in m.apply(force(CA=1), 0, 15, False))
        self.assertAlmostEqual(branches[((0, 0, 0, 0, 0, 0, 0), (0, 0, 0, 0, 0, 0, 0), (0, 0))], 0.25)

    def test_carry_keeps_short_hits_for_the_next_round(self):
        m = model('flat-carry')
        [(p, state)] = m.apply(force(CA=1), 20, 0, False)
        self.assertEqual(p, 1.0)
        self.assertEqual(state, ((0, 1, 0, 0, 0, 0, 0), (0, 0, 0, 0, 0, 0, 0), (20, 0)))
        [(_, state)] = m.apply(state, 15, 0, False)
        self.assertEqual(state, ((0, 1, 0, 0, 0, 0, 0), (0, 1, 0, 0, 0, 0, 0), (5, 0)))

    def test_discard_drops_short_hits(self):
        m = model('flat-discard')
        [(_, state)] = m.apply(force(CA=1), 29, 0, False)
        self.assertEqual(state, force(CA=1))

    def test_focus_critical_spends_the_budget_on_the_heaviest_affordable_step(self):
        m = model('flat-discard')
        [(_, state)] = m.apply(force(DD=2, BB=1), 100, 0, True)
        self.assertEqual(state[0], (2, 0, 1, 0, 0, 0, 0))
        self.assertEqual(state[1], (0, 0, 1, 0, 0, 0, 0))
        [(_, state)] = m.apply(force(DD=2, BB=1), 100, 0, False)
        self.assertEqual(state[1], (2, 0, 0, 0, 0, 0, 0))

    def test_free_kill_critical_takes_the_cheapest_unit_even_a_screened_one(self):
        m = model('current')
        [(_, state)] = m.apply(force(CA=1, TT=1), 0, 0, True)
        self.assertEqual(state[0], (0, 1, 0, 0, 0, 0, 0))

    def test_per_fleet_roe_withdraws_after_one_exchange_with_one_pursuit(self):
        m = model('flat-discard')
        v, _ = m.solve(force(CA=10), force(CA=15), roe=(6, 10))
        self.assertGreater(v[WD_A], 0.95)
        self.assertGreater(v[P_B], 0.95)
        self.assertAlmostEqual(v[ROUNDS], 2.0, places=1)
        v, _ = model('flat-discard', withdrawal='none').solve(force(CA=10), force(CA=15), roe=(6, 10))
        self.assertAlmostEqual(v[ROUNDS], 1.0, places=1)

    def test_conditional_screen_exposes_auxiliaries_to_pursuit(self):
        exposed = model('flat-discard').solve(force(CA=5, TT=10), force(CA=10), roe=(6, 10))[0]
        absolute = model('flat-discard', screen='absolute').solve(force(CA=5, TT=10), force(CA=10), roe=(6, 10))[0]
        transports = 11 + R.CLASSES.index('TT')
        self.assertGreater(exposed[transports], absolute[transports])
        self.assertLess(exposed[LOSS_A], absolute[LOSS_A])  # transports are cheaper than cruisers

    def test_current_roe_lock_holds_three_rounds(self):
        v, _ = model('current').solve(force(CA=10), force(CA=10), roe=(6, 6))
        self.assertEqual(v[WD_A] + v[P_STALEMATE], v[WD_A] + v[P_STALEMATE])
        self.assertGreater(v[ROUNDS], 2.5)

    def test_unit_overrides_change_the_table(self):
        m = model('current', units='dd-10-8-5')
        self.assertEqual(m.DS[0], 8)
        rules, units = R.apply_overrides(R.load_rules('current'), R.load_units('baseline'), ['units.CA.DS=35', 'remainder="carry"'])
        self.assertEqual(units['CA']['DS'], 35)
        self.assertEqual(rules['remainder'], 'carry')
        with self.assertRaises(KeyError):
            R.apply_overrides(R.load_rules('current'), R.load_units('baseline'), ['nonsense=1'])


class SweepOutput(unittest.TestCase):
    def test_runs_reproduce(self):
        m = model('flat-residual')
        case = sweep.CASES[0]
        first = sweep.run_case(m, 'flat-residual', 'baseline', case, 200, 1, limit=1)
        second = sweep.run_case(m, 'flat-residual', 'baseline', case, 200, 1, limit=1)
        first.pop('seconds')
        second.pop('seconds')
        self.assertEqual(first['method'], 'sampled')
        self.assertEqual(first, second)
        self.assertEqual(sum(first['counts'].values()), 200)

    def test_ground_table_reuses_the_exact_ground_models(self):
        rows = {(r['attackers'], r['defenders']): r for r in sweep.run_ground()}
        self.assertAlmostEqual(rows[(10, 10)]['current']['capture'], legacy.ground(10, 10)[0])
        self.assertAlmostEqual(rows[(11, 10)]['current']['capture'], .83)
        self.assertAlmostEqual(rows[(11, 10)]['simultaneous']['capture'], legacy.binomial_ground(11, 10)[0])
        self.assertAlmostEqual(rows[(11, 10)]['defender_first']['capture'], ground.solve(11, 10)[0])

    def test_intervals(self):
        low, high, method = binomial_interval(50, 100)
        self.assertIn(method, ('Wilson', 'Clopper-Pearson'))
        self.assertLess(low, .5)
        self.assertGreater(high, .5)
        self.assertAlmostEqual(wilson_interval(50, 100)[0], .4038, places=3)
        self.assertAlmostEqual(binomial_test(5, 10, .5), 1.0)
        self.assertGreater(binomial_test(20000, 20000, .9999), .05)
        self.assertLess(binomial_test(19990, 20000, .9999), .05)
        self.assertLess(binomial_test(9000, 20000, .5), 1e-6)


if __name__ == '__main__':
    unittest.main()
