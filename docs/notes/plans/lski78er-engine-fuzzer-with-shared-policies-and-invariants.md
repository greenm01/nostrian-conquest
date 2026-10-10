---
id: lski78er
date: 2026-10-09
kind: plan
tags: [plan, fuzz]
---
# Engine fuzzer with shared policies and invariants

## Scope and exit

Most bugs in the
[October 2026 bug sweep](../milestones/ym5u2c1x-engine-and-door-bug-sweep-of-october-2026.md)
were invariant violations that random multi-turn play would expose: stale
fleet indexes, a maintenance error from the combat round cap, battle order
that varied between runs, free builds, and off-by-one coordinates. Build a
property-based fuzzer that plays seeded games with generated orders and checks
invariants after every turn, driven by both
[Hegel](https://docs.rs/hegeltest) and
[proptest](https://github.com/proptest-rs/proptest).

Exit: the fuzzer rediscovers the reverted fixes listed under t017 and shrinks
each to a small scenario, and a short run is part of `cargo test`.

## Task details

### t015

Create an `nc-fuzz` crate depending on nc-engine, nc-data, and nc-harness:

- A choice-source trait: pick a number in a range, or a boolean. It is the only
  thing a driver implements.
- Policies that read the live state and spend choices on orders: random valid,
  aggressor, swarm (constant join, split, and rendezvous), adversarial edge
  values, and the autopilot. Choices are indexes resolved against current
  state ("fleet 2 of 7"), so a fixed choice list stays valid as the game
  changes. Orders go through the real turn submission path.
- An invariant checker returning typed violations. Start with three:
  maintenance never errors on a validated state, the same seed gives the same
  result, and every coordinate is within 1 through the map size.
- Failure output as an nc-harness KDL scenario.

Add the Hegel driver: a `#[hegel::test]` that feeds its test case into the
choice source and notes one line per turn. Pin `hegeltest` to an exact
version; the crate is beta and changes often.

Result (2026-10-10): `rust/nc-fuzz` is a workspace crate on nc-engine,
nc-data, and nc-harness. `ChoiceSource` (`pick`, `flip`, and an optional
`note`) is the only driver interface. A case draws 2 to 4 empires on the
18x18 map, a map seed, one policy per empire, and contact-forcing setup
options (one empire's homeworld unowned, every fleet rallied at the map
center, all empires at war, edge-valued armadas), then plays up to
`NC_FUZZ_TURNS` turns (default 8). The policies are random, aggressor, swarm,
adversarial, and autopilot (`house autopilot=#true`). Every action is
rendered to turn KDL, parsed back, and applied with the turn so far through
`TurnSubmission::apply_to`; rejected actions are dropped and a lossy round
trip is a violation. Maintenance runs through the new
`nc_harness::ScenarioPlay`, which carries player lifecycle and winner state
across turns, and runs a second time on a clone to compare. The first
invariants are maintenance errors or panics on a validated state,
nondeterminism, and coordinates within `1..=map_size` with one planet per
sector. A failure is written as `scenario.kdl` (a `combat-scenario`) plus
`turn-NN-empire-E.kdl` files under `$NC_FUZZ_OUT/<driver>`, or the system
temp directory's `nc-fuzz/<driver>`;
`cargo run -q -p nc-cli -- harness run-combat <dir>/scenario.kdl` replays it,
and `tests/core.rs` checks that a written scenario replays to the same state.
nc-harness gained `turn=` on `turn-file` and `autopilot` on `house`, and its
houses no longer overwrite the player mode byte with the record number, which
had left every house but the first neither joined nor in civil disorder.

The Hegel driver (`tests/hegel_driver.rs`) pins `hegeltest = "=0.49.2"`. Its
`static-engine` feature cannot be used: its `rand_core` 0.10 clashes with the
release candidate russh pins, so Hegel's build script compiles the engine
with a nested cargo build, which needs crates.io on a clean build. Each draw
is `draw_silent` and each turn adds one `tc.note` line. Hegel's database
lives in the cargo target directory. Run it with
`cargo test -p nc-fuzz --test hegel_driver`; `NC_FUZZ_CASES` (default 30)
and `NC_FUZZ_TURNS` lengthen it. The whole crate's tests take about 10
seconds in debug.

Fuzzing current behavior found one real bug, recorded in
[Turn files accept record index 0](../investigations/u040gnr1-turn-files-accept-record-index-0.md);
the adversarial policy leaves record 0 out until it is fixed.

### t016

Add the proptest driver over the same core: a strategy generates a choice
list that the game consumes, and failures persist in proptest's regressions
file. Then add the remaining invariants: fleet identity across removals, ship
conservation, spending within budget and no negative treasury, ownership
changes only through events, report records within 72 printable bytes sent
only to entitled empires, and SQLite plus classic round trips.

Result (2026-10-10): `tests/proptest_driver.rs` feeds a generated `Vec<u32>`
(up to 4096 values) to the game as a `ChoiceTape`; an exhausted tape answers
with the simplest choice. Failures persist in
`proptest_driver.proptest-regressions` beside the test (none are committed).
Run it with `cargo test -p nc-fuzz --test proptest_driver` and the same
environment variables. The invariants added:

- Event fleet indexes keep naming a fleet of the event's empire, with its
  fleet number or post-movement location when the event captured one.
- Fleet ships across empires change only by reported battle, retreat,
  bombard, and assault losses, salvage, and civil-disorder defection,
  counting a task force's losses once and the ships merged into a removed
  fleet. A turn where a removed fleet is ambiguous, or a defecting empire
  also fought, is not checked.
- Submitted builds and maintenance spending stay within each planet's build
  budget, and no treasury wraps.
- Planets change owner only with an ownership-change or colonization event.
- Report lines fit 72 printable bytes; rows go only to empires with an
  active player or a terminal outcome; a campaign-outlook announcement
  produces a broadcast row, flagged for every active empire.
- The final state, report rows, mail, lifecycle, and winner survive a SQLite
  round trip. The classic `.DAT` round trip was not added.

1,500-game runs of each driver (12 turns) and 1,500 fixed-tape games on
current behavior found no violation.

### t017

On a scratch branch, revert the stale fleet-index remap, the combat round cap,
the battle-order determinism fix, and the broadcast-viewer fix. Run both
drivers; record which finds each bug, how long it takes, and how small the
shrunk scenario is. Use the result to choose the default driver.

Result (2026-10-10): each fix was undone on its own on a scratch branch (the
remap by reverting 72bc7c2c; the round-cap error, hash-ordered task forces,
battle locations and ROE ties, and the mode-byte broadcast viewers put back
by hand), and each driver ran once in debug with `NC_FUZZ_CASES=3000` and 8
turns. Times are to the first failing case, then the whole run including
shrinking; size is the turns and actions of the shrunk scenario.

| Bug | Hegel | Shrunk | proptest | Shrunk |
| --- | --- | --- | --- | --- |
| Stale fleet-index remap | case 5, 0.5 s; 281 s | 1 turn, 1 action | case 6, 0.6 s; 761 s | 5 turns, 8 actions |
| Round cap returns an error | case 52, 9.8 s; 311 s | 1 turn, 1 action | case 90, 9.6 s; 669 s | 1 turn, 0 actions |
| Battle order in hash order | case 9, 0.7 s; 192 s | 1 turn, 0 actions | case 3, 0.3 s; 0.5 s | 1 turn, 4 actions |
| Broadcast viewers | case 88, 7.9 s; 20 s | 1 turn, 0 actions | case 21, 2.7 s; 38 s | 4 turns, 0 actions |

The broadcast bug was not found at first. With only empire 1 matching, a
sole-contender announcement for another empire was dropped before any row
was built, so the first check (broadcast rows flagged for every active
empire) had nothing to look at, and the winner's own notices flagged it
anyway. Neither driver found it in 3,000 cases until the invariant also
required a broadcast row for every campaign-outlook event; the table shows
that second run. That earlier proptest run also exposed a ship-conservation
false positive, a defecting fleet that had fought the same turn, now
skipped. The remap bug surfaced as a salvage event (Hegel) and a join-host
retarget (proptest) naming the wrong fleet. proptest did not shrink the
battle-order failure because a nondeterministic failure does not recur on
replay. Each row is one random run, so the times are indicative only.

Recommendation: make Hegel the default driver. It finds bugs as quickly,
shrank three of the four to one turn with at most one action, and finished
shrinking sooner on three of them. Keep the proptest driver as a second
opinion and for builds without crates.io access, since Hegel's engine is
compiled by a nested cargo build.

## Connections

Later work, not yet tasks: metamorphic checks (permuted player slots should
mirror outcomes; a far-away bystander should change nothing), a cargo-fuzz
driver over the same choice source, cargo-mutants to grade the invariants,
and occasional exports run through the original ECMAINT under DOSBox.
