#!/usr/bin/env python3
"""Audit GuardStarbase runtime bytes against classic ECMAINT."""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

from ncmaint_oracle import DEFAULT_BASELINE, ROOT, ensure_engine, run_ecmaint


RUST_WORKSPACE = ROOT / "rust"
NC_CLI_BIN = RUST_WORKSPACE / "target" / "debug" / "nc-cli"
DEFAULT_WORK_ROOT = Path("/tmp/ecmaint-guard-starbase-audit")
DEFAULT_OUTPUT = ROOT / "docs" / "dev" / "guard-starbase-runtime-audit.md"
NC_CLI_READY = False
BASE_RECORD_SIZE = 35


@dataclass(frozen=True)
class GuardStarbaseProbeCase:
    name: str
    start: tuple[int, int]
    target: tuple[int, int]
    turns: int
    destroy_base_after_turn: int | None = None
    compare_rust: bool = True


@dataclass
class FleetTrace:
    turn: int
    coords: tuple[int, int]
    target: tuple[int, int]
    order: str
    order_code: int
    current_speed: int
    mission_aux: str
    raw_0d: str
    raw_0e: str
    raw_0f: str
    raw_10: str
    raw_11: str
    raw_12: str
    raw_19: str
    raw_1a: str
    raw_1b: str
    raw_1c: str
    raw_1d: str
    raw_1e: str


@dataclass
class BaseTrace:
    turn: int
    player_starbase_count: int
    base_count: int
    base_active_flag: int | None
    base_id: int | None
    base_coords: tuple[int, int] | None


@dataclass
class ProbeResult:
    case: GuardStarbaseProbeCase
    classic_fleet_traces: list[FleetTrace]
    classic_base_traces: list[BaseTrace]
    rust_fleet_traces: list[FleetTrace] | None = None

    @property
    def rust_matches_classic(self) -> bool | None:
        if self.rust_fleet_traces is None:
            return None
        return trace_signature(self.rust_fleet_traces) == trace_signature(
            self.classic_fleet_traces
        )


DEFAULT_CASES = [
    GuardStarbaseProbeCase(
        name="guard-starbase-axial-a",
        start=(8, 8),
        target=(11, 8),
        turns=3,
    ),
    GuardStarbaseProbeCase(
        name="guard-starbase-axial-b",
        start=(11, 10),
        target=(14, 10),
        turns=3,
    ),
    GuardStarbaseProbeCase(
        name="guard-starbase-base-lost-after-arrival",
        start=(8, 8),
        target=(11, 8),
        turns=3,
        destroy_base_after_turn=2,
        compare_rust=False,
    ),
]


def repo_relative(path: Path) -> str:
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return str(path)


def ensure_nc_cli() -> Path:
    global NC_CLI_READY
    if NC_CLI_READY and NC_CLI_BIN.exists():
        return NC_CLI_BIN
    result = subprocess.run(
        ["cargo", "build", "-q", "-p", "nc-cli"],
        cwd=RUST_WORKSPACE,
        text=True,
        capture_output=True,
    )
    if result.returncode != 0:
        sys.stdout.write(result.stdout)
        sys.stderr.write(result.stderr)
        raise SystemExit(result.returncode)
    NC_CLI_READY = True
    return NC_CLI_BIN


def run_nc_cli(args: list[str]) -> subprocess.CompletedProcess[str]:
    binary = ensure_nc_cli()
    result = subprocess.run(
        [str(binary), *args],
        cwd=RUST_WORKSPACE,
        text=True,
        capture_output=True,
    )
    if result.returncode != 0:
        sys.stdout.write(result.stdout)
        sys.stderr.write(result.stderr)
        raise SystemExit(result.returncode)
    return result


def build_probe_dirs(case: GuardStarbaseProbeCase, work_root: Path) -> tuple[Path | None, Path]:
    case_root = work_root / case.name
    if case_root.exists():
        shutil.rmtree(case_root)
    case_root.mkdir(parents=True, exist_ok=True)

    rust_dir = case_root / "rust" if case.compare_rust else None
    classic_dir = case_root / "classic"

    if rust_dir is not None:
        run_nc_cli(
            [
                "scenario-init-replayable",
                str(DEFAULT_BASELINE),
                str(rust_dir),
                "move",
            ]
        )
        configure_probe_dir(rust_dir, case)

    run_nc_cli(
        [
            "scenario-init-replayable",
            str(DEFAULT_BASELINE),
            str(classic_dir),
            "move",
        ]
    )
    configure_probe_dir(classic_dir, case)
    run_nc_cli(["db-export", str(classic_dir), str(classic_dir)])
    ensure_engine(classic_dir)
    return rust_dir, classic_dir


