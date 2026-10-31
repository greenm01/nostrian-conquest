#!/usr/bin/env python3
"""Audit Rust's week-by-week movement and reports against classic ECMAINT.

Classic saves only at year end, but its fleet records keep the exact
in-sector position and depth, and its reports carry the week each event
happened in, in the order it happened. For every case of
`ncmaint_mission_sweep.py` (one order, speed and start offset for empire 1's
first fleet), this runs both engines year by year and compares:

- the fleet's motion state: sector, fractions, depth, speed, order, target,
  aux bytes, ROE;
- every empire's reports of the year, in order: recipient, sender,
  stardate and text.

Combat dice are not reproducible in classic, so a combat report keeps its
place and week but its numbers are masked. Run under DOSBox-X:

    nix shell nixpkgs#dosbox-x -c python3 tools/ncmaint_stardate_audit.py \
        [--missions 1,2] [--speeds 1,3] [--offsets d3,d6] [--years 6]

`--rendezvous [N]` (mission 14) sends fleets 2..N to the rendezvous too, so
they merge. `--variant` aims the order at another target (see
`ncmaint_mission_sweep.TARGETS`), `--base-move` also orders the starbase to
move, and `--retarget MISSION:X,Y[:YEAR]` changes fleet 1's order after a
year, as a player would. Cases faster than the fleet's ships are skipped.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections.abc import Callable
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from ncmaint_mission_sweep import (
    BASE_MOVES, BASE_RECORD, MISSIONS, NC_CLI_BIN, SECTOR, TARGETS, Case,
    build_classic, build_rust, is_world, nc, patch_fleet, snapshot, turbo_real,
)
from ncmaint_oracle import run_ecmaint

DEFAULT_WORK_ROOT = Path("/tmp/ecmaint-stardate-audit")
RECORD = 84
PLAYER_RECORD, RESULTS_FIRST, NEXT_HEADER = 110, 0x38, 78
TEXT_START, TEXT_SIZE = 2, 72
REPORT = re.compile(
    r"(?P<sender>[^\n]*?)\s*Stardate:\s*(?P<week>\d+)/(?P<year>\d+)\s*"
    r"(?P<body>.*?)<end of transmission>",
    re.S,
)
COMBAT = re.compile(r"battle|combat|bombard|attack|destroyed|casualt|invasion|invad|blitz", re.I)
MOTION = ("pos", "x_frac", "y_frac", "tuple_c", "speed", "order", "target", "aux", "roe", "base")


def header_viewers(directory: Path, data: bytes) -> dict[int, int]:
    """0-based header record -> empire, walking each empire's report chain
    from its PLAYER.DAT first-header pointer."""
    players = (directory / "PLAYER.DAT").read_bytes()
    count = len(data) // RECORD
    viewers = {}
    for empire, offset in enumerate(range(0, len(players), PLAYER_RECORD), 1):
        header = int.from_bytes(players[offset + RESULTS_FIRST:offset + RESULTS_FIRST + 2], "little")
        while 0 < header <= count and header - 1 not in viewers:
            viewers[header - 1] = empire
            start = (header - 1) * RECORD + NEXT_HEADER
            header = int.from_bytes(data[start:start + 2], "little")
    return viewers


def results_text(directory: Path) -> dict[int, str]:
    """Each empire's RESULTS.DAT report text, one line per record (classic's
    and Rust's). A header's body records follow it."""
    path = directory / "RESULTS.DAT"
    data = path.read_bytes() if path.exists() else b""
    viewers = header_viewers(directory, data)
    out: dict[int, list[bytes]] = {}
    viewer = 0
    for record, i in enumerate(range(0, len(data) - RECORD + 1, RECORD)):
        viewer = viewers.get(record, viewer)
        chunk = data[i:i + RECORD]
        used = chunk[1]
        # Byte 1 is the record's text length; classic leaves stale bytes after it.
        if used <= TEXT_SIZE:
            text = chunk[TEXT_START:TEXT_START + used]
        else:
            text = chunk[1:76].split(b"\0", 1)[0]
        out.setdefault(viewer, []).append(text + b"\n")
    return {v: b"".join(lines).decode("cp437", "replace") for v, lines in sorted(out.items())}


def reports(directory: Path, year: int) -> list[str]:
    """The year's reports, each empire's in order, as `E<n> week sender: text`."""
    out = []
    for viewer, text in results_text(directory).items():
        for match in REPORT.finditer(text):
            if int(match["year"]) != year:
                continue  # classic keeps unread reports of earlier years
            sender = " ".join(match["sender"].split()).rstrip(":")
            body = " ".join(match["body"].split())
            if COMBAT.search(body):
                body = "[combat] " + re.sub(r"\d+", "#", body)
            out.append(f"E{viewer} {int(match['week']):2d} {sender}: {body}")
    return out


def base_motion(directory: Path) -> list | None:
    """The first starbase's sector, fractions, depth, moving flag and target."""
    path = directory / "BASES.DAT"
    data = path.read_bytes() if path.exists() else b""
    if len(data) < BASE_RECORD:
        return None
    reals = [round(turbo_real(data[i:i + 6]), 6) for i in (0x0D, 0x13, 0x19)]
    return [list(data[0x0B:0x0D]), *reals, data[0x1F], list(data[0x20:0x23])]


