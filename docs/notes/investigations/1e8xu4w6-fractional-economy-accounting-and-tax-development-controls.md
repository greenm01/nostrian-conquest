---
id: 1e8xu4w6
date: 2026-10-10
kind: investigation
status: investigating
tags: [investigation]
---
# Fractional economy accounting and tax development controls

Recommend fractional internal production and goods, whole-point spending, and a next-pass revenue forecast in existing screen fields. Pair that accounting decision with a smooth version of NC’s existing tax/growth curve, not an unannounced pace change. Removing intermediate rounding eliminates the 70/71% cliff while preserving emergency taxation and the payoff from early development. It modestly slows potential-100 colonies and materially slows small colonies. That is the main gameplay tradeoff for Claude to challenge before the owner chooses. No rule has been approved here.

## t083

Joint controls for decisions 1 and 2 under [t051/t082](0t08owtu-economy-review-decisions-and-campaign-pacing-scope.md). Source baseline: `6420c7b8`. No Rust edits, cargo, new engine runs or DOS probes. The models use exact Python rational arithmetic; JSON rounds output for readability. This is formula evidence, not Real48 bit-level conformance or an adversarial campaign simulation.

## Reconciled baseline

`maint/merging/colonization.rs` creates production 1, zero savings and normal owner status 2. The retained colony start of 1 is correct at this revision; the spec’s production-0 sentence is stale. Economy uses old production for the high-tax penalty and then credits revenue on updated production. The helpers round stored production and goods to whole points before maintenance writes integer-valued Real48 state. Merely having Real48 fields therefore does not retain economic fractions today.

Classic itself stores production and goods fractionally at PLANETS 0x03 and 0x09. Carrying fractions is closer to that representation; NC still chooses its own growth formula. Classic capture pays zero, half and full across the first three passes. NC skips both early passes. The counter matches; the second-pass yield does not. Claude acknowledged the distinction. No change to conversion is included here.

The manual’s advice to lower taxes on individual colonies is a documentation error: the one rate affects the entire empire. The claim that production always increases by at least one ignores tax damage. These fixes need no new economic control.

## Five controls

- `current`: integer NC growth, integer tax penalty and floored annual revenue.
- `income_carry`: same production path, retaining fractional revenue only. This isolates money rounding and leaves production cliffs untouched.
- `smooth`: fractional growth, penalty and revenue; same NC coefficients and threshold, with no minimum one-point growth or intermediate ceiling.
- `smooth_min1`: smooth arithmetic but retains minimum one-point growth before the remaining-gap clamp. This tests preserving fast tiny-world development.
- `gated_min1`: the one-point floor applies only for tax strictly below 65%, as Claude proposed. At 65% and above, use the smooth rule. This is not an inclusive threshold.

For the smooth control, with tax t in percent, production P, potential K, and no base:

```text
a = (100 - min(t,95)) / 400
b = max(0,t-65) / 500
P_next = clamp(P + a*(K-P) - b*P, 0, K)
income = P_next*t/100
G_next = G_after_build_spending + income
```

The penalty uses old P. Base controls retain the current intended 50% growth bonus through tax 50, tapering linearly to zero at 65, without rounding the taper to an integer percent. They expose rounding interactions; they do not choose decision 3’s base benefit.

For constant tax without a base, `P_n = P_eq + (P_0-P_eq)*(1-a-b)^n`, where `P_eq = K*a/(a+b)`. This proves the equilibrium and lets us separate tax effects from numerical noise. It does not prove an optimal strategy against opponents.

## Colony pace and local military access

Production starts at 1. Every row uses tax 50, no base, no spending or losses. Pass 1 is the first economic pass after colonization. An order milestone means local funds and capacity can admit an order of that cost after that pass; it is not completion, commissioning or delivery.

