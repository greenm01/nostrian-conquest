"""Fleet-agent pieces: those that need no games, and the build head on one
small batch of games."""

import math
import sys
import unittest
from pathlib import Path

import numpy as np
import torch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import nc_ai_env  # noqa: E402
from fleet_policy import FleetAgent, bucket, build_arrays, fleet_batch, greedy_join  # noqa: E402


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


class BuildHeadTest(unittest.TestCase):
    def setUp(self):
        torch.manual_seed(0)
        self.env = nc_ai_env.Env(4, 10)
        self.env.reset([1, 2], seats=[["hybrid:builds", "executor", "executor", "executor"]] * 2)
        self.select = self.env.hybrid.copy()
        masks = self.env.masks()
        self.allowed = masks["build"][self.select]
        self.batch = bucket(fleet_batch(self.env.observe(), masks, self.env.goal_masks(),
                                        self.select, torch.device("cpu")))
        self.model = FleetAgent()

    def test_starts_as_the_executor_and_only_builds_what_the_mask_allows(self):
        with torch.no_grad():
            goals, *_ = self.model.act(self.batch, greedy=True, builds=True)
        executor = nc_ai_env.BUILD_EXECUTOR
        owned = self.allowed[:, :, executor]
        self.assertTrue(owned.any())
        chosen = goals["build"][:, : self.env.worlds].numpy()
        self.assertTrue((chosen[owned] == executor).all())
        self.assertTrue((chosen[~owned] == 0).all())
        with torch.no_grad():
            for _ in range(20):
                goals, _, goal_lp, *_ = self.model.act(self.batch, builds=True)
                self.assertTrue(torch.isfinite(goal_lp).all())
                chosen = goals["build"][:, : self.env.worlds].numpy()
                taken = np.take_along_axis(self.allowed, chosen, -1)
                self.assertTrue(taken.all())

    def test_greedy_builds_when_building_is_likelier_than_none(self):
        # Each kind is less likely than none (or the unlikely executor), but
        # three or more together are likelier, so greedy builds where at
        # least three are allowed.
        with torch.no_grad():
            bias = self.model.world_build.bias.view(nc_ai_env.BUILD_SLOTS, -1)
            bias[:, 0], bias[:, 1:] = 2.0, 1.0
            bias[:, nc_ai_env.BUILD_EXECUTOR] = -20.0
            goals, *_ = self.model.act(self.batch, greedy=True, builds=True)
        kinds = self.allowed[:, :, 1:nc_ai_env.BUILD_EXECUTOR].sum(-1)
        built = goals["build"][:, : self.env.worlds, 0].numpy() > 0
        self.assertTrue(kinds.max() >= 3)
        self.assertTrue((built == (kinds >= 3)).all())

    def test_the_env_takes_the_heads_builds(self):
        with torch.no_grad():
            goals, *_ = self.model.act(self.batch, builds=True)
        arrays = build_arrays(goals, self.select, self.env.worlds)
        shape = (2, 4, self.env.worlds, nc_ai_env.BUILD_SLOTS)
        self.assertEqual(arrays["build"].shape, shape)
        self.assertEqual(arrays["amount"].shape, shape)


if __name__ == "__main__":
    unittest.main()
