---
id: 7y0ypr6b
date: 2026-10-10
kind: investigation
status: investigating
tags: [investigation, combat, recovery]
---
# Combat withdrawal and recovery after decision 1

The owner chose automatic recovery through **RT — Recovery Turns**, a tunable
ship-class stat. Paid repairs, repair installments, and shipyard requirements
are rejected. The [accepted RT decision](../decisions/p01cg7ll-rt-controls-automatic-ship-recovery.md)
owns that policy. Recovery values and decision 2 remain open.

### t062

Compare withdrawal rules on the approved flat 0.625/carry/full-DS resolution
with today's mixed/base shifts and free critical. Discuss recovery without
silently selecting the old sweep's other candidate switches.

## Withdrawal evidence

[withdrawal_review.py](../../../tools/combat-model/withdrawal_review.py) and
[its results](../../../tools/combat-model/nc_withdrawal_review.json) retain
54 cases: nine controls across six encounters. Exact enumeration uses a
1,000-state limit; other rows use 10,000 trials with seed 20261010. Sampled
rows retain outcome and transport-survival intervals. The baseline is a
model of the specified t060 combination, not a completed engine comparison.

For 5 CA + 10 TT at ROE 6 against 10 CA, mean surviving transports are:

| Rule | TT surviving | At least 5 survive |
| --- | ---: | ---: |
| Three-round lock, existing withdrawal | 1.017 | 10.5% |
| ROE after first exchange | 5.773 | 60.0% |
| Also allow unarmed survivors to escape | 7.807 | 83.2% |
| No pursuit catch, with unarmed escape | 9.903 | 100.0% sampled |
| Certain pursuit, ordinary screening | 7.831 | 83.7% |
| Certain pursuit, auxiliaries targeted first | 5.696 | 63.5% |

These are survivors, not cargo delivered. Aux-first pursuit reduces the
lost-build-point bill from 68.27 to 54.14 while losing more transports.
A cheaper bill can hide a failed mission. Ten unarmed TT against three DD
have zero survivors under the original escape restriction, but average
8.33 after one compulsory exchange plus certain pursuit with escape enabled.

Claude and Codex recommend per-fleet ROE after one compulsory exchange,
automatic escape attempts by unarmed survivors, and speed/contact-gated
pursuit limited to one success per opposing empire per episode. Keep ordinary
line-first screening during pursuit. Today's free critical can pierce that
screen; do not describe it as complete protection. Any critical change is
separate. The aggregate-side model does not test per-fleet joining or exit.

An episode follows the fleeing fleet in its departure sector and layer,
ends on actual departure or a new order, and persists across year boundaries.
Suppress its voluntary first-identification commitment while withdrawing.
An immediate pursuit exchange inside the same encounter retains its hit
bank. A later-tick pursuit is a new encounter with zero bank. The sweep's
0/50/100% pursuit probabilities are immediate-catch sensitivity controls,
not movement simulations or fixed proposed catch chances. Fleet split and
merge must not replenish the episode's pursuit allowance.

The [cap probe](../../../tools/combat-model/cap_review.py) found **zero actual
cap exits in 200,000 sampled battles**, 10,000 for each of the 20 original
matchup/hull combinations under the decision-1 control. See
[nc_cap_review.json](../../../tools/combat-model/nc_cap_review.json).
This is not a proof of impossibility. We agree the cap should end an encounter
unresolved, preserve survivors, and grant no assault permission from AS
comparison. Later ticks use ordinary encounter rules; add no special cap
suppression state. Recovery does not occur between those encounters.

## Rejected paid-repair experiments

The [repair pacing experiment](../../../tools/economy-model/repair_pacing.py)
and [results](../../../tools/economy-model/repair_pacing_results.json) are
historical evidence only. Do not implement their spending or installment
rules. They tested artificial year-1 damage shocks across 1,000 identical
peaceful maps, retaining the two starter ETACs and seeking two extra ETACs.
They do not simulate contested campaigns or damage caused by actual wars.

With no shock, four colonies are reached at mean year 4.462. Repair-first
with nine damaged CA reaches them at 5.290; twelve damaged BB at 5.598.
The latter diverts 50, 50, then 35 local points and delays extra ETACs until
year 4. Builds-first avoids the expansion delay but delays recovery. These
results explain the rejected tradeoff; they do not evaluate free RT recovery.

The separate repeated-contact probe used 15 CA against 10, then five fresh
CA. Retaining crippled steps leaves 10.754 CA after the second encounter,
versus 13.040 when survivors are restored between encounters. This supports
testing recovery timing, but chooses neither an RT value nor a campaign
policy. It omits travel and the time between fights.

## Validation and remaining work

39 combat-model tests and 18 economy-model tests passed. No Rust edits or
cargo runs. The historical before-fix evidence and original sweep results
were not changed. Engine comparison awaits t060 and fresh combat_odds output;
the typed-state seed change means old engine output is not fresh evidence.
Test RT schedules in repeated-contact and invasion campaigns before assigning
class values. The former paid-repair tools must stay labelled as rejected
alternatives if retained.

## Connections

- [Decision-1 consultation](qjev5wpq-review-of-the-t055-combat-sweep-semantics.md#t058).
- [Source synthesis and owner decisions](ebrogywq-nc-combat-design-review-against-its-sources.md#decisions-for-the-user).
- [Economy investigation](tkce84f0-a-simple-nc-economy-informed-by-classic-ec-vbam-and-ec4x.md).
