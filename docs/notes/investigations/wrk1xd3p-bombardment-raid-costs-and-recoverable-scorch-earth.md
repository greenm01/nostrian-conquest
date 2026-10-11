---
id: wrk1xd3p
date: 2026-10-10
kind: investigation
status: investigating
tags: [investigation]
---
# Bombardment raid costs and recoverable Scorch Earth

Reduce bombardment's treasury damage to25 goods per10 hits reaching industry; keep production damage at1. Small raids remain worthwhile, but a cruiser no longer usually destroys almost all of a large unguarded treasury in one pass. Battleships retain destructive siege power. Replace NC's permanent Scorch wipe with classic's repeating, cancellable decay and capture penalty, preserving potential. Leave imported debt unchanged. Claude agrees with both recommendations; the owner has expressed a preference for classic Scorch but has not yet approved this complete item5 contract.

## t087: joint recommendation

1. Bombardment destroys25 goods and1 production per10 hits reaching industry, instead of100 goods and1 production. Keep the support CRT, battery shield, third-exchange breakthrough and discarded planetary remainders. In the controls, an unopposed CA destroys about123 of500 goods rather than441; a BB still destroys473 rather than500.
2. Scorch is an order flag, active until rescinded or the world is captured. An order issued this turn first applies in the next maintenance economy step, before movement and assaults. Builds spend first; Scorch then retains30–49% of production, floors it at1, sets positive goods to `min(G/10,sqrt(G))`, and gives no growth or income. Potential survives.
3. On capture while flagged, halve production and positive goods, empty the dock, clear the flag, then floor production at1. Preserve negative goods unchanged in both annual decay and capture. Keep NC's approved conversion and queue inheritance; do not import classic's ordinary capture damage. One annual Scorch plus capture takes P100/G500 to mean P19.75/G11.18 before rebuilding.
4. Use the existing Scorch command to set or rescind the flag, with explicit confirmation and a short order/report notice. No new screen or column. Explain that it repeats until cancelled. Keep surviving unfinished work; builds completed before capture can still be destroyed in the captured dock. No new economic controls, upkeep or repair bills.
5. No remaining disagreement with Claude. The raid results include orbital combat and battery fire but condition on reaching the target; they are not full-map campaign forecasts. Three BB against a base, three batteries and ten armies lose57.5 points on average; adding three defending CA raises the loss to85.8. Choosing to raid a defended world still carries real risk.

## Scope and evidence levels

