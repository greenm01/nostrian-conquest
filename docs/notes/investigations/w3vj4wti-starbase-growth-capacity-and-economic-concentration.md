---
id: w3vj4wti
date: 2026-10-10
kind: investigation
status: investigating
tags: [investigation]
---
# Starbase growth, capacity and economic concentration

Keep the current starbase economic role: a 50% growth bonus at tax50 or below, tapering continuously to zero at65, and 5x local build capacity. Neither benefit stacks with more bases. Under the approved smooth/P3 economy, growth can repay a base on a promising young world, but usually cannot justify one on a small or mature world. Capacity converts saved local money into a defensive surge; it does not multiply annual income. Defense and automatic recovery assistance add value already approved elsewhere. No price change or new economic control is warranted by these controls.

## t085

Decision3 of [t051’s scope](0t08owtu-economy-review-decisions-and-campaign-pacing-scope.md). Inputs are approved [fractional accounting, smooth growth and P3](../decisions/ubnjcchu-fractional-economy-accounting-smooth-growth-and-a-colony-head-start.md), base cost50, existing physical eligibility, and the approved combat/RT rules. No Rust edits or engine/DOS runs. The controls use exact rational arithmetic and no randomness. They measure economic opportunity, not campaign win odds.

## Recommendation to Claude

1. Keep 5x capacity and the existing intended growth taper: +50% through tax50, +50% times `(65-tax)/15` between50 and65, zero at65+. Apply the taper without intermediate rounding under the approved smooth economy.
2. Treat both effects as presence benefits: one qualifying owner base is enough; two or ten do not add growth or raise capacity above5x. Extra bases buy military resilience, not another economic multiplier.
3. Keep the price50 and existing commissioning/eligibility rules for this decision. A docked base gives neither benefit. A completed base must be commissioned; growth eligibility is evaluated before the economic pass. Do not grant an arrival a retroactive bonus or require a full quiet year for economics merely because RT assistance has that separate condition.
4. Report through the existing Budget/Revenue fields and manual: a base spends reserves faster, helps an underdeveloped world grow, and does not generate money at a fully developed world. No new menus, upkeep, repair charges or economic damage scaling is proposed.

Claude agrees with the rates, non-stacking and retaining binary economic benefits for crippled bases while they survive and otherwise qualify. Combat AS remains reduced under the approved combat rule. RT assistance eligibility remains its separate decision. The follow-up cycle and source checks below complete the requested review; the owner has not approved item3 yet.

## Exact growth model

Without high-tax damage, let `a=(100-tax)/400`, growth bonus `b`, potential K and starting production P0. Without a base, the gap after n passes is `(K-P0)*(1-a)^n`; with a continuously eligible base it is `(K-P0)*(1-a*(1+b))^n`. Revenue is tax times post-growth production, retained fractionally.

The lifetime extra tax income from a base present from the first pass is:

```text
extra_income_limit = (tax/100)*(K-P0)*b / (a*(1+b))
```

This sums future income without discounting. If the limit is below the50-point price, growth alone never pays for the base, even in a peaceful campaign of unlimited length. At tax50, the limit is `4/3*(K-P0)`. At65+ the growth bonus is zero. At full production the base changes neither growth nor revenue at any non-damaging tax.

A base already owned is a sunk cost; redeploying it need not pay another50. The purchase-payback comparison below instead charges the full price once and asks whether extra local revenue alone earns that cost back. It omits defense, recovery support, construction speed, risk and alternative deployments. Increased revenue on one world is not transferable cash for the world that built the base.

## Growth benefit at tax50

New colony P3. The base is assumed already commissioned and eligible for the first economic pass. This is a favorable placement control, not a claim that a fresh colony can build it immediately.

| Potential | Extra income after10 passes | Extra after20 | Lifetime extra limit | First pass extra income covers cost50 |
| --- | ---: | ---: | ---: | ---: |
| 10 | 4.79 | 7.88 | 9.33 | Never |
| 20 | 11.63 | 19.13 | 22.67 | Never |
| 50 | 32.16 | 52.88 | 62.67 | 18 |
| 100 | 66.37 | 109.14 | 129.33 | 8 |
| 150 | 100.58 | 165.40 | 196.00 | 6 |

