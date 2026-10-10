# Starbase Economy Oracle Audit

This note records the current black-box state of the starbase economy follow-up.

Goal:

- verify whether classic uses a specific starbase growth bonus
- understand the "withstand tax burden better" wording from the manuals

## Manual Baseline

The original docs are clear on the semantic direction:

- taxes above roughly `65%` can harm production
- a starbase lets a planet spend up to `5x` current production on builds
- a starbase helps a planet grow faster
- a planet with a starbase "may endure a tax rate of `67%` to `70%`, but
  there is no guarantee"

That gives a strong player-facing spec, but not an exact internal formula.

## Controlled Probe

Current reproducible tooling:

- CLI initializer:
  [rust/nc-cli/src/commands/economy.rs](../../rust/nc-cli/src/commands/economy.rs)
  `economy-starbase-probe-init`
- Oracle runner:
  [tools/ncmaint_starbase_economy_audit.py](../../tools/ncmaint_starbase_economy_audit.py)

Probe shape:

- create a fresh runtime campaign
- activate a stable player-facing setup
- seed one harmless movement order so classic definitely runs a yearly pass
- create two identical owned probe colonies:
  - `Plain Colony`
  - `Base Colony`
- commission a friendly active starbase at `Base Colony`
- export through `nc-compat`
- run classic `ECMAINT /R`
- import and compare the two colony rows

## Current Result

What *is* confirmed from the current probe family:

- the commissioned starbase survives the oracle run as a real active base
- the post-oracle imported colony still shows the starbase tag
- the post-oracle imported colony still shows `5x` build-capacity semantics
  through the larger `build_capacity` value

What is **not** reliable enough yet:

- the exact current-production growth bonus
- the exact tax-burden threshold or penalty rule

The generated probe worlds still produce pathological post-oracle production
values that do not line up cleanly with the manuals. Representative sweep
results from the current generated baseline:

- `25%`: plain `50 -> 34`, starbase `50 -> 36`
- `50%`: plain `50 -> 45`, starbase `50 -> 37`
- `65%`: plain `50 -> 32`, starbase `50 -> 32`
- `67%`: plain `50 -> 32`, starbase `50 -> 36`
- `70%`: plain `50 -> 32`, starbase `50 -> 32`

Those outputs are too noisy and counterintuitive to promote into a canonical
classic formula. The likely conclusion is not "classic really behaves exactly
like this"; it is that the current generated colony-state probe is still not a
clean enough oracle baseline for exact economy recovery.

## Fixture-Backed Colony Sweep

The more useful follow-up is a fixture-backed ordinary-colony sweep against the
accepted `ecmaint-econ-pre/v1.5` baseline, using:

- one owned non-homeworld colony at a time
- the same colony in plain vs active-starbase form
- the same tax rate in both cases
- classic `ECMAINT /R` over the exported directory

Reproducible tooling:

- [tools/ncmaint_starbase_colony_sweep.py](../../tools/ncmaint_starbase_colony_sweep.py)

Current two-colony result:

- active starbases consistently preserve the `starbase` tag and `5x`
  build-capacity effect
- active starbases consistently increase the imported `grow` column on
  underdeveloped colonies
- at lower taxes (`25%`, `50%`), the starbase colony also ends the year with
  higher `present` and `rev`
- across the manual warning band (`65%` through `80%`), both tested colonies
  keep the same imported `present` and `rev` with and without a starbase, but
  the starbase colony still shows a higher imported `grow` value

Representative `Probe` colony results:

- `25%`: plain `present=22 rev=5 grow=9`, starbase `present=26 rev=6 grow=12`
- `50%`: plain `present=19 rev=9 grow=6`, starbase `present=22 rev=11 grow=9`
- `65%`: plain `present=16 rev=10 grow=5`, starbase `present=16 rev=10 grow=8`
- `67%`: plain `present=16 rev=10 grow=5`, starbase `present=16 rev=10 grow=8`
- `70%`: plain `present=16 rev=11 grow=4`, starbase `present=16 rev=11 grow=6`
- `80%`: plain `present=16 rev=12 grow=3`, starbase `present=16 rev=12 grow=5`

Representative `ProbeB` colony result at `67%`:

- plain `present=16 rev=10 grow=2`, starbase `present=16 rev=10 grow=3`

This does **not** recover an exact classic internal formula, but it is the
clearest current oracle evidence of the player-facing effect:

- starbases definitely improve colony growth under tax pressure
- starbases definitely give `5x` build capacity
- the current oracle sweep does **not** show a separate immediate
  current-production / revenue threshold shift at `65..70%`

## Practical Conclusion

At the moment, the project can safely claim:

- classic/manuals clearly support `5x` starbase build capacity
- classic/manuals clearly support a separate starbase colony-growth benefit
- the manuals' "withstand tax burden better" wording is currently best read as
  "retains stronger colony growth under taxation", not as a recovered hard
  `65 -> 70` immediate-production threshold shift
- the exact classic numeric rule for that benefit remains unrecovered

So the Rust policy remains:

- keep starbase `5x` build capacity
- keep a tax-sensitive starbase growth bonus as canonical Rust behavior:
  - full `+50%` over base growth at tax `<= 50%`
  - linearly tapering to `0%` by tax `65%`
- use the same high-tax penalty threshold for all planets (`65%`)
- do **not** describe those exact numbers as oracle-verified classic formulas

## If Revisited

The next useful step is **not** another blind tax sweep on the original
generated baseline.

Do one of these first:

1. deepen the accepted fixture-backed colony sweep if more confidence is needed
   across additional colony shapes
2. or do a deeper static RE pass on the planet-side economy functions in the
   unwrapped `ECMAINTU.EXE` project

Until then, this thread is evidence of the limit of the current black-box
formula recovery, not evidence for a new exact classic numeric rule.
