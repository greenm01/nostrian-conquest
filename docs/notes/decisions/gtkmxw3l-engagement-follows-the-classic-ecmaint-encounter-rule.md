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

Rust follows classic exactly: who may engage whom, by sector, depth layer,
diplomacy, mission, and ROE, and also how encounters happen. Each year runs
the 52-tick loop: fleets in a random order, and for each fleet each tick, the
movement step, the encounter check with its sighting and intercept rolls, and
the order's arrival test. Rust matches classic's rules and odds; it does not
reproduce Turbo Pascal's random number sequence, so individual results match
in distribution, not roll for roll.

## Alternatives

- Follow the classic rule but keep a yearly encounter model, with no sighting
  or intercept rolls. Rejected: it would engage every allowed encounter in
  the year it arises and miss fleets caught mid-year, such as one climbing
  out of orbit.
- Keep the 1992-guide rule as a deliberate Rust design. Rejected: the binary
  is the acceptance oracle, and the guide's wording was a summary, not the
  rule.

## Consequences

Encounter resolution moves from year-end co-location into the tick loop, and
the movement stepper runs inside that loop instead of once per fleet per
year. Classic has no blockade departure strike: a blockader catches a fleet leaving
its world only through the normal sighting and intercept rolls while the
leaver is still in orbit, and only if the blockader's empire has declared the
leaver an enemy. The Rust departure strike from t021 goes. The full rule and
its constants are in [ec-encounter-spec.md](../../dev/ec-encounter-spec.md). Rust departs from classic in one place, with the user's approval on
2026-10-10, because classic has a bug there: in a sector with no world,
classic reads the world's owner through a garbage pointer. Rust treats such a
sector as nobody's system. Rust does not copy classic bugs.

The player manual's "Who Fights Whom"
section and the combat spec change with the implementation. That work depends
on the depth layer from t031.

## Acceptance and connections

Accepted by the user on 2026-10-10. Supersedes
[mnb1phgg](mnb1phgg-engagement-follows-the-1992-player-guide.md). Evidence is
in the t034 probe (code addresses `18ea:13cf`, `18ea:15c4`, `1b05:1277`,
`1b05:1d11`), summarized in the
[movement investigation](../investigations/9msqq5e0-classic-move-orders-complete-on-entering-the-target-sector.md).
The normative text will be the player manual's "Who Fights Whom" section and
[nc-combat-spec.md](../../dev/nc-combat-spec.md).
