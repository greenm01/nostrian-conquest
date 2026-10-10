#!/usr/bin/env python3
"""Probe original ECMAINT behavior when a build completes into a full stardock."""

from __future__ import annotations

import argparse
import shutil
from pathlib import Path

from ncmaint_oracle import ROOT, collect_diffs, print_diff_summary, run_nc_cli, run_ecmaint, snapshot_dir

PLANET_RECORD_SIZE = 97
TARGET_PLANET_RECORD = 15


def prepare_case(target: Path) -> None:
    source = ROOT / "fixtures" / "ecmaint-build-pre" / "v1.5"
    if target.exists():
        shutil.rmtree(target)
    shutil.copytree(source, target)

    for slot in range(10):
        result = run_nc_cli(
            ["planet-stardock", str(target), str(TARGET_PLANET_RECORD), str(slot), "1", "1"]
        )
        if result.returncode != 0:
            raise SystemExit(result.stderr or result.stdout)


def planet_record_bytes(path: Path, record_index_1_based: int) -> bytes:
    data = path.read_bytes()
    start = (record_index_1_based - 1) * PLANET_RECORD_SIZE
    end = start + PLANET_RECORD_SIZE
    return data[start:end]


def summarize_planet(record: bytes) -> str:
    build_counts = list(record[0x24:0x2E])
    build_kinds = list(record[0x2E:0x38])
    stardock_counts = [
        int.from_bytes(record[0x38 + slot * 2 : 0x38 + slot * 2 + 2], "little")
        for slot in range(10)
    ]
    stardock_kinds = list(record[0x4C:0x56])
    return "\n".join(
        [
            f"build_counts={build_counts}",
            f"build_kinds={build_kinds}",
            f"stardock_counts={stardock_counts}",
            f"stardock_kinds={stardock_kinds}",
        ]
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "target",
        nargs="?",
        default="/tmp/ecmaint-full-stardock-build",
        help="working directory",
    )
    args = parser.parse_args()
    target = Path(args.target)

    prepare_case(target)
    before = snapshot_dir(target, "before")
    result = run_ecmaint(target)
    after = snapshot_dir(target, "after")

    print(f"Target: {target}")
    print(f"ECMAINT return code: {result.returncode}")
    if result.stdout.strip():
        print("ECMAINT stdout:")
        print(result.stdout)
    if result.stderr.strip():
        print("ECMAINT stderr:")
        print(result.stderr)

    print_diff_summary("Diffs after ECMAINT", collect_diffs(before, after))

    before_planet = planet_record_bytes(before / "PLANETS.DAT", TARGET_PLANET_RECORD)
    after_planet = planet_record_bytes(after / "PLANETS.DAT", TARGET_PLANET_RECORD)
    print("Before PLANETS.DAT target record:")
    print(summarize_planet(before_planet))
    print("After PLANETS.DAT target record:")
    print(summarize_planet(after_planet))

    errors = target / "ERRORS.TXT"
    if errors.exists():
        print("ERRORS.TXT:")
        print(errors.read_text(errors="ignore"))


if __name__ == "__main__":
    main()
