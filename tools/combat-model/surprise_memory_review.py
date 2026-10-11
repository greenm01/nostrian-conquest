"""t073 exact unseen-contact controls; not a campaign or intercept simulation.

Stationary Hold targets, one continuously scanning observer, awareness zero
until first sighting. Formula mirrors encounter/rules.rs fleet_visibility
and sighting_percent for these small, non-wrapping fleets.
"""
import json
import math
from pathlib import Path


def unseen_probability(*, ticks, factor, layer, ca=0, dd=0, sc=0):
    visibility = (math.sqrt(30 * ca + 15 * dd) * (5 if layer else 1)
                  + math.sqrt(3 * sc)) / 52
    per_tick = min(1.0, visibility * factor / 100)
    return per_tick, (1 - per_tick) ** ticks


def controls():
    cases = [
        ('10 CA / Hold / deep', dict(ca=10, factor=1, layer=0)),
        ('10 CA / Patrol / deep', dict(ca=10, factor=10, layer=0)),
        ('10 CA / Blockade guard / orbit', dict(ca=10, factor=5, layer=1)),
        ('1 DD / Blockade guard / orbit', dict(dd=1, factor=5, layer=1)),
        ('1 SC / Blockade guard / orbit', dict(sc=1, factor=5, layer=1)),
        ('1 SC / Starbase / orbit', dict(sc=1, factor=25, layer=1)),
    ]
    rows = []
    for name, values in cases:
        for ticks in (1, 4, 52):
            p, unseen = unseen_probability(ticks=ticks, **values)
            rows.append(dict(case=name, prior_scan_ticks=ticks,
                             sighting_probability_per_tick=p,
                             unsighted_holdover_probability=unseen,
                             snapshot_suppresses_but_memory_allows=unseen))
    return dict(
        method='Exact independent sighting trials at awareness zero; no battles, movement, intercepts or additional observers. Both forces remain across the boundary. Memory means any prior sighting, not identification.',
        interpretation='Eligibility disagreement among these controlled holdovers, not ambush frequency or campaign frequency. Scouts cannot attack alone. Later sightings can eliminate the disagreement before an attack.',
        rows=rows)


if __name__ == '__main__':
    result = controls()
    Path(__file__).with_name('nc_surprise_memory_review.json').write_text(
        json.dumps(result, indent=2) + '\n')
    for row in result['rows']:
        print(f"{row['case']}: {row['prior_scan_ticks']} scans, "
              f"{100 * row['unsighted_holdover_probability']:.2f}% unseen")
