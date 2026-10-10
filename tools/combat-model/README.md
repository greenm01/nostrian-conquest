# NC combat model

Python arithmetic models of NC combat, written for the October 2026 combat
review (`docs/notes/investigations/rolr9ti0-independent-nc-combat-balance-analysis.md`).
They model the combat rules, not the engine: no movement, detection,
pursuit, repair, or economy.

- `nc_combat_analysis.py`: the current CRT (mode C), a literal synthesis
  candidate (S), and the flat-table experiment (R, called P in the note).
  Writes `nc_combat_results.json` beside itself. Standard library only.
- `nc_ship_sweep.py`: one-variable AS/DS/cost sweeps for destroyers against
  ten cruisers; output in `nc_ship_sweep.log`.

Confidence intervals in the note came from SciPy's `binomtest`; the models
themselves need no third-party packages. Seed 20261010 reproduces the note's
tables.
