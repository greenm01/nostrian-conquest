#!/usr/bin/env python3
"""Capture Borland Pascal RandSeed after ECMAINT exits using a DOS COM stub.

Instead of driving the DOSBox debugger via pexpect (slow, fragile), this
runs ECMAINT normally and then immediately runs READSEED.COM in the same
DOS session.  READSEED.COM reads the 4-byte RandSeed from the still-intact
ECMAINT data segment (3529:03A6) and writes it to SEED.BIN.

The captured seed is the *post-run* RandSeed.  Combined with the known
initial seed (0x000E000E at the 96c4 bridge) and the Borland Pascal LCG
(seed = seed * 0x08088405 + 1), every intermediate seed value can be
computed by forward-stepping the LCG from the initial seed or
reverse-stepping from the final seed.

Usage:
    python3 tools/capture_randseed.py [scenario ...]
    python3 tools/capture_randseed.py all

Scenarios: bombard, econ, fleet-order, planet-build (the 4 non-combat,
non-destruction scenarios with known visit orders from file-I/O traces).
"""

from __future__ import annotations

import json
import os
import shutil
import struct
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# Known visit orders from artifacts/ecmaint-fileio-trace/cross_scenario_comparison.txt
KNOWN_VISIT_ORDERS: dict[str, list[int]] = {
    "bombard":      [11, 15, 0, 10, 4, 3, 2, 1, 14, 5, 13, 8, 7, 6, 9, 12],
    "econ":         [11, 1, 4, 14, 12, 8, 3, 15, 0, 5, 7, 6, 9, 13, 2, 10],
    "fleet-order":  [6, 3, 7, 1, 2, 9, 13, 12, 4, 5, 0, 15, 10, 14, 11, 8],
    "planet-build": [15, 12, 9, 4, 0, 3, 7, 8, 11, 14, 2, 1, 13, 6, 10, 5],
}


def prepare(fixture_src: Path, target: Path) -> None:
    """Copy fixture to working directory, add ECMAINT.EXE and READSEED.COM."""
    if target.exists():
        shutil.rmtree(target)
    shutil.copytree(fixture_src, target)
    engine = target / "ECMAINT.EXE"
    if not engine.exists():
        shutil.copy2(ROOT / "original" / "v1.5" / "ECMAINT.EXE", engine)
    # Generate READSEED.COM if needed, then copy to target
    readseed_src = ROOT / "tools" / "READSEED.COM"
    if not readseed_src.exists():
        subprocess.run(
            [sys.executable, str(ROOT / "tools" / "readseed_com.py")],
            check=True,
        )
    shutil.copy2(readseed_src, target / "READSEED.COM")


def run_ecmaint_then_readseed(target: Path) -> subprocess.CompletedProcess[str]:
    """Run ECMAINT /R then READSEED in the same DOS session."""
    env = os.environ.copy()
    env["SDL_VIDEODRIVER"] = "dummy"
    env["SDL_AUDIODRIVER"] = "dummy"
    cmd = [
        "dosbox-x",
        "-defaultconf", "-nopromptfolder", "-nogui", "-nomenu",
        "-defaultdir", str(target),
        "-set", "dosv=off",
        "-set", "machine=vgaonly",
        "-set", "core=normal",
        "-set", "cputype=386_prefetch",
        "-set", "cycles=fixed 3000",
        "-set", "xms=false",
        "-set", "ems=false",
        "-set", "umb=false",
        "-set", "output=surface",
        "-c", f"mount c {target}",
        "-c", "c:",
        "-c", "ECMAINT /R",
        "-c", "READSEED",
        "-c", "exit",
    ]
    return subprocess.run(cmd, env=env, text=True, capture_output=True, timeout=120)


def read_seed_bin(target: Path) -> int | None:
    """Read the 4-byte little-endian seed from SEED.BIN."""
    seed_path = target / "SEED.BIN"
    if not seed_path.exists():
        return None
    data = seed_path.read_bytes()
    if len(data) < 4:
        return None
    return struct.unpack("<I", data[:4])[0]


def capture_for_scenario(scenario: str, fixture_src: Path) -> dict:
    """Run one scenario and capture the post-run RandSeed."""
    target = Path(f"/tmp/ecmaint-randseed-{scenario}")
    prepare(fixture_src, target)

    print(f"  Running ECMAINT /R + READSEED for {scenario}...")
    result = run_ecmaint_then_readseed(target)

    if result.returncode != 0:
        print(f"  WARNING: DOSBox exited with code {result.returncode}")

    errors_path = target / "ERRORS.TXT"
    if errors_path.exists():
        first = errors_path.read_text(errors="ignore").splitlines()[:1]
        if first:
            print(f"  ERRORS.TXT: {first[0]}")

    seed = read_seed_bin(target)
    if seed is not None:
        print(f"  Post-run RandSeed: 0x{seed:08X}")
    else:
        print("  WARNING: SEED.BIN not found or too short")

    return {
        "scenario": scenario,
        "post_run_seed": seed,
        "visit_order": KNOWN_VISIT_ORDERS.get(scenario),
    }


def main() -> int:
    from ncmaint_oracle import KNOWN_SCENARIOS

    scenarios = sys.argv[1:] if len(sys.argv) > 1 else ["bombard"]
    if scenarios == ["all"]:
        scenarios = list(KNOWN_VISIT_ORDERS.keys())

    results = []
    for scenario in scenarios:
        if scenario not in KNOWN_SCENARIOS:
            print(f"Unknown scenario: {scenario}")
            continue
        print(f"\n{'='*60}")
        print(f"Scenario: {scenario}")
        print(f"{'='*60}")
        result = capture_for_scenario(scenario, KNOWN_SCENARIOS[scenario]["pre"])
        results.append(result)

    # Summary
    print(f"\n{'='*60}")
    print("RANDSEED CAPTURE SUMMARY")
    print(f"{'='*60}")
    for r in results:
        seed_str = f"0x{r['post_run_seed']:08X}" if r['post_run_seed'] is not None else "MISSING"
        print(f"  {r['scenario']:15s} post_seed={seed_str}")
        if r['visit_order']:
            print(f"  {'':15s} visit_order={r['visit_order']}")

    # Write artifact
    artifact_dir = ROOT / "artifacts" / "ecmaint-randseed"
    artifact_dir.mkdir(parents=True, exist_ok=True)

    lines = [
        "ECMAINT RandSeed Capture (COM-stub method)",
        "=" * 50,
        "",
        "Method: READSEED.COM runs after ECMAINT /R in the same DOS session.",
        "READSEED.COM reads 4 bytes from 3529:03A6 (Borland Pascal RandSeed)",
        "and writes them to SEED.BIN.",
        "",
        "PRNG: RandSeed = RandSeed * 0x08088405 + 1 (Borland Pascal LCG)",
        "Initial seed at bridge (96c4): 0x000E000E",
        "",
    ]
    for r in results:
        seed_str = f"0x{r['post_run_seed']:08X}" if r['post_run_seed'] is not None else "MISSING"
        lines.append(f"Scenario: {r['scenario']}")
        lines.append(f"  Post-run RandSeed: {seed_str}")
        if r['visit_order']:
            lines.append(f"  Known visit order: {r['visit_order']}")
        lines.append("")

    artifact_path = artifact_dir / "randseed_capture.txt"
    artifact_path.write_text("\n".join(lines))
    print(f"\nArtifact: {artifact_path}")

    # Also write machine-readable JSON
    json_path = artifact_dir / "randseed_capture.json"
    json_path.write_text(json.dumps(results, indent=2))
    print(f"JSON:     {json_path}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
