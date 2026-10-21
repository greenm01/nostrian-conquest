---
id: y95fnqkm
date: 2026-10-10
kind: investigation
status: investigating
tags: [investigation]
---
# Unit prices and opening resources under the approved economy

Keep all prices and the joined opening package unchanged. The approved economy already makes new colonies useful, and the combat decisions corrected destroyer durability without changing its price. Fifty opening points fund several different choices; buying extra colonizers trades immediate military resources for faster expansion. The tested small ETAC price changes do not improve arrival dates. Large battleship price changes would substantially alter where and when capital ships become available. No measured failure here warrants them. These controls support a conservative starting balance, not a claim that campaign strategy or map fairness is solved.

## t088: recommendation for joint presentation

1. Keep DD5, CA15, BB45, SC15, TT5, ETAC20, starbase50, army2 and battery20. Keep the approved DD DS8 and other combat decisions; no new price change or artificial ship-unlock era.
2. Keep the joined opening: P/K100 homeworld, goods50 at default tax50,10 armies,4 batteries, two CA+ETAC fleets and two lone DD. The unjoined ECUTIL goods35 is a different state. Reducing joined goods to35 delays an opening BB, base or second extra ETAC by one pass.
3. Keep the expansion tradeoff. At goods50, two extra bare ETACs finish in pass1 and depart in pass2; two new CA+ETAC pairs finish in passes1/2 and depart in2/3. Across1,000 paired peaceful synthetic maps, eight colonies arrive at mean pass7.52 for bare expansion,7.79 with escorts,11.13 using only the two starters. Those are opportunity dates, not equal survival odds.
4. No price intervention is supported. ETAC15/20/25 yields identical expansion dates in this opening policy because all buy two within50. Cutting BB45 to30 moves a K50 colony's first order from pass17 to7; raising it to60 blocks that order without a base. Keep the established roles and test the implemented rules on actual maps before tuning them.

Claude found the preliminary recommendation sound and requested the completed note. No disagreement has been raised; owner approval is still needed. This is the final item of [the six-decision economy scope](0t08owtu-economy-review-decisions-and-campaign-pacing-scope.md), not an implementation task or another ADR.

## Reconciled opening and rules

The [manual](../../manuals/nc_player_manual.typ) lists the prices above and the four starter fleets. [builder.rs](../../../rust/nc-data/src/builder.rs) seeds a homeworld with production/potential100, goods35,10 armies,4 batteries and full ownership. Pre-joined empires then receive the same treasury as joining: [mutators_player.rs](../../../rust/nc-data/src/directory/mutators_player.rs) seeds one year's revenue at the fixed default join tax50, giving goods50. The caller does not let the player select a high tax for this grant. The review therefore uses50, not35, for the normal joined game. It does not change classic initialization or existing saves.

The starting commissioned ships are2 CA,2 ETAC and2 DD, worth80 build points; armies add20 and batteries80. Those assets are not a spendable180-point treasury. No base, scout, battleship or transport is granted. The initial cruisers let both starter colonizers have escorts without buying them. Additional unescorted colonizers increase exposure, even though the peaceful model cannot inflict those losses.

Controls use approved P3 founding, smooth fractional development, whole-point spending and nearest-whole production capacity. Tax is50 throughout. No base is purchased in the map benchmark, so the starbase taper does not alter it. Colonizers are reusable and speed3. No paid repairs or upkeep. Scorch, capture and bombardment are absent from this peaceful control; their costs are in [item4](et03gvm9-conquest-income-inherited-assets-and-bombardment-economics.md) and [item5](wrk1xd3p-bombardment-raid-costs-and-recoverable-scorch-earth.md), not silently omitted from a claimed war simulation.

## Opening orders, completion and deployment

Orders made before pass1 can finish in its pre-tick build phase. These controls assume manual commissioning in the order window after completion, followed by departure next pass. No mid-maintenance player orders or automatic same-pass deployment. Every example below is an alternative use of the opening treasury, not a list of simultaneous purchases.

| Opening plan at goods50 | Cost | Completion pass | Earliest departure pass |
| --- | ---: | --- | --- |
| Two extra bare ETACs | 40 | Both1 | Both2 |
| Two extra ETACs, each with a new CA | 70 | First pair1, second2 | First2, second3 |
| One BB | 45 | 1 | 2 |
| One base | 50 | 1 | Commission after1; already at its world |
| One CA and one battery | 35 | 1 | CA2; battery defends during1 |
| Six TT and six new armies | 42 | 1 | 2 after loading |
| Six CA | 90 | Three1, three2 | First group2, second3 |
| Three BB | 135 | One each in1,2,3 | One each in2,3,4 |
| Ten CA | 150 | Three1, three2, four3 | Groups2,3,4 |

