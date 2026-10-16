"""Run `executor_matches` on this machine and one remote together: the games
are split in halves by map range, and the halves are combined (t189).

    uv run split_matches.py --remote bean-void -- --weights tuned \\
        --opponent tuned --with watch --games 1024 --years 100 --first-map 1200000 --paired

Both machines must be on the same commit and have the examples built
(`cargo build --release -p nc-ai --examples` in `rust/`). The engine is
deterministic, so a half gives the same numbers on either machine. A
`.kdl` personality named in the arguments is copied to the remote first.
"""

import argparse
import re
import shlex
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
EXAMPLE = "rust/target/release/examples/executor_matches"
PAIRED = re.compile(
    r"(\d+) paired games, (\d+) years: win rate with ([\d.]+), without ([\d.]+); "
    r"difference ([+-][\d.]+) ± ([\d.]+)"
)
PLAIN = re.compile(r"win rate ([\d.]+)")


def option(args, name):
    return args[args.index(name) + 1]


def with_option(args, name, value):
    args = list(args)
    args[args.index(name) + 1] = str(value)
    return args


def run_local(args):
    return subprocess.run([str(REPO / EXAMPLE), *args], capture_output=True, text=True, check=True).stdout


def run_remote(remote, args):
    # The remote shell may be fish, so keep the command to plain words.
    command = f"cd ~/dev/nostrian-conquest; {shlex.join(['./' + EXAMPLE, *args])}"
    return subprocess.run(["ssh", remote, command], capture_output=True, text=True, check=True).stdout


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--remote", required=True)
    parser.add_argument("args", nargs=argparse.REMAINDER, help="executor_matches arguments, after --")
    ns = parser.parse_args()
    args = ns.args[1:] if ns.args[:1] == ["--"] else ns.args
    games, first = int(option(args, "--games")), int(option(args, "--first-map"))
    if games % 2:
        sys.exit("--games must be even")
    half = games // 2
    for word in args:
        if word.endswith(".kdl"):
            subprocess.run(["scp", "-q", word, f"{ns.remote}:{word}"], check=True)

    here = with_option(with_option(args, "--games", half), "--first-map", first)
    there = with_option(with_option(args, "--games", half), "--first-map", first + half)
    with ThreadPoolExecutor(2) as pool:
        local, remote = pool.map(lambda job: job(), [lambda: run_local(here), lambda: run_remote(ns.remote, there)])

    halves = [PAIRED.search(text) for text in (local, remote)]
    if all(halves):
        w, wo, diff = (sum(float(m.group(i)) for m in halves) / 2 for i in (3, 4, 5))
        se = sum(float(m.group(6)) ** 2 for m in halves) ** 0.5 / 2
        print(f"{games} paired games, {halves[0].group(2)} years: win rate with {w:.3f}, "
              f"without {wo:.3f}; difference {diff:+.3f} ± {se:.3f} (standard error)")
        return
    rates = [PLAIN.search(text) for text in (local, remote)]
    if all(rates):
        print(f"{games} games: win rate {sum(float(m.group(1)) for m in rates) / 2:.3f}")
        return
    sys.exit("could not read the results:\n" + local + remote)


if __name__ == "__main__":
    main()
