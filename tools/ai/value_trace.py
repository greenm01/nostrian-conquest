"""The value estimate through games (t186): how a fleet agent's critic
rates its own seat year by year, and the years it fell most.

    uv run value_trace.py fleet-final.pt --strategist strategist-alloc-final.pt \\
        [--opponent executor|strategist|AGENT.pt] [--games 256] [--years 100] \\
        [--first-map 100000] [--top 20] [--csv values.csv]

One greedy learner seat plays three ``--opponent`` seats (a scripted
policy, or three greedy copies of a frozen agent), the seat rotated game by
game as ``head_to_head`` does. It prints the mean value, worlds and
production share by decade beside the final win share, then the ``--top``
largest one-year falls in value with the seat's state before and after.
Map, seat and year name a position exactly: greedy play from that map
reaches it again, so the falls are start points for t185.
``--csv`` writes every game-year for plotting elsewhere.
"""

import argparse
import csv

import numpy as np

import nc_ai_env
import policy
from fleet_policy import load_fleet_agent
from fleet_ppo import PLAYERS, frozen_name, learner_name, play_year
from policy import device

FAMILIES = ["strike", "builds"]


def falls(values, top):
    """``(game, year)`` of the ``top`` largest drops ``values[year] -
    values[year + 1]`` in ``[years + 1, games]`` (NaN once a game ends)."""
    drop = values[:-1] - values[1:]
    drop = np.where(np.isnan(drop), -np.inf, drop)
    order = np.argsort(drop, axis=None)[::-1][:top]
    years, games = np.unravel_index(order, drop.shape)
    return [(int(g), int(y)) for g, y in zip(games, years) if np.isfinite(drop[y, g])]


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("model")
    parser.add_argument("--strategist", required=True)
    parser.add_argument("--opponent", default="executor")
    parser.add_argument("--games", type=int, default=256)
    parser.add_argument("--years", type=int, default=100)
    parser.add_argument("--first-map", type=int, default=100_000)
    parser.add_argument("--top", type=int, default=20)
    parser.add_argument("--threads", type=int, default=0)
    parser.add_argument("--csv")
    args = parser.parse_args()
    dev = device()
    model = load_fleet_agent(args.model, dev)
    strategist = policy.load(args.strategist, dev)
    frozen = args.opponent.endswith(".pt")
    target = load_fleet_agent(args.opponent, dev) if frozen else None
    other = frozen_name(FAMILIES) if frozen else args.opponent
    other_role = 3 if frozen else 2 if args.opponent == "strategist" else 0

    games, years = args.games, args.years
    seat = np.arange(games) % PLAYERS
    names = [[learner_name(FAMILIES) if s == seat[g] else other for s in range(PLAYERS)]
             for g in range(games)]
    roles = np.where(np.arange(PLAYERS)[None] == seat[:, None], 1, other_role).astype(np.int8)
    env = nc_ai_env.Env(PLAYERS, years, args.threads)
    env.reset(list(range(args.first_map, args.first_map + games)), seats=names)

    # Row y: the value the seat saw at the start of year y, and its worlds
    # and production share then.
    values = np.full((years + 1, games), np.nan)
    worlds = np.full((years + 1, games), np.nan)
    share = np.full((years + 1, games), np.nan)
    win = np.zeros(games)
    done = np.zeros(games, bool)
    year = 0
    while not done.all():
        live = np.flatnonzero(~done)
        step, sample, _ = play_year(env, model, strategist, roles, done, FAMILIES, dev,
                                    greedy=True, target=target)
        values[year, live] = sample["value"].numpy()
        production = step["production"].astype(np.float64)
        total = production.sum(1)
        mine = production[np.arange(games), seat]
        worlds[year + 1, live] = step["worlds"][live, seat[live]]
        share[year + 1, live] = np.divide(mine, total, out=np.zeros(games), where=total > 0)[live]
        win += step["reward"][np.arange(games), seat]
        done = step["done"].copy()
        year += 1

    print(f"{games} games, {args.opponent} opponents, mean win share {win.mean():.3f}\n")
    print("year  games  value  worlds  share")
    for y in range(0, year, 10):
        ok = ~np.isnan(values[y])
        print(f"{y:4d}  {ok.sum():5d}  {np.nanmean(values[y]):5.3f}  "
              f"{np.nanmean(worlds[y]) if y else float('nan'):6.1f}  "
              f"{np.nanmean(share[y]) if y else float('nan'):5.3f}")

    print(f"\nlargest one-year falls in value (map, seat, year: the year it fell over)")
    print("  map     seat  year  value        worlds    share          win")
    for g, y in falls(values, args.top):
        print(f"  {args.first_map + g:<7d} {seat[g]:4d}  {y:4d}  "
              f"{values[y, g]:.3f}>{values[y + 1, g]:.3f}  "
              f"{worlds[y, g]:3.0f}>{worlds[y + 1, g]:<3.0f}  "
              f"{share[y, g]:.3f}>{share[y + 1, g]:.3f}  {win[g]:.2f}")

    if args.csv:
        with open(args.csv, "w", newline="") as out:
            rows = csv.writer(out)
            rows.writerow(["map", "seat", "year", "value", "worlds", "share", "win"])
            for g in range(games):
                for y in range(year + 1):
                    if not np.isnan(values[y, g]):
                        rows.writerow([args.first_map + g, seat[g], y, f"{values[y, g]:.4f}",
                                       worlds[y, g], f"{share[y, g]:.4f}", win[g]])


if __name__ == "__main__":
    main()
