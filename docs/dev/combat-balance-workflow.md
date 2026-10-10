# Combat Balance Workflow

How a combat rule candidate gets numbers before anyone changes the engine.
The tools are in [tools/combat-model](../../tools/combat-model/README.md);
the candidates come from the
[combat review](../notes/investigations/ebrogywq-nc-combat-design-review-against-its-sources.md)
and the
[independent analysis](../notes/investigations/rolr9ti0-independent-nc-combat-balance-analysis.md).

## Three levels of evidence

1. **Exact enumeration.** A d10 exchange has 100 roll pairs, so a small
   battle is a finite state space and its odds are a sum, not an estimate.
   `sweep.py` enumerates every fleet matchup whose state space fits its
   limit and every ground case; the ground candidates (`ground.py`,
   `nc_combat_analysis.py`) are exact at any size the sweep uses. Exact
   numbers settle cliffs and thresholds, such as one army or one cruiser
   changing a capture or win chance by tens of points.
2. **Seeded sampling.** Larger battles are sampled with a seed derived from
   the run seed, the rule set, the hull table, and the case, so every number
   reproduces and carries a 95% interval (Clopper-Pearson with SciPy,
   Wilson without). Near 50%, 10,000 trials give about one point either
   way; a candidate that moves a case by less than that has not been shown
   to move it.
3. **Campaign scenarios.** The model has no movement, sighting,
   interception, repair, or economy. Claims about pursuit, convoys,
   raids, relief, and sieges need `nc-harness` scenarios through real
   maintenance, with fresh campaign seeds per trial;
   `rust/nc-fuzz/examples/combat_odds.rs` is the pattern (it calls the
   seeded engine entry point directly, because `ScenarioPlay::run_maintenance`
   passes seed 0 and repeats the same luck).

The model is trusted only as far as it has been checked against the
engine: `combat_odds` runs the current rules on the cases tagged `engine`
and `sweep.py --engine` tests the model's current-rule odds against the
engine's counts. Rerun that check after any engine combat change; a
candidate's numbers mean nothing if the baseline has drifted.

## Comparing candidates

- Keep the matchups, hull table, seed, and trial count fixed across rule
  sets; change one rule field at a time when the question is one rule
  (`--set remainder=carry`).
- Read the outcome split, not only the left win rate: mutual destruction,
  stalemates (a rule with discarded remainders can leave two crippled ships
  unable to finish each other), withdrawals, exchanges fought, destroyed
  build points, and the crippled remainder a repair rule would charge.
- Treat 100% as "no loss seen", with the interval shown, not as certainty.
- Record the run (its Markdown and JSON, under `tools/combat-model/nc_*`)
  and the assumptions in the investigation note that asked the question.
  Appendix B of the player manual takes its odds tables from the same
  output once a rule set is selected.

## Adding a candidate

1. If it is a new value of an existing field, add a preset to
   `RULESETS` in `rules.py` with `variant(...)`, or pass `--set` on the
   command line, or write a JSON rule file and pass its path to `--rules`.
2. If it is a new mechanic, add the field to `CURRENT` in `rules.py` (the
   engine's value is the default) and handle it in `fleet_model.py`, in
   `volley`, `apply`, or `step`, keeping the branch-list shape so exact and
   sampled runs stay one code path. Add a case to `CASES` in `sweep.py` if
   the existing matchups cannot show the mechanic.
3. Add a check to `test_sweep.py`: the semantics on a hand-built state,
   and, for anything that touches the current rules, the exchange-for-exchange
   match with the review model.
4. Run the sweep, then the engine check for the current rules, and save
   both outputs.
