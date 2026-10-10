"""t062: shared local budgets and peaceful expansion under a repair shock.
Historical rejected alternative: the owner selected free RT recovery.
"""
import json
import statistics
from pathlib import Path
from campaign import simulate, distribution
from repair import repair_batch, repair_installments


def main():
    trials = 1000
    rows = []
    for costs in ([], [15] * 9, [45] * 12):
        for first in (True, False):
            runs = [simulate(20261010 + i, colonizers=4, repair_costs=costs,
                             repair_first=first, repair_mode='installments') for i in range(trials)]
            rows.append(dict(damaged_build_costs=costs, repair_first=first,
                             four_colonies=distribution([r['milestones']['four_colonies'] for r in runs], trials),
                             eight_colonies=distribution([r['milestones']['eight_colonies'] for r in runs], trials),
                             colonies_year10=statistics.mean(r['snapshots'][10]['colonies'] for r in runs),
                             local_budget_years=runs[0].get('repair', [])[:5]))
    # Fixed local capacity: isolates the yard bottleneck, without growth or battles.
    yards = []
    for capacity in (2, 10, 50, 100):
        for costs in ([15]*9, [45]*12):
            for mode in ('whole', 'installments'):
                left = list(costs) if mode == 'whole' else [(c, 0) for c in costs]
                rule = repair_batch if mode == 'whole' else repair_installments
                years = []
                for year in range(1, 31):
                    left, bill = rule(left, capacity)
                    years.append(dict(year=year, bill=bill, damaged=len(left)))
                    if not left:
                        break
                yards.append(dict(capacity=capacity, costs=costs, mode=mode, years=years))
    Path(__file__).with_name('repair_pacing_results.json').write_text(
        json.dumps(dict(trials=trials, seed=20261010, repair_mode='installments',
                        campaigns=rows, fixed_yards=yards), indent=2)+'\n')
    for row in rows:
        print(sum(row['damaged_build_costs']), row['repair_first'],
              row['four_colonies'], row['colonies_year10'])


if __name__ == '__main__':
    main()
