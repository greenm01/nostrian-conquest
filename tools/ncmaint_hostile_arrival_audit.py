#!/usr/bin/env python3
"""Audit delayed hostile-world arrivals against classic ECMAINT."""

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
DEFAULT_WORK_ROOT = Path("/tmp/ecmaint-hostile-arrival-audit")
DEFAULT_OUTPUT = ROOT / "docs" / "dev" / "hostile-arrival-oracle-audit.md"
NC_CLI_READY = False


@dataclass(frozen=True)
class HostileArrivalProbeCase:
    name: str
    scenario: str
    fleet_record: int
    turns: int
    order_code: int | None = None
    speed: int | None = None
    target: tuple[int, int] | None = None
    ships: tuple[int, int, int, int, int, int, int] | None = None


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
    raw_19: str
    raw_1a: str
    raw_1b: str
    raw_1c: str
    raw_1d: str
    raw_1e: str


@dataclass
class ProbeResult:
    case: HostileArrivalProbeCase
    rust_traces: list[FleetTrace]
    classic_traces: list[FleetTrace]

    @property
    def rust_arrival_turn(self) -> int | None:
        return arrival_turn(self.rust_traces)

    @property
    def classic_arrival_turn(self) -> int | None:
        return arrival_turn(self.classic_traces)

    @property
    def rust_matches_classic(self) -> bool:
        return trace_signature(self.rust_traces) == trace_signature(self.classic_traces)

    @property
    def rust_arrival_trace(self) -> FleetTrace | None:
        return trace_at_turn(self.rust_traces, self.rust_arrival_turn)

    @property
    def classic_arrival_trace(self) -> FleetTrace | None:
        return trace_at_turn(self.classic_traces, self.classic_arrival_turn)

    @property
    def rust_resolution_trace(self) -> FleetTrace | None:
        return trace_at_turn(
            self.rust_traces,
            None if self.rust_arrival_turn is None else self.rust_arrival_turn + 1,
        )

    @property
    def classic_resolution_trace(self) -> FleetTrace | None:
        return trace_at_turn(
            self.classic_traces,
            None if self.classic_arrival_turn is None else self.classic_arrival_turn + 1,
        )


