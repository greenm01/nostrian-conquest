---
id: x8arlb3c
date: 2026-10-10
kind: plan
tags: [plan, milestone]
---
# Implementation waves after t039

## Scope and exit

Ten tasks wait on [t039 phase 5](7h5r6ne8-typed-game-state-in-rust-and-sqlite.md),
and several touch the same engine files. This plan orders them so that
parallel agents work on disjoint files and each wave merges green. Exit:
every task below merged with `cargo test --workspace --no-fail-fast` green
on main, the manuals rebuilt, and the classic oracle sweeps unchanged where
a task does not deliberately change classic behaviour.

## Wave A: storage and classic edge (parallel)

- **t076** stale-session saves: the store's save path and the session
  layers that call it (nc-data storage, nc-game, nc-host). Priority A.
- **t056** build queue at the classic edge, folding in **t063**: the planet
  model's build slots, the PLANETS codec (in its new edge crate after
  phase 5), the engine's build pass, nc-game's build screen.
- **t064** and **t069** home pickers and salvage: nc-engine movement,
  retargeting, salvage, and the autopilot orders in economics/ai.rs.

These three touch disjoint files except nc-data's model, where t056 owns
the planet build slots and the others do not edit the model.

**t084** (economy decisions 1-2: fractional accounting, smooth growth, the
colony head start) follows t056 in the same lane, because whole-point
spending from fractional goods changes the build pass t056 rewrites.

## Wave B: combat (two parallel lanes)

- **Fleet lane, in order:** t078 (DD 10/8, the unit table), t065 (decision
  2: per-fleet ROE, unarmed escape, pursuit episodes, the cap, NC AS as the
  one yardstick), t066 (Recovery Turns: cohort counts, turn-cycle timing,
  damage-aware strength, display limits), t079 (surprise memory: a new
  typed table, so after t076's storage change). Files: maint/combat
  exchange, fleet_battle, retreat, state; maint/encounter; the fleet model.
- **Planetary lane, in order:** t071 (decision 4, the landing battle,
  blitz included), then t074 (decision 5, the battery shield and failed
  landings on Hold). Files: maint/combat/assault.rs and its tests.

The two lanes share only the combat spec and the manual's combat chapters;
each lane edits its own sections, and the second to merge resolves the
prose. Decision 6's participation items (t073) follow when approved.

### Follow-up after t066

**t093** standardizes ASCII ship composition notation (healthy and damaged),
documents it in the manual/help, and adds `3!`
crippled-only input to the existing door Detach and Transfer flows. Plain
quantities become healthy-first; recovery cohorts stay intact. See the
[approved interface plan](tafp4e55-compact-crippled-ship-notation-and-condition-aware-detach-and-transfer.md).
Coordinate its shared fleet mutations with the fleet lane; keep existing
BBS columns and menus.

## Wave C: the rest

t070 rogue fleet orders and the starbase spec correction; t077 the nc-helm
year filter; t061 hosted maintenance (candidate); t051 the economy review,
which now has the settled combat rules to work from; t024 the beta.3
Windows packages once the queue is clear.

## Rules for every wave

Each agent works in its own worktree from current main, runs cargo through
`cargo-slot` (a per-worktree pool), waits on log markers, never on
`pgrep -f`, signs every commit, and does not push. Claude merges each
branch, runs the full suite on main, and pushes.