def state(directory: Path, year: int, case: Case) -> dict:
    row = snapshot(directory, 0, case.spec.target)
    row["base"] = base_motion(directory)
    return {"motion": {k: row.get(k) for k in MOTION}, "reports": reports(directory, year)}


def game_year(directory: Path) -> int:
    return int(re.search(r"conquest\.game_year=(\d+)", nc("core-report", directory))[1])


def run_case(case: Case, work_root: Path, humans: tuple[int, ...] = (1,),
             prepare: Callable[[Path], None] | None = None,
             retarget: tuple[int, tuple[int, int], int] | None = None) -> dict:
    """`prepare` may patch the classic directory before Rust copies it."""
    root = work_root / case.name
    classic, rust, export = root / "classic", root / "rust", root / "rust-export"
    build_classic(case, classic, humans)
    max_speed = (classic / "FLEETS.DAT").read_bytes()[0x09]
    if case.speed > max_speed:
        return {"case": case.name, "skipped": f"speed {case.speed} > max speed {max_speed}"}
    if prepare:
        prepare(classic)
    build_rust(case, classic, rust)
    first = game_year(rust)
    years = []
    for year in range(1, case.years + 1):
        result = run_ecmaint(classic)
        if result.returncode != 0:
            raise SystemExit(f"ECMAINT failed in {classic}:\n{result.stdout}{result.stderr}")
        nc("maint-rust", rust, 1)
        nc("db-export", rust, export)
        stamp = first + year - 1
        years.append({"classic": state(classic, stamp, case), "rust": state(export, stamp, case)})
        if retarget and retarget[2] == year:
            change_order(case, classic, rust, export, *retarget[:2])
    return {"case": case.name, "years": years}


def change_order(case: Case, classic: Path, rust: Path, export: Path,
                 order: int, target: tuple[int, int]) -> None:
    """Give fleet 1 a new order between years: classic gets the bytes ECGAME
    writes, Rust gets `fleet-order`. Fails if `fleet-order` changes any motion
    field the classic patch leaves alone."""
    before = {"c": state(classic, 0, case)["motion"], "r": state(export, 0, case)["motion"]}
    world = int(is_world(classic, target))
    patch_fleet(classic, 1, {0x0A: case.speed, 0x1F: order, 0x20: target[0], 0x21: target[1], 0x22: world, 0x23: 0})
    nc("fleet-order", rust, 1, case.speed, order, *target, world, 0)
    nc("db-export", rust, export)
    after = {"c": state(classic, 0, case)["motion"], "r": state(export, 0, case)["motion"]}
    changed = {side: {k for k in MOTION if before[side][k] != after[side][k]} for side in "cr"}
    wrong = changed["r"] - changed["c"] | {k for k in changed["c"] if after["c"][k] != after["r"][k]}
    if wrong:
        raise SystemExit(f"{case.name}: fleet-order and the classic patch differ in {sorted(wrong)}")


