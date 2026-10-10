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

### t016

Add the proptest driver over the same core: a strategy generates a choice
list that the game consumes, and failures persist in proptest's regressions
file. Then add the remaining invariants: fleet identity across removals, ship
conservation, spending within budget and no negative treasury, ownership
changes only through events, report records within 72 printable bytes sent
only to entitled empires, and SQLite plus classic round trips.

### t017

On a scratch branch, revert the stale fleet-index remap, the combat round cap,
the battle-order determinism fix, and the broadcast-viewer fix. Run both
drivers; record which finds each bug, how long it takes, and how small the
shrunk scenario is. Use the result to choose the default driver.

## Connections

Later work, not yet tasks: metamorphic checks (permuted player slots should
mirror outcomes; a far-away bystander should change nothing), a cargo-fuzz
driver over the same choice source, cargo-mutants to grade the invariants,
and occasional exports run through the original ECMAINT under DOSBox.
