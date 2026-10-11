---
id: 4lasivnr
date: 2026-10-10
kind: investigation
status: investigated
tags: [investigation, combat, ground, recovery]
---
# Ground combat and Recovery Turns campaign calibration

Use defender-first ground fire on the shared 25/50/100 effectiveness table. Carry fractional damage within the landing. Add no defender modifier, homeworld bonus, or ground critical. This replaces the current 10-to-11-army capture cliff with a useful progression of risk. Prepared invasions need more transports but lose fewer armies at comparable confidence. The owner accepted this as decision 4. For recovery, use the approved RT 2 game-wide rule, including starbases. RT 1 removes the benefit of starbase assistance. BB RT 3 sharply increases losses when fleets return every third year. Fast DD recovery gives a modest benefit in one spacing window, not an inexhaustible screen. The owner subsequently approved RT 2 as one game-wide rule; class-specific schedules remain comparison experiments.

## t068

Compare exact ground capture odds across force size and through blitz landing fire. Price a 90% single-attempt capture. Compare RT schedules in repeated-contact campaigns with the approved damage-year exclusion and starbase assistance. No Rust changes. No cargo. This note supplies evidence; it is not another ADR.

## Method and boundaries

Ground is exact enumeration in floating point. Shared-roll candidates visit every reachable army/remainder state. The tool checks the longest roll path against 64 rounds. Independent per-army fire uses a finite 64-round transition matrix, including no-hit loops and unresolved probability. Current-rule controls retain both the ratio CRT and its larger-force mutual-annihilation award.

RT uses 5,000 seeded trials per cell. Each trial gets the same encounter-specific random seeds across schedules. The model reuses the fleet hit allocator and observes its suppression, execution and critical steps to update recovery cohorts. It uses decisions 1 and 2, baseline hull values, half crippled AS, full finishing DS, current modifiers and unscreened natural-9 criticals. It preserves damage between battles and resets hit banks. It does not use the proposed ground rule for fleet fire.

These RT campaigns impose four encounters against fresh opponents, with no reinforcements. They do not generate maps, movement, contact, player orders or income. A new order permits each scheduled contact. Normal rows assume a clean escape; sensitivity rows assume an immediate successful pursuit with the ordinary screen. Assistance means a full quiet pass at a protected friendly base between battles. It never applies in a battle pass or to the base itself. The travel needed to qualify is outside this control.

This is new design evidence, not a classic-oracle comparison. The base fleet allocator was compared with t060 separately. Neither the new ground rule nor RT has been checked against an implementing engine here.

## Ground rule and formula

Let A and D be armies, and let each roll select k from 1, 2, 4 with probabilities .3, .3, .4. Keep an integer quarter-army remainder for each target side, initially zero.

First, defenders inflict `floor((D*k + remainder_against_A)/4)` army losses. Keep the remainder modulo 4. Then surviving attackers do the same to D with their own roll. Cap losses at the armies present. Repeat. End immediately when either force is gone; discard both remainders. Ground armies have one damage step. There are no fleet-style crippled ground armies.

Before losses or rounding, 10 defenders average 6.25 army-equivalents of fire. An equally large attacking force replies with roughly 3.75 survivors on average, whose mean fire is roughly 2.34 army-equivalents. Actual integer losses and later rounds are enumerated. First fire supplies a substantial advantage; adding another defender modifier would charge for that advantage twice.

Player wording: “Defenders fire first. Each side rolls once for 25%, 50%, or 100% effectiveness. Remove attacking armies lost, then the surviving attackers fire back. Damage short of eliminating an army carries into the next round of this landing battle. Repeat until one side is gone. Reports show whole armies lost and remaining.” The remainder stays hidden. The manual explains it; the BBS tables do not gain fractional counters.

Mutual annihilation cannot arise in this sequential rule. Simultaneous comparison rules leave an empty planet with its owner if both forces disappear. They do not create a replacement army. There is no homeworld bonus in these candidates.

## Capture odds and scale

The [full 5–20 table](../../../tools/combat-model/nc_ground_calibration.md) compares the current rule, its tie-rule correction, both defender-first tables, shared simultaneous fire and independent per-army fire. Every number below is a capture percentage.

