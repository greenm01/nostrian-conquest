---
id: 0t08owtu
date: 2026-10-10
kind: investigation
status: investigating
tags: [investigation]
---
# Economy review decisions and campaign pacing scope

Review six economy decisions in order: accounting, the tax/development tradeoff, starbase economics, conquest rewards, bombardment damage, and prices/opening resources. Keep one empire tax rate as the player's economic control. Use existing production, potential, local treasury, builds and missions. Treat classic as a documented comparison, not the target balance. Decide scope with Claude before new numerical runs, then present one joint recommendation at a time. Economic blockade is an optional later experiment, not part of the initial six decisions.

## t082

This analysis slice supports [t051](../plans/q0a0mhm2-classic-follow-ups-from-the-october-2026-oracle-work.md#t051). The scoping deliverable is complete. Claude accepted the six decisions and requested joint accounting/development controls in [t083](1e8xu4w6-fractional-economy-accounting-and-tax-development-controls.md). This note contains no simulation results or policy changes. Claude challenges the draft and records owner-approved decisions.

## Fixed inputs

- One empire-wide tax rate. Existing build, commission, movement and mission choices remain. No investment menus, research budgets, per-world taxes or new economic orders.
- RT 2, automatic recovery without repair bills or yards. War costs destroyed hulls, lost cargo and territory, interrupted production and time spent at reduced strength. Qualified starbase recovery assistance stays as approved.
- DD AS 10 / DS 8 / build cost 5; all other approved combat values remain fixed. Current prices are the baseline, not a pretext to reopen combat decisions.
- Bombard's industrial role and delayed battery-first dock shield are approved. Its current rate of one production and 100 stored goods per ten hits is specifically open to this review. Ground and support-fire tables stay fixed.
- t056's approved build contract: the planet budget limits new commitments; no separate annual starbase count limit; classic counts import as owed points. Do not confuse multi-year completion of an existing queue with permission to place an order exceeding today's budget. Use t056's final implementation for conformance checks.
- Local treasuries, one pre-tick economy/build phase, and the 52-tick movement year stay. No empire purse, transfers, ship-era unlocks, hidden catch-up subsidy, leader penalty, upkeep or support cap in the initial scope.

## Proposed owner decisions, in priority order

| Order | Question the owner decides | Why it matters | Evidence needed before recommendation |
| --- | --- | --- | --- |
| 1. Accounting | Keep today's repeated whole-point rounding, or retain fractional production/revenue internally and round only at stated spending/display boundaries? | Tiny worlds lose fractional income; rounding creates tax breakpoints and magnifies small base bonuses. This affects fairness without needing another control. | Enumerate tax rates and small/mature potentials; compare existing rounding with a fractional candidate, including caps, negative imported goods and classic export. Show whole-point BBS examples and specify forecasts in existing planet/empire fields and tax feedback; no new columns or menus. Do not select coefficients here. |
| 2. Tax and automatic development | How fast should colonies become useful, and how much future output should emergency high taxes sacrifice? Keep or retune the current curve, 65% threshold and minimum-growth behavior as one calibrated choice. | Low taxes should buy later strength at a real early military cost. High taxes should offer an understandable emergency option. A hidden best percentage is poor strategy. | Compare young, mixed-age and mature empires; constant rates and limited reactive policies. Measure development, cumulative income, first local defense and reinforcement timing. Test decisions 1 and 2 together before calling either balance-neutral. |
| 3. Starbase economic role | Keep the current low-tax growth bonus and 5x local build capacity, or tune the growth benefit while preserving the intended construction role? Do multiple bases provide any extra economic benefit? | Bases already defend, aid RT recovery and turn local reserves into a surge. Unlimited annual builds must not accidentally create compounding economic bonuses. Capacity is not revenue. | Compare no base, one and several bases at young/mature worlds, reserve-rich and reserve-poor worlds; include cost, commissioning and travel. Specify eligibility and non-stacking explicitly; do not reopen combat or RT assistance. |
| 4. Conquest payoff | Keep NC’s two full post-capture economic passes without growth/revenue (default baseline), or use a graded conversion such as classic’s 0/half/full yield? What usable local savings and queued work does the winner receive? | A successful invasion should earn a useful prize without funding an immediate runaway advance. Recovery, garrison replacement and travel determine whether a defeat is recoverable. | Trace capture in early/late ticks, recapture, retained industry/savings, queue progress and next spending dates. Compare time to a useful captured-world reinforcement and attacker payback. Separate verified current behavior from proposed changes. |
| 5. Bombardment's economic damage | Keep or retune production and treasury losses per bombardment hit, including how damage rounds across repeated exchanges? | Raiding should matter, but a cheap visit should not erase years of savings without a credible defense. Excessive destruction can make fighting less rewarding than waiting or scorched-earth play. | Price a raid and recovery against invader capture value, batteries and orbital defenses; test small repeated strikes, protected reserves and devastation across world sizes. Account for growth rebuilding production but not immediately replacing lost goods. Keep the approved battle rules fixed. |
| 6. Prices and opening resources | After the above rules are calibrated, are current costs and starting resources appropriate, or is there a specific price/opening adjustment worth a separate playtest? | Unit access must fit growth, travel and threats. Artificial early/mid/late ship eras would undermine the simple game. An ETAC investment must trade current safety for a worthwhile frontier. | Measure actual purchase, completion, commission and arrival, not just nominal affordability. Compare guarded expansion, rapid expansion, early pressure and reserves on paired maps. Include ships, armies, batteries, bases and reusable ETACs. Default to unchanged prices unless a specific failure survives the earlier decisions. |

Decision 3 starts from keeping 5x capacity and non-stacking economic benefits, not removing the established rule. A change to that settled capacity rule would require an explicit owner choice, not a side effect of rounding or t056. Decision 6 likewise does not silently reopen approved DD cost 5: evidence for a price change must identify it as a separate supersession/playtest.

Optional, only after these controls show an unmet role: should sustained orbital blockade reduce next year's revenue through the existing mission? This is a new economic rule, even if automatic. Do not bundle it into bombardment calibration or quietly add supply, upkeep or deficit management. First ask whether bombardment, interception and threatened invasion already provide enough profitable disruption.

Classic already stores fractional production at PLANETS 0x03 and goods at 0x09 as Real48. Fractional internal accounting is closer to classic, not a new control or a claim that NC must use classic’s growth formula. Its status counter matches NC, but its yield does not: classic pays 0/half/full, NC currently 0/0/full. Claude confirmed this correction; item 4 must not claim economic parity. The misleading ECMAINT-parity comment in `maint/economics/planets.rs` is for the item-4 implementation, not this analysis or the concurrent t081 rename.

Items 4 and 5 must jointly assess whether war remains costly enough through hull losses, time, captured worlds and bombardment, without upkeep or repair bills. Baseline comparisons must include limited raids, a won invasion and a lost invasion, not only peaceful growth.

## Baseline reconciliation before numbers

Read the [manual](../../manuals/nc_player_manual.typ), [economics spec](../../dev/economics.md), engine/helpers and final t056 implementation together. Record differences; do not choose whichever is easiest to model. The older [investigation](tkce84f0-a-simple-nc-economy-informed-by-classic-ec-vbam-and-ec4x.md) and [model README](../../../tools/economy-model/README.md) are evidence at their stated revision, not current engine conformance.

Known issues to resolve in that baseline:

- The old note's paid-repair recommendation is rejected by the RT decision. Its repair results remain historical controls only.
- The manual correctly says one empire tax rate, but also advises lowering taxes on individual colonies. Its unconditional minimum-growth wording hides the high-tax penalty.
- The economics spec still calls classic formulas opaque and refers to a post-loop update despite its newer pre-tick section. The decoded classic reference now exists.
- The spec describes fresh production 0; the retained model starts at 1. Check the current colonization path before repeating the 77-versus-37.5 comparison.
- The engine caller computes the penalty on old production. The spec's sequential wording can imply post-growth production. Freeze the intended baseline explicitly.
- The old colony-BB milestone means order admission, not a ship delivered to battle. Update assumptions for t056 without treating partial construction as unrestricted credit.

These checks do not require owner votes on obvious documentation contradictions. If two legitimate gameplay contracts conflict, bring the specific choice back rather than silently changing the rule. No manual or spec edits are made in this scoping slice.

## Evidence standard and campaign arc

Use exact arithmetic/enumeration for tax thresholds, deterministic growth, queue timing and capture schedules. Monte Carlo is for geography, incomplete information, policy choices and combat uncertainty, not a reason to randomize the economy. Keep economy coefficients predictable and document exact chosen formulas in Appendix A; fit ordinary feedback into existing BBS screens and reports.

First compare selected model cases to the NC engine/harness once the relevant implementations land. Separately compare classic reference cases against retained oracle evidence or matched ECMAINT probes. The previous Python comparison did not establish model-to-oracle agreement. Tag every result as formula analysis, model simulation or engine/oracle evidence.

Campaign comparisons should use paired seeds and mirrored seats, actual map generation and movement where available, and several policies. Report unreached milestones as well as conditional medians. Exploration, expansion, development and war should overlap, not unlock on fixed dates. Measure time to useful scouting, first contested colony, local defense, colony-majority revenue, delivered replacements and a recoverable defeat. Include repeated fights within one economic year and RT downtime; do not charge rejected repair bills.

The earlier findings motivate this review: fast NC development relative to classic, 70/71% tax discontinuities, and ETAC travel sensitivity. They are not new measurements or calibration targets. A campaign lead earned through risk should remain valuable. Test whether opponents have meaningful responses, not whether all policies or seats end equally wealthy.

## Source lessons, not imported systems

From EC4X, retain the useful separation between productive capacity, revenue and local construction, and use diminishing returns through current production versus potential. The owner designed that economy; consult its actual rules when selecting candidates, rather than treating an old summary as current authority. From VBAM, measure development payback and the economic consequences of war. Do not copy upkeep, purchased development or repairs that conflict with approved NC scope. Classic supplies reference behavior and operational timing, not the desired growth curve.

## Connections

- [RT](../decisions/p01cg7ll-rt-controls-automatic-ship-recovery.md): automatic recovery, not repair expenditure.
- [DD values](../decisions/8pc4t661-destroyers-become-ds-8-for-the-first-playtest.md): unchanged cost 5 is the baseline.
- [Bombardment](../decisions/mk1ie4e3-batteries-shield-the-stardock-and-failed-landings-hold-orbit.md): industrial damage rates delegated here.
- [Build queue](jladc1qu-build-queue-translation-between-rust-slots-and-classic-counts.md#owner-decisions-2026-10-10): approved queue limits and compatibility semantics.
- [Classic economy reference](../../dev/ec-economy-reference.md): probe-confirmed versus disassembly-only claims remain distinct.