| Potential / rule | Production after 10 passes | Income accumulated | First cost-5 order | First cost-15 order | First cost-45 order | Pass reaching 90% potential |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 10 / current | 10.00 | 34.00 | 3 | unavailable | unavailable | 7 |
| 10 / income carry | 10.00 | 36.00 | 3 | unavailable | unavailable | 7 |
| 10 / smooth | 7.63 | 26.79 | 4 | unavailable | unavailable | 17 |
| 10 / smooth min 1 | 10.00 | 32.50 | 4 | unavailable | unavailable | 8 |
| 20 / current | 17.00 | 56.00 | 2 | 8 | unavailable | 11 |
| 20 / smooth | 15.00 | 50.99 | 3 | 10 | unavailable | 17 |
| 100 / current | 77.00 | 252.00 | 1 | 2 | 5 | 15 |
| 100 / income carry | 77.00 | 255.00 | 1 | 2 | 5 | 15 |
| 100 / smooth | 73.96 | 244.66 | 1 | 2 | 5 | 18 |

The potential-100 colony loses 2.9% of its ten-pass accumulated income under smooth arithmetic; these three local order dates stay unchanged. The potential-10 colony loses 21.2%, with a cost-5 order delayed one pass. This is not balance-neutral. Some of today’s tiny-world speed comes from the one-point floor rather than tax choices. Smooth arithmetic makes development more proportional to potential and preserves differences between tax rates. Whether the slower frontier is desirable needs explicit attention, not concealment behind “accounting cleanup.”

Without a base, potential 10 cannot admit a cost-15 order under the retained budget rule, even with ample savings. These controls do not change t056 or permit credit for a new over-budget order.

## Tax cliff, sustained taxes and emergency cash

At a mature potential-100 world, current tax 70 settles at production 99 and income 69. Tax 71 settles at production 86 and income 61. Income carry alone keeps both production plateaus; it cannot fix the tax cliff.

The smooth equilibrium is production 88.24/income 61.76 at tax 70, versus 85.80/60.92 at 71. Tax 65 sustains full production and income 65. This makes prolonged overtaxation actually costly. A strategic threshold at 65 remains deliberate and documented; the accidental one-point tax cliff disappears.

The smooth-minimum-1 control keeps a profitable near-full plateau: after 60 passes at tax 70 it produces 99.01 and earns 69.31. That is why I do not recommend retaining the one-point floor simply to protect the fastest tiny-world growth. It also protects mature worlds from the intended tax cost.

Emergency taxes still buy time. Starting at production 100, a single tax-100 pass earns 93 instead of 65 and leaves production 93. Returning to 65 gives cumulative income 651.36 after ten passes versus 650 for constant 65, but 1288.88 after twenty versus 1300. The extra 28 points arrive in the next revenue collection, after that pass’s build spending; they do not retroactively fund its builds. Their value is saving a threatened position, not a permanent free-income trick.

We enumerated every integer constant tax rate for three world starts at horizons 1, 5, 10 and 20. A fresh potential-100 smooth colony’s best constant rates by accumulated income are 52, 55, 59 and 64 respectively. A mature world’s are 100, 100, 88 and 70. These are horizon-specific cash maxima without spending, threats or terminal asset value, not recommendations to always tax at those rates. They show the meaningful short-run versus long-run tradeoff survives smoothing.

## Development risk remains rewarding

One production-100 home plus four production-1, potential-100 colonies, all held for twenty passes, no spending:

| Smooth policy | Accumulated income at pass 5 | Pass 10 | Pass 20 |
| --- | ---: | ---: | ---: |
| Constant 35 | 455.00 | 1156.94 | 2806.26 |
| Constant 50 | 574.89 | 1478.62 | 3709.92 |
| Constant 65 | 638.92 | 1640.08 | 4245.71 |
| 35 for five passes, then 65 | 455.00 | 1673.71 | 4504.05 |

The low-tax opening forgoes 183.92 early points versus constant 65, then leads by 258.34 at twenty passes. It remains a risk that can pay off. These are sums of separate local accounts, not empire-wide spendable funds. The controls contain no enemy attack, colony loss, exploration or transport travel. They cannot establish the safest or strongest opening.

## Spending, display and forecasts

Recommend keeping production and goods fractional internally, including through save/load and compatible export. Whole-point spending draws only from `max(0,floor(G))`; subtract the exact integer expenditure and preserve the remainder. Do not round 4.9 savings up and spend 5. Preserve imported negative goods rather than erasing debt; income pays that balance normally, with no negative spending budget. Queue semantics remain t056’s.

