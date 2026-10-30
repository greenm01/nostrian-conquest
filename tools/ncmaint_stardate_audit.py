#!/usr/bin/env python3
"""Audit Rust report stardates against classic ECMAINT, order by order.

Classic only saves its files at the end of a year, but its reports carry the
week each event happened in. Each case gives empire 1's first fleet one order
on a hazard-free map, runs the same directory through classic ECMAINT and
`nc-cli maint-rust` year by year, and compares that fleet's reports: kind,
location and stardate. Run under DOSBox-X:

    nix shell nixpkgs#dosbox-x -c python3 tools/ncmaint_stardate_audit.py
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import sys
from dataclasses import asdict, dataclass
from pathlib import Path

from ncmaint_movement_audit import parse_coords, parse_key_values, run_classic_turn, run_nc_cli
from ncmaint_oracle import DEFAULT_BASELINE, ensure_engine

DEFAULT_WORK_ROOT = Path("/tmp/ecmaint-stardate-audit")
RECORD = 84
TEXT_START, TEXT_SIZE = 2, 72
HEADER = re.compile(
    r"From your (?P<fleet>[\w ]+?Fleet), located in (?P<layer>System|Sector)"
    r"\((?P<x>\d+),(?P<y>\d+)\):?\s*Stardate:\s*(?P<week>\d+)/(?P<year>\d+)\s*"
    r"(?P<body>.*?)<end of transmission>",
    re.S,
)


@dataclass(frozen=True)
class Case:
    name: str
    order: int
    target: tuple[int, int]
    start: tuple[int, int]
    ships: tuple[str, ...]  # nc-cli fleet-ships: sc bb ca dd tt [loaded] [etac]
    speed: int
    years: int


CRUISER = ("0", "0", "1", "0", "0")
SCOUT = ("1", "0", "0", "0", "0")
ETAC = ("0", "0", "0", "0", "0", "0", "1")
HOME = (16, 13)
# Orders that target a sector rather than a world (Order::targets_world):
# Move, Patrol, Scout Sector, Rendezvous.
SECTOR_ORDERS = {1, 3, 10, 14}

CASES = [
    Case("move", 1, (12, 9), HOME, CRUISER, 3, 3),
    Case("move-scout", 1, (10, 13), HOME, SCOUT, 5, 2),
    Case("seek-home", 2, HOME, (12, 9), CRUISER, 3, 3),
    Case("patrol", 3, (13, 10), HOME, CRUISER, 3, 3),
    Case("guard-blockade-home", 5, HOME, (12, 12), CRUISER, 3, 3),
    Case("view-world", 9, (12, 12), HOME, CRUISER, 3, 2),
    Case("view-world-scout", 9, (12, 11), HOME, SCOUT, 5, 2),
    Case("scout-sector", 10, (12, 9), HOME, SCOUT, 5, 2),
    Case("scout-system", 11, (12, 11), HOME, SCOUT, 5, 2),
    Case("colonize", 12, (15, 13), HOME, ETAC, 1, 2),
    Case("colonize-far", 12, (12, 12), HOME, ETAC, 1, 3),
    Case("rendezvous", 14, (13, 10), HOME, CRUISER, 3, 2),
]


@dataclass
class Report:
    year: int
    week: int
    layer: str
    sector: tuple[int, int]
    kind: str


def results_text(path: Path) -> str:
    """RESULTS.DAT's report text, record by record (classic's and Rust's)."""
    data = path.read_bytes() if path.exists() else b""
    out = []
    for i in range(0, len(data) - RECORD + 1, RECORD):
        chunk = data[i:i + RECORD]
        used = chunk[1]
        # Byte 1 is the record's text length; classic leaves stale bytes after it.
        if used <= TEXT_SIZE:
            out.append(chunk[TEXT_START:TEXT_START + used])
        else:
            out.append(chunk[1:76].split(b"\0", 1)[0])
    return b"".join(out).decode("cp437", "replace")


def fleet_reports(path: Path, fleet: str = "1st Fleet") -> list[Report]:
    reports = []
    for match in HEADER.finditer(results_text(path)):
        if match["fleet"] != fleet:
            continue
        kind = re.search(r"([A-Z][\w/ ]*?) mission report", match["body"])
        reports.append(Report(int(match["year"]), int(match["week"]), match["layer"],
                              (int(match["x"]), int(match["y"])),
                              kind[1] if kind else " ".join(match["body"].split())[:40]))
    return reports


