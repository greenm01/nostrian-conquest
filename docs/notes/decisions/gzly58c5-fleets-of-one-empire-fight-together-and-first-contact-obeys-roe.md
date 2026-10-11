---
id: gzly58c5
date: 2026-10-10
kind: adr
status: accepted
tags: [adr, combat, participation, engagement, roe]
---
# Fleets of one empire fight together and first contact obeys ROE

## Context

Classic's encounter loop pairs one observer with one target, so an
empire's fleets sharing a sector and layer fought one at a time and paid a
heavy split penalty: two 5 CA fleets against 10 CA won 10% fighting
separately. Classic also lets any fleet with ROE above 0 attack at first
identification whatever the odds, an exception
[decision 2](kiswrzos-withdrawal-pursuit-and-one-strength-yardstick.md)
kept. Surprise, the other part of decision 6, is recorded separately in
[atdsvb5y](atdsvb5y-surprise-uses-local-sighting-memory.md).

## Decision

The owner approved Claude and Codex's joint recommendation on 2026-10-10,
items 1 to 3 together and item 4 separately.

1. **Joined sides.** A battle is still between two empires. Each side
   joins its empire's fleets in the battle's sector and layer. Each
   attacking volunteer tests its own ROE against the assembled sides'
   effective NC attack strength (crippled hulls at crippled AS); the
   defenders present form a fixed roster and take the compulsory first
   exchange. After exchange 1, both sides drop every fleet whose ROE fails,
   together and for good, recomputing until none fails; a dropped fleet is
   not re-added even if the odds later favour it. ROE 0 never volunteers
   but fights when attacked. Unarmed survivors escape automatically.
2. **Loss allocation.** Hit allocation still chooses the class, damage
   state, recovery cohort, and cargo subgroup it strikes; within that
   choice, losses are shared across the side's fleets in proportion to
   their eligible hulls by largest remainder (8 and 2 eligible cruisers
   share 3 losses as 2 and 1). Ties at the cutoff are broken by draws from
   the battle's seeded roll stream, without replacement, only when a tied
   group straddles the remaining losses; a completed allocation is never
   redrawn. The manual says ties are broken at random, and that splitting
   or merging fleets can shift a rounding loss by one hull per allocation.
3. **Who starts and who joins.** If admission removes the fleet whose
   sighting triggered the encounter, the trigger is cancelled; another
   willing fleet attacks only through its own contact and intercept check.
   The removed fleet's sighting still counts toward surprise memory; its
   catch is not inherited. Starbases never volunteer to attack; they join
   only as defenders under the existing target and orbital-gate rules.
   Pursuit stays per fleeing fleet and per opposing empire. With three
   empires in a sector, battles remain two-empire battles; others do not
   join.
4. **First contact obeys ROE.** A fleet no longer attacks merely because it
   identified an enemy first: every attack passes the same assembled ROE
   test. A fleet that is attacked still fights. This supersedes the
   first-identification clause decision 2 kept; classic hostility,
   sighting, and intercept are otherwise unchanged.

## Alternatives

- Classic one-pair battles: the split penalty above.
- Joined sides that test each fleet's own AS: two 6 CA fleets at ROE 6
  would each refuse 10 CA despite 360 against 300 combined, restoring the
  split penalty after round 1.
- Tie-breaks by roster order: deterministic but lets a player shape which
  fleet absorbs losses by detaching and re-forming fleets.
- Keeping the first-identification bypass: an attack could then skip the
  ROE test that every other attack must pass.

## Consequences

From Codex's 20,000-trial controls with destroyers at DS 8 (t073), joined
against one at a time: two 5 CA against 10 CA win 49.7% against 10.0%, and
lose 93.9 against 144.4 build points; two 6 CA 78.2% against 36.8%; two
fleets of 5 DD and 5 CA against 12 CA 70.6% against 23.0%, losing 88.2
against 182.1. These are imposed-contact battle odds; how often joining
changes outcomes in a campaign is not measured. A static audit found the
encounter test whose expectation changes with item 4.

It supersedes the "exactly two sides" clause of
[the engagement ADR](gtkmxw3l-engagement-follows-the-classic-ecmaint-encounter-rule.md)
in the sense that each side is now an empire's assembled fleets, not one
fleet. The engine work is done (see Implementation).

## Implementation (t080)