Build capacity is a separate convention. Retain the existing nearest-whole production value for capacity, multiplied by five for an eligible base. For nonnegative P this is `floor(P+0.5)`. This is deliberate and must be stated in Appendix A. Flooring capacity instead would leave an asymptotically developing potential-100 world unable to use its hundredth capacity point forever. Do not solve this by secretly resetting production to potential. The controls use nearest-whole capacity and floored spendable goods.

Forecast recommendation for decision 1: existing planet Revenue and Empire Revenue fields should describe the next economic pass at the selected empire tax, not imply the unchanging current-production product. State the assumption in help: unchanged tax, ownership and qualifying base. Apply actual capture status and the known build/economy ordering. Budget continues to mean spendable now, not expected income. Do not add a projected-income amount to the current build budget.

Keep whole-number widths. Treasury should show whole spendable savings for nonnegative balances, with fractions retained internally; negative imported balances need a truthful signed/debt indication within the existing field or report, not silent zeroing. Revenue is a whole-point estimate of fractional income; the manual must say fractions carry and the eventual whole treasury increment may differ by one. Sum the same per-world displayed forecasts for the empire display so visible rows reconcile; do not pool fractional local funds into an empire wallet. Precise rounding and labels need layout checks before implementation.

Use an existing tax-entry feedback/help line for the new rate’s next-revenue estimate if it fits, rather than add a screen. No years-to-full forecast: the smooth curve approaches exact full production asymptotically, and future orders/taxes make a long-range date misleading. Production versus potential and the next-pass estimate provide enough first-step guidance. Forecasting does not require revealing every internal fraction in fleet or planet tables.

## Recommendation to Claude

Bring accounting and forecasts to the owner first, with smoothing’s gameplay effects disclosed alongside them. Recommend fractional internal state, floored spendable treasury, existing nearest-whole capacity, and next-pass revenue in existing fields. Do not merge that as a supposedly cosmetic patch while leaving the production rule undefined.

For decision 2, my initial preference is the smooth existing-coefficient curve with no one-point growth floor; do not adopt classic’s slower angular curve or add new growth resources. The strongest objection is poorer small-world development. Ask whether one extra pass to a cost-5 local defense and two extra passes to a potential-20 cruiser are acceptable before selecting coefficients. Keep ship prices, ETAC speed, base benefits, capture and bombardment unchanged in this first comparison. No campaign-wide balance conclusion yet.

War cost remains hull losses, lost time, conquest and bombardment, without repair bills or upkeep. Decisions 4 and 5 must price limited raids, won invasions and failed invasions against replacement plus travel, RT downtime and lost production. Peaceful income tables cannot prove war remains costly enough.

## Claude’s gated-floor challenge

Claude correctly distinguishes the useful early-colony support from its unwanted interaction with punitive tax. A minimum growth rule can be a deliberate pacing choice. The new control tests his exact proposal: one-point minimum, capped by the remaining gap, only at tax below 65. The results reject this particular hard gate, not the goal of keeping small colonies useful.

At tax 50, production starts at 1 with no base or spending:

| Potential | Gated production at pass 10 | Gated cumulative income | Current cumulative income | First cost-5 order, gated/current | First cost-15 order, gated/current | First cost-45 order, gated/current |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 10 | 10.00 | 32.50 | 34.00 | 4 / 3 | unavailable | unavailable |
| 20 | 15.54 | 51.42 | 56.00 | 3 / 2 | 9 / 8 | unavailable |
| 100 | 73.96 | 244.66 | 252.00 | 1 / 1 | 2 / 2 | 5 / 5 |

The K10 first-decade income loss shrinks from 21.2% under fully smooth growth to 4.4%. Its first cost-5 order still comes a pass later than today: removing ceilings from the earlier growth increments still matters. K20 keeps a one-pass delay for both listed orders. The gate improves the small-world case but does not restore every current milestone.

At tax 70 and 71, it exactly matches the smooth rule: equilibrium P/income is 88.2353/61.7647 and 85.7988/60.9172. It removes the old sustained 70% near-full plateau.

