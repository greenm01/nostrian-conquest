# Classic battle model

Python models of classic ECMAINT's battles as decoded for
[the classic battle spec](../../docs/dev/ec-battle-spec.md), used to check the decode against classic
probe runs (1,500+ runs, section 6 of the spec).

The [classic-reference investigation](../../docs/notes/investigations/o7w8k8uh-what-the-completed-classic-combat-reference-changes-for-nc.md)
explains what these mechanics imply for NC. Its comparisons and retained
results are documented in the [NC combat model guide](../combat-model/README.md).

- `sim.py`: the planetary assault (bombard, invade, blitz), with classic's
  bugs B1-B3 and B5 included so it matches the binary.
- `fsim.py`: the fleet battle.
- `cmp.py`, `fcmp.py`: compare model runs with probe output.

The probe harness that drove DOSBox-X lives with the session scratch work
and is not included; `tools/ncmaint_mission_sweep.py` is the maintained way
to run classic.
