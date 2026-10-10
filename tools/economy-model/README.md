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