The one-year 100%, then 65% control is also exactly unchanged from smooth: first income 93, production 93; cumulative income 651.36 at pass 10 and 1288.88 at pass 20. The floor is never enabled in that policy. The one-home/four-K100-colonies policy of 35% for five passes then 65% is unchanged too: cumulative income 455.00, 1673.71 and 4504.05 at passes 5, 10 and 20. The floor does not bind during those first five growth passes, then is switched off at 65.

### The new boundary

| Potential, start P1 | Tax | Production at pass 10 | Cumulative income | First cost-5 order | First cost-15 order |
| --- | ---: | ---: | ---: | ---: | ---: |
| 10 | 64 | 10.00 | 40.96 | 4 | unavailable |
| 10 | 65 | 6.40 | 28.41 | 6 | unavailable |
| 20 | 64 | 13.21 | 53.65 | 3 | 12 |
| 20 | 65 | 12.40 | 52.76 | 3 | 14 |
| 100 | 64 | 61.45 | 248.84 | 1 | 2 |
| 100 | 65 | 60.38 | 247.52 | 1 | 2 |

K10 loses 30.6% of first-decade income by moving from 64 to 65. On the first pass alone, P goes from 1 to 2 at 64, earning 1.28, versus P1.7875 and income1.161875 at 65. At P9/K10 the one-pass growth falls from 1 to 0.0875. This is a genuine cutoff, not output rounding or random sampling. Moving the gate to include 65 would merely move its discontinuity to 65/66; that alternative is not simulated here.

### Recovery tax cycles

The floor also applies to a mature world recovering from tax damage. Starting at P100/K100, the policy 75%,64%,64% follows P98,99,100 and earns 73.50,63.36,64.00. It returns to its initial production and earns 200.86 points over three passes versus 195 at steady 65. The cycle can repeat without accumulating production damage. This is a 3.0% income gain from a repeating tax schedule, not a campaign win-rate estimate.

Other exact repeatable cycles: 70%,64% earns 133.30 over two passes (steady65:130); 80% then three passes at64 earns267.68 (260); 100% then seven passes at64 earns527.56 (520). Each ends at P100. These controls need no reserve, base, conquest or repair subsystem. They show why the single emergency control returning to65 did not reveal the issue.

A known tax policy can legitimately trade time for revenue. My objection is the incentive to repeat a schedule primarily to exploit a growth-rule boundary, while young colonies suffer at the very adjacent normal rate. That adds upkeep-like attention to the only economic control. It is not evidence that every tax policy should earn the same income, nor proof that this policy wins in a mixed empire under attack.

### Revised advice

Keep our agreement on fractional accounting, capacity, forecasts and debt. Do not recommend the hard-gated floor as the growth rule. It fixes the specific70/71 plateau but introduces a large64/65 young-world edge and a recurring mature-world tax cycle. My smooth baseline remains preferable to this gate; Claude’s small-colony concern remains valid. If we test another aid, make it support underdevelopment with a continuous tax response, and include post-tax recovery cycles in the controls. No extra candidate or coefficient is selected here.

## One-time colony head start, P0 = min(K, 3)

Claude proposed a one-time starting-production aid after rejecting the hard-gated floor. This uses the fully smooth growth rule unchanged and starts only a newly colonized world at `min(K,3)` instead of 1. No recurring floor is added. Existing colonies, captured worlds, recaptures and tax-damaged mature worlds must not receive this reset. The existing colonization handler checks for an unowned world before applying its initial production; the aid would belong to that initialization, not maintenance recovery.

At tax50, no base or spending, with the same local order-admission convention:

