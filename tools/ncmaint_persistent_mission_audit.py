#!/usr/bin/env python3
"""Audit persistent mission arrivals against classic ECMAINT."""

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
DEFAULT_WORK_ROOT = Path("/tmp/ecmaint-persistent-mission-audit")
DEFAULT_OUTPUT = ROOT / "docs" / "dev" / "persistent-mission-oracle-audit.md"
NC_CLI_READY = False


@dataclass(frozen=True)
class PersistentMissionProbeCase:
    name: str
    order_code: int
    speed: int
    start: tuple[int, int]
    target: tuple[int, int]
    turns: int
    ships: tuple[int, int, int, int, int, int, int]
    aux: tuple[int, int] | None = None
    setup: str = "generic"


@dataclass
class FleetTrace:
    turn: int
    coords: tuple[int, int]
    target: tuple[int, int]
    order: str
    order_code: int
    current_speed: int
    max_speed: int
    mission_aux: str
    route_steps: int
    eta_status: str
    eta_years: int | None
    arrival_year: int | None
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
    route: str


@dataclass
class ProbeResult:
    case: PersistentMissionProbeCase
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
            (
                trace.coords,
                trace.order,
                trace.current_speed,
                trace.mission_aux,
                trace.raw_19,
                trace.raw_1a,
                trace.raw_1b,
                trace.raw_1c,
                trace.raw_1d,
                trace.raw_1e,
            )
            for trace in self.rust_traces
        ] == [
            (
                trace.coords,
                trace.order,
                trace.current_speed,
                trace.mission_aux,
                trace.raw_19,
                trace.raw_1a,
                trace.raw_1b,
                trace.raw_1c,
                trace.raw_1d,
                trace.raw_1e,
            )
            for trace in self.classic_traces
        ]

    @property
    def rust_arrival_trace(self) -> FleetTrace | None:
        return trace_at_turn(self.rust_traces, self.rust_arrival_turn)

    @property
    def classic_arrival_trace(self) -> FleetTrace | None:
        return trace_at_turn(self.classic_traces, self.classic_arrival_turn)

    @property
    def rust_post_arrival_trace(self) -> FleetTrace | None:
        return trace_at_turn(
            self.rust_traces,
            None if self.rust_arrival_turn is None else self.rust_arrival_turn + 1,
        )

    @property
    def classic_post_arrival_trace(self) -> FleetTrace | None:
        return trace_at_turn(
            self.classic_traces,
            None if self.classic_arrival_turn is None else self.classic_arrival_turn + 1,
        )


