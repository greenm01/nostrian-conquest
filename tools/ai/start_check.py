"""Win share from start positions (t185): how well each fleet agent plays on
from the positions where an older agent's games went wrong.

    uv run start_check.py fleet-final.pt t185/run/snapshot-0200.pt \\
        --strategist strategist-alloc-final.pt --starts starts.json \\
        --start-model fleet-final-pre-t184.pt [--start-lead 10]

For every position of ``--starts`` (from ``value_trace.py --starts``), the
start model plays the seat greedily against three executors until
``--start-lead`` years before the fall, and then each model in turn plays
it greedily to the end, the same positions for every model. Prints each
model's win share and worlds at the end, and the paired difference from the
first model.
"""

import argparse
import json

import numpy as np

import nc_ai_env
import policy
from fleet_policy import load_fleet_agent
from fleet_ppo import PLAYERS, learner_name, place_starts, play_year
from policy import device

FAMILIES = ["strike", "builds"]


def play_on(model, warm, strategist, starts, lead, years, dev):
    """Win share and worlds at the end of ``model`` on every start position."""
    games = len(starts)
    names = [None] * games
    roles = np.zeros((games, PLAYERS), np.int8)
    maps = [0] * games
    handover = np.zeros(games, np.int64)
    for game, start in enumerate(starts):
        # One position at a time, so each game gets its own (place_starts draws).
        one_names, one_maps = [None], [0]
        handover[game] = place_starts(np.random.default_rng(0), [start], 1,
                                      learner_name(FAMILIES), 3, one_names,
                                      roles[game:game + 1], one_maps, lead)[0]
        names[game], maps[game] = one_names[0], one_maps[0]
    seat = np.array([start["seat"] for start in starts])
    env = nc_ai_env.Env(PLAYERS, years, 0, limit_credit=0.5)
    env.reset(maps, seats=names)
    done = np.zeros(games, bool)
    win = np.zeros(games)
    worlds = np.zeros(games)
    year = 0
    while not done.all():
        roles[handover == year, :] = np.where(roles[handover == year] == 3, 1, 0)
        step, _, _ = play_year(env, model, strategist, roles, done, FAMILIES, dev,
                               greedy=True, frozen=[warm])
        win += step["reward"][np.arange(games), seat]
        live = ~done
        worlds[live] = step["worlds"][live, seat[live]]
        done = step["done"].copy()
        year += 1
    return win, worlds


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("models", nargs="+")
    parser.add_argument("--strategist", required=True)
    parser.add_argument("--starts", required=True)
    parser.add_argument("--start-model", required=True)
    parser.add_argument("--start-lead", type=int, default=10)
    parser.add_argument("--years", type=int, default=100)
    args = parser.parse_args()
    dev = device()
    strategist = policy.load(args.strategist, dev)
    warm = load_fleet_agent(args.start_model, dev)
    starts = json.loads(open(args.starts).read())
    results = []
    for path in args.models:
        win, worlds = play_on(load_fleet_agent(path, dev), warm, strategist, starts,
                              args.start_lead, args.years, dev)
        results.append((win, worlds))
        line = f"{path}: win {win.mean():.3f}, worlds at end {worlds.mean():.2f}"
        if len(results) > 1:
            diff = win - results[0][0]
            line += (f"; paired win {diff.mean():+.3f} +/- "
                     f"{diff.std(ddof=1) / len(diff) ** 0.5:.3f}")
        print(line, flush=True)
    print(f"{len(starts)} positions, handed over {args.start_lead} years before the fall")


if __name__ == "__main__":
    main()
