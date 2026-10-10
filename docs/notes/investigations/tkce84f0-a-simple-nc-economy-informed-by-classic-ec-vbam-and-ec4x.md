---
id: tkce84f0
date: 2026-10-10
kind: investigation
status: investigating
tags: [investigation, economy, balance, simulation]
---
# A simple NC economy informed by classic EC VBAM and EC4X

Keep one empire tax rate as the player's economic control. Borrow EC4X's automatic development, diminishing returns, and connection between economic strength and military reach. Borrow VBAM's lasting costs of war and economic objectives for fleets. Express these through NC's existing production, potential, treasury, ships, and missions. The numbers show that NC already develops colonies far faster than classic. Geography and ETAC travel strongly affect when those colonies matter. Improve forecasts and rounding before changing that pace. Use campaign simulations to tune meaningful opportunities for expansion, defense, and conquest. Do not add investment menus, research budgets, hidden handicaps, or arbitrary eras.

The practical findings, in brief:

- **NC's frontier becomes useful much sooner.** At 50% tax, a production-1, potential-100 colony reaches production 77 after ten economic passes in the NC model, versus about 37.5 in the classic model. Cumulative income is 252 versus about 106.4. Keep expansion rewarding; do not restore classic's stagnation merely for fidelity.
- **Tax already creates a worthwhile risk.** Lower taxes can trade early defenses for later income. Integer rounding also creates obscure thresholds: a mature potential-100 world can settle at income 69 under 70% tax, but only 61 under 71%. Test smoother internal accounting and explain the intended emergency-tax tradeoff. No replacement coefficients are selected.
- **Movement and local capacity shape the campaign.** In the synthetic tax-50 benchmark, colonies collectively match homeworld income in median year 7 under NC growth versus year 11 under classic growth. Slowing ETACs from speed 3 to 2 moves the eighth colony from median year 11 to 15. A base increases build capacity, not income; savings elsewhere cannot instantly fund one front.
- **Avoid artificial ship eras.** Default home savings already cover a battleship. Ship roles should follow combat value, travel, and opportunity cost. Price changes must be tested against both fleet balance and the time to build and deliver replacements.
- **Connect war to the economy through consequences players understand.** Paid repairs are the first candidate to test. Sustained blockade may provide another role for smaller fleets. Defer general upkeep and support penalties until campaign evidence shows a need. Retain one tax control, automatic development, and useful forecasts.
- **These are model results, not an oracle comparison.** The simulations use decoded classic formulas and synthetic maps. They do not yet test rivals, scouting, combat, repair, or actual generated starting positions. Their pacing differences justify further tests; they do not establish an optimal strategy or final balance.

**Scope and evidence**

