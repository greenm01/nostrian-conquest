#!/usr/bin/env python3
"""Audit fleet encounters against classic ECMAINT: who sights whom, when,
and what each empire is told.

Empire 1's fleet 1 crosses Sector(11,11), where empire 2's fleet 5 patrols,
under each pairing of diplomacy and ROE. Sightings are rolls in both
engines, so each case runs `--reps` times and the tool tallies the report
sequences each engine produced, with their weeks. Classic seeds its rolls
from the clock, so its runs go one at a time; each Rust run gets its own
campaign seed. Combat numbers are masked as in `ncmaint_stardate_audit.py`.

    nix shell nixpkgs#dosbox-x -c python3 tools/ncmaint_encounter_audit.py [--reps 16]
"""

from __future__ import annotations

import argparse
import json
import re
import sqlite3
import zlib
from collections import Counter, defaultdict
from pathlib import Path

import ncmaint_mission_sweep as sweep
import ncmaint_stardate_audit as audit

DEFAULT_WORK_ROOT = Path("/tmp/ecmaint-encounter-audit")
HALF = bytes([0x80, 0, 0, 0, 0, 0])  # Turbo Pascal real 0.5
PLAYER_RECORD = 110
SECTOR = (11, 11)  # on the diagonal from (10,10) to the sweep's (13,13)
# fleet-ships order, as u16 words from FLEETS 0x26: BB CA DD empty-TT loaded-TT ETAC SC
CRUISERS = (0, 3, 0, 0, 0, 0, 0)
SCOUT = (0, 0, 0, 0, 0, 0, 1)

# name, sweep mission (1 cruiser Move, 10 scout Scout Sector), empire 1
# toward 2, empire 2 toward 1 (1 enemy), empire 2's ships, its ROE
CASES = [
    ("neutral-pass", 1, 0, 0, CRUISERS, 6),
    ("we-hostile-cruiser", 1, 1, 0, CRUISERS, 6),
    ("they-hostile-cruiser", 1, 0, 1, CRUISERS, 6),
    ("both-hostile-cruiser", 1, 1, 1, CRUISERS, 6),
    ("they-hostile-our-scout", 10, 0, 1, CRUISERS, 6),
    ("we-hostile-their-scout", 1, 1, 0, SCOUT, 0),
    ("both-hostile-their-roe0", 1, 1, 1, CRUISERS, 0),
]
# Arrival notices say nothing about the encounter.
QUIET = ("Move mission report", "beginning to scout")


def patrol_in_path(stance12: int, stance21: int, ships: tuple[int, ...], roe: int):
    """Put empire 2's fleet 5 on Patrol at rest in SECTOR and set both
    empires' stances, in the classic files Rust is copied from."""
    def patch(directory: Path) -> None:
        changes = {0x0A: 0, 0x0B: SECTOR[0], 0x0C: SECTOR[1], 0x1F: 3, 0x20: SECTOR[0],
                   0x21: SECTOR[1], 0x22: 0, 0x23: 0, 0x24: 0, 0x25: roe}
        for i, byte in enumerate(HALF):
            changes[0x0D + i] = byte
            changes[0x13 + i] = byte
        for i in range(6):
            changes[0x19 + i] = 0  # depth 0: deep space
        for i, count in enumerate(ships):
            changes[0x26 + 2 * i] = count
            changes[0x27 + 2 * i] = 0
        sweep.patch_fleet(directory, 5, changes)
        players = bytearray((directory / "PLAYER.DAT").read_bytes())
        players[0x53 + 2] = stance12  # empire 1 toward 2
        players[PLAYER_RECORD + 0x53 + 1] = stance21  # empire 2 toward 1
        (directory / "PLAYER.DAT").write_bytes(bytes(players))
    return patch


def run(case: tuple, rep: int, work_root: Path) -> dict:
    name, mission, s12, s21, ships, roe = case
    build_rust = audit.build_rust

    def seeded(sweep_case, classic, rust):
        build_rust(sweep_case, classic, rust)
        # Distinct per case and run, so cases don't share their luck.
        seed = zlib.crc32(f"{name}-{rep}".encode())
        db = sqlite3.connect(rust / "ncgame.db")
        db.execute("UPDATE campaign_metadata SET int_value = ? WHERE key = 'campaign_seed'", (seed,))
        db.commit()
        db.close()

    audit.build_rust = seeded
    try:
        result = audit.run_case(sweep.Case(mission, 3, "d3", 2), work_root / f"{name}-{rep}",
                                humans=(1, 2), prepare=patrol_in_path(s12, s21, ships, roe))
    finally:
        audit.build_rust = build_rust
    result["case"], result["rep"] = name, rep
    return result


def shape(report: str) -> str:
    match = re.match(r"(E\d) +(\d+) .*?: (.*)", report)
    return f"{match[1]} {re.sub(r'[(]\d+,\d+[)]', '', match[3])[:70]}"


def tally(results: list[dict]) -> str:
    by_case = defaultdict(list)
    for result in results:
        by_case[result["case"]].append(result)
    lines = []
    for name, runs in by_case.items():
        lines.append(f"== {name} ({len(runs)} runs)")
        for side in ("classic", "rust"):
            sequences, weeks = Counter(), defaultdict(Counter)
            for result in runs:
                reports = [(y, t) for y, row in enumerate(result["years"], 1)
                           for t in row[side]["reports"] if not any(q in t for q in QUIET)]
                sequences[" || ".join(shape(t) for _, t in reports) or "(none)"] += 1
                for y, t in reports:
                    weeks[shape(t)[:45]][f"y{y}w{int(t.split()[1])}"] += 1
            lines.append(f"  {side}")
            lines += [f"    {n:2d}x {seq}" for seq, n in sequences.most_common()]
            lines += [f"       weeks {k}: {dict(sorted(w.items()))}" for k, w in weeks.items()]
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--reps", type=int, default=16)
    parser.add_argument("--work-root", type=Path, default=DEFAULT_WORK_ROOT)
    parser.add_argument("--json", type=Path, help="write the raw results here")
    args = parser.parse_args()
    # One at a time: classic runs started together share a clock seed.
    results = [run(case, rep, args.work_root) for case in CASES for rep in range(args.reps)]
    print(tally(results))
    if args.json:
        args.json.write_text(json.dumps(results, indent=1))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