def configure_probe_dir(target_dir: Path, case: GuardStarbaseProbeCase) -> None:
    run_nc_cli(
        [
            "guard-starbase-onebase",
            str(target_dir),
            str(case.target[0]),
            str(case.target[1]),
        ]
    )
    run_nc_cli(
        [
            "fleet-ships",
            str(target_dir),
            "1",
            "0",
            "0",
            "1",
            "0",
            "0",
            "0",
            "0",
        ]
    )
    run_nc_cli(
        [
            "fleet-location",
            str(target_dir),
            "1",
            str(case.start[0]),
            str(case.start[1]),
        ]
    )
    run_nc_cli(
        [
            "fleet-order",
            str(target_dir),
            "1",
            "3",
            "4",
            str(case.target[0]),
            str(case.target[1]),
            "1",
            "1",
        ]
    )


def parse_key_values(text: str) -> dict[str, str]:
    values: dict[str, str] = {}
    for line in text.splitlines():
        if "=" not in line:
            continue
        key, value = line.split("=", 1)
        values[key.strip()] = value.strip()
    return values


def parse_coords(raw: str) -> tuple[int, int]:
    left, right = raw.split(",", 1)
    return int(left), int(right)


def inspect_fleet(dir_path: Path, turn: int, live_dir: bool) -> FleetTrace:
    args = ["inspect-fleet-movement", str(dir_path), "1"]
    if live_dir:
        args.append("--live-dir")
    stdout = run_nc_cli(args).stdout
    values = parse_key_values(stdout)
    return FleetTrace(
        turn=turn,
        coords=parse_coords(values["coords"]),
        target=parse_coords(values["target"]),
        order=values["order"],
        order_code=int(values["order_code"]),
        current_speed=int(values["current_speed"]),
        mission_aux=values["mission_aux"],
        raw_0d=values["raw_0d"],
        raw_0e=values["raw_0e"],
        raw_0f=values["raw_0f"],
        raw_10=values["raw_10"],
        raw_11=values["raw_11"],
        raw_12=values["raw_12"],
        raw_19=values["raw_19"],
        raw_1a=values["raw_1a"],
        raw_1b=values["raw_1b"],
        raw_1c=values["raw_1c"],
        raw_1d=values["raw_1d"],
        raw_1e=values["raw_1e"],
    )


def inspect_base(dir_path: Path, turn: int) -> BaseTrace:
    player_bytes = (dir_path / "PLAYER.DAT").read_bytes()
    bases_bytes = (dir_path / "BASES.DAT").read_bytes()
    player_starbase_count = int.from_bytes(player_bytes[0x44:0x46], "little")
    base_count = len(bases_bytes) // BASE_RECORD_SIZE if bases_bytes else 0

    base_active_flag = None
    base_id = None
    base_coords = None
    if base_count > 0:
        base_active_flag = bases_bytes[0x02]
        base_id = bases_bytes[0x04]
        base_coords = (bases_bytes[0x0B], bases_bytes[0x0C])

    return BaseTrace(
        turn=turn,
        player_starbase_count=player_starbase_count,
        base_count=base_count,
        base_active_flag=base_active_flag,
        base_id=base_id,
        base_coords=base_coords,
    )


def patch_destroy_base(dir_path: Path) -> None:
    player_path = dir_path / "PLAYER.DAT"
    base_path = dir_path / "BASES.DAT"
    player = bytearray(player_path.read_bytes())
    player[0x44] = 0x00
    player[0x45] = 0x00
    player_path.write_bytes(player)
    if base_path.exists():
        base_path.write_bytes(bytes(len(base_path.read_bytes())))


def run_classic_turn(dir_path: Path) -> None:
    result = run_ecmaint(dir_path)
    if result.returncode != 0:
        sys.stdout.write(result.stdout)
        sys.stderr.write(result.stderr)
        raise SystemExit(result.returncode)


