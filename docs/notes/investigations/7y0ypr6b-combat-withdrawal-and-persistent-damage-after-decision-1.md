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

## RT timing and avoiding micromanagement

The owner asked us to proceed with strategic recovery that uses existing
fleet orders and preserves BBS layouts. Claude owns the ADR timing contract.
The [cohort model](../../../tools/combat-model/recovery.py),
[regression tests](../../../tools/combat-model/test_recovery.py), and
[timing results](../../../tools/combat-model/nc_recovery_timing.json) check
that contract independently of the combat odds model. These are counts by
class and recovery cohort, not individual ship objects.

The separate fresh-damage count is equivalent to an RT+1 sentinel for
ordinary recovery. With starbase assistance, explicit fresh-damage exclusion
prevents an RT 1 hull healing during its damage year. The model keeps bonus
eligibility with the qualifying hull counts through splits and merges;
a late arrival cannot acquire its host's year of eligibility. Departure or
loss of the qualifying base clears assistance, not ordinary recovery time.
The movement layer must establish continuous eligibility; this model does
not simulate that layer.

| RT candidate | Full recovery years away from a base | With uninterrupted starbase assistance |
| --- | ---: | ---: |
| 1 | 1 | 1 |
| 2 | 2 | 1 |
| 3 | 3 | 2 |
| 4 | 4 | 2 |

Every row excludes the damage year. These are timing examples, not selected
class stats. The base bonus is useful for RT 2 or more but cannot accelerate
RT 1. Thus a DD 1 / CA 2 / BB 3 experiment also changes the relative value
of bases for each class. Tune these together, rather than adding the bonus
after selecting recovery values in isolation.

No player needs to schedule individual repairs. Ordinary orders continue;
moving leaves the ordinary timer intact. Remaining at a base is an existing
positioning choice. Split-and-remerge produces exactly the same recovery
as leaving the fleet intact. Destroyed hulls cannot recover, and the fresh
cohort is removed first when a crippled casualty must be selected. Equal-age
assisted/unassisted tie handling in the model is bookkeeping, not an approved
new player control.

Validation: all 48 combat-model tests pass, including nine new recovery
probes. They cover damage-year exclusion, base assistance, departure,
late merges, split/merge conservation, casualty selection and the equivalence
of fresh flags and RT+1 for ordinary healing. No Rust or BBS UI change was
made. Continuous base eligibility, store round-trips, export disclosure and
actual door rendering still require engine implementation checks. Recovery
class values remain open; the timing model does not establish campaign
balance or justify any new menu.

## t060 engine comparison at 28814c9d

The [fresh engine output](../../../tools/combat-model/nc_engine_odds_t060.json)
was supplied by Claude from main `28814c9d`, after the typed-state seed change:
20,000 trials per case, map seed 1515, campaign seeds starting at 1, and up
to six maintenance years to obtain a battle. Its SHA256 is
`15a946e3704ad5f245f82a09f59b2dfb7e725c3f8779b7339fde06e58c85ae32`.
The copied file is unchanged. Codex ran the Python comparison, not a new
Rust or classic-oracle run.

Added `engine-t060`: flat mean 0.625, carry, full DS, current mixed and
present-base die shifts capped at +2, natural-9 free critical, ordinary
line-first screening, lock3, and an unmodified flat withdrawal exchange.
Critical order is fixed at DD, SC, TT, ET, CA, BB, SB, crippled first,
ignoring the screen. The former DS-sorted model order happens to agree at
the standard values; explicit order keeps future DS tuning from changing it.
When both sides fail the pre-round-4 ROE check, the engine still exchanges
fire once. The new preset reproduces this through
`mutual_withdrawal_exchange=true`; old presets retain their historical
behavior. Their evidence was not rewritten to hide the difference.

