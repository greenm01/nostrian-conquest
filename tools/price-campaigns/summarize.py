#!/usr/bin/env python3
"""Tables from price_campaigns TSV output (t090).

Reads every `.tsv` given on the command line (the output of
`cargo run --release -p nc-harness --example price_campaigns`), groups the
rows by price label, goods, player count, opening, and doctrine, and prints
Markdown tables of the means that bear on prices. A `-` in a dated column is
"never within the game"; the share column beside it says how often it
happened at all.

    python3 tools/price-campaigns/summarize.py /tmp/price-campaigns/*.tsv
"""

import argparse
import json
import statistics
from collections import defaultdict

G_COLUMNS = (
    "kind label prices goods players opening doctrine seed years winner winner_year "
    "eliminations battles battle_reports assaults assaults_resolved home_assaults "
    "assault_failures captures bombards colonies first_battle first_capture "
    "lost_dd lost_ca lost_bb lost_sc lost_tt lost_etac"
).split()
E_COLUMNS = (
    "kind label prices goods players opening doctrine seed empire empire_opening empire_doctrine "
    "first_colony col_by5 col_by10 col_by20 owned5 owned10 owned20 owned_end "
    "built_dd built_ca built_bb built_sc built_tt built_etac built_gb built_army "
    "built_sb built_bb_home first_bb_order first_base exhausted_years wait_years "
    "etacs_bought assaults captures eliminated home_lost alive winner "
    "lost_dd lost_ca lost_bb lost_sc lost_tt lost_etac"
).split()


def parse(paths):
    games, empires = [], []
    for path in paths:
        with open(path, encoding="utf-8") as handle:
            for line in handle:
                if line.startswith("#") or not line.strip():
                    continue
                fields = line.rstrip("\n").split("\t")
                columns = {"G": G_COLUMNS, "E": E_COLUMNS}.get(fields[0])
                if columns is None or len(fields) != len(columns):
                    raise ValueError(f"{path}: malformed {fields[0]} row ({len(fields)} fields)")
                row = dict(zip(columns, fields))
                (games if fields[0] == "G" else empires).append(row)
    return games, empires


def number(value):
    return None if value == "-" else float(value)


def mean(rows, column):
    values = [number(row[column]) for row in rows]
    values = [value for value in values if value is not None]
    return statistics.fmean(values) if values else None


def share(rows, column):
    """Share of rows where the dated column has a value."""
    if not rows:
        return None
    return sum(1 for row in rows if row[column] != "-") / len(rows)


def fmt(value, digits=1):
    if value is None:
        return "-"
    return f"{value:.{digits}f}"


def pct(value):
    return "-" if value is None else f"{100 * value:.0f}%"


def key_of(row):
    return (row["label"], row["goods"], row["players"], row["opening"], row["doctrine"])


def header(label):
    return f"{label[0]} g{label[1]} {label[2]}p {label[3]} {label[4]}"


def table(title, columns, rows_by_key, cell):
    print(f"\n### {title}\n")
    print("| cell | n | " + " | ".join(columns) + " |")
    print("| --- | ---: | " + " | ".join("---:" for _ in columns) + " |")
    for key in sorted(rows_by_key):
        rows = rows_by_key[key]
        print(
            f"| {header(key)} | {len(rows)} | "
            + " | ".join(cell(rows, column) for column in columns)
            + " |"
        )


