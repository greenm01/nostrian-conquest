---
id: almb7dbt
date: 2026-10-31
kind: adr
status: accepted
tags: [adr, combat, balance, doom-stacks, orbital-bombardment]
---
# A larger side commits part of its force, and batteries punish a crowded orbit

## Context

[e1w4xpes](../investigations/e1w4xpes-lanchesters-square-law-doom-stacks-and-the-orbit.md)
found that NC rewards doom stacks. Every ship fires every exchange, which is
Lanchester's square law. At a fortified world the batteries' column falls
as the stack grows, so overkill past about 2.5:1 costs nothing. In fleet
battles, suppression crippled every healthy hull before it destroyed any, so
a cruiser stack above about 1.5:1 was nearly immune. In the owner's crunch
game an AI fleet of CA33 DD153 rolled over worlds one at a time. The owner
wants to keep the game simple and classic in spirit, and to make large
fleets pay without forbidding them.

## Decision

The owner chose all three rules on 2026-10-31.

1. **Classic's engagement count in fleet battles.** A side with more AS
   than its enemy fires with `min(A, round(B·y))` AS, where
   `y = (A/B)^(1/3.3)`, damped to `(y + 1.3)/2` above 1.3 and capped at 2.0
   ([ec-battle-spec §1.3](../../dev/ec-battle-spec.md), fleet constants),
   counted in AS rather than whole ships. The pools still split by the
   side's total AS.
2. **The target-rich orbit.** Batteries firing at ships in orbit read one
   column better for each doubling of the ships' AS past three times theirs:
   at least Pressed from 3:1, Even from 6:1, Advantaged from 12:1. Ships'
   fire at the batteries, and every other planetary exchange, are unchanged.
3. **Dispersed suppression.** Within a class, suppression strikes nominal and
   crippled hulls in proportion, nominal first, one strike per DS and at most
   one per hull in a pass. A strike cripples a nominal hull or destroys a
   crippled one at full DS. A class the hits cannot pay for, crippled or
   not, shields the classes behind it.

## Alternatives

- One rule alone. The count alone leaves a 2:1 cruiser stack losing 3% of
  its cost; the target-rich orbit alone leaves fleet battles square-law.
- A Hughes-style fixed break-off point. Helmbold found fixed break points
  do not fit historical battles, and NC's ROE already weighs odds.
- Unit caps, stacking limits or fleet upkeep (Civilization V's one unit per
  tile). Rejected as nannying and unclassic.
- Price changes. Ship costs stay classic.

## Consequences

Who wins barely changes: the stronger side's win rate falls by at most 7
points, at 1.5:1, and the winner pays more. Against
60 cruisers, a cruiser stack's losses at 2:1 rise from 0.4% to 12% of its
cost, near the square law's 13%. Destroyer fleets and the AI's DD:CA mix
lose 12–30% more at 1.5:1 to 3:1. A fortress still falls to 1.5:1 or
more at least 96% of the time, but a stack of 25 times the batteries' cost
loses about two and a half times what one of five times loses. Smaller
sides survive exchanges they used to lose outright, so more battles end in
withdrawal and pursuit fire. The AI must be retrained on the new rules.

## Acceptance and connections

Accepted by the owner on 2026-10-31 ("Both", then "Adopt it" for dispersed
suppression after its simulation). Task t232. Normative text: the player
manual's combat appendix, [nc-combat-spec.md](../../dev/nc-combat-spec.md)
and [ec-combat-reference.md](../../dev/ec-combat-reference.md). Evidence:
[e1w4xpes](../investigations/e1w4xpes-lanchesters-square-law-doom-stacks-and-the-orbit.md).