def configure(target: Path, case: Case) -> None:
    # A human empire: neither engine reports to one in civil disorder.
    run_nc_cli(["player-name", str(target), "1", "probe", "Probe Empire"])
    run_nc_cli(["fleet-ships", str(target), "1", *case.ships])
    run_nc_cli(["fleet-location", str(target), "1", *map(str, case.start)])
    # ECGAME's world-target flag (Order::targets_world) and a clear mission id.
    world = "0" if case.order in SECTOR_ORDERS else "1"
    run_nc_cli(["fleet-order", str(target), "1", str(case.speed), str(case.order),
                *map(str, case.target), world, "0"])


def run_case(case: Case, work_root: Path) -> dict:
    root = work_root / case.name
    if root.exists():
        shutil.rmtree(root)
    dirs = {side: root / side for side in ("rust", "classic")}
    for side, path in dirs.items():
        run_nc_cli(["scenario-init-replayable", str(DEFAULT_BASELINE), str(path), "move"])
        configure(path, case)
    run_nc_cli(["db-export", str(dirs["classic"]), str(dirs["classic"])])
    ensure_engine(dirs["classic"])
    years = []
    first_year = int(parse_key_values(
        run_nc_cli(["core-report", str(dirs["rust"])]).stdout)["conquest.game_year"])
    for year in range(1, case.years + 1):
        run_nc_cli(["maint-rust", str(dirs["rust"]), "1"])
        # The year's reports reach RESULTS.DAT through the export.
        run_nc_cli(["db-export", str(dirs["rust"]), str(dirs["rust"])])
        run_classic_turn(dirs["classic"])
        row = {}
        for side, path in dirs.items():
            args = ["inspect-fleet-movement", str(path), "1"]
            fleet = parse_key_values(run_nc_cli(args + ["--live-dir"] * (side == "classic")).stdout)
            row[side] = {"coords": parse_coords(fleet["coords"]), "order": fleet["order"],
                         # Unread reports of earlier years stay in RESULTS.DAT.
                         "reports": [asdict(r) for r in fleet_reports(path / "RESULTS.DAT")
                                     if r.year == first_year + year - 1]}
        years.append(row)
    return {"case": asdict(case), "years": years}


def same(row: dict) -> bool:
    return row["rust"] == row["classic"]


def render(results: list[dict]) -> str:
    lines = ["# Stardate audit: Rust against classic ECMAINT", "",
             "Empire 1's first fleet, one order per case, hazard-free map. Each row "
             "is a year: the fleet's sector and order after it, and its reports as "
             "`week:layer:kind`.", "",
             "| case | year | match | classic | rust |", "|---|---|---|---|---|"]
    for result in results:
        for year, row in enumerate(result["years"], 1):
            cells = []
            for side in ("classic", "rust"):
                reports = " ".join(f"{r['week']}:{r['layer']}:{r['kind']}"
                                   for r in row[side]["reports"]) or "—"
                cells.append(f"{tuple(row[side]['coords'])} {row[side]['order']}; {reports}")
            lines.append(f"| {result['case']['name']} | {year} | "
                         f"{'yes' if same(row) else '**no**'} | {cells[0]} | {cells[1]} |")
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--work-root", type=Path, default=DEFAULT_WORK_ROOT)
    parser.add_argument("--case", action="append", help="run only these cases")
    parser.add_argument("--json", type=Path, help="write the raw results here")
    parser.add_argument("--markdown", type=Path, help="write the table here")
    args = parser.parse_args()
    cases = [c for c in CASES if not args.case or c.name in args.case]
    results = [run_case(case, args.work_root) for case in cases]
    table = render(results)
    print(table)
    if args.json:
        args.json.write_text(json.dumps(results, indent=1))
    if args.markdown:
        args.markdown.write_text(table)
    mismatched = sum(not same(row) for r in results for row in r["years"])
    print(f"{mismatched} mismatched case-years", file=sys.stderr)
    return 1 if mismatched else 0


if __name__ == "__main__":
    raise SystemExit(main())
