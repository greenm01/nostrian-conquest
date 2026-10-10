# NC combat model

Python arithmetic models used by the [October 2026 combat review](../../docs/notes/investigations/rolr9ti0-independent-nc-combat-balance-analysis.md).
They explore rules and reproduce its tables. They do not execute the Rust
engine and are not an engine-conformance suite or accepted gameplay rules.
The current-rule model was read from commit
`b8502a0af07af6464a5106d48edd8cf008f84b87`.

- `nc_combat_analysis.py`: current CRT (C), a literal synthesis candidate
  (S), flat all-suppression (R), and flat with DD/base execution (X, called
  P in the note). Writes `nc_combat_results.json` beside itself. Its 56
  cases use 12,000 trials each and seed 20261010.
- `nc_ship_sweep.py`: AS/DS/cost sweeps across DD, CA, BB, and bases. Prints
  JSON lines. Its 22 cases use 6,000 trials each and seed 20261010.
  `nc_ship_sweep.json` preserves the review's result rows in a JSON wrapper;
  the original `.log` is ignored by Git.
- `ground.py`: exact enumeration of current ground rules and two
  candidates, simultaneous army fire and defender-first volleys. Results
  are preserved in `nc_ground_comparison.json`.
- `confidence.py`: SciPy exact binomial confidence intervals, saved in
  `nc_confidence_intervals.json` for the fleet results.
- `test_models.py`: small arithmetic checks. These are not Rust tests.

Run from the repository root with Python 3. No Rust build is needed.
Models and sweeps use the standard library. Only `confidence.py` needs the
NumPy/SciPy dependencies in [requirements.txt](requirements.txt). Use an
environment with those installed when running that script. Use `-B` to
avoid generating bytecode in this analysis directory.

```sh
python3 -B tools/combat-model/nc_combat_analysis.py
python3 -B tools/combat-model/nc_ship_sweep.py
python3 -B tools/combat-model/ground.py --output tools/combat-model/nc_ground_comparison.json
python3 -B tools/combat-model/confidence.py tools/combat-model/nc_combat_results.json --output tools/combat-model/nc_confidence_intervals.json
python3 -B -m unittest discover -s tools/combat-model -p 'test_*.py'
```

Omit `--output` on the ground/interval tools to print JSON without writing
a file. Preserve new experiments under descriptive result names and
record their assumptions in the investigation.

## Rule modes

| Mode | Meaning |
| --- | --- |
| C | Current ratio CRT, mixed/base bonuses, suppression/execution split, discarded remainders, and free critical loss. |
| S | One literal synthesis experiment: VBAM effectiveness, all suppression, mixed/base die modifiers, free highest-DS critical kill, discarded remainders. The synthesis does not specify every detail. |
| R | Flat EotS effectiveness, all suppression, budgeted critical focus, probabilistic residual damage, no mixed bonus, undamaged-base modifier. |
| X | R with DD/base execution retained. Called P in the investigation. |

Do not treat X/P as selected for implementation. The residual execution
roll is disputed. The next fleet experiment must isolate three remainder
treatments under otherwise identical rules: discard, probability, and
carry-forward. The present scripts do not yet implement fleet carry-forward.
The ground candidate does carry damage within a battle.

Space modes use simultaneous fire, two full-DS steps, half AS when
crippled, and a 64-round guardrail. Ordinary allocation uses the current
class order. Wins exclude mutual destruction and the guardrail. The ROE6
illustration models withdrawal without pursuit. The unit arrays are
ordered DD, CA, BB, SB, SC, TT, ETAC. Use even AS values in the current
stat sweep: odd-value cripple rounding has not been checked against Rust.

The ship sweep runs serially because unit tables are module globals.
Its cost-6 DD case fields 25 DD against ten CA, both costing 150. The
models start a fresh seeded stream per case. These are reproducible runs,
not a formally paired common-random-number experiment.

## Ground alternatives

`ground.py` enumerates three rules without Monte Carlo sampling:

- Current NC ground CRT, defender column shift, critical, and remnant rule.
- Simultaneous independent army fire: attacker hit probability 0.30,
  defender 0.35; mutual destruction leaves the defender in control.
- Defender-first volleys: armies AS10/DS10; one d10 effectiveness roll per
  side, using 10%, 20%, 20%, 30%, 30%, 30%, 30%, 40%, 40%, 50%. Resolve
  defender fire, then fire with surviving attackers. Partial damage carries
  against each side's next army through that battle. No critical, odds
  column, or extra defender modifier. A wiped-out attacker cannot reply.

The finite-state sums use floating-point probabilities. “Exact” means all
outcomes are enumerated rather than sampled, not symbolic rational
arithmetic. The sequential model loses positive total remaining durability
each exchange and terminates without a round cap. At 1:1, a volley cannot
kill an army immediately; accumulated damage still resolves the battle.
Start with the supplied small cases; state space grows with army counts.

## Limits and validation

Arithmetic checks cover the current lone-DD capital-kill lottery, the
ground cliff, removal before return fire, subthreshold ground damage, and
seeded reproducibility. During preservation, all 56 fleet cases and 22
stat cases reproduced the original review's values exactly.

Space simulations use Python's RNG, not Rust's contextual RNG. They omit
movement, visibility, joins, fleet-specific casualty distribution, pursuit,
planetary fire, economy, repair, and campaign objectives. Cost totals omit
embarked armies. Crippled value is separate from destroyed cost; a repair
bill is an analysis assumption.

The original sampled results contain frequencies rounded to five decimal
places. For their 6,000 and 12,000 trials, the exact win count can be
recovered by nearest-integer rounding. The interval tool rejects that
fallback at 100,000 trials or more. Future sweep output should store exact
outcome counts and its rule configuration.

Use [nc-harness](../../rust/nc-harness/src/play.rs) and an eventual
[nc-fuzz](../../rust/nc-fuzz/src/lib.rs) odds example to verify selected
rules against actual maintenance behavior. Vary the actual combat seed;
replaying an unchanged scenario with seed 0 is not an odds sweep. Fuzzer
case frequencies are not estimates of player encounter frequencies.
