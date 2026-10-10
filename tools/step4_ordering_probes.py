#!/usr/bin/env python3
"""Black-box oracle probes for step-4 internal ordering constraints.

Each probe constructs a specific game state designed to expose whether
subphase A runs before or after subphase B, then inspects post-ECMAINT
state and report text for ordering evidence.

Probes:
  econ-vs-movement    Fleet arriving at owned world same tick as growth.
                      Does the arrival report reference old or new economy?
  production-vs-combat Build queue completing same tick as hostile arrival.
                      Do newly produced units participate in defense?
  command-normalization Fleet ordered to attack a target destroyed by
                      another fleet same tick. Normalized before or after?
  econ-weekly-timing   Run econ fixture, diff PLANETS.DAT, check which
                      stardates produce RESULTS.DAT economy entries.

Usage:
    python3 tools/step4_ordering_probes.py <probe-name>
    python3 tools/step4_ordering_probes.py --list
    python3 tools/step4_ordering_probes.py all
"""

from __future__ import annotations

import argparse
import struct
import shutil
import sys
from dataclasses import dataclass
from pathlib import Path

from ncmaint_oracle import (
    KNOWN_SCENARIOS,
    TRACKED_FILES,
    collect_diffs,
    copy_tree,
    ensure_engine,
    run_ecmaint,
    snapshot_dir,
    summarize_clusters,
)

ROOT = Path(__file__).resolve().parents[1]

FLEET_RECORD_SIZE = 54
PLANET_RECORD_SIZE = 0x61  # 97
RESULTS_RECORD_SIZE = 84
PLAYER_RECORD_SIZE = 110


# --- Binary helpers ---

def read_record(path: Path, index: int, record_size: int) -> bytes:
    data = path.read_bytes()
    start = index * record_size
    end = start + record_size
    if end > len(data):
        raise ValueError(f"record {index} out of range in {path.name} (size={len(data)})")
    return data[start:end]


def write_record(path: Path, index: int, record_size: int, record: bytes) -> None:
    data = bytearray(path.read_bytes())
    start = index * record_size
    data[start:start + record_size] = record
    path.write_bytes(bytes(data))


def patch_bytes(data: bytearray, offset: int, values: bytes) -> None:
    data[offset:offset + len(values)] = values


def u16le(data: bytes, offset: int) -> int:
    return struct.unpack_from("<H", data, offset)[0]


def set_u16le(data: bytearray, offset: int, value: int) -> None:
    struct.pack_into("<H", data, offset, value & 0xFFFF)


def parse_results_records(path: Path) -> list[dict]:
    """Parse RESULTS.DAT into a list of record dicts."""
    if not path.exists():
        return []
    data = path.read_bytes()
    records = []
    for i in range(0, len(data), RESULTS_RECORD_SIZE):
        chunk = data[i:i + RESULTS_RECORD_SIZE]
        if len(chunk) < RESULTS_RECORD_SIZE:
            break
        kind = chunk[0]
        if kind == 0:
            continue
        # Extract text (offset 1-75, null-terminated)
        text_raw = chunk[1:76]
        null_pos = text_raw.find(b'\x00')
        if null_pos >= 0:
            text_raw = text_raw[:null_pos]
        text = text_raw.decode("ascii", errors="replace")
        tail = chunk[76:84]
        records.append({
            "index": i // RESULTS_RECORD_SIZE,
            "kind": kind,
            "text": text,
            "tail": tail.hex(),
        })
    return records


def diff_record_fields(before: bytes, after: bytes, field_map: dict[str, tuple[int, int]]) -> dict[str, tuple[int, int]]:
    """Compare two records and return changed fields with before/after values."""
    changes = {}
    for name, (offset, size) in field_map.items():
        bval = int.from_bytes(before[offset:offset + size], "little")
        aval = int.from_bytes(after[offset:offset + size], "little")
        if bval != aval:
            changes[name] = (bval, aval)
    return changes


PLANET_FIELDS = {
    "coords_x": (0x00, 1),
    "coords_y": (0x01, 1),
    "potential_prod_lo": (0x02, 1),
    "potential_prod_hi": (0x03, 1),
    "stored_goods": (0x0A, 4),
    "economy_marker": (0x0E, 1),
    "army_count": (0x58, 1),
    "ground_batteries": (0x5A, 1),
    "ownership_status": (0x5C, 1),
    "owner_empire": (0x5D, 1),
}

FLEET_FIELDS = {
    "owner_empire": (0x02, 1),
    "invasion_army": (0x08, 1),
    "max_speed": (0x09, 1),
    "current_speed": (0x0A, 1),
    "location_x": (0x0B, 1),
    "location_y": (0x0C, 1),
    "standing_order": (0x1F, 1),
    "target_x": (0x20, 1),
    "target_y": (0x21, 1),
    "scout_count": (0x24, 1),
    "battleship_count": (0x26, 2),
    "cruiser_count": (0x28, 2),
    "destroyer_count": (0x2A, 2),
    "transport_count": (0x2C, 2),
    "army_count": (0x2E, 2),
    "etac_count": (0x30, 2),
}


