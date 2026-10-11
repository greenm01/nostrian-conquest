# NC combat model

Python models of NC combat and the combat odds sweep (t055). They put
numbers on the resolution candidates from the
[October 2026 combat review](../../docs/notes/investigations/rolr9ti0-independent-nc-combat-balance-analysis.md)
and its
[consultation](../../docs/notes/investigations/ebrogywq-nc-combat-design-review-against-its-sources.md#consultation-with-codex-2026-10-10)
before any engine rule changes. They do not run the Rust engine; the
engine-side harness below checks the model's current rules against it.
[combat-balance-workflow.md](../../docs/dev/combat-balance-workflow.md)
says how the pieces fit.

The [classic-reference follow-up](../../docs/notes/investigations/o7w8k8uh-what-the-completed-classic-combat-reference-changes-for-nc.md)
explains `classic_reference.py` and its diagnostic comparisons. The
[t055 sweep review](../../docs/notes/investigations/qjev5wpq-review-of-the-t055-combat-sweep-semantics.md)
records the solver-cap probes, carry and pursuit assumptions, and convoy
findings. Its `review_t055.py` and `nc_t055_review_before_fix.json` preserve
the review's reproduction tool and historical evidence.

## Files

- `rules.py`: the unit table (AS, DS, build cost, fire pool, combat line)
  and the rule sets as plain dicts: `current`, `flat-discard`,
  `flat-residual`, `flat-carry`, `flat-half-ds`, `vbam-full-ds`,
  `vbam-half-ds`, and the decision-1 carry combinations below; hull variants; `--set`
  overrides.
- `fleet_model.py`: the rule-driven battle. One step function returns
  branches with probabilities, so `solve` enumerates a battle exactly,
  round by round up to the round cap, and `simulate` samples it with the
  same code.
- `sweep.py`: the sweep: fleet matchups per rule set, the ground
  candidates, the engine comparison, Markdown and JSON output.
- `test_sweep.py`: checks that `current` matches the review's mode C
  exchange for exchange, that exact and sampled runs agree, that each
  candidate's semantics hold on hand-built states, that `solve` stops at
  the round cap, and that runs reproduce.
- `nc_sweep_results.md`, `nc_sweep_results.json`: the sweep's output for
  the default run with `--engine`; `nc_engine_odds.json`: the engine run
  it is compared with.
- `review_t055.py`: Codex's probes of the solver's round cap and the
  carry and convoy semantics; `nc_t055_review_before_fix.json` is their
  output before the cap fix, kept as evidence, so run the script without
  `--output` to check the fixed solver against its finite-horizon
  traversal.
- The review's own scripts, kept as written: `nc_combat_analysis.py`
  (modes C, S, R, X), `nc_ship_sweep.py`, `ground.py` (exact ground
  candidates), `confidence.py` (intervals), `test_models.py`, and their
  preserved `nc_*.json` results. The sweep reuses `ground.py`,
  `nc_combat_analysis.ground` and `binomial_ground`, and `confidence.py`.

## Running

From the repository root, with Python 3 and the standard library. `-I`
keeps the interpreter isolated (the scripts add their own directory to the
path), `-B` keeps bytecode out of the directory.

```sh
python3 -I -B tools/combat-model/sweep.py --engine tools/combat-model/nc_engine_odds.json --markdown tools/combat-model/nc_sweep_results.md --json tools/combat-model/nc_sweep_results.json
python3 -I -B tools/combat-model/sweep.py --rules current --set remainder=carry --cases ca --no-ground
python3 -I -B tools/combat-model/sweep.py --rules flat-discard --set units.DD.DS=8 --cases dd
python3 -I -B -m unittest discover -s tools/combat-model -p 'test_*.py'
```

With SciPy the sampled intervals are exact Clopper-Pearson instead of
Wilson; `uv` supplies it without installing anything:

```sh
uv run --no-project --with scipy python -I -B tools/combat-model/sweep.py --markdown /tmp/sweep.md
```

Options: `--rules` (comma-separated preset names or JSON rule files, default
every preset), `--set KEY=VALUE` (repeatable; a rule field such as
`remainder=carry`, or a unit field such as `units.CA.DS=35`), `--units`
(`baseline`, `dd-10-8-5`, or `all`: baseline everywhere and the DD variant
on the DD cases), `--cases` (names or tags: `ca`, `dd`, `mixed`, `small`,
`roe`, `convoy`, `engine`), `--trials` (default 10,000), `--seed` (default
20261010), `--exact-limit` (default 100,000), `--no-ground`,
`--engine FILE`, `--json`, `--markdown`.

Limits: `solve` enumerates (state, round) pairs, since the round cap is
part of a state's future, and gives up past `--exact-limit` of them, after
which the case is sampled. The round-keyed solver visits several pairs per
state (about 60,000 for 30 CA vs 10 BB under `flat-discard`, 73,000 under
`flat-residual`, 74,000 for 30 DD 10/8 vs 10 CA under `flat-carry`), so
the default limit is 100,000, which keeps every case exact that was exact
before the cap fix and adds the convoy. The larger `flat-carry` battles
stay sampled: 10 CA vs 10 CA needs about 310,000 pairs and 30 CA vs 10 BB
about 1.6 million. The default run takes about two minutes.

### The engine check

`rust/nc-fuzz/examples/combat_odds.rs` runs the cases tagged `engine`
through real maintenance. Each trial builds the same two-player game, puts
one fleet of each empire at rest in an empty sector, at war, on Hold, with
both seats off autopilot (the autopilot recalls idle fleets), and runs
maintenance years with a fresh campaign seed until the engine reports a
battle there. The seed keys the sighting and combat rolls, so each trial
is new luck; `ScenarioPlay::run_maintenance` passes seed 0 and would repeat
one battle.

```sh
cd rust
cargo-slot nostrian-conquest cargo run --release -p nc-fuzz --example combat_odds -- --trials 20000 --json ../tools/combat-model/nc_engine_odds_new.json
cd ..
python3 -I -B tools/combat-model/sweep.py --rules engine-t060 --units baseline --cases engine --no-ground --engine tools/combat-model/nc_engine_odds_new.json --engine-rules engine-t060
```

The example prints the outcome split with a Wilson interval and the
destroyed build points per side; `--case NAME` selects cases, `--seed` the
first campaign seed, `--years` how long a trial waits for a battle. Rounds
fought and crippled survivors are not in the maintenance events, so the
comparison covers outcomes and destroyed build points. The `agree` column
checks every one of them and names what disagrees: each outcome count
(left win, right win, mutual, stalemate) against an exact model value by
a two-sided exact binomial test at 5%, or against a sampled one by
overlapping 95% intervals; and each side's mean destroyed build points
within 2% of that side's starting build points, a stated tolerance rather
than a test, since the engine file carries no loss dispersion. A case in
the engine file without a model record, a model case tagged `engine`
missing from the file, or a case without a battle is listed with the
reason instead of skipped. Failing to reject a difference is not proof of
equivalence: the check covers the cases and measures it lists, for DD,
CA, and BB fleets, and says nothing about auxiliaries, loaded transports,
starbases, or the round cap.

Historical CRT-era result, retained in `nc_engine_odds.json` (20,000 trials per case, campaign seeds 1 to 20,000,
model exact unless an interval is shown):

| case | model left win | engine left win [95%] | model lost L / R | engine lost L / R |
| --- | --- | --- | --- | --- |
| 10 CA vs 10 CA | 49.9% | 50.2% [49.5, 50.9] | 96.6 / 96.6 | 96.2 / 96.7 |
| 11 CA vs 10 CA | 85.0% | 85.3% [84.8, 85.8] | 42.5 / 134.5 | 41.6 / 134.7 |
| 15 CA vs 10 CA | 100.0% | 100.0% [100.0, 100.0] | 3.0 / 150.0 | 2.8 / 150.0 |
| 30 DD vs 10 CA | 4.6% | 4.4% [4.2, 4.7] | 146.3 / 87.0 | 146.4 / 86.5 |
| 90 DD vs 10 BB | 1.8% [1.6, 2.1] | 1.8% [1.7, 2.0] | 445.6 / 222.3 | 445.4 / 222.1 |
| 30 CA vs 10 BB | 36.9% | 37.0% [36.3, 37.7] | 333.5 / 227.3 | 332.0 / 227.7 |
| 29 CA + 3 DD vs 30 CA | 63.5% [62.5, 64.4] | 64.4% [63.8, 65.1] | 244.0 / 351.7 | 238.6 / 354.6 |
| 10 CA ROE 6 vs 15 CA | 0.0% | 0.0% [0.0, 0.0] | 150.0 / 3.0 | 150.0 / 3.0 |

Every historical case agrees under that comparison. `current` retains those
CRT-era rules; it no longer means the latest engine. Sighting, intercept,
and retreat geometry are outside the battle model.

## Rule sets

A rule set is a dict with these fields. `current` preserves the CRT-era
engine; `engine-t060` models the implemented fleet rules at `28814c9d`.
The `flat-*` family is the
consultation's candidate: `flat-discard`, `flat-residual`, and `flat-carry`
differ only in the remainder rule, and `flat-half-ds` is `flat-discard`
with VBAM's damage step, a crippled ship at half DS as well as half AS.

The controlled effectiveness/durability comparison uses four presets:

| preset | mean effectiveness | finishing DS | remainder |
| --- | ---: | --- | --- |
| `flat-discard` | 0.625 | full | discard |
| `flat-half-ds` | 0.625 | half | discard |
| `vbam-full-ds` | 0.300 | full | discard |
| `vbam-half-ds` | 0.300 | half | discard |

The VBAM-rate pair uses its CER distribution while keeping the other NC
candidate rules, including DD/base execution and budgeted critical focus.
This tests damage rate and durability together; it does not reproduce all
VBAM combat rules. These are experiments, not selected NC rules.

The [decision-1 consultation](../../docs/notes/investigations/qjev5wpq-review-of-the-t055-combat-sweep-semantics.md#t058)
adds `flat-carry-half-ds`, `mid-carry`, `mid-carry-half-ds`, and
`vbam-carry-half-ds`. The intermediate table scales every EotS row by 0.72:
d10 0–2 gives 0.18, 3–5 gives 0.36, and 6–9 gives 0.72, for mean 0.45.
Its relative spread is unchanged. The VBAM-rate carry/half-DS control
checks whether earlier low-rate results depended on discarding remainders.
All four keep the other flat candidate rules and the existing hull tables.

```sh
python3 -I -B tools/combat-model/sweep.py --rules flat-discard,flat-half-ds,vbam-full-ds,vbam-half-ds --units baseline --cases ca,convoy --no-ground
```

| field | values | meaning |
| --- | --- | --- |
| `effect` | `crt` rows x 5 columns, or `flat` rows | `crt` picks a column from the AS ratio (Disadvantaged under 0.5, Pressed, Even, Advantaged from 1.5, Overwhelming from 3) and shifts it; `flat` reads one row per die and adds shifts to the die. `rows` are multipliers in `unit`ths of AS; hits are rounded up. |
| `mixed_shift` | 0, 1 | shift for two or more of DD, CA, BB present (the engine's test). |
| `base_shift`, `base_condition` | `present`, `undamaged`, `none` | shift for a starbase: any, only a nominal one, or never. |
| `shift_cap` | 2 | bound on the total shift. |
| `execution_classes` | class names | classes firing into the execution pool (finish crippled at the finishing cost, destroy nominal at DS plus it); every other armed class suppresses (cripple in line order, destroy crippled once no nominal remain). Hits split by AS share, rounded half up. |
| `crippled_ds` | `full`, `half` | the finishing cost of a crippled unit: its DS again (2xDS to destroy a nominal one, the engine), or half its DS rounded down like crippled AS (1.5xDS to destroy; VBAM). |
| `remainder` | `discard`, `residual`, `carry` | what hits too few for the next step do: nothing; buy that step with probability hits/cost; or stay on the books as a temporary budget against the same side, per pool, for the rest of this battle, and join the next round's hits of that pool. |
| `critical` | `free-kill`, `focus`, `none` | a natural 9: destroy one unit for free, crippled first, cheapest first, screened or not; or let the suppression pool buy the highest-DS affordable line steps first, finishing a crippled unit when there is one; or nothing. |
| `screen` | `absolute`, `conditional` | auxiliaries (SC, TT, ET) are hit only once the line is gone; `conditional` also exposes them, first, to a pursuit exchange (`withdrawal=pursuit`; it changes nothing else). |
| `roe` | `lock3`, `after1` | when a side compares its AS with the enemy's against its ROE threshold: from round 4, declared before the round and checked after it (the engine); or after every round from the first. |
| `withdrawal` | `pressed`, `pursuit`, `none` | what a leaving side takes: the engine's withdrawal exchange (both sides fire once without shifts, at the Pressed column for CRT rules or the flat table for flat rules); one ordinary exchange that always happens, both sides firing, the pursuer's fire reaching auxiliaries only under `screen=conditional`; or nothing. |
| `round_cap` | 64 | maximum ordinary battle iterations, exact and sampled alike; a withdrawal/pursuit exchange may follow the last ordinary round. An undecided fight then ends as a stalemate. Total exchanges can reach cap + 1. The engine's `COMBAT_GUARDRAIL_MAX_ROUNDS` uses the same ordinary-round limit. |

Semantics the candidates assume, chosen here and open to change:

- **Carry-forward** is a temporary per-side, per-pool budget within one
  battle, not persistent damage to a class or a step: the unspent
  suppression and execution hits against a side are kept per pool and
  spent first in the next round against that side, under that round's
  target order. They are dropped when the battle ends or that side
  leaves, and an undamaged ship with a budget against it is still at full
  AS. A side is one aggregate force, so nothing follows a fleet that
  splits off; a rule that persists sub-step damage across battles would
  be a different candidate with per-fleet state.
- **Residual** rolls once per pool per round for the first step in
  target order the leftover cannot afford, at that step's cost (2xDS for
  execution against a nominal ship).
- **Per-fleet ROE** is one check per side after every exchange from the
  first; with one fleet per side in the model, per fleet and per side are
  the same. Candidate rules let both sides failing at once leave together
  without pursuit (a stalemate in the tables). `engine-t060` instead retains
  one unmodified exchange when both withdraw at the pre-round-4 check. A side whose combat AS is zero cannot
  leave and is shot while it has units: that is the engine's behaviour,
  which the model reproduces and the engine check tests, not a candidate
  rule.
- **Pursuit** (`withdrawal=pursuit`) is a single exchange that always
  succeeds: the sighting and intercept tests that would gate it in the
  engine are not modelled. The pursued side fires back with what it has.
  Whether that exchange reaches the pursued side's auxiliaries first is
  the separate `screen` switch, so each can be turned off alone:
  `--set withdrawal=none` keeps the conditional screen without a pursuit
  (where it then has no effect), `--set screen=absolute` keeps the pursuit
  firing in the ordinary target order. The convoy case shows the two
  apart (5 CA + 10 TT at ROE 6 vs 10 CA, `flat-carry`, exact; the
  review's 10,000-seed probe in `review_t055.py` agrees):

  | pursuit | screen in pursuit | TT lost | CA lost | at least 5 TT survive | build points lost |
  | --- | --- | --- | --- | --- | --- |
  | none | either | 4.00 | 2.00 | 60.0% | 50.0 |
  | certain | absolute | 4.32 | 3.81 | 58.2% | 78.8 |
  | certain | conditional, auxiliaries first | 6.01 | 2.41 | 43.8% | 66.2 |

  The four lost transports without a pursuit are the 40% of battles in
  which four full-effectiveness rolls out of ten give the ten CA the 300
  hits that destroy all five escorts outright, after which the unarmed
  transports cannot leave (the zero-AS rule above).
- **Transport survival.** Surviving transports are the TT a side still has
  when the battle ends, including crippled and retreating ships. The tables
  give the mean and the chance that at least half survive. JSON fields
  `surviving_left/right` hold the means; `viable_left/right` hold those
  half-survival probabilities (1 for a side with no starting transports).
  This is a proxy for remaining transport capacity, not successful delivery
  or landing. Cargo, destination, and later movement are not modelled.
- **Outcomes.** Left wins when the right side's combat AS is zero and its
  own is not, or when the right side left and it stands; mutual when both
  have no combat AS; stalemate at the round cap (a state that cannot
  change, two crippled ships that cannot finish each other, sits there
  until the cap and counts those exchanges) or when both leave. Exchanges
  count every volley, withdrawal and pursuit included.
  Losses are build points destroyed; crippled is the build points of the
  survivors a repair rule would charge for.
- Not modelled: surprise, joined sides, loaded transports, bases as
  non-retreating partners, movement, sighting, interception, repair.

## Cases

`sweep.py` runs these at ROE 10 unless named otherwise, with the baseline
hull table and, for the DD cases, DD 10/8 at cost 5:

- cruiser odds: 10, 11, 15, 20, 30 CA vs 10 CA (`ca`);
- equal cost: 30 DD vs 10 CA, 90 DD vs 10 BB, 30 CA vs 10 BB, 10 DD vs
  1 SB (`dd`, `ca`);
- mixed vs mono: 29 CA + 3 DD vs 30 CA (`mixed`);
- small exact cases: 3 DD vs 1 CA, 3 CA vs 1 BB, 1 DD vs 1 BB (`small`);
- ROE and pursuit: 10 CA at ROE 6 vs 15 CA, 10 CA vs 10 CA both at ROE 6
  (`roe`), and a convoy, 5 CA + 10 TT at ROE 6 vs 10 CA (`convoy`), where
  the pursuit and screen switches show in transports surviving.

Add a case by appending to `CASES` in `sweep.py`.

## Ground candidates

The ground table enumerates every candidate exactly, from the review's
scripts, after landing and any softening fire:

- the current CRT ground rule: army counts on the ratio CRT, the defender
  column shifted +1, a critical kills one more army, mutual destruction
  leaves a remnant to the larger force entering the exchange, ties to the
  defender (`nc_combat_analysis.ground`);
- the same with every mutual destruction to the defender;
- simultaneous per-army fire: each attacker hits with probability 0.30,
  each defender 0.35, a hit kills one army, no critical, mutual destruction
  to the defender (`nc_combat_analysis.binomial_ground`);
- defender-first volleys: armies AS 10, DS 10; one d10 effectiveness roll
  per side from 10, 20, 20, 30, 30, 30, 30, 40, 40, 50 percent; the
  defender's damage lands first, surviving attackers reply; damage short
  of an army carries against that side's next army through the battle; no
  critical, no column, no remnant (`ground.py`).

## The review's scripts

`nc_combat_analysis.py` holds the review's rule modes: C, the current
arithmetic; S, a literal synthesis experiment (VBAM effectiveness, all
suppression, free highest-DS critical kill); R, the flat EotS table with
all suppression, budgeted critical focus, and the residual roll; X, R with
DD and base execution kept (P in the investigation, the sweep's
`flat-residual` up to the ROE rule). Its 56 cases use 12,000 trials each
and seed 20261010; `nc_ship_sweep.py` runs its 22 hull-variant cases at
6,000 trials; `ground.py` and `confidence.py` write the preserved
`nc_ground_comparison.json` and `nc_confidence_intervals.json`.
`test_models.py` checks their claims.

```sh
python3 -B tools/combat-model/nc_combat_analysis.py
python3 -B tools/combat-model/nc_ship_sweep.py
python3 -B tools/combat-model/ground.py --output tools/combat-model/nc_ground_comparison.json
python3 -B tools/combat-model/classic_reference.py --output tools/combat-model/classic_reference_results.json
python3 -B tools/combat-model/confidence.py tools/combat-model/nc_combat_results.json --output tools/combat-model/nc_confidence_intervals.json
```

They sample with Python's RNG, not the engine's contextual RNG, and stop a
battle when one combat line is gone, where the engine and the sweep keep
firing at the screened auxiliaries. Their unit tables are module globals,
so the hull sweep runs serially. Use even AS values in them: odd-value
cripple rounding has not been checked against Rust.

## Withdrawal and recovery consultation (t062)

`withdrawal_review.py` and `nc_withdrawal_review.json` isolate withdrawal
controls on the specified decision-1 baseline with current modifiers.
`cap_review.py` and `nc_cap_review.json` count actual guardrail exits.
Run each script with `python3 -B`; results write beside the script.
The [investigation](../../docs/notes/investigations/7y0ypr6b-combat-withdrawal-and-persistent-damage-after-decision-1.md)
records assumptions, rejected paid repairs, and the approved RT direction.
These tools do not yet simulate class-specific RT schedules or chase movement.

`recovery.py`, `test_recovery.py`, and `nc_recovery_timing.json` check the
[RT timing contract](../../docs/notes/decisions/p01cg7ll-rt-controls-automatic-ship-recovery.md).
Run `python3 -B tools/combat-model/recovery.py` to regenerate the timing
examples. The model accepts starbase eligibility from the caller; it does
not simulate movement, select RT values, or change the engine. It verifies
that fresh damage cannot heal in its damage year and fleet reshuffling
cannot accelerate recovery.

### t060 engine comparison

`engine-t060` keeps current mixed/base modifiers (+1 die each, capped at +2),
full finishing DS, free natural-9 critical, ordinary line-first screening,
and the three-round ROE lock. It changes resolution to flat EotS effectiveness
and carry. The critical order is fixed as DD, SC, TT, ET, CA, BB, SB, crippled
first, ignoring the screen. `mutual_withdrawal_exchange=true` retains one
exchange when both sides declare withdrawal; older presets keep their
historical semantics. These two optional fields appear in the saved rule dict.

`nc_engine_odds_t060.json` is fresh engine output supplied by Claude at
`28814c9d`, after the typed-state seed change. The CRT-era engine file and
all earlier results stay unchanged. Reproduce the new comparison:

```sh
python3 -I -B tools/combat-model/sweep.py --rules engine-t060 --units baseline --cases engine --trials 20000 --exact-limit 100000 --no-ground --engine tools/combat-model/nc_engine_odds_t060.json --engine-rules engine-t060 --json tools/combat-model/nc_t060_comparison.json --markdown tools/combat-model/nc_t060_comparison.md
```

`--engine-rules` chooses the model records used for comparison; its default
remains `current` so historical commands retain their meaning. See the
[results](nc_t060_comparison.md) and the [t062 investigation](../../docs/notes/investigations/7y0ypr6b-combat-withdrawal-and-persistent-damage-after-decision-1.md).

## Ground and RT calibration (t068)

The [t068 investigation](../../docs/notes/investigations/4lasivnr-ground-combat-and-recovery-turns-campaign-calibration.md)
connects these experiments to the ground decision and proposed initial RT values.

```sh
uv run --no-project --with scipy python -B tools/combat-model/ground_calibration.py
python3 -B tools/combat-model/invasion_price.py
python3 -B tools/combat-model/rt_campaign.py --trials 5000
uv run --no-project --with scipy python -B -m unittest discover -s tools/combat-model -p 'test_*.py'
```

`ground_calibration.py` writes `nc_ground_calibration.json` and `.md` beside
it. Shared rolls enumerate every reachable state, including fractional
army damage; the independent-fire control uses SciPy sparse transitions for
exactly 64 rounds. The shared candidates prove their longest reachable
path is within that cap for the listed cases. Probabilities use floating
point. This finite-matrix control requires NumPy and SciPy; the `uv --with scipy`
command supplies both. Its unit test skips when either is absent, so the
standard-library test command still works. Run the documented uv test command
to include it. No sampling or engine-parity claim for these new ground candidates.

`invasion_price.py` writes `nc_invasion_price.json`. It separates committed
build cost from expected casualties at a 90% single-attempt capture threshold.
Invade prices assume the battery line was already cleared and the named
escorts remain nominal. Blitz includes its cover and landing fire on the
existing planetary CRT. All cases assume the orbital gate has been won.
This is not a full campaign or a price for first securing orbit.

`rt_campaign.py` writes `nc_rt_campaign.json`. It reuses fleet allocation
and the approved cohort timing. The force meets four fresh enemies on a
fixed schedule, without replacements. Each schedule gets the same seed
for each trial and encounter. Gap 0 means four encounters in one year;
gap 2 means years 0, 2, 4, 6. All fights occur before end-year recovery.
Assisted controls qualify only full quiet passes at a separate friendly
base, never the battle pass; travel feasibility is not simulated. Starbases
never assist themselves. None/certain pursuit are immediate-catch controls,
not chase simulations. First-return damage is conditioned on being alive;
full-force destruction and destroyed build points are also reported.
Uniform 1, uniform 2, class 1/2/3 with SB 2 or 3, and a DD-only RT 1 control
keep baseline hull AS/DS and prices fixed. These are stress campaigns, not
measured frequencies of combat on generated maps.

## Assault collateral (t072)

`python3 -B tools/combat-model/assault_collateral.py` writes
`nc_assault_collateral.json`. It exactly enumerates three-exchange bombardment
with current dock-first, same-exchange battery-first overflow, and delayed
battery shielding. A separately labelled flat-support control measures the
size of changing the planetary CRT too. It retains current economic damage
and battery execution, excludes the preceding orbital gate, and does not add
ROE checks inside planetary fire. See the [investigation](../../docs/notes/investigations/twexjjwm-assault-collateral-and-planetary-support-after-the-ground-decision.md)
for scope, contracts, rates and limits. `test_assault_collateral.py` checks
round-start protection and a closed-form undefended bombardment.

## Destroyer durability (t075)

`python3 -B tools/combat-model/hull_review.py` writes `nc_hull_review.json`.
The default is 20,000 trials per battle and 5,000 per repeated-contact cell;
`--trials`, `--campaign-trials`, and `--seed` override these. It changes DD DS
only, keeps all prices and other hull values fixed, and applies decisions 1–3.
It compares equal-cost lines, 75-point convoy escorts, starbases with and
without a guard, the mixed die modifier, and four contacts with RT 2.

Each side is one aggregated fleet. ROE-10 warship rows match decision 1;
ROE-6 contacts and unarmed escape exercise decision 2. No-catch and certain
immediate-catch controls do not simulate speed, contact or episode geometry.
RT contacts use fresh enemies, no replacements and no base assistance. Gap 0
is an imposed multiple-contact stress case, not repeated permitted pursuit
against one fugitive; it assumes separate contacts can legally occur. Every
new encounter resets carry. Transport survival is not arrival or landing.
Wilson 95% intervals accompany battle wins and campaign wipe rates.

`test_hull_review.py` checks ROE-10 equivalence, equal escort budgets, cohort
accounting with altered DS, RT timing and the transport measure. See the
[investigation](../../docs/notes/investigations/wnwxz2et-destroyer-durability-under-withdrawal-and-recovery.md)
for the recommendation and its limits. These are design controls, not an
engine validation of the pending retreat and recovery implementations.

### Surprise memory controls (t073)

`python3 tools/combat-model/surprise_memory_review.py` regenerates
`nc_surprise_memory_review.json`. It calculates exact probabilities of an
unsighted force remaining across the annual boundary, comparing surprise
eligibility under a co-presence snapshot with actual sighting memory.
These are conditional detection controls, not campaign ambush frequencies.
Run `python3 -m unittest discover -s tools/combat-model -p test_surprise_memory_review.py`.
The [decision-6 investigation](../../docs/notes/investigations/m1h272uh-joined-participation-surprise-and-allocation-fairness.md)
records the assumptions, state cost and pending participation work.

### Joined participation controls (t073)

`python3 tools/combat-model/participation_review.py --trials 20000` writes
`nc_participation_review.json`. This uses DD 10/8/5 and the approved flat/carry
fleet resolution with ROE after exchange one, no catch and unchanged criticals.
It compares two imposed sequential contacts with one joined battle, and
measures the first-exchange surprise bonus. Wilson intervals accompany wins.
No movement, legal contact frequency or engine outcome-flip rate is simulated.
The deterministic participation tests cover assembled willingness, irrevocable
simultaneous withdrawals and proportional allocation within an eligible
class/damage/recovery pool, including loaded-first transport casualties.
Remainder ties use uniform draws from the battle’s seeded roll stream, with
a fixed battle roster. Splitting or merging can shift a rounding loss by one
hull per allocation; this is not a bound on its later combat consequences. See the
[decision-6 note](../../docs/notes/investigations/m1h272uh-joined-participation-surprise-and-allocation-fairness.md)
for pending contract details and the separately approved surprise ADR.

## Implemented t065 pursuit preset

`engine-t065` uses after-first-exchange ROE, automatic unarmed escape and
speed-gated pursuit, with `dd-10-8-5` as its default unit variant. It retains
t060 effectiveness/carry/criticals and ordinary screening. Faster pursuers
catch certainly; equal/slower speeds integrate the engine's two-uniform
intercept roll to `2 * pursuer_speed / (pi * fleeing_speed)`. Speeds use the
slowest surviving ship class. A base alone cannot chase; a side containing
a base cannot withdraw in this implementation. The preset models one fresh
contact and episode, not persisted pursuit allowances or a moving chase.
Direct `Model` callers should use `load_units(rules['default_units'])`.

```sh
python3 tools/combat-model/sweep.py --rules engine-t065 --cases engine --trials 20000 --exact-limit 0 --no-ground --engine tools/combat-model/nc_engine_odds_t065.json --engine-rules engine-t065 --json tools/combat-model/nc_t065_comparison.json --markdown tools/combat-model/nc_t065_comparison.md
```

The corrected input has20,000 trials at main7c31f736. Files named
`*_before_harness_fix` retain the earlier4,000-trial input and comparison:
its harness omitted caught withdrawal from classification. The corrected
comparison passes8/9 cases; rare mutual destruction in the mixed case still
fails. `nc_t065_mixed_probe` retains the independent100,000-trial follow-up.
See [t091's review](../../docs/notes/investigations/l5usd3l3-engine-t065-pursuit-and-odds-comparison.md)
for numbers, hashes, reproduction, casualty-tolerance limits and the harness
issue with matching disposition events to a particular battle. Older CRT
and t060 files are unchanged.
