---
id: 59v787ut
date: 2026-10-10
kind: adr
status: accepted
tags: [adr, combat, ground, invasion]
---
# Defender-first ground combat on the shared table

## Context

NC's landing battle rolled both sides at once on the ratio-column CRT, with
a +1 column for the defender, and gave a mutual annihilation to the larger
force. Small edges decided too much: 10 attacking armies took a world held
by 10 about 8% of the time, 11 took it 83%, and at small scale any numeric
edge won outright (2 against 1, 3 against 2, and 5 against 2 all captured
every time). The tie rule alone caused much of the cliff: giving mutual
annihilation to the defender cuts 11 against 10 from 83% to 43%.
[Decision 1](ld2f252j-flat-combat-effectiveness-with-battle-local-carry.md)
moved fleet battles to the EotS 25/50/100 table with carried hits and left
planetary combat for this decision.

## Decision

The owner approved Claude and Codex's joint recommendation on 2026-10-10.
The landing battle runs in rounds:

1. The defenders fire first. Each side rolls once on the shared table:
   unmodified d10 0-2 gives 25%, 3-5 gives 50%, 6-9 gives 100% of its army
   AS.
2. Attacking losses come off; the surviving attackers fire back.
3. Damage short of eliminating an army carries, in quarter-army units, into
   the next round of the same landing battle, and is lost when the battle
   ends.
4. Rounds repeat until one side has no armies. A mutual annihilation cannot
   happen, because the attackers fire with what survives.

There is no extra defender modifier, no homeworld bonus, and no ground
critical; firing first is the defender's advantage. Reports show whole
armies lost and remaining; the carried fractions stay hidden, as the fleet
bank does.

Planetary support fire keeps its current rules: invasion suppression of
batteries, softening fire on armies, bombardment, and blitz landing fire.
Invade supplies its own softening once suppression has cleared the
batteries, so a separate Bombard order is not required first.

## Alternatives

- Today's rule: the cliff above.
- The same rule with mutual annihilation to the defender: removes part of
  the cliff (11 against 10 at 43%) but keeps the column jumps.
- Simultaneous independent rolls (0.30/0.35): mass removes upset risk; at
  1.5:1, three against two win 76% and thirty against twenty 99%.
- Simultaneous fire on the shared table with a +1 defender modifier: still
  jumps from 30% to 57% between 10 and 11 attackers, with 24% mutual
  annihilation at parity.

## Consequences

Capture odds follow the force ratio, roughly the same at every scale (an
approximation the measurements support, not a theorem; overwhelming ratios
still guarantee victory):

| Attackers vs defenders | Today | This rule |
| --- | ---: | ---: |
| 1 vs 1 | 0% | 33.6% |
| 2 vs 1 | 100% | 85.1% |
| 3 vs 2 | 100% | 55.6% |
| 3 vs 1 | 100% | 99.4% |
| 5 vs 5 | 8% | 22.6% |
| 10 vs 10 | 8% | 19.9% |
| 11 vs 10 | 83% | 27.6% |
| 15 vs 10 | 100% | 56.1% |
| 20 vs 10 | 100% | 91.8% |
| 20 vs 20 | | 19.0% |
| 40 vs 20 | | 92.3% |

A careful invasion costs a little more and loses less. For a 90% capture
of 10 armies after 3 CA of softening: 12 armies, 129 points committed, and
8.98 expected army points lost, against 9 armies, 108 committed, and 12.57
lost today. Without softening a reliable capture needs about 2:1. Blitz
needs a few more armies and loses fewer. The ratio property holds for the
ground battle, not the whole mission: support fire still uses the
planetary CRT (10 against 10 after 3 CA of softening captures 82.4%).

## Implementation (t071)

The engine's landing battle, invasion and blitz alike, follows the rounds
above. Damage is counted in quarter armies: a side with `n` armies deals
`n x 1`, `n x 2`, or `n x 4` quarters on the 25/50/100 table and four
quarters destroy an army, so carry is exact. The seeded roll derivation is
unchanged (same year, world, combat kind, round, and empires), and the 64-round
cap is gone because a landing ends within four rounds per landed army. Softening,
cover fire, and landing fire still read the ratio CRT. A sampled check of
20,000 landings per row through the production dice matches the model's exact
capture rate and mean survivors within 4.5 standard errors for 1 v 1, 3 v 2,
10 v 10, 15 v 10, and 20 v 10 (for example 20.0%, 56.5%, and 91.4% against
19.9%, 56.1%, and 91.8%), and 400 whole-year invasions and blitzes per row
agree too. The combat spec, the manual's Planetary Combat chapter, and
Appendix B describe the rule and carry the odds table.

## Acceptance and connections

Accepted by the owner on 2026-10-10. Evidence: Codex's exact ground and
invasion-price calculations (t068) in
[4lasivnr](../investigations/4lasivnr-ground-combat-and-recovery-turns-campaign-calibration.md)
and `tools/combat-model/`. Source synthesis:
[ebrogywq](../investigations/ebrogywq-nc-combat-design-review-against-its-sources.md).
Refines [NC combat stays](b5u6lekb-rust-keeps-its-own-combat-system.md).
