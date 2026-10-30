---
id: kiswrzos
date: 2026-10-10
kind: adr
status: accepted
tags: [adr, combat, roe, engagement]
---
# Withdrawal, pursuit, and one strength yardstick

## Context

NC combat locked ROE for three rounds, then let a side withdraw through one
compulsory exchange at the Pressed column. In a convoy probe (5 CA and 10
transports at ROE 6 against 10 CA) that left about one transport of ten.
Decisions to fight used classic's encounter weights (BB 21, CA 7, DD 2, a
starbase constant, and a +30 guard proxy), while battles used NC attack
strength, so a fleet could agree to a fight its own battle would call
hopeless, and the reverse. [Decision 1](ld2f252j-flat-combat-effectiveness-with-battle-local-carry.md)
settled the resolution table and [decision 3](p01cg7ll-rt-controls-automatic-ship-recovery.md)
made damage last, which made the yardstick question pressing: a damaged
fleet must look as weak as it is.

> Its first-identification clause (item 5's "classic's first-identification
> rule ... unchanged") is superseded on 2026-10-10 by
> [fleets of one empire fight together and first contact obeys ROE](gzly58c5-fleets-of-one-empire-fight-together-and-first-contact-obeys-roe.md):
> every attack now passes the assembled ROE test.

## Decision

The owner approved Claude and Codex's joint recommendation on 2026-10-10,
items 1 to 4 and item 5 separately.

1. **ROE per fleet after one compulsory exchange.** Every fleet checks its
   own ROE after each exchange from the first; the three-round lock goes.
   Unarmed survivors attempt escape automatically, including a fleet that
   has just lost its last armed ship.
2. **Pursuit replaces the compulsory withdrawal exchange.** A pursuer catches
   a fleeing fleet only when speed and contact allow it, and gets at most
   one successful pursuit exchange per opposing empire per escape episode.
   The episode belongs to the fleeing fleet and its departure sector and
   layer; it ends when the fleet actually leaves or receives a new order,
   and it persists across the year boundary. A fleet in an episode does not
   re-volunteer through classic's first-identification commitment. When both
   sides break off, nobody pursues. The immediate pursuit exchange inside a
   battle keeps that battle's carried hits; a pursuit on a later tick is a
   new encounter and starts at zero.
3. **Screening stays line-first during pursuit.** Auxiliaries are not exposed
   first because a retreat occurred. The free critical keeps its current
   exception (order DD, SC, TT, ETAC, CA, BB, SB, ignoring the screen), now
   documented rather than called an absolute screen; changing it needs its
   own approval.
4. **The 64-round cap** ends an encounter unresolved. Survivors keep their
   state, and nobody gains assault permission from it. The next tick's
   ordinary encounter rules decide whether they fight again; there is no
   contact-suppression state.
5. **One strength yardstick.** Attack willingness, in-battle ROE checks, and
   the orbital gate use effective NC attack strength: each hull at its
   current AS, crippled hulls at their crippled AS, and a participating
   starbase counted once at 100 (50 crippled). Classic's 2/7/21 weights and
   the +30 guard proxy go. Qualitative die bonuses do not inflate AS. ROE
   thresholds, sighting, intercept, and classic's first-identification rule
   (apart from item 2's suppression) are unchanged, and no fleet joins a
   fight it would not join today.

## Alternatives

- The three-round lock with the compulsory Pressed exchange: kept about one
  transport of ten in the convoy probe.
- Exposing auxiliaries first in pursuit: a cheaper loss bill (54.1 against
  68.3 build points) that hides a failed mission (5.7 against 7.8
  transports surviving under certain pursuit).
- Contact suppression after a capped battle: the cap fired in none of
  200,000 sampled battles under decision 1, and decision 3 makes a resumed
  fight start from the damage already done.
- Keeping classic's weights for the decision to fight, made damage-aware
  (crippled hulls at half weight): rejected for one yardstick shared by the
  decision and the battle.

## Consequences

Convoy probe, transports surviving of ten: 1.0 with the lock, 5.8 with ROE
after the first exchange, 7.8 when unarmed survivors also escape; under
certain immediate pursuit, 7.8 with line-first screening and 5.7 with
auxiliaries first; 9.9 with no catch. These are battle and immediate-catch
controls, not full chase odds; [7y0ypr6b](../investigations/7y0ypr6b-combat-withdrawal-and-persistent-damage-after-decision-1.md)
holds the evidence and its limits.

Item 5 changes who agrees to fight. Destroyers count a little more against
cruisers (1:3 instead of 2:7), a starbase deters a little less (about 3.3
cruisers instead of 4.3), BB to CA is unchanged, and damaged fleets look as
weak as they are. Three DD against one CA at ROE 6 now attack (30 against
30; classic weights gave 6 against 7), and four CA now attack a lone base
(120 against 100; classic gave 28 against 30). It supersedes the
willingness clause of [the engagement ADR](gtkmxw3l-engagement-follows-the-classic-ecmaint-encounter-rule.md):
classic still decides who may meet whom and when, but NC decides whether a
fleet agrees to fight. Encounter-parity tests and the classic mission sweep
will differ wherever ROE decides engagement; that difference is a recorded
NC deviation, not a bug. No campaign measure of how often decisions flip
exists yet.

## Implementation (t065)

The engine fights one compulsory exchange, then every side checks its ROE
after every exchange; a side whose ships have no attack strength left
escapes at once. A side with a starbase never breaks off (the base cannot
leave, and its guard fights with it until decision 6 restructures sides).
When one side breaks off and the other stays, the other pursues if it has
armed ships, has not had its exchange with this escape, and catches the
fleeing side: a pursuer whose slowest surviving class is faster catches for
certain, otherwise the intercept's speed roll decides, without the awareness
term, since the fleeing side is in contact and running. The pursuit exchange
is an ordinary exchange with die modifiers and carried hits; when both sides
break off nobody pursues. The 64-round cap ends the battle unresolved: no
winner, no retreats, no mission aborts, and no strike at the orbital gate.

The escape episode is `Fleet::escape` (`EscapeEpisode`: departure sector,
layer, and a bit set of the empires that have had their exchange), stored in
the `escape_*` columns of `snapshot_fleets`, hashed into the state hash when
present, dropped by the classic export, cleared when the fleet changes
sector or layer or gets a player, Join, or autopilot order, kept by a
detached fleet, and unioned by a merge. An attack on an escaping fleet by
an empire that has not had its exchange is that empire's pursuit: a new
encounter at zero carry whose compulsory exchange is the one it gets. An
empire that has had it cannot attack the fleet again there, and the
escaping fleet does not volunteer through first identification.

The yardstick is `encounter::rules::strength`, now the fleet's NC attack
strength, shared by the attack decision, the intercept's "fights anyway"
test, the orbital gate (each base at 100), and the in-battle check. Reports
say why a fleet left (its ROE, or no warships left: `UnarmedEscape`) and
whether it was caught (`PursuitFire`); a mission abort names the orbital
defence when the world's owner held the field, else the ships the mission
lost. Of the encounter tests only the two strength unit tests changed;
every classic-parity scenario kept its expectation under the new weights.

## Acceptance and connections

Accepted by the owner on 2026-10-10. Evidence:
[7y0ypr6b](../investigations/7y0ypr6b-combat-withdrawal-and-persistent-damage-after-decision-1.md)
and the models in `tools/combat-model/`. Source synthesis:
[ebrogywq](../investigations/ebrogywq-nc-combat-design-review-against-its-sources.md).
Refines [NC combat stays](b5u6lekb-rust-keeps-its-own-combat-system.md) and
partly supersedes [the engagement ADR](gtkmxw3l-engagement-follows-the-classic-ecmaint-encounter-rule.md)
(its willingness clause).

## Amendment (2026-10-30): fleets met en route carry on, and routes avoid contacts

The owner's live game: an unescorted transport fleet on Join met a one-
destroyer picket in empty space on its way, broke off under pursuit fire,
and was put on Seek Home. Re-ordered each year, it met the same picket and
lost a transport every time. The owner called the break-off right but the
Seek Home wrong: the fleet should carry on to join its host.

- **Carry on en route.** A fleet that loses the field or breaks off before it
  reaches its target sector keeps its order, on a moving order and with the
  ships its mission needs, at no more than its new top speed. It takes the
  pursuit first and stays in its escape episode. A fleet beaten at its own
  target, or one that lost the ships its mission needs, still seeks home.
  Owner's choice among en route only, every order, and non-combat orders.
- **Enemy contacts steer routes.** Each empire keeps the sectors where its
  units sighted or fought another empire's units, with the year, in the
  sighting memory (`snapshot_contacts`). Unlike the surprise memory it
  outlives the empire's presence, but it counts for two years of movement
  and then expires, since a sighted fleet may have been passing through.
  Routing adds a contact sector's cost beside known enemy worlds.
- **Every travelling order routes.** Bombard, Invade, Blitz, Guard/Blockade,
  Guard Starbase and Salvage now use the hazard-aware route on their
  approach, like Move and Join; the target sector itself is never avoided.

The player manual (Movement and Arrival, Withdrawal and Retreat) and the
combat spec (step 6, Retreat) carry the rules.
