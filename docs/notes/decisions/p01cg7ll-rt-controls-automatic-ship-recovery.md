---
id: p01cg7ll
date: 2026-10-10
kind: adr
status: accepted
tags: [adr, combat, recovery]
---
# RT controls automatic ship recovery

## Decision

The owner approved **RT — Recovery Turns** as a tunable ship-class stat.
RT is the number of full game turns (years) a surviving crippled ship needs
to recover automatically to nominal, full strength.

Until recovery completes, the ship retains its crippled combat strength.
Recovery continues while moving. It requires no payment, repair order,
shipyard, owned planet, or local production capacity. Destroyed ships do
not recover. Splitting, transferring, or merging fleets preserves remaining
recovery time. It must never reset or accelerate recovery.

The maintenance year in which a ship is crippled does not count as a full
recovery turn. Count subsequent complete maintenance years. For RT 2, a
ship crippled during year N remains crippled through N+1 and recovers after
N+2 completes. Encounter count does not advance recovery.

Show the class's RT alongside its other stats. Fleet status and reports
show crippled counts grouped by remaining turns, and announce completed
recoveries. No new repair-management menu is needed.

## Scope

The automatic recovery concept and the RT name are approved. Numerical
RT values remain unselected. Uniform RT 2 and DD 1 / CA 2 / BB 3 are
experiments to compare, not accepted values. Set explicit values for the
other ship classes and consider starbase recovery separately when tuning.

The owner rejected paid repairs and shipyard management. The earlier
25%-cost and installment proposals are historical alternatives, not future
implementation work. The owner subsequently chose automatic recovery;
there is no accepted policy that all damage must end with the battle.

[Decision 1](ld2f252j-flat-combat-effectiveness-with-battle-local-carry.md)
still applies: crippled finishing costs full DS, and unused hit banks end
with the encounter. Recovery tracks completed crippling, not unused hits.
Withdrawal and pursuit remain a separate owner decision.

## Implementation proposals and remaining choices

Use counts by ship class and remaining recovery turns, rather than per-hull
objects. Merge adds matching counts. Bound the bucket range by the supported
RT values. Use a maintenance-start eligibility snapshot so newly crippled
ships cannot recover in their damage year.

For detaching crippled hulls, Claude proposes taking the longest remaining
recovery times first, leaving the retained fleet closest to full strength.
Codex agrees this is deterministic and explainable. This is an implementation
proposal, not an extra player choice. Document the default selection between
nominal and crippled ships as well; no operation may lose recovery metadata.

Reports should state, for example, “CA: 4 crippled, recover in 1 turn” and
“3 CA recovered to nominal.” Export behavior for recovery metadata still
needs an explicit compatibility contract: classic has no such field. The
consultation recommends exporting whole surviving hulls without changing
the SQLite state and disclosing that classic loses recovery state; this
compatibility recommendation has not been separately approved by the owner.

Engine implementation and manual updates are pending. Test recovery timing,
split/merge conservation, repeated encounters, and automatic recovery in
motion. Compare class RT values in campaigns before fixing them. This
approval does not authorize a crippled speed penalty or partial AS recovery.

## Evidence and acceptance

Approved by the owner on 2026-10-10 following the discussion of automatic
recovery and the name RT. See the [decision-2/3 investigation](../investigations/7y0ypr6b-combat-withdrawal-and-persistent-damage-after-decision-1.md)
for the earlier experiments and their limits. The
[combat spec](../../dev/nc-combat-spec.md) and player manual need updating
with implementation; this ADR does not claim the engine already recovers
ships over time.