def trace_signature(traces: list[FleetTrace]) -> list[tuple[object, ...]]:
    return [
        (
            trace.coords,
            trace.order,
            trace.current_speed,
            trace.mission_aux,
            trace.raw_0d,
            trace.raw_0e,
            trace.raw_0f,
            trace.raw_10,
            trace.raw_11,
            trace.raw_12,
            trace.raw_19,
            trace.raw_1a,
            trace.raw_1b,
            trace.raw_1c,
            trace.raw_1d,
            trace.raw_1e,
        )
        for trace in traces
    ]


def trace_compact(trace: FleetTrace) -> str:
    return (
        f"{trace.coords[0]},{trace.coords[1]} "
        f"order={trace.order} spd={trace.current_speed} aux={trace.mission_aux} "
        f"0d..12={trace.raw_0d}/{trace.raw_0e}/{trace.raw_0f}/{trace.raw_10}/{trace.raw_11}/{trace.raw_12} "
        f"19..1e={trace.raw_19}/{trace.raw_1a}/{trace.raw_1b}/{trace.raw_1c}/{trace.raw_1d}/{trace.raw_1e}"
    )


def base_compact(trace: BaseTrace) -> str:
    return (
        f"player_starbases={trace.player_starbase_count} "
        f"base_count={trace.base_count} "
        f"base_active={trace.base_active_flag if trace.base_active_flag is not None else 'N/A'} "
        f"base_id={trace.base_id if trace.base_id is not None else 'N/A'} "
        f"base_coords={trace.base_coords if trace.base_coords is not None else 'N/A'}"
    )


def run_probe(case: GuardStarbaseProbeCase, work_root: Path) -> ProbeResult:
    rust_dir, classic_dir = build_probe_dirs(case, work_root)
    rust_traces = [inspect_fleet(rust_dir, 0, live_dir=False)] if rust_dir else None
    classic_traces = [inspect_fleet(classic_dir, 0, live_dir=True)]
    classic_base_traces = [inspect_base(classic_dir, 0)]

    for turn in range(1, case.turns + 1):
        if rust_dir is not None:
            run_nc_cli(["maint-rust", str(rust_dir), "1"])
            rust_traces.append(inspect_fleet(rust_dir, turn, live_dir=False))

        run_classic_turn(classic_dir)
        classic_traces.append(inspect_fleet(classic_dir, turn, live_dir=True))
        classic_base_traces.append(inspect_base(classic_dir, turn))

        if case.destroy_base_after_turn == turn:
            patch_destroy_base(classic_dir)

    return ProbeResult(
        case=case,
        classic_fleet_traces=classic_traces,
        classic_base_traces=classic_base_traces,
        rust_fleet_traces=rust_traces,
    )


