---
id: nrvey7jk
date: 2026-10-10
kind: adr
status: accepted
tags: [adr, economy, bombardment, scorch]
---
# Lighter bombardment of goods and classic Scorch Earth

## Context

Bombardment destroyed 100 goods and 1 production per 10 hits reaching a
world's industry: one unopposed cruiser took 441 of a 500-goods treasury in
a pass. NC's Scorch Earth zeroed a world's potential, production, goods,
build queue, and dock at once, a permanent wipe, while classic's decays the
world each year it is set and leaves it recoverable. This is item 5 of the
economy review (t051); [decision 5](mk1ie4e3-batteries-shield-the-stardock-and-failed-landings-hold-orbit.md)
handed it the bombardment rates.

## Decision

The owner approved Claude and Codex's joint recommendation on 2026-10-10.

1. **Bombardment** destroys 25 goods and 1 production per 10 hits that
   reach industry. The support-fire table, the battery shield, and the
   breakthrough rules are unchanged.
2. **Scorch Earth follows classic.** The order stays set until the player
   rescinds it or the world is captured. Ordered this turn, it first takes
   effect in the next pass, after builds and before the 52 ticks, so it is
   already in force when an assault strikes that pass. Each pass while set:
   production keeps 30-49% of itself (`0.30 + Random(20)/100`), floored at
   1; positive goods become `min(G/10, sqrt(G))`; the world neither grows
   nor earns income. Potential is preserved.
3. **Capture of a scorched world** halves production and positive goods,
   empties the dock, clears the order, and floors production at 1.0, as
   ECMAINT does (`110a:6e6a..6f02`). Negative goods (debt) are untouched,
   both by the yearly decay and on capture. Otherwise item 4 applies
   ([3htuxw8w](3htuxw8w-captured-worlds-convert-in-two-quiet-years-and-keep-what-survives.md)):
   two quiet passes, surviving unfinished queue progress kept; the flagged
   capture's dock loss is that decision's one inheritance exception. NC
   does not adopt classic's ordinary per-mission capture multipliers. A
   world with potential 0 stays at production 0.
4. **Controls.** The existing Scorch command sets and rescinds the order
   with a short confirmation; reports say when it is active and when it
   ends; the revenue forecast shows 0 while it is set. No new screen or
   column.

## Alternatives

- 100 goods per 10 hits: one cruiser raid empties a treasury (production
  stays about 95%).
- 10 goods per 10 hits: one cruiser takes 49 of 500, too weak to matter.
- NC's immediate wipe: removes the conquest prize permanently and leaves a
  world that can never recover.

## Consequences

One unopposed cruiser takes about 123 of 500 goods (2.5 passes of a mature
world's income at 50% tax); a battleship 473. Repeated raids still hurt: a
cruiser against a world defended only by 10 armies destroys 364 goods over
five uncontested years; a battleship 626 counting later income, leaving
production 16.7. Raiding a defended world costs the raider: 3 BB against a
starbase, 3 batteries, and 10 armies lose 57.5 points on average, 85.8 with
3 guard cruisers added (conditional on reaching orbit). A production-100
world with 500 goods, scorched one pass and then captured, is left with
production 15-24.5 (mean 19.75) and 11.18 goods, and takes 18-19 passes to
rebuild to 90% at 50% tax without a base.

## Implementation

The Scorch order, its yearly decay, the confirmations, reports, and forecast
are done in t084. The Scorch command sets the planet's `scorch_earth` flag at
once and wipes nothing; the same command rescinds it, after three
confirmations to set and one to rescind. Each yearly pass
(`maint/economics/planets.rs`), after builds and before the ticks, decays a
flagged world in place of growth and income: production keeps
`0.30 + Random(20) / 100` of itself, at least 1 and at most the potential;
positive goods become `min(G / 10, sqrt(G))`; a debt and the potential are
left alone. `Random(20)` comes from the campaign seed, one draw per world per
year (`RNG_TAG_ECONOMY`). The pass also applies to a converting world. The
owner's maintenance report says each year that the order is in force, what it
cost, and that it stays until rescinded. The separate pending-orders set
(`planet_scorch_orders`) is gone; the flag on the planet is the one source.
`economy_model.rs` checks the decay against `scorch_review.py`: all twenty
factors, both goods branches, debt untouched, the floor, and recovery after
rescinding. The bombard rate and capture handling belong to the planetary
lane.

## Acceptance and connections

Accepted by the owner on 2026-10-10. Evidence: Codex's controls in
[wrk1xd3p](../investigations/wrk1xd3p-bombardment-raid-costs-and-recoverable-scorch-earth.md)
and `tools/economy-model/raid_review.py`, `scorch_review.py`; classic
Scorch in [ec-economy-reference.md](../../dev/ec-economy-reference.md) and
[ec-battle-spec.md](../../dev/ec-battle-spec.md) §2.9.
