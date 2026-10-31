---
id: bd8trz18
date: 2026-10-31
kind: adr
status: accepted
tags: [adr, combat, participation, engagement, orbital-gate]
---
# Battles pair one fleet with one target as classic does

## Context

[gzly58c5](gzly58c5-fleets-of-one-empire-fight-together-and-first-contact-obeys-roe.md)
items 1 to 3 made each side of a battle an empire's fleets in the sector and
layer, and t080 extended the orbital gate to the owner's fleets with
warships in orbit. In the owner's ENiGMA game an assault on (4,7) killed a
starbase and a BB1 DD1 fleet in one battle, and the two loss notices read
as two engagements with the same numbers. The owner judged the merge wrong
in play terms: a player who leaves fleets apart has chosen to, or made a
mistake to learn from, and the engine should not consolidate for him. Time
(the tick), sector and layer put units together; in classic a unit's own
sighting, or the enemy's sighting of it, puts it in a fight.

## Decision

The owner chose classic's pairing on 2026-10-31, the orbital gate included.

1. **Pairing.** A battle is the attacking fleet alone against its target:
   a fleet with the base it guards at rest, or a base with the guard
   fleets at rest beside it (`guard_fleet_of`). No other fleet of either
   empire joins. Each fleet engages through its own sighting and intercept;
   alerts still raise the empire's awareness, so a fleet that sights the
   alien later goes straight to its decision.
2. **ROE.** The attacker weighs its own effective NC attack strength
   against its target's side (decision 2's yardstick, a starbase at 100).
3. **Orbital gate.** Only the world owner's starbase in orbit, with its
   guard, fights an attacker at the strike tick, sighted or not, as
   classic's `110a:67bd`. His other fleets in orbit meet the attacker only
   through the encounter loop, so a raider may strike past an unalerted
   fleet. A transport-only blitz passes any orbit without a starbase.
4. Kept from gzly58c5: first contact obeys ROE (item 4), a refused trigger
   is cancelled, starbases never volunteer, battles stay two-empire, and
   in-battle withdrawal under each fleet's ROE.

## Alternatives

- Keep joined sides (gzly58c5): removes the split penalty (two 5 CA fleets
  against 10 CA win 50% joined against 8 to 10% one at a time), but takes
  fleet composition out of the player's hands and makes one battle read as
  two in the loss notices.
- Join only fleets already aware of the enemy (alert-driven joining):
  rejected for classic's simpler rule.
- Keep NC's gate but fight each orbiting fleet in its own battle: the
  owner chose classic's gate.

## Consequences

Splitting fleets costs what it cost in classic; Join and Guard Starbase are
the player's tools to consolidate. The battle engine's rosters, `prune`
and loss sharing stay, since a base and its guards are still several
records on one side. The serial alert probes (t225) show the gate change
directly: classic's raider bombards past a Hold fleet in 29 of 30 runs,
NC's old gate in 0 of 40.

## Acceptance and connections

Accepted by the owner on 2026-10-31 ("go back to classic's combat pairing",
with classic's gate). Supersedes gzly58c5 items 1 to 3 and the t080 gate
extension. Task t226. Normative text: the player manual's battle and
orbit sections, [ec-encounter-spec.md](../../dev/ec-encounter-spec.md),
[ec-combat-reference.md](../../dev/ec-combat-reference.md).
