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

Groups for t099 (merge pass, hold depth, planetless civil disorder):

- `mergeplace`: a rogue empire's fleets meet at its home world, at another
  world of its own, over an unowned world, in deep space, and in the deep
  layer of its home sector; the merge pass (`24a3:216f`) joins only the
  ones in orbit of its worlds.
- `mergeauto`: a human empire on autopilot with five worlds: Hold and
  Guard/Blockade fleets merge, Guard Starbase hosts need a base at rest,
  Guard Starbase fleets are never absorbed, moving and Move fleets are left.
- `mergecap`: two fleets of 3000 ships merge, two of 3001 do not
  (`2895:3057`).
- `holdlost`: a human empire's fleet whose order needs ships it lacks holds
  (`0b5d:07f1` and the other order tests); the target depth, target, and
  mission word it keeps.
- `civil`: a civil-disorder empire with no planet becomes a rogue when the
  orders pass starts (`24a3:0c2a`); one with a planet does not.
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
    names: dict[int, str] = field(default_factory=dict)
    handles: dict[int, str] = field(default_factory=dict)

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
            if empire in self.names:
                name = self.names[empire].encode()
                player[0x1B] = len(name)
                player[0x1C : 0x1C + len(name)] = name
            if empire in self.handles:
                player[0x01:0x1A] = self.handles[empire].ljust(25).encode()
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
        name = p[0x1C : 0x1C + p[0x1B]].decode(errors="replace")
        handle = p[0x01:0x1A].decode(errors="replace").rstrip()
        lines.append(
            f"  P{empire} mode={p[0]:02x} worlds={p[0x50]} autopilot={p[0x6D]} enemies={enemies}"
            f" name={name!r} handle={handle!r}"
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


OTHER_WORLDS = [(12, 12), (12, 11), (18, 15), (2, 16)]


def quiet_neighbours(scenario: Scenario, empires: tuple[int, ...]) -> None:
    """Each listed empire owns its home world and keeps one idle cruiser there."""
    for empire in empires:
        scenario.owners[HOME[empire]] = empire
        scenario.fleets.append(Fleet(empire, 1, HOME[empire], order=0, ships=(0, 1, 0, 0, 0, 0, 0)))


def numbered(fleets: list[Fleet]) -> list[Fleet]:
    """Give one empire's fleets the numbers 1.. in list order."""
    for number, fleet in enumerate(fleets, start=1):
        fleet.number = number
    return fleets


def group_mergeplace(_: int) -> list[tuple[str, Scenario]]:
    """Rogue empire 2 owns (4,13) and (12,12); its fleets sit at five kinds of place."""
    scenario = Scenario(modes={1: 1, 2: 0xFF, 3: 0, 4: 0})
    scenario.owners[HOME[2]] = 2
    scenario.owners[WORLD] = 2
    quiet_neighbours(scenario, (1, 3, 4))
    cruiser, destroyer, etac = (0, 1, 0, 0, 0, 0, 0), (0, 0, 1, 0, 0, 0, 0), (0, 0, 0, 0, 0, 1, 0)
    rogue = [
        # home world, orbit: a Move fleet under way, a Hold fleet, a Guard/Blockade fleet
        Fleet(2, 0, HOME[2], order=1, target=(8, 13), speed=3, roe=3, ships=cruiser),
        Fleet(2, 0, HOME[2], order=0, ships=destroyer),
        Fleet(2, 0, HOME[2], order=5, target=HOME[2], ships=etac),
        # another world of its own, orbit: Patrol and Seek Home; deep layer: one more
        Fleet(2, 0, WORLD, order=3, target=WORLD, ships=destroyer),
        Fleet(2, 0, WORLD, order=2, target=HOME[2], speed=3, ships=cruiser),
        Fleet(2, 0, WORLD, z=0.0, order=0, ships=destroyer),
        # an unowned world, orbit
        Fleet(2, 0, WORLD2, order=0, ships=destroyer),
        Fleet(2, 0, WORLD2, order=0, ships=cruiser),
        # deep space
        Fleet(2, 0, (8, 13), z=0.0, order=0, ships=destroyer),
        Fleet(2, 0, (8, 13), z=0.0, order=0, ships=cruiser),
        # home sector, deep layer
        Fleet(2, 0, HOME[2], z=0.0, order=0, ships=destroyer),
        Fleet(2, 0, HOME[2], z=0.0, order=0, ships=cruiser),
    ]
    scenario.fleets += numbered(rogue)
    return [("mergeplace", scenario)]


def group_mergeauto(_: int) -> list[tuple[str, Scenario]]:
    """Human empire 1 on autopilot owns five worlds; its fleets sit at one each."""
    scenario = Scenario(modes={1: 1, 2: 0, 3: 0, 4: 0})
    scenario.autopilot.add(1)
    sites = [HOME[1], *OTHER_WORLDS]
    for site in sites:
        scenario.owners[site] = 1
    for empire in (2, 3, 4):
        scenario.owners[HOME[empire]] = empire
        scenario.fleets.append(Fleet(empire, 1, HOME[empire], order=0, ships=(0, 1, 0, 0, 0, 0, 0)))
    cruiser, destroyer = (0, 1, 0, 0, 0, 0, 0), (0, 0, 1, 0, 0, 0, 0)
    a, b, c, d, e = sites
    mine = [
        # a: Hold, Hold, Guard/Blockade
        Fleet(1, 0, a, order=0, ships=cruiser),
        Fleet(1, 0, a, order=0, ships=destroyer),
        Fleet(1, 0, a, order=5, target=a, roe=6, ships=cruiser),
        # b: a Guard Starbase host over a base at rest, then a Hold fleet
        Fleet(1, 0, b, order=4, target=b, mission=1, roe=6, ships=destroyer),
        Fleet(1, 0, b, order=0, ships=cruiser),
        # c: a Hold fleet, then a Guard Starbase host over a base at rest
        Fleet(1, 0, c, order=0, ships=cruiser),
        Fleet(1, 0, c, order=4, target=c, mission=2, roe=6, ships=destroyer),
        # d: a Guard Starbase host whose base is lifting, a Hold fleet, a Move fleet
        Fleet(1, 0, d, order=4, target=d, mission=3, roe=6, ships=destroyer),
        Fleet(1, 0, d, order=0, ships=cruiser),
        Fleet(1, 0, d, order=1, target=d, ships=destroyer),
        # e: Guard/Blockade at rest, Guard/Blockade setting out for another world
        Fleet(1, 0, e, order=5, target=e, roe=6, ships=cruiser),
        Fleet(1, 0, e, order=5, target=b, speed=2, roe=6, ships=destroyer),
    ]
    scenario.fleets += [f for f in numbered(mine)]
    scenario.bases += [
        Base(1, 1, b, escort=4),
        Base(1, 2, c, escort=7),
        Base(1, 3, d, escort=8, tz=0),
    ]
    return [("mergeauto", scenario)]


def group_mergeguard(_: int) -> list[tuple[str, Scenario]]:
    """Rogue empire 2: Guard Starbase fleets over a base at rest, a lifting base, and no base."""
    scenario = Scenario(modes={1: 1, 2: 0xFF, 3: 0, 4: 0})
    sites = [HOME[2], WORLD, WORLD2, OTHER_WORLDS[2]]
    for site in sites:
        scenario.owners[site] = 2
    quiet_neighbours(scenario, (1, 3, 4))
    destroyer = (0, 0, 1, 0, 0, 0, 0)
    a, b, c, d = sites
    scenario.fleets += numbered(
        [
            # a: base at rest; the guard hosts, the Hold fleet is taken in
            Fleet(2, 0, a, order=4, target=a, mission=1, roe=10, ships=destroyer),
            Fleet(2, 0, a, order=0, ships=destroyer),
            # b: base lifting (TZ 0); the guard cannot host but the Hold fleet takes it in
            Fleet(2, 0, b, order=4, target=b, mission=2, roe=10, ships=destroyer),
            Fleet(2, 0, b, order=0, ships=destroyer),
            # c: the guard names a base that does not exist, after a Hold host
            Fleet(2, 0, c, order=0, ships=destroyer),
            Fleet(2, 0, c, order=4, target=c, mission=9, roe=10, ships=destroyer),
            # d: a guard over a lifting base, then a guard that names no base (it
            # goes by the lifting guard's flag), then a Hold fleet that hosts
            Fleet(2, 0, d, order=4, target=d, mission=3, roe=10, ships=destroyer),
            Fleet(2, 0, d, order=4, target=d, mission=9, roe=10, ships=destroyer),
            Fleet(2, 0, d, order=0, ships=destroyer),
        ]
    )
    scenario.bases += [Base(2, 1, a, escort=1), Base(2, 2, b, escort=3, tz=0), Base(2, 3, d, escort=7, tz=0)]
    return [("mergeguard", scenario)]


def group_mergenobase(_: int) -> list[tuple[str, Scenario]]:
    """A Guard Starbase fleet that names no base, first in the pass, for each kind of empire."""
    destroyer = (0, 0, 1, 0, 0, 0, 0)
    out = []
    for label, empire, mode, autopilot in (("rogue", 2, 0xFF, False), ("autopilot", 1, 1, True)):
        modes = {1: 1, 2: 0, 3: 0, 4: 0}
        modes[empire] = mode
        scenario = home_scenario(modes)
        if autopilot:
            scenario.autopilot.add(empire)
        scenario.fleets += numbered(
            [
                Fleet(empire, 0, HOME[empire], order=4, target=HOME[empire], mission=9, roe=6, ships=destroyer),
                Fleet(empire, 0, HOME[empire], order=0, ships=destroyer),
            ]
        )
        for other in range(1, 5):
            if other != empire:
                scenario.fleets.append(Fleet(other, 1, HOME[other], order=0, ships=(0, 1, 0, 0, 0, 0, 0)))
        out.append((f"mergenobase-{label}", scenario))
    return out


def group_mergecap(_: int) -> list[tuple[str, Scenario]]:
    scenario = Scenario(modes={1: 1, 2: 0xFF, 3: 0, 4: 0})
    scenario.owners[HOME[2]] = 2
    scenario.owners[WORLD] = 2
    quiet_neighbours(scenario, (1, 3, 4))
    scenario.fleets += numbered(
        [
            Fleet(2, 0, HOME[2], order=0, ships=(0, 0, 1500, 0, 0, 0, 0)),
            Fleet(2, 0, HOME[2], order=0, ships=(0, 0, 1500, 0, 0, 0, 0)),
            Fleet(2, 0, WORLD, order=0, ships=(0, 0, 1500, 0, 0, 0, 0)),
            Fleet(2, 0, WORLD, order=0, ships=(0, 0, 1501, 0, 0, 0, 0)),
        ]
    )
    return [("mergecap", scenario)]


def group_holdlost(_: int) -> list[tuple[str, Scenario]]:
    """A human empire's fleet whose order needs ships it does not have."""
    etac, cruiser = (0, 0, 0, 0, 0, 1, 0), (0, 1, 0, 0, 0, 0, 0)
    out = []
    for label, worlds, fleet in (
        ("guard-onworld", 1, Fleet(1, 1, HOME[1], order=5, target=HOME[1], ships=etac)),
        ("guard-twoworlds", 2, Fleet(1, 1, HOME[1], order=5, target=HOME[1], ships=etac)),
        ("guard-underway", 2, Fleet(1, 1, (14, 13), z=0.0, order=5, target=WORLD, tz=1, speed=3, ships=etac)),
        ("patrol-underway", 2, Fleet(1, 1, (14, 13), z=0.0, order=3, target=(12, 13), tz=0, speed=3, ships=etac)),
        ("scout-twoworlds", 2, Fleet(1, 1, HOME[1], order=10, target=(16, 11), tz=0, ships=etac)),
        ("colonize-twoworlds", 2, Fleet(1, 1, HOME[1], order=12, target=WORLD2, tz=1, ships=cruiser)),
        ("join-nohost", 1, Fleet(1, 1, HOME[1], order=13, mission=9, target=HOME[1], tz=1, ships=cruiser)),
        ("bombard-own", 1, Fleet(1, 1, HOME[1], order=6, target=HOME[1], tz=1, ships=cruiser)),
    ):
        scenario = Scenario(modes={1: 1, 2: 0, 3: 0, 4: 0})
        scenario.owners[HOME[1]] = 1
        if worlds == 2:
            scenario.owners[(18, 15)] = 1
        for empire in (2, 3, 4):
            scenario.owners[HOME[empire]] = empire
            scenario.fleets.append(Fleet(empire, 1, HOME[empire], order=0, ships=cruiser))
        scenario.fleets.append(fleet)
        out.append((f"holdlost-{label}", scenario))
    return out


def group_holdnorm(_: int) -> list[tuple[str, Scenario]]:
    """Fleets already on Hold with a stale target, and a fleet that finishes a Move."""
    cruiser = (0, 1, 0, 0, 0, 0, 0)
    scenario = Scenario(modes={1: 1, 2: 0, 3: 0, 4: 0})
    scenario.owners[HOME[1]] = 1
    for empire in (2, 3, 4):
        scenario.owners[HOME[empire]] = empire
        scenario.fleets.append(Fleet(empire, 1, HOME[empire], order=0, ships=cruiser))
    scenario.fleets += numbered(
        [
            Fleet(1, 0, HOME[1], order=0, target=(8, 8), tz=0, speed=0, ships=cruiser),
            Fleet(1, 0, (14, 13), z=0.0, order=0, target=(8, 8), tz=1, speed=0, ships=cruiser),
            Fleet(1, 0, (14, 12), z=0.0, order=0, target=(8, 8), tz=1, speed=3, mission=5, ships=cruiser),
        ]
    )
    return [("holdnorm", scenario)]


def group_civil(_: int) -> list[tuple[str, Scenario]]:
    """Planetless civil-disorder empires, with and without fleets and a base."""
    cruiser, etac, scout = (0, 1, 0, 0, 0, 0, 0), (0, 0, 0, 0, 0, 1, 0), (0, 0, 0, 0, 0, 0, 1)
    out = []
    for label, fleets in (("fleets", True), ("nofleets", False)):
        scenario = Scenario(modes={1: 1, 2: 0, 3: 0, 4: 0})
        scenario.owners[HOME[1]] = 1
        scenario.owners[HOME[2]] = 2  # civil disorder with a world: stays as it is
        scenario.fleets.append(Fleet(1, 1, HOME[1], order=0, ships=cruiser))
        scenario.fleets.append(Fleet(2, 1, HOME[2], order=0, ships=cruiser))
        # empires 3 and 4 own nothing; 4 was a player and keeps a handle and a name
        scenario.names[4] = "Fallen Empire"
        scenario.handles[4] = "OLDPLAYER"
        scenario.enemies.update({(3, 4), (4, 2)})
        if fleets:
            for empire in (3, 4):
                scenario.fleets.append(Fleet(empire, 1, (8, 8 + empire), z=0.0, order=0, roe=3, ships=cruiser))
                scenario.fleets.append(Fleet(empire, 2, (9, 8 + empire), z=0.0, order=0, ships=etac))
                scenario.fleets.append(Fleet(empire, 3, (10, 8 + empire), z=0.0, order=0, ships=scout))
        for empire in (3, 4):
            scenario.owners[HOME[empire]] = 0
            scenario.bases.append(Base(empire, 1, HOME[empire]))
        out.append((f"civil-{label}", scenario))
    return out


GROUPS = {
    "scatter": (group_scatter, 1),
    "destroy": (group_destroy, 1),
    "merge": (group_merge, 1),
    "abort": (group_abort, 1),
    "etac": (group_etac, 3),
    "worldless": (group_worldless, 3),
    "mergeplace": (group_mergeplace, 1),
    "mergeauto": (group_mergeauto, 1),
    "mergeguard": (group_mergeguard, 1),
    "mergenobase": (group_mergenobase, 1),
    "mergecap": (group_mergecap, 1),
    "holdlost": (group_holdlost, 1),
    "holdnorm": (group_holdnorm, 1),
    "civil": (group_civil, 3),
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
