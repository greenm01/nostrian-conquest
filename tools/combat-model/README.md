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
  `flat-residual`, `flat-carry`; hull variants; `--set` overrides.
- `fleet_model.py`: the rule-driven battle. One step function returns
  branches with probabilities, so `solve` enumerates a battle exactly and
  `simulate` samples it with the same code.
- `sweep.py`: the sweep: fleet matchups per rule set, the ground
  candidates, the engine comparison, Markdown and JSON output.
- `test_sweep.py`: checks that `current` matches the review's mode C
  exchange for exchange, that exact and sampled runs agree, that each
  candidate's semantics hold on hand-built states, and that runs reproduce.
- `nc_sweep_results.md`, `nc_sweep_results.json`: the sweep's output for
  the default run; `nc_engine_odds.json`: the engine run it is compared
  with.
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
python3 -I -B tools/combat-model/sweep.py --markdown tools/combat-model/nc_sweep_results.md --json tools/combat-model/nc_sweep_results.json
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
20261010), `--exact-limit` (states, default 60,000), `--no-ground`,
`--engine FILE`, `--json`, `--markdown`. The default run takes about a
minute.

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
cargo run --release -p nc-fuzz --example combat_odds -- --trials 20000 --json /tmp/engine.json
cd ..
python3 -I -B tools/combat-model/sweep.py --rules current --units baseline --cases engine --no-ground --engine /tmp/engine.json
```

The example prints the outcome split with a Wilson interval and the
destroyed build points per side; `--case NAME` selects cases, `--seed` the
first campaign seed, `--years` how long a trial waits for a battle. Rounds
fought and crippled survivors are not in the maintenance events, so the
comparison covers outcomes and destroyed build points. An exact model value
agrees when the engine's count passes a two-sided exact binomial test
against it at 5%; a sampled one when the intervals overlap.

Result of the check (20,000 trials per case, campaign seeds 1 to 20,000,
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

Every case agrees. The model's `current` rules are the engine's for a
battle between two combat fleets; the engine's sighting, intercept, and
retreat geometry are outside it.

## Rule sets

A rule set is a dict with these fields. `current` is the engine
(`rust/nc-engine/src/maint/combat/`); the `flat-*` family is the
consultation's candidate, differing only in the remainder rule.

| field | values | meaning |
| --- | --- | --- |
| `effect` | `crt` rows x 5 columns, or `flat` rows | `crt` picks a column from the AS ratio (Disadvantaged under 0.5, Pressed, Even, Advantaged from 1.5, Overwhelming from 3) and shifts it; `flat` reads one row per die and adds shifts to the die. `rows` are multipliers in `unit`ths of AS; hits are rounded up. |
| `mixed_shift` | 0, 1 | shift for two or more of DD, CA, BB present (the engine's test). |
| `base_shift`, `base_condition` | `present`, `undamaged`, `none` | shift for a starbase: any, only a nominal one, or never. |
| `shift_cap` | 2 | bound on the total shift. |
| `execution_classes` | class names | classes firing into the execution pool (finish crippled at 1xDS, destroy nominal at 2xDS); every other armed class suppresses (cripple in line order, destroy crippled once no nominal remain). Hits split by AS share, rounded half up. |
| `remainder` | `discard`, `residual`, `carry` | what hits too few for the next step do: nothing; buy that step with probability hits/cost; or stay on the books against the same side, per pool, and join the next round's hits of that pool. |
| `critical` | `free-kill`, `focus`, `none` | a natural 9: destroy one unit for free, crippled first, cheapest first, screened or not; or let the suppression pool buy the highest-DS affordable line steps first, finishing a crippled unit when there is one; or nothing. |
| `screen` | `absolute`, `conditional` | auxiliaries (SC, TT, ET) are hit only once the line is gone; `conditional` also exposes them, first, to a pursuit exchange. |
| `roe` | `lock3`, `after1` | when a side compares its AS with the enemy's against its ROE threshold: from round 4, declared before the round and checked after it (the engine); or after every round from the first. |
| `withdrawal` | `pressed`, `pursuit`, `none` | what a leaving side takes: the engine's withdrawal exchange (both sides fire once at the Pressed column, no shifts); one ordinary exchange, both sides firing, the pursuer's fire reaching auxiliaries under the conditional screen; or nothing. |
| `round_cap` | 64 | sampled battles stop here as a stalemate. |

Semantics the candidates assume, chosen here and open to change:

- **Carry-forward** is a property of the battle, not of a ship: the
  unspent suppression and execution hits against a side are kept per
  pool and spent first in the next round against that side, under that
  round's target order. They are dropped when the battle ends or that
  side leaves. A side is one aggregate force, so nothing follows a fleet
  that splits off; a rule that persists sub-step damage across battles
  would be a different candidate with per-fleet state.
- **Residual** rolls once per pool per round for the first step in
  target order the leftover cannot afford, at that step's cost (2xDS for
  execution against a nominal ship).
- **Per-fleet ROE** is one check per side after every exchange from the
  first; with one fleet per side in the model, per fleet and per side are
  the same. Both sides failing at once leave together and nobody pursues
  (a stalemate in the tables). A side whose combat AS is zero cannot
  leave and is shot while it has units, as in the engine.
- **Pursuit** is a single exchange; the sighting and intercept tests that
  would gate it in the engine are not modelled, so it always happens. The
  pursued side fires back with what it has.
- **Outcomes.** Left wins when the right side's combat AS is zero and its
  own is not, or when the right side left and it stands; mutual when both
  have no combat AS; stalemate at the round cap, in a state that cannot
  change (two crippled ships that cannot finish each other), or when both
  leave. Exchanges count every volley, withdrawal and pursuit included.
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
  the screen rule shows in transports lost.

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