For K100/P50, growth-only payback is pass17 at tax50. At tax35 lifetime extra revenue is35.90, below cost50; at tax60 it is42.86; at65 it is zero. Faster development and faster cash payback are different goals. Because tax is empire-wide, these figures do not justify lowering a mature empire’s rate just to develop one base world. That would forgo revenue on all its other worlds too.

The controls therefore support a base as a useful commitment to a promising colony or a strategic position, not a mandatory investment on every rock. A poor frontier world may still justify one for defense, construction surge or recovering fleets. Those benefits need not each repay the entire cost independently.

## Arrival and local construction matter

For K100/P3 at tax50, delay means that many ordinary economic passes occur before the base starts helping. It is an imposed delay, not a simulated travel route.

| Unboosted passes first | Extra income by pass10 | Lifetime extra limit | Purchase payback pass from colonization’s first economy |
| --- | ---: | ---: | ---: |
| 0 | 66.37 | 129.33 | 8 |
| 2 | 40.27 | 99.02 | 12 |
| 5 | 15.19 | 66.34 | 22 |
| 10 | 0.00 | 34.02 | Never |

At base speed1, orbital climb/descent and travel can consume much of the young-world opportunity. These delay rows do not validate engine ETAs. They show why free immediate placement exaggerates economic returns.

A separate local-build control starts P3 and treasury0, saves all revenue, orders a base as soon as the existing local budget admits50, completes it at the next pass, and commissions it afterward for the following economic pass. No base acts while still in the dock; no instant mid-pass commissioning is assumed.

| Potential, tax50 | Order after pass | Complete during pass | First growth bonus pass | Growth-only cost payback pass |
| --- | ---: | ---: | ---: | ---: |
| 10 or20 | Cannot admit50 without an arriving base | — | — | — |
| 50 | 35 | 36 | 37 | Not reached in60 passes |
| 100 | 5 | 6 | 7 | 27 |
| 150 | 3 | 4 | 5 | 13 |

This is why construction capacity is meaningful geography. A small colony with large savings cannot locally order its first base until its capacity permits it; an imported base changes its industrial options. The table preserves t056’s order budget and does not invent borrowing or unrestricted new multi-year orders. K50’s rounded capacity eventually reaches50 under the approved nearest-whole convention.

A locally built K100 base still trails the no-base treasury by10.30 at pass20, despite providing defense and faster build capacity in the meantime. This is a substantial opportunity cost. There is no evidence here that the growth bonus needs to be reduced to keep expansion risky.

## Capacity is a reserve surge, not income

A mature P100 world at tax50 earns50 per pass with or without a base. Spending occurs before that income is credited.

| Starting local goods | Qualifying bases | Spending in passes1–5 |
| --- | ---: | --- |
| 50 | 0,1 or3 | 50,50,50,50,50 |
| 500 | 0 | 100,100,100,100,100 |
| 500 | 1 or3 | 500,50,50,50,50 |

These are generic build-point throughput controls, not an assertion that arbitrary unit mixes complete or commission in one pass. Capacity is100 without a base,500 with any qualifying base, capped by available whole goods. Multiple bases leave that cap unchanged. Builds plus remaining goods always equal starting goods plus earned income.

A reserve-rich world can produce a large response after a base arrives or is commissioned. That makes scouting, reserves and attacking a construction center matter. It also creates concentration incentives. Local savings and travel constrain them; this control does not prove they suffice in a competitive campaign. Do not counter them with upkeep or a stack penalty in this decision. The pending bombardment review must price the vulnerability of those reserves and docked ships under the approved battery shield.

## Alternative growth strengths

For the favorable K100/P3, tax50, immediate-base control:

| Maximum growth bonus | Extra income after10 | Purchase payback pass | Lifetime extra limit |
| --- | ---: | ---: | ---: |
| 0% | 0.00 | Never | 0.00 |
| 25% | 36.18 | 15 | 77.60 |
| 50% current intended | 66.37 | 8 | 129.33 |
| 100% | 112.88 | 5 | 194.00 |

