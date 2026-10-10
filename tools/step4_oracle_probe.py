#!/usr/bin/env python3
"""Run targeted step-4 ECMAINT probes with per-tick snapshots and summaries."""

from __future__ import annotations

import argparse
import shutil
from dataclasses import dataclass
from pathlib import Path

from ncmaint_oracle import (
    TRACKED_FILES,
    collect_diffs,
    copy_tree,
    ensure_engine,
    run_ecmaint,
    snapshot_dir,
    summarize_clusters,
)


PLANET_RECORD_SIZE = 0x61
REPORT_FILES = [
    "RESULTS.DAT",
    "MESSAGES.DAT",
    "ERRORS.TXT",
    "DATABASE.DAT",
    "RANKINGS.TXT",
]


@dataclass(frozen=True)
class ByteEdit:
    record_index: int
    offset: int
    value: int


def parse_edit(text: str) -> ByteEdit:
    try:
        lhs, value_text = text.split("=")
        record_text, offset_text = lhs.split(":")
        record_index = int(record_text, 0)
        offset = int(offset_text, 0)
        value = int(value_text, 0)
    except ValueError as exc:
        raise argparse.ArgumentTypeError(
            f"invalid edit '{text}', expected RECORD:OFFSET=VALUE"
        ) from exc

    if record_index < 0:
        raise argparse.ArgumentTypeError("record index must be >= 0")
    if offset < 0 or offset >= PLANET_RECORD_SIZE:
        raise argparse.ArgumentTypeError(
            f"planet offset must be within 0..{PLANET_RECORD_SIZE - 1}"
        )
    if value < 0 or value > 0xFF:
        raise argparse.ArgumentTypeError("value must be within 0..255")

    return ByteEdit(record_index, offset, value)


def record_bytes(path: Path, record_index: int, record_size: int) -> bytes:
    data = path.read_bytes()
    start = record_index * record_size
    end = start + record_size
    if end > len(data):
        raise SystemExit(
            f"record {record_index} out of range for {path.name} "
            f"(size={len(data)}, record_size={record_size})"
        )
    return data[start:end]


def apply_planet_edits(target: Path, edits: list[ByteEdit]) -> None:
    if not edits:
        return
    path = target / "PLANETS.DAT"
    data = bytearray(path.read_bytes())
    for edit in edits:
        index = edit.record_index * PLANET_RECORD_SIZE + edit.offset
        if index >= len(data):
            raise SystemExit(
                f"edit {edit.record_index}:{edit.offset} out of range for PLANETS.DAT"
            )
        data[index] = edit.value
    path.write_bytes(bytes(data))


def changed_offsets(before: bytes, after: bytes) -> list[int]:
    shared = min(len(before), len(after))
    offsets = [idx for idx in range(shared) if before[idx] != after[idx]]
    offsets.extend(range(shared, max(len(before), len(after))))
    return offsets


def summarize_record_delta(name: str, before: bytes, after: bytes) -> list[str]:
    offsets = changed_offsets(before, after)
    if not offsets:
        return [f"    {name}: unchanged"]
    lines = [
        f"    {name}: differing_bytes={len(offsets)}, clusters={summarize_clusters(offsets)}"
    ]
    for offset in offsets:
        before_byte = before[offset] if offset < len(before) else None
        after_byte = after[offset] if offset < len(after) else None
        lines.append(
            f"      +0x{offset:02x}: "
            f"{format_byte(before_byte)} -> {format_byte(after_byte)}"
        )
    return lines


def format_byte(value: int | None) -> str:
    if value is None:
        return "--"
    return f"0x{value:02x}"


def print_nonzero_file_diffs(before_dir: Path, after_dir: Path) -> None:
    diffs = collect_diffs(before_dir, after_dir)
    printed = False
    for diff in diffs:
        if not diff.differing_offsets and diff.before_size == diff.after_size:
            continue
        if not printed:
            print("  file_diffs")
            printed = True
        print(
            f"    {diff.name}: size {diff.before_size} -> {diff.after_size}, "
            f"differing_bytes={len(diff.differing_offsets)}, "
            f"clusters={summarize_clusters(diff.differing_offsets)}"
        )
    if not printed:
        print("  file_diffs")
        print("    no tracked file changes")


def print_report_status(target: Path) -> None:
    print("  report_outputs")
    for name in REPORT_FILES:
        path = target / name
        if not path.exists():
            print(f"    {name}: missing")
            continue
        size = path.stat().st_size
        status = "non-empty" if size else "empty"
        print(f"    {name}: {status}, bytes={size}")
        if name == "ERRORS.TXT" and size:
            first_line = path.read_text(errors="ignore").splitlines()[:1]
            if first_line:
                print(f"      first_line={first_line[0]}")


def cmd_probe(args: argparse.Namespace) -> int:
    source = Path(args.source).resolve()
    target = Path(args.target).resolve()
    watch_records = sorted(set(args.watch_planet))

    if target.exists():
        shutil.rmtree(target)
    copy_tree(source, target)
    ensure_engine(target)
    snapshot_dir(target, "prepared")

    apply_planet_edits(target, args.planet_edit)
    edited_snapshot = snapshot_dir(target, "edited")

    print("Prepared step-4 probe directory")
    print(f"  source={source}")
    print(f"  target={target}")
    print(f"  edited_snapshot={edited_snapshot}")
    if args.planet_edit:
        print("  planet_edits")
        for edit in args.planet_edit:
            print(
                f"    record={edit.record_index} +0x{edit.offset:02x} = 0x{edit.value:02x}"
            )

    for tick in range(1, args.ticks + 1):
        before_snapshot = snapshot_dir(target, f"tick-{tick:02d}-before")
        before_records = {
            record_index: record_bytes(target / "PLANETS.DAT", record_index, PLANET_RECORD_SIZE)
            for record_index in watch_records
        }

        result = run_ecmaint(target)
        if result.returncode != 0:
            raise SystemExit(f"ECMAINT failed on tick {tick} with exit code {result.returncode}")

        after_snapshot = snapshot_dir(target, f"tick-{tick:02d}-after")

        print(f"Tick {tick}")
        print_nonzero_file_diffs(before_snapshot, target)
        print_report_status(target)

        if watch_records:
            print("  watched_planets")
            for record_index in watch_records:
                after_record = record_bytes(target / "PLANETS.DAT", record_index, PLANET_RECORD_SIZE)
                for line in summarize_record_delta(
                    f"record {record_index}", before_records[record_index], after_record
                ):
                    print(line)

        oracle_root = target / ".oracle"
        (oracle_root / f"tick-{tick:02d}-dosbox.stdout.txt").write_text(
            result.stdout, encoding="utf-8", errors="ignore"
        )
        (oracle_root / f"tick-{tick:02d}-dosbox.stderr.txt").write_text(
            result.stderr, encoding="utf-8", errors="ignore"
        )

        print(f"  before_snapshot={before_snapshot}")
        print(f"  after_snapshot={after_snapshot}")

    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", help="source fixture/directory to clone")
    parser.add_argument("target", help="disposable working directory")
    parser.add_argument("--ticks", type=int, default=1, help="number of ECMAINT ticks to run")
    parser.add_argument(
        "--planet-edit",
        action="append",
        type=parse_edit,
        default=[],
        help="planet byte edit as RECORD:OFFSET=VALUE; may be repeated",
    )
    parser.add_argument(
        "--watch-planet",
        action="append",
        type=int,
        default=[],
        help="planet record index to summarize after each tick; may be repeated",
    )
    parser.set_defaults(func=cmd_probe)
    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
