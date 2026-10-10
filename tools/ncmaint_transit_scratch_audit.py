#!/usr/bin/env python3
"""Audit in-transit movement scratch bytes against classic ECMAINT."""

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
DEFAULT_WORK_ROOT = Path("/tmp/ecmaint-transit-scratch-audit")
DEFAULT_OUTPUT = ROOT / "docs" / "dev" / "transit-scratch-oracle-audit.md"
NC_CLI_READY = False


@dataclass(frozen=True)
class TransitScratchProbeCase:
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
    current_speed: int
    mission_aux: str
    eta_status: str
    eta_years: int | None
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

    @property
    def raw_0d_to_12(self) -> str:
        return "/".join(
            [
                self.raw_0d,
                self.raw_0e,
                self.raw_0f,
                self.raw_10,
                self.raw_11,
                self.raw_12,
            ]
        )

    @property
    def raw_19_to_1e(self) -> str:
        return "/".join(
            [
                self.raw_19,
                self.raw_1a,
                self.raw_1b,
                self.raw_1c,
                self.raw_1d,
                self.raw_1e,
            ]
        )

    @property
    def is_transit_turn(self) -> bool:
        return self.turn > 0 and self.current_speed > 0 and self.order != "hold"


@dataclass
class ProbeResult:
    case: TransitScratchProbeCase
    rust_traces: list[FleetTrace]
    classic_traces: list[FleetTrace]

    @property
    def transit_turns(self) -> list[int]:
        return [
            trace.turn for trace in self.classic_traces if trace.is_transit_turn
        ]

    @property
    def classic_transit_19_to_1e_all_zero(self) -> bool:
        return all(
            trace.raw_19_to_1e == "00/00/00/00/00/00"
            for trace in self.classic_traces
            if trace.is_transit_turn
        )

    @property
    def transit_byte_match(self) -> bool:
        transit_turns = set(self.transit_turns)
        return [
            (
                trace.turn,
                trace.raw_0d_to_12,
                trace.raw_19_to_1e,
                trace.coords,
                trace.order,
                trace.current_speed,
            )
            for trace in self.rust_traces
            if trace.turn in transit_turns
        ] == [
            (
                trace.turn,
                trace.raw_0d_to_12,
                trace.raw_19_to_1e,
                trace.coords,
                trace.order,
                trace.current_speed,
            )
            for trace in self.classic_traces
            if trace.turn in transit_turns
        ]

    @property
    def classic_orders_match_rust(self) -> bool:
        return [
            (trace.turn, trace.coords, trace.order, trace.current_speed)
            for trace in self.rust_traces
        ] == [
            (trace.turn, trace.coords, trace.order, trace.current_speed)
            for trace in self.classic_traces
        ]


