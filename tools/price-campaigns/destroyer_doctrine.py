#!/usr/bin/env python3
"""Cost and benefit of destroyer-only empires (t146).

Reads price_campaigns TSVs, one cell per file, and prints per-doctrine seat
means plus the destroyer seat's advantage over its opponents in mixed cells,
with 95% intervals from resampling whole games.
"""
import argparse
import random
import statistics
import sys
from collections import defaultdict
from pathlib import Path

COST = {"dd": 5, "ca": 15, "bb": 45}


def read(path):
    header, games = None, defaultdict(list)
    for line in Path(path).read_text().splitlines():
        if line.startswith("# E\t"):
            header = line[2:].split("\t")
        elif line.startswith("E\t"):
            row = dict(zip(header, line.split("\t")))
            games[row["seed"]].append(seat(row))
    return games


def seat(row):
    num = lambda key: float(row[key]) if row[key] not in ("", "-") else 0.0
    spent = sum(num(f"built_{k}") * c for k, c in COST.items())
    lost = sum(num(f"lost_{k}") * c for k, c in COST.items())
    return {
        "doctrine": row["empire_doctrine"],
        "win": float(row["winner"] in ("1", "yes", "true")),
        "alive": float(row["alive"] in ("1", "yes", "true")),
        "owned20": num("owned20"),
        "owned_end": num("owned_end"),
        "captures": num("captures"),
        "spent": spent,
        "lost": lost,
        "per100": 100 * num("owned_end") / spent if spent else 0.0,
    }


METRICS = ["win", "alive", "owned20", "owned_end", "captures", "spent", "lost", "per100"]


def interval(values, draws=2000, rng=random.Random(146)):
    if not values:
        return (float("nan"),) * 3
    means = sorted(statistics.fmean(rng.choices(values, k=len(values))) for _ in range(draws))
    return statistics.fmean(values), means[int(0.025 * draws)], means[int(0.975 * draws)]


def fmt(stat, pct=False):
    mean, lo, hi = stat
    return f"{100*mean:.1f}% [{100*lo:.1f}, {100*hi:.1f}]" if pct else f"{mean:.2f} [{lo:.2f}, {hi:.2f}]"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("tsv", nargs="+")
    args = parser.parse_args()
    for path in args.tsv:
        games = read(path)
        print(f"## {Path(path).stem} ({len(games)} games)")
        by = defaultdict(lambda: defaultdict(list))
        for seats in games.values():
            for s in seats:
                for m in METRICS:
                    by[s["doctrine"]][m].append(s[m])
        for doctrine, values in sorted(by.items()):
            cells = ", ".join(
                f"{m} {fmt(interval(values[m]), m in ('win', 'alive'))}" for m in METRICS
            )
            print(f"- {doctrine} ({len(values['win'])} seats): {cells}")
        doctrines = {s["doctrine"] for seats in games.values() for s in seats}
        if "destroyers" in doctrines and len(doctrines) > 1:
            for m in METRICS:
                diffs = []
                for seats in games.values():
                    dd = [s[m] for s in seats if s["doctrine"] == "destroyers"]
                    other = [s[m] for s in seats if s["doctrine"] != "destroyers"]
                    diffs.append(statistics.fmean(dd) - statistics.fmean(other))
                print(f"  destroyer seat minus opponents, {m}: {fmt(interval(diffs), m in ('win', 'alive'))}")
        print()


if __name__ == "__main__":
    sys.exit(main())
