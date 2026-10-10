#!/usr/bin/env python3
"""Capture full ECMAINT file I/O trace for coarse phase-boundary evidence.

Runs ECMAINT with DOSBox-X debug logging enabled, then parses the I/O trace
to determine the order in which DAT files are read and written. This does not
prove the exact simulation-subphase order, but it does expose broad boundaries
such as "heavy state mutation" versus later rebuild/flush work.
"""

from __future__ import annotations

import os
import re
import shutil
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ARTIFACT_DIR = ROOT / "artifacts" / "ecmaint-fileio-trace"

READ_RE = re.compile(r"DEBUG FILES:Reading (\d+) bytes from ([^ ]+)")
WRITE_RE = re.compile(r"DEBUG FILES:Writing (\d+) bytes to ([^ ]+)")
OPEN_RE = re.compile(r"FILES:(?:file open command \d+ file|Special file open command \d+ file) (.+)")
CLOSE_RE = re.compile(r"FILES:Closing file (.+)")
SEEK_RE = re.compile(r"DEBUG FILES:Seeking to (\d+) bytes from position type \((\d+)\) in ([^ ]+)")
CREATE_RE = re.compile(r"FILES:(?:file create command \d+ file|Special file create command \d+ file) (.+)")

GAME_FILES = {
    "CONQUEST.DAT", "SETUP.DAT", "PLAYER.DAT", "PLANETS.DAT",
    "FLEETS.DAT", "BASES.DAT", "IPBM.DAT", "DATABASE.DAT",
    "MESSAGES.DAT", "RESULTS.DAT", "RANKINGS.TXT", "ERRORS.TXT",
    "MAIN.TOK", "PLAYER.TOK", "PLANETS.TOK", "FLEETS.TOK",
    "DATABASE.TOK", "CONQUEST.TOK", "MOVE.TOK",
}


@dataclass
class IOEvent:
    index: int
    kind: str  # open, close, read, write, seek, create
    name: str
    detail: str


def normalize_name(name: str) -> str:
    name = name.strip().strip('"')
    if "\\" in name:
        name = name.rsplit("\\", 1)[-1]
    return name.upper()


def run_ecmaint_with_logging(target: Path, log_path: Path) -> int:
    env = os.environ.copy()
    env["SDL_VIDEODRIVER"] = "dummy"
    env["SDL_AUDIODRIVER"] = "dummy"
    cmd = [
        "dosbox-x", "-defaultconf", "-nopromptfolder", "-nogui", "-nomenu",
        "-defaultdir", str(target),
        "-debug", "-log-int21", "-log-fileio",
        "-time-limit", "30",
        "-set", "dosv=off", "-set", "machine=vgaonly", "-set", "core=normal",
        "-set", "cputype=386_prefetch", "-set", "cycles=fixed 3000",
        "-set", "xms=false", "-set", "ems=false", "-set", "umb=false",
        "-set", "output=surface",
        "-c", f"mount c {target}", "-c", "c:",
        "-c", "ECMAINT /R", "-c", "exit",
    ]
    with log_path.open("w") as handle:
        result = subprocess.run(cmd, stdout=handle, stderr=subprocess.STDOUT, env=env, check=False)
    return result.returncode


def parse_trace(log_path: Path) -> list[IOEvent]:
    events: list[IOEvent] = []
    armed = False
    idx = 0

    for line in log_path.read_text(errors="ignore").splitlines():
        if "Execute ECMAINT.EXE" in line or "ECMAINT" in line and "Execute" in line:
            armed = True
        if not armed:
            continue

        match = OPEN_RE.search(line)
        if match:
            name = normalize_name(match.group(1))
            if name in GAME_FILES:
                events.append(IOEvent(idx, "open", name, ""))
                idx += 1
            continue

        match = CREATE_RE.search(line)
        if match:
            name = normalize_name(match.group(1))
            if name in GAME_FILES:
                events.append(IOEvent(idx, "create", name, ""))
                idx += 1
            continue

        match = READ_RE.search(line)
        if match:
            name = normalize_name(match.group(2))
            if name in GAME_FILES:
                events.append(IOEvent(idx, "read", name, f"{match.group(1)} bytes"))
                idx += 1
            continue

        match = WRITE_RE.search(line)
        if match:
            name = normalize_name(match.group(2))
            if name in GAME_FILES:
                events.append(IOEvent(idx, "write", name, f"{match.group(1)} bytes"))
                idx += 1
            continue

        match = SEEK_RE.search(line)
        if match:
            name = normalize_name(match.group(3))
            if name in GAME_FILES:
                events.append(IOEvent(idx, "seek", name, f"offset={match.group(1)} whence={match.group(2)}"))
                idx += 1
            continue

        match = CLOSE_RE.search(line)
        if match:
            name = normalize_name(match.group(1))
            if name in GAME_FILES:
                events.append(IOEvent(idx, "close", name, ""))
                idx += 1

    return events


