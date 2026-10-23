"""Fleet PPO pieces that need no games."""

import math
import sys
import unittest
from pathlib import Path

import torch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from fleet_ppo import act_entropy, categorical_kl, coverage, ema_toward, smoothed_kl  # noqa: E402


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


if __name__ == "__main__":
    unittest.main()