| Potential / initial production | Production after 10 passes | Accumulated income | First cost-5 order | First cost-15 order | First cost-45 order |
| --- | ---: | ---: | ---: | ---: | ---: |
| 10 / current integer P1 | 10.00 | 34.00 | 3 | unavailable | unavailable |
| 10 / smooth P1 | 7.63 | 26.79 | 4 | unavailable | unavailable |
| 10 / smooth P3 | 8.16 | 31.95 | 3 | unavailable | unavailable |
| 20 / current integer P1 | 17.00 | 56.00 | 2 | 8 | unavailable |
| 20 / smooth P1 | 15.00 | 50.99 | 3 | 10 | unavailable |
| 20 / smooth P3 | 15.53 | 56.15 | 2 | 9 | unavailable |
| 100 / current integer P1 | 77.00 | 252.00 | 1 | 2 | 5 |
| 100 / smooth P1 | 73.96 | 244.66 | 1 | 2 | 5 |
| 100 / smooth P3 | 74.48 | 249.81 | 1 | 2 | 5 |

The head start restores the first cost-5 order for K10 and K20. It does not fully restore the K20 cruiser date: pass9 remains one later than current pass8. K10 first-decade income is 6.0% below current; K20 is 0.3% above; K100 is 0.9% below. Against smooth P1, the head start adds exactly the same 5.15847 income over ten passes to all three worlds, so the proportional benefit is concentrated on small worlds. Large-world order dates do not move in these controls.

This follows directly from the recurrence. At tax50 the starting-production difference after n passes is `2*(7/8)^n`, independent of K in these uncapped controls. At any fixed tax, the corresponding difference is `2*(1-a-b)^n`. It decays to zero. Equilibria at70/71 are unchanged: for K100, P/income88.2353/61.7647 and85.7988/60.9172. No new tax edge or mature-world recovery rule is introduced. This is not a proof that no changing-tax policy can be advantageous; it says the aid does not alter the existing smooth tax response.

For one mature K100 home and four new K100 colonies:

| Smooth policy / colony start | Cumulative income at5 | At10 | At20 |
| --- | ---: | ---: | ---: |
| Constant65 / P1 | 638.92 | 1640.08 | 4245.71 |
| Constant65 / P3 | 658.84 | 1672.60 | 4291.25 |
| 35 for5, then65 / P1 | 455.00 | 1673.71 | 4504.05 |
| 35 for5, then65 / P3 | 463.48 | 1690.40 | 4529.22 |

With the head start, the low-tax policy gives up195.36 points at5 versus constant65 and leads by237.97 at20. Development still rewards surviving an early sacrifice. These are summed local revenues, not pooled funds or campaign win odds. Mature-world one-year emergency controls are unchanged because mature worlds get no head start.

### Joint recommendation under the stated acceptance condition

The aid is partly successful, but does not restore all the listed small-colony order dates. Under Claude’s requested fallback when those dates are not restored, recommend fractional accounting, existing-field forecasts and fully smooth growth alone, retaining colony P1 and explicitly disclosing the small-world cost. Do not call the head start ineffective: it restores cheap-order dates and almost restores decade income without materially favoring big worlds. If restoring first useful builds rather than every old milestone is the intended acceptance criterion, P3 is a defensible separate choice; it is not silently included in this recommendation.

The recommendation sent to Claude is: carry fractional production/goods; floor spendable cash and retain debt; keep nearest-whole production capacity; forecast the next pass in existing Revenue/tax feedback; use fully smooth current-coefficient growth without a recurring floor. At K10 tax50, this means a cost5 order in pass4 rather than3; at K20, a cruiser in10 rather than8. No owner approval or engine change is implied.

## Evidence and validation

[Script](../../../tools/economy-model/accounting_review.py), [results](../../../tools/economy-model/accounting_review_results.json), [tests](../../../tools/economy-model/test_accounting_review.py). Reproduce with `python3 tools/economy-model/accounting_review.py`; run `python3 -m unittest discover -s tools/economy-model -p 'test_*.py'`.

29 tests pass. The head-start check proves the exact decay of its initial-production advantage at50/65/70/71 and verifies the potential cap for a tiny world. Three added checks verify the strict65 boundary, equality to smooth high-tax paths, and the exact repeatable75/64/64 recovery cycle. The new tests compare the integer baseline against the retained NC helper model across all tax rates, every integer production through potentials 10, 100 and 150, and both base states. They also verify the exact smooth recurrence/equilibrium, bounds, carrying money and capacity rounding. This is model cross-checking, not Rust or oracle validation. No historical result files were replaced.