def render_markdown(results: list[ProbeResult]) -> str:
    normal_results = [result for result in results if result.rust_fleet_traces is not None]
    base_loss = next(
        (result for result in results if result.case.destroy_base_after_turn is not None), None
    )
    arrival_payloads = {
        (
            trace.raw_0d,
            trace.raw_0e,
            trace.raw_0f,
            trace.raw_10,
            trace.raw_11,
            trace.raw_12,
        )
        for result in normal_results
        for trace in result.classic_fleet_traces
        if trace.coords == trace.target and trace.current_speed == 0
    }

    lines = [
        "# Guard Starbase Runtime Audit",
        "",
        "Focused `GuardStarbase` probes comparing Rust maintenance against classic `ECMAINT` and drilling into the remaining runtime-field mismatches.",
        "",
        "Current takeaways from this probe set:",
        "",
        "- Classic clears `mission_aux[0]` from `01` to `00` on the first maintenance pass for active `GuardStarbase` fleets, even while the mission remains armed.",
        "- During the transit year, `GuardStarbase` uses the normal in-transit motion bytes in `0x0d..0x12`; the earlier doc note that classic left that window zero during transit was incorrect.",
        "- In the controlled axial arrival cases below, classic converges on the same guarded-arrival `0x0d..0x12` payload after arrival: `{}`.".format(
            ", ".join("/".join(payload) for payload in sorted(arrival_payloads))
            if arrival_payloads
            else "N/A"
        ),
        "- If the guarded base is removed after arrival, classic abandons the mission on the following maintenance tick even though `mission_aux[0]` is already `00`; long-lived guard continuation is therefore keyed from the actual guarded base at the target, not from the original index byte.",
        "- After the Rust runtime-linkage fix and guarded-arrival payload mirror, the only remaining mismatch in these controlled cases is the transit-year `0x1a..0x1e` window: classic leaves it zero while Rust still stores exact in-transit position there for geometry/ETA continuity.",
        "",
        "| case | rust/classic match |",
        "| --- | --- |",
    ]

    for result in normal_results:
        lines.append(
            "| {name} | {match_status} |".format(
                name=result.case.name,
                match_status="yes" if result.rust_matches_classic else "no",
            )
        )

    for result in results:
        lines.extend(
            [
                "",
                f"## {result.case.name}",
                "",
                f"- start: `{result.case.start[0]},{result.case.start[1]}`",
                f"- target: `{result.case.target[0]},{result.case.target[1]}`",
                f"- compare Rust: `{'yes' if result.rust_fleet_traces is not None else 'no'}`",
            ]
        )
        if result.case.destroy_base_after_turn is not None:
            lines.append(
                f"- destroy guarded base after turn: `{result.case.destroy_base_after_turn}`"
            )
        if result.rust_fleet_traces is not None:
            lines.append(
                f"- turn-by-turn match: `{'yes' if result.rust_matches_classic else 'no'}`"
            )
            lines.extend(["", "| turn | Rust | Classic | Classic base state |", "| ---: | --- | --- | --- |"])
            for idx, classic_trace in enumerate(result.classic_fleet_traces):
                rust_trace = result.rust_fleet_traces[idx]
                base_trace = result.classic_base_traces[idx]
                lines.append(
                    f"| {classic_trace.turn} | `{trace_compact(rust_trace)}` | `{trace_compact(classic_trace)}` | `{base_compact(base_trace)}` |"
                )
        else:
            lines.extend(["", "| turn | Classic | Classic base state |", "| ---: | --- | --- |"])
            for classic_trace, base_trace in zip(
                result.classic_fleet_traces, result.classic_base_traces
            ):
                lines.append(
                    f"| {classic_trace.turn} | `{trace_compact(classic_trace)}` | `{base_compact(base_trace)}` |"
                )

    if base_loss is not None:
        final_trace = base_loss.classic_fleet_traces[-1]
        lines.extend(
            [
                "",
                "## Practical Rust Consequence",
                "",
                "The runtime guard-starbase model should treat `mission_aux[0]` as an input/setup byte, not as the durable post-maint linkage key.",
                "Rust should therefore:",
                "",
                "- normalize the runtime `GuardStarbase` aux index byte to `00` during maintenance",
                "- keep the mission armed while a friendly active base still exists at the guarded target coords",
                "- abandon to `Hold` when that guarded base disappears, even if the aux index is already zero",
                "- mirror the guarded-arrival `0x0d..0x12` payload as a compatibility shape even though its low-level meaning is still not decoded",
                "- leave the transit-year `0x1a..0x1e` mismatch for the broader motion-scratch recovery pass",
                "",
                "The controlled base-loss probe ended with classic trace:",
                "",
                f"`{trace_compact(final_trace)}`",
            ]
        )

    return "\n".join(lines) + "\n"


def print_stdout_summary(results: list[ProbeResult]) -> None:
    print("Guard Starbase runtime audit")
    for result in results:
        if result.rust_matches_classic is None:
            print(f"  {result.case.name}: classic-only")
        else:
            print(
                f"  {result.case.name}: turn_match={'yes' if result.rust_matches_classic else 'no'}"
            )


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--work-root",
        default=str(DEFAULT_WORK_ROOT),
        help="temp root for generated audit scenarios",
    )
    parser.add_argument(
        "--output",
        default=str(DEFAULT_OUTPUT),
        help="markdown report path",
    )
    parser.add_argument(
        "--case",
        action="append",
        choices=[case.name for case in DEFAULT_CASES],
        help="optional case filter (can be repeated)",
    )
    return parser


def main() -> None:
    args = build_parser().parse_args()
    work_root = Path(args.work_root)
    output_path = Path(args.output)
    cases = DEFAULT_CASES
    if args.case:
        wanted = set(args.case)
        cases = [case for case in DEFAULT_CASES if case.name in wanted]

    results = [run_probe(case, work_root) for case in cases]
    output_path.write_text(render_markdown(results), encoding="utf-8")
    print_stdout_summary(results)
    print(f"Wrote {repo_relative(output_path)}")


if __name__ == "__main__":
    main()
