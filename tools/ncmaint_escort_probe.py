#!/usr/bin/env python3
"""Classic ECMAINT probes for t094: escort release, unarmed guards, and
worldless rogues.

Each group builds a scenario on the move fixture (empire 1 at (16,13),
empire 2 at (4,13), empire 3 at (6,5), empire 4 at (13,5); unowned worlds at
(12,12), (12,11), (18,15), (2,16)), runs ECMAINT for a few years under
DOSBox-X, and prints the fleet, starbase, and empire records after each
year. With --rust it runs the same directory through `nc-cli maint-rust`
and prints those records too, so the two engines can be compared by eye.

    nix shell nixpkgs#dosbox-x -c python3 tools/ncmaint_escort_probe.py \
        --group merge --years 1 --rust

Groups:

- `scatter`: an escorting Guard Starbase fleet attacks a lone enemy
  destroyer. Over repeated runs it wins, is destroyed, or scatters
  (`1b05:2562`) or retreats; the base's escort word shows whether the fleet
  let go of it.
- `destroy`: the same with no transports, so a lost fight deletes the fleet
  (`1b05:9927`, `2895:3d0c`).
- `merge`: a rogue empire's Guard Starbase fleet meets another of its fleets
  at its world; the fleets merge (`24a3:216f`, `2895:4a6b`) and the merged
  fleet is deleted (`2895:3d0c`).
- `abort`: a bombard fleet dropping into orbit of an enemy world whose guard
  its ROE forbids fighting aborts (`1b05:185e`); a base in its sector holds
  its number as escort word.
- `etac`: ETAC-only fleets on Guard/Blockade at an empire's world with an
  unescorted base: rogue, autopilot, and human, three years.
- `worldless`: a rogue empire with no world and warships patrols; with no
  fleet it is left alone (`24a3:0c2a`, `24a3:0dbc`), three years.
"""

from __future__ import annotations

import argparse
import shutil
import struct
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

from ncmaint_oracle import ROOT, ensure_engine, run_ecmaint

FIXTURE = ROOT / "fixtures" / "ecmaint-move-pre" / "v1.5"
FLEET, BASE, PLAYER, PLANET = 54, 35, 110, 97

HOME = {1: (16, 13), 2: (4, 13), 3: (6, 5), 4: (13, 5)}
WORLD = (12, 12)
WORLD2 = (12, 11)


def real(value: float) -> bytes:
    """Encode a Turbo Pascal 6-byte real (only 0, 0.5, and 1 are needed)."""
    return {0.0: bytes(6), 0.5: bytes([0x80, 0, 0, 0, 0, 0]), 1.0: bytes([0x81, 0, 0, 0, 0, 0])}[value]


def unreal(raw: bytes) -> float:
    if raw[0] == 0:
        return 0.0
    mantissa = int.from_bytes(raw[1:6], "little")
    sign = -1.0 if mantissa >> 39 else 1.0
    mantissa &= (1 << 39) - 1
    return sign * (1 + mantissa / 2**39) * 2.0 ** (raw[0] - 129)


@dataclass
class Fleet:
    owner: int
    number: int
    sector: tuple[int, int]
    z: float = 1.0
    order: int = 0
    target: tuple[int, int] | None = None
    tz: int = 1
    mission: int = 0
    roe: int = 0
    speed: int = 0
    # battleships, cruisers, destroyers, empty TT, loaded TT, ETACs, scouts
    ships: tuple[int, ...] = (0, 0, 0, 0, 0, 0, 0)

    def raw(self, template: bytes) -> bytearray:
        record = bytearray(template)
        speeds = [4, 5, 6, 5, 5, 3, 6]
        present = [s for count, s in zip(self.ships, speeds) if count]
        record[0x00:0x02] = struct.pack("<H", self.number)
        record[0x02] = self.owner
        record[0x09] = min(present) if present else 0
        record[0x0A] = self.speed
        record[0x0B], record[0x0C] = self.sector
        record[0x0D:0x13] = real(0.5)
        record[0x13:0x19] = real(0.5)
        record[0x19:0x1F] = real(self.z)
        record[0x1F] = self.order
        record[0x20], record[0x21] = self.target or self.sector
        record[0x22] = self.tz
        record[0x23:0x25] = struct.pack("<H", self.mission)
        record[0x25] = self.roe
        record[0x26:0x34] = struct.pack("<7H", *self.ships)
        return record