def rendezvous_partners(count: int) -> Callable[[Path], None]:
    """Send fleets 2..count from home to the Rendezvous sector at speeds 2, 1,
    2, ..., so they meet fleet 1 there in different weeks."""
    def prepare(directory: Path) -> None:
        for fleet in range(2, count + 1):
            patch_fleet(directory, fleet, {0x0A: 2 - fleet % 2, 0x1F: 14, 0x20: SECTOR[0],
                                           0x21: SECTOR[1], 0x22: 0, 0x23: 0})
    return prepare


def diff(result: dict) -> list[str]:
    """Each mismatched year: the differing motion fields, then both report lists."""
    lines = []
    for year, row in enumerate(result.get("years", []), 1):
        c, r = row["classic"], row["rust"]
        fields = [k for k in MOTION if c["motion"][k] != r["motion"][k]]
        if fields:
            lines.append(f"  y{year} motion " + "; ".join(
                f"{k} {c['motion'][k]} / {r['motion'][k]}" for k in fields))
        if c["reports"] != r["reports"]:
            lines.append(f"  y{year} reports")
            lines += [f"    C {text}" for text in c["reports"]]
            lines += [f"    R {text}" for text in r["reports"]]
    return lines


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--missions", default=",".join(map(str, MISSIONS)))
    parser.add_argument("--speeds", default="1,3")
    parser.add_argument("--offsets", default="d3,d6")
    parser.add_argument("--years", type=int, default=6)
    parser.add_argument("--jobs", type=int, default=4)
    parser.add_argument("--work-root", type=Path, default=DEFAULT_WORK_ROOT)
    parser.add_argument("--json", type=Path, help="write the raw results here")
    parser.add_argument("--rendezvous", type=int, nargs="?", const=2, default=0, metavar="N",
                        help="mission 14: fleets 2..N also head for the rendezvous (N=2)")
    parser.add_argument("--variant", default="", choices=["", *TARGETS],
                        help="aim the order at another target")
    parser.add_argument("--base-move", default="", choices=["", *BASE_MOVES],
                        help="also order the starbase (created at the target) to move")
    parser.add_argument("--retarget", metavar="MISSION:X,Y[:YEAR]",
                        help="change fleet 1's order after YEAR (default 1)")
    args = parser.parse_args()
    if args.rendezvous and args.missions != "14":
        raise SystemExit("--rendezvous applies to mission 14 only")
    prepare = rendezvous_partners(args.rendezvous) if args.rendezvous else None
    retarget = None
    if args.retarget:
        order, coords, *after = args.retarget.split(":")
        x, y = map(int, coords.split(","))
        retarget = (int(order), (x, y), int(after[0]) if after else 1)
    if not NC_CLI_BIN.exists():
        print(f"build nc-cli first: {NC_CLI_BIN}", file=sys.stderr)
        return 1
    cases = [Case(int(m), int(s), o, args.years, args.variant, args.base_move)
             for m in args.missions.split(",")
             for s in args.speeds.split(",") for o in args.offsets.split(",")]
    with ThreadPoolExecutor(args.jobs) as pool:
        results = list(pool.map(
            lambda c: run_case(c, args.work_root, prepare=prepare, retarget=retarget), cases))
    mismatched = 0
    for result in results:
        if "skipped" in result:
            print(f"{result['case']}: skipped, {result['skipped']}")
            continue
        lines = diff(result)
        mismatched += bool(lines)
        print(f"{result['case']}: {'MISMATCH' if lines else 'match'}")
        if lines:
            print("\n".join(lines))
    if args.json:
        args.json.write_text(json.dumps(results, indent=1))
    print(f"{mismatched} of {len(results)} cases mismatched", file=sys.stderr)
    return 1 if mismatched else 0


if __name__ == "__main__":
    raise SystemExit(main())
