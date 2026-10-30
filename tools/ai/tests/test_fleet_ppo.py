"""Fleet PPO pieces that need no games."""

import math
import sys
import unittest
from pathlib import Path

import torch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import numpy as np  # noqa: E402

from fleet_ppo import League, act_entropy, choose_lineups, place_starts, place_stored, categorical_kl, coverage, ema_toward, lead_potential, smoothed_kl, upgo_advantages  # noqa: E402


class ActEntropyTest(unittest.TestCase):
    def test_counts_only_whether_the_head_acts(self):
        # Acting half the time is the most uncertain, however many targets
        # share the other half.
        half = torch.distributions.Categorical(probs=torch.tensor([0.5, 0.1, 0.1, 0.1, 0.1, 0.1]))
        self.assertAlmostEqual(float(act_entropy(half)), math.log(2), places=5)
        # A uniform choice over six options acts five times in six, which is
        # less uncertain about acting than half and half.
        uniform = torch.distributions.Categorical(probs=torch.full((6,), 1 / 6))
        self.assertLess(float(act_entropy(uniform)), float(act_entropy(half)))

    def test_a_head_that_can_only_decline_has_none(self):
        only_none = torch.distributions.Categorical(
            logits=torch.tensor([0.0, float("-inf"), float("-inf")]))
        self.assertLess(float(act_entropy(only_none)), 1e-4)



def masked(logits):
    """A categorical over ``logits``, with ``None`` marking an illegal choice."""
    return torch.distributions.Categorical(
        logits=torch.tensor([float("-inf") if x is None else x for x in logits]))


class MissionSmoothingTest(unittest.TestCase):
    def test_every_legal_mission_keeps_a_share_of_the_reference(self):
        # The copy gives the second legal mission almost nothing; blended
        # with 0.05 spread over three legal choices, it keeps at least 0.05/3.
        reference = masked([5.0, -12.0, 0.0, None])
        new = masked([0.0, 0.0, 0.0, None])
        smooth = 0.05
        kl = float(smoothed_kl(new, reference, smooth))
        blended = (1 - smooth) * reference.probs + smooth * torch.tensor([1, 1, 1, 0]) / 3
        expected = float((new.probs[:3] * (new.probs[:3] / blended[:3]).log()).sum())
        self.assertAlmostEqual(kl, expected, places=5)
        self.assertGreater(float(blended[1]), smooth / 3 - 1e-6)
        self.assertEqual(float(blended[3]), 0.0)

    def test_smoothing_makes_a_rare_mission_cheaper_to_try(self):
        reference = masked([4.0, -10.0, None])
        tried = masked([0.0, math.log(0.05 / 0.95), None])
        self.assertLess(float(smoothed_kl(tried, reference, 0.05)),
                        float(categorical_kl(tried, reference)) / 2)

    def test_no_smoothing_is_the_plain_kl(self):
        reference = masked([1.0, -2.0, 0.5, None])
        new = masked([0.0, 0.3, -1.0, None])
        self.assertAlmostEqual(float(smoothed_kl(new, reference, 0.0)),
                               float(categorical_kl(new, reference)), places=5)


class CoverageTest(unittest.TestCase):
    def test_lifts_rare_missions_and_leaves_common_ones(self):
        logits = torch.tensor([3.0, -6.0, -6.0, float("-inf")], requires_grad=True)
        dist = torch.distributions.Categorical(logits=logits)
        coverage(dist, 0.03).backward()
        # The rare legal missions are pushed up, the illegal one untouched.
        self.assertGreater(float(logits.grad[1]), 0)
        self.assertGreater(float(logits.grad[2]), 0)
        self.assertEqual(float(logits.grad[3]), 0.0)

    def test_no_gradient_once_every_legal_mission_is_above_the_floor(self):
        logits = torch.tensor([0.0, 0.2, -0.3, float("-inf")], requires_grad=True)
        coverage(torch.distributions.Categorical(logits=logits), 0.03).backward()
        self.assertTrue(torch.allclose(logits.grad, torch.zeros(4)))

class EmaTowardTest(unittest.TestCase):
    def test_moves_float_weights_a_share_of_the_way_and_leaves_counts(self):
        ref, model = torch.nn.BatchNorm1d(2), torch.nn.BatchNorm1d(2)
        with torch.no_grad():
            model.weight.fill_(3.0)
        model.num_batches_tracked.fill_(7)
        ema_toward(ref, model, 0.25)
        self.assertTrue(torch.allclose(ref.weight, torch.full((2,), 1.5)))
        self.assertEqual(int(ref.num_batches_tracked), 0)


class LeagueShareTest(unittest.TestCase):
    def test_only_the_league_share_of_games_meets_the_league(self):
        league = League(["executor@rusher"], [], "hybrid:strike", 3, share=0.25)
        names, _, opponents = choose_lineups(np.random.default_rng(0), 400, "hybrid:strike",
                                             league=league)
        drawn = opponents >= 0
        self.assertTrue(0.15 < drawn.mean() < 0.35)
        for lineup, from_league in zip(names, drawn):
            self.assertEqual("executor@rusher" in lineup, from_league)


