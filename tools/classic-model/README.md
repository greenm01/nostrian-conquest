# Classic battle model

Python models of classic ECMAINT's battles as decoded for
`docs/dev/ec-battle-spec.md`, used to check the decode against classic
probe runs (1,500+ runs, section 6 of the spec).

- `sim.py`: the planetary assault (bombard, invade, blitz), with classic's
  bugs B1-B3 and B5 included so it matches the binary.
- `fsim.py`: the fleet battle.
- `cmp.py`, `fcmp.py`: compare model runs with probe output.

The probe harness that drove DOSBox-X lives with the session scratch work
and is not included; `tools/ncmaint_mission_sweep.py` is the maintained way
to run classic.
