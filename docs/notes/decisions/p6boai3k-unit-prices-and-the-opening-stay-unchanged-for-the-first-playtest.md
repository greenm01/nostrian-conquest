---
id: p6boai3k
date: 2026-10-10
kind: adr
status: accepted
tags: [adr, economy, prices, opening]
---
# Unit prices and the opening stay unchanged for the first playtest

## Context

The last item of the economy review (t051) asked whether unit prices or the
joined empire's opening needed changes after the approved accounting,
growth, starbase, conquest, bombardment, and combat decisions.

## Decision

The owner approved Claude and Codex's joint recommendation on 2026-10-10:
change nothing now.

1. **Prices:** DD 5, CA 15, BB 45, SC 15, TT 5, ETAC 20, starbase 50,
   army 2, battery 20. Destroyers keep the approved DS 8
   ([8pc4t661](8pc4t661-destroyers-become-ds-8-for-the-first-playtest.md)).
   No ship-unlock eras.
2. **Opening:** a joined empire's homeworld at production and potential
   100, 50 goods, 50% tax, 10 armies, 4 batteries, with 2 CA, 2 ETAC, and
   2 DD. ECUTIL's unjoined seed (35 goods) is a separate baseline.
3. **Re-test after implementation:** once the economy decisions are in the
   engine (t084), test prices on real generated maps with hostile openings
   before changing any of them.

## Alternatives

- Joined opening cash of 35: delays a first battleship, starbase, or second
  extra ETAC by a pass.
- ETAC at 15 or 25: the same tested colony dates at 50 goods; only the
  reserve moves.
- BB at 30: a potential-50 colony could order one in pass 7 instead of 17;
  BB at 60: never without a starbase.

## Consequences

Expansion remains a real choice. Two extra bare ETACs (40 points) finish in
pass 1 and depart in pass 2; two cruiser-escorted ETACs (70) finish in
passes 1 and 2. On 1,000 paired peaceful synthetic maps at 50% tax the
mean pass to eight colonies is 7.52 with two bare extras, 7.79 with
escorted extras, and 11.13 with the starters only. Those maps have no
rivals, detection, or battles, so this is not evidence that unescorted
expansion is better; the first colony lands around pass 2.2 and colony
income passes the home world's around passes 6.3-7.2, as emergent stages,
not fixed eras.

## Acceptance and connections

Accepted by the owner on 2026-10-10. Evidence: Codex's controls in
[y95fnqkm](../investigations/y95fnqkm-unit-prices-and-opening-resources-under-the-approved-economy.md)
and `tools/economy-model/opening_review.py`. The economy review's other
decisions: [ubnjcchu](ubnjcchu-fractional-economy-accounting-smooth-growth-and-a-colony-head-start.md),
[iq6sfxde](iq6sfxde-starbases-keep-their-growth-and-capacity-bonuses-without-stacking.md),
[3htuxw8w](3htuxw8w-captured-worlds-convert-in-two-quiet-years-and-keep-what-survives.md),
[nrvey7jk](nrvey7jk-lighter-bombardment-of-goods-and-classic-scorch-earth.md).