Halving the bonus makes a delayed growth investment much harder to justify. Doubling it pushes richer frontier worlds toward an early base rush. Neither alternative has a demonstrated gameplay need. Keep50% for the first economy playtest; do not tune away a useful base role just because the unit also fights and aids recovery.

## Eligibility and persistence boundaries

The current source has two deliberately distinct tests, which the spec describes:

- Growth: owner’s commissioned base with nonzero ID, same sector, layer1 (`Z>=0.5`), and moving flag clear, sampled before the ticks.
- Capacity: owner’s commissioned base with nonzero ID, same sector and layer1, targeting that world’s orbit. The build-screen rule does not test the moving flag; a base descending toward its destination can count for capacity before it qualifies for growth.

Both use presence, not base count. Their source paths are `maint/economics/mod.rs::starbase_planets` and `directory/economy.rs::planet_has_friendly_starbase`. Economic checks are not RT’s whole-year/same-base eligibility. A foreign or deep-space base is not an economic base for that world. No proposal here normalizes these rules silently. If the owner wants one simpler economic eligibility test, present that as an explicit follow-up change with arrival examples; it is not required to retain the current rates and non-stacking rule.

A commissioned surviving base may lose combat strength under RT. No existing economy rule scales the civilian bonus by combat strength. The implementation needs to pin this explicitly when damage persistence lands. Keep one or more qualifying bases equivalent economically; do not let destruction or merging refresh money or multiply benefits. Extra bases can provide military redundancy, but this review does not compute the marginal survival benefit or RT capacity.

## Player explanation

“A base at rest in orbit makes its world grow half again as fast at taxes of50% or less. The extra growth fades to zero as taxes rise to65%. A base lets the world spend saved points up to five times its production. More bases at one world add defense, not growth or build capacity.” Add the existing arrival exception in the manual: a base descending toward its own world can enable the larger build budget before it settles enough to help growth.

Use existing budget and next-revenue fields, and Appendix A for the taper and eligibility distinction. No additional status column, investment command or economic stat is needed. A mature world’s unchanged revenue alongside a larger budget should make the different roles clear.

## Follow-up: tax cycles with the smooth taper

Continuity alone does not prove tax cycling is unprofitable. We tested the two cycles that exposed the gated minimum, with one base at a K100 world starting at P50 or P100. No spending, war or change in ownership/base eligibility occurs. We also retain no-base controls in the JSON. Taxes are empire-wide in the game; these isolated-world results are not an empire policy ranking.

| Starting P | Repeating taxes | Extra income over steady65 after one cycle | After20 cycles | Limiting mean income per pass | Steady65 limiting income |
| --- | --- | ---: | ---: | ---: | ---: |
| 50 | 75,64,64 | +0.51 | -105.07 | 62.62 | 65.00 |
| 100 | 75,64,64 | +4.29 | -96.46 | 62.62 | 65.00 |
| 50 | 70,64 | +0.86 | -40.62 | 63.24 | 65.00 |
| 100 | 70,64 | +2.72 | -34.71 | 63.24 | 65.00 |

Twenty cycles means60 passes in the first pair and40 in the second. The comparisons use the same starting production and base on both policies. The partly developed steady65 control grows; it is not incorrectly credited65 from its first pass.

These are exact limiting-cycle calculations, not just a finite cutoff. Each pass is an affine map `P_next=q*P+c` with rational coefficients. Compose the maps over the tax sequence to obtain `P_after=Q*P_before+C`. The unique limiting start of cycle is `C/(1-Q)`. Running that production through the sequence returns exactly to itself and gives the mean incomes above. The limiting start-of-cycle production is93.29051 for75/64/64 and94.66769 for70/64, below full production100.

Starting at full production, the first75/64/64 cycle ends at98.354702 and earns199.28605 versus195 for steady65. The first70/64 cycle ends at99.093 and earns132.71952 versus130. The early cash advantage is real, but the world has not recovered its starting productive strength. Repetition pays less in the long run. This preserves emergency taxation’s intended timing tradeoff rather than the rejected floor’s repeating extra income at fully restored production.

This establishes no lasting advantage for the two requested cycles. It is not a theorem covering every adaptive policy, finite campaign horizon, base placement or empire age mix. No change to the taper is warranted by these controls.