DEFAULT_CASES = [
    PersistentMissionProbeCase(
        name="patrol-speed3-axial",
        order_code=3,
        speed=3,
        start=(8, 10),
        target=(11, 10),
        turns=3,
        ships=(0, 0, 1, 0, 0, 0, 0),
    ),
    PersistentMissionProbeCase(
        name="guard-starbase-speed3-axial",
        order_code=4,
        speed=3,
        start=(8, 8),
        target=(11, 8),
        turns=3,
        ships=(0, 0, 1, 0, 0, 0, 0),
        aux=(1, 1),
        setup="guard_starbase",
    ),
    PersistentMissionProbeCase(
        name="guard-blockade-speed3-axial",
        order_code=5,
        speed=3,
        start=(8, 8),
        target=(11, 8),
        turns=3,
        ships=(0, 0, 1, 0, 0, 0, 0),
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


def build_probe_dirs(
    case: PersistentMissionProbeCase, work_root: Path
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


def configure_probe_dir(target_dir: Path, case: PersistentMissionProbeCase) -> None:
    if case.setup == "guard_starbase":
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
            *[str(value) for value in case.ships],
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

    order_args = [
        "fleet-order",
        str(target_dir),
        "1",
        str(case.speed),
        str(case.order_code),
        str(case.target[0]),
        str(case.target[1]),
    ]
    if case.aux is not None:
        order_args.extend([str(case.aux[0]), str(case.aux[1])])
    run_nc_cli(order_args)


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
        order_code=int(values["order_code"]),
        current_speed=int(values["current_speed"]),
        max_speed=int(values["max_speed"]),
        mission_aux=values["mission_aux"],
        route_steps=int(values["route_steps"]),
        eta_status=values["eta_status"],
        eta_years=eta_years,
        arrival_year=arrival_year,
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
        route=values.get("route", ""),
    )


def run_classic_turn(dir_path: Path) -> None:
    result = run_ecmaint(dir_path)
    if result.returncode != 0:
        sys.stdout.write(result.stdout)
        sys.stderr.write(result.stderr)
        raise SystemExit(result.returncode)


def run_probe(case: PersistentMissionProbeCase, work_root: Path) -> ProbeResult:
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


def trace_at_turn(traces: list[FleetTrace], turn: int | None) -> FleetTrace | None:
    if turn is None:
        return None
    for trace in traces:
        if trace.turn == turn:
            return trace
    return None


def arrival_bytes(trace: FleetTrace | None) -> str:
    if trace is None:
        return "N/A"
    return (
        f"19={trace.raw_19} 1a={trace.raw_1a} 1b={trace.raw_1b} "
        f"1c={trace.raw_1c} 1d={trace.raw_1d} 1e={trace.raw_1e}"
    )


def trace_compact(trace: FleetTrace) -> str:
    eta = (
        f"{trace.eta_years}"
        if trace.eta_status == "years" and trace.eta_years is not None
        else trace.eta_status
    )
    return (
        f"{trace.coords[0]},{trace.coords[1]} "
        f"order={trace.order} spd={trace.current_speed} aux={trace.mission_aux} "
        f"eta={eta} {arrival_bytes(trace)}"
    )


def render_markdown(results: list[ProbeResult]) -> str:
    lines = [
        "# ECMAINT Persistent Mission Audit",
        "",
        "Controlled standing-mission probes comparing Rust maintenance against classic `ECMAINT`.",
        "",
        "Current takeaways from this probe set:",
        "",
        "- All three standing missions keep their order on arrival, but classic stops the fleet: `current_speed` becomes `0` instead of staying at the travel speed.",
        "- `PatrolSector` and `GuardBlockadeWorld` both converge on the same classic post-arrival shape: order preserved, speed `0`, and tuple-c reset to `19=81 1a..1e=00`.",
        "- `GuardStarbase` also stops on arrival and clears tuple-c, but classic still diverges in two unresolved ways: it flips `mission_aux[0]` from `01` to `00`, and it leaves a different nonzero `0x0d..0x12` state than the other standing missions.",
        "- After the Rust standing-arrival fix, the remaining mismatches in this audit are scratch-byte details rather than visible arrival semantics.",
        "",
        "Scope of this probe set:",
        "",
        "- `PatrolSector`",
        "- `GuardStarbase`",
        "- `GuardBlockadeWorld`",
        "- controlled axial `speed=3` arrivals with one post-arrival maintenance tick",
        "",
        "The goal is not full mission-combat semantics. It is to pin down:",
        "",
        "- whether classic preserves the standing order after arrival",
        "- whether classic preserves the fleet speed after arrival",
        "- what classic writes into the `0x19..0x1e` arrival-state scratch window",
        "- whether the next maintenance tick still treats the fleet as armed on that order",
        "",
        "| case | rust arrival | classic arrival | arrival byte match | turn-by-turn match |",
        "| --- | ---: | ---: | --- | --- |",
    ]

    for result in results:
        rust_arrival = arrival_bytes(result.rust_arrival_trace)
        classic_arrival = arrival_bytes(result.classic_arrival_trace)
        lines.append(
            "| {name} | {rust_turn} | {classic_turn} | {arrival_match} | {match_status} |".format(
                name=result.case.name,
                rust_turn=result.rust_arrival_turn
                if result.rust_arrival_turn is not None
                else "N/A",
                classic_turn=result.classic_arrival_turn
                if result.classic_arrival_turn is not None
                else "N/A",
                arrival_match="yes" if rust_arrival == classic_arrival else "no",
                match_status="yes" if result.rust_matches_classic else "no",
            )
        )

    for result in results:
        lines.extend(
            [
                "",
                f"## {result.case.name}",
                "",
                f"- order code: `{result.case.order_code}`",
                f"- speed: `{result.case.speed}`",
                f"- start: `{result.case.start[0]},{result.case.start[1]}`",
                f"- target: `{result.case.target[0]},{result.case.target[1]}`",
                f"- initial Rust ETA: `{result.rust_initial_eta_years}`",
                f"- Rust arrival turn: `{result.rust_arrival_turn}`",
                f"- Classic arrival turn: `{result.classic_arrival_turn}`",
                f"- Rust arrival bytes: `{arrival_bytes(result.rust_arrival_trace)}`",
                f"- Classic arrival bytes: `{arrival_bytes(result.classic_arrival_trace)}`",
                f"- Rust post-arrival trace: `{trace_compact(result.rust_post_arrival_trace) if result.rust_post_arrival_trace else 'N/A'}`",
                f"- Classic post-arrival trace: `{trace_compact(result.classic_post_arrival_trace) if result.classic_post_arrival_trace else 'N/A'}`",
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
    print("ECMAINT persistent mission audit")
    for result in results:
        print(
            "  {name}: rust_arrival={rust_arrival} classic_arrival={classic_arrival} turn_match={trace_match}".format(
                name=result.case.name,
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
