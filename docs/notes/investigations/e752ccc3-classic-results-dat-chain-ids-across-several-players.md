---
id: e752ccc3
date: 2026-10-09
kind: investigation
status: investigating
tags: [investigation, compat, reports]
---
# Classic RESULTS.DAT chain ids across several players

## Question

Does the classic RESULTS.DAT export chain each player's reports correctly when
more than one human empire has reports?

## Evidence

From the October 2026 reports review, without an oracle run:

- In fixture `ecmaint-fleet-battle-pre/.oracle/after-ecmaint`, RESULTS.DAT is
  one shared file of 22 records. Its report headers are records 1, 6, 12, and
  17, with (ChainId, NextChainId) pairs (0,6), (1,12), (6,17), (12,0). These
  are absolute 1-based record indexes.
- That fixture's only human player has PLAYER.DAT `+0x38 = 1`, the first
  header, and `+0x3c = 17`, the last header index rather than a "next free"
  value.
- Rust's `build_rows_for_viewer` restarts chain ids for each viewer. Both
  `export_snapshot_to_dir` and `build_results_dat` in nc-compat concatenate
  viewers, so every viewer block after the first points into earlier blocks.
- Writers of the PLAYER.DAT pointers disagree. `apply_results_reviewable_flags`
  writes `+0x3c` as a record count, nc-harness `build.rs` writes count plus 1,
  and the nc-game startup controller writes 1. The comment in
  `nc-data/src/records/player.rs` contradicts the fixture.

These are observations from one single-player fixture. That each player's
chain uses absolute indexes across the whole file, and that broadcast reports
must be copied into each chain, is inferred, not proven.

## Finding and resolution

Open. The responsible boundary is the nc-compat export, not the runtime
report store.

## Validation and remaining work

Needs a classic run with two human players, both with reports, read back
through ECGAME. Tracked as t008 in [todo.md](../../../todo.md).

## Connections

[classic-results-dat-compat.md](../../dev/classic-results-dat-compat.md) holds
the current record contract. Found during the
[October 2026 bug sweep](../milestones/ym5u2c1x-engine-and-door-bug-sweep-of-october-2026.md).
