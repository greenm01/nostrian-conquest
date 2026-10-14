"""Score a strategist policy in seeded 4-player games against scripted
opponents, with the strategist's seat rotated across games.

    uv run evaluate.py --policy labels --opponent executor
    uv run evaluate.py --policy /tmp/nc-ai/imitate.pt --opponent aggressor

``labels`` plays the executor's own goals as a strategist: the reference a
learned strategist must beat. An even seat wins 0.25.
"""

import argparse
import time

import numpy as np

import nc_ai_env


def lineups(games, players, opponent):
    """The strategist in seat ``game % players``; every other seat
    ``opponent``."""
    out = []
    for game in range(games):
        lineup = [opponent] * players
        lineup[game % players] = "strategist"
        out.append(lineup)
    return out


def evaluate(choose, opponent, games, years, first_map, players=4, threads=0):
    """Play ``games`` maps from ``first_map``; ``choose(env)`` returns the
    strategist goals for a step. Returns win share and production share of
    the strategist seat, per game."""
    env = nc_ai_env.Env(players, years, threads)
    maps = list(range(first_map, first_map + games))
    env.reset(maps, seats=lineups(games, players, opponent))
    seat = np.arange(games) % players
    wins = np.zeros(games)
    production = np.zeros(games)
    done = np.zeros(games, bool)
    while not done.all():
        step = env.step(goals=choose(env))
        wins += step["reward"][np.arange(games), seat]
        finished = step["done"] & ~done
        totals = step["production"].sum(axis=1).clip(min=1)
        share = step["production"][np.arange(games), seat] / totals
        production[finished] = share[finished]
        done = step["done"].copy()
    return wins, production


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--policy", default="labels",
                        help="'labels' or a checkpoint path")
    parser.add_argument("--opponent", default="executor",
                        help="executor, baseline, aggressor or autopilot")
    parser.add_argument("--games", type=int, default=256)
    parser.add_argument("--years", type=int, default=60)
    parser.add_argument("--first-map", type=int, default=100000)
    parser.add_argument("--greedy", action="store_true",
                        help="take each head's most likely goal")
    args = parser.parse_args()

    if args.policy == "labels":
        choose = lambda env: env.goal_labels()
    else:
        from policy import load_chooser
        choose = load_chooser(args.policy, greedy=args.greedy)

    started = time.perf_counter()
    wins, production = evaluate(choose, args.opponent, args.games, args.years, args.first_map)
    half = len(wins) // 2
    print(f"{args.policy} vs 3x {args.opponent}, {args.games} maps from "
          f"{args.first_map}, {args.years} years, {time.perf_counter() - started:.1f}s")
    print(f"win share {wins.mean():.3f} (halves {wins[:half].mean():.3f}, "
          f"{wins[half:].mean():.3f}); production share {production.mean():.3f}")


if __name__ == "__main__":
    main()
