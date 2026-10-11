---
id: wnwxz2et
date: 2026-10-10
kind: investigation
status: investigated
tags: [investigation, combat, balance, recovery]
---
# Destroyer durability under withdrawal and recovery

Test DD 10/8 at cost 5 as the first hull adjustment. Keep the other ships, starbases, batteries, prices and planetary support rules unchanged. Destroyers should become useful fast combat ships without displacing cruisers as convoy escorts. The mixed-fleet die bonus and retreat thresholds affect the result; durability alone does not explain every gain. This is evidence for decision 7, not an accepted rule.

## t075

Challenge Claude's hull proposal with the approved withdrawal and RT2 contracts. Separate the mixed bonus, measure convoy survival, test guarded and unguarded bases, and track damage across repeated contacts. Decision 6 stays paused. No Rust edits or cargo.

## Evidence and limits

The reusable [hull review](../../../tools/combat-model/hull_review.py) and its [results](../../../tools/combat-model/nc_hull_review.json) use the existing fleet step function and recovery cohorts. [Tests](../../../tools/combat-model/test_hull_review.py) check equivalence and cohort bookkeeping. Run instructions live in the [model README](../../../tools/combat-model/README.md#destroyer-durability-t075).

Battle rows use 20,000 seeded samples; repeated-contact rows use 5,000. The JSON records seeds, inputs and Wilson 95% intervals. Shared trial seeds reduce noise between controls but do not make rows independent estimates. These are hypothetical legal contacts, not measured campaign encounter frequencies. Each side is one fleet; joined participation is not assumed.

No-catch and certain immediate-catch rows bracket an extra exchange. They do not simulate speed, detection, pursuit episodes, travel or the chance of another contact. Both sides shoot during a successful immediate catch, with ordinary screening and the same battle bank. New encounters reset the bank. A transport surviving battle has not necessarily delivered an army.

The repeated-contact test sends the survivors into four fresh enemy forces without replacements. RT is 2, damage-year recovery is excluded, and there is no base assistance. Gaps 1–3 mean maintenance-year labels. Gap 0 is an additional stress case only: four separate contacts must be legal; it does not authorize four pursuit exchanges against the same fugitive. No production or reinforcement model claims that this schedule is normal.

## Hull recommendation

Recommend DD AS 10, DS 8, price 5 for the first playtest. Keep every other value and price unchanged. DS 6 repairs too little of the destroyer's weakness; DS 10 makes its price-normalized durability match the cruiser while retaining higher speed and execution fire. That is a crowding-out risk, not proof that cruisers become strictly obsolete: suppression and execution have different effects. Cost 4 is another plausible lever but changes fleet production and replacement rates. Claude's price-4 rows also spend slightly less than the nominal budget where whole destroyers do not divide it. They are not exact equal-cost pairs.

All three warships have nominal AS/build-point = 2. DS/build-point is 1.0 today for DD, 1.6 at DS 8, 2.0 for CA and 2.22 for BB. A starbase has AS/point 2.0 and DS/point 2.4. These ratios explain a durability direction, not battle odds: allocation, crippling, execution, criticals and bonuses intervene. DD 10/8 still buys only 80% of the cruiser's first-step durability per build point.

ROE-10 warship controls never withdraw. Decision 2 therefore leaves this matrix unchanged; regression tests compare its outcomes directly with the implemented decision-1 preset at the same seeds. Left-side victory percentages:

| Matchup | DD DS 5 | DS 6 | DS 8 | DS 10 |
| --- | ---: | ---: | ---: | ---: |
| 30 DD vs 10 CA | 9.8% | 16.5% | 30.9% | 48.7% |
| 90 DD vs 30 CA | 9.5% | 16.1% | 30.7% | 49.3% |
| 90 DD vs 10 BB | 5.2% | 10.6% | 23.4% | 39.2% |
| 30 CA vs 10 BB | 39.6% | 39.6% | 39.6% | 39.6% |
| 15 DD + 5 CA vs 10 CA | 30.1% | 34.3% | 46.4% | 57.4% |
| 6 DD + 3 CA + 1 BB vs 8 CA | 46.4% | 50.0% | 56.3% | 64.2% |
| 10 DD vs 1 SB | 6.3% | 12.4% | 18.1% | 21.1% |
| 1 DD + 3 CA vs 1 SB | 24.6% | 24.6% | 25.2% | 25.6% |
| 10 CA vs 10 CA | 49.0% | 49.0% | 49.0% | 49.0% |

The 10 CA parity row estimates one side's wins, not half of non-draws; it includes mutual destruction. Sampling error also remains. DD8 vs CA is 30.9%, with a 95% interval of 30.2–31.5%; the three-class row is 56.3% [55.6, 57.0]. These independently seeded estimates broadly support Claude's smaller sweep. Neither proves optimal fleet composition in a campaign.

## What the mixed bonus contributes

| Fleet vs pure CA | DS5, bonus off | DS5, bonus on | DS8, bonus off | DS8, bonus on |
| --- | ---: | ---: | ---: | ---: |
| 15 DD + 5 CA vs 10 CA | 23.2% | 30.1% | 37.3% | 46.4% |
| 6 DD + 3 CA + 1 BB vs 8 CA | 37.2% | 46.4% | 46.1% | 56.3% |

With the bonus off, DS8 raises the two-class fleet from 23.2% to 37.3%, and the three-class fleet from 37.2% to 46.1%. At DS8 the mixed bonus then adds 9.1 and 10.2 percentage points. These are controlled contrasts, not additive causal shares independent of hull values. The three-class fleet beats pure cruisers only with the current bonus in these controls. Keep that fact separate from the DS recommendation; changing participation or mixed eligibility later requires another calibration.

## Withdrawal and convoy roles

ROE6 vs ROE10 below means the left fleet fights one compulsory exchange and then uses the approved retreat threshold. These are imposed contacts, not a claim that a weak convoy would volunteer to attack. Field wins and surviving ships answer different questions.

| Matchup | DS5 no catch / immediate catch | DS8 no catch / immediate catch |
| --- | ---: | ---: |
| 30 DD vs 10 CA | 8.3% / 8.5% | 23.1% / 23.2% |
| 90 DD vs 10 BB | 4.4% / 4.5% | 18.4% / 18.6% |
| 15 DD + 5 CA vs 10 CA | 23.9% / 23.9% | 35.2% / 35.2% |
| 6 DD + 3 CA + 1 BB vs 8 CA | 32.8% / 32.8% | 46.6% / 46.6% |

Every convoy carries 10 TT with 75 build points of escorts, against 10 CA. Its ROE is 6. The table gives mean transports surviving and the chance at least five survive under certain immediate pursuit.

| Escort | DS5: TT / at least five | DS8: TT / at least five |
| --- | ---: | ---: |
| 15 DD | 3.52 / 36.6% | 5.99 / 63.5% |
| 5 CA | 7.77 / 83.1% | 7.77 / 83.1% |
| 6 DD + 3 CA | 6.32 / 65.7% | 7.44 / 78.4% |

Without a catch, the DD5 convoy keeps 7.87 TT; DD8, CA and mixed escorts keep 9.90. DD8 fixes a serious screen weakness but does not make destroyers the best convoy escort. Under a catch, cruiser escorts retain 7.77 TT for a 68.53-point total loss bill; DD8 escorts retain 5.99 for an 85.99-point bill. Mixed DD8 escorts retain 7.44 for 73.63 points. The bill includes transports and escorts.

This is a healthy role split. Cruisers suppress attackers and buy more durability per point. Independent destroyer fleets travel at speed 6, cruiser fleets at 5. Transports also travel at 5, so adding DD does not make a convoy faster than a cruiser escort. ETACs cap a mixed fleet at speed 3; battleships cap it at 4. The model does not price the separate destroyer raider's reach or estimate pursuit probability.

## Bases and other values

| Attacker vs lone base, ROE10 | DS5 wins | DS8 wins |
| --- | ---: | ---: |
| 6 DD | 5.2% | 6.3% |
| 10 DD | 6.3% | 18.1% |
| 15 DD | 19.1% | 43.9% |
| 20 DD | 44.5% | 82.6% |
| 30 DD | 92.4% | 99.9% |

Equal-price DD8 attackers win only 18.1% against a lone base; the interval is 17.5–18.6%. Spending twice the base's price on 20 DD raises that to 82.6%. A base is a defensive investment, not a guarantee against a larger mobile force. Add three cruiser guards and those 20 DD win only 28.2%, versus 13.0% with DS5. The defended position costs 95 points against 100 attacking points. Ten DD8 against that guarded base win 0.7%. Keep starbase values for this playtest.

Do not raise cruiser or battleship prices to make destroyers attractive. CA and BB retain distinct durability, suppression and speed roles. Scouts, transports and ETACs buy reconnaissance, invasion capacity and colonization rather than AS; these combat controls cannot establish their campaign prices. Keep them pending the economy and pacing review.

Keep batteries and planetary support unchanged. DD bombard AS remains 5 while CA has 30 and BB 135, so improving DD fleet DS does not grant equal-price siege firepower. No DD8 planetary sweep was run here. This is a limit, not evidence that siege results are unchanged: stronger destroyers can survive more battery fire. The first playtest should watch that interaction before changing batteries themselves.

## Recovery and the ROE tradeoff

RT2 does not grant rapid free replacement. In both mixed-force probes, every surviving fleet is still damaged at its first return in N+1 or N+2. In the two-class fleet, 96.8% of surviving DD8 hulls are still crippled at that return without pursuit; nearly 100% are crippled with an immediate catch. A return in N+3 starts with fully recovered survivors. Destroyed hulls stay destroyed.

For the 120-point fleet of 6 DD, 3 CA and 1 BB, four contacts against fresh 8-CA forces give these losses. Its ROE is 6; the enemy's is 10. These rows assume no catch and no replacements.

| Years between contacts | DS5 mean loss / wiped out | DS8 mean loss / wiped out |
| --- | ---: | ---: |
| 1 | 110.51 / 82.2% | 107.95 / 78.5% |
| 2 | 100.52 / 67.2% | 98.89 / 65.8% |
| 3 | 60.74 / 18.1% | 58.09 / 22.8% |

The gap-3 wipe risk rises from 18.1% [17.0, 19.2] to 22.8% [21.6, 24.0]: **4.7 percentage points**, while mean losses fall by only **2.65 points**, from 60.74 to 58.09. This is material, not rounding noise. It is a change in the loss distribution as well as the average. With certain immediate pursuit, the same gap-3 case instead improves: wipe risk falls from 85.6% to 82.4%, and mean loss from 111.26 to 109.06. There is no general claim that DS8 always increases wipe risk.

ROE reacts to surviving strength, not a fixed number of rounds or a target probability of survival. More durable ships can keep a fleet above its retreat threshold longer. In the first three-class contact without pursuit, DS8 raises field wins from 32.8% to 46.6%, lengthens the fight from 2.04 to 2.61 exchanges, and raises mean own losses from 17.01 to 18.44 points. ROE6 stays while friendly effective AS is at least enemy effective AS. This is consistent with the later wipe tradeoff; the experiment does not isolate every allocation path as a causal decomposition. Stronger ships are not a promise of safer outcomes under an unchanged order.

The two-class fleet benefits much more after recovery: with gap 3 and no catch, its four-contact mean loss falls from 123.93 to 81.74 of 150 points, and wipe risk from 63.8% to 22.5%. At gap 1, mean loss remains 142.48 with DD8; pressing the same survivors into fresh enemies each year is still costly. An automatic two-year recovery window rewards giving battered forces time rather than issuing repair orders.

## Player consequences and joint recommendation

Calling today's destroyers a universal trap overstates the evidence. Their straight fleet fight is weak; their speed, interception reach and finishing fire still have value. Equal-cost duels should not make every class win half the time. DD8 keeps that distinction while making the purchase less punishing.

The two-class DD8 fleet wins the field less often than pure cruisers, but destroys more enemy build points: 107.48 versus 92.99 in the parity control, while losing 97.98 versus 94.51 itself. Those are fight-to-finish controls, not campaign profit. They show why execution fire can matter even when field-win odds are below half. Surviving enemy damage can later recover; destroyed hulls cannot.

Recommend **DD 10/8 at cost 5; all other hulls, prices, RT2 and support fire unchanged** for the first playtest. Cruisers remain the stronger convoy escort; independent DD fleets retain speed 6 and execution fire. Do not present the mixed +1 gain as a gain from DS alone. Disclose the ROE6 wipe tradeoff and measure it in play. No disagreement remains about the proposed value; the qualifications narrow the claims made for it.

No new player menu or recovery statistic is needed. If approved, change the destroyer's DS entry and explain its role in the existing manual. Preserve the explicit ROE explanation: a fleet compares its current fighting strength after each exchange. Avoid advertising ROE6 as a fixed casualty ceiling. Risk-taking is rewarded by a more useful raider and finishing unit, with losses and recovery time still consequential.

## Validation

The standard-library suite ran 68 tests successfully, with one expected skip for the optional NumPy/SciPy ground control. Five new tests cover the hull review. No engine or classic-oracle claim attaches to the new retreat/RT combinations. No Rust source changed and cargo was not run for t075. The earlier t060 comparison remains the engine evidence for the underlying fleet model; implementation of decisions 2 and 3 still needs its own parity check.

## Connections

Uses [decision 1](../decisions/ld2f252j-flat-combat-effectiveness-with-battle-local-carry.md), [decision 2](../decisions/kiswrzos-withdrawal-pursuit-and-one-strength-yardstick.md), and [recovery](../decisions/p01cg7ll-rt-controls-automatic-ship-recovery.md). Extends [RT calibration](4lasivnr-ground-combat-and-recovery-turns-campaign-calibration.md) and the [withdrawal investigation](7y0ypr6b-combat-withdrawal-and-persistent-damage-after-decision-1.md). Leaves [decision 5's planetary support rules](../decisions/mk1ie4e3-batteries-shield-the-stardock-and-failed-landings-hold-orbit.md) intact. [Participation and surprise](m1h272uh-joined-participation-surprise-and-allocation-fairness.md) remain a separate paused review.