**Sides.** An encounter's defending roster is the target's empire's fleets
in the sector and layer, less any fleet escaping from a battle there that
the attacking empire has already caught once, with the starbases the target
rule brings: the base attacked, and the base each guard fleet at rest among
them guards (a base joins whenever its guard does, so "an attack on either
is a battle against both" still holds). The attacking side is the
observer's empire's fleets there that `rules::admitted` lets volunteer:
each with ROE above 0 and warships, dropped and recomputed until every
fleet left accepts the assembled odds. The observer's own decision
(`decide_vs_fleet`, `decide_vs_base`) is the same test, so when admission
removes the observer there is no attack and the report says its ROE
declined; its sighting and the awareness it raised stand. The intercept's
fallback, an unnoticed target choosing to fight, weighs the same assembled
sides. `Situation` carries the two strengths; `Mode` and the first
identification bypass are gone, and with them `FleetFacts::escaping`. The
orbital gate's attacking side is the striking fleet and its empire's other
fleets in orbit that volunteer; its defending roster is unchanged. Against
a starbase the attacker weighs the base's whole side in both cases, where
classic weighed a declared enemy's base alone (the code settled this under
"every attack passes the same assembled ROE test").

**The battle.** A `TaskForce` is a roster of `Member`s, one per fleet or
starbase record with its own hulls, and fires with the aggregate of the
members still in. After every exchange `prune` drops, on both sides at
once, every fleet whose ROE fails its side's strength against the other's
and every fleet with no attack strength left, rebuilds the aggregates, and
repeats until none fails; a dropped fleet is `left` for good. A starbase
never leaves; its guard leaves under its own ROE like any fleet, which the
code settled uniformly (t065's "its guard fights with it" was explicitly
until this decision). A ROE 0 fleet therefore fights the compulsory
exchange and then leaves. Fleets that leave while their side fights on
leave unpursued; when a whole side breaks off and the other stays, each
fleeing fleet is caught on its own (speed, one catch draw per fleet, and
not when the pursuer has had its exchange of that fleet's escape), and the
fleets caught stand for one pursuit exchange. The carry bank is per side
and pool; a departing fleet neither takes nor duplicates it.

**Allocation.** After each side's hits are applied to its aggregate,
`share_losses` spreads the change per class over the members in:
`damage::apply_class_change` shares each finished cohort among the
holders of that cohort, the nominal hulls destroyed among the holders of
nominal hulls, and the hulls newly crippled among those left nominal, each
by `apportion` (largest remainder; ties at the cutoff drawn without
replacement from `BattleDice::allocation`, the battle's seeded stream keyed
by exchange and empire, only when the tied group straddles the hulls left).
Transports are shared loaded-first across the side at every step, the
fleet record keeping one crippled count for loaded and empty transports
together: a fleet's loaded transports are read as its most damaged, the
first in the finishing order, which is also how a lone fleet's losses have
always fallen on its loaded transports first. The write-back puts each
member's hulls on its record; a strike, one fleet, writes its state back
directly (`write_back_strike`). Since t096 a fleet keeps a crippled
counter for each cargo state, and each fleet's loaded and empty transports
are separate holders; see
[p01cg7ll](p01cg7ll-rt-controls-automatic-ship-recovery.md).

**Tests.** Inline, with scripted dice: simultaneous irrevocable pruning,
ROE 0 fighting the first exchange, per-fleet pursuit, the carry staying
with the side, the 8 and 2 example in a battle, `apportion`'s example and
bounds, a seeded tie and its replay, and the loaded-first split. Whole
years over seeds (`tests/combat/participation.rs`): the model's three
joined cases, first contact obeying ROE, the cancelled trigger, bases never
volunteering, three-empire sectors. Changed expectations, each for the
same reason: two decision tests gave ROE 1 attackers who relied on the
first-identification bypass (now ROE 6); the guard in the classic
base-and-guard test and the defenders of the nc-cli destroyed-starbase
report fixture are on ROE 10 (a guard on a lower ROE now leaves after the
first exchange, and the base dies alone); `decide_vs_base` against a
declared enemy weighs the base's side, not the base alone; and the
replayed fuzz scenario `bombard-arrival-then-roe-abort` names empire 2's
main fleet as record 4, since its four guard fleets, joined, destroy the
lone destroyer of turn 3 that a single guard had let break off, and the
dead record is compacted away.

## Acceptance and connections

Accepted by the owner on 2026-10-10. Evidence:
[m1h272uh](../investigations/m1h272uh-joined-participation-surprise-and-allocation-fairness.md)
and `tools/combat-model/participation_review.py`. Source synthesis:
[ebrogywq](../investigations/ebrogywq-nc-combat-design-review-against-its-sources.md).
Related: [decision 2](kiswrzos-withdrawal-pursuit-and-one-strength-yardstick.md)
(its first-identification clause, superseded here),
[surprise](atdsvb5y-surprise-uses-local-sighting-memory.md).