| Attackers vs defenders | Current | Defender-first shared table |
| --- | ---: | ---: |
| 1 vs 1 | 0.0 | 33.6 |
| 2 vs 1 | 100.0 | 85.1 |
| 3 vs 1 | 100.0 | 99.4 |
| 3 vs 2 | 100.0 | 55.6 |
| 5 vs 2 | 100.0 | 97.3 |
| 5 vs 5 | 8.0 | 22.6 |
| 10 vs 10 | 8.0 | 19.9 |
| 11 vs 10 | 83.0 | 27.6 |
| 15 vs 10 | 100.0 | 56.1 |
| 20 vs 10 | 100.0 | 91.8 |
| 20 vs 20 | 8.0 | 19.0 |
| 30 vs 20 | 100.0 | 56.0 |
| 40 vs 20 | 100.0 | 92.3 |

The current cliff has two causes. Giving mutual destruction to the defender alone reduces 11-vs-10 capture from 83% to 42.6%. The ratio columns still cause jumps. Correcting the tie rule alone does not solve the problem.

Independent per-army .30/.35 fire gives 34.5%, 52.8%, 96.2%, and 99.98% at 10, 11, 15, and 20 against 10. Its 1.5:1 odds rise from 75.9% at 3-vs-2 to 99.4% at 30-vs-20. That is a legitimate design, but it makes large forces much more predictable at the same ratio. Shared defender-first fire retains upset risk. Its approximate ratio stability is measured, not a theorem. Small integer armies differ, and overwhelming ratios can guarantee capture.

Shared simultaneous fire with a +1 defender die modifier still jumps from 30.0% to 56.6% between 10 and 11 attackers. It mutually destroys both forces in 24.1% of equal 10-army battles. The defender-first version avoids both problems. It averages 2.10 rounds at 10-vs-10 and 2.44 at 15-vs-10. VBAM-rate defender-first carry gives 15.2% and 96.6% at those sizes, with 5.06 and 4.03 rounds. It sharply concentrates the transition to reliable capture near 1.3:1. None of these round counts consume extra movement ticks.

## Price of a capture

An army costs 2 points; its reusable TT costs 5. A CA costs 15 and moves at speed 5, as does a TT. Committed cost below assumes buying all of them. It is not all lost. Survivors and transports retain value after capture.

Each row finds the smallest army count reaching at least 90% capture in one attempt against 10 initial defenders. Invade rows assume batteries already cleared and the stated escorts still nominal at softening. All rows assume the orbital gate has been won. The price of securing that condition is excluded.

| Situation | Current armies / committed points / expected casualty points | Candidate armies / committed points / expected casualty points |
| --- | --- | --- |
| No softening; no landing fire | 13 / 91 / 21.01 | 20 / 140 / 17.15 |
| Invade, 1 CA softening | 11 / 92 / 17.76 | 18 / 141 / 14.63 |
| Invade, 3 CA softening | 9 / 108 / 12.57 | 12 / 129 / 8.98 |
| Invade, 6 CA softening | 6 / 132 / 9.64 | 10 / 160 / 8.29 |
| Blitz, 3 CA, 3 batteries | 30 / 255 / 61.40 | 33 / 276 / 58.09 |
| Blitz, 6 CA, 3 batteries | 15 / 195 / 24.40 | 21 / 237 / 19.40 |

Blitz casualty cost includes loaded armies, transports and escorts destroyed during landing fire. It excludes earlier orbital losses. Invade rows count surface casualties after their stated prerequisites. Do not compare these as complete missions against identical initial defenses. These are single-attempt expectations, not expected cost over repeated attacks against a changing garrison.

With 3 CA softening, committed cost rises 19.4%, while expected army losses fall from 6.28 to 4.49 armies. Preparation buys reusable transport capacity and surviving occupation troops. The defender paid 20 for the initial garrison. Adding one army costs 2; without softening, increasing defenders from 10 to 11 raises the candidate's 90% attacker commitment from 20 to 22 loaded transports, or 14 points. With 3 CA softening the corresponding requirement is 12 to 14, then remains 14 at 12 defenders. Defense buys time cheaply, but it is immobile. These arithmetic comparisons alone do not prove a campaign cannot stall.

