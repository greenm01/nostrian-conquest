#!/usr/bin/env python3
"""Run a mixed-production economy probe against the original ECMAINT oracle."""

from __future__ import annotations

import argparse
import subprocess
import sys
import tempfile
import shutil
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RUST = ROOT / "rust"
SETUP_CONFIG = ROOT / "rust" / "nc-data" / "config" / "setup.example.kdl"
MUTABLE_FILES = [
    "PLAYER",
    "PLANETS",
    "FLEETS",
    "BASES",
    "IPBM",
    "SETUP",
    "CONQUEST",
    "DATABASE",
    "MESSAGES",
    "RESULTS",
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


def copy_tree(source: Path, target: Path) -> None:
    if target.exists():
        shutil.rmtree(target)
    shutil.copytree(source, target)


def sync_dat_to_existing_sav(target: Path) -> None:
    for stem in MUTABLE_FILES:
        dat = target / f"{stem}.DAT"
        sav = target / f"{stem}.SAV"
        if dat.exists() and sav.exists():
            shutil.copy2(dat, sav)


def apply_probe_via_mutable_context(target: Path, player: int, tax_rate: int) -> str:
    output = ""
    with tempfile.TemporaryDirectory(prefix="ec-economy-probe-dat-") as tmpdir:
        mutable = Path(tmpdir)
        for stem in MUTABLE_FILES:
            source = target / f"{stem}.DAT"
            if source.exists():
                shutil.copy2(source, mutable / f"{stem}.DAT")
        output = run_nc_cli(
            ["economy-tax-probe-init", str(mutable), str(player), str(tax_rate)]
        )
        for stem in MUTABLE_FILES:
            dat = mutable / f"{stem}.DAT"
            if dat.exists():
                shutil.copy2(dat, target / f"{stem}.DAT")

    sav_stems = [stem for stem in MUTABLE_FILES if (target / f"{stem}.SAV").exists()]
    if sav_stems:
        with tempfile.TemporaryDirectory(prefix="ec-economy-probe-sav-") as tmpdir:
            mutable = Path(tmpdir)
            for stem in MUTABLE_FILES:
                source = target / f"{stem}.SAV"
                if not source.exists():
                    source = target / f"{stem}.DAT"
                if source.exists():
                    shutil.copy2(source, mutable / f"{stem}.DAT")
            run_nc_cli(["economy-tax-probe-init", str(mutable), str(player), str(tax_rate)])
            for stem in sav_stems:
                dat = mutable / f"{stem}.DAT"
                if dat.exists():
                    shutil.copy2(dat, target / f"{stem}.SAV")

    return output


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("target_root", type=Path)
    parser.add_argument(
        "--source",
        type=Path,
        default=None,
        help="optional pre-maint fixture/baseline to copy instead of generating a new game",
    )
    parser.add_argument("--seed", type=int, default=1515)
    parser.add_argument("--player", type=int, default=1)
    parser.add_argument("--turns", type=int, default=1)
    parser.add_argument("--tax", type=int, nargs="+", default=[0, 25, 50, 65, 80])
    args = parser.parse_args()

    args.target_root.mkdir(parents=True, exist_ok=True)

    for tax_rate in args.tax:
        target = args.target_root / f"tax-{tax_rate:03d}"
        print(f"== tax {tax_rate}% ==")
        if args.source is not None:
            copy_tree(args.source, target)
            print(f"Copied source fixture: {args.source}")
        else:
            print(run_nc_cli([
                "sysop",
                "new-game",
                str(target),
                "--config",
                str(SETUP_CONFIG),
                "--seed",
                str(args.seed),
            ]).strip())
        print(apply_probe_via_mutable_context(target, args.player, tax_rate).strip())

        for turn in range(1, args.turns + 1):
            print(f"-- oracle turn {turn} --")
            print(run_oracle(target).strip())
            report = run_nc_cli(["economy-report", str(target), str(args.player)]).strip()
            report_path = target / f"economy-after-turn-{turn}.txt"
            report_path.write_text(report + "\n", encoding="utf-8")
            print(report)
            print(f"saved {report_path}")
            sync_dat_to_existing_sav(target)

    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except RuntimeError as exc:
        print(str(exc), file=sys.stderr)
        raise SystemExit(1)
