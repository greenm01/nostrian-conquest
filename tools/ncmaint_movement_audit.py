#!/usr/bin/env python3
"""Audit Rust movement/ETA against classic ECMAINT with controlled probes."""

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
DEFAULT_WORK_ROOT = Path("/tmp/ecmaint-movement-audit")
DEFAULT_OUTPUT = ROOT / "docs" / "dev" / "movement-oracle-audit.md"


@dataclass(frozen=True)
class MovementProbeCase:
    name: str
    speed: int
    start: tuple[int, int]
    target: tuple[int, int]
    turns: int


@dataclass
class FleetTrace:
    turn: int
    coords: tuple[int, int]
    target: tuple[int, int]
    order: str
    current_speed: int
    max_speed: int
    route_steps: int
    eta_status: str
    eta_years: int | None
    arrival_year: int | None
    raw_0d: str
    raw_0f: str
    route: str


@dataclass
class ProbeResult:
    case: MovementProbeCase
    rust_traces: list[FleetTrace]
    classic_traces: list[FleetTrace]

    @property
    def rust_initial_eta_years(self) -> int | None:
        return self.rust_traces[0].eta_years if self.rust_traces else None

    @property
    def rust_arrival_turn(self) -> int | None:
        return arrival_turn(self.rust_traces, self.case.target)

    @property
    def classic_arrival_turn(self) -> int | None:
        return arrival_turn(self.classic_traces, self.case.target)

    @property
    def rust_matches_classic(self) -> bool:
        return [
            (trace.coords, trace.order, trace.current_speed)
            for trace in self.rust_traces
        ] == [
            (trace.coords, trace.order, trace.current_speed)
            for trace in self.classic_traces
        ]

    @property
    def rust_eta_matches_classic_arrival(self) -> bool:
        return self.rust_initial_eta_years == self.classic_arrival_turn


DEFAULT_CASES = [
    MovementProbeCase("speed3-horizontal", 3, (10, 10), (16, 10), 3),
    MovementProbeCase("speed3-diagonal", 3, (10, 10), (16, 16), 3),
    MovementProbeCase("speed6-diagonal", 6, (10, 10), (16, 16), 2),
    MovementProbeCase("speed1-diagonal", 1, (10, 10), (13, 13), 6),
    MovementProbeCase("speed3-shallow", 3, (10, 10), (16, 12), 3),
    MovementProbeCase("speed3-steep", 3, (10, 10), (12, 16), 3),
]


def repo_relative(path: Path) -> str:
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return str(path)


def ensure_nc_cli() -> Path:
    if NC_CLI_BIN.exists():
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


def build_probe_dirs(
    case: MovementProbeCase, work_root: Path
) -> tuple[Path, Path]:
    case_root = work_root / case.name
    if case_root.exists():
        shutil.rmtree(case_root)
    case_root.mkdir(parents=True, exist_ok=True)

    rust_dir = case_root / "rust"
    classic_dir = case_root / "classic"
    run_nc_cli(
        [
            "scenario-init-replayable",
            str(DEFAULT_BASELINE),
            str(rust_dir),
            "move",
        ]
    )
    run_nc_cli(
        [
            "scenario-init-replayable",
            str(DEFAULT_BASELINE),
            str(classic_dir),
            "move",
        ]
    )
    configure_probe_dir(rust_dir, case)
    configure_probe_dir(classic_dir, case)
    run_nc_cli(["db-export", str(classic_dir), str(classic_dir)])
    ensure_engine(classic_dir)
    return rust_dir, classic_dir


