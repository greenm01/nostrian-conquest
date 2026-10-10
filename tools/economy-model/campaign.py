"""Synthetic geography and economic milestones; no combat or engine emulation."""

import argparse
import json
import math
import random
import statistics
from pathlib import Path

from analyze import NC_SOURCE, SEED, classic_step, fractional_step, nc_step
from repair import repair_batch, repair_installments


HOME = (16, 16)
HORIZON = 25


def travel_ticks(start, end, speed):
    """Ideal full-orbit arrival. Omits real-number and mission arrival exceptions."""
    return math.ceil(52 * (math.dist(start, end) + 2) / speed)


def travel_years(start, end, speed):
    return math.ceil(travel_ticks(start, end, speed) / 52)


def sampled_map(seed, worlds):
    rng = random.Random(seed)
    cells = [(x, y) for x in range(1, 32) for y in range(1, 32) if (x, y) != HOME]
    return [(coords, rng.randint(10, 150)) for coords in rng.sample(cells, worlds)]


def tax_at(policy, year):
    return 35 if policy == "35_then_65" and year <= 5 else 65 if policy == "35_then_65" else int(policy)


def simulate(seed, worlds=24, model="nc", policy="50", colonizers=2, etac_cost=20, speed=3,
             repair_costs=(), repair_first=True, repair_mode='whole'):
    targets = sampled_map(seed, worlds)
    rng = random.Random(seed + 1000000)
    planets = [{"coords": HOME, "p": 100, "k": 100, "goods": 50, "home": True}]
    # Two reusable starter ETACs; starter escorts/defenders are outside this model.
    ships = [{"coords": HOME, "ready": 1, "target": None, "arrival": None} for _ in range(2)]
    metrics = {key: None for key in (
        "first_colony", "four_colonies", "eight_colonies", "colony_income_reaches_home",
        "first_colony_can_order_CA", "first_colony_can_order_BB30",
        "first_colony_can_order_BB45", "first_colony_can_order_BB60",
    )}
    rows = {}
    damaged = list(repair_costs)
    repair_rule = repair_batch
    if repair_mode == 'installments':
        damaged = [(cost, 0) for cost in repair_costs]
        repair_rule = repair_installments
    repair_rows = []

    def step(planet, tax):
        if model == "classic":
            return classic_step(planet["p"], planet["k"], tax, rng)
        rule = nc_step if model == "nc" else fractional_step
        return rule(planet["p"], planet["k"], tax)

    for year in range(1, HORIZON + 1):
        tax = tax_at(policy, year)
        # Only home builds extra ETACs. Pay before income; commission next year.
        home = planets[0]
        budget = int(min(home["goods"], home["p"]))
        repair_bill = 0
        if repair_first:
            damaged, repair_bill = repair_rule(damaged, budget)
            home["goods"] -= repair_bill
            budget -= repair_bill
        while len(ships) < colonizers and budget >= etac_cost:
            home["goods"] -= etac_cost
            budget -= etac_cost
            ships.append({"coords": HOME, "ready": year + 1, "target": None, "arrival": None})
        if not repair_first:
            damaged, repair_bill = repair_rule(damaged, budget)
            home["goods"] -= repair_bill
        if repair_costs:
            repair_rows.append(dict(year=year, bill=repair_bill, damaged=len(damaged),
                                    etacs=len(ships)))

        # Affordability is an alternative use of unspent local reserves, not purchases.
        for planet in planets[1:]:
            available = int(min(planet["goods"], planet["p"]))
            for key, price in (("CA", 15), ("BB30", 30), ("BB45", 45), ("BB60", 60)):
                metric = "first_colony_can_order_" + key
                if metrics[metric] is None and available >= price:
                    metrics[metric] = year

        incomes = []
        for planet in planets:
            planet["p"], income = step(planet, tax)
            planet["goods"] = min(65535, planet["goods"] + income)
            incomes.append(income)
        if metrics["colony_income_reaches_home"] is None and sum(incomes[1:]) >= incomes[0]:
            metrics["colony_income_reaches_home"] = year

        # Each ship selects its nearest unreserved world; quality does not affect orders.
        # It cannot receive a new player order in the middle of a maintenance year.
        for ship in ships:
            if ship["target"] is None and ship["ready"] <= year and targets:
                target = min(targets, key=lambda t: (math.dist(ship["coords"], t[0]), t[0]))
                targets.remove(target)
                ship["target"] = target
                ship["arrival"] = year + travel_years(ship["coords"], target[0], speed) - 1
            if ship["target"] is not None and ship["arrival"] == year:
                coords, potential = ship["target"]
                planets.append({"coords": coords, "p": 1, "k": potential, "goods": 0, "home": False})
                ship.update(coords=coords, ready=year + 1, target=None, arrival=None)
        for key, count in (("first_colony", 1), ("four_colonies", 4), ("eight_colonies", 8)):
            if metrics[key] is None and len(planets) - 1 >= count:
                metrics[key] = year

        if year in (5, 10, 20, 25):
            rows[year] = {
                "colonies": len(planets) - 1,
                "production": sum(p["p"] for p in planets),
                "annual_income": sum(incomes),
                "local_reserves_sum": sum(p["goods"] for p in planets),
            }
    result = {"milestones": metrics, "snapshots": rows}
    if repair_costs:
        result['repair'] = repair_rows
    return result