# --- Probe infrastructure ---

@dataclass
class ProbeResult:
    name: str
    conclusion: str  # "A_BEFORE_B", "B_BEFORE_A", "INCONCLUSIVE", "ERROR"
    evidence: list[str]
    detail: str


def prepare_probe_dir(source: Path, probe_name: str) -> Path:
    target = Path(f"/tmp/ecmaint-ordering-probe-{probe_name}")
    if target.exists():
        shutil.rmtree(target)
    copy_tree(source, target)
    ensure_engine(target)
    return target


def run_probe_ecmaint(target: Path, ticks: int = 1) -> list[dict]:
    """Run ECMAINT for N ticks, return RESULTS.DAT records from the final state."""
    for tick in range(ticks):
        snapshot_dir(target, f"tick-{tick:02d}-before")
        result = run_ecmaint(target)
        snapshot_dir(target, f"tick-{tick:02d}-after")
        if result.returncode != 0:
            print(f"  WARNING: ECMAINT exit code {result.returncode} on tick {tick}")
    return parse_results_records(target / "RESULTS.DAT")


# --- Probe: Economy vs Movement ---

def probe_econ_vs_movement() -> ProbeResult:
    """Does a fleet arriving at an owned world see pre-growth or post-growth economy?

    Strategy: Use the econ fixture (which has active economy), check if fleet
    movement outcomes reference economy state before or after growth is applied.
    Compare PLANETS.DAT stored_goods changes with fleet arrival timing in RESULTS.
    """
    evidence: list[str] = []
    source = KNOWN_SCENARIOS["econ"]["pre"]
    ticks = KNOWN_SCENARIOS["econ"]["ticks"]
    target = prepare_probe_dir(source, "econ-vs-movement")

    # Snapshot planet state before
    planets_before = {}
    for i in range(20):
        try:
            planets_before[i] = read_record(target / "PLANETS.DAT", i, PLANET_RECORD_SIZE)
        except ValueError:
            break

    # Snapshot fleet state before
    fleets_before = {}
    for i in range(16):
        try:
            fleets_before[i] = read_record(target / "FLEETS.DAT", i, FLEET_RECORD_SIZE)
        except ValueError:
            break

    evidence.append(f"Source fixture: econ (ticks={ticks})")
    evidence.append(f"Planets loaded: {len(planets_before)}")
    evidence.append(f"Fleets loaded: {len(fleets_before)}")

    # Log fleet positions and orders
    for idx, rec in fleets_before.items():
        order = rec[0x1F]
        loc_x, loc_y = rec[0x0B], rec[0x0C]
        tgt_x, tgt_y = rec[0x20], rec[0x21]
        speed = rec[0x0A]
        owner = rec[0x02]
        if order != 0 or speed != 0:
            evidence.append(
                f"  Fleet {idx}: owner={owner} order=0x{order:02x} "
                f"loc=({loc_x},{loc_y}) target=({tgt_x},{tgt_y}) speed={speed}"
            )

    # Run ECMAINT
    results = run_probe_ecmaint(target, ticks)

    # Compare planet economy fields
    evidence.append("")
    evidence.append("Planet economy changes:")
    for i in range(min(20, len(planets_before))):
        try:
            after = read_record(target / "PLANETS.DAT", i, PLANET_RECORD_SIZE)
        except ValueError:
            break
        changes = diff_record_fields(planets_before[i], after, PLANET_FIELDS)
        if changes:
            evidence.append(f"  Planet {i}: {changes}")

    # Compare fleet state
    evidence.append("")
    evidence.append("Fleet changes:")
    for i in range(min(16, len(fleets_before))):
        try:
            after = read_record(target / "FLEETS.DAT", i, FLEET_RECORD_SIZE)
        except ValueError:
            break
        changes = diff_record_fields(fleets_before[i], after, FLEET_FIELDS)
        if changes:
            evidence.append(f"  Fleet {i}: {changes}")

    # Parse RESULTS.DAT
    evidence.append("")
    evidence.append(f"RESULTS.DAT records: {len(results)}")
    for rec in results:
        evidence.append(f"  [{rec['index']:3d}] kind=0x{rec['kind']:02x} tail={rec['tail']}")
        evidence.append(f"       text={rec['text'][:70]}")

    # Look for ordering evidence in report text
    fleet_reports = [r for r in results if r["kind"] in (0x05, 0x06, 0x07)]
    econ_related = [r for r in results if "economy" in r["text"].lower() or "production" in r["text"].lower()]

    conclusion = "INCONCLUSIVE"
    detail = "Need to compare fleet arrival reports against economy state changes."
    if fleet_reports:
        detail += f" Found {len(fleet_reports)} fleet-type reports."
    if econ_related:
        detail += f" Found {len(econ_related)} economy-related reports."

    return ProbeResult(
        name="econ-vs-movement",
        conclusion=conclusion,
        evidence=evidence,
        detail=detail,
    )


