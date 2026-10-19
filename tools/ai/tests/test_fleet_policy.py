"""Fleet-agent pieces that need no games."""

import math
import sys
import unittest
from pathlib import Path

import torch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from fleet_policy import greedy_join  # noqa: E402


class GreedyJoinTest(unittest.TestCase):
    def test_joins_when_joining_at_all_is_likelier_than_none(self):
        # A plain argmax would pick none at 0.4 against six hosts at 0.1.
        spread = torch.tensor([0.4, 0.1, 0.1, 0.1, 0.1, 0.15, 0.05]).log()
        self.assertEqual(int(greedy_join(spread)), 5)

    def test_declines_when_none_is_at_least_half(self):
        half = torch.tensor([0.5, 0.3, 0.2]).log()
        self.assertEqual(int(greedy_join(half)), 0)

    def test_a_row_with_no_host_declines(self):
        alone = torch.tensor([[0.0, -math.inf, -math.inf]])
        self.assertEqual(greedy_join(alone).tolist(), [0])


if __name__ == "__main__":
    unittest.main()