@dataclass
class Base:
    owner: int
    base_id: int
    sector: tuple[int, int]
    escort: int = 0
    z: float = 1.0
    moving: int = 0
    target: tuple[int, int] | None = None
    tz: int = 1

    def raw(self) -> bytearray:
        record = bytearray(BASE)
        record[0x00:0x02] = struct.pack("<H", self.escort)
        record[0x02:0x04] = struct.pack("<H", self.base_id)
        record[0x04] = self.owner
        record[0x0B], record[0x0C] = self.sector
        record[0x0D:0x13] = real(0.5)
        record[0x13:0x19] = real(0.5)
        record[0x19:0x1F] = real(self.z)
        record[0x1F] = self.moving
        record[0x20], record[0x21] = self.target or self.sector
        record[0x22] = self.tz
        return record


@dataclass
class Scenario:
    """The move fixture with fleets, bases, and ownership replaced."""

    modes: dict[int, int] = field(default_factory=lambda: {1: 1, 2: 0, 3: 0, 4: 0})
    autopilot: set[int] = field(default_factory=set)
    enemies: set[tuple[int, int]] = field(default_factory=set)
    owners: dict[tuple[int, int], int] = field(default_factory=dict)
    fleets: list[Fleet] = field(default_factory=list)
    bases: list[Base] = field(default_factory=list)

    def write(self, target: Path) -> None:
        if target.exists():
            shutil.rmtree(target)
        shutil.copytree(FIXTURE, target)
        ensure_engine(target)
        template = (FIXTURE / "FLEETS.DAT").read_bytes()[:FLEET]
        players = bytearray((FIXTURE / "PLAYER.DAT").read_bytes())
        planets = bytearray((FIXTURE / "PLANETS.DAT").read_bytes())
        for index in range(len(planets) // PLANET):
            record = planets[index * PLANET : (index + 1) * PLANET]
            sector = (record[0], record[1])
            if sector in self.owners:
                record[0x5D] = self.owners[sector]
                record[0x5C] = 2
                planets[index * PLANET : (index + 1) * PLANET] = record
        fleets = bytearray()
        bases = bytearray()
        for empire in range(1, 5):
            player = players[(empire - 1) * PLAYER : empire * PLAYER]
            player[0] = self.modes[empire]
            player[0x6D] = 1 if empire in self.autopilot else 0
            player[0x50] = sum(1 for owner in self.owners.values() if owner == empire)
            for other in range(1, 5):
                player[0x53 + other] = 1 if (empire, other) in self.enemies else 0
            own = [f for f in self.fleets if f.owner == empire]
            first = len(fleets) // FLEET + 1 if own else 0
            for i, fleet in enumerate(own):
                number = first + i
                record = fleet.raw(template)
                record[0x05:0x07] = struct.pack("<H", number)
                record[0x03:0x05] = struct.pack("<H", number + 1 if i + 1 < len(own) else 0)
                record[0x07:0x09] = struct.pack("<H", number - 1 if i else 0)
                fleets += record
            player[0x40:0x42] = struct.pack("<H", first)
            player[0x42:0x44] = struct.pack("<H", first + len(own) - 1 if own else 0)
            mine = sorted((b for b in self.bases if b.owner == empire), key=lambda b: b.base_id)
            first_base = len(bases) // BASE + 1 if mine else 0
            for i, base in enumerate(mine):
                number = first_base + i
                record = base.raw()
                record[0x07:0x09] = struct.pack("<H", number)
                record[0x05:0x07] = struct.pack("<H", number + 1 if i + 1 < len(mine) else 0)
                record[0x09:0x0B] = struct.pack("<H", number - 1 if i else 0)
                bases += record
            player[0x44:0x46] = struct.pack("<H", first_base)
            player[0x46:0x48] = struct.pack("<H", first_base + len(mine) - 1 if mine else 0)
            players[(empire - 1) * PLAYER : empire * PLAYER] = player
        (target / "PLAYER.DAT").write_bytes(bytes(players))
        (target / "PLANETS.DAT").write_bytes(bytes(planets))
        (target / "FLEETS.DAT").write_bytes(bytes(fleets))
        (target / "BASES.DAT").write_bytes(bytes(bases))


def dump(directory: Path) -> list[str]:
    """One line per record that matters, from the classic files."""
    lines = []
    players = (directory / "PLAYER.DAT").read_bytes()
    for empire in range(1, len(players) // PLAYER + 1):
        p = players[(empire - 1) * PLAYER : empire * PLAYER]
        enemies = [k for k in range(1, 5) if p[0x53 + k]]
        lines.append(
            f"  P{empire} mode={p[0]:02x} worlds={p[0x50]} autopilot={p[0x6D]} enemies={enemies}"
            f" fleets={struct.unpack('<HH', p[0x40:0x44])} bases={struct.unpack('<HH', p[0x44:0x48])}"
        )
    data = (directory / "FLEETS.DAT").read_bytes()
    for i in range(len(data) // FLEET):
        r = data[i * FLEET : (i + 1) * FLEET]
        if r[0x02] == 0:
            continue
        ships = struct.unpack("<7H", r[0x26:0x34])
        lines.append(
            f"  F{r[0x02]}.{struct.unpack('<H', r[0:2])[0]} at ({r[0x0B]},{r[0x0C]}) z={unreal(r[0x19:0x1F]):.2f}"
            f" order={r[0x1F]} target=({r[0x20]},{r[0x21]},{r[0x22]}) mt={struct.unpack('<H', r[0x23:0x25])[0]}"
            f" roe={r[0x25]} speed={r[0x0A]} ships(bb,ca,dd,tt,ttl,etac,sc)={ships}"
        )
    data = (directory / "BASES.DAT").read_bytes()
    for i in range(len(data) // BASE):
        r = data[i * BASE : (i + 1) * BASE]
        if r[0x04] == 0:
            continue
        lines.append(
            f"  B{r[0x04]}.{struct.unpack('<H', r[2:4])[0]} at ({r[0x0B]},{r[0x0C]}) z={unreal(r[0x19:0x1F]):.2f}"
            f" escort={struct.unpack('<H', r[0:2])[0]} moving={r[0x1F]} target=({r[0x20]},{r[0x21]},{r[0x22]})"
        )
    return lines


def run_rust(directory: Path) -> Path:
    """One Rust year on the directory; the classic files it exports."""
    exported = directory / "rust-export"
    if exported.exists():
        shutil.rmtree(exported)
    for command in (["maint-rust", str(directory), "1"], ["db-export", str(directory), str(exported)]):
        subprocess.run(
            ["cargo-slot", "nostrian-conquest", "cargo", "run", "-q", "-p", "nc-cli", "--", *command],
            cwd=ROOT / "rust",
            check=True,
            capture_output=True,
        )
    return exported


def play(name: str, scenario: Scenario, years: int, work: Path, engines: tuple[str, ...]) -> None:
    for engine in engines:
        target = work / f"{name}-{engine}"
        scenario.write(target)
        print(f"== {name} ({engine})")
        print("  year 0")
        print("\n".join(dump(target)))
        for year in range(1, years + 1):
            shown = target
            if engine == "classic":
                run_ecmaint(target)
            else:
                shown = run_rust(target)
            print(f"  year {year}")
            print("\n".join(dump(shown)))


def home_scenario(modes: dict[int, int]) -> Scenario:
    """Empires own their home worlds and keep one idle cruiser at each."""
    scenario = Scenario(modes=modes)
    for empire, sector in HOME.items():
        scenario.owners[sector] = empire
    return scenario


def group_scatter(n: int, transports: int = 3, name: str = "scatter") -> list[tuple[str, Scenario]]:
    scenario = home_scenario({1: 1, 2: 0, 3: 0, 4: 0})
    scenario.enemies.add((1, 2))
    guard = Fleet(1, 2, HOME[1], order=4, mission=1, roe=6, ships=(0, 0, 1, transports, 0, 0, 0))
    intruder = Fleet(2, 1, HOME[1], order=0, roe=0, ships=(0, 0, 1, 0, 0, 0, 0))
    scenario.fleets += [guard, intruder]
    scenario.bases.append(Base(1, 1, HOME[1], escort=2))
    for empire in (3, 4):
        scenario.fleets.append(Fleet(empire, 1, HOME[empire], order=5, target=HOME[empire], roe=6, ships=(0, 1, 0, 0, 0, 0, 0)))
    return [(f"{name}{i}", scenario) for i in range(n)]


def group_destroy(n: int) -> list[tuple[str, Scenario]]:
    return group_scatter(n, transports=0, name="destroy")


def group_merge(_: int) -> list[tuple[str, Scenario]]:
    scenario = home_scenario({1: 1, 2: 0xFF, 3: 0, 4: 0})
    guard_low = Fleet(2, 1, HOME[2], order=5, target=HOME[2], roe=10, ships=(0, 0, 1, 0, 0, 0, 0))
    guard_high = Fleet(2, 2, HOME[2], order=4, mission=1, roe=10, ships=(0, 0, 1, 0, 0, 0, 0))
    scenario.fleets += [guard_low, guard_high]
    scenario.bases.append(Base(2, 1, HOME[2], escort=2))
    scenario.fleets.append(Fleet(1, 1, HOME[1], order=0, ships=(0, 1, 0, 0, 0, 0, 0)))
    return [("merge", scenario)]


def group_abort(n: int) -> list[tuple[str, Scenario]]:
    scenario = home_scenario({1: 1, 2: 0, 3: 0, 4: 0})
    bombard = Fleet(1, 1, HOME[2], z=0.0, order=6, target=HOME[2], speed=3, roe=1, ships=(0, 1, 0, 0, 0, 0, 0))
    scenario.fleets.append(bombard)
    scenario.bases.append(Base(1, 1, HOME[2], escort=1))
    scenario.fleets.append(Fleet(2, 1, HOME[2], order=5, target=HOME[2], roe=0, ships=(0, 1, 0, 0, 0, 0, 0)))
    return [(f"abort{i}", scenario) for i in range(n)]


def group_etac(_: int) -> list[tuple[str, Scenario]]:
    out = []
    for label, empire, mode, autopilot in (("rogue", 2, 0xFF, False), ("autopilot", 1, 1, True), ("human", 1, 1, False)):
        modes = {1: 1, 2: 0, 3: 0, 4: 0}
        modes[empire] = mode
        scenario = home_scenario(modes)
        if autopilot:
            scenario.autopilot.add(empire)
        scenario.fleets.append(Fleet(empire, 1, HOME[empire], order=5, target=HOME[empire], roe=6, ships=(0, 0, 0, 0, 0, 1, 0)))
        scenario.bases.append(Base(empire, 1, HOME[empire]))
        for other in range(1, 5):
            if other != empire:
                scenario.fleets.append(Fleet(other, 1, HOME[other], order=0, ships=(0, 1, 0, 0, 0, 0, 0)))
        out.append((f"etac-{label}", scenario))
    return out


def group_worldless(_: int) -> list[tuple[str, Scenario]]:
    out = []
    for label, with_fleets in (("fleets", True), ("nofleets", False)):
        scenario = Scenario(modes={1: 1, 2: 0xFF, 3: 0, 4: 0})
        scenario.owners[HOME[2]] = 0
        for empire in (1, 3, 4):
            scenario.owners[HOME[empire]] = empire
            scenario.fleets.append(Fleet(empire, 1, HOME[empire], order=0, ships=(0, 1, 0, 0, 0, 0, 0)))
        if with_fleets:
            scenario.fleets.append(Fleet(2, 1, (8, 13), z=0.0, order=0, roe=3, ships=(0, 1, 0, 0, 0, 0, 0)))
            scenario.fleets.append(Fleet(2, 2, (9, 13), z=0.0, order=0, ships=(0, 0, 0, 0, 0, 1, 0)))
        scenario.bases.append(Base(2, 1, HOME[2]))
        out.append((f"worldless-{label}", scenario))
    return out


GROUPS = {
    "scatter": (group_scatter, 1),
    "destroy": (group_destroy, 1),
    "merge": (group_merge, 1),
    "abort": (group_abort, 1),
    "etac": (group_etac, 3),
    "worldless": (group_worldless, 3),
}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--group", choices=sorted(GROUPS), required=True)
    parser.add_argument("--years", type=int, help="maintenance years per scenario")
    parser.add_argument("--runs", type=int, default=6, help="repeated runs (scatter only)")
    parser.add_argument("--rust", action="store_true", help="also run nc-cli maint-rust")
    parser.add_argument("--rust-only", action="store_true", help="run only nc-cli maint-rust")
    parser.add_argument("--work", type=Path, default=Path("/tmp/ncmaint-escort-probe"))
    args = parser.parse_args()
    build, default_years = GROUPS[args.group]
    engines = ("rust",) if args.rust_only else ("classic", "rust") if args.rust else ("classic",)
    for name, scenario in build(args.runs):
        play(name, scenario, args.years or default_years, args.work, engines)
    return 0


if __name__ == "__main__":
    sys.exit(main())