The engine selector `--engine-rules engine-t060` prevents comparison against
the CRT-era `current` preset. The [README reproduction command](../../../tools/combat-model/README.md#t060-engine-comparison)
produces [JSON](../../../tools/combat-model/nc_t060_comparison.json) and
[Markdown](../../../tools/combat-model/nc_t060_comparison.md) results.
The model uses exact enumeration up to 100,000 state/round pairs, then
20,000 seeded trials. All nine comparisons pass the existing four-outcome
checks and the destroyed-build-points tolerance on both sides.

| Matchup | Model left win | Engine left win [95%] | Comparison |
| --- | ---: | ---: | --- |
| 10 CA vs 10 CA | 49.4% [48.7, 50.1] | 49.7% [49.0, 50.4] n=19998 | yes |
| 11 CA vs 10 CA | 65.2% [64.6, 65.9] | 65.8% [65.1, 66.4] n=20000 | yes |
| 15 CA vs 10 CA | 96.8% [96.6, 97.1] | 96.8% [96.6, 97.1] n=20000 | yes |
| 20 CA vs 10 CA | 100.0% | 100.0% [100.0, 100.0] n=20000 | yes |
| 30 DD vs 10 CA | 9.8% | 9.7% [9.3, 10.1] n=20000 | yes |
| 90 DD vs 10 BB | 5.4% [5.1, 5.7] | 5.5% [5.1, 5.8] n=20000 | yes |
| 30 CA vs 10 BB | 40.3% [39.6, 41.0] | 40.3% [39.7, 41.0] n=20000 | yes |
| 29 CA + 3 DD vs 30 CA | 55.0% [54.3, 55.7] | 55.5% [54.8, 56.2] n=20000 | yes |
| 10 CA ROE 6 vs 15 CA | 2.5% | 2.4% [2.2, 2.7] n=20000 | yes |

These checks detect no mismatch in the tested combat-line fights. They are
not proof of equivalence. The harness does not cover auxiliaries, bases,
recovery, a two-sided ROE failure, or the guardrail. The two-sided withdrawal
and fixed critical order have targeted model regression tests and were
checked against engine source; they still need dedicated engine cases.
The 10-versus-10 case has 19,998 battles from 20,000 trials; two trials did
not obtain a battle within the horizon, and probabilities are conditioned
on those that fought. No engine stalemates occurred. The t059 retreat-home
change concerns routing, which this battle model does not validate.

All 52 combat-model tests pass, including four new t060 probes. Generated
Markdown matches the JSON renderer. CRT-era engine output, old sweep
JSON/Markdown, before-fix JSON, and review reproduction script are byte
unchanged from `28814c9d`. No cargo or Rust edits. This completes t062's
comparison and consultation evidence; RT class tuning and implementation
of remaining owner choices are separate work.

## Joint decision-2 recommendation

Claude and Codex agree on the following proposal. It awaits owner approval.
Item 5 is separately approvable from items 1–4.

1. Check each fleet's ROE after one compulsory exchange, then after each
   exchange. Unarmed survivors automatically attempt escape. Keeping the
   existing withdrawal exchange fixed, the convoy control improves from
   1.017 surviving TT with lock3, to 5.773 with after1, to 7.807 when
   unarmed survivors may escape.
2. Replace the compulsory withdrawal volley with speed/contact-gated
   pursuit. Allow at most one successful pursuit exchange per opposing
   empire per fleeing-fleet episode, bound to its departure sector and
   layer. End the episode on actual departure or a new order; preserve it
   across the year boundary. Suppress voluntary first-identification
   commitment while withdrawing. Neither side pursues if both disengage.
   Same-encounter pursuit retains its bank; a later encounter starts at zero.
3. Keep ordinary line-first screening during pursuit; retain and explain
   the free-critical exception. Certain immediate pursuit leaves 7.831 TT
   under ordinary screening versus 5.696 with auxiliaries targeted first.
   These are conditional immediate-catch controls, not full chase odds.
4. The 64-round cap ends the encounter unresolved, preserves survivors,
   and grants no assault permission from relative AS. Ordinary encounter
   rules apply next tick; add no special cap-suppression state. The cap
   probe found zero exits in 200,000 tested battles, not a proof it can
   never occur.
5. Separately approve effective NC combat AS as the common strength measure
   for attack willingness, in-battle ROE, and the orbital gate. Use actual
   crippled AS, count each participating base once (100 nominal, 50 crippled),
   and do not inflate AS with qualitative die bonuses. Remove the classic
   2/7/21 weights and +30 guard proxy from these tests. Keep ROE thresholds,
   detection and interception mechanics; this does not join new fleets or
   remove first-identification commitment outside withdrawal.

Item 5 deliberately changes who agrees to fight. It supersedes the relevant
willingness clause of [the classic engagement ADR](../decisions/gtkmxw3l-engagement-follows-the-classic-ecmaint-encounter-rule.md)
if approved; classic encounter-parity tests and mission sweeps must then
label expected NC differences rather than claim exact parity there.
Destroyers count a little more against cruisers (1:3 instead of 2:7),
a base deters a little less (3.33 CA instead of 4.29), and BB:CA remains 3:1.
Where the ROE ratio test applies, three DD against one CA at ROE 6 change
from refusing (6 < 7) to accepting (30 = 30); four CA against an unguarded
base change from refusing (28 < 30) to accepting (120 > 100). Damaged ships
contribute their actual reduced combat AS. No campaign measurement yet
establishes how often these entry decisions flip. The battle sweep already
uses NC AS for in-battle ROE, so it cannot measure that operational change.

No disagreement remains between the reviewers. Separate approval for the
strength measure keeps this deliberate encounter-rule change explicit.

Decision-7 hull controls: [destroyer durability under withdrawal and recovery](wnwxz2et-destroyer-durability-under-withdrawal-and-recovery.md) tests DD DS 8 with this approved timing and retreat contract, including convoy survival and repeated-contact wipe risk.