def configure_probe_dir(target_dir: Path, case: MovementProbeCase) -> None:
    if case.speed > 3:
        ship_args = ["1", "0", "0", "0", "0", "0", "0"]
    else:
        ship_args = ["0", "0", "1", "0", "0", "0", "0"]
    run_nc_cli(["fleet-ships", str(target_dir), "1", *ship_args])
    run_nc_cli(
        [
            "fleet-location",
            str(target_dir),
            "1",
            str(case.start[0]),
            str(case.start[1]),
        ]
    )
    # Clear the mission aux bytes as ECGAME does for a move order; the
    # fixture's stale join-host byte makes classic finish a year late.
    run_nc_cli(
        [
            "fleet-order",
            str(target_dir),
            "1",
            str(case.speed),
            "1",
            str(case.target[0]),
            str(case.target[1]),
            "0",
            "0",
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
    eta_years = (
        int(values["eta_years"])
        if "eta_years" in values and values["eta_status"] == "years"
        else 0
        if values.get("eta_status") == "arrived"
        else None
    )
    arrival_year = int(values["arrival_year"]) if "arrival_year" in values else None
    return FleetTrace(
        turn=turn,
        coords=parse_coords(values["coords"]),
        target=parse_coords(values["target"]),
        order=values["order"],
        current_speed=int(values["current_speed"]),
        max_speed=int(values["max_speed"]),
        route_steps=int(values["route_steps"]),
        eta_status=values["eta_status"],
        eta_years=eta_years,
        arrival_year=arrival_year,
        raw_0d=values["raw_0d"],
        raw_0f=values["raw_0f"],
        route=values.get("route", ""),
    )


def run_classic_turn(dir_path: Path) -> None:
    result = run_ecmaint(dir_path)
    if result.returncode != 0:
        sys.stdout.write(result.stdout)
        sys.stderr.write(result.stderr)
        raise SystemExit(result.returncode)


def run_probe(case: MovementProbeCase, work_root: Path) -> ProbeResult:
    rust_dir, classic_dir = build_probe_dirs(case, work_root)

    rust_traces = [inspect_fleet(rust_dir, 0, live_dir=False)]
    classic_traces = [inspect_fleet(classic_dir, 0, live_dir=True)]

    for turn in range(1, case.turns + 1):
        run_nc_cli(["maint-rust", str(rust_dir), "1"])
        rust_traces.append(inspect_fleet(rust_dir, turn, live_dir=False))

        run_classic_turn(classic_dir)
        classic_traces.append(inspect_fleet(classic_dir, turn, live_dir=True))

    return ProbeResult(case=case, rust_traces=rust_traces, classic_traces=classic_traces)


def arrival_turn(traces: list[FleetTrace], target: tuple[int, int]) -> int | None:
    for trace in traces:
        if trace.coords == target:
            return trace.turn
    return None


def trace_compact(trace: FleetTrace) -> str:
    eta = (
        f"{trace.eta_years}"
        if trace.eta_status == "years" and trace.eta_years is not None
        else trace.eta_status
    )
    return (
        f"{trace.coords[0]},{trace.coords[1]} "
        f"order={trace.order} spd={trace.current_speed} eta={eta}"
    )


def render_markdown(results: list[ProbeResult]) -> str:
    lines = [
        "# ECMAINT Movement Audit",
        "",
        "Controlled `MoveOnly` probes comparing Rust maintenance against classic `ECMAINT`.",
        "",
        "Current takeaways from this probe set:",
        "",
        "- Annual movement is confirmed to happen before the weekly `1..52` scheduling loop; the probe directories only advanced once per maintenance turn.",
        "- Horizontal `speed=3` travel matches classic turn-by-turn until arrival, so the current Rust `speed * 8 / 9` annual distance budget is at least directionally correct on axial paths.",
        "- Diagonal and sloped routes do **not** match classic intermediate positions. Classic advances more conservatively than the current Rust straight-line rounding path.",
        "- The clearest ETA miss is the `speed=1` diagonal case: Rust predicts and arrives in `4` years, while classic arrives in `5`.",
        "- Classic clears `MoveOnly` to `hold` with `speed=0` on arrival in the horizontal, `speed=6` diagonal, shallow, and steep probes. Rust currently preserves `MoveOnly` on arrival.",
        "- This also makes early contact more plausible than it first looked: the current 4-player mapgen target is an `18x18` map with roughly `5` sectors of minimum homeworld spacing, while classic `speed=3` probes still reach `6` sectors of axial/diagonal separation in `3` maintenance turns.",
        "",
        "| case | speed | start | target | rust ETA | rust arrival | classic arrival | trace match |",
        "| --- | ---: | --- | --- | ---: | ---: | ---: | --- |",
    ]
    for result in results:
        lines.append(
            "| {name} | {speed} | `{sx},{sy}` | `{tx},{ty}` | {eta} | {rust_arrival} | {classic_arrival} | {match_status} |".format(
                name=result.case.name,
                speed=result.case.speed,
                sx=result.case.start[0],
                sy=result.case.start[1],
                tx=result.case.target[0],
                ty=result.case.target[1],
                eta=result.rust_initial_eta_years
                if result.rust_initial_eta_years is not None
                else "N/A",
                rust_arrival=result.rust_arrival_turn
                if result.rust_arrival_turn is not None
                else "N/A",
                classic_arrival=result.classic_arrival_turn
                if result.classic_arrival_turn is not None
                else "N/A",
                match_status="yes" if result.rust_matches_classic else "no",
            )
        )

    for result in results:
        lines.extend(
            [
                "",
                f"## {result.case.name}",
                "",
                f"- speed: `{result.case.speed}`",
                f"- start: `{result.case.start[0]},{result.case.start[1]}`",
                f"- target: `{result.case.target[0]},{result.case.target[1]}`",
                f"- initial Rust ETA: `{result.rust_initial_eta_years}`",
                f"- Rust arrival turn: `{result.rust_arrival_turn}`",
                f"- Classic arrival turn: `{result.classic_arrival_turn}`",
                f"- turn-by-turn match: `{'yes' if result.rust_matches_classic else 'no'}`",
                "",
                "| turn | Rust | Classic |",
                "| ---: | --- | --- |",
            ]
        )
        for rust_trace, classic_trace in zip(result.rust_traces, result.classic_traces):
            lines.append(
                f"| {rust_trace.turn} | `{trace_compact(rust_trace)}` | `{trace_compact(classic_trace)}` |"
            )

    return "\n".join(lines) + "\n"


def print_stdout_summary(results: list[ProbeResult]) -> None:
    print("ECMAINT movement audit")
    for result in results:
        print(
            "  {name}: speed={speed} start={start} target={target} rust_eta={eta} rust_arrival={rust_arrival} classic_arrival={classic_arrival} trace_match={trace_match}".format(
                name=result.case.name,
                speed=result.case.speed,
                start=result.case.start,
                target=result.case.target,
                eta=result.rust_initial_eta_years,
                rust_arrival=result.rust_arrival_turn,
                classic_arrival=result.classic_arrival_turn,
                trace_match=result.rust_matches_classic,
            )
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
    parser.add_argument(
        "--keep-workdirs",
        action="store_true",
        help="keep generated /tmp probe directories instead of deleting them first",
    )
    return parser


def main() -> int:
    args = build_parser().parse_args()
    work_root = Path(args.work_root).resolve()
    if work_root.exists() and not args.keep_workdirs:
        shutil.rmtree(work_root)
    work_root.mkdir(parents=True, exist_ok=True)

    selected = (
        [case for case in DEFAULT_CASES if case.name in set(args.case)]
        if args.case
        else DEFAULT_CASES
    )

    results = [run_probe(case, work_root) for case in selected]
    print_stdout_summary(results)

    output_path = Path(args.output).resolve()
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(render_markdown(results), encoding="utf-8")
    print(f"  report={repo_relative(output_path)}")
    print(f"  work_root={work_root}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