DEFAULT_CASES = [
    HostileArrivalProbeCase(
        name="bombard-delayed",
        scenario="bombard",
        fleet_record=3,
        turns=2,
    ),
    HostileArrivalProbeCase(
        name="invade-delayed",
        scenario="invade",
        fleet_record=3,
        turns=2,
        ships=(0, 0, 1, 0, 10, 10, 0),
    ),
    HostileArrivalProbeCase(
        name="blitz-delayed-strong",
        scenario="invade",
        fleet_record=3,
        turns=2,
        order_code=8,
        speed=3,
        target=(15, 13),
        ships=(0, 100, 50, 50, 50, 50, 0),
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


def build_probe_dirs(case: HostileArrivalProbeCase, work_root: Path) -> tuple[Path, Path]:
    case_root = work_root / case.name
    if case_root.exists():
        shutil.rmtree(case_root)
    case_root.mkdir(parents=True, exist_ok=True)

    rust_dir = case_root / "rust"
    classic_dir = case_root / "classic"
    for target_dir in [rust_dir, classic_dir]:
        run_nc_cli(
            [
                "scenario-init-replayable",
                str(DEFAULT_BASELINE),
                str(target_dir),
                case.scenario,
            ]
        )
        configure_probe_dir(target_dir, case)

    run_nc_cli(["db-export", str(classic_dir), str(classic_dir)])
    ensure_engine(classic_dir)
    return rust_dir, classic_dir


def configure_probe_dir(target_dir: Path, case: HostileArrivalProbeCase) -> None:
    if case.ships is not None:
        run_nc_cli(
            [
                "fleet-ships",
                str(target_dir),
                str(case.fleet_record),
                *[str(value) for value in case.ships],
            ]
        )

    if case.order_code is not None:
        if case.speed is None or case.target is None:
            raise ValueError(f"{case.name} needs speed and target for fleet-order override")
        run_nc_cli(
            [
                "fleet-order",
                str(target_dir),
                str(case.fleet_record),
                str(case.speed),
                str(case.order_code),
                str(case.target[0]),
                str(case.target[1]),
                "1",
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


def inspect_fleet(dir_path: Path, fleet_record: int, turn: int, live_dir: bool) -> FleetTrace:
    args = ["inspect-fleet-movement", str(dir_path), str(fleet_record)]
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
        raw_19=values["raw_19"],
        raw_1a=values["raw_1a"],
        raw_1b=values["raw_1b"],
        raw_1c=values["raw_1c"],
        raw_1d=values["raw_1d"],
        raw_1e=values["raw_1e"],
    )


def run_classic_turn(dir_path: Path) -> None:
    result = run_ecmaint(dir_path)
    if result.returncode != 0:
        sys.stdout.write(result.stdout)
        sys.stderr.write(result.stderr)
        raise SystemExit(result.returncode)


def run_probe(case: HostileArrivalProbeCase, work_root: Path) -> ProbeResult:
    rust_dir, classic_dir = build_probe_dirs(case, work_root)

    rust_traces = [inspect_fleet(rust_dir, case.fleet_record, 0, live_dir=False)]
    classic_traces = [inspect_fleet(classic_dir, case.fleet_record, 0, live_dir=True)]

    for turn in range(1, case.turns + 1):
        run_nc_cli(["maint-rust", str(rust_dir), "1"])
        rust_traces.append(inspect_fleet(rust_dir, case.fleet_record, turn, live_dir=False))

        run_classic_turn(classic_dir)
        classic_traces.append(
            inspect_fleet(classic_dir, case.fleet_record, turn, live_dir=True)
        )

    return ProbeResult(case=case, rust_traces=rust_traces, classic_traces=classic_traces)


def arrival_turn(traces: list[FleetTrace]) -> int | None:
    target = traces[0].target if traces else None
    if target is None:
        return None
    for trace in traces:
        if trace.coords == target and trace.order_code == traces[0].order_code:
            return trace.turn
    return None


def trace_at_turn(traces: list[FleetTrace], turn: int | None) -> FleetTrace | None:
    if turn is None:
        return None
    for trace in traces:
        if trace.turn == turn:
            return trace
    return None


def trace_signature(traces: list[FleetTrace]) -> list[tuple[object, ...]]:
    return [
        (
            trace.coords,
            trace.order,
            trace.current_speed,
            trace.raw_19,
            trace.raw_1a,
            trace.raw_1b,
            trace.raw_1c,
            trace.raw_1d,
            trace.raw_1e,
        )
        for trace in traces
    ]


def ready_bytes(trace: FleetTrace | None) -> str:
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
        f"order={trace.order} spd={trace.current_speed} eta={eta} "
        f"{ready_bytes(trace)}"
    )


def render_markdown(results: list[ProbeResult]) -> str:
    lines = [
        "# ECMAINT Hostile Arrival Audit",
        "",
        "Controlled delayed hostile-world probes comparing Rust maintenance against classic `ECMAINT`.",
        "",
        "Current takeaways from this probe set:",
        "",
        "- `BombardWorld`, `InvadeWorld`, and `BlitzWorld` keep their order and travel speed while the fleet descends to orbit.",
        "- A hostile fleet strikes on the tick its depth reaches `1.0`. If the attack fails, classic keeps the state from the tick before, so the depth stays just under `1.0` (for a one-sector straight path, `19=80 1a=b9 1b=ff 1c=ff 1d=ff 1e=7f`), and the fleet strikes again the next year.",
        "- The strike year and the retry bytes match classic. What the battle does to the order afterward depends on the planetary battle, which is random in classic.",
        "",
        "Scope of this probe set:",
        "",
        "- one-sector delayed hostile arrivals",
        "- `BombardWorld`, `InvadeWorld`, and `BlitzWorld`",
        "- one follow-up ready-resolution tick after arrival",
        "- `BlitzWorld` uses a strong combined-arms fleet so the second tick exercises a valid assault path",
        "",
        "| case | rust arrival | classic arrival | arrival byte match | turn-by-turn match |",
        "| --- | ---: | ---: | --- | --- |",
    ]

    for result in results:
        lines.append(
            "| {name} | {rust_turn} | {classic_turn} | {arrival_match} | {match_status} |".format(
                name=result.case.name,
                rust_turn=result.rust_arrival_turn
                if result.rust_arrival_turn is not None
                else "N/A",
                classic_turn=result.classic_arrival_turn
                if result.classic_arrival_turn is not None
                else "N/A",
                arrival_match="yes"
                if ready_bytes(result.rust_arrival_trace)
                == ready_bytes(result.classic_arrival_trace)
                else "no",
                match_status="yes" if result.rust_matches_classic else "no",
            )
        )

    for result in results:
        lines.extend(
            [
                "",
                f"## {result.case.name}",
                "",
                f"- source scenario: `{result.case.scenario}`",
                f"- fleet record: `{result.case.fleet_record}`",
                f"- Rust arrival turn: `{result.rust_arrival_turn}`",
                f"- Classic arrival turn: `{result.classic_arrival_turn}`",
                f"- Rust arrival bytes: `{ready_bytes(result.rust_arrival_trace)}`",
                f"- Classic arrival bytes: `{ready_bytes(result.classic_arrival_trace)}`",
                f"- Rust resolution trace: `{trace_compact(result.rust_resolution_trace) if result.rust_resolution_trace else 'N/A'}`",
                f"- Classic resolution trace: `{trace_compact(result.classic_resolution_trace) if result.classic_resolution_trace else 'N/A'}`",
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
    print("ECMAINT hostile arrival audit")
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