def distribution(values, trials):
    reached = sorted(v for v in values if v is not None)
    result = {"reached": len(reached), "trials": trials, "reach_fraction": len(reached) / trials}
    if reached:
        result.update(
            mean_when_reached=statistics.mean(reached),
            median_when_reached=statistics.median(reached),
            p10_when_reached=reached[max(0, math.ceil(0.1 * len(reached)) - 1)],
            p90_when_reached=reached[math.ceil(0.9 * len(reached)) - 1],
        )
    return result


def sweep(trials):
    cases = []
    configs = [
        {"model": model, "policy": policy}
        for model in ("classic", "nc", "fractional")
        for policy in ("50", "65", "35_then_65")
    ]
    configs += [
        {"worlds": worlds} for worlds in (12, 48)
    ] + [
        {"colonizers": 4, "etac_cost": cost} for cost in (15, 20, 30)
    ] + [
        {"speed": speed} for speed in (2, 4)
    ]
    for override in configs:
        config = dict(worlds=24, model="nc", policy="50", colonizers=2, etac_cost=20, speed=3)
        config.update(override)
        runs = [simulate(SEED + i, **config) for i in range(trials)]
        milestones = {
            key: distribution([r["milestones"][key] for r in runs], trials)
            for key in runs[0]["milestones"]
        }
        snapshots = {
            year: {key: distribution([r["snapshots"][year][key] for r in runs], trials)
                   for key in runs[0]["snapshots"][year]}
            for year in (5, 10, 20, 25)
        }
        cases.append({"config": config, "milestones": milestones, "snapshots": snapshots})
    return {
        "source": NC_SOURCE, "seed": SEED, "trials_per_case": trials,
        "horizon_years": HORIZON,
        "assumptions": [
            "Synthetic 31x31 square; home center; uniform empty cells and integer potential 10..150.",
            "Home starts P=K=100, goods=50, matching NC's default joined treasury; shared across models.",
            "These are sensitivity inputs, not the distribution of NC's generated maps.",
            "Two starter reusable ETACs. Optional home-built extras cost money and one commissioning cycle.",
            "All target locations known; nearest unreserved target; no quality optimization or scouting.",
            "Full-orbit Euclidean travel plus two depth units, rounded to ticks and player-order years.",
            "No classic real-number rounding, end-year half-depth colonization shortcut, or route avoidance.",
            "No enemies, battles, capture, repair, upkeep, base growth, or new colony ship purchases off home.",
            "No military spending; local reserve sums cannot be spent centrally.",
            "Milestone dates are order affordability, not commissioning or arrival of combat ships.",
            "Quantiles describe synthetic-map variation; unreached milestones are reported separately.",
            "Same-density cases share maps. Classic also samples economy rolls; no engine RNG is used.",
        ],
        "cases": cases,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--trials", type=int, default=1000)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if args.trials < 1:
        parser.error("--trials must be positive")
    rendered = json.dumps(sweep(args.trials), indent=2) + "\n"
    if args.output:
        args.output.write_text(rendered)
    else:
        print(rendered, end="")


if __name__ == "__main__":
    main()
