"""α-Rank a league (t248): rank the agents of a league played with
``league.py play --every-lineup N`` and show who beats whom.

    uv run --with open_spiel --with matplotlib league_alpharank.py --db league.db

Each lineup gives one row of a heuristic payoff table: a seat's payoff is
its mean win share, so the four seats of a game sum to one and self-play is
0.25 a seat. The four seats are four populations sharing that table
(OpenSpiel's single-population α-Rank takes two players only). Prints:

- the invasion table, an agent's win share alone against three copies of
  another, with its standard error; above 0.25 it invades;
- α-Rank's mass on each agent, at infinite α and over a sweep of α.

Omelchenko et al. (2019), α-Rank: Multi-Agent Evaluation by Evolution.
"""

import argparse
import sqlite3
from collections import defaultdict

import numpy as np
from open_spiel.python.egt import alpharank, heuristic_payoff_table, utils

PLAYERS = 4


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--db", required=True)
    parser.add_argument("--population", type=int, default=50, help="α-Rank's m")
    args = parser.parse_args()

    db = sqlite3.connect(args.db)
    names = [n for (n,) in db.execute("select name from agents order by id")]
    index = {i: k for k, (i,) in enumerate(db.execute("select id from agents order by id"))}
    n = len(names)
    games = defaultdict(list)
    for match, agent, share in db.execute("select match_id, agent_id, win_share from seats"):
        games[match].append((index[agent], share))

    # distribution -> strategy -> win shares of its seats, game by game
    shares = defaultdict(lambda: defaultdict(list))
    for seats in games.values():
        counts = [0] * n
        for agent, _ in seats:
            counts[agent] += 1
        per = defaultdict(float)
        for agent, share in seats:
            per[agent] += share / counts[agent]
        for agent, share in per.items():
            shares[tuple(counts)][agent].append(share)

    table = heuristic_payoff_table.PayoffTable(PLAYERS, n)
    for d in utils.distribute(PLAYERS, n):
        key = tuple(int(x) for x in d)
        payoff = np.zeros(n)
        if max(key) == PLAYERS:
            payoff[key.index(PLAYERS)] = 1.0 / PLAYERS
        elif key not in shares:
            raise SystemExit(f"no games for lineup {key}; play with --every-lineup")
        else:
            for agent, values in shares[key].items():
                payoff[agent] = np.mean(values)
        table[d] = payoff
    assert all(abs(sum(table[d] * np.array(d)) - 1.0) < 1e-9 for d in utils.distribute(PLAYERS, n))

    width = max(len(x) for x in names)
    print(f"{len(games)} games. Win share of the row agent alone against three of the column")
    print(f"agent (standard error); above 0.25 the row invades.\n")
    print(" " * width + "".join(f"{x[:12]:>14}" for x in names))
    for r in range(n):
        cells = []
        for s in range(n):
            if r == s:
                cells.append(f"{'-':>14}")
                continue
            key = [0] * n
            key[r], key[s] = 1, PLAYERS - 1
            values = shares[tuple(key)][r]
            se = np.std(values, ddof=1) / np.sqrt(len(values))
            cells.append(f"{np.mean(values):>8.3f}({se:.2f})")
        print(f"{names[r]:<{width}}" + "".join(cells))

    tables = [table] * PLAYERS
    strats = utils.get_num_strats_per_population(tables, True)

    def marginal(pi):
        mass = np.zeros(n)
        for profile_id, p in enumerate(pi):
            for strat in utils.get_strat_profile_from_id(strats, profile_id):
                mass[strat] += p / PLAYERS
        return mass

    _, _, pi, _, _ = alpharank.compute(tables, m=args.population, use_inf_alpha=True)
    top = np.argsort(-pi)[:5]
    print("\nα-Rank at infinite α; the most-visited lineups:")
    for profile_id in top:
        profile = utils.get_strat_profile_from_id(strats, profile_id)
        print(f"  {pi[profile_id]:.3f}  " + ", ".join(names[s] for s in profile))
    sweep = [(alpha, marginal(alpharank.compute(tables, m=args.population, alpha=alpha)[2]))
             for alpha in (0.1, 1, 10, 100)]
    print("\nmass on each agent, by α (infinite last)")
    print(" " * width + "".join(f"{a:>8}" for a, _ in sweep) + f"{'inf':>8}")
    inf = marginal(pi)
    for s in np.argsort(-inf):
        print(f"{names[s]:<{width}}" + "".join(f"{m[s]:>8.3f}" for _, m in sweep) + f"{inf[s]:>8.3f}")


if __name__ == "__main__":
    main()