The troops could instead come from the initial garrison, saving12 points in the six-TT example while weakening home defense. Neither choice is free. A base uses the whole opening budget but does not multiply full-homeworld income; it spends accumulated reserves faster and provides military benefits. Building a BB is possible immediately, but its slower arrival and opportunity cost are real. There is no need to invent a technology gate to manufacture a late battleship era.

Travel follows the previously documented ideal full-orbit benchmark `ceil((Euclidean distance+2)/speed)` whole maintenance passes. At distance3, a newly completed CA departing in pass2 arrives in2; a BB or ETAC arrives in3. At distance8, their arrivals are pass3 for CA,4 for BB,5 for ETAC. Actual classic tick rounding, the colonization depth shortcut and interceptions can change those dates. They are not a substitute for the game's ETA calculation.

Joined goods35 buys only one of the two20-point ETACs initially; the second finishes in2. It also postpones BB45 and base50 until2. Goods65 adds reserves but still cannot buy two35-point guarded expansion pairs together. Goods75 can. Raising starting money enough to remove that distinction would subsidize expansion and escorts together; the current50 leaves a clear choice.

## Local development and unit access

These exact arithmetic dates start from a newly founded P=min(K,3) colony with no savings. The first economic pass is the year after founding. Each entry is the first order window after a pass in which an otherwise unspent local budget admits one whole unit. Completion follows next pass. Ships can be commissioned after completion and depart in the following pass. Each unit is an independent alternative; reserves are not spent on earlier units in the table.

| Potential | DD / TT5 | CA / SC15 | ETAC / battery20 | BB45 | Base50 |
| --- | ---: | ---: | ---: | ---: | ---: |
| 10 | 3 | Never without base | Never without base | Never without base | Cannot build its own |
| 20 | 2 | 9 | 27 | Never without base | Cannot build its own |
| 50 | 2 | 3 | 4 | 17 | 35 |
| 100 | 1 | 2 | 2 | 5 | 5 |

An army can be ordered after pass2 on K10, and after pass1 on K20/50/100. Surface defense completes and is available before that completion pass's ticks. A K100 colony's BB ordered after5 completes6, commissions after6 and can depart7. This is more useful information than “affordable in year5.” A K50 colony can begin regional cruiser production much earlier than battleship production.

A small world can accumulate more goods than its capacity allows it to spend in one order. Approved queue installments do not authorize an order that exceeds its admission budget. Thus a K10 world cannot locally start a15-point cruiser without an eligible base, however much it saves. A commissioned imported base increases capacity fivefold; small-world capital production depends on that deliberate infrastructure choice. It is not fixed by lowering every price. These restrictions and nearest-whole capacity need accurate documentation so a player does not mistake savings for an unrestricted build budget.

**Amended 2026-10-21 (t207), owner decision:** one exception. A world with no starbase in orbit or in its stardock may pay for one starbase (50 points) from its treasury outside its build capacity. The rule above made a small world's first base impossible to build locally, and a base shipped from the homeworld crawls one sector a year. Other orders keep the full capacity but share the treasury that is left; a second base follows the standard rules. The cruiser example above still holds. Code: `CoreGameData::planet_spend_limits` and `planet_build_points_left_for` in `nc-data`, used by turn admission and maintenance; tests in `rust/nc-data/tests/first_starbase.rs`; the player manual's build-capacity sections say the same.

Alternative BB prices expose how large the intervention would be:

| Potential | Order BB30 | Order BB45 | Order BB60 |
| --- | ---: | ---: | ---: |
| 20 | Cannot without base | Cannot without base | Cannot without base |
| 50 | 7 | 17 | Cannot without base |
| 100 | 3 | 5 | 7 |

The approved combat calibration already gives BBs higher durability per point than CAs and stronger bombardment, balanced partly by speed and concentrating strength into fewer hulls. BB30 would buy50% more battleship AS per point than BB45. There is no demonstrated need for that buff. BB60 would increase its price by a third and remove native access on K50 worlds. These are role changes, not harmless pacing adjustments.

## Paired peaceful expansion controls