This follows [the economy scope](0t08owtu-economy-review-decisions-and-campaign-pacing-scope.md) and [item4 evidence](et03gvm9-conquest-income-inherited-assets-and-bombardment-economics.md). [Item4's ADR](../decisions/3htuxw8w-captured-worlds-convert-in-two-quiet-years-and-keep-what-survives.md) preserves assets after an ordinary capture. Scorch's explicit capture destruction is an exception for item5 approval, not an unannounced change to ordinary Invade or Blitz.

The raid model uses approved fleet effectiveness/carry/full finishing DS, DD DS8, damage-aware strength, natural-9 criticals and existing mixed/base modifiers. It assembles one known opposing force per empire, with eligible bases defending only. Both sides use ROE10, so willingness pruning and pursuit are inactive in these armed-force controls. There is no surprise. Crippled survivors of orbital combat enter bombardment still crippled; the orbital hit bank does not migrate into support fire. Battery fire executes with discarded remainders. Batteries shield the dock for their whole final exchange. Only exchange3 can reach armies and industry, and only when it starts without batteries.

These are staged complete combat raids, including the gate, rather than just conditioned hit budgets. They do not simulate detection, interception en route, dispatch dates, reinforcements, reinvasion, escape or the voyage home. Travel and commitment are priced separately below. RT recovery does not occur inside a raid. Actual engine implementation of several approved rules is pending, so this is a Python candidate model, not fresh engine or classic-oracle validation.

Each sampled row has10,000 trials, independent seeds870001 onward. Rates100,25,10 use the same seeds within each case. Only industrial goods damage changes. JSON includes95% Wilson intervals for binary outcomes and approximate95% standard-error intervals for mean losses. These intervals describe sampling error, not uncertainty about the model. Production recovery and Scorch use exact fractions except the goods square root, evaluated in Python floating point. Scorch's twenty production factors are enumerated, not sampled.

## Raid results

Each world starts with potential100, production100 and goods500 unless labelled small. All units begin nominal. “Loss” means attacker build points destroyed, not committed points or crippled survivor value. Goods columns compare rates100,25,10. Industrial production damage is identical across those rates.

| Attacking force and defenses | Committed | Attacker loss | Goods destroyed:100 /25 /10 | Production lost |
| --- | ---: | ---: | ---: | ---: |
| 1 DD, undefended | 5 | 0 | 40.4 /10.1 /4.0 | 0.40 |
| 1 CA, undefended | 15 | 0 | 440.7 /122.8 /49.1 | 4.91 |
| 1 BB, undefended | 45 | 0 | 500.0 /473.2 /229.5 | 22.95 |
| 1 DD vs1 army | 5 | 0 | 0 /0 /0 | 0 |
| 1 CA vs10 armies | 15 | 0 | 0 /0 /0 | 0 |
| 1 BB vs10 armies | 45 | 0 | 480.4 /316.2 /129.5 | 12.95 |
| 1 CA vs1 battery | 15 | 15.0 | 0 /0 /0 | 0 |
| 1 BB vs1 battery | 45 | 4.4 | 451.3 /427.0 /207.5 | 20.75 |
| 3 CA vs1 battery,10 armies | 45 | 19.4 | 106.0 /30.7 /12.3 | 1.23 |
| 6 CA vs3 batteries,10 armies | 90 | 80.2 | 44.5 /15.0 /6.0 | 0.60 |
| 3 BB vs3 batteries,10 armies | 135 | 40.3 | 451.3 /447.0 /357.1 | 39.78 |
| 1 CA vs1 guarding CA | 15 | 8.5 | 134.5 /35.8 /14.3 | 1.43 |
| 3 CA vs2 guarding CA,1 battery,10 armies | 45 | 37.6 | 14.1 /3.9 /1.6 | 0.16 |
| 6 CA vs1 base,3 batteries,10 armies | 90 | 88.0 | 5.7 /1.7 /0.7 | 0.07 |
| 3 BB vs1 base,3 batteries,10 armies | 135 | 57.5 | 402.3 /378.9 /286.5 | 31.88 |
| 6 CA vs3 guarding CA,1 base,3 batteries,10 armies | 90 | 89.9 | 0.2 /0.04 /0.02 | 0.002 |
| 3 BB vs3 guarding CA,1 base,3 batteries,10 armies | 135 | 85.8 | 292.8 /256.6 /176.7 | 19.28 |

At rate25, the unopposed CA goods mean has interval122.14–123.41. The three-BB/base case has mean goods378.88, interval375.00–382.76, and attacker losses57.49, interval56.59–58.38. With three additional guarding CA, those become goods256.57 [252.06,261.08] and losses85.75 [84.81,86.69]. These results do not price destroyed enemy defenses into the goods number; the JSON records those losses separately.

Batteries make cruiser-only assaults expensive. Battleships earn their bombardment role through AS135 in support fire, versus a cruiser's30, as well as their durability. The rate25 change preserves that role. Rate10 is a possible gentler setting, but halves the undefended battleship's goods damage and reduces the unopposed cruiser to about one mature tax50 income pass. Rate25 preserves strong rewards for taking an undefended opportunity while reducing the extreme return from the smallest raids. These three rates are sensitivity controls, not a proof that25 is an optimum.

Small worlds remain vulnerable. Against a P10/G50 undefended world, a cruiser destroys all50 goods at both100 and25, plus4.91 production on average. The proposed rate is not a small-world shield. Fleet defense, early warning and responding to an arrived raider still matter. Do not describe100-rate worlds as total write-offs: a cruiser destroys reserves, but in this control leaves about95 of100 production and all potential.

## Repeated bombardment and the cost of time

A Bombard order remains active and can strike again in later years. The stationary controls let it bombard five consecutive passes without reinforcement, new defense builds or counterattack. Normal smooth tax50 income/growth runs before each later raid. These deliberately exposed worlds test whether the lower rate makes a raider harmless; they do not estimate the chance of holding enemy orbit for five years.

| Raider / initial army screen | Goods destroyed in5 raids:100 /25 /10 | Production left |
| --- | ---: | ---: |
| 1 DD /1 army | 109.3 /27.3 /10.9 | 99.06 |
| 1 CA /10 armies | 693.4 /363.6 /145.4 | 87.25 |
| 1 BB /10 armies | 625.8 /625.8 /625.8 | 16.70 |

Totals can exceed starting goods500 because they include destroyed later income. A garrison protects against the first small strike but loses armies; it does not permanently shield a treasury against an uncontested warship. At25 the cruiser still destroys364 points over five raids, while the battleship repeatedly empties the treasury and devastates industry. Only the goods rate changes; production damage remains a lasting consequence with gradual automatic recovery.

For an isolated orbit-to-orbit trip, ideal travel is approximately `(map distance+2)/speed`. At distance3, DD/CA/BB arrival takes0.83/1/1.25 years; at distance8,1.67/2/2.5 years; at distance13,2.5/3/3.75 years. A return journey roughly doubles travel if routes and destination are unchanged. This ignores tick rounding, interceptions and mission handling. The three bombardment exchanges do not consume three years or ticks. Sustained bombardment commits ships in enemy orbit instead of returning them to defend or escort elsewhere.

A surviving ship is reusable capital, not an expense equal to its price on every sortie. A destroyed one must be replaced. Crippled orbital survivors carry reduced strength into this raid and later contacts under RT2. The raid controls do not add repair bills or pretend that recovery has no opportunity cost. Their implication is narrower: at25, profitable raids and costly defended assaults both exist. Full campaigns must still measure how often either situation occurs.

## Scorch contract for approval

**Lifetime and control.** Reuse the existing Scorch command to enable or rescind the world's flag. Enabling changes an order, not the world's production immediately. The flag persists across saves, turns and exports until rescinded or capture. Reissuing the command must offer cancellation instead of “already scorched.” The typed model and classic codec already have `scorch_earth`; no new stat or player menu is required. A short confirmation says “Scorch each year until cancelled; no growth or income.” Existing report/detail text should show it is active and acknowledge cancellation. This is not hidden from the owner.

**Timing.** Scorch ordered this turn first runs in the next pass's economy phase, before the52 movement/combat ticks. Complete or advance queued builds using the ordinary local budget first. Then apply Scorch instead of growth, the normal high-tax penalty and income. Conversion status still advances on schedule. The attacker's strike later in that pass finds the world already scorched. Rescinding before maintenance prevents that pass's annual decay; no order written after a pass reverses damage already done.

**Annual damage.** Preserve potential K. For K>=1, set `P=max(1,P*(30+d)/100)` with one seeded uniform integer d from0 through19 per world/pass. Keep P bounded by K. For positive goods use `G=min(G/10,sqrt(G))`. For G<=0 leave G unchanged; debt is not destroyed or halved. No growth or income while active. Bases do not offset the decay. Use the ordinary seeded turn randomness rather than an order-time reroll, so toggling or previewing cannot reroll damage. Forecast revenue is0 while Scorch will run.

**Capture.** Apply the assault's existing losses first. If the world is still flagged at capture, halve P and positive G, empty surviving dock contents, clear the flag, then floor P at1. Preserve K and negative G. The conversion counter resets to0 as approved. Do not also apply classic's ordinary Invade/Blitz retention multipliers; item4 deliberately kept surviving assets. This Scorch exception must be explicit in capture reports and the manual. Later recaptures do not get another Scorch penalty unless the flag is set again.

**Queue and builds.** Keep NC's unfinished build progress and amount owed. Annual Scorch does not independently clear the dock or queue. This follows NC's approved build spending and item4 inheritance, rather than importing classic's annual completion and queue clearing. Ships completed before a later flagged capture are in the dock and are destroyed there; remaining unpaid work passes with the world. The owner may cancel pending orders using the existing command. Neither annual Scorch nor capture refunds work or creates credit. Spending before Scorch lets a defender convert some reserves into real units; that is the documented order of operations, not an extra resource sink.

**Existing zero-potential worlds.** Do not recreate potential that today's irreversible Scorch already destroyed. K0 staysP0. The classic minimum1 must not manufacture production beyond potential0. This is a small explicit NC boundary adaptation, alongside debt preservation and queue semantics, not a claim of byte-identical classic economy behavior.

## Scorch magnitude and rebuilding

[Classic's documented annual rule](../../dev/ec-economy-reference.md) preserves30–49% of production and uses the smaller of a tenth and square root for positive goods. It is not universally a90% treasury loss: G500 becomes22.36, a95.5% loss. The capture floor follows the halving, verified at110a:6edd..6f02 and recorded in [classic battle §2.9](../../dev/ec-battle-spec.md).

These controls start at K100/P100/G500 after any build spending. There are no queued builds. They apply one or more annual Scorch passes, then clear the flag voluntarily or capture the flagged world. “Next20 income” starts after clearing/capture, excludes the prior active Scorch passes, uses smooth tax50 without a base, and includes the approved two zero-income passes after capture.

| Annual Scorch passes / ending | Mean P left | Goods left | Next20 income | Pass reaching90% potential |
| --- | ---: | ---: | ---: | ---: |
| None, ordinary capture without Scorch | 100 | 500 | 900 | Already full |
| 1, owner rescinds | 39.50 | 22.36 | 802.90 | 13–15 |
| 1, captured while flagged | 19.75 | 11.18 | 644.51 | 18–19 |
| 2, captured while flagged | 7.80 | 1.12 | 606.47 | 19 |
| 3, captured while flagged | 3.08 | 0.112 | 591.45 | 19–20 |

With one annual Scorch and capture, actual production is15–24.5 rather than one guaranteed mean. Next20 income is255.49 below the intact-capture control; goods are488.82 lower. The original owner also forgoes income while Scorch runs. These are substantial denial costs even though the world can rebuild. The comparison is not an instruction to Scorch every threatened world: if the owner holds it, they bear the loss themselves. The decision rewards judging when a world is truly lost and forces the conqueror to choose between speed, preservation and rebuilding.

The JSON includes K10 and half-developed K100 controls, debt cases, and a flagged-capture-without-prior-annual-decay boundary case. That last case halves P even with zero annual passes; it is distinct from ordinary unflagged capture. No distribution over campaign capture dates is assumed.

## Player-facing explanation

“Bombardment first fights through the world's defenses. Hits reaching industry destroy1 production and25 saved points for each complete group of10 hits. Scorch Earth repeats at maintenance until cancelled: it destroys most production and savings and stops growth and income, but leaves the world's potential intact. If the world falls while Scorch is active, more assets are destroyed and the order ends. The new owner can rebuild after conversion.”

Put the exact30–49%, goods formula, phase order and debt exception in the economy reference/manual appendix. The ordinary screens need only the existing command's active/cancel wording, zero revenue forecast and concise damage reports. No new economic slider, research choice or repair management is proposed.

## Files, checks and remaining limits

- [raid_review.py](../../../tools/economy-model/raid_review.py) and [raid_review_results.json](../../../tools/economy-model/raid_review_results.json):19 staged raid cases, three goods rates,10,000 trials each; plus nine five-raid controls.
- [scorch_review.py](../../../tools/economy-model/scorch_review.py) and [scorch_review_results.json](../../../tools/economy-model/scorch_review_results.json): enumerated annual production decay and analytical smooth recovery; floating-point square roots for goods.
- [test_raid_review.py](../../../tools/economy-model/test_raid_review.py): battery shield and breakthrough, damage persistence from gate to strike, denied-gate safety, matched-rate invariance, debt preservation and agreement with the prior exact unopposed bombardment model.
- [test_scorch_review.py](../../../tools/economy-model/test_scorch_review.py): twenty-factor range/mean, goods branch boundary, post-capture floor, zero potential, debt, direct-step recovery agreement and one-year-plus-capture arithmetic.
- `python3 -m unittest discover -s tools/economy-model -p 'test_*.py'`:59 tests pass,12 new. No Rust edits or cargo. No new engine/oracle conformance claim. The old rate100 evidence remains in prior files; this task does not overwrite it.

Campaign frequency, dispatch decisions and defender response remain unmeasured. This recommendation is a bounded first balance choice, not a claim that every world or force ratio has optimal incentives. Item6 prices and opening resources should use the approved economy after this decision, rather than compensating for its old reserve-destruction rate.