# --- Probe: Production Completion vs Combat ---

def probe_production_vs_combat() -> ProbeResult:
    """Do units completing production participate in same-tick combat?

    Strategy: Use planet-build fixture (which has active build queues) and
    overlay with a hostile fleet arriving at the build planet. Compare
    combat results with and without the build queue.
    """
    evidence: list[str] = []

    # First run: planet-build baseline (no combat)
    source_build = KNOWN_SCENARIOS["planet-build"]["pre"]
    target_build = prepare_probe_dir(source_build, "production-baseline")

    planets_before_build = {}
    for i in range(20):
        try:
            planets_before_build[i] = read_record(target_build / "PLANETS.DAT", i, PLANET_RECORD_SIZE)
        except ValueError:
            break

    evidence.append("=== Baseline: planet-build (no combat) ===")
    results_build = run_probe_ecmaint(target_build, KNOWN_SCENARIOS["planet-build"]["ticks"])

    for i in range(min(20, len(planets_before_build))):
        try:
            after = read_record(target_build / "PLANETS.DAT", i, PLANET_RECORD_SIZE)
        except ValueError:
            break
        changes = diff_record_fields(planets_before_build[i], after, PLANET_FIELDS)
        if changes:
            evidence.append(f"  Planet {i} changes: {changes}")

    evidence.append(f"  RESULTS records: {len(results_build)}")
    for rec in results_build:
        evidence.append(f"    [{rec['index']:3d}] kind=0x{rec['kind']:02x} {rec['text'][:60]}")

    # Second run: bombard fixture (has combat + planet interaction)
    source_bombard = KNOWN_SCENARIOS["bombard"]["pre"]
    target_bombard = prepare_probe_dir(source_bombard, "production-vs-combat")

    planets_before_bombard = {}
    for i in range(20):
        try:
            planets_before_bombard[i] = read_record(target_bombard / "PLANETS.DAT", i, PLANET_RECORD_SIZE)
        except ValueError:
            break

    fleets_before_bombard = {}
    for i in range(16):
        try:
            fleets_before_bombard[i] = read_record(target_bombard / "FLEETS.DAT", i, FLEET_RECORD_SIZE)
        except ValueError:
            break

    evidence.append("")
    evidence.append("=== Comparison: bombard (combat at planet) ===")

    # Log hostile fleet targeting
    for idx, rec in fleets_before_bombard.items():
        order = rec[0x1F]
        tgt_x, tgt_y = rec[0x20], rec[0x21]
        owner = rec[0x02]
        if order in (0x06, 0x07, 0x08):  # bombard/invade orders
            evidence.append(f"  Hostile fleet {idx}: owner={owner} order=0x{order:02x} target=({tgt_x},{tgt_y})")

    results_bombard = run_probe_ecmaint(target_bombard, KNOWN_SCENARIOS["bombard"]["ticks"])

    for i in range(min(20, len(planets_before_bombard))):
        try:
            after = read_record(target_bombard / "PLANETS.DAT", i, PLANET_RECORD_SIZE)
        except ValueError:
            break
        changes = diff_record_fields(planets_before_bombard[i], after, PLANET_FIELDS)
        if changes:
            evidence.append(f"  Planet {i} changes: {changes}")

    evidence.append(f"  RESULTS records: {len(results_bombard)}")
    for rec in results_bombard:
        evidence.append(f"    [{rec['index']:3d}] kind=0x{rec['kind']:02x} {rec['text'][:60]}")

    conclusion = "INCONCLUSIVE"
    detail = (
        "Compare planet defense state changes between build-only and build+combat scenarios. "
        "If newly built units appear in combat reports, production completes before combat."
    )

    return ProbeResult(
        name="production-vs-combat",
        conclusion=conclusion,
        evidence=evidence,
        detail=detail,
    )


# --- Probe: Command Normalization Timing ---

