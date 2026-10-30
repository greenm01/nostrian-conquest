---
id: mk1ie4e3
date: 2026-10-10
kind: adr
status: accepted
tags: [adr, combat, assault, bombardment]
---
# Batteries shield the stardock and failed landings hold orbit

## Context

Bombardment let docked ships in a world's stardock absorb hits while its
ground batteries still stood, so the dock served as armor for the
batteries and docked ships died before the defences fell. The approved
decisions moved fleet battles ([decision 1](ld2f252j-flat-combat-effectiveness-with-battle-local-carry.md))
and the landing battle ([decision 4](59v787ut-defender-first-ground-combat-on-the-shared-table.md))
to the shared 25/50/100 table, and left planetary support fire, bombardment
damage, and the end of a failed assault to this decision. t052 already made
assaults resolve chronologically in their strike tick, with no class
priority and no annual ownership cap.

## Decision

The owner approved Claude and Codex's joint recommendation on 2026-10-10.

1. **Batteries shield the stardock.** An exchange that begins with
   batteries standing hits batteries only, and hits beyond what destroys
   them expire. Docked ships are exposed from the next exchange. In player
   terms: while a world has batteries, bombardment hits them first; ships
   in its stardock are exposed only once the batteries are gone.
2. **Planetary support fire keeps its current table.** Invasion suppression
   of batteries, softening fire on armies, bombardment, and blitz landing
   fire stay on the ratio CRT. Moving them to the flat table would roughly
   halve battery effectiveness, which is a change to batteries' role and
   belongs with hull and battery values (decision 7).
3. **Bombard keeps its industrial damage** at today's rates (1 production
   and 100 goods per 10 hits); Invade and Blitz preserve production.
   Reports state the damage plainly. The exact rates go to the economy
   review (t051).
4. **Failure is not defeat.** A fleet beaten in the orbital battle leaves
   under [decision 2](kiswrzos-withdrawal-pursuit-and-one-strength-yardstick.md)'s
   withdrawal rules, with no extra battery volley. A fleet whose landing
   battle fails, or whose Invade is blocked by surviving batteries, keeps
   its survivors in orbit on Hold with a failure report. It does not
   blockade automatically and is subject to ordinary encounters.

## Alternatives

- Today's order, with docked ships exposed alongside batteries: expected
  dock loss 33.5 points in the 6 CA against 3 batteries and a docked BB
  case, against 7.6 under the shield, with the attacker's losses unchanged
  (82.4 and 80.2).
- Moving support fire to the flat table now: attacker losses in that case
  fall from 80.2 to 41.4 points, and for 1 BB against 2 batteries from 22.7
  to 5.7.
- Treating every failed assault as a lost battle: would force a fleet that
  still holds orbit to retreat.

## Consequences

Docked ships are much safer while batteries stand, and destroying batteries
first is the only way to reach them. Bombard remains the order that damages
a world's economy; Invade and Blitz capture it intact. A fleet that fails a
landing stays where it is, and the player decides its next move.

## Implementation (t074)

A bombardment exchange now hits batteries only while any stood at its start,
and the excess expires; the first exchange that begins with none standing
reaches the stardock, and the round-3 breakthrough pays the dock before the
armies. Invasion suppression and blitz cover fire never reached the stardock.
The engine's mean losses for the 6 CA against 3 batteries and a docked BB case
over 1,000 whole-year trials (80.7 ship points, 7.1 dock points, 2.92
batteries, 2.33 armies, 0.48 production) match the model's shield order in
`tools/combat-model/assault_collateral.py` (80.2, 7.6, 2.93, 2.44, 0.49). The
failure rules needed no new code beyond tests: the orbital gate never involved
the batteries, and a failed landing or blocked Invade already ended on Hold
with its report. Bombard's industrial rates are unchanged here; the economy
task changes them.

## Amendment (2026-10-30)

The owner changed point 1 after a lone cruiser killed one army a year at an
undefended world: rounds 1 and 2 hit only an empty stardock and wasted their
fire. Every round now pours its hits down one order: batteries, stardock,
armies, then stored goods and factories. Batteries still shield what lies
behind them while any stands, but the hits left in the round that destroys
the last one go on down the order, and there is no separate round-3
breakthrough. The rule lives in `apply_planet_bombardment_damage`
(`rust/nc-engine/src/maint/combat/assault.rs`). The owner chose the full
spill over having batteries absorb the round in which they die.

## Acceptance and connections

Accepted by the owner on 2026-10-10. Evidence: Codex's exact bombardment
controls (t072) in
[twexjjwm](../investigations/twexjjwm-assault-collateral-and-planetary-support-after-the-ground-decision.md)
and `tools/combat-model/`. Source synthesis:
[ebrogywq](../investigations/ebrogywq-nc-combat-design-review-against-its-sources.md).
Refines [NC combat stays](b5u6lekb-rust-keeps-its-own-combat-system.md).
