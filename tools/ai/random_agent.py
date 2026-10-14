"""Play seeded games with uniformly random choices within the masks, write
the experience as Parquet shards, and report throughput and mask cost.

    uv run random_agent.py --out /tmp/nc-shards --games 64 --years 40
"""

import argparse
import time

import numpy as np

import nc_ai_env
from shards import ShardWriter, experience_batch


def sample(mask: np.ndarray, rng: np.random.Generator) -> np.ndarray:
    """A uniformly random allowed index along the last axis."""
    scores = rng.random(mask.shape)
    scores[~mask] = -1.0
    return scores.argmax(axis=-1)


def random_actions(env, masks, rng):
    games, seats = masks["diplomacy"].shape[:2]
    fleets = masks["mission"].shape[2]
    mission = sample(masks["mission"], rng)
    targets = np.take_along_axis(masks["target"], mission[..., None, None], axis=3)
    diplomacy = sample(masks["diplomacy"], rng)
    # Mostly keep relations, so games are not all-out war.
    diplomacy[rng.random(diplomacy.shape) >= 1 / 8] = 0
    return {
        "tax": rng.integers(nc_ai_env.TAX_CHOICES, size=(games, seats)),
        "diplomacy": diplomacy,
        "build": sample(masks["build"], rng),
        "amount": rng.integers(
            nc_ai_env.BUILD_AMOUNT_CHOICES, size=(games, seats, env.worlds)
        ),
        "mission": mission,
        "target": sample(targets[:, :, :, 0, :], rng),
        "speed": rng.integers(nc_ai_env.SPEED_CHOICES, size=(games, seats, fleets)),
        "roe": sample(masks["roe"], rng),
        "load": (masks["load"] & (rng.random(masks["load"].shape) < 0.5)).astype(
            np.int64
        ),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--out", required=True, help="shard directory")
    parser.add_argument("--players", type=int, default=4)
    parser.add_argument("--games", type=int, default=64)
    parser.add_argument("--years", type=int, default=40)
    parser.add_argument("--first-seed", type=int, default=1)
    parser.add_argument("--threads", type=int, default=0)
    args = parser.parse_args()

    rng = np.random.default_rng(args.first_seed)
    map_seeds = np.arange(args.first_seed, args.first_seed + args.games, dtype=np.uint64)
    campaign_seeds = map_seeds ^ np.uint64(0x5EED)
    env = nc_ai_env.Env(args.players, args.years, args.threads)

    started = time.perf_counter()
    env.reset(map_seeds.tolist(), campaign_seeds.tolist())
    reset_seconds = time.perf_counter() - started

    writer = ShardWriter(args.out)
    done = np.zeros(args.games, bool)
    game_years = 0
    seconds = {"observe": 0.0, "masks": 0.0, "sample": 0.0, "step": 0.0, "write": 0.0}
    total = np.zeros((args.games, args.players), np.float32)
    while not done.all():
        clock = time.perf_counter()
        obs = env.observe()
        seconds["observe"] += time.perf_counter() - clock

        clock = time.perf_counter()
        masks = env.masks()
        seconds["masks"] += time.perf_counter() - clock

        clock = time.perf_counter()
        actions = random_actions(env, masks, rng)
        seconds["sample"] += time.perf_counter() - clock

        clock = time.perf_counter()
        step = env.step(actions)
        seconds["step"] += time.perf_counter() - clock

        clock = time.perf_counter()
        writer.add(
            experience_batch(map_seeds, campaign_seeds, done, obs, masks, actions, step)
        )
        seconds["write"] += time.perf_counter() - clock

        game_years += int((~done).sum())
        total += step["reward"]
        done = step["done"].copy()
    writer.close()

    played = sum(seconds.values())
    mask_seconds, mask_count = env.mask_cost
    print(f"{args.games} games of {args.players} seats, {game_years} game-years")
    print(f"reset {reset_seconds:.2f}s, play {played:.2f}s: "
          f"{game_years / played:.0f} game-years/s")
    for name, spent in seconds.items():
        print(f"  {name:8} {spent:7.2f}s  {1000 * spent / game_years:6.3f} ms/game-year")
    print(f"seat masks: {mask_count} built, "
          f"{1e6 * mask_seconds / mask_count:.0f} us each (thread time)")
    print(f"rewards per game sum to {total.sum(axis=1).min():.3f}"
          f"..{total.sum(axis=1).max():.3f}")
    print(f"wrote {writer.shards} shard(s) to {args.out}")


if __name__ == "__main__":
    main()
