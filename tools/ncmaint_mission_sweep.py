#!/usr/bin/env python3
"""Trace one fleet per mission through classic ECMAINT and Rust, year by year.

Each case gives empire 1's fleet 1 one order, written with the mission aux
bytes ECGAME writes, from a start offset of the target, then runs N
maintenance years on both engines. Rows record the fleet's position, order,
speed, aux bytes, and tuple c (0x19..0x1e decoded as a Turbo Pascal real),
plus the target planet record, so the year a mission enters the target
sector, arms, and resolves can be read off directly.

    nix shell nixpkgs#dosbox-x -c python3 tools/ncmaint_mission_sweep.py \
        --missions 5,7 --speeds 1,3 --offsets d3,d6,s3 --years 8
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from pathlib import Path

from ncmaint_oracle import DEFAULT_BASELINE, ROOT, run_ecmaint

NC_CLI_BIN = ROOT / "rust" / "target" / "debug" / "nc-cli"
ORIGINAL = ROOT / "original" / "v1.5"
DEFAULT_WORK_ROOT = Path("/tmp/ncmaint-mission-sweep")
FLEET_RECORD = 54
PLANET_RECORD = 97

HOME = (16, 13)  # empire 1's home world
SECTOR = (13, 13)  # empty deep-space sector
WORLD = (12, 12)  # unowned planet
ENEMY = (15, 13)  # empire 2's world in the "bombard" scenario

# fleet-ships order: sc bb ca dd tt armies etac
CRUISER = (0, 0, 1, 0, 0, 0, 0)


@dataclass(frozen=True)
class Mission:
    scenario: str
    target: tuple[int, int]
    ships: tuple[int, ...]
    aux: tuple[int, int]
    roe: int | None = None
    scout: bool = False
    starbase: bool = False


# Aux bytes as ECGAME writes them: 0x22 is 1 for a world target, 0x23 is the
# guarded starbase id or the join host fleet id.
MISSIONS = {
    1: Mission("move", SECTOR, CRUISER, (0, 0)),
    2: Mission("move", HOME, CRUISER, (1, 0)),
    3: Mission("move", SECTOR, CRUISER, (0, 0)),
    4: Mission("move", HOME, CRUISER, (1, 1), starbase=True),
    5: Mission("move", HOME, CRUISER, (1, 0)),
    6: Mission("bombard", ENEMY, CRUISER, (1, 0)),
    7: Mission("bombard", ENEMY, (0, 0, 1, 0, 2, 2, 0), (1, 0)),
    8: Mission("bombard", ENEMY, (0, 0, 0, 0, 2, 2, 0), (1, 0), roe=0),
    9: Mission("move", WORLD, CRUISER, (1, 0)),
    10: Mission("move", SECTOR, CRUISER, (0, 0), roe=0, scout=True),
    11: Mission("move", WORLD, CRUISER, (1, 0), roe=0, scout=True),
    12: Mission("move", WORLD, (0, 0, 1, 0, 0, 0, 1), (1, 0)),
    13: Mission("move", HOME, CRUISER, (1, 2)),  # join fleet 2, parked at home
    14: Mission("move", SECTOR, CRUISER, (0, 0)),
    15: Mission("move", HOME, CRUISER, (1, 0)),
}

# Mission 4 with the starbase over the unowned world instead of the home
# world: with no friendly world beneath it, the base climbs to deep space in
# the first year and its guards follow its depth (--deep-base).
DEEP_BASE = Mission("move", WORLD, CRUISER, (1, 1), starbase=True)


@dataclass(frozen=True)
class Case:
    mission: int
    speed: int
    offset: str  # "d3" = 3 sectors diagonally, "s3" = 3 sectors in x
    years: int
    deep_base: bool = False

    @property
    def name(self) -> str:
        suffix = "-deep" if self.deep_base else ""
        return f"m{self.mission:02d}-v{self.speed}-{self.offset}{suffix}"

    @property
    def spec(self) -> Mission:
        return DEEP_BASE if self.deep_base else MISSIONS[self.mission]

    def start(self) -> tuple[int, int]:
        tx, ty = self.spec.target
        n = int(self.offset[1:])
        return (tx - n, ty - n) if self.offset[0] == "d" else (tx - n, ty)


def turbo_real(raw: bytes) -> float:
    """Decode a Turbo Pascal 6-byte real (exponent byte first)."""
    if raw[0] == 0:
        return 0.0
    mantissa = int.from_bytes(raw[1:6], "little")
    sign = -1.0 if mantissa >> 39 else 1.0
    mantissa &= (1 << 39) - 1
    return sign * (1 + mantissa / 2**39) * 2.0 ** (raw[0] - 129)


def nc(*args: object) -> str:
    result = subprocess.run(
        [str(NC_CLI_BIN), *map(str, args)], cwd=ROOT / "rust", text=True, capture_output=True
    )
    if result.returncode != 0:
        raise SystemExit(f"nc-cli {args} failed:\n{result.stdout}\n{result.stderr}")
    return result.stdout


def patch_fleet(directory: Path, record: int, changes: dict[int, int]) -> None:
    path = directory / "FLEETS.DAT"
    data = bytearray(path.read_bytes())
    for offset, value in changes.items():
        data[(record - 1) * FLEET_RECORD + offset] = value
    path.write_bytes(bytes(data))


def snapshot(directory: Path, year: int, target: tuple[int, int]) -> dict:
    data = (directory / "FLEETS.DAT").read_bytes()
    fleets = [data[i : i + FLEET_RECORD] for i in range(0, len(data), FLEET_RECORD)]
    fleet = next((r for r in fleets if r[0x02] == 1 and r[0x05] == 1), None)
    planets = (directory / "PLANETS.DAT").read_bytes()
    planet = next(
        (
            planets[i : i + PLANET_RECORD]
            for i in range(0, len(planets), PLANET_RECORD)
            if (planets[i], planets[i + 1]) == target
        ),
        None,
    )
    row: dict = {
        "year": year,
        "empire1_fleets": sum(1 for r in fleets if r[0x02] == 1),
        "planet": planet.hex() if planet else None,
        "planet_owner": planet[0x5D] if planet else None,
        "planet_armies": planet[0x58] if planet else None,
        "planet_batteries": planet[0x5A] if planet else None,
    }
    if fleet is not None:
        row.update(
            pos=[fleet[0x0B], fleet[0x0C]],
            speed=fleet[0x0A],
            order=fleet[0x1F],
            target=[fleet[0x20], fleet[0x21]],
            aux=[fleet[0x22], fleet[0x23]],
            roe=fleet[0x25],
            tuple_c=fleet[0x19:0x1F].hex(" "),
            tuple_c_real=round(turbo_real(fleet[0x19:0x1F]), 6),
            x_frac=round(turbo_real(fleet[0x0D:0x13]), 6),
            y_frac=round(turbo_real(fleet[0x13:0x19]), 6),
            raw=fleet.hex(),
        )
    return row


def build_classic(case: Case, directory: Path, human: bool = False) -> None:
    mission = case.spec
    if directory.exists():
        shutil.rmtree(directory)
    directory.parent.mkdir(parents=True, exist_ok=True)
    nc("scenario-init-replayable", DEFAULT_BASELINE, directory, mission.scenario)
    if mission.starbase:
        nc("guard-starbase-onebase", directory, *mission.target)
    if mission.scenario == "bombard":
        # Park the scenario's own attacker at home, out of the way.
        nc("fleet-ships", directory, 3, *CRUISER)
        nc("fleet-order", directory, 3, 0, 5, *HOME, 1, 0)
    nc("fleet-ships", directory, 1, *mission.ships)
    nc("fleet-location", directory, 1, *case.start())
    if human:
        # Neither engine reports to an empire in civil disorder.
        nc("player-name", directory, 1, "probe", "Probe Empire")
    nc("db-export", directory, directory)
    shutil.copy2(ORIGINAL / "ECMAINT.EXE", directory / "ECMAINT.EXE")
    tx, ty = mission.target
    changes = {
        0x0A: case.speed,
        0x1F: case.mission,
        0x20: tx,
        0x21: ty,
        0x22: mission.aux[0],
        0x23: mission.aux[1],
    }
    if mission.roe is not None:
        changes[0x25] = mission.roe
    if mission.scout:
        # One scout, no cruiser. Classic counts scouts at 0x32.
        changes.update({0x28: 0, 0x32: 1, 0x09: 6})
    patch_fleet(directory, 1, changes)


def build_rust(case: Case, classic: Path, directory: Path) -> None:
    if directory.exists():
        shutil.rmtree(directory)
    directory.mkdir(parents=True)
    for source in classic.glob("*.DAT"):
        shutil.copy2(source, directory / source.name)
    if case.spec.scout:
        patch_fleet(directory, 1, {0x24: 1})  # Rust counts scouts at 0x24
    nc("db-import", directory)


def run_case(case: Case, work_root: Path, rust: bool) -> dict:
    root = work_root / case.name
    target = case.spec.target
    classic = root / "classic"
    build_classic(case, classic)
    classic_rows = [snapshot(classic, 0, target)]
    rust_rows: list[dict] = []
    if rust:
        rust_dir, export = root / "rust", root / "rust-export"
        build_rust(case, classic, rust_dir)
        nc("db-export", rust_dir, export)
        rust_rows.append(snapshot(export, 0, target))
    for year in range(1, case.years + 1):
        result = run_ecmaint(classic)
        if result.returncode != 0:
            raise SystemExit(f"ECMAINT failed in {classic}:\n{result.stdout}{result.stderr}")
        classic_rows.append(snapshot(classic, year, target))
        if rust:
            nc("maint-rust", rust_dir, 1)
            nc("db-export", rust_dir, export)
            rust_rows.append(snapshot(export, year, target))
    trace = {
        "case": case.name,
        "mission": case.mission,
        "speed": case.speed,
        "offset": case.offset,
        "start": case.start(),
        "target": target,
        "classic": classic_rows,
        "rust": rust_rows,
    }
    (root / "trace.json").write_text(json.dumps(trace, indent=1))
    return trace


def milestones(rows: list[dict], target: tuple[int, int]) -> dict:
    """Years the fleet enters the target sector, arms, settles, and resolves."""
    first = rows[0]
    out: dict = {"enter": None, "armed": None, "settled": None, "order_change": None,
                 "planet_change": None, "gone": None}
    for prev, row in zip(rows, rows[1:]):
        year = row["year"]
        if "pos" not in row:
            out["gone"] = out["gone"] or year
            continue
        if out["enter"] is None and tuple(row["pos"]) == tuple(target):
            out["enter"] = year
        if out["armed"] is None and row["tuple_c"].startswith("80"):
            out["armed"] = year
        if out["settled"] is None and row["tuple_c"].startswith("81") and row["speed"] == 0:
            out["settled"] = year
        if out["order_change"] is None and row["order"] != first["order"]:
            out["order_change"] = year
        if out["planet_change"] is None and row["planet"] != prev["planet"]:
            out["planet_change"] = year
    return out


def summary_line(trace: dict) -> str:
    target = tuple(trace["target"])
    c = milestones(trace["classic"], target)
    line = f"{trace['case']:<16} classic {fmt(c)}"
    if trace["rust"]:
        line += f"   rust {fmt(milestones(trace['rust'], target))}"
    return line


def fmt(m: dict) -> str:
    return " ".join(f"{k}={'-' if v is None else v}" for k, v in m.items())


def parse_cases(args: argparse.Namespace) -> list[Case]:
    missions = [int(m) for m in args.missions.split(",")]
    speeds = [int(s) for s in args.speeds.split(",")]
    offsets = args.offsets.split(",")
    if args.deep_base and missions != [4]:
        raise SystemExit("--deep-base applies to mission 4 only")
    return [
        Case(m, s, o, args.years, args.deep_base)
        for m in missions
        for s in speeds
        for o in offsets
    ]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--missions", default=",".join(map(str, MISSIONS)))
    parser.add_argument("--speeds", default="1,3")
    parser.add_argument("--offsets", default="d3,d6")
    parser.add_argument("--years", type=int, default=9)
    parser.add_argument("--jobs", type=int, default=4)
    parser.add_argument("--work-root", type=Path, default=DEFAULT_WORK_ROOT)
    parser.add_argument("--classic-only", action="store_true", help="skip the Rust engine")
    parser.add_argument(
        "--deep-base",
        action="store_true",
        help="mission 4 only: put the starbase over an unowned world, so it climbs to deep space",
    )
    args = parser.parse_args()
    if not NC_CLI_BIN.exists():
        print(f"build nc-cli first: {NC_CLI_BIN}", file=sys.stderr)
        return 1
    cases = parse_cases(args)
    with ThreadPoolExecutor(args.jobs) as pool:
        for trace in pool.map(lambda c: run_case(c, args.work_root, not args.classic_only), cases):
            print(summary_line(trace), flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