DEFAULT_CASES = [
    TransitScratchProbeCase(
        name="move-only-speed3-horizontal",
        order_code=1,
        speed=3,
        start=(10, 10),
        target=(16, 10),
        turns=3,
        ships=(0, 0, 1, 0, 0, 0, 0),
        aux=(0, 0),
    ),
    TransitScratchProbeCase(
        name="move-only-speed3-diagonal",
        order_code=1,
        speed=3,
        start=(10, 10),
        target=(16, 16),
        turns=4,
        ships=(0, 0, 1, 0, 0, 0, 0),
        aux=(0, 0),
    ),
    TransitScratchProbeCase(
        name="move-only-speed1-diagonal",
        order_code=1,
        speed=1,
        start=(10, 10),
        target=(13, 13),
        turns=6,
        ships=(0, 0, 1, 0, 0, 0, 0),
        aux=(0, 0),
    ),
    TransitScratchProbeCase(
        name="move-only-speed3-shallow",
        order_code=1,
        speed=3,
        start=(10, 10),
        target=(16, 12),
        turns=3,
        ships=(0, 0, 1, 0, 0, 0, 0),
        aux=(0, 0),
    ),
    TransitScratchProbeCase(
        name="patrol-speed3-axial",
        order_code=3,
        speed=3,
        start=(8, 10),
        target=(11, 10),
        turns=3,
        ships=(0, 0, 1, 0, 0, 0, 0),
        aux=(0, 0),
    ),
    TransitScratchProbeCase(
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
    TransitScratchProbeCase(
        name="guard-blockade-speed3-axial",
        order_code=5,
        speed=3,
        start=(8, 8),
        target=(11, 8),
        turns=3,
        ships=(0, 0, 1, 0, 0, 0, 0),
        aux=(1, 0),
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
    case: TransitScratchProbeCase, work_root: Path
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


def configure_probe_dir(target_dir: Path, case: TransitScratchProbeCase) -> None:
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
    return FleetTrace(
        turn=turn,
        coords=parse_coords(values["coords"]),
        target=parse_coords(values["target"]),
        order=values["order"],
        current_speed=int(values["current_speed"]),
        mission_aux=values["mission_aux"],
        eta_status=values["eta_status"],
        eta_years=eta_years,
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


def run_classic_turn(dir_path: Path) -> None:
    result = run_ecmaint(dir_path)
    if result.returncode != 0:
        sys.stdout.write(result.stdout)
        sys.stderr.write(result.stderr)
        raise SystemExit(result.returncode)


def run_probe(case: TransitScratchProbeCase, work_root: Path) -> ProbeResult:
    rust_dir, classic_dir = build_probe_dirs(case, work_root)

    rust_traces = [inspect_fleet(rust_dir, 0, live_dir=False)]
    classic_traces = [inspect_fleet(classic_dir, 0, live_dir=True)]

    for turn in range(1, case.turns + 1):
        run_nc_cli(["maint-rust", str(rust_dir), "1"])
        rust_traces.append(inspect_fleet(rust_dir, turn, live_dir=False))

        run_classic_turn(classic_dir)
        classic_traces.append(inspect_fleet(classic_dir, turn, live_dir=True))

    return ProbeResult(case=case, rust_traces=rust_traces, classic_traces=classic_traces)


def trace_compact(trace: FleetTrace) -> str:
    eta = (
        f"{trace.eta_years}"
        if trace.eta_status == "years" and trace.eta_years is not None
        else trace.eta_status
    )
    return (
        f"{trace.coords[0]},{trace.coords[1]} order={trace.order} spd={trace.current_speed} "
        f"aux={trace.mission_aux} eta={eta} "
        f"0d..12={trace.raw_0d_to_12} 19..1e={trace.raw_19_to_1e}"
    )


def render_markdown(results: list[ProbeResult]) -> str:
    lines = [
        "# ECMAINT Transit Scratch Audit",
        "",
        "Controlled in-transit probes comparing Rust maintenance against classic `ECMAINT`, focused on the movement scratch windows `0x0d..0x12` and `0x19..0x1e`.",
        "",
        "Current takeaways from this probe set:",
        "",
        "- `0x0d..0x12` and `0x13..0x18` hold the fleet's in-sector x and y fractions, and `0x19..0x1e` its depth, all as Turbo Pascal 6-byte reals.",
        "- In deep-space transit the depth is `0`, so `0x19..0x1e` reads all zero. A fleet still climbing out of orbit in its first year leaves a small residue such as `5b 00 00 00 00 40`.",
        "- Sector orders complete in the year the fleet enters the target sector, anywhere in it.",
        "",
        "| case | transit turns | classic transit `19..1e` all zero | transit byte match | visible movement match |",
        "| --- | --- | --- | --- | --- |",
    ]

    for result in results:
        lines.append(
            "| {name} | `{turns}` | {classic_zero} | {match_status} | {movement_match} |".format(
                name=result.case.name,
                turns=", ".join(str(turn) for turn in result.transit_turns) or "-",
                classic_zero="yes" if result.classic_transit_19_to_1e_all_zero else "no",
                match_status="yes" if result.transit_byte_match else "no",
                movement_match="yes" if result.classic_orders_match_rust else "no",
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
                f"- transit turns checked: `{', '.join(str(turn) for turn in result.transit_turns) or '-'}`",
                f"- classic transit `19..1e` all zero: `{'yes' if result.classic_transit_19_to_1e_all_zero else 'no'}`",
                f"- transit byte match: `{'yes' if result.transit_byte_match else 'no'}`",
                "",
                "| turn | Rust | Classic |",
                "| ---: | --- | --- |",
            ]
        )
        for rust_trace, classic_trace in zip(result.rust_traces, result.classic_traces, strict=True):
            lines.append(
                f"| {rust_trace.turn} | `{trace_compact(rust_trace)}` | `{trace_compact(classic_trace)}` |"
            )

    lines.extend(
        [
            "",
            f"_Generated by `{repo_relative(Path(__file__))}`._",
            "",
        ]
    )
    return "\n".join(lines)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--work-root",
        type=Path,
        default=DEFAULT_WORK_ROOT,
        help=f"temporary working directory (default: {DEFAULT_WORK_ROOT})",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=DEFAULT_OUTPUT,
        help=f"markdown output path (default: {repo_relative(DEFAULT_OUTPUT)})",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    results = [run_probe(case, args.work_root) for case in DEFAULT_CASES]
    markdown = render_markdown(results)
    args.output.write_text(markdown, encoding="utf-8")
    print(f"wrote {repo_relative(args.output)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
