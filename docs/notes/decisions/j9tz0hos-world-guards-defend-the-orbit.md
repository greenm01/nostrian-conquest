---
id: j9tz0hos
date: 2026-11-02
kind: adr
status: accepted
tags: [adr, combat, orbital-gate, guard, roe]
---
# World guards defend the orbit

## Context

[bd8trz18](bd8trz18-battles-pair-one-fleet-with-one-target-as-classic-does.md)
kept classic's orbital gate: only the world owner's starbase, with its
Guard a Starbase escorts, fights an attacker at the strike tick. Every
other fleet in orbit, Guard/Blockade a World included, meets the attacker
only through the encounter loop.

t243 measured what that does to a guard
([e1w4xpes](../investigations/e1w4xpes-lanchesters-square-law-doom-stacks-and-the-orbit.md#simulation-battleships-starbases-invasion-and-recovery)).
A Guard/Blockade fleet on ROE 8 against more than twice its strength
declines to attack. The attacker must then catch it, and at equal speed it
often slips away and sits in orbit, untouched, while the world is
bombarded. A 5-unit guard was cleared in 98% of games at twice its
strength but in only 49% at three times. Being more outgunned kept the
guard safer, at no cost. Classic's ROE table says a fleet that flees takes
fire as it goes; here it took none and stayed.

## Decision

The owner chose on 2026-11-02 to make world guards defend the orbit.

1. **The gate's defenders.** Before an attacker bombards, invades or
   blitzes a world, it fights the orbit's defence: the world owner's
   starbases in orbit, their Guard a Starbase escorts, and his
   Guard/Blockade a World fleets with warships in orbit, sighted or
   not.
2. **One battle.** The defenders form one side, as a starbase and its
   escorts already did. The owner chose this over one battle per guard
   fleet.
3. **ROE.** Unchanged. The attacker weighs its strength against the whole
   defence; if its ROE refuses, it aborts without a shot. In the battle,
   each defending fleet fights the first exchange and then breaks off,
   under pursuit fire, when its ROE refuses the odds, and seeks home.
4. **Unchanged from bd8trz18.** Fleets on Hold, Patrol or any other order
   still meet the attacker only through the encounter loop, so a raider can
   still strike past them. Elsewhere, battles still pair one fleet with
   one target. A guard escaping from a battle with this attacker has got
   away and does not join.

## Alternatives

- Keep the rule and document it: rejected; the outcome runs backwards and
  a guard that refuses a fight loses nothing.
- An evading guard leaves the world (seeks home) without being drawn into
  battle: costs the guard its station but not its ships, and still lets
  ROE work as a free escape.
- One battle per guard fleet, the attacker holding the field in each:
  keeps bd8trz18's pairing but lets an attacker beat split guards in
  detail; the owner chose one battle.

## Consequences

- Every empire's starting fleets are on Guard/Blockade at its homeworld,
  so an early raid on a homeworld now meets them at the orbit.
- A transport-only blitz is turned back by any Guard/Blockade fleet with
  warships in orbit, as by a starbase.
- The trained AI learned under the old gate. Its play under the new one is
  checked in t250's results.

## Acceptance and connections

Accepted by the owner on 2026-11-02 ("Guards defend the orbit", then "One
orbit battle"). Amends bd8trz18 item 3. Task t250. Normative text: the
player manual's Attacking Worlds, Guard/Blockade a World and battle
sections; [ec-combat-reference.md](../../dev/ec-combat-reference.md).
Code: `holds_orbit` and `add_world_guards` in
`rust/nc-engine/src/maint/encounter/handler.rs`; tests in
`rust/nc-engine/tests/combat/assault.rs`.