def probe_command_normalization() -> ProbeResult:
    """Is command normalization done before or after combat resolution?

    Strategy: Use fleet-battle fixture where multiple hostile fleets converge.
    Check if fleets targeting destroyed targets get redirected or report
    target-not-found errors.
    """
    evidence: list[str] = []
    source = KNOWN_SCENARIOS["fleet-battle"]["pre"]
    ticks = KNOWN_SCENARIOS["fleet-battle"]["ticks"]
    target = prepare_probe_dir(source, "command-normalization")

    fleets_before = {}
    for i in range(16):
        try:
            fleets_before[i] = read_record(target / "FLEETS.DAT", i, FLEET_RECORD_SIZE)
        except ValueError:
            break

    evidence.append(f"Source fixture: fleet-battle (ticks={ticks})")
    evidence.append("Fleet state before ECMAINT:")
    for idx, rec in fleets_before.items():
        order = rec[0x1F]
        loc_x, loc_y = rec[0x0B], rec[0x0C]
        tgt_x, tgt_y = rec[0x20], rec[0x21]
        owner = rec[0x02]
        ships = {
            "BS": u16le(rec, 0x26), "CA": u16le(rec, 0x28),
            "DD": u16le(rec, 0x2A), "TT": u16le(rec, 0x2C),
            "army": u16le(rec, 0x2E),
        }
        total = sum(ships.values())
        if total > 0:
            evidence.append(
                f"  Fleet {idx}: owner={owner} order=0x{order:02x} "
                f"loc=({loc_x},{loc_y}) target=({tgt_x},{tgt_y}) ships={ships}"
            )

    results = run_probe_ecmaint(target, ticks)

    evidence.append("")
    evidence.append("Fleet state after ECMAINT:")
    for i in range(min(16, len(fleets_before))):
        try:
            after = read_record(target / "FLEETS.DAT", i, FLEET_RECORD_SIZE)
        except ValueError:
            break
        changes = diff_record_fields(fleets_before[i], after, FLEET_FIELDS)
        if changes:
            evidence.append(f"  Fleet {i}: {changes}")

    evidence.append("")
    evidence.append(f"RESULTS.DAT records: {len(results)}")
    for rec in results:
        evidence.append(f"  [{rec['index']:3d}] kind=0x{rec['kind']:02x} {rec['text'][:70]}")

    # Look for normalization evidence: redirected orders, target-not-found
    redirect_evidence = [r for r in results if "redirect" in r["text"].lower() or "no target" in r["text"].lower()]
    battle_reports = [r for r in results if r["kind"] == 0x06]

    conclusion = "INCONCLUSIVE"
    detail = (
        f"Found {len(battle_reports)} battle reports. "
        "Check if fleets with orders against destroyed targets were redirected or "
        "if they attempted to execute the original orders."
    )
    if redirect_evidence:
        detail += f" Found {len(redirect_evidence)} redirect-related reports."

    return ProbeResult(
        name="command-normalization",
        conclusion=conclusion,
        evidence=evidence,
        detail=detail,
    )


# --- Probe: Economy Weekly Timing ---

