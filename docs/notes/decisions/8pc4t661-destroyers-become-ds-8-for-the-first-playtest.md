---
id: 8pc4t661
date: 2026-10-10
kind: adr
status: superseded
tags: [adr, combat, hulls, balance]
---
# Destroyers become DS 8 for the first playtest

## Context

Under the approved fleet rules ([decision 1](ld2f252j-flat-combat-effectiveness-with-battle-local-carry.md))
destroyers at AS 10 / DS 5 / cost 5 were a trap: 30 DD lost to 10 CA of
equal cost about 90% of the time, and replacing 5 CA with 15 DD of equal
cost dropped a fleet against 10 CA from parity to 30%. Adding destroyers
made a fleet worse.

## Decision

The owner approved Claude and Codex's joint recommendation on 2026-10-10:
destroyers become **AS 10, DS 8, cost 5**. Every other hull, every price,
Recovery Turns (2), the starbase, ground batteries, and the planetary
support-fire rules stay unchanged for the first playtest. DS is a
combat-only stat with no classic file field, so the `.DAT` edge is
unaffected.

## Alternatives

- DS 6: still a trap in mixed fleets (15 DD + 5 CA against 10 CA at 35%).
- DS 10: destroyers match cruisers' AS and DS per point and add execution
  fire and speed, which makes cruisers obsolete (30 DD against 10 CA at
  50%).
- Cost 4 at DS 5: behaves much like DS 8 in battle but changes the economy,
  which belongs to the economy review (t051).

## Consequences

From Codex's 20,000-trial controls at equal build cost (t075): 30 DD
against 10 CA win 30.9% (about 10% before); 90 DD against 10 BB 23.4%;
15 DD + 5 CA against 10 CA 46.4%; 6 DD + 3 CA + 1 BB against 8 CA 56.3%.
The mixed +1 bonus supplies about 10 points of the last two; without it,
DS 8 alone raises them from 23.2% to 37.3% and from 37.2% to 46.1%.

Roles: destroyers become raiders and line support that improve a fleet,
not a cruiser replacement. Cruisers remain the better convoy escort: with
certain immediate pursuit, 15 DD keep 6.0 transports of 10 (3.5 before),
5 CA keep 7.8, and 6 DD + 3 CA keep 7.4. A destroyer's speed 6 does not
speed a convoy, which transports cap at 5. Starbases stay unchanged: 20 DD
against a lone base win 82.6%, against a base with 3 CA 28.2%.

A tail risk to watch: a 6 DD + 3 CA + 1 BB fleet at ROE 6 meeting four
fresh 8 CA forces three years apart without being caught wins more first
contacts (46.6% against 32.8%) and loses slightly less on average (58.1
against 60.7 of 120 points), but is wiped out more often over the four
(22.8% against 18.1%), because sturdier destroyers keep its ROE satisfied
longer. Under certain pursuit the wipe rate improves instead (85.6% to
82.4%). Sturdier destroyers also last longer against batteries; that and
campaign encounter rates go on the playtest watch list.

## Implementation (t078)

The engine's unit table has the destroyer at AS 10, DS 8 (`DS_DD` in
`nc-engine`'s combat state), so crippling one costs 8 hits and execution
fire pays 16 to destroy a nominal one; a docked destroyer in a stardock
costs 8 too. The combat spec's unit and stardock tables and the manual's
three ship tables (quick start, Forces, Appendix B) say 8. No existing test
pinned the old value, so none moved; a regression test pins the new step
costs. The Python model's `UNITS` table keeps DS 5 as the t055 baseline,
and its `dd-10-8-5` variant now describes the engine.

## Acceptance and connections

Accepted by the owner on 2026-10-10. Evidence: Claude's equal-cost matrix
and Codex's controls in
[wnwxz2et](../investigations/wnwxz2et-destroyer-durability-under-withdrawal-and-recovery.md)
and `tools/combat-model/hull_review.py`. Source synthesis:
[ebrogywq](../investigations/ebrogywq-nc-combat-design-review-against-its-sources.md).
Refines [NC combat stays](b5u6lekb-rust-keeps-its-own-combat-system.md).

Superseded on 2026-10-16 by [6vqrcldx](6vqrcldx-hulls-follow-a-punch-ladder.md): destroyers become AS 12 / DS 7.