## Follow-up: where the eligibility rules come from

Both rules have classic provenance, but they belong to different executables. They are not two inventions of Rust:

- **Growth eligibility:** ECMAINT1852:011b..0206, including the moving-flag check, documented under [Starbase tax cut](../../dev/ec-economy-reference.md#starbase-tax-cut). This has disassembly and t041 probe evidence: layer1 and stationary count; deep space and moving do not. Rust deliberately requires the owner’s base instead of reproducing classic’s foreign-base bug. The50% taper is NC’s chosen benefit, not classic’s random tax reduction.
- **Capacity eligibility:** ECGAME Planet Command B, file0x12616..0x126D4, with budget calculation0x126EB..0x12843, documented under [Build screen](../../dev/ec-economy-reference.md#build-screen). The source marks this code-only evidence. It scans the player’s own base chain, matching the world, layer1 and destination orbit; it does not check moving. It is the classic order-admission rule, not an ECMAINT maintenance spending limit. Classic maintenance completes its queued orders without that capacity gate; NC applies its own budget-limited completion rule.

The reference explicitly records the orbit-test mismatch. The older [oracle audit](../../dev/starbase-economy-oracle-audit.md) reports larger imported build-capacity values, but that alone is not a direct probe of the precise moving-base eligibility predicate. Do not promote it to stronger evidence than the decoded reference.

Recommend preserving both eligibility tests in this item. Unifying on the stationary condition would deliberately change classic build-screen eligibility, delaying an arriving base’s larger budget; it would not merely clean up two Rust-only helpers. The ordinary final descent can fit within one year, but this review does not prove an unconditional “one pass at most” bound under movement interruption, changed orders or ownership changes. No source or contract needs that promise to retain today’s rule.

## Joint recommendation after Claude’s challenges

1. Keep the growth bonus: half again as fast at tax50 or below, fading continuously to zero at65; keep5x capacity and cost50. A favorable K100/P3 base pays for itself through extra income in8 passes, but a five-pass arrival delay pushes that to22; K20 cannot repay50 through growth alone.
2. One qualifying base supplies the full economic benefit; more add defense/redundancy, not extra growth or capacity. A crippled surviving base still qualifies economically. RT assistance remains separate.
3. Keep the inherited eligibility distinction: growth requires the owner’s base at rest in layer1; capacity uses ECGAME’s own-world/destination-orbit test and can count a descending base. No docked or foreign-base benefit. Explain the arrival exception in the manual and keep existing Budget/Revenue fields.
4. The tested75/64/64 and70/64 cycles with a base settle at62.62 and63.24 income per pass, below steady65. Brief emergency cash gains come with unrecovered production. No disagreement remains on rates, stacking or crippled-base economics; preserving the two classic-derived predicates is the proposed resolution to Claude’s provenance challenge.

## Evidence and limits

[Model](../../../tools/economy-model/starbase_review.py), [results](../../../tools/economy-model/starbase_review_results.json), [tests](../../../tools/economy-model/test_starbase_review.py). Run `python3 tools/economy-model/starbase_review.py` and the economy-model unittest discovery command.

38 tests pass across economy-model, including nine starbase checks for the approved smooth-growth equivalence, non-stacking, full-production revenue, the finite return bound, delayed arrival, local order/completion/commission timing cash conservation and exact limiting-cycle/temporary-gain behavior. The timing control is an explicit model assumption informed by the human commissioning flow, not an engine-run result. No Monte Carlo is needed for these deterministic controls. No live campaign, oracle validation or post-implementation parity is claimed.

## Connections

[Accounting evidence](1e8xu4w6-fractional-economy-accounting-and-tax-development-controls.md) defines the smooth/P3 inputs. [Automatic recovery](../decisions/p01cg7ll-rt-controls-automatic-ship-recovery.md) is a separate base benefit with no money charge. [Build queue decisions](jladc1qu-build-queue-translation-between-rust-slots-and-classic-counts.md#owner-decisions-2026-10-10) supply the budget rule. [Bombardment](../decisions/mk1ie4e3-batteries-shield-the-stardock-and-failed-landings-hold-orbit.md) explains why local reserves and dock throughput need risk assessment in economy item5.
