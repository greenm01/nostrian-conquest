"""Start positions from played games, for ``fleet_ppo.py --stored-starts``.

Reads each game database (``<server>-<game>-<YYYY-MM-DD>.db``, see
docs/dev/game-history-backups.md) and writes one entry per seat and year
to play on from: ``{"db", "year", "seat", "human"}``, ``seat`` 0-based and
``human`` false for the game's AI seats. A seat that lost its last world
gives the ``--before`` years before that fall; a seat that never fell gives
every ``--every``-th year. A newer copy of a game replaces older ones.

    uv run stored_starts.py ~/.local/state/nostrian-conquest/game-history/*.db --out starts.json
"""

import argparse
import json
import re
import sqlite3
from pathlib import Path

NAME = re.compile(r"^(?P<game>.+)-(?P<date>\d{4}-\d{2}-\d{2})\.db$")


def latest_copies(paths):
    """The newest dated copy of each game; other names are kept as given."""
    latest = {}
    for path in map(Path, paths):
        match = NAME.match(path.name)
        key = match["game"] if match else path.name
        if key not in latest or path.name > latest[key].name:
            latest[key] = path
    return sorted(latest.values())


def worlds_by_year(db):
    """``{year: {empire: worlds}}`` for every stored year."""
    rows = db.execute(
        "SELECT s.game_year, p.owner, count(*) FROM snapshot_planets p "
        "JOIN snapshots s ON s.id = p.snapshot_id WHERE p.owner > 0 GROUP BY 1, 2")
    years = {year: {} for (year,) in db.execute("SELECT game_year FROM snapshots")}
    for year, owner, count in rows:
        years[year][owner] = count
    return years


def seat_starts(years, players, before, every):
    """``[(seat, year)]``: before each seat's fall, or every ``every`` years."""
    ordered = sorted(years)
    starts = []
    for empire in range(1, players + 1):
        held = [year for year in ordered if years[year].get(empire, 0) > 0]
        fall = next((year for year in ordered if held and year > held[0]
                     and years[year].get(empire, 0) == 0), None)
        if fall is not None:
            chosen = [year for year in ordered if fall - before <= year < fall and year in held]
        else:
            chosen = held[::every]
        starts += [(empire - 1, year) for year in chosen]
    return starts


def stored_starts(paths, before=15, every=5):
    entries = []
    for path in latest_copies(paths):
        db = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
        players = db.execute(
            "SELECT player_count FROM snapshot_conquest c JOIN snapshots s "
            "ON s.id = c.snapshot_id ORDER BY s.game_year DESC LIMIT 1").fetchone()[0]
        ai = {seat for (seat,) in db.execute("SELECT player_record_index FROM ai_seats")}
        for seat, year in seat_starts(worlds_by_year(db), players, before, every):
            entries.append({"db": str(path.resolve()), "year": year, "seat": seat,
                            "human": seat + 1 not in ai})
        db.close()
    return entries


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("dbs", nargs="+", help="game databases")
    parser.add_argument("--out", required=True, help="the JSON file to write")
    parser.add_argument("--before", type=int, default=15,
                        help="years before a seat's fall to start from")
    parser.add_argument("--every", type=int, default=5,
                        help="start every this many years for a seat that never fell")
    parser.add_argument("--human-only", action="store_true", help="only the human seats")
    args = parser.parse_args()
    entries = stored_starts(args.dbs, args.before, args.every)
    if args.human_only:
        entries = [entry for entry in entries if entry["human"]]
    Path(args.out).write_text(json.dumps(entries, indent=1))
    print(f"{len(entries)} starts from {len(latest_copies(args.dbs))} games")


if __name__ == "__main__":
    main()