def probe_econ_weekly_timing() -> ProbeResult:
    """At which stardate week does economic growth produce visible effects?

    Strategy: Run econ fixture tick by tick, tracking both planet AND fleet
    state changes per tick. Also dump raw hex diffs for changed records.
    """
    import re

    evidence: list[str] = []
    source = KNOWN_SCENARIOS["econ"]["pre"]
    ticks = KNOWN_SCENARIOS["econ"]["ticks"]
    target = prepare_probe_dir(source, "econ-weekly-timing")

    planets_initial = {}
    for i in range(25):
        try:
            planets_initial[i] = read_record(target / "PLANETS.DAT", i, PLANET_RECORD_SIZE)
        except ValueError:
            break

    fleets_initial = {}
    for i in range(16):
        try:
            fleets_initial[i] = read_record(target / "FLEETS.DAT", i, FLEET_RECORD_SIZE)
        except ValueError:
            break

    evidence.append(f"Source fixture: econ (ticks={ticks})")

    # Log initial economy state for ALL owned planets
    evidence.append("Initial planet state (owned):")
    for i, rec in planets_initial.items():
        owner = rec[0x5D]
        if owner > 0:
            stored = struct.unpack_from("<I", rec, 0x0A)[0]
            status = rec[0x5C]
            coords = (rec[0x00], rec[0x01])
            pot_lo, pot_hi = rec[0x02], rec[0x03]
            armies = rec[0x58]
            batteries = rec[0x5A]
            econ_marker = rec[0x0E]
            evidence.append(
                f"  Planet {i}: coords={coords} owner={owner} status=0x{status:02x} "
                f"pot=({pot_lo},{pot_hi}) stored={stored} armies={armies} "
                f"batteries={batteries} econ_marker={econ_marker}"
            )

    # Log initial fleet state for ALL active fleets
    evidence.append("Initial fleet state:")
    for idx, rec in fleets_initial.items():
        owner = rec[0x02]
        order = rec[0x1F]
        loc = (rec[0x0B], rec[0x0C])
        tgt = (rec[0x20], rec[0x21])
        speed = rec[0x0A]
        max_spd = rec[0x09]
        scouts = rec[0x24]
        bs = u16le(rec, 0x26)
        ca = u16le(rec, 0x28)
        dd = u16le(rec, 0x2A)
        tt = u16le(rec, 0x2C)
        army = u16le(rec, 0x2E)
        etac = u16le(rec, 0x30)
        total = scouts + bs + ca + dd + tt + army + etac
        if total > 0 or order != 0:
            evidence.append(
                f"  Fleet {idx}: owner={owner} order=0x{order:02x} "
                f"loc={loc} target={tgt} speed={speed} max_spd={max_spd} "
                f"ships=[sc={scouts} bs={bs} ca={ca} dd={dd} tt={tt} army={army} etac={etac}]"
            )

    # Run ECMAINT tick by tick
    for tick in range(1, ticks + 1):
        before_snap = snapshot_dir(target, f"tick-{tick:02d}-before")
        result = run_ecmaint(target)
        snapshot_dir(target, f"tick-{tick:02d}-after")

        evidence.append(f"")
        evidence.append(f"=== Tick {tick} ===")

        if result.returncode != 0:
            evidence.append(f"  WARNING: exit code {result.returncode}")

        # Diff fleets
        evidence.append("  Fleet changes:")
        for i in range(16):
            try:
                before_rec = read_record(before_snap / "FLEETS.DAT", i, FLEET_RECORD_SIZE)
                after_rec = read_record(target / "FLEETS.DAT", i, FLEET_RECORD_SIZE)
            except (ValueError, FileNotFoundError):
                continue
            changes = diff_record_fields(before_rec, after_rec, FLEET_FIELDS)
            if changes:
                evidence.append(f"    Fleet {i}: {changes}")
            # Also show raw byte diffs for non-mapped offsets
            raw_diffs = []
            for off in range(FLEET_RECORD_SIZE):
                if before_rec[off] != after_rec[off]:
                    raw_diffs.append(f"+0x{off:02x}:{before_rec[off]:02x}->{after_rec[off]:02x}")
            if raw_diffs:
                evidence.append(f"    Fleet {i} raw: {' '.join(raw_diffs)}")

        # Diff planets
        evidence.append("  Planet changes:")
        for i in range(min(25, len(planets_initial))):
            try:
                before_rec = read_record(before_snap / "PLANETS.DAT", i, PLANET_RECORD_SIZE)
                after_rec = read_record(target / "PLANETS.DAT", i, PLANET_RECORD_SIZE)
            except (ValueError, FileNotFoundError):
                continue
            changes = diff_record_fields(before_rec, after_rec, PLANET_FIELDS)
            if changes:
                evidence.append(f"    Planet {i}: {changes}")
            raw_diffs = []
            for off in range(PLANET_RECORD_SIZE):
                if before_rec[off] != after_rec[off]:
                    raw_diffs.append(f"+0x{off:02x}:{before_rec[off]:02x}->{after_rec[off]:02x}")
            if raw_diffs:
                evidence.append(f"    Planet {i} raw: {' '.join(raw_diffs)}")

        # Check other tracked files for size/content changes
        for fname in ["RESULTS.DAT", "MESSAGES.DAT", "DATABASE.DAT", "ERRORS.TXT", "RANKINGS.TXT"]:
            before_path = before_snap / fname
            after_path = target / fname
            before_size = before_path.stat().st_size if before_path.exists() else 0
            after_size = after_path.stat().st_size if after_path.exists() else 0
            if before_size != after_size:
                evidence.append(f"  {fname}: size {before_size} -> {after_size}")

        # Parse RESULTS for this tick
        tick_results = parse_results_records(target / "RESULTS.DAT")
        if tick_results:
            evidence.append(f"  RESULTS records: {len(tick_results)}")
            for rec in tick_results:
                evidence.append(f"    [{rec['index']:3d}] kind=0x{rec['kind']:02x} {rec['text'][:70]}")
                sdm = re.search(r"Stardate[:\s]+(\d+/\d+)", rec["text"])
                if sdm:
                    evidence.append(f"      -> Stardate: {sdm.group(1)}")

    conclusion = "INCONCLUSIVE"
    detail = "Compare fleet arrival timing against planet economy changes per tick."

    return ProbeResult(
        name="econ-weekly-timing",
        conclusion=conclusion,
        evidence=evidence,
        detail=detail,
    )


# --- Probe: Invade Ordering ---

