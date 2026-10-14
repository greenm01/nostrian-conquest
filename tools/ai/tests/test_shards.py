"""Shards written from random play read back with every stored choice inside
its stored mask, and the rows match what the environment showed."""

import sys
import tempfile
import unittest
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import nc_ai_env  # noqa: E402
from random_agent import random_actions  # noqa: E402
from shards import ShardWriter, experience_batch, read_shards, rows  # noqa: E402

PLAYERS = 4


class ShardRoundTrip(unittest.TestCase):
    def test_choices_and_tables_survive_the_round_trip(self):
        rng = np.random.default_rng(3)
        map_seeds = np.array([41, 42, 43], dtype=np.uint64)
        campaign_seeds = map_seeds + np.uint64(1)
        env = nc_ai_env.Env(PLAYERS, 6, 2)
        env.reset(map_seeds.tolist(), campaign_seeds.tolist())
        done = np.zeros(len(map_seeds), bool)
        seen_fleets = seen_planets = 0
        with tempfile.TemporaryDirectory() as out:
            writer = ShardWriter(out, rows_per_shard=30)
            while not done.all():
                obs, masks = env.observe(), env.masks()
                actions = random_actions(env, masks, rng)
                step = env.step(actions)
                writer.add(experience_batch(
                    map_seeds, campaign_seeds, env.learned, done, obs, masks,
                    actions, step))
                seen_fleets += int(obs["fleets_valid"][~done].sum())
                seen_planets += int(obs["planets_valid"][~done].sum())
                done = step["done"].copy()
            writer.close()
            self.assertGreater(writer.shards, 1)
            table = read_shards(out)

        self.assertEqual(table.num_rows, 6 * len(map_seeds) * PLAYERS)
        n_planets = table.column("n_planets").to_numpy()
        n_fleets = table.column("n_fleets").to_numpy()
        self.assertEqual(n_fleets.sum(), seen_fleets)
        self.assertEqual(n_planets.sum(), seen_planets)

        def check(choice, mask, count, width, slots=1):
            for chosen, allowed in zip(rows(table, choice, count, slots),
                                       rows(table, mask, count, width)):
                for slot in range(slots):
                    self.assertTrue(
                        allowed[np.arange(len(chosen)), chosen[:, slot]].all(),
                        f"{choice} outside {mask}")

        check("build", "mask_build", n_planets, nc_ai_env.BUILD_CHOICES,
              nc_ai_env.BUILD_SLOTS)
        check("mission", "mask_mission", n_fleets, nc_ai_env.MISSION_CHOICES)
        check("roe", "mask_roe", n_fleets, nc_ai_env.ROE_CHOICES)
        # One target-mask row per fleet, as wide as the map.
        targets = rows(table, "target", n_fleets, 1)
        for idx, (chosen, worlds) in enumerate(zip(targets, n_planets)):
            allowed = rows(table.slice(idx, 1), "mask_target",
                           n_fleets[idx:idx + 1], int(worlds))[0]
            self.assertTrue(allowed[np.arange(len(chosen)), chosen[:, 0]].all())

        rewards = table.column("reward").to_numpy()
        games = table.column("game").to_numpy()
        for game in range(len(map_seeds)):
            self.assertAlmostEqual(rewards[games == game].sum(), 1.0, places=5)
        years = np.array([row[0] for row in table.column("global").to_pylist()])
        self.assertEqual(sorted(set(years)), list(range(int(years.min()),
                                                         int(years.min()) + 6)))

    def test_only_learned_seats_are_stored(self):
        rng = np.random.default_rng(5)
        map_seeds = np.array([44, 45], dtype=np.uint64)
        env = nc_ai_env.Env(PLAYERS, 4, 2)
        lineup = ["learned", "executor", "aggressor", "learned"]
        env.reset(map_seeds.tolist(), seats=[lineup, lineup])
        done = np.zeros(len(map_seeds), bool)
        with tempfile.TemporaryDirectory() as out:
            writer = ShardWriter(out)
            while not done.all():
                obs, masks = env.observe(), env.masks()
                actions = random_actions(env, masks, rng)
                step = env.step(actions)
                writer.add(experience_batch(
                    map_seeds, map_seeds, env.learned, done, obs, masks,
                    actions, step))
                done = step["done"].copy()
            writer.close()
            table = read_shards(out)
        self.assertEqual(table.num_rows, 4 * len(map_seeds) * 2)
        self.assertEqual(set(table.column("seat").to_pylist()), {0, 3})


if __name__ == "__main__":
    unittest.main()
