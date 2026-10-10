#!/usr/bin/env python3
"""Prepare and run black-box ECMAINT oracle experiments."""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_BASELINE = ROOT / "fixtures" / "ecmaint-post" / "v1.5"
CORE_FILES = [
    "PLAYER.DAT",
    "PLANETS.DAT",
    "FLEETS.DAT",
    "BASES.DAT",
    "IPBM.DAT",
    "SETUP.DAT",
    "CONQUEST.DAT",
]
TRACKED_FILES = [
    "PLAYER.DAT",
    "PLANETS.DAT",
    "FLEETS.DAT",
    "BASES.DAT",
    "IPBM.DAT",
    "SETUP.DAT",
    "CONQUEST.DAT",
    "DATABASE.DAT",
    "MESSAGES.DAT",
    "RESULTS.DAT",
    "RANKINGS.TXT",
    "ERRORS.TXT",
]

KNOWN_SCENARIOS = {
    "fleet-order": {
        "pre": ROOT / "fixtures" / "ecmaint-fleet-pre" / "v1.5",
        "post": ROOT / "fixtures" / "ecmaint-fleet-post" / "v1.5",
        "ticks": 1,
    },
    "planet-build": {
        "pre": ROOT / "fixtures" / "ecmaint-build-pre" / "v1.5",
        "post": ROOT / "fixtures" / "ecmaint-build-post" / "v1.5",
        "ticks": 1,
    },
    "guard-starbase": {
        "pre": ROOT / "fixtures" / "ecmaint-starbase-pre" / "v1.5",
        "post": ROOT / "fixtures" / "ecmaint-starbase-post" / "v1.5",
        "ticks": 1,
    },
    "ipbm": {
        "pre": ROOT / "fixtures" / "ecmaint-post" / "v1.5",
        "post": ROOT / "fixtures" / "ecmaint-post" / "v1.5",
        "ticks": 1,
    },
    "move": {
        "pre": ROOT / "fixtures" / "ecmaint-move-pre" / "v1.5",
        "post": ROOT / "fixtures" / "ecmaint-move-post" / "v1.5",
        "ticks": 3,
    },
    "bombard": {
        "pre": ROOT / "fixtures" / "ecmaint-bombard-pre" / "v1.5",
        "post": ROOT / "fixtures" / "ecmaint-bombard-post" / "v1.5",
        "ticks": 2,
    },
    "econ": {
        "pre": ROOT / "fixtures" / "ecmaint-econ-pre" / "v1.5",
        "post": ROOT / "fixtures" / "ecmaint-econ-post" / "v1.5",
        "ticks": 2,
    },
    "fleet-battle": {
        "pre": ROOT / "fixtures" / "ecmaint-fleet-battle-pre" / "v1.5",
        "post": ROOT / "fixtures" / "ecmaint-fleet-battle-post" / "v1.5",
        "ticks": 1,
    },
    "invade": {
        "pre": ROOT / "fixtures" / "ecmaint-invade-pre" / "v1.5",
        "post": ROOT / "fixtures" / "ecmaint-invade-post" / "v1.5",
        "ticks": 2,
    },
}


@dataclass(frozen=True)
class FileDiff:
    name: str
    before_size: int
    after_size: int
    differing_offsets: list[int]

    @property
    def differing_bytes(self) -> int:
        return len(self.differing_offsets)


def repo_relative(path: Path) -> str:
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return str(path)


def copy_tree(source: Path, target: Path) -> None:
    if target.exists():
        shutil.rmtree(target)
    shutil.copytree(source, target)


def snapshot_dir(target: Path, snapshot_name: str) -> Path:
    snapshot_root = target / ".oracle"
    snapshot_path = snapshot_root / snapshot_name
    if snapshot_path.exists():
        shutil.rmtree(snapshot_path)
    snapshot_path.mkdir(parents=True, exist_ok=True)
    for name in TRACKED_FILES:
        source = target / name
        if source.exists():
            shutil.copy2(source, snapshot_path / name)
    return snapshot_path


def read_bytes(path: Path) -> bytes:
    return path.read_bytes() if path.exists() else b""


def diff_offsets(before: bytes, after: bytes) -> list[int]:
    shared = min(len(before), len(after))
    offsets = [idx for idx in range(shared) if before[idx] != after[idx]]
    offsets.extend(range(shared, max(len(before), len(after))))
    return offsets


def summarize_clusters(offsets: list[int]) -> str:
    if not offsets:
        return "[]"
    clusters: list[str] = []
    start = prev = offsets[0]
    for offset in offsets[1:]:
        if offset == prev + 1:
            prev = offset
            continue
        clusters.append(f"{start}" if start == prev else f"{start}-{prev}")
        start = prev = offset
    clusters.append(f"{start}" if start == prev else f"{start}-{prev}")
    return "[" + ", ".join(clusters) + "]"


def collect_diffs(before_dir: Path, after_dir: Path) -> list[FileDiff]:
    diffs: list[FileDiff] = []
    for name in TRACKED_FILES:
        before = read_bytes(before_dir / name)
        after = read_bytes(after_dir / name)
        diffs.append(
            FileDiff(
                name=name,
                before_size=len(before),
                after_size=len(after),
                differing_offsets=diff_offsets(before, after),
            )
        )
    return diffs


