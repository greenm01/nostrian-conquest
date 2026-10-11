#!/usr/bin/env python3
"""Paired map bootstrap for value versus cruisers (t090). Prints JSON."""
import json
import random
import statistics
import sys
import summarize


def compare(paths):
    _, empires = summarize.parse(paths)
    controls = {}
    for label in ("base", "bb30", "bb60"):
        rows = [r for r in empires if r["label"] == label and r["doctrine"] == "mixed"]
        differences = []
        for seed in sorted({r["seed"] for r in rows}, key=int):
            seats = [r for r in rows if r["seed"] == seed]
            differences.append(
                summarize.mean([r for r in seats if r["empire_doctrine"] == "value"], "owned_end")
                - summarize.mean([r for r in seats if r["empire_doctrine"] == "cruisers"], "owned_end")
            )
        rng = random.Random(20261011)
        means = sorted(statistics.fmean(rng.choices(differences, k=len(differences))) for _ in range(5000))
        controls[label] = {"campaigns": len(differences),
                           "mean_world_difference": statistics.fmean(differences),
                           "map_bootstrap_95": [means[125], means[4874]],
                           "map_differences": differences}
    return {"method": "Within each 4-player campaign: mean final worlds of value seats minus mean of cruisers seats. Resample paired campaigns 5000 times with random.Random(20261011); percentile indices 125 and 4874. Conditional on this map range and policy.",
            "controls": controls}


if __name__ == "__main__":
    print(json.dumps(compare(sys.argv[1:]), indent=2))