def probe_invade_ordering() -> ProbeResult:
    """What is the ordering of invasion effects relative to other subphases?

    Strategy: Run invade, bombard, and fleet-battle per-tick with full fleet+planet
    tracking. Compare which tick fleet arrival, combat, planet mutation, and
    reports appear in each scenario.
    """
    import re

    evidence: list[str] = []

    for scenario_name in ("invade", "bombard", "fleet-battle"):
        source = KNOWN_SCENARIOS[scenario_name]["pre"]
        ticks = KNOWN_SCENARIOS[scenario_name]["ticks"]
        target = prepare_probe_dir(source, f"invade-ordering-{scenario_name}")

        planets_initial = {}
        for i in range(25):
            try:
                planets_initial[i] = read_record(target / "PLANETS.DAT", i, PLANET_RECORD_SIZE)
            except ValueError:
                break

        fleets_initial = {}
        for i in range(16):
            try:
                fleets_initial[i] = read_record(target / "FLEETS.DAT", i, FLEET_RECORD_SIZE)
            except ValueError:
                break

        evidence.append(f"=== {scenario_name} (ticks={ticks}) ===")

        # Log all active fleets with full state
        evidence.append("Initial fleets:")
        for idx, rec in fleets_initial.items():
            owner = rec[0x02]
            order = rec[0x1F]
            loc = (rec[0x0B], rec[0x0C])
            tgt = (rec[0x20], rec[0x21])
            speed = rec[0x0A]
            inv_army = rec[0x08]
            ships = {
                "sc": rec[0x24], "bs": u16le(rec, 0x26), "ca": u16le(rec, 0x28),
                "dd": u16le(rec, 0x2A), "tt": u16le(rec, 0x2C),
                "army": u16le(rec, 0x2E), "etac": u16le(rec, 0x30),
            }
            total = sum(ships.values())
            if total > 0 or order != 0:
                evidence.append(
                    f"  Fleet {idx}: owner={owner} order=0x{order:02x} loc={loc} "
                    f"target={tgt} speed={speed} inv_army={inv_army} ships={ships}"
                )

        # Log owned planets
        evidence.append("Initial planets (owned):")
        for i, rec in planets_initial.items():
            owner = rec[0x5D]
            if owner > 0:
                coords = (rec[0x00], rec[0x01])
                armies = rec[0x58]
                batteries = rec[0x5A]
                status = rec[0x5C]
                econ = rec[0x0E]
                evidence.append(
                    f"  Planet {i}: coords={coords} owner={owner} status=0x{status:02x} "
                    f"armies={armies} batteries={batteries} econ={econ}"
                )

        # Per-tick tracking
        for tick in range(1, ticks + 1):
            before_snap = snapshot_dir(target, f"tick-{tick:02d}-before")
            result = run_ecmaint(target)
            snapshot_dir(target, f"tick-{tick:02d}-after")

            evidence.append(f"  --- Tick {tick} ---")
            if result.returncode != 0:
                evidence.append(f"  WARNING: exit code {result.returncode}")

            # Fleet diffs
            for i in range(16):
                try:
                    before_rec = read_record(before_snap / "FLEETS.DAT", i, FLEET_RECORD_SIZE)
                    after_rec = read_record(target / "FLEETS.DAT", i, FLEET_RECORD_SIZE)
                except (ValueError, FileNotFoundError):
                    continue
                changes = diff_record_fields(before_rec, after_rec, FLEET_FIELDS)
                if changes:
                    evidence.append(f"    Fleet {i}: {changes}")

            # Planet diffs
            for i in range(min(25, len(planets_initial))):
                try:
                    before_rec = read_record(before_snap / "PLANETS.DAT", i, PLANET_RECORD_SIZE)
                    after_rec = read_record(target / "PLANETS.DAT", i, PLANET_RECORD_SIZE)
                except (ValueError, FileNotFoundError):
                    continue
                changes = diff_record_fields(before_rec, after_rec, PLANET_FIELDS)
                if changes:
                    evidence.append(f"    Planet {i}: {changes}")
                # Raw diffs for non-mapped offsets
                raw_diffs = []
                for off in range(PLANET_RECORD_SIZE):
                    if before_rec[off] != after_rec[off]:
                        raw_diffs.append(f"+0x{off:02x}:{before_rec[off]:02x}->{after_rec[off]:02x}")
                if raw_diffs:
                    evidence.append(f"    Planet {i} raw: {' '.join(raw_diffs)}")

            # Output file changes
            for fname in ["RESULTS.DAT", "MESSAGES.DAT", "DATABASE.DAT", "ERRORS.TXT"]:
                before_path = before_snap / fname
                after_path = target / fname
                before_size = before_path.stat().st_size if before_path.exists() else 0
                after_size = after_path.stat().st_size if after_path.exists() else 0
                if before_size != after_size:
                    evidence.append(f"    {fname}: size {before_size} -> {after_size}")

            # RESULTS records
            tick_results = parse_results_records(target / "RESULTS.DAT")
            if tick_results:
                evidence.append(f"    RESULTS records: {len(tick_results)}")
                for rec in tick_results:
                    evidence.append(f"      [{rec['index']:3d}] kind=0x{rec['kind']:02x} {rec['text'][:70]}")
                    sdm = re.search(r"Stardate[:\s]+(\d+/\d+)", rec["text"])
                    if sdm:
                        evidence.append(f"        -> Stardate: {sdm.group(1)}")

        evidence.append("")

    conclusion = "INCONCLUSIVE"
    detail = (
        "Cross-scenario per-tick comparison of fleet arrival, combat, planet "
        "mutation, and report timing."
    )

    return ProbeResult(
        name="invade-ordering",
        conclusion=conclusion,
        evidence=evidence,
        detail=detail,
    )


# --- Probe: Colonization Timing ---