The benchmark reuses [campaign.py's geography](../../../tools/economy-model/campaign.py): a centered home on a31-by31 grid,24 distinct raw worlds with potentials uniformly sampled from10 through150. All locations are known. Ships choose the nearest unreserved world; no rival contests it. The same1,000 map seeds880001 onward are used for every price/opening policy at the same density. These are synthetic sensitivity maps, not NC generator output or evidence of equal starting positions.

Additional colonizers are bought only at home; their costs, and new escorts where specified, are deducted. They complete before that pass's income and become available for orders next pass. Existing starter ships can leave in pass1. Arrival founds a P3 colony with zero goods; it first earns income next pass. Colonizers cannot take a new player order midway through a pass. Colonies do not buy further ships in this control. Local reserves are never pooled into home funds.

| Policy | Extra capital purchased | First colony | Four colonies | Eight colonies | Colony income reaches home's50 |
| --- | ---: | ---: | ---: | ---: | ---: |
| Use two starter ETACs | 0 | 2.20 | 5.58 | 11.13 | 7.18 |
| Buy two extra bare ETACs | 40 | 2.20 | 4.49 | 7.52 | 6.33 |
| Buy two extra CA+ETAC pairs | 70 | 2.20 | 5.03 | 7.79 | 6.52 |

All listed milestones were reached within25 passes on all sampled maps. These are sample means, not universal dates. Starter-only first-colony p10–p90 is1–3; four colonies5–7; eight colonies10–13. Rapid expansion's eight-colony range is7–8; guarded expansion7–9. These quantiles describe map variation, not confidence intervals.

At ETAC15,20 or25, two extra bare ships all fit goods50 and finish together in pass1. Expansion dates are identical; home reserves after pass5 are270,260,250. This price range changes opportunity costs but does not fix a movement delay in the tested opening. ETAC30 or starting goods35 delays the second extra ship one pass: four-colony mean becomes5.03 and eight7.79. Starting goods65 does not accelerate bare expansion; goods75 accelerates the two new guarded pairs to the bare-expansion dates.

Density changes matter more here than small price changes. With only two starter ETACs,12 raw worlds give first-colony mean2.63 and eight-colony mean15.18;48 worlds give1.90 and9.17. These are separate density samples, not paired identical positions or real map fairness tests. Moving ships across empty space cannot be repaired merely by giving the player a few more starting points.

## Human choices and the stages of a 4X game

The early stage starts with useful tools rather than waiting for the first colonizer. Players decide where their two escorted expansion fleets go, whether the DDs scout or remain nearby, and whether the opening50 buys speed, defense or striking power. Scouts cost as much as CAs but buy reconnaissance and stealth, not combat AS. This study does not measure information value, so zero AS is not evidence for a scout discount.

Economic consolidation emerges as colonies start funding cheap ships and local defense. A K100 colony can order a CA after two productive passes while a K20 world takes nine. Shipyards and forward bases therefore emerge unevenly across the map. Colony revenues overtake the homeworld around passes6–7 in these uncontested controls; that marks diversification of income, not a global trigger for a new era.

Pressure and conquest overlap expansion. An opening BB can arrive before many colonies have developed defenses; a neighboring defended homeworld is a different target from an empty new colony. The [raid controls](wrk1xd3p-bombardment-raid-costs-and-recoverable-scorch-earth.md) show both profitable exposed targets and costly defended assaults. The opening army/battery package helps explain why home and frontier risks differ, but these peaceful maps do not prove that every generated home spacing resists a rush.

Later play should turn on routes, local reserves, reinforcement distance, RT recovery, holding captured worlds through conversion, and whether to preserve or destroy their output. It should not require a late-game menu or an artificial universal ship progression. Prices alone cannot guarantee an exciting endgame; actual campaigns must measure leader conversion of territory into force and the defender's chances to recover.

The faster bare expansion policy is not demonstrated to be dominant. Its two extra fleets have no new escorts, while guarded expansion spends30 more on cruisers that retain military value. The benchmark cannot price intercepted ETACs, missed contacts, surprise or lost colonies. A reserve policy retains40–70 points for other choices; its higher unspent cash is not a measured victory advantage either. This is why the recommendation is to retain the prices and test the implemented interactions, rather than tune against a peaceful race alone.

## Validation and follow-up

[opening_review.py](../../../tools/economy-model/opening_review.py) writes [opening_review_results.json](../../../tools/economy-model/opening_review_results.json):13 synthetic-map configurations,1,000 maps each, exact local milestones, alternative BB costs and actual home purchase/completion schedules. It leaves historical economy results unchanged. Production/income on the maps use the closed-form smooth tax50 trajectory; local admission dates use exact fractions.

[test_opening_review.py](../../../tools/economy-model/test_opening_review.py) checks opening affordability, completion versus deployment, colony dates and capacity limits, price/date plateaus, actual expansion spending and censored milestones. `python3 -m unittest discover -s tools/economy-model -p 'test_*.py'`:66 tests pass, seven new. No Rust changes or cargo. No new classic-oracle or engine-conformance claim. Current manuals still contain previously identified stale per-colony-tax and pre-redesign combat wording; the approved implementation work owns those reconciliations.

No model here measures optimal aggressive play against an adaptive rival. Once the approved engine rules land, use actual generated maps and scripted policies to compare expansion, escort, reserve and early-pressure openings. Record commission/departure/arrival dates, permanent losses, surviving ETACs, colony retention and regional reinforcement time. The decisive failure signal would be an opening that stays superior after its military exposure is priced, or home spacing that makes defense futile. Neither has been established here. Keep prices and resources unchanged unless that evidence identifies a specific correction.
