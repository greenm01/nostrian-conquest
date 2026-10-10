"""Checks for the combat odds sweep: the current rule set reproduces the
review's mode C model exchange for exchange, exact and sampled results
agree, candidate semantics behave as documented, the exact solver stops at
the round cap, and runs reproduce.
Run with `python3 -I -B -m unittest discover -s tools/combat-model -p 'test_*.py'`.
"""
import json
import os
import random
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import ground  # noqa: E402
import nc_combat_analysis as legacy  # noqa: E402
import rules as R  # noqa: E402
import sweep  # noqa: E402
from confidence import binomial_interval, binomial_test, wilson_interval  # noqa: E402
from fleet_model import (  # noqa: E402
    CRIP_A, CRIP_B, LOSS_A, LOSS_B, P_A, P_B, P_MUTUAL, P_STALEMATE, ROUNDS, VIABLE_A, VIABLE_B, WD_A,
    Model, force,
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
    def test_vbam_rate_pair_changes_damage_rate_and_finishing_step_independently(self):
        full = model('vbam-full-ds', **{'units.CA.AS': 100})
        half = model('vbam-half-ds', **{'units.CA.AS': 100})
        expected = [10, 20, 20, 30, 30, 30, 30, 40, 40, 50]
        for m in (full, half):
            self.assertEqual([m.volley(force(CA=1), 100, roll)[0] for roll in range(10)], expected)
        # Same 45-hit budget: full DS leaves a cripple; half DS destroys it.
        self.assertEqual(full.apply(force(CA=1), 45, 0, False)[0][1][1][1], 1)
        self.assertEqual(half.apply(force(CA=1), 45, 0, False)[0][1][0][1], 0)

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

    def test_half_ds_finishes_a_crippled_unit_at_half_its_ds(self):
        # VBAM's damage step: a crippled CA (DS 30) dies to 15 more hits, so
        # 45 destroy a nominal one from either pool; the flat presets with
        # the full step need 60.
        m = model('flat-half-ds')
        crippled = ((0, 1, 0, 0, 0, 0, 0), (0, 1, 0, 0, 0, 0, 0), (0, 0))
        self.assertEqual(m.FIN[R.CLASSES.index('CA')], 15)
        self.assertEqual(m.KILL[R.CLASSES.index('DD')], 7)  # DS 5: half rounds down to 2
        self.assertEqual(m.apply(crippled, 15, 0, False)[0][1][0], (0,) * 7)
        self.assertEqual(m.apply(crippled, 14, 0, False)[0][1], crippled)
        self.assertEqual(m.apply(force(CA=1), 45, 0, False)[0][1][0], (0,) * 7)
        self.assertEqual(m.apply(force(CA=1), 44, 0, False)[0][1], crippled)
        self.assertEqual(m.apply(force(CA=1), 0, 45, False)[0][1][0], (0,) * 7)
        self.assertEqual(m.apply(force(CA=1), 0, 44, False)[0][1], force(CA=1))
        self.assertEqual(model('flat-discard').apply(force(CA=1), 45, 0, False)[0][1], crippled)
        self.assertEqual(model('flat-discard').apply(force(CA=1), 0, 59, False)[0][1], force(CA=1))
        # The residual roll prices the finishing step the same way.
        branches = dict((state[0], p) for p, state in model('flat-half-ds', remainder='residual').apply(crippled, 0, 3, False))
        self.assertAlmostEqual(branches[(0,) * 7], 0.2)

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

    def test_pursuit_and_screen_are_independent_switches(self):
        # The review's convoy ablation, exact: 5 CA + 10 TT at ROE 6 against
        # 10 CA under flat-carry. Without a pursuit the screen changes
        # nothing; a pursuit at the line costs cruisers, one that reaches
        # the auxiliaries first costs transports and the landing.
        convoy = (force(CA=5, TT=10), force(CA=10))
        transports = 11 + R.CLASSES.index('TT')
        cruisers = 11 + R.CLASSES.index('CA')
        runs = {(withdrawal, screen): model('flat-carry', withdrawal=withdrawal, screen=screen).solve(*convoy, roe=(6, 10))[0]
                for withdrawal in ('none', 'pursuit') for screen in ('absolute', 'conditional')}
        self.assertEqual(runs[('none', 'absolute')], runs[('none', 'conditional')])
        self.assertAlmostEqual(runs[('none', 'absolute')][transports], 4.0)
        self.assertAlmostEqual(runs[('none', 'absolute')][VIABLE_A], 0.6)
        self.assertAlmostEqual(runs[('pursuit', 'absolute')][transports], 4.324)
        self.assertAlmostEqual(runs[('pursuit', 'absolute')][cruisers], 3.812)
        self.assertAlmostEqual(runs[('pursuit', 'conditional')][transports], 6.007)
        self.assertAlmostEqual(runs[('pursuit', 'conditional')][cruisers], 2.408)
        self.assertAlmostEqual(runs[('pursuit', 'conditional')][VIABLE_A], 0.438)
        # Fewer build points lost, a worse landing: the measure the hull bill hides.
        self.assertLess(runs[('pursuit', 'conditional')][LOSS_A], runs[('pursuit', 'absolute')][LOSS_A])
        self.assertLess(runs[('pursuit', 'conditional')][VIABLE_A], runs[('pursuit', 'absolute')][VIABLE_A])
        self.assertAlmostEqual(runs[('pursuit', 'conditional')][VIABLE_B], 1.0)  # no transports: vacuous

    def test_current_roe_lock_holds_three_rounds_then_withdraws(self):
        # 10 CA at ROE 2 (needs 4:1) fail the check from the first exchange
        # but may not act on it before round 4: `step` keeps them in through
        # round 3 and, at round 4, every branch is the left side's
        # withdrawal after one more exchange. Over the battle that exchange
        # adds exactly one per withdrawal to the clean-break variant's count.
        m = model('current')
        a, b = force(CA=10), force(CA=10)
        for r in (1, 2, 3):
            for _, fought, nxt in m.step(a, b, r, (2, 10), m.exchange):
                self.assertEqual(fought, 1)
                if nxt[0] == 'state':
                    self.assertEqual(nxt[3], r + 1)
                else:
                    self.assertTrue(m.done(nxt[1], nxt[2]))
                    self.assertEqual(nxt[3:], (False, False))
        branches = m.step(a, b, 4, (2, 10), m.exchange)
        self.assertAlmostEqual(sum(p for p, _, _ in branches), 1.0)
        self.assertTrue(all(fought == 1 and nxt[0] == 'terminal' and nxt[3:] == (True, False)
                            for _, fought, nxt in branches))
        v, _ = m.solve(a, b, roe=(2, 10))
        clean, _ = model('current', withdrawal='none').solve(a, b, roe=(2, 10))
        self.assertAlmostEqual(v[WD_A], 0.1123, places=3)
        self.assertAlmostEqual(v[WD_A], clean[WD_A])
        self.assertAlmostEqual(v[ROUNDS] - clean[ROUNDS], v[WD_A])

    def test_unit_overrides_change_the_table(self):
        m = model('current', units='dd-10-8-5')
        self.assertEqual(m.DS[0], 8)
        rules, units = R.apply_overrides(R.load_rules('current'), R.load_units('baseline'), ['units.CA.DS=35', 'remainder="carry"'])
        self.assertEqual(units['CA']['DS'], 35)
        self.assertEqual(rules['remainder'], 'carry')
        with self.assertRaises(KeyError):
            R.apply_overrides(R.load_rules('current'), R.load_units('baseline'), ['nonsense=1'])


class RoundCap(unittest.TestCase):
    """`solve` counts rounds to the cap exactly, as `simulate` does. It used
    to drop the round from its state key and answer for an unlimited
    battle (Codex's review of t055)."""

    def test_one_round_cap_ends_an_undecided_duel_as_a_stalemate(self):
        # Two CA cannot finish each other in one exchange, so the capped
        # answer is a stalemate after it; the unlimited answer was 44.5%
        # a side, 11.1% mutual, 3.56 exchanges.
        v, _ = model('flat-carry', round_cap=1).solve(force(CA=1), force(CA=1))
        self.assertAlmostEqual(v[P_STALEMATE], 1.0)
        self.assertAlmostEqual(v[ROUNDS], 1.0)

    def test_last_ordinary_round_can_include_a_pursuit_exchange(self):
        m = model('flat-carry', round_cap=1)
        a, b = force(CA=10), force(CA=15)
        exact, _ = m.solve(a, b, roe=(6, 10))
        self.assertAlmostEqual(exact[ROUNDS], 2.0)
        self.assertAlmostEqual(exact[WD_A], 1.0)
        for seed in range(100):
            sampled = m.simulate(a, b, random.Random(seed), roe=(6, 10))
            self.assertEqual(sampled[ROUNDS], 2.0)
            self.assertEqual(sampled[WD_A], 1.0)

    def test_a_slow_duel_stops_at_the_cap_with_crippled_survivors(self):
        # CA with AS 2, DS 100 at constant effectiveness 1 under carry: 50
        # exchanges to cripple, 100 more to destroy. The cap stops it at 64
        # with both crippled; unlimited, it was mutual after 150.
        m = model('flat-carry', **{'effect.rows': [4] * 10, 'units.CA.AS': 2, 'units.CA.DS': 100})
        v, _ = m.solve(force(CA=1), force(CA=1))
        self.assertAlmostEqual(v[P_STALEMATE], 1.0)
        self.assertAlmostEqual(v[P_MUTUAL], 0.0)
        self.assertAlmostEqual(v[ROUNDS], 64.0)
        self.assertAlmostEqual(v[LOSS_A], 0.0)
        self.assertAlmostEqual(v[CRIP_A], 15.0)
        self.assertAlmostEqual(v[CRIP_B], 15.0)

    def test_a_stuck_state_counts_the_exchanges_up_to_the_cap(self):
        # Under discard two crippled CA (15 hits at most against DS 30)
        # sit at the cap in a quarter of duels; those paths fight 64
        # exchanges in all, not 64 more. Unlimited, the mean was 19.4375.
        v, _ = model('flat-discard').solve(force(CA=1), force(CA=1))
        self.assertAlmostEqual(v[P_STALEMATE], 0.25)
        self.assertAlmostEqual(v[ROUNDS], 19.046875)

    def test_exact_and_sampled_agree_at_small_caps(self):
        for cap in (1, 2, 3, 5):
            m = model('flat-carry', round_cap=cap)
            v, _ = m.solve(force(CA=2), force(CA=2))
            rng = random.Random(cap)
            trials = 2000
            samples = [m.simulate(force(CA=2), force(CA=2), rng) for _ in range(trials)]
            self.assertLessEqual(max(s[ROUNDS] for s in samples), cap)
            self.assertAlmostEqual(sum(s[ROUNDS] for s in samples) / trials, v[ROUNDS], delta=0.1)
            for index in (P_A, P_STALEMATE):
                count = int(sum(s[index] for s in samples))
                low, high = wilson_interval(count, trials, confidence=.999)
                self.assertTrue(low - 1e-9 <= v[index] <= high + 1e-9, (cap, index, v[index], low, high))


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

    def test_records_carry_transports_surviving(self):
        convoy = next(case for case in sweep.CASES if 'convoy' in case[4].split())
        record = sweep.run_case(model('flat-carry'), 'flat-carry', 'baseline', convoy, 10, 1, limit=100_000)
        self.assertEqual(record['method'], 'exact')
        self.assertAlmostEqual(record['surviving_left'], 10 - record['lost_left']['TT'])
        self.assertAlmostEqual(record['surviving_left'], 3.993)
        self.assertAlmostEqual(record['viable_left'], 0.438)
        self.assertEqual(record['surviving_right'], 0)
        self.assertIn('3.99 of 10 (43.8%)', sweep.surviving(record))

    def test_engine_comparison_checks_every_measure_and_reports_missing_cases(self):
        case = sweep.CASES[0]  # 10 CA vs 10 CA, tagged engine
        battles = [sweep.run_case(model('current'), 'current', 'baseline', case, 10, 1, limit=100_000)]
        engine = dict(cases=[
            dict(case=case[0], battles=20000, years_per_battle=1.2,
                 counts=dict(left=9980, right=9960, mutual=60, stalemate=0), loss_left=96.5, loss_right=96.4),
            dict(case='no battles', battles=0, years_per_battle=0.0,
                 counts=dict(left=0, right=0, mutual=0, stalemate=0), loss_left=0.0, loss_right=0.0),
            dict(case='30 DD vs 10 CA', battles=20000, years_per_battle=1.2,
                 counts=dict(left=889, right=19037, mutual=74, stalemate=0), loss_left=146.4, loss_right=86.5),
        ])
        with tempfile.TemporaryDirectory() as directory:
            path = os.path.join(directory, 'engine.json')
            with open(path, 'w') as handle:
                json.dump(engine, handle)
            rows = {row['case']: row['agree'] for row in sweep.compare_engine(path, battles, sweep.CASES)}
        self.assertEqual(rows[case[0]], 'yes')
        self.assertEqual(rows['no battles'], 'no battles')
        self.assertEqual(rows['30 DD vs 10 CA'], 'no model record')
        self.assertEqual(rows['11 CA vs 10 CA'], 'not in the engine file')

    def test_engine_comparison_names_the_measure_that_disagrees(self):
        case = sweep.CASES[0]
        battles = [sweep.run_case(model('current'), 'current', 'baseline', case, 10, 1, limit=100_000)]
        good = dict(case=case[0], battles=20000, years_per_battle=1.2,
                    counts=dict(left=9980, right=9960, mutual=60, stalemate=0), loss_left=96.5, loss_right=96.4)
        for change, expected in ((dict(counts=dict(left=9980, right=9820, mutual=200, stalemate=0)), 'NO: right win, mutual'),
                                 (dict(loss_right=90.0), 'NO: loss R'),
                                 (dict(counts=dict(left=10500, right=9440, mutual=60, stalemate=0)), 'NO: left win, right win')):
            with tempfile.TemporaryDirectory() as directory:
                path = os.path.join(directory, 'engine.json')
                with open(path, 'w') as handle:
                    json.dump(dict(cases=[dict(good, **change)]), handle)
                [row] = [r for r in sweep.compare_engine(path, battles, sweep.CASES) if r['case'] == case[0]]
            self.assertEqual(row['agree'], expected)

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