def probe_colonization_timing() -> ProbeResult:
    """When does ETAC colonization resolve relative to movement and economy?

    Strategy: Take the econ fixture base, repurpose fleet 3 (empire 1) as an
    ETAC colonize fleet targeting nearby unowned planet 16 at (18,15), 2 sectors
    from homeworld (16,13). Also set player 1 active (byte 0 = 0xFF) so economy
    fires. Track per-tick fleet, planet, and report state for 3 ticks.
    """
    import re

    evidence: list[str] = []
    source = KNOWN_SCENARIOS["econ"]["pre"]
    target = prepare_probe_dir(source, "colonization-timing")

    evidence.append("Base fixture: econ (all civil disorder, no rogue captures)")

    # Repurpose fleet 2 (empire 1, 50CA+50DD) for colonize
    # Planet 16 at (18,15) is 2 sectors from (16,13), reachable at speed 3
    fleet_dat = bytearray((target / "FLEETS.DAT").read_bytes())
    fleet2_off = 2 * FLEET_RECORD_SIZE
    fleet_dat[fleet2_off + 0x1F] = 0x0C        # order = ColonizeWorld (12)
    fleet_dat[fleet2_off + 0x20] = 18          # target_x = 18
    fleet_dat[fleet2_off + 0x21] = 15          # target_y = 15
    # Already has speed=3, 50CA, 50DD; add 1 ETAC for colonization
    set_u16le(fleet_dat, fleet2_off + 0x30, 1)  # etac_count = 1
    (target / "FLEETS.DAT").write_bytes(bytes(fleet_dat))
    evidence.append("Patched: fleet 2 -> colonize (18,15) speed=3 with 50CA+50DD+1ETAC")

    # Load initial state
    planets_initial = {}
    for i in range(20):
        try:
            planets_initial[i] = read_record(target / "PLANETS.DAT", i, PLANET_RECORD_SIZE)
        except ValueError:
            break

    fleets_initial = {}
    for i in range(16):
        try:
            fleets_initial[i] = read_record(target / "FLEETS.DAT", i, FLEET_RECORD_SIZE)
        except ValueError:
            break

    # Log key initial state
    evidence.append(f"Planet 16 (target): coords=({planets_initial[16][0]},{planets_initial[16][1]}) "
                    f"owner={planets_initial[16][0x5D]} status=0x{planets_initial[16][0x5C]:02x}")
    evidence.append(f"Planet 14 (homeworld): coords=({planets_initial[14][0]},{planets_initial[14][1]}) "
                    f"owner={planets_initial[14][0x5D]} armies={planets_initial[14][0x58]} econ={planets_initial[14][0x0E]}")

    evidence.append("Fleet 2 after patch:")
    rec = fleets_initial[2]
    evidence.append(f"  owner={rec[0x02]} order=0x{rec[0x1F]:02x} loc=({rec[0x0B]},{rec[0x0C]}) "
                    f"target=({rec[0x20]},{rec[0x21]}) speed={rec[0x0A]} "
                    f"ca={u16le(rec, 0x28)} dd={u16le(rec, 0x2A)} etac={u16le(rec, 0x30)}")

    ticks = 3
    for tick in range(1, ticks + 1):
        before_snap = snapshot_dir(target, f"tick-{tick:02d}-before")
        result = run_ecmaint(target)
        snapshot_dir(target, f"tick-{tick:02d}-after")

        evidence.append(f"")
        evidence.append(f"=== Tick {tick} ===")
        if result.returncode != 0:
            evidence.append(f"  WARNING: exit code {result.returncode}")

        # Fleet 2 diff (colonizer)
        try:
            before_rec = read_record(before_snap / "FLEETS.DAT", 2, FLEET_RECORD_SIZE)
            after_rec = read_record(target / "FLEETS.DAT", 2, FLEET_RECORD_SIZE)
            changes = diff_record_fields(before_rec, after_rec, FLEET_FIELDS)
            if changes:
                evidence.append(f"  Fleet 2 (colonizer): {changes}")
            raw_diffs = []
            for off in range(FLEET_RECORD_SIZE):
                if before_rec[off] != after_rec[off]:
                    raw_diffs.append(f"+0x{off:02x}:{before_rec[off]:02x}->{after_rec[off]:02x}")
            if raw_diffs:
                evidence.append(f"  Fleet 2 raw: {' '.join(raw_diffs)}")
            else:
                evidence.append(f"  Fleet 2: no changes")
        except (ValueError, FileNotFoundError):
            evidence.append(f"  Fleet 2: read error")

        # All fleet diffs
        evidence.append("  All fleet changes:")
        for i in range(16):
            if i == 2:
                continue  # already shown
            try:
                before_rec = read_record(before_snap / "FLEETS.DAT", i, FLEET_RECORD_SIZE)
                after_rec = read_record(target / "FLEETS.DAT", i, FLEET_RECORD_SIZE)
            except (ValueError, FileNotFoundError):
                continue
            changes = diff_record_fields(before_rec, after_rec, FLEET_FIELDS)
            if changes:
                evidence.append(f"    Fleet {i}: {changes}")

        # Planet 16 diff (colonize target)
        try:
            before_p = read_record(before_snap / "PLANETS.DAT", 16, PLANET_RECORD_SIZE)
            after_p = read_record(target / "PLANETS.DAT", 16, PLANET_RECORD_SIZE)
            changes = diff_record_fields(before_p, after_p, PLANET_FIELDS)
            if changes:
                evidence.append(f"  Planet 16 (target): {changes}")
            raw_diffs = []
            for off in range(PLANET_RECORD_SIZE):
                if before_p[off] != after_p[off]:
                    raw_diffs.append(f"+0x{off:02x}:{before_p[off]:02x}->{after_p[off]:02x}")
            if raw_diffs:
                evidence.append(f"  Planet 16 raw: {' '.join(raw_diffs)}")
            else:
                evidence.append(f"  Planet 16: no changes")
        except (ValueError, FileNotFoundError):
            evidence.append(f"  Planet 16: read error")

        # Planet 14 diff (homeworld economy)
        try:
            before_p = read_record(before_snap / "PLANETS.DAT", 14, PLANET_RECORD_SIZE)
            after_p = read_record(target / "PLANETS.DAT", 14, PLANET_RECORD_SIZE)
            changes = diff_record_fields(before_p, after_p, PLANET_FIELDS)
            if changes:
                evidence.append(f"  Planet 14 (homeworld): {changes}")
            raw_diffs = []
            for off in range(PLANET_RECORD_SIZE):
                if before_p[off] != after_p[off]:
                    raw_diffs.append(f"+0x{off:02x}:{before_p[off]:02x}->{after_p[off]:02x}")
            if raw_diffs:
                evidence.append(f"  Planet 14 raw: {' '.join(raw_diffs)}")
        except (ValueError, FileNotFoundError):
            pass

        # Output files
        for fname in ["RESULTS.DAT", "MESSAGES.DAT", "DATABASE.DAT", "ERRORS.TXT"]:
            bp = before_snap / fname
            ap = target / fname
            bs = bp.stat().st_size if bp.exists() else 0
            az = ap.stat().st_size if ap.exists() else 0
            if bs != az:
                evidence.append(f"  {fname}: size {bs} -> {az}")

        # RESULTS records
        tick_results = parse_results_records(target / "RESULTS.DAT")
        if tick_results:
            evidence.append(f"  RESULTS records: {len(tick_results)}")
            for rec in tick_results:
                evidence.append(f"    [{rec['index']:3d}] kind=0x{rec['kind']:02x} {rec['text'][:70]}")
                sdm = re.search(r"Stardate[:\s]+(\d+/\d+)", rec["text"])
                if sdm:
                    evidence.append(f"      -> Stardate: {sdm.group(1)}")

    conclusion = "INCONCLUSIVE"
    detail = "Track colonization fleet arrival, planet ownership change, and economy interaction."

    return ProbeResult(
        name="colonization-timing",
        conclusion=conclusion,
        evidence=evidence,
        detail=detail,
    )


