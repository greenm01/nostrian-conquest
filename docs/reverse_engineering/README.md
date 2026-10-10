# Reverse Engineering and Compatibility

This directory is the repo's provenance and evidence entrypoint: how the
classic rules, file formats, and yearly maintenance behavior were recovered,
and how the Rust side is still checked against the original game.

The top-level README is intentionally product-facing. This page is the
"how we know" side of the project.

## How EC Was Recovered

The Rust engine was not built from guesswork. The current model and docs came
from repeated cross-checking between the original manuals, the original DOS
binaries, preserved fixtures, and controlled Rust-generated scenarios.

| Tool / source | What it was used for | Why it mattered |
| --- | --- | --- |
| Current Rust manuals plus original EC manuals in [`original/v1.5/*.DOC`](../../original/v1.5) | Current manuals are authoritative for the Rust edition; original manuals are the historical/ambiguity fallback for classic rules, setup constraints, turn structure, and terminology | Kept the Rust clone grounded in intended game behavior instead of raw binary quirks alone |
| Ghidra disassembly and headless scripts | Static recovery of file layouts, maint flow, scheduler logic, and helper call structure | Turned opaque Pascal-era code paths into stable Rust-facing specs |
| DOSBox-X debugger, INT 21 tracing, and memory dumps | Dynamic tracing of `ECGAME` / `ECMAINT` behavior, file I/O order, token handling, and live state changes | Proved phase ordering, runtime transitions, and report/output boundaries that static RE alone could not settle |
| Controlled gamestate file diffs | Compared Rust-generated or hand-shaped directories against classic `.DAT` outputs before and after maintenance | Exposed real cross-file invariants and kept the Rust side honest at the compatibility boundary |
| Report and log analysis | Studied `RESULTS.DAT`, `MESSAGES.DAT`, shipped `ec*.txt` logs, and preserved output captures | Recovered player-visible timing, report cadence, `Stardate` behavior, and event sequencing |
| Rust-generated scenarios and oracle sweeps | Created narrow test cases, ran the original binaries as oracle, and promoted repeated outcomes into shared rules | Turned reverse engineering into reusable implementation guidance instead of one-off notes |

## Working Rule

The rule that fell out of that recovery work is simple:

- the current Rust manuals are the gameplay/sysop guide
- the original `.DOC` set is the historical/ambiguity fallback
- the original binaries are the compatibility oracle
- classic `.DAT` files are the interchange boundary
- Rust may be explicit, deterministic, and testable where the original
  implementation was hidden, stochastic, or buggy
- original bugs are documented when they matter to compatibility, but not
  cloned unless they are required for file safety or oracle acceptance

## Practical Workflow

- start with manuals, preserved fixtures, or a Rust-generated controlled
  directory
- mutate one narrow mechanic or order family
- run original `ECMAINT` or `ECGAME` as oracle
- diff `.DAT` files and report artifacts, then promote repeated outcomes into
  shared Rust rules and regression tests
- escalate to Ghidra or DOSBox-X only when black-box probing plateaus or a real
  compatibility blocker requires deeper recovery

## Oracle Runbooks

### Default ECMAINT black-box loop

For new mechanics, the concrete workflow is:

1. `python3 tools/ncmaint_oracle.py prepare <target_dir> [source_dir]`
2. Submit one controlled set of orders or mutate one narrow field family.
3. `python3 tools/ncmaint_oracle.py run <target_dir>`
4. Inspect the `.oracle/` snapshots plus the printed diff clusters across
   state files (`PLAYER.DAT`, `PLANETS.DAT`, `FLEETS.DAT`, `BASES.DAT`,
   `IPBM.DAT`, `CONQUEST.DAT`) and report/output files (`RESULTS.DAT`,
   `MESSAGES.DAT`, `ERRORS.TXT`, `DATABASE.DAT`, `RANKINGS.TXT`).
5. Treat "no report output" as evidence too: a mechanic that mutates
   persistent state while leaving `RESULTS.DAT` / `MESSAGES.DAT` empty is
   still useful for placing the mechanic inside the yearly simulation core
   rather than the report side.
6. Promote only strong repeated rules into shared Rust logic.

### Known-scenario replay loop

1. `python3 tools/ncmaint_oracle.py replay-known fleet-order /tmp/ecmaint-fleet-oracle`
2. Inspect the `.oracle/` snapshots and the comparison against the preserved
   post-maint fixture.
3. Use the same pattern for `planet-build` and `guard-starbase` before opening
   a new mechanic.

### Preserved-fixture replay validation

1. `python3 tools/ncmaint_oracle.py replay-preserved fleet-order /tmp/ecmaint-fleet-pre-direct`
2. Confirm the preserved pre-maint fixture replays to the preserved post-maint
   fixture exactly.
3. Use `replay-known` to measure the remaining gap in the Rust-generated
   pre-maint state, not to question the oracle harness itself.

### Deep RE escalation

Use static/dynamic RE when a blocker survives repeated black-box tests. Prefer
narrow, reproducible captures over broad exploratory tracing. Stop the deep
dive once the missing rule can be stated precisely enough for Rust
validation/generation.

The anti-rabbit-hole rule: do not apply full deep-dive treatment to every
mechanic. If a path is not currently blocking broader compliant generation,
keep it in the black-box queue until it becomes a real blocker.

## Current Posture

- the heavy RE phase is closed for normal development; day-to-day work should
  bias toward the Rust engine, classic export correctness, and the Rust client
- original DOS binaries remain the compatibility oracle, not the product front
  end
- deep RE should reopen only for a concrete oracle diff, crash, or gameplay
  mismatch that blocks broader compliant generation
- [docs/dev/next-session.md](../dev/next-session.md) carries the current
  project handoff, not an ongoing oracle lab notebook

## Where To Read

- [docs/dev/approach.md](../dev/approach.md)
  - compatibility policy, milestone ladder, and deliberate divergence rules
- [docs/dev/next-session.md](../dev/next-session.md)
  - current restart brief, recorded proof baseline, and remaining compatibility
    work
- [docs/dev/archive/RE_NOTES.md](../dev/archive/RE_NOTES.md)
  - chronological archival notebook
- [NC_UNLOCKED/README.md](../../NC_UNLOCKED/README.md)
  - decrypted runnable binaries used for static and dynamic analysis
- [docs/dev/dosbox-workflow.md](../dev/dosbox-workflow.md)
  - oracle-running and dump-capture workflow
- [docs/dev/ghidra-workflow.md](../dev/ghidra-workflow.md)
  - headless/static RE workflow

## Known Deliberate Divergences

- Rust does not reproduce the original lone-active `ScoutSolarSystem` abort
  bug in `ECMAINT`
- Rust does not reproduce the legacy rogue-viewer foreign-scout refresh quirk
- Rust keeps deterministic, explicit behavior where the original implementation
  was hidden, stochastic, or not worth cloning literally
