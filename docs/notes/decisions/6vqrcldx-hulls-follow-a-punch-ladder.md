---
id: 6vqrcldx
date: 2026-10-16
kind: adr
status: accepted
tags: [adr, combat, hulls, balance]
---
# Hulls follow a punch ladder

## Context

Every warship bought 2 AS per build point (DD 10/8, CA 30/30, BB 90/100),
so cruisers and battleships differed little in a fleet battle (a battleship
was 7% stronger per point) and only by role
([hull balance investigation](../investigations/yqg056tf-nc-hull-balance-against-vbam-and-empire-of-the-sun.md)).
The owner wanted smaller hulls to carry more punch per point and the
destroyer to be what it was always meant to be, a glass cannon. Space
Empires 4X follows the same pattern: smaller hulls carry more attack per
point, larger hulls more staying power.

## Decision

The owner approved on 2026-10-16:

| Ship | Cost | Speed | AS | DS | AS per point | DS per point |
| --- | --- | --- | --- | --- | --- | --- |
| Destroyer | 5 | 6 | 12 | 7 | 2.4 | 1.4 |
| Cruiser | 15 | 5 | 33 | 30 | 2.2 | 2.0 |
| Battleship | 45 | 4 | 90 | 110 | 2.0 | 2.44 |
| Starbase | 50 | 1 | 100 | 130 | 2.0 | 2.6 |

Punch per point falls and durability per point rises with size. Prices
and speeds stay as they were. Bombardment keeps its multipliers
(0.5x / 1x / 1.5x), so its per-hull weights become 6, 33, and 135. Scouts,
transports, ETACs, ground batteries, and armies are unchanged. This
supersedes the destroyer values of
[8pc4t661](8pc4t661-destroyers-become-ds-8-for-the-first-playtest.md) and
goes with the removal of the mixed-fleet bonus
([r4hd41gi](r4hd41gi-mixed-fleets-get-no-combat-bonus.md)).

## Alternatives

Measured on the combat model with the bonus off (investigation, "Proposed
punch ladder (t200)"):

- **DD 11/8, CA 33/30, BB 90/100.** Destroyers stay safe, but battleships
  lose their battle edge and fall behind cruisers (field-win 0.56 against
  0.65 at ROE 6).
- **DD 12/8.** An all-destroyer fleet beats cruisers (0.51) and battleships
  (0.56) at equal cost; rejected.
- **DD 12/7 with BB DS 100.** Battleships fall to the destroyer swarm's
  level (0.47).
- **Starbase DS 120 or 135.** At 120 a base is less durable per point than
  the new battleship; at 135 four cruisers (1.2 times its cost) win only 43%
  against it, against 52% at 130.

## Consequences

At ROE 6 with equal cost, cruisers and battleships hold the field about
equally (0.58 and 0.59), and battleships keep a 6% per-point battle edge
that pays for speed 4. Destroyers lose head to head to both (0.42) and keep
the best attrition ratio (1.24). A lone starbase beats equal-cost
attackers, falls about half the time to 1.2 times its cost in cruisers, and
nearly always to twice its cost; it now beats equal-cost destroyer raids
85% of the time.

The engine agrees with the model within 0.036 on every ROE 6 pairing of
the nine-fleet round robin (`combat_odds`, 4,000 trials each; see the
investigation's engine check). The starbase values were checked only on
the model.

## Acceptance and connections

Accepted by the owner on 2026-10-16 in a Claude session on crunch. Evidence:
the [hull balance investigation](../investigations/yqg056tf-nc-hull-balance-against-vbam-and-empire-of-the-sun.md)
and `tools/combat-model/nc_hull_ladder2_roe6.json`,
`nc_ladder2_breakeven.json`, and `nc_sb_check.json`. The player manual's
force table and the [combat spec](../../dev/nc-combat-spec.md) carry the
contract.
