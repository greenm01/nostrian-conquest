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

Engine implementation is pending, after t039: the encounter assembly,
admission and pruning, allocation, the cancelled-trigger rule, the combat
spec, and the manual's "Who Fights Whom" and ROE sections. It supersedes
the "exactly two sides" clause of [the engagement ADR](gtkmxw3l-engagement-follows-the-classic-ecmaint-encounter-rule.md)
in the sense that each side is now an empire's assembled fleets, not one
fleet.

## Acceptance and connections

Accepted by the owner on 2026-10-10. Evidence:
[m1h272uh](../investigations/m1h272uh-joined-participation-surprise-and-allocation-fairness.md)
and `tools/combat-model/participation_review.py`. Source synthesis:
[ebrogywq](../investigations/ebrogywq-nc-combat-design-review-against-its-sources.md).
Related: [decision 2](kiswrzos-withdrawal-pursuit-and-one-strength-yardstick.md)
(its first-identification clause, superseded here),
[surprise](atdsvb5y-surprise-uses-local-sighting-memory.md).