def summarize_phases(events: list[IOEvent]) -> str:
    """Group I/O events into phases based on file open/close boundaries."""
    lines = []
    lines.append("=" * 100)
    lines.append("FILE I/O PHASE SUMMARY")
    lines.append("=" * 100)
    lines.append("")

    # Track first/last write to each file for ordering
    first_write: dict[str, int] = {}
    last_write: dict[str, int] = {}
    first_read: dict[str, int] = {}
    write_counts: dict[str, int] = {}
    read_counts: dict[str, int] = {}
    write_bytes: dict[str, int] = {}

    for ev in events:
        if ev.kind == "write":
            if ev.name not in first_write:
                first_write[ev.name] = ev.index
            last_write[ev.name] = ev.index
            write_counts[ev.name] = write_counts.get(ev.name, 0) + 1
            nbytes = int(ev.detail.split()[0]) if ev.detail else 0
            write_bytes[ev.name] = write_bytes.get(ev.name, 0) + nbytes
        elif ev.kind == "read":
            if ev.name not in first_read:
                first_read[ev.name] = ev.index
            read_counts[ev.name] = read_counts.get(ev.name, 0) + 1

    lines.append("WRITE ORDER (first write to each file):")
    for name, idx in sorted(first_write.items(), key=lambda x: x[1]):
        lines.append(f"  [{idx:4d}] {name:<20s}  writes={write_counts.get(name,0)}  total_bytes={write_bytes.get(name,0)}")

    lines.append("")
    lines.append("READ ORDER (first read of each file):")
    for name, idx in sorted(first_read.items(), key=lambda x: x[1]):
        lines.append(f"  [{idx:4d}] {name:<20s}  reads={read_counts.get(name,0)}")

    lines.append("")
    lines.append("=" * 100)
    lines.append("WRITE CLUSTERING (groups of consecutive writes to same file):")
    lines.append("=" * 100)
    lines.append("")

    # Find write clusters
    write_events = [ev for ev in events if ev.kind == "write"]
    if write_events:
        clusters = []
        current_name = write_events[0].name
        current_start = write_events[0].index
        current_count = 1
        current_bytes = int(write_events[0].detail.split()[0]) if write_events[0].detail else 0

        for we in write_events[1:]:
            if we.name == current_name:
                current_count += 1
                current_bytes += int(we.detail.split()[0]) if we.detail else 0
            else:
                clusters.append((current_name, current_start, current_count, current_bytes))
                current_name = we.name
                current_start = we.index
                current_count = 1
                current_bytes = int(we.detail.split()[0]) if we.detail else 0
        clusters.append((current_name, current_start, current_count, current_bytes))

        for name, start, count, nbytes in clusters:
            lines.append(f"  [{start:4d}] {name:<20s}  {count:3d} writes  {nbytes:6d} bytes")

    return "\n".join(lines) + "\n"


def main() -> int:
    scenario = sys.argv[1] if len(sys.argv) > 1 else "fleet-order"

    from ncmaint_oracle import KNOWN_SCENARIOS
    if scenario not in KNOWN_SCENARIOS:
        print(f"Unknown scenario: {scenario}")
        print(f"Known: {', '.join(sorted(KNOWN_SCENARIOS))}")
        return 1

    fixture_src = KNOWN_SCENARIOS[scenario]["pre"]
    ticks = KNOWN_SCENARIOS[scenario]["ticks"]
    target = Path(f"/tmp/ecmaint-fileio-trace-{scenario}")

    print(f"Scenario: {scenario} (ticks={ticks})")
    print(f"Fixture: {fixture_src}")
    print(f"Target: {target}")

    # Prepare
    if target.exists():
        shutil.rmtree(target)
    shutil.copytree(fixture_src, target)
    engine = target / "ECMAINT.EXE"
    if not engine.exists():
        fallback = ROOT / "original" / "v1.5" / "ECMAINT.EXE"
        if fallback.exists():
            shutil.copy2(fallback, engine)

    # Run with logging
    log_path = target / "dosbox_fileio.log"
    print(f"Running ECMAINT with file I/O logging...")
    rc = run_ecmaint_with_logging(target, log_path)
    print(f"Exit code: {rc}")
    print(f"Log size: {log_path.stat().st_size:,} bytes")

    # Parse
    events = parse_trace(log_path)
    print(f"Parsed {len(events)} game-file I/O events")

    # Write artifacts
    ARTIFACT_DIR.mkdir(parents=True, exist_ok=True)

    # Full event trace
    trace_lines = []
    trace_lines.append(f"ECMAINT File I/O Trace — scenario={scenario}")
    trace_lines.append(f"Total events: {len(events)}")
    trace_lines.append("")
    for ev in events:
        trace_lines.append(f"[{ev.index:4d}] {ev.kind:<6s}  {ev.name:<20s}  {ev.detail}")
    (ARTIFACT_DIR / f"fileio_trace_{scenario}.txt").write_text("\n".join(trace_lines) + "\n")

    # Phase summary
    summary = summarize_phases(events)
    (ARTIFACT_DIR / f"phase_summary_{scenario}.txt").write_text(summary)
    print()
    print(summary)

    # Run fleet/database analysis
    from analyze_fileio_trace import analyze_trace, format_analysis
    trace_file = ARTIFACT_DIR / f"fileio_trace_{scenario}.txt"
    analysis = analyze_trace(trace_file)
    analysis_text = format_analysis(analysis)
    (ARTIFACT_DIR / f"analysis_{scenario}.txt").write_text(analysis_text)
    print()
    print(analysis_text)

    print(f"\nArtifacts written to: {ARTIFACT_DIR}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
