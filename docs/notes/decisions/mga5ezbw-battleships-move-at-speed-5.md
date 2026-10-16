---
id: mga5ezbw
date: 2026-10-16
kind: adr
status: rejected
tags: [adr, movement, hulls, balance, classic-deviation]
---
# Battleships move at speed 5

## Context

NC keeps classic EC's ship speeds: ETAC 3, battleship 4, cruiser and
transport 5, destroyer and scout 6, starbase 1 (`slowest_speed` in
`nc-data/src/fleet_ships.rs`). Classic governs movement, so any change is a
deliberate deviation.

The proposal came from the Mac mini's t189 AI trials: battleship-led seats
fell short of an even share at speed 4 at every price from 45 down to 20,
and scored 0.307 / 0.290 at speed 5. Empire of the Sun moves every naval
unit at 5, and a fleet moves at its slowest ship, so a battleship invasion
force is held to 4 while its transports could move 5.

## Decision

Rejected on 2026-10-16. Battleships stay at speed 4, and every speed stays
classic.

The design case decides it. Per build point against a cruiser, a
battleship is 7% stronger in battle, 11% more durable, and bombards at 1.5
times the rate
([hull balance investigation](../investigations/yqg056tf-nc-hull-balance-against-vbam-and-empire-of-the-sun.md)).
At the same speed and price it would beat the cruiser on every axis but
build size, and cruisers would have no role. Speed 4 is the battleship's
cost, as in classic EC: slow, heavy siege ships. VBAM sells extra speed as a
paid ability; Empire of the Sun's uniform speed works because its units
differ by function and scarcity, not price per point.

The owner set the principle: game balance comes from design analysis, and
AI results confirm it but do not drive it. The AI trials that prompted this
proposal measured an AI weakness. The executor sailed every force home
after each strike, so a slow force paid a long round trip every cycle. With
forward muster (`d59d1061`), battleship-led seats reach an even seat at
speed 4 (0.258 / 0.254), which confirms the classic ladder.

## Alternatives

- **Speed 5 at cost 45.** Makes cruisers redundant on the stats alone.
- **Speed 5 at a higher price (about 50).** Possible, but it trades a
  classic movement value for a price change with no design problem to fix.
- **Cheaper battleships at speed 4.** Unneeded; the value per point is
  already sound.

## Consequences

No rule change. Task t199 is withdrawn. Ship speeds stay as the player
manual's force table gives them. The mixed-fleet bonus removal
([r4hd41gi](r4hd41gi-mixed-fleets-get-no-combat-bonus.md), t198) is
unaffected.

## Acceptance and connections

Proposed and approved earlier on 2026-10-16, then rejected the same day in
a Claude session on crunch, on the design case above, once the forward-
muster result in the
[learned-AI plan](../plans/qkgqypzw-learned-ai-opponent-harness-hierarchical-agent-and-fleet-level-bootstrap-on-the-mac-mini.md)
(section t189) explained the AI trials. The
[movement spec](../../dev/ec-movement-spec.md) keeps the classic speeds.
