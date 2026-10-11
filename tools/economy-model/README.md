# Economy and campaign pacing models

Reproduction tools for the [economy investigation](../../docs/notes/investigations/tkce84f0-a-simple-nc-economy-informed-by-classic-ec-vbam-and-ec4x.md).
These are arithmetic experiments, not accepted NC rules, engine tests, or
DOS conformance evidence. They require only Python's standard library.

```sh
python3 -B tools/economy-model/analyze.py --output tools/economy-model/results.json
python3 -B tools/economy-model/campaign.py --output tools/economy-model/campaign_results.json
python3 -B -m unittest discover -s tools/economy-model -p 'test_*.py'
```

Omit `--output` to print JSON. `--trials` changes the sampling count.
The retained results use seed 20261010, 5,000 classic trajectories per
single-world case, and 1,000 synthetic maps per campaign case.

`analyze.py` compares the decoded classic economy with the NC helpers and
their maintenance caller at `38e703f4`. Both start a colony at production 1,
as NC's colonization handler does. It also tests a fractional candidate
using NC's coefficients without intermediate integer growth/penalty rounding.
That candidate is not EC4X's formula or an adopted change. The source
arithmetic for EC4X uses `9f931281a76ed8f3ffca9c5e816f04d5170c973d`.
Classic uses Python floats and RNG, rounds production to .001, and omits
the foreign-base and record-order bugs. Normal worlds only: no capture,
scorch, construction spending, or treasury saturation in the single-world
cases. Results include approximate 95% confidence intervals for classic
sample means; these are not intervals for individual outcomes.

`campaign.py` measures economic opportunity on synthetic geography. It runs
16 configurations, including classic/current/fractional growth, tax policies,
planet density, two versus four ETACs, ETAC prices, ETAC speeds, and alternate
BB prices for local build eligibility. A BB-price milestone does not purchase
a BB or change other purchases. A four-ETAC policy buys two additional ships
at home; their costs are actually deducted before income. They become
available for orders the following year. The two initial ETACs are already
owned, as in the starter fleets.

Every case uses the same seeded maps: a centered home on a 31 by 31 grid,
with distinct random unowned sectors and potential uniformly drawn from
10 through 150. This is a sensitivity distribution, not the NC map generator.
Home production and potential are 100; starting savings are 50, as NC's
default joined treasury specifies. The classic-growth cases use the same
opening resources to isolate the effect of growth.
Rule, tax, speed, and price comparisons at the same density use identical
maps. Density comparisons share seeds but use distinct maps. Neither
symmetric starts nor map fairness is being tested yet.

ETACs choose the nearest unreserved target. All target locations are known;
quality does not guide target choice. Both ships can depart simultaneously.
They wait for the next player's order after arriving. Arrival creates a
production-1 colony with zero savings; economics starts the following year.
Travel uses Euclidean center distance plus two depth units, rounded up to
52-tick years. It omits Pascal rounding, the end-year half-depth colonization
shortcut, encounter detours, and sector-entry arrival for deep-space orders.
It is an ideal full-orbit travel benchmark, not an ETA replacement.

There are no rivals, interception, battles, ship losses, repair, upkeep,
base bonuses, or off-home ETAC builds. Starting escorts are not simulated.
Military build milestones mean a colony can admit that order from its own
unspent treasury and capacity. They are not ships built, commissioned, or
delivered. Apart from optional ETAC purchases, savings remain unspent; summed
local reserves are not an empire treasury. Do not infer win odds or optimal
strategy from this peaceful benchmark.

Milestones record the fraction reached within 25 years. Their means and
quantiles are conditional on reaching them; never omit the reach fraction
when many maps fail to reach a milestone. Quantiles describe map variation,
not statistical confidence. All configurations reuse maps. Classic growth
has a separate seeded RNG stream. Results are reproducible, but Python RNG
is not the engine's combat or maintenance RNG.

