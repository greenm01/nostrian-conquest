---
id: gtkmxw3l
date: 2026-10-10
kind: adr
status: accepted
tags: [adr, combat, movement]
---
# Engagement follows the classic ECMAINT encounter rule

## Context

[Engagement follows the 1992 player guide](mnb1phgg-engagement-follows-the-1992-player-guide.md)
took its rule from the 1992 player guide's wording. Task t034 then read
ECMAINT's encounter code and confirmed it with classic probes, after the
movement work found that fleets have a depth layer (see the
[movement investigation](../investigations/9msqq5e0-classic-move-orders-complete-on-entering-the-target-sector.md)).
Classic does not decide engagement by the visitor's order:

- Units meet only in the same sector and the same depth layer. A fleet is in
  a world's system when its depth is at least 0.5, whatever its order.
- A world owner's fleets in orbit attack a neutral fleet in the system:
  "Even though the alien fleet belongs to a neutral empire, it is within one
  of our star systems." Hold, View, Colonize, Join, and Seek Home fleets in
  orbit are attacked, and so are fleets climbing out of orbit.
- Declared enemies are attacked only in the same layer. An enemy at depth 0
  is not even sighted by a defender in orbit.
- Ground batteries and starbases never start a fight. A starbase only alerts
  its empire's fleets. A fleet attacks a starbase only if its empire has
  declared the base's owner an enemy, or the base guards its assault target.
- A Guard/Blockade fleet attacks fleets arriving in orbit to bombard, invade,
  or blitz, or to colonize an unowned world. It ignores other neutral fleets.
  An assault or colonize fleet attacks guards blocking its path, and aborts
  to Seek Home if its ROE forbids the fight. Two fleets colonizing the same
  unowned world fight.
- A fight does not change the empires' stored diplomacy.

Classic also decides each encounter by chance, each tick. An observer spots a
unit with a probability set by the observer's order (Hold 1, Guard/Blockade
5, Scout 5, Patrol 10), the target's visibility, and an awareness level that
rises after contact. An attack then needs an intercept, certain for a faster
attacker and a roll otherwise.

## Decision

Rust follows the classic rule above: who may engage whom, by sector, depth
layer, diplomacy, mission, and ROE. It keeps its yearly encounter model,
resolving encounters from positions and depths at the end of movement, with
no per-tick sighting or intercept rolls.

## Alternatives

- Port the per-tick sighting and intercept rolls as well. That closes the
  remaining gaps but needs a tick-level encounter loop and a large combat
  rewrite. It stays open as possible follow-up work.
- Keep the 1992-guide rule as a deliberate Rust design. Rejected: the binary
  is the acceptance oracle, and the guide's wording was a summary, not the
  rule.

## Consequences

Remaining differences from classic:

- Rust engages every allowed encounter in the year it arises; classic may
  take years to sight a fleet, or fail to intercept it.
- Rust checks positions at year end; classic checks every tick, so it can
  catch a fleet passing through a layer mid-year, such as one climbing out of
  orbit.
- The blockade departure strike (t021) was not examined in classic and stays
  as is until probed.

The player manual's "Who Fights Whom" section and the combat spec change with
the implementation. That work depends on the depth layer from t031.

## Acceptance and connections

Accepted by the user on 2026-10-10. Supersedes
[mnb1phgg](mnb1phgg-engagement-follows-the-1992-player-guide.md). Evidence is
in the t034 probe (code addresses `18ea:13cf`, `18ea:15c4`, `1b05:1277`,
`1b05:1d11`), summarized in the
[movement investigation](../investigations/9msqq5e0-classic-move-orders-complete-on-entering-the-target-sector.md).
The normative text will be the player manual's "Who Fights Whom" section and
[nc-combat-spec.md](../../dev/nc-combat-spec.md).
