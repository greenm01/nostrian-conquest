#!/usr/bin/env python3
"""Run a fixture-backed classic oracle sweep for starbase colony economics."""

from __future__ import annotations

import argparse
import re
import shutil
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RUST = ROOT / "rust"
FIXTURE = ROOT / "fixtures" / "ecmaint-econ-pre" / "v1.5"
DEFAULT_TAXES = [25, 50, 65, 67, 68, 69, 70, 71, 80]
ROW_RE = re.compile(
    r"^\s*(?P<record>\d+)\s+\((?P<x>\d+),(?P<y>\d+)\)\s+"
    r"(?P<name>.{13})\s+"
    r"(?P<present>\d+)\s+(?P<potential>\d+)\s+(?P<stored>\d+)\s+"
    r"(?P<rev>\d+)\s+(?P<grow>\d+)\s+(?P<cap>\d+)\s+(?P<army>\d+)\s+(?P<bat>\d+)"
    r"(?P<tags>.*)$"
)

COLONIES = [
    {
        "record": 10,
        "name": "Probe",
        "coords": (6, 8),
        "present": 25,
        "armies": 1,
        "batteries": 0,
    },
    {
        "record": 11,
        "name": "ProbeB",
        "coords": (12, 11),
        "present": 25,
        "armies": 1,
        "batteries": 0,
    },
]


def run(cmd: list[str], cwd: Path) -> str:
    result = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)
    if result.returncode != 0:
        raise RuntimeError(
            f"command failed: {' '.join(cmd)}\nstdout:\n{result.stdout}\nstderr:\n{result.stderr}"
        )
    return result.stdout


def run_nc_cli(args: list[str]) -> str:
    return run(["cargo", "run", "-q", "-p", "nc-cli", "--", *args], RUST)


def run_oracle(target: Path) -> str:
    return run(["python3", "tools/ncmaint_oracle.py", "run", str(target)], ROOT)


def extract_row(report: str, name: str) -> dict[str, str]:
    for line in report.splitlines():
        match = ROW_RE.match(line)
        if match and match.group("name").strip() == name:
            row = match.groupdict()
            row["name"] = row["name"].strip()
            row["tags"] = row["tags"].strip()
            return row
    raise RuntimeError(f"row '{name}' not found in report:\n{report}")


def setup_case(case_dir: Path, tax_rate: int, colony: dict[str, object], with_base: bool) -> None:
    shutil.copytree(FIXTURE, case_dir, dirs_exist_ok=True)
    run_nc_cli(["db-import", str(case_dir)])
    run_nc_cli(["player-join", str(case_dir), "1", "SYSOP", "Alpha", "Foundation"])
    run_nc_cli(["player-tax", str(case_dir), "1", str(tax_rate)])

    record = int(colony["record"])
    name = str(colony["name"])
    present = int(colony["present"])
    armies = int(colony["armies"])
    batteries = int(colony["batteries"])
    x, y = colony["coords"]

    run_nc_cli(["planet-owner", str(case_dir), str(record), "1"])
    run_nc_cli(["planet-name", str(case_dir), str(record), name])
    run_nc_cli(["planet-present", str(case_dir), str(record), str(present)])
    run_nc_cli(["planet-stored", str(case_dir), str(record), "0"])
    run_nc_cli(["planet-stats", str(case_dir), str(record), str(armies), str(batteries)])

    if with_base:
        run_nc_cli(["guard-starbase-onebase", str(case_dir), str(x), str(y)])


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("target_root", type=Path)
    parser.add_argument("--tax", type=int, nargs="+", default=DEFAULT_TAXES)
    args = parser.parse_args()

    args.target_root.mkdir(parents=True, exist_ok=True)
    summary_lines = [
        "Starbase Colony Oracle Sweep",
        f"target_root={args.target_root}",
        f"fixture={FIXTURE}",
        "",
    ]

    for colony in COLONIES:
        colony_name = str(colony["name"])
        summary_lines.append(
            f"colony={colony_name} record={colony['record']} coords={colony['coords']} present={colony['present']}"
        )
        for tax_rate in args.tax:
            rows: dict[str, dict[str, str]] = {}
            for mode, with_base in [("plain", False), ("base", True)]:
                case_dir = args.target_root / f"{colony_name.lower()}-{mode}-tax-{tax_rate:03d}"
                setup_case(case_dir, tax_rate, colony, with_base)
                pre_report = run_nc_cli(["economy-report", str(case_dir), "1"])
                run_nc_cli(["db-export", str(case_dir), str(case_dir)])
                oracle_log = run_oracle(case_dir)
                run_nc_cli(["db-import", str(case_dir)])
                post_report = run_nc_cli(["economy-report", str(case_dir), "1"])

                (case_dir / "economy-before.txt").write_text(pre_report, encoding="utf-8")
                (case_dir / "economy-after.txt").write_text(post_report, encoding="utf-8")
                (case_dir / "oracle.log").write_text(oracle_log, encoding="utf-8")

                rows[mode] = extract_row(post_report, colony_name)

            line = (
                f"  tax={tax_rate:>3} "
                f"plain_present={rows['plain']['present']} base_present={rows['base']['present']} "
                f"plain_rev={rows['plain']['rev']} base_rev={rows['base']['rev']} "
                f"plain_grow={rows['plain']['grow']} base_grow={rows['base']['grow']} "
                f"plain_cap={rows['plain']['cap']} base_cap={rows['base']['cap']}"
            )
            print(f"{colony_name} {line.strip()}")
            summary_lines.append(line)
        summary_lines.append("")

    (args.target_root / "SUMMARY.txt").write_text(
        "\n".join(summary_lines) + "\n", encoding="utf-8"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except RuntimeError as exc:
        print(str(exc), file=sys.stderr)
        raise SystemExit(1)