Thirteen arithmetic checks cover the tax plateau, old-production penalty,
fractional income, source colony start, depth cost, commissioning payment,
post-colonization income timing, local capacity, and censored milestones.
Next validate selected cases in the existing KDL harness, then combine real
map seeds and movement with chosen t055 combat rules. Do not add random
economic events merely to give Monte Carlo something to sample.

The t062 `repair.py`, `repair_pacing.py`, and `repair_pacing_results.json` preserve
a **rejected paid-repair alternative**. They are not selected NC mechanics.
The owner instead approved [automatic RT recovery](../../docs/notes/decisions/p01cg7ll-rt-controls-automatic-ship-recovery.md).
The optional repair arguments in `campaign.py` support that historical comparison;
the default peaceful campaign is unchanged.

## Accounting and development controls (t083)

`python3 tools/economy-model/accounting_review.py` writes
`accounting_review_results.json`. Five deterministic candidates separate
income carry from smooth production/penalty arithmetic and the minimum-growth
rule, including a one-point floor strictly below65% tax. Boundary and
repeatable emergency/recovery-cycle controls expose the cost of that gate.
Calculations use exact fractions, rounded only in JSON presentation.
All integer tax rates are enumerated at selected horizons; no Monte Carlo
or engine/oracle validation is claimed. Local order dates are not deliveries.
Candidate spending floors goods and retains nearest-whole production capacity.
Run the existing unittest discovery command, which includes
`test_accounting_review.py`. See the [joint accounting/development review](../../docs/notes/investigations/1e8xu4w6-fractional-economy-accounting-and-tax-development-controls.md)
for the formulas, small-world pacing cost, forecast scope and open choices.

The same t083 results also compare smooth colonies starting at1 and
`min(potential,3)`. This changes initial conditions only. The tables preserve
small-world order dates, decade income, low-tax investment controls and the
exact decay of the initial advantage; it does not restore every current
order date and is not an approved rule.

## Starbase economics (t085)

`python3 tools/economy-model/starbase_review.py` writes
`starbase_review_results.json`. Exact controls separate growth payback,
imposed arrival delay, locally funded build/commission timing and reserve
spending capacity. Exact limiting-cycle controls compare75/64/64 and70/64
with steady65, at starting production50 and100, with and without a base.
They use approved smooth growth/P3 and base cost50, with
non-stacking bonuses; there is no engine movement or battle simulation.
`test_starbase_review.py` is included in unittest discovery. See the
[starbase review](../../docs/notes/investigations/w3vj4wti-starbase-growth-capacity-and-economic-concentration.md)
for eligibility, mathematical return limits and the draft recommendation.

## Conquest and exposed-industry controls (t086)

`python3 tools/economy-model/conquest_review.py` writes
`conquest_review_results.json`. Exact controls compare0/0/full with0/half/full
conversion on the approved smooth economy, inherited local savings and one
partly paid order, and economic damage from hits already reaching industry.
They also isolate destroying potential under Scorch. They do not simulate
classic capture destruction, raid success or complete campaigns.
`test_conquest_review.py` runs under unittest discovery. See the
[conquest review](../../docs/notes/investigations/et03gvm9-conquest-income-inherited-assets-and-bombardment-economics.md)
for the proposed decision4 and remaining decision5 questions.

## Bombardment and recoverable Scorch (t087)

`python3 tools/economy-model/raid_review.py --trials 10000` writes
`raid_review_results.json`: orbital gates followed by bombardment, with
retained damage, DD8 and the approved battery shield. Three goods rates use
matched seeds; repeated stationary raids are separate controls. This is
conditional on arriving at contact, not an engine run or map campaign.
`python3 tools/economy-model/scorch_review.py` writes
`scorch_review_results.json`: enumerated classic-style annual production
decay, proposed NC debt/queue adaptations, and approved smooth recovery.
Both new test files run under unittest discovery. See the
[joint item5 review](../../docs/notes/investigations/wrk1xd3p-bombardment-raid-costs-and-recoverable-scorch-earth.md)
for the proposed rates, full Scorch contract and limits.