def collect_expected_diffs(expected_dir: Path, actual_dir: Path) -> list[FileDiff]:
    diffs: list[FileDiff] = []
    for name in TRACKED_FILES:
        expected_path = expected_dir / name
        actual_path = actual_dir / name
        if not expected_path.exists() and not actual_path.exists():
            continue
        if not expected_path.exists() and name not in CORE_FILES and name != "DATABASE.DAT":
            # Preserve scenario comparisons should ignore generated files that are
            # not part of the preserved expected fixture, such as RANKINGS.TXT.
            continue
        expected = read_bytes(expected_path)
        actual = read_bytes(actual_path)
        diffs.append(
            FileDiff(
                name=name,
                before_size=len(expected),
                after_size=len(actual),
                differing_offsets=diff_offsets(expected, actual),
            )
        )
    return diffs


def run_ecmaint(target: Path, extra_env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    # Force headless SDL so DOSBox never opens a window, regardless of the
    # caller's environment (DISPLAY, WAYLAND_DISPLAY, etc.).
    env["SDL_VIDEODRIVER"] = "dummy"
    env["SDL_AUDIODRIVER"] = "dummy"
    if extra_env:
        env.update(extra_env)

    cmd = [
        "dosbox-x",
        "-defaultconf",
        "-nopromptfolder",
        "-nogui",
        "-nomenu",
        "-defaultdir",
        str(target),
        "-set",
        "dosv=off",
        "-set",
        "machine=vgaonly",
        "-set",
        "core=normal",
        "-set",
        "cputype=386_prefetch",
        "-set",
        "cycles=fixed 3000",
        "-set",
        "xms=false",
        "-set",
        "ems=false",
        "-set",
        "umb=false",
        "-set",
        "output=surface",
        "-c",
        f"mount c {target}",
        "-c",
        "c:",
        "-c",
        "ECMAINT /R",
        "-c",
        "exit",
    ]
    return subprocess.run(cmd, env=env, text=True, capture_output=True)


def run_nc_cli(args: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["cargo", "run", "-q", "-p", "nc-cli", "--", *args],
        cwd=ROOT / "rust",
        text=True,
        capture_output=True,
    )


def require_known_scenario(name: str) -> dict[str, Path]:
    if name not in KNOWN_SCENARIOS:
        raise SystemExit(
            f"unknown scenario: {name}. known scenarios: {', '.join(sorted(KNOWN_SCENARIOS))}"
        )
    return KNOWN_SCENARIOS[name]


def print_diff_summary(label: str, diffs: list[FileDiff]) -> None:
    print(label)
    for diff in diffs:
        print(
            f"  {diff.name}: size {diff.before_size} -> {diff.after_size}, "
            f"differing_bytes={diff.differing_bytes}, clusters={summarize_clusters(diff.differing_offsets)}"
        )


def cmd_prepare(args: argparse.Namespace) -> int:
    source = Path(args.source).resolve()
    target = Path(args.target).resolve()
    copy_tree(source, target)
    ensure_engine(target)
    snapshot_path = snapshot_dir(target, "prepared")
    print("Prepared ECMAINT oracle directory")
    print(f"  source={repo_relative(source)}")
    print(f"  target={target}")
    print(f"  snapshot={snapshot_path}")
    print("  next step: submit player orders or mutate files, then run:")
    print(f"    python3 tools/ncmaint_oracle.py run {target}")
    return 0


def ensure_engine(target: Path) -> None:
    engine = target / "ECMAINT.EXE"
    if not engine.exists():
        source_engine = ROOT / "original" / "v1.5" / "ECMAINT.EXE"
        if source_engine.exists():
            shutil.copy2(source_engine, engine)


def cmd_prepare_known(args: argparse.Namespace) -> int:
    target = Path(args.target).resolve()
    source = Path(args.source).resolve() if args.source else DEFAULT_BASELINE
    require_known_scenario(args.scenario)
    result = run_nc_cli(["scenario-init-replayable", str(source), str(target), args.scenario])
    if result.returncode != 0:
        sys.stdout.write(result.stdout)
        sys.stderr.write(result.stderr)
        raise SystemExit(result.returncode)
    ensure_engine(target)
    snapshot_path = snapshot_dir(target, "prepared")
    print(result.stdout.strip())
    print("Prepared known ECMAINT scenario directory")
    print(f"  scenario={args.scenario}")
    print(f"  source={repo_relative(source)}")
    print(f"  target={target}")
    print(f"  snapshot={snapshot_path}")
    print(f"  expected_post={repo_relative(KNOWN_SCENARIOS[args.scenario]['post'])}")
    print("  next step:")
    print(f"    python3 tools/ncmaint_oracle.py run {target}")
    print(f"    python3 tools/ncmaint_oracle.py compare {target} {KNOWN_SCENARIOS[args.scenario]['post']}")
    return 0


def cmd_run(args: argparse.Namespace) -> int:
    target = Path(args.target).resolve()
    if not target.exists():
        raise SystemExit(f"target does not exist: {target}")

    before_snapshot = snapshot_dir(target, "before-ecmaint")
    result = run_ecmaint(target)
    after_snapshot = snapshot_dir(target, "after-ecmaint")
    diffs = collect_diffs(before_snapshot, target)

    oracle_root = target / ".oracle"
    (oracle_root / "dosbox.stdout.txt").write_text(result.stdout, encoding="utf-8", errors="ignore")
    (oracle_root / "dosbox.stderr.txt").write_text(result.stderr, encoding="utf-8", errors="ignore")

    print("ECMAINT oracle run complete")
    print(f"  target={target}")
    print(f"  before_snapshot={before_snapshot}")
    print(f"  after_snapshot={after_snapshot}")
    print(f"  dosbox_exit_code={result.returncode}")
    print_diff_summary("  file_diffs", diffs)
    if (target / "ERRORS.TXT").exists():
        first_line = (target / "ERRORS.TXT").read_text(errors="ignore").splitlines()[:1]
        if first_line:
            print(f"  ERRORS.TXT first line: {first_line[0]}")
    return 0


def cmd_compare(args: argparse.Namespace) -> int:
    target = Path(args.target).resolve()
    expected = Path(args.expected).resolve()
    diffs = collect_expected_diffs(expected, target)
    print("ECMAINT oracle comparison")
    print(f"  target={target}")
    print(f"  expected={expected}")
    print_diff_summary("  file_diffs", diffs)
    return 0


def cmd_replay_preserved(args: argparse.Namespace) -> int:
    scenario = require_known_scenario(args.scenario)
    target = Path(args.target).resolve()
    ticks: int = scenario.get("ticks", 1)  # type: ignore[assignment]
    prepare_args = argparse.Namespace(target=str(target), source=str(scenario["pre"]))
    cmd_prepare(prepare_args)
    run_args = argparse.Namespace(target=str(target))
    for _ in range(ticks):
        cmd_run(run_args)
    compare_args = argparse.Namespace(target=str(target), expected=str(scenario["post"]))
    cmd_compare(compare_args)
    return 0


def cmd_replay_known(args: argparse.Namespace) -> int:
    scenario = require_known_scenario(args.scenario)
    source = Path(args.source).resolve() if args.source else DEFAULT_BASELINE
    target = Path(args.target).resolve()
    ticks: int = scenario.get("ticks", 1)  # type: ignore[assignment]
    prepare_args = argparse.Namespace(scenario=args.scenario, source=str(source), target=str(target))
    cmd_prepare_known(prepare_args)
    run_args = argparse.Namespace(target=str(target))
    for _ in range(ticks):
        cmd_run(run_args)
    compare_args = argparse.Namespace(target=str(target), expected=str(scenario["post"]))
    cmd_compare(compare_args)
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    prepare = subparsers.add_parser("prepare", help="copy a baseline into a working directory")
    prepare.add_argument("target", help="working directory to create")
    prepare.add_argument(
        "source",
        nargs="?",
        default=str(DEFAULT_BASELINE),
        help="baseline directory to copy (default: fixtures/ecmaint-post/v1.5)",
    )
    prepare.set_defaults(func=cmd_prepare)

    prepare_known = subparsers.add_parser(
        "prepare-known",
        help="materialize a known accepted pre-maint scenario through nc-cli",
    )
    prepare_known.add_argument("scenario", help="known scenario name")
    prepare_known.add_argument("target", help="working directory to create")
    prepare_known.add_argument(
        "source",
        nargs="?",
        default=None,
        help="optional baseline directory for nc-cli scenario-init",
    )
    prepare_known.set_defaults(func=cmd_prepare_known)

    run = subparsers.add_parser("run", help="snapshot, run ECMAINT, and diff results")
    run.add_argument("target", help="working directory to process in place")
    run.set_defaults(func=cmd_run)

    compare = subparsers.add_parser(
        "compare",
        help="compare a directory against an expected post-maint fixture directory",
    )
    compare.add_argument("target", help="directory to inspect")
    compare.add_argument("expected", help="expected post-maint directory")
    compare.set_defaults(func=cmd_compare)

    replay_known = subparsers.add_parser(
        "replay-known",
        help="materialize a known pre-maint scenario, run ECMAINT, and compare to its preserved post fixture",
    )
    replay_known.add_argument("scenario", help="known scenario name")
    replay_known.add_argument("target", help="working directory to create and run")
    replay_known.add_argument(
        "source",
        nargs="?",
        default=None,
        help="optional baseline directory for nc-cli scenario-init",
    )
    replay_known.set_defaults(func=cmd_replay_known)

    replay_preserved = subparsers.add_parser(
        "replay-preserved",
        help="run ECMAINT from the preserved pre-maint fixture and compare to the preserved post fixture",
    )
    replay_preserved.add_argument("scenario", help="known scenario name")
    replay_preserved.add_argument("target", help="working directory to create and run")
    replay_preserved.set_defaults(func=cmd_replay_preserved)

    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