def main(paths):
    games, empires = parse(paths)
    games_by = defaultdict(list)
    for row in games:
        games_by[key_of(row)].append(row)
    empires_by = defaultdict(list)
    for row in empires:
        empires_by[key_of(row)].append(row)

    print("## Per empire (means over empires; dated columns: mean year when it happened, share it happened)")
    table(
        "Expansion",
        ["first_colony", "col_by5", "col_by10", "col_by20", "owned10", "owned20", "owned_end", "etacs_bought", "lost_etac"],
        empires_by,
        lambda rows, column: fmt(mean(rows, column), 2)
        + (f" ({pct(share(rows, column))})" if column == "first_colony" else ""),
    )
    table(
        "Builds by class (units ordered per empire over the game)",
        ["built_dd", "built_ca", "built_bb", "built_tt", "built_etac", "built_army", "built_gb", "built_sb", "built_bb_home"],
        empires_by,
        lambda rows, column: fmt(mean(rows, column), 1),
    )
    table(
        "First battleship order and first starbase at home (mean year, share)",
        ["first_bb_order", "first_base", "exhausted_years", "wait_years"],
        empires_by,
        lambda rows, column: fmt(mean(rows, column), 2)
        + (f" ({pct(share(rows, column))})" if column.startswith("first") else ""),
    )
    table(
        "War per empire",
        ["assaults", "captures", "lost_dd", "lost_ca", "lost_bb", "lost_tt", "home_lost", "eliminated", "alive", "winner"],
        empires_by,
        lambda rows, column: (
            fmt(mean(rows, column), 1) + f" ({pct(share(rows, column))})"
            if column in ("home_lost", "eliminated")
            else pct(mean(rows, column))
            if column in ("alive", "winner")
            else fmt(mean(rows, column), 1)
        ),
    )

    print("\n## Per game")
    table(
        "Campaign",
        ["years", "winner_year", "eliminations", "battles", "assaults_resolved", "home_assaults", "assault_failures", "captures", "colonies", "first_battle", "first_capture"],
        games_by,
        lambda rows, column: (
            fmt(mean(rows, column), 1) + f" ({pct(share(rows, column))})"
            if column in ("winner_year", "first_battle", "first_capture")
            else fmt(mean(rows, column), 1)
        ),
    )

    # Mixed games: outcomes by the empire's own opening or doctrine, pooled
    # over seeds.
    for mixed_column, own_column, title in (
        ("opening", "empire_opening", "opening"),
        ("doctrine", "empire_doctrine", "doctrine"),
    ):
        mixed = [row for row in empires if row[mixed_column] == "mixed"]
        if not mixed:
            continue
        print(f"\n## Mixed games: outcome by the empire's {title}")
        by = defaultdict(list)
        for row in mixed:
            by[(row["label"], row["goods"], row["players"], row["opening"] + "/" + row["doctrine"], row[own_column])].append(row)
        columns = ["col_by10", "owned20", "owned_end", "built_bb", "built_ca", "first_bb_order", "first_base", "captures", "home_lost", "eliminated", "alive", "winner"]
        print("\n| cell | n | " + " | ".join(columns) + " |")
        print("| --- | ---: | " + " | ".join("---:" for _ in columns) + " |")
        for key in sorted(by):
            rows = by[key]
            cells = []
            for column in columns:
                if column in ("home_lost", "eliminated"):
                    cells.append(fmt(mean(rows, column), 1) + f" ({pct(share(rows, column))})")
                elif column in ("alive", "winner"):
                    cells.append(pct(mean(rows, column)))
                elif column.startswith("first"):
                    cells.append(fmt(mean(rows, column), 1) + f" ({pct(share(rows, column))})")
                else:
                    cells.append(fmt(mean(rows, column), 2))
            print(f"| {key[0]} g{key[1]} {key[2]}p {key[3]} {key[4]} | {len(rows)} | " + " | ".join(cells) + " |")


def compact_summary(paths):
    """Campaign counts and group means; samples are campaigns, not independent seats."""
    games, empires = parse(paths)
    cells = []
    for key in sorted({key_of(row) for row in games}):
        g = [row for row in games if key_of(row) == key]
        e = [row for row in empires if key_of(row) == key]
        seeds = [int(row["seed"]) for row in g]
        if len(set(seeds)) != len(seeds):
            raise ValueError(f"duplicate campaign seeds in {key}")
        for seed in seeds:
            seats = [int(row["empire"]) for row in e if int(row["seed"]) == seed]
            if sorted(seats) != list(range(1, int(key[2]) + 1)):
                raise ValueError(f"missing or duplicate empire rows in {key}, seed {seed}")
        def metrics(rows, columns):
            return {column: {"mean": mean(rows, column), "observed": share(rows, column)}
                    for column in columns}
        cells.append({
            "label": key[0], "goods": int(key[1]), "players": int(key[2]),
            "opening": key[3], "doctrine": key[4], "seeds": sorted(seeds),
            "campaigns": len(g), "empires": len(e),
            "game": metrics(g, G_COLUMNS[8:]),
            "empire": metrics(e, E_COLUMNS[11:]),
            "by_opening": {o: metrics([r for r in e if r["empire_opening"] == o], E_COLUMNS[11:])
                           for o in sorted({r["empire_opening"] for r in e}) if key[3] == "mixed"},
            "by_doctrine": {d: metrics([r for r in e if r["empire_doctrine"] == d], E_COLUMNS[11:])
                            for d in sorted({r["empire_doctrine"] for r in e}) if key[4] == "mixed"},
        })
    return {"campaigns": len(games), "empires": len(empires), "cells": cells}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="+")
    parser.add_argument("--json", help="also write compact numeric results")
    args = parser.parse_args()
    if args.json:
        with open(args.json, "w", encoding="utf-8") as handle:
            json.dump(compact_summary(args.paths), handle, indent=2)
            handle.write("\n")
    main(args.paths)
