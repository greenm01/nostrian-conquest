#!/usr/bin/env python3
"""Run a controlled classic oracle sweep for starbase growth and tax burden."""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RUST = ROOT / "rust"
DEFAULT_TAXES = [25, 50, 65, 67, 68, 69, 70, 71, 80]
ROW_RE = re.compile(
    r"^\s*(?P<record>\d+)\s+\((?P<x>\d+),(?P<y>\d+)\)\s+"
    r"(?P<name>.{13})\s+"
    r"(?P<present>\d+)\s+(?P<potential>\d+)\s+(?P<stored>\d+)\s+"
    r"(?P<rev>\d+)\s+(?P<grow>\d+)\s+(?P<cap>\d+)\s+(?P<army>\d+)\s+(?P<bat>\d+)"
    r"(?P<tags>.*)$"
)


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


def setup_runtime_probe(runtime_dir: Path, tax_rate: int, seed: int) -> str:
    log = []
    log.append(
        run_nc_cli(
            [
                "sysop",
                "new-game",
                str(runtime_dir),
                "--players",
                "4",
                "--seed",
                str(seed),
            ]
        ).strip()
    )
    log.append(
        run_nc_cli(
            ["player-join", str(runtime_dir), "1", "SYSOP", "Alpha", "Foundation"]
        ).strip()
    )
    log.append(
        run_nc_cli(["player-name", str(runtime_dir), "2", "HECATE", "RedHorizon"]).strip()
    )
    log.append(run_nc_cli(["player-name", str(runtime_dir), "3", "ORION", "Vela"]).strip())
    log.append(run_nc_cli(["player-name", str(runtime_dir), "4", "TESS", "Helios"]).strip())
    for player in range(1, 5):
        log.append(
            run_nc_cli(["player-tax", str(runtime_dir), str(player), "50"]).strip()
        )
    log.append(
        run_nc_cli(
            ["fleet-ships", str(runtime_dir), "2", "1", "0", "1", "2", "0", "0", "0"]
        ).strip()
    )
    log.append(
        run_nc_cli(["fleet-order", str(runtime_dir), "2", "3", "1", "12", "12"]).strip()
    )
    log.append(
        run_nc_cli(
            ["economy-starbase-probe-init", str(runtime_dir), "1", str(tax_rate)]
        ).strip()
    )
    return "\n".join(log) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("target_root", type=Path)
    parser.add_argument("--seed", type=int, default=1515)
    parser.add_argument("--turns", type=int, default=1)
    parser.add_argument("--tax", type=int, nargs="+", default=DEFAULT_TAXES)
    args = parser.parse_args()

    args.target_root.mkdir(parents=True, exist_ok=True)
    summary_lines = [
        "Starbase Economy Oracle Audit",
        f"target_root={args.target_root}",
        f"seed={args.seed}",
        f"turns={args.turns}",
        "",
    ]

    for tax_rate in args.tax:
        case_root = args.target_root / f"tax-{tax_rate:03d}"
        runtime_dir = case_root / "runtime"
        classic_dir = case_root / "classic"
        case_root.mkdir(parents=True, exist_ok=True)

        print(f"== tax {tax_rate}% ==")
        setup_log = setup_runtime_probe(runtime_dir, tax_rate, args.seed)
        (case_root / "setup.log").write_text(setup_log, encoding="utf-8")

        pre_report = run_nc_cli(["economy-report", str(runtime_dir), "1"])
        (case_root / "economy-before.txt").write_text(pre_report, encoding="utf-8")

        export_log = run_nc_cli(["db-export", str(runtime_dir), str(classic_dir)])
        oracle_logs = [export_log.strip()]
        for turn in range(1, args.turns + 1):
            oracle_logs.append(f"-- oracle turn {turn} --")
            oracle_logs.append(run_oracle(classic_dir).strip())
        (case_root / "oracle.log").write_text("\n".join(oracle_logs) + "\n", encoding="utf-8")

        import_log = run_nc_cli(["db-import", str(classic_dir)])
        post_report = run_nc_cli(["economy-report", str(classic_dir), "1"])
        (case_root / "economy-after.txt").write_text(post_report, encoding="utf-8")
        (case_root / "import.log").write_text(import_log, encoding="utf-8")

        plain = extract_row(post_report, "Plain Colony")
        starbase = extract_row(post_report, "Base Colony")
        plain_before = extract_row(pre_report, "Plain Colony")
        starbase_before = extract_row(pre_report, "Base Colony")

        summary_line = (
            f"tax={tax_rate:>3} "
            f"plain_present={plain['present']} delta={int(plain['present']) - int(plain_before['present']):+d} "
            f"starbase_present={starbase['present']} delta={int(starbase['present']) - int(starbase_before['present']):+d} "
            f"plain_cap={plain['cap']} starbase_cap={starbase['cap']}"
        )
        print(summary_line)
        summary_lines.append(summary_line)
        summary_lines.append(f"  plain_row={next(line.strip() for line in post_report.splitlines() if 'Plain Colony' in line)}")
        summary_lines.append(
            f"  starbase_row={next(line.strip() for line in post_report.splitlines() if 'Base Colony' in line)}"
        )
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