# --- Registry ---

PROBES = {
    "econ-vs-movement": probe_econ_vs_movement,
    "production-vs-combat": probe_production_vs_combat,
    "command-normalization": probe_command_normalization,
    "econ-weekly-timing": probe_econ_weekly_timing,
    "invade-ordering": probe_invade_ordering,
    "colonization-timing": probe_colonization_timing,
}


def format_result(result: ProbeResult) -> str:
    lines = []
    lines.append("=" * 100)
    lines.append(f"ORDERING PROBE: {result.name}")
    lines.append(f"CONCLUSION: {result.conclusion}")
    lines.append("=" * 100)
    lines.append("")
    for line in result.evidence:
        lines.append(f"  {line}")
    lines.append("")
    lines.append(f"DETAIL: {result.detail}")
    lines.append("")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("probe", nargs="?", default=None, help="probe name or 'all'")
    parser.add_argument("--list", action="store_true", help="list available probes")
    args = parser.parse_args()

    if args.list or args.probe is None:
        print("Available ordering probes:")
        for name in PROBES:
            print(f"  {name}")
        return 0

    artifact_dir = ROOT / "artifacts" / "ecmaint-ordering-probes"
    artifact_dir.mkdir(parents=True, exist_ok=True)

    if args.probe == "all":
        probe_names = list(PROBES.keys())
    else:
        if args.probe not in PROBES:
            print(f"Unknown probe: {args.probe}")
            print(f"Known: {', '.join(PROBES.keys())}")
            return 1
        probe_names = [args.probe]

    for name in probe_names:
        print(f"\nRunning probe: {name}")
        print("-" * 60)
        result = PROBES[name]()
        output = format_result(result)
        print(output)

        out_path = artifact_dir / f"probe_{name}.txt"
        out_path.write_text(output)
        print(f"Written to: {out_path}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