The owner requested this comparison, then asked for Monte Carlo analysis of the 4X campaign arc, unit prices, movement, and pacing. The owner also specified that NC should remain much simpler than EC4X. This is investigation input to [t051](../plans/q0a0mhm2-classic-follow-ups-from-the-october-2026-oracle-work.md#t051), not promotion of that task or an accepted economy redesign. The combat candidates in [rolr9ti0](rolr9ti0-independent-nc-combat-balance-analysis.md) and [o7w8k8uh](o7w8k8uh-what-the-completed-classic-combat-reference-changes-for-nc.md) remain separate decisions.

The later [combat-sweep review](qjev5wpq-review-of-the-t055-combat-sweep-semantics.md) connects those candidates to transport survival, persistent damage, and repair costs. Its validation limits also apply when using combat results in an economic campaign model.

Sources are the [NC manual](../../manuals/nc_player_manual.typ), [economics spec](../../dev/economics.md), [decoded classic economy](../../dev/ec-economy-reference.md), and [movement spec](../../dev/ec-movement-spec.md). NC arithmetic and starting conditions were checked at `38e703f4`. Later documentation commits `38832018` and `5777b5e2` do not change those calculations. VBAM citations refer to printed pages of *Victory by Any Means*, third edition, v1.0.2, VBM 3001. EC4X citations use its [economy spec](https://github.com/greenm01/ec4x/blob/9f931281a76ed8f3ffca9c5e816f04d5170c973d/docs/specs/03-economy.md), configuration, and code at `9f931281a76ed8f3ffca9c5e816f04d5170c973d`, read locally.

The [tools and methods](../../../tools/economy-model/README.md), [economic results](../../../tools/economy-model/results.json), and [campaign results](../../../tools/economy-model/campaign_results.json) are retained. No cargo, engine changes, or new DOS probes were used.

To answer the classic-oracle question explicitly: this investigation compared Python models of NC and classic arithmetic. It did not run matched scenarios through `ECMAINT.EXE`, and it did not directly validate its Python outputs against retained oracle outputs. The classic reference includes probe-confirmed and disassembly-only rules, marked separately; that source evidence does not automatically validate this model. A later validation should compare selected normal-world, tax-boundary, base, and capture cases against the original executable, then check the intended NC rules through the engine harness. Random classic cases need distribution or controlled-roll comparisons; Python and DOS seeds are not interchangeable.

**What each economy contributes**

| System | Main economic structure | Useful lesson for NC |
| --- | --- | --- |
| Classic EC | One tax rate; automatic development; fixed potential; savings on each world; reusable colony ships; no routine fleet upkeep. | A small set of decisions can create a substantial strategic game. Location and time matter. |
| Current NC | The same broad structure, much faster deterministic development, integer growth and income, and a different conquest delay. | Preserve the simple interface. Check the consequences of the new growth curve against unchanged prices and movement. |
| VBAM | Shared funds, local construction limits, purchased development, maintenance, supply, trade, repair, and unrest. | Saving ships and disrupting an economy can matter without conquering everything. Several balancing constraints accompany its combat rules. |
| EC4X | Automatic population and industrial growth, planet limits, tax effects, investment, research, maintenance, and construction infrastructure. | Economic depth can arise from predictable development and physical capacity. NC can retain those relationships with fewer quantities and orders. |

EC4X's useful distinction is between current revenue, underlying productive strength, and the ability to support force. NC already has enough vocabulary to express much of this. It does not need separate population units, industrial units, transfer units, and research currencies to benefit from the design.

VBAM turns represent roughly a month; classic/NC turns a year; EC4X turns much longer periods. Compare player decision cycles, development times, and cost-to-income ratios. A percentage copied across these games is not automatically equivalent.

**Classic's tax rule explains its slow frontier**

For a normal world, let `P` be production, `K` potential, `t` the tax percentage, `R` uniform on integers 0 through 10, and `b` a base's random tax reduction, otherwise zero:

```text
theta = asin(P / K)
theta' = clamp(theta + (pi/24) × [1 - (t-b+5-R) × 1.42857/100], 0, pi/2)
P' = round(K × sin(theta'), 3)
if t > 70: P' *= 1 - (t-70) × (U+0.5)/100, with U uniform [0,1)
P' = max(1, P')
income = P' × t/100
```

Classic retains fractional income. Its literal upper-angle handling differs slightly from mathematical `asin(1)`; the tool follows the reference's denominator replacement. It omits the foreign-base and record-order bugs.

Without a base, the mean angular step corresponds to roughly 24 years from zero to full at 35% tax, 42 at 50%, and 168 at 65%. These are mean-step illustrations, not expected random first-arrival times. At 70%, positive and negative angular steps nearly cancel. Above 70%, a second contraction compounds the damage. This explains why classic could feel stagnant even while a player followed its 65% warning.

The following uses 5,000 sampled classic trajectories per case and deterministic NC arithmetic. Both begin at `P=1`, `K=100`, with no savings, spending, base, or losses. Year 1 is the first economic pass after colonization.

| Tax | Classic production after 10 years, mean | NC production after 10 years | Classic income accumulated, mean | NC income accumulated |
| --- | ---: | ---: | ---: | ---: |
| 35% | 61.7 | 87 | 124.3 | 208 |
| 50% | 37.5 | 77 | 106.4 | 252 |
| 65% | 10.3 | 63 | 39.9 | 253 |
| 70% | 2.2 | 50 | 12.6 | 214 |

The large difference is the growth rule, not favorable random sampling. At 65%, NC produces over six times as much accumulated income in this example. An ETAC still costs 20 and survives colonization. Expansion therefore has a different return even though familiar ship prices remain. This is not a reason to restore classic's delay. It is a reason to test the new campaign pace.

**One tax control already supports a real strategic gamble**

Take one mature `P=K=100` homeworld and four new `P=1, K=100` colonies. Keep all worlds for 20 years and spend nothing. These are cumulative income totals, not an empire-wide spendable account:

| Tax policy | Income through year 5 | Through year 10 | Through year 20 |
| --- | ---: | ---: | ---: |
| Always 35% | 459 | 1,182 | 2,860 |
| Always 50% | 582 | 1,508 | 3,796 |
| Always 65% | 641 | 1,662 | 4,344 |
| Always 70% | 609 | 1,546 | 4,024 |
| 35% for five years, then 65% | 459 | 1,704 | 4,626 |

The development policy gives up 182 early points against constant 65%, then earns 282 more by year 20. The early sacrifice is equivalent to roughly four battleships at current prices. That creates a meaningful vulnerability. The player earns the later benefit by surviving it.

This is not a universal opening. In the campaign benchmark, colonies arrive throughout the game. The same five-year switch slightly trails constant 65% in total savings at year 20. A calendar rule cannot substitute for judging how many worlds still need development and how soon an enemy can arrive. That is enough economic decision-making for NC.

**Remove arithmetic surprises before adding more economics**

The current [helpers](../../../rust/nc-data/src/economy.rs) and [maintenance caller](../../../rust/nc-engine/src/maint/economics/planets.rs) use:

```text
g = ceil((K-P) × (100-min(t,95)) / 400), clamped to the remaining gap
d = ceil(P × (t-65) / 500) if t > 65, otherwise 0
P' = min(K, max(0, P+g-d))
income = floor(P' × t/100)
```

Growth includes the documented base bonus when eligible. The penalty uses old `P`, not post-growth production.

A mature 100-production world at 70% becomes 99, then stays there: its one-point growth cancels its one-point penalty. It earns 69 each year, against 65 under 65% tax. At 71%, the same starting world eventually settles at 86 and earns 61. At 75%, another rounding plateau settles at 83 and earns 62. History also matters: integer fixed points need not be unique.

Profitable emergency taxation is good. An obscure 70-to-71 breakpoint is a poor way to create it. Likewise, rounding a nominal 50% base bonus upward can double a one-point growth increment. Rounding each world's income downward discards small colonies' fractions every year.

Test fractional internal production and savings, with whole points required for purchases. This needs no new player action. One experiment keeps NC's coefficients while removing intermediate ceiling and floor operations. With tax fraction `tau=t/100` and no base:

```text
g = 0.25 × (K-P) × [1-min(tau,0.95)]
d = 0.20 × P × max(0,tau-0.65)
P' = clamp(P+g-d, 0, K)
income = tau × P', retained fractionally
```

At 70%, its stable production is `100 × 0.075 / (0.075+0.01) = 88.24`, earning 61.76. This is a candidate, not an adoption recommendation for these exact coefficients. Simply removing rounding changes balance. Retune the curve if necessary to preserve attractive temporary overtaxation and colony development. Keep the intended tradeoff; remove dependence on undocumented integer accidents. This particular formula is an NC experiment, not a formula copied from EC4X.

**What to borrow from EC4X under the hood**

EC4X's production structure is:

```text
GCO = PU × resource_efficiency + IU × economic_tech × construction_tech
      × (1 + tax_productivity_modifier)
tax_productivity_modifier = (50-t)/500
income = tax_fraction × GCO, subject to rounding
```

Its code also adds an operational-base industrial bonus. Automatic population development is bounded by planetary capacity. Industrial development follows population. The increasingly expensive conversion from transferred people to productive units makes young settlements efficient destinations. These mechanisms give worlds different development opportunities without requiring every increment to be purchased.

For NC, collapse these relationships into `P`, `K`, the tax curve, and the existing base benefit. Keep young colonies worth claiming and mature worlds valuable. Growth toward a ceiling already gives NC diminishing returns. Do not add a hidden population subsystem merely to reproduce a curve available with those two existing values.

EC4X's soft naval support limit also has a useful principle: maintaining force should relate to productive strength. Its [asset rules](https://github.com/greenm01/ec4x/blob/9f931281a76ed8f3ffca9c5e816f04d5170c973d/docs/specs/02-assets.md) give `C2 = 0.3 × total IU` and excess-command expense of `0.5 × max(0, fleet CC-C2)`. This permits overextension at a cost. It is more flexible than prohibiting large fleets. But importing it into NC would require explaining another capacity, paying expenses from local treasuries, and defining shortfalls. Keep it as a later experiment if actual campaigns need it.

Two source distinctions matter. EC4X's implemented population growth is proportional growth with a minimum increment and a hard capacity cap; stale comments call it logistic. Its economy spec gives passive industry's tax factor as `1-tax_fraction`, but [income.nim](https://github.com/greenm01/ec4x/blob/9f931281a76ed8f3ffca9c5e816f04d5170c973d/src/engine/systems/income/income.nim) uses the low-tax population bonus instead. With PU 100 and no base, 50% tax gives one IU under the written formula and two under the implementation. Resolve that distinction before treating either as a calibrated reference. It does not invalidate the economic design.

A local ceiling does not prevent an empire from gaining a deserved lead by owning more worlds. Do not introduce leader penalties to erase that lead. Give rivals reachable targets and opportunities to interrupt its conversion into military power.

**What to borrow from VBAM without adding menus**

VBAM's basic income is `Population × RAW` (p. 20). Development costs increase with the next population level: `10 × new_population` (p. 67). At RAW 4, advancing population 1 to 2 pays back its 20-point price in five turns; advancing 4 to 5 takes 12.5 turns. The useful principle is declining return on concentrating development. NC can express that through automatic growth toward potential. It need not import purchase orders for population.

VBAM charges 10% of active units' total original build cost, rounded after summing (p. 22). This ties accumulated military power to current income. In a deliberately simple model with constant income `Y`, maintenance fraction `u`, no losses, and all remaining income spent on fleets:

```text
F[next] = (1-u) × F + Y
F[steady] = Y/u, if u > 0
```

With no upkeep, fleet value grows by `Y` forever. At 10%, it approaches `10Y`; after 20 turns from zero it is about `8.78Y`, versus `20Y` without upkeep. This is a major change, even if the calculation runs automatically. VBAM also offers mothballing and forces scrapping after a deficit. Those consequences do not fit NC merely because the interface could hide them.

Prefer paid repairs as the first economic connection to war. VBAM's 25% repair cost (p. 61) makes withdrawal worthwhile without taxing every peaceful turn. Under the existing NC repair candidate, four surviving crippled BBs cost 45 to restore rather than 180 to replace. Paid repair still needs an explicit spending priority and affordability rule. Automatic calculation must not silently spend a world's defense budget before its queued builds. The campaign model here does not yet simulate repair.

VBAM and EC4X also make blockade economically meaningful. That gives smaller fleets an objective short of invasion. NC's current blockade mission provides a watch/attack behavior; an income penalty would be a new rule. If adopted, use orbital control and the duration of disruption, not VBAM's raw ship-count test. With economics before the 52 ticks, a stable-owner illustration is `next_income = ordinary_income × (1-q × blockaded_ticks/52)`. The severity `q`, ownership transitions, and reporting need a decision. A one-tick visit should not erase a whole year's income. This can work through an existing mission rather than a trade-management screen.

**Monte Carlo results for the campaign arc**

The campaign tool runs 16 configurations over 1,000 maps each. Each synthetic map is a 31×31 square with a centered homeworld and 24 randomly placed unowned worlds, unless density is being varied. Potential is uniformly sampled from 10 to 150. This is a sensitivity distribution, not NC's generator. Home production/potential are 100 and starting savings are 50, matching the default joined NC treasury. Two ETACs begin in orbit. They visit nearest unreserved targets and remain reusable.

All target locations are available to the policy, but it does not choose by potential. There are no rivals, combat, interception, repair, upkeep, or base benefits. Except for explicitly purchased extra ETACs, savings remain unspent. Travel uses full-orbit Euclidean distance plus climb/descent, rounded to ticks and order years. It omits classic's rounding anomalies and the end-year colonization shortcut. These results establish economic opportunities under a controlled route policy. They do not predict live-game win rates or exact ETAs.

| Milestone, median year | Classic growth, tax 50 | NC growth, tax 50 | NC growth, tax 65 | Fractional candidate, tax 50 |
| --- | ---: | ---: | ---: | ---: |
| First colony | 2 | 2 | 2 | 2 |
| Four colonies | 6 | 6 | 6 | 6 |
| Eight colonies | 11 | 11 | 11 | 11 |
| Combined colony income reaches home income | 11 | 7 | 8 | 7 |
| First colony can order a CA | 8 | 6 | 6 | 6 |
| First colony can order a cost-45 BB | 15 | 8 | 9 | 8 |

Build milestones require both local funds and local capacity. They are order dates, not launched or delivered ships. All listed milestones occur within 25 years on at least 99.8% of sampled maps. Medians exclude unreached cases; the JSON preserves their counts. In a revealing separate case, classic at 65% gives only one map in 1,000 a colony able to order a BB by year 25. NC should not imitate that stagnation.

At NC tax 50, the first colony's 10th-to-90th percentile interval is years 1–3; four colonies, 5–7; eight colonies, 10–13; a colony eligible to order a BB, 6–10. These are variation across synthetic maps, not confidence intervals or promises to players.

| One change from the NC tax-50 baseline | Median eighth colony | Mean annual income in year 20 |
| --- | ---: | ---: |
| Baseline: 24 worlds, ETAC speed 3 | 11 | 410.6 |
| Sparse: 12 worlds | 15 | 315.0 |
| Dense: 48 worlds | 9 | 506.7 |
| ETAC speed 2 | 15 | 306.4 |
| ETAC speed 4 | 9 | 497.6 |

Travel and density change the timing of opportunity substantially. Slowing ETACs to restrain expansion also adds waiting. Raising their price changes the military spending forgone. Those are different experiences and should be tested separately. The tool also buys two extra ETACs at home under prices 15, 20, and 30. Their peaceful returns cannot establish a dominant opening: it does not simulate the undefended investments or enemy pressure that should make expansion risky.

**The four Xs should overlap**

| Activity | What should create the decision | What to measure |
| --- | --- | --- |
| Explore | Incomplete information, routes, planet quality, enemy intentions. | Time to useful information; avoidable bad expeditions; value of a scout versus spending 15 on a cruiser. |
| Expand | Reusable ETAC travel, escorts, limited early money, exposed new worlds. | Colony arrival distribution; income foregone for colony ships; survival before payback; nearby potential reachable in time. |
| Exploit | Automatic development, tax policy, local build capacity, reinforcement routes. | When colonies fund local defenses and warships; whether lower taxes repay their early vulnerability; when a base earns its deployment cost. |
| Exterminate | Concentrating force, taking economic prizes, retreating, and replacing losses. | Time to rebuild and deliver a useful force; frequency of recoverable defeats; whether a victory creates further decisions. |

Do not lock these into fixed turn ranges. Early contact can cause war during expansion. A mature empire may keep exploring a distant frontier. A player losing a battle may exploit a quiet region while an ally holds the front. Artificial stage unlocks would weaken that freedom.

Starting NC forces already include two CA/ETAC groups and two DDs. At default join, the home treasury is 50, enough to order a 45-cost BB immediately and commission it after maintenance. Cruisers are not a middle-game unlock; battleships are not a late-game unlock. Their place in the campaign must come from suitability and delivery time.

Current combat ships all buy two nominal AS per point: DD `10/5`, CA `30/15`, BB `90/45`. Their durability, execution behavior, speed, and loss granularity distinguish them. Economy changes must be checked against t055's AS/DS/cost experiments. Raising the BB price to delay an imagined era also changes its battlefield cost efficiency. The benchmark's alternate BB prices 30/45/60 move first-colony eligibility to median years 7/8/9 at 50% tax, without proving those prices balanced in combat.

Depth makes small distances matter. A full orbit-to-orbit trip over distance `d` costs about `d+2` movement units:

| Distance between worlds | DD/SC, speed 6 | CA/TT, speed 5 | BB, speed 4 | ETAC, speed 3 | Base, speed 1 |
| --- | ---: | ---: | ---: | ---: | ---: |
| 3 sectors | 1 year | 1 | 2 | 2 | 5 |
| 10 sectors | 2 | 3 | 3 | 4 | 12 |
| 20 sectors | 4 | 5 | 6 | 8 | 22 |

These are ideal full-orbit travel ceilings without avoidance or classic arrival exceptions. The engine's ETA remains authoritative. A DD can influence a nearby fight a year before a BB despite inferior sustained combat. Adding a slow ship can delay an entire force. A base's defense and growth benefits cannot justify placing it everywhere if delivery takes twelve years.

The base's 5× build allowance is capacity, not income. At `P=100`, tax 50, a world still earns 50 annually. A base enables a reserve-funded surge; it does not sustainably fund five times as many ships. Local treasuries also make an empire's total income a poor proxy for how quickly it can reinforce one front. Keeping those constraints gives movement strategic value without a supply-management menu.

After a defeat, measure replacement plus commissioning plus travel. Ten CA cost 150. A mature world earning 50, with no reserves, needs three income collections to fund them, then any remaining build/commissioning delay and the journey. A three-year route can make the return take roughly six years or more. This is why usable retreat, lasting but repairable damage, and forward production matter more than an abstract equal-cost battle alone.

There is one economic pass per year, but several fights can occur during its 52 ticks. Replacement income cannot arrive between those fights. Persistent damage and pursuit therefore change the economic cost of a campaign even if every ship price stays fixed. Test repeated encounters within one year, not just one battle per income collection.

Conquest should reward risk. NC preserves captured industry; Blitz can also retain batteries. Classic's destructive capture economy is diagnostic, not a balance target. Current NC skips growth and revenue for two subsequent economic passes after capture and resumes on the third. Classic uses zero, half, then full. Keep conversion legible and test how that delay interacts with travel and repair before lengthening it. Successful attackers should gain a valuable position, not only an obligation to rebuild rubble.

**Fairness and what stays under the hood**

Keep calibration coefficients, simulation distributions, and campaign telemetry out of the ordinary turn workflow. Expose the chosen tax, expected revenue, production trend, local build budget, prices, and ETA. Put exact formulas in an appendix. A player should be able to say why a plan failed: the colony was vulnerable, reinforcements arrived late, taxes sacrificed development, or the enemy brought more force.

Do not make outcomes depend on hidden help for the loser or penalties for the leader. Fairness means comparable starting opportunities and predictable consequences, not equal results. Evaluate starts by potential reachable within ETAC travel times and by the time opponents can threaten it. Equal planet counts at equal map distances are not enough. This benchmark does not yet test actual generated starts or opponent positions.

Risk should have an intelligible payoff. An extra ETAC trades current defense for future local income. Lower taxes trade current ships for faster development. A raid trades exposure for disruption. Retreat trades ground and time for saved hulls. Random production penalties add little to those decisions. Monte Carlo should sample maps, information, policies, and combat uncertainty; it does not require random taxes or arbitrary economic disasters.

**Corrections and implementation order**

1. **Make the current contract truthful.** The manual says one empire rate, then advises lowering taxes on individual colonies. Its minimum-growth sentence omits the high-tax penalty. The economics spec still calls decoded classic formulas opaque and contains an obsolete post-loop description. Explain base capacity separately from income and show exact capture timing. These are inexpensive improvements.
2. **Fix the build promise before tuning ship access.** [Order validation](../../../rust/nc-data/src/turns/apply.rs) compares the full order price with the current local budget. A world with production 20 and savings 100 cannot start a 45-point BB without a base. The [maintenance queue](../../../rust/nc-engine/src/maint/economics/build.rs) supports partial progress, but that does not make every expensive new order admissible. Choose and document the intended rule. VBAM's explicit partial construction is useful precedent, not authorization to change NC's restriction silently.
3. **Test smooth accounting and automatic development.** Preserve one tax control. Compare income, colony military readiness, and high-tax emergency windows across potential values and campaign lengths. The current fractional candidate broadly preserves the tested campaign arc, while changing several integer cliffs. No exact coefficients are selected here.
4. **Connect the campaign to the chosen combat rules.** Add persistent damage and the selected repair rule. Measure money lost, crippled value saved, repair delay, and replacement delivery. Then test sustained economic blockade through the existing mission. Defer general upkeep and naval support penalties unless campaign evidence demonstrates a need.
5. **Calibrate with real campaigns.** Use paired engine map seeds, mirrored seats, and several policies: rapid expansion, guarded expansion, early aggression, and reserve building. Include imperfect scouting, route risk, per-tick encounters, pursuit, captures, and repeated wars. Compare t055's combat candidates and AS/DS/cost tables. Measure time to first contact, defended colony, useful capital force, colony-majority income, recoverable defeat, and decisive advantage. Preserve failures to reach a milestone instead of averaging them away. Follow with human playtests for waiting, clarity, and excitement.

The retained models include 13 passing arithmetic and timing checks. They test their own stated models, not Rust conformance. The Monte Carlo counts are 25,000 sampled single-world classic trajectories and 16,000 synthetic campaign runs. A full repeat reproduced all retained values in 8.53 seconds on the development host. This workload needs only standard-library Python. It is enough to expose large pacing differences; it cannot establish a final economy without adversarial play and actual engine scenarios. t051 remains an investigation, with implementation decisions deferred.
