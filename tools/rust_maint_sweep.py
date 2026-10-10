#!/usr/bin/env python3
"""
Validate multi-turn Rust maintenance outputs against the original ECMAINT oracle.

This script:

- creates seeded new games through `nc-cli sysop new-game`
- runs `nc-cli maint-rust` for multiple turns
- runs the original ECMAINT oracle on the resulting directory

It is intended as an end-to-end confidence check that Rust-produced live
campaign state remains acceptable to the original toolchain after repeated
maintenance, not just after initial setup.

It therefore uses the explicit `builder-compatible` setup config rather than
the default `sysop new-game` joinable baseline used for live `ECGAME`
onboarding.
"""

from __future__ import annotations

import argparse
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import List


REPO_ROOT = Path(__file__).parent.parent
RUST_DIR = REPO_ROOT / "rust"


@dataclass
class SweepResult:
    player_count: int
    seed: int
    turns: int
    passed_generation: bool
    passed_maint: bool
    passed_oracle: bool
    errors: List[str]


def run_cmd(cmd: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)


def generate_seeded_new_game(target_dir: Path, player_count: int, seed: int) -> tuple[bool, list[str]]:
    result = run_cmd(
        [
            "cargo",
            "run",
            "-q",
            "-p",
            "nc-cli",
            "--",
            "sysop",
            "new-game",
            str(target_dir),
            "--config",
            "nc-data/config/setup.example.kdl",
            "--players",
            str(player_count),
            "--seed",
            str(seed),
        ],
        RUST_DIR,
    )
    if result.returncode != 0:
        return False, [result.stderr or result.stdout or "generation failed"]
    return True, []


def run_rust_maint(target_dir: Path, turns: int) -> tuple[bool, list[str]]:
    result = run_cmd(
        ["cargo", "run", "-q", "-p", "nc-cli", "--", "maint-rust", str(target_dir), str(turns)],
        RUST_DIR,
    )
    if result.returncode != 0:
        return False, [result.stderr or result.stdout or "maint-rust failed"]
    if "Rust maintenance complete." not in result.stdout:
        return False, [f"unexpected maint-rust stdout: {result.stdout}"]
    return True, []


def run_oracle(target_dir: Path) -> tuple[bool, list[str]]:
    result = run_cmd(
        ["python3", "tools/ncmaint_oracle.py", "run", str(target_dir)],
        REPO_ROOT,
    )
    if result.returncode != 0:
        return False, [result.stderr or result.stdout or "oracle run failed"]

    stdout_lower = result.stdout.lower()
    errors: list[str] = []
    if "integrity" in stdout_lower and "abort" in stdout_lower:
        errors.append("ECMAINT integrity abort detected")
    if "failed" in stdout_lower:
        errors.append("ECMAINT failure detected")
    if result.stdout.count("ERROR") > 1:
        errors.append("multiple ERROR entries detected")
    return len(errors) == 0, errors


def sweep(turns: int, seeds: list[int]) -> list[SweepResult]:
    results: list[SweepResult] = []
    for player_count in [4, 9, 16, 25]:
        for seed in seeds:
            errors: list[str] = []
            with tempfile.TemporaryDirectory() as tmpdir:
                target = Path(tmpdir) / f"rust-maint-{player_count}p-{seed}"

                passed_generation, generation_errors = generate_seeded_new_game(
                    target, player_count, seed
                )
                errors.extend(generation_errors)
                if not passed_generation:
                    results.append(
                        SweepResult(
                            player_count, seed, turns, False, False, False, errors
                        )
                    )
                    print(f"✗ {player_count}p seed={seed}: generation failed - {errors}")
                    continue

                passed_maint, maint_errors = run_rust_maint(target, turns)
                errors.extend(maint_errors)
                if not passed_maint:
                    results.append(
                        SweepResult(
                            player_count, seed, turns, True, False, False, errors
                        )
                    )
                    print(f"✗ {player_count}p seed={seed}: maint-rust failed - {errors}")
                    continue

                passed_oracle, oracle_errors = run_oracle(target)
                errors.extend(oracle_errors)
                results.append(
                    SweepResult(
                        player_count, seed, turns, True, True, passed_oracle, errors
                    )
                )
                if passed_oracle:
                    print(f"✓ {player_count}p seed={seed} turns={turns}: PASSED")
                else:
                    print(f"✗ {player_count}p seed={seed} turns={turns}: oracle failed - {errors}")

    return results


def main() -> int:
    parser = argparse.ArgumentParser(description="Run multi-turn Rust maint oracle sweeps.")
    parser.add_argument("--turns", type=int, default=3, help="number of Rust maint turns to run")
    parser.add_argument(
        "--seeds",
        type=int,
        nargs="*",
        default=[1515, 2025],
        help="seeds to test",
    )
    args = parser.parse_args()

    results = sweep(args.turns, args.seeds)
    passed = sum(1 for result in results if result.passed_oracle)
    total = len(results)
    print(f"\nSuccess rate: {passed}/{total}")
    return 0 if passed == total else 1


if __name__ == "__main__":
    raise SystemExit(main())
