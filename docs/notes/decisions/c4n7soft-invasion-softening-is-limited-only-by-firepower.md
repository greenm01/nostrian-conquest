---
id: c4n7soft
date: 2026-11-04
kind: adr
status: accepted
tags: [adr, combat, invasion, armies]
---
# Invasion softening is limited only by firepower

## Context

Once an invasion clears the batteries, the fleet fires one exchange at the
garrison: armies only, 10 hits each. That softening could destroy at most
half the armies the world had when the assault began, so a real landing
battle was always left. The cap was NC's own; classic softens factories,
goods and the dock as well, with no such cap
([ec-combat-reference.md](../../dev/ec-combat-reference.md)).

In the x13 lab game, the owner's BB7 CA3 DD11 invaded Liturgia with four
armies. The world had raised 8 armies that year, so it held 17. Softening
stopped at 8, and the 9 left threw back the landing. The owner expected the
fleet's fire to concentrate on the batteries and armies before the landing,
however many it could kill.

## Decision

The owner chose on 2026-11-04 to remove the cap. Softening destroys as many
armies as its hits buy, up to the whole garrison; a world softened to
nothing falls to whatever lands. Suppression, the softening exchange and its
CRT column are unchanged. Task t259.

## Consequences

- A heavy fleet can take a lightly held world with a token landing. Armies
  still decide landings against fleets too weak to clear them.
- Armies still counter blitzes, which have no softening pass.

## Balance check (t259, 2026-11-04)

Measured on the Mac mini at 400 trials a case against t258 (534b6958).

- **The t254 attack-and-defence game** (`assault_odds --grid 400 --attack
  400|600|800`, `assault_game.py --boot 200`) barely moves. Capture values
  stay 0, 0.224 and 0.584 (0.583 before); the seen-defence best stays 0,
  0.297 and 0.955, with the same best defences. Siege cells change by under
  0.01 on average, at most 0.07. The grid's siege bombards until its armies
  outnumber the garrison before it invades, so the cap seldom bound there.
- **Direct invasions** (`assault_odds`'s fixed cases, 20 to 40 armies
  against 10 to 20) capture as often as before but lose far fewer armies:
  for example 10 BB and 40 armies against 4 batteries and 20 armies lose 0
  armies instead of 6.3. The gain is largest for a strong fleet landing few
  armies, as in the Liturgia report.

So the rule changes how cheaply a strong fleet takes a world, not who wins
the defence game. Raw output is in the mini's
`~/.local/state/nostrian-conquest/balance/t259/`.

## Acceptance and connections

Accepted by the owner on 2026-11-04.

- **Normative text:** the player manual's Invade mission, Attacking worlds
  and Invasion sections, and
  [nc-combat-spec.md](../../dev/nc-combat-spec.md) Invasion Priority.
- **Code:** `apply_invasion_soften_damage` in
  `rust/nc-engine/src/maint/combat/assault.rs`; test
  `canonical_invade_softening_is_limited_only_by_firepower`.
- **Related:** [kazzj70c](kazzj70c-batteries-pick-their-targets-at-random-among-the-warships.md),
  whose balance check this repeats.
