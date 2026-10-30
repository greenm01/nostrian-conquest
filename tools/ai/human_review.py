"""How a fleet agent reads a game people played: for every stored year of a
game database, the agent takes each seat in turn and plays one greedy year
from it against three executors.

    uv run human_review.py GAME.db fleet-final.pt --strategist strategist-alloc-final.pt \\
        [--years 100] [--detail 6] [--out review.md]

It prints the critic's win share for each seat by year, undiscounted as
``value_trace.py`` shows it, beside the worlds each seat held. For the
human seats it then compares the agent's orders with the ones the player
stored that year: a door game saves a player's orders into the year's
snapshot, so the agent sees them and may keep them. Each fleet counts as
kept (the agent left the player's order), same (the agent gave it again)
or changed, and the last ``--detail`` years before the seat's fall list
every changed fleet and the tax. The AI seats' own orders are not stored,
so they are rated but not compared.
"""

import argparse
import re
import sqlite3

import numpy as np

import nc_ai_env
import policy
from fleet_policy import load_fleet_agent
from fleet_ppo import PLAYERS, learner_name, play_year
from policy import device
from stored_starts import worlds_by_year

FAMILIES = ["strike", "builds"]
ORDERS = ["hold", "move", "seek_home", "patrol", "guard_starbase", "guard_blockade", "bombard",
          "invade", "blitz", "view", "scout_sector", "scout_system", "colonize", "join",
          "rendezvous", "salvage"]
FLEET = re.compile(r"fleet record=(\d+) \{")
ORDER = re.compile(r'order speed=\d+ kind="(\w+)"(?: x=(\d+) y=(\d+))?')
TAX = re.compile(r"tax rate=(\d+)")


def turn_orders(turns, empire):
    """``(tax, {record: (kind, x, y)})`` from ``empire``'s turns, later
    submissions overriding earlier ones."""
    tax, orders = None, {}
    for turn in turns:
        if not turn.startswith(f"turn player={empire} "):
            continue
        record = None
        for line in turn.splitlines():
            if (match := TAX.search(line)) and record is None:
                tax = int(match[1])
            elif match := FLEET.search(line):
                record = int(match[1])
            elif (match := ORDER.search(line)) and record is not None:
                x, y = (int(match[2]), int(match[3])) if match[2] else (None, None)
                orders[record] = (match[1], x, y)
            elif line.strip() == "}":
                record = None
    return tax, orders


def stored_orders(db, year, empire):
    """``(tax, {record: (kind, x, y, ships, fleet number)})`` as the player
    stored them."""
    (snapshot,) = db.execute("SELECT id FROM snapshots WHERE game_year = ?", (year,)).fetchone()
    (tax,) = db.execute("SELECT tax_rate FROM snapshot_players WHERE snapshot_id = ? "
                        "AND record_index = ?", (snapshot, empire)).fetchone()
    rows = db.execute(
        "SELECT record_index, standing_order, target_x, target_y, battleships + cruisers + "
        "destroyers + empty_transports + loaded_transports + etacs + scouts, fleet_number "
        "FROM snapshot_fleets "
        "WHERE snapshot_id = ? AND owner = ?", (snapshot, empire))
    return tax, {record: (ORDERS[order] if order < len(ORDERS) else str(order), x, y, ships,
                          number)
                 for record, order, x, y, ships, number in rows if ships > 0}


def compare(stored, agent):
    """``(kept, same, changed)`` fleet records."""
    kept, same, changed = [], [], []
    for record, (kind, x, y, *_) in stored.items():
        if record not in agent:
            kept.append(record)
        elif agent[record][0] == kind and agent[record][1:] in ((x, y), (None, None)):
            same.append(record)
        else:
            changed.append(record)
    return kept, same, changed


def place(order):
    kind, x, y = order[:3]
    return f"{kind}" + (f" ({x},{y})" if x is not None else "")


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("db")
    parser.add_argument("model")
    parser.add_argument("--strategist", required=True)
    parser.add_argument("--years", type=int, default=100,
                        help="the agent's year limit, as in training")
    parser.add_argument("--gamma", type=float, default=0.995)
    parser.add_argument("--limit-credit", type=float, default=0.5)
    parser.add_argument("--detail", type=int, default=6,
                        help="list changed orders for this many years before a fall")
    parser.add_argument("--threads", type=int, default=0)
    parser.add_argument("--out", help="also write the report here")
    args = parser.parse_args()
    dev = device()
    model = load_fleet_agent(args.model, dev)
    strategist = policy.load(args.strategist, dev)
    db = sqlite3.connect(f"file:{args.db}?mode=ro", uri=True)
    worlds = worlds_by_year(db)
    ai = {seat for (seat,) in db.execute("SELECT player_record_index FROM ai_seats")}
    first = min(worlds)
    # Every seat holding a world, every stored year it still held one.
    games = [(year, seat) for year in sorted(worlds) for seat in range(PLAYERS)
             if worlds[year].get(seat + 1, 0) > 0]
    names = [[learner_name(FAMILIES) if s == seat else "executor" for s in range(PLAYERS)]
             for _, seat in games]
    roles = np.array([[1 if s == seat else 0 for s in range(PLAYERS)] for _, seat in games],
                     np.int8)
    env = nc_ai_env.Env(PLAYERS, args.years, args.threads, limit_credit=args.limit_credit)
    env.reset(list(range(len(games))), seats=names, starts=[(args.db, y) for y, _ in games])
    _, sample, _ = play_year(env, model, strategist, roles, np.zeros(len(games), bool),
                             FAMILIES, dev, greedy=True)
    values = sample["value"].numpy()
    value = {}
    for g, (year, seat) in enumerate(games):
        left = max(0, args.years - (year - first))
        value[year, seat] = values[g] / args.gamma ** left

    lines = [f"# {args.db}", "",
             f"The agent's win share for each seat, and worlds held ({len(games)} positions).", "",
             "| year | " + " | ".join(f"E{s + 1}{'' if s + 1 in ai else ' (human)'}"
                                     for s in range(PLAYERS)) + " |",
             "|---|" + "---|" * PLAYERS]
    for year in sorted(worlds):
        cells = [f"{value[year, s]:.2f} / {worlds[year].get(s + 1, 0)}"
                 if (year, s) in value else "—" for s in range(PLAYERS)]
        lines.append(f"| {year} | " + " | ".join(cells) + " |")

    for seat in range(PLAYERS):
        if seat + 1 in ai:
            continue
        held = [y for y, s in games if s == seat]
        lines += ["", f"## Empire {seat + 1}: the agent's orders against the player's", "",
                  "| year | tax (player > agent) | kept | same | changed |", "|---|---|---|---|---|"]
        details = []
        for year in held:
            g = games.index((year, seat))
            tax, stored = stored_orders(db, year, seat + 1)
            agent_tax, agent = turn_orders(env.turns(g), seat + 1)
            kept, same, changed = compare(stored, agent)
            lines.append(f"| {year} | {tax} > {agent_tax if agent_tax is not None else tax} | "
                         f"{len(kept)} | {len(same)} | {len(changed)} |")
            if year > held[-1] - args.detail:
                details.append(f"\n**{year}**")
                details += [f"- fleet {stored[record][4]} ({stored[record][3]} ships): "
                            f"{place(stored[record])} > {place(agent[record])}"
                            for record in changed]
        lines += details

    report = "\n".join(lines)
    print(report)
    if args.out:
        with open(args.out, "w") as out:
            out.write(report + "\n")


if __name__ == "__main__":
    main()