For ideal orbit-to-orbit travel at speed 5, time is approximately `(map distance + 2)/5` years. Distances 3, 8 and 13 therefore take about 1, 2 and 3 years, or 52, 104 and 156 ticks. The extra transports do not slow the fleet. They do tie up more production and transport capacity during the trip. A speed-4 BB escort changes those examples to approximately 65, 130 and 195 ticks. Actual arrival rounding, interceptions and mission handling are outside this estimate. Build time depends on the source world's treasury, capacity and queues; no universal build delay is claimed.

Invade supplies its own softening once it clears the batteries. A separate Bombard order is not required. Replacing that support-fire CRT with the flat table would be another balance change: 10 attackers with 3 CA softening against 10 initial armies fall from 82.4% to 70.7% capture. Decision 4 keeps support fire unchanged. Only the landing battle has the measured approximate ratio stability.

## Blitz and the battery shield

Against 10 armies and 3 batteries with 3 nominal CA, the candidate gives 8.2%, 29.7% and 57.0% capture at 10, 15 and 20 loaded transports. With 6 CA those become 18.3%, 53.7% and 89.6%. With no batteries, blitz has the unsoftened ground odds: 19.9%, 56.1%, 91.8%.

Blitz still offers immediate landing despite surviving batteries and captures those batteries if successful. It is not the cheap default against a defended world. With 3 CA, 20 loaded transports and 3 batteries, expected captured batteries are only 0.225 per attempt. Successful cover often destroys the prize, and surviving batteries can destroy the landing. Do not sell battery capture as guaranteed compensation for casualties. Invade may clear and land in the same strike too; a universal one-year speed advantage for blitz would be false.

Batteries-first collateral is a separate decision. These blitz controls use current cover, which already targets batteries directly. They do not model docked assets as additional armor or infer that a collateral change fixes landing risk.

## RT spacing and casualties

Damage in year N receives no recovery at N's end. RT 2 remains damaged through the battles in N+1 and N+2, recovers at the end of N+2, and is whole for N+3. One fully qualified starbase year reduces both remaining turns, so that ship is whole for N+2. RT 1 is already whole for N+2 without a base; the approved base bonus does nothing for RT 1.

The mixed fleet is 12 DD, 6 CA and 2 BB, costing 240 points. Each opposing wave has 12 CA. Both sides fight one compulsory exchange; the mixed fleet uses ROE 6, its opponent ROE 10. First-return measurements refer to the next battle after the first encounter, conditioned on the fleet surviving. Both schedules enter N+1 damaged in 100% of trials. A same-year return also leaves both damaged.

| Gap between battles | Uniform 2: damaged at first return | DD 1 / CA 2 / BB 3: damaged at first return | Uniform 2: destroyed points over four battles | Class schedule: destroyed points |
| --- | ---: | ---: | ---: | ---: |
| 1 year | 100% | 100% | 208.38 | 214.53 |
| 2 years | 100% | 100% | 183.85 | 177.60 |
| 3 years | 0% | 61.8% | 105.49 | 136.67 |
| 4 years | 0% | 0% | 105.49 | 105.49 |

At the two-year return, all surviving DD remain crippled under uniform 2 and all are whole under DD 1. Mean entering fleet AS rises from 263.4 to 304.7. Over four battles, DD destroyed falls from 11.86 to 11.74 of the original 12. Changing DD alone produces the same cost improvement in this spacing. Faster recovery helps; it does not replace destroyed ships or make the screen free. At three-year spacing DD 1 alone changes nothing, while BB 3 causes the extra loss. The mixed fleet is wiped in 9.4% versus 22.9% of trials.

The capital control starts with 6 BB against each fresh wave of 15 CA. At three-year spacing, uniform 2 loses 117.50 points over four fights; BB 3 loses 181.35. Wipe probability rises from 13.8% to 38.8%. All surviving capital fleets still contain damage at that first return under BB 3; none do under BB 2. This is a real penalty for accepting another battle on the same schedule, not proof that a player would choose to do so.

