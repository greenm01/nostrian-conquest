"""Economic pacing experiments, not engine or DOS conformance tests."""

import argparse
import json
import math
import random
import statistics
from pathlib import Path


NC_SOURCE = "38e703f4bee662abc5278c06edca1b34e61ae465"
EC4X_SOURCE = "9f931281a76ed8f3ffca9c5e816f04d5170c973d"
SEED = 20261010


def classic_step(production, potential, tax, rng, base=False):
    """Normal owned world. Omit classic's foreign-base and record-order bugs."""
    cut = rng.randrange(10) if base else 0
    ratio = production / potential
    denominator = math.sqrt(max(0, 1 - ratio * ratio)) or 0.00001
    angle = math.atan(ratio / denominator)
    effective_tax = tax - cut + 5 - rng.randrange(11)
    angle += math.pi / 24 * (1 - effective_tax * 1.42857 / 100)
    production = round(potential * math.sin(max(0, min(math.pi / 2, angle))), 3)
    if tax > 70:
        production *= 1 - (tax - 70) * (rng.random() + 0.5) / 100
    production = max(1, production)
    return production, production * tax / 100


def nc_step(production, potential, tax, base=False):
    """Current integer helpers and the maintenance caller's old-P penalty."""
    gap = max(0, potential - production)
    growth = (gap * (100 - min(tax, 95)) + 399) // 400
    bonus = 50 if tax <= 50 else max(0, (65 - tax) * 50 // 15)
    if base:
        growth += (growth * bonus + 99) // 100
    growth = min(gap, max(1, growth))
    penalty = (production * (tax - 65) + 499) // 500 if tax > 65 else 0
    production = min(potential, max(0, production + growth - penalty))
    return production, production * tax // 100


def fractional_step(production, potential, tax, base=False):
    """Candidate only: NC coefficients, fractional state, no intermediate ceil."""
    gap = max(0, potential - production)
    growth = gap * (100 - min(tax, 95)) / 400
    bonus = 0.5 if tax <= 50 else max(0, (65 - tax) / 30)
    growth = min(gap, growth * (1 + bonus if base else 1))
    penalty = production * max(0, tax - 65) / 500
    production = min(potential, max(0, production + growth - penalty))
    return production, production * tax / 100


def path(step, start, taxes, potential=100, base=False):
    production, treasury, rows = start, 0, []
    for tax in taxes:
        production, income = step(production, potential, tax, base)
        treasury += income
        rows.append({"production": production, "income": income, "treasury": treasury})
    return rows


def moment(values):
    mean = statistics.mean(values)
    half_width = 1.96 * statistics.stdev(values) / math.sqrt(len(values))
    return {"mean": mean, "mean_95pct_normal_interval": [mean - half_width, mean + half_width]}


def classic_sample(tax, trials, base=False):
    rng = random.Random(SEED)
    samples = {year: {"production": [], "treasury": []} for year in (5, 10, 20)}
    for _ in range(trials):
        production, treasury = 1, 0
        for year in range(1, 21):
            production, income = classic_step(production, 100, tax, rng, base)
            treasury += income
            if year in samples:
                samples[year]["production"].append(production)
                samples[year]["treasury"].append(treasury)
    return {year: {key: moment(values) for key, values in row.items()} for year, row in samples.items()}


def experiment(trials):
    result = {
        "sources": {"nc": NC_SOURCE, "ec4x": EC4X_SOURCE},
        "seed": SEED,
        "classic_trials_per_case": trials,
        "assumptions": [
            "Normal owned K=100 world; no spending, conquest, movement, or loss.",
            "All three models start P=1, matching the colonization handler.",
            "Treasury is income accumulated locally; it is not spendable empire cash.",
            "A turn is one economic pass, starting after the colonization year.",
            "Classic uses Python floats/RNG, not Pascal reals or the DOS RNG.",
            "The fractional candidate is not an accepted rule or EC4X's formula.",
        ],
        "colony": [],
    }
    for tax, base in ((35, False), (50, False), (65, False), (70, False), (50, True)):
        case = {"tax": tax, "base_from_first_pass": base, "classic": classic_sample(tax, trials, base)}
        for name, step in (("nc", nc_step), ("fractional_candidate", fractional_step)):
            rows = path(step, 1, [tax] * 20, base=base)
            case[name] = {year: rows[year - 1] for year in (5, 10, 20)}
        result["colony"].append(case)

    result["mature_tax_sweep"] = []
    for tax in (50, 60, 65, 66, 67, 68, 69, 70, 71, 75, 80, 90, 100):
        row = {"tax": tax}
        for name, step in (("nc", nc_step), ("fractional_candidate", fractional_step)):
            rows = path(step, 100, [tax] * 300)
            row[name] = {year: rows[year - 1] for year in (1, 10, 20, 300)}
        result["mature_tax_sweep"].append(row)

    result["empire_policy_examples"] = []
    policies = {str(tax): [tax] * 20 for tax in (35, 50, 65, 70)}
    policies["35_for_5_then_65"] = [35] * 5 + [65] * 15
    for colonies in (0, 4):
        for policy, taxes in policies.items():
            row = {"mature_worlds": 1, "new_colonies": colonies, "policy": policy}
            for name, step in (("nc", nc_step), ("fractional_candidate", fractional_step)):
                home, colony = path(step, 100, taxes), path(step, 1, taxes)
                row[name] = {
                    year: {
                        "total_production": home[year - 1]["production"] + colonies * colony[year - 1]["production"],
                        "total_income_accumulated": home[year - 1]["treasury"] + colonies * colony[year - 1]["treasury"],
                    }
                    for year in (5, 10, 20)
                }
            result["empire_policy_examples"].append(row)

    result["automatic_cost_illustration"] = [
        {"upkeep_fraction": u, "income": 100, "fleet_cost_after_20_turns":
         2000 if u == 0 else 100 * (1 - (1 - u) ** 20) / u,
         "equilibrium_fleet_cost": None if u == 0 else 100 / u}
        for u in (0, 0.02, 0.05, 0.10)
    ]
    result["source_arithmetic"] = {
        "vbam_population_1_to_2_at_raw_4_payback": 20 / 4,
        "vbam_population_4_to_5_at_raw_4_payback": 50 / 4,
        "ec4x_one_IU_low_tier_EL1_CST1_tax50_payback_ignoring_rounding": 30 / (1.05 * 0.5),
    }
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--trials", type=int, default=5000)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if args.trials < 2:
        parser.error("--trials must be at least 2 for the mean interval")
    rendered = json.dumps(experiment(args.trials), indent=2) + "\n"
    if args.output:
        args.output.write_text(rendered)
    else:
        print(rendered, end="")


if __name__ == "__main__":
    main()
