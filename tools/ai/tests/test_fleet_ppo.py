"""Fleet PPO pieces that need no games."""

import math
import sys
import unittest
from pathlib import Path

import torch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from fleet_ppo import act_entropy  # noqa: E402


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


if __name__ == "__main__":
    unittest.main()
