---
id: 3htuxw8w
date: 2026-10-10
kind: adr
status: accepted
tags: [adr, economy, conquest]
---
# Captured worlds convert in two quiet years and keep what survives

## Context

A captured world's status steps 0, 1, 2 over two maintenance passes, as in
ECMAINT. Classic pays no income at status 0, half at status 1, and full at
status 2; NC pays none until status 2. Classic capture also damages the
world by mission (Invade keeps 20-40% of factories, Blitz 85-100% scaled by
the survivors), halves production and goods under Scorch-Earth, and floors
production at 1.0 (`110a:6edd`). This is item 4 of the economy review
(t051).

## Decision

The owner approved Claude and Codex's joint recommendation on 2026-10-10.

1. **Conversion.** A world captured in pass N produces no income in N+1
   and N+2 and full income from N+3 (NC's rule, not classic's half-income
   second pass).
2. **Inheritance.** Whatever survives the assault (stored goods,
   production, docked ships, and build-queue progress) passes intact to
   the new owner. There is no capture bonus and no debt forgiveness.
3. **Screens and reports.** No new screens. Capture reports list the
   inherited goods, ships, and work. The cancel-build screen's "fully
   refunded" wording is corrected: cancelling releases unpaid commitments,
   not points already spent.
4. **War costs.** No upkeep and no repair bills. War costs remain
   permanent losses, recovery time (RT 2), travel, and lost worlds.
   Bombardment rates and Scorch-Earth are item 5.

## Alternatives

- Classic's half-income second pass: adds 25 points over ten passes at
  production 100 and 50% tax (425 against 400), while captured reserves
  already fund the next pass's builds.
- A capture bonus or forgiving a captured world's debt: rewards conquest
  beyond the assets that survive.

## Consequences

A captured world holding 50 goods finishes a cruiser that still owes 12 in
the next pass; with no goods it finishes in N+4 (N+3 under the half-income
option). Implementation is pending, with the economy work (t084): capture
keeps surviving assets and queue progress, the inherited-assets lines in
capture reports, and the cancel-screen wording.

## Acceptance and connections

Accepted by the owner on 2026-10-10. Evidence: Codex's controls in
[et03gvm9](../investigations/et03gvm9-conquest-income-inherited-assets-and-bombardment-economics.md)
and `tools/economy-model/conquest_review.py`; classic capture in
[ec-battle-spec.md](../../dev/ec-battle-spec.md) §2.9 and
[ec-economy-reference.md](../../dev/ec-economy-reference.md).