class PlaceStartsTest(unittest.TestCase):
    def test_replays_take_the_first_games_and_hand_over_before_the_fall(self):
        names, roles = choose_lineups(np.random.default_rng(0), 4, "hybrid:strike")
        maps = [10, 11, 12, 13]
        starts = [{"map": 100504, "seat": 2, "year": 35}]
        handover = place_starts(np.random.default_rng(0), starts, 2, "hybrid:strike", 5,
                                names, roles, maps)
        self.assertEqual(maps, [100504, 100504, 12, 13])
        self.assertEqual(handover.tolist(), [33, 33, 0, 0])
        for game in (0, 1):
            self.assertEqual(names[game], ["executor", "executor", "hybrid:strike", "executor"])
            self.assertEqual(roles[game].tolist(), [0, 0, 5, 0])

    def test_the_lead_sets_the_handover_and_never_goes_before_year_zero(self):
        names, roles = choose_lineups(np.random.default_rng(0), 2, "hybrid:strike")
        starts = [{"map": 1, "seat": 0, "year": 8}]
        handover = place_starts(np.random.default_rng(0), starts, 1, "hybrid:strike", 5,
                                names, roles, [10, 11], lead=10)
        self.assertEqual(handover.tolist(), [0, 0])
        handover = place_starts(np.random.default_rng(0), starts, 1, "hybrid:strike", 5,
                                names, roles, [10, 11], lead=5)
        self.assertEqual(handover.tolist(), [3, 0])


class PlaceStoredTest(unittest.TestCase):
    def test_stored_games_take_the_first_games_with_the_learner_in_the_seat(self):
        names, roles = choose_lineups(np.random.default_rng(0), 3, "hybrid:strike")
        stored = [{"db": "/games/crunch-ai-2026-10-30.db", "year": 3040, "seat": 0,
                   "human": True}]
        starts = place_stored(np.random.default_rng(0), stored, 2, "hybrid:strike", names, roles)
        self.assertEqual(starts, [("/games/crunch-ai-2026-10-30.db", 3040)] * 2 + [None])
        for game in (0, 1):
            self.assertEqual(names[game], ["hybrid:strike", "executor", "executor", "executor"])
            self.assertEqual(roles[game].tolist(), [1, 0, 0, 0])


if __name__ == "__main__":
    unittest.main()


class LeadPotentialTest(unittest.TestCase):
    def test_is_the_share_lead_and_zero_without_worlds_or_after_the_game(self):
        production = np.array([[40, 20, 20, 20], [0, 30, 30, 40]])
        worlds = np.array([[3, 2, 2, 2], [0, 2, 2, 3]])
        phi = lead_potential(production, worlds, np.array([False, False]))
        self.assertAlmostEqual(float(phi[0, 0]), 0.4 - 0.2, places=6)
        self.assertAlmostEqual(float(phi[0, 1]), 0.2 - 0.8 / 3, places=6)
        self.assertEqual(float(phi[1, 0]), 0.0)
        self.assertFalse(lead_potential(production, worlds, np.array([True, False]))[0].any())

    def test_shaping_over_a_game_does_not_depend_on_the_play(self):
        # Undiscounted, the terms telescope to phi(end) - phi(start) = 0 for
        # every trajectory; with discounting each step's term is unchanged.
        rng = np.random.default_rng(1)
        for _ in range(5):
            years = 12
            production = rng.integers(1, 50, (years, 1, 4))
            phi, total = np.zeros((1, 4), np.float32), np.zeros((1, 4))
            for t in range(years):
                done = np.array([t == years - 1])
                phi_next = lead_potential(production[t], np.ones((1, 4)), done)
                total += phi_next - phi
                phi = phi_next
            np.testing.assert_allclose(total, 0.0, atol=1e-6)


class UpgoTest(unittest.TestCase):
    def step(self, reward, value, done):
        select = np.array([[True]])
        return {"select": select, "reward": np.array([reward], np.float32),
                "value": torch.tensor([value]), "done": np.array([done])}

    def test_follows_a_continuation_that_beat_the_critic(self):
        # Year 2 pays 1 where the critic expected 0.2, so year 1 takes the
        # actual return rather than the critic's 0.2, and so does year 0.
        steps = [self.step(0.0, 0.3, False), self.step(0.0, 0.2, False), self.step(1.0, 0.2, True)]
        out = upgo_advantages(steps, 1.0)
        np.testing.assert_allclose([o[0] for o in out], [0.7, 0.8, 0.8], atol=1e-6)

    def test_is_zero_when_the_game_went_worse_than_expected(self):
        steps = [self.step(0.0, 0.5, False), self.step(0.0, 0.4, True)]
        out = upgo_advantages(steps, 1.0)
        # Year 0 bootstraps from the critic's 0.4, below its own 0.5.
        np.testing.assert_allclose([o[0] for o in out], [0.0, 0.0], atol=1e-6)