A base control uses 2 SB and 4 CA against four waves of 10 CA, holding to destruction. At three-year spacing, base RT 2 produces 92.5% final wipe probability; RT 3 produces 98.8%. This is deliberately severe pressure. Bases cannot self-assist; do not give them the longest recovery merely because they cost the most.

Certain immediate pursuit increases the mixed fleet's three-year losses from 161.20 points with uniform 2 to 196.61 with class RT. The ordering survives this harsher escape assumption. A full quiet starbase year between two-year encounters gives uniform 2 the same recovery opportunity as a three-year field gap. Actual ability to reach and remain at a base is a movement question, not something these rows measure.

With 5,000 trials, the largest approximate 95% binomial half-width is 1.4 percentage points for an unconditional probability. Conditional measures have fewer observations where fleets die. Decimal loss means are reproducible estimates, not exact expectations. No measured campaign distribution of encounter gaps exists here. Do not convert this table into a galaxy-wide percentage of damaged fleets without one.

## Recommendations and validation

Decision 4 is now approved in [59v787ut](../decisions/59v787ut-defender-first-ground-combat-on-the-shared-table.md). Claude and Codex agree. Reports retain whole armies. Support fire and collateral are separate decisions.

Uniform RT 1 control, at a two-year gap: the mixed fleet loses 105.49 points and is wiped in 9.4% of trials, versus 183.85 and 42.1% under RT 2. The capital fleet loses 117.50 points and is wiped in 13.8%, versus 181.35 and 38.8%. Both RT 1 forces return whole; both RT 2 forces return damaged. At a three-year gap, uniform 1 and 2 give identical results. This is the expected timing effect. It does not establish which gap real campaigns most often produce.

The joint initial RT recommendation was uniform 2, including auxiliaries and starbases. The owner approved that value, then chose a single game-wide rule in the RT ADR. Do not add a per-class implementation stat or an RT display. Class-specific values would require a later playtest and decision; its measured benefit is modest. Do not add BB 3 as the initial price of being a capital ship. Uniform RT 1 is a control, but it removes the strategic use of the approved starbase assistance. RT 2 is the shortest value that makes that assistance useful under the approved timing. Player wording: “A ship damaged this year fights damaged next year and the year after, then returns to full strength. A full qualifying year at a friendly starbase speeds that recovery.”

Validation covers the single-army defender-first probability by independent direct enumeration, simultaneous symmetry, an actual one-round cap for independent fire, current ground odds against the earlier implementation, deterministic cover cases, cohort allocation against the uninstrumented fleet model over 50 seeds, and first-return timing. Tests do not substitute for future engine comparison or playtests.

## Evidence and connections

- [Ground tool](../../../tools/combat-model/ground_calibration.py), [JSON](../../../tools/combat-model/nc_ground_calibration.json), and [tables](../../../tools/combat-model/nc_ground_calibration.md).
- [Invasion price tool](../../../tools/combat-model/invasion_price.py) and [results](../../../tools/combat-model/nc_invasion_price.json).
- [RT campaign tool](../../../tools/combat-model/rt_campaign.py) and [results](../../../tools/combat-model/nc_rt_campaign.json).
- [Calibration tests](../../../tools/combat-model/test_calibration.py) and [reproduction commands](../../../tools/combat-model/README.md#ground-and-rt-calibration-t068).
- [Decision 1](../decisions/ld2f252j-flat-combat-effectiveness-with-battle-local-carry.md), [decision 2](../decisions/kiswrzos-withdrawal-pursuit-and-one-strength-yardstick.md), and [decision 3](../decisions/p01cg7ll-rt-controls-automatic-ship-recovery.md) supply the fleet and recovery contracts.
- [Earlier withdrawal and recovery investigation](7y0ypr6b-combat-withdrawal-and-persistent-damage-after-decision-1.md) supplies timing tests and the rejected repair alternatives.
- [Source synthesis](ebrogywq-nc-combat-design-review-against-its-sources.md) raised the ground choice. This note challenges its candidates with odds, scale, and economic commitment.

Decision-7 hull controls: [destroyer durability under withdrawal and recovery](wnwxz2et-destroyer-durability-under-withdrawal-and-recovery.md) tests DD DS 8 with this approved timing and retreat contract, including convoy survival and repeated-contact wipe risk.
