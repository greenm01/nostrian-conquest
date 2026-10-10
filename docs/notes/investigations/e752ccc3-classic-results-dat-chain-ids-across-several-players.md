---
id: e752ccc3
date: 2026-10-09
kind: investigation
status: closed
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

Those first observations came from one single-player fixture.

### Oracle runs with several human empires

Original `ECMAINT /R` was run through
`nix shell nixpkgs#dosbox-x -c python3 tools/ncmaint_oracle.py run <dir>`
(DOSBox-X 2026.08.31) on copies of `fixtures/ecmaint-fleet-battle-pre/v1.5`
made with `python3 tools/ncmaint_oracle.py prepare <dir> <fixture>`. A replay
of the unmodified fixture matched its preserved `RESULTS.DAT` except the random
stardates, so the harness is sound.

- **Two humans.** Empire 1 was made human (`PLAYER.DAT[0x00]=0x01`, handle
  `BAR`). Both empires' fleets met in System(10,10). `RESULTS.DAT` held 21
  records: empire 1 headers 1, 6 with (ChainId, NextChainId) (0,6), (1,0);
  empire 2 headers 12, 16 with (0,16), (12,0). PLAYER.DAT: empire 1
  `+0x38=1`, `+0x3c=6`; empire 2 `+0x38=12`, `+0x3c=16`; empires 3 and 4 zero.
  Empire 2's first header has ChainId 0 even though it follows empire 1's
  reports.
- **Three humans, interleaved.** Empire 3 was also made human (handle `BAZ`),
  and the location, movement and order fields `FLEETS.DAT[0x0A..0x24]` of
  fleets 1 and 9 were swapped so empire 1's fleet arrives late. The 52-record
  file interleaves chains in event order: empire 3 headers 1, 6, 42, 47;
  empire 2 headers 12, 16, 31, 36; empire 1 headers 22, 26. Every header's
  ChainId names the previous header of the same empire and its NextChainId
  the next one, both as absolute 1-based record indexes. PLAYER.DAT: empire 1
  (22, 26), empire 2 (12, 36), empire 3 (1, 47).
- Every report went to exactly one empire's chain. Where two empires saw the
  same contact, each got its own report from its own fleet. No report was
  shared between chains, and none of these runs produced a broadcast report.
- Continuation and EOT records carry the header's ChainId and NextChainId 0.
  Bytes past each record's declared text length hold stale buffer contents.

Both runs are preserved as `fixtures/ecmaint-results-two-human-{pre,post}` and
`fixtures/ecmaint-results-interleaved-{pre,post}`, with the `.oracle`
before/after snapshots under each `-pre` fixture.

ECGAME report review was not run. `tools/capture_ecgame_report_review.py`
needs the DOSBox-X debugger shell (`DEBUGBOX`, `MEMDUMPBIN`), which the
nixpkgs build lacks, and the local debugger build is broken.

## Finding and resolution

`RESULTS.DAT` is one shared file. Each empire's reports form their own chain
with absolute 1-based header indexes. PLAYER.DAT `+0x38` is the first header
and `+0x3c` the last header of that chain, both zero without reports. The
earlier "flag" and "next free" readings were wrong; `+0x38=1` in the old
fixture only meant the chain started at record 1.

Fixed in Rust:

- `nc_data::layout_classic_results` lays out the shared file from report rows:
  per-empire blocks in ascending empire order, broadcast rows copied into
  every active human empire's chain, all chain ids rewritten to absolute
  header indexes. It returns each empire's first and last header.
- `ClassicResultsLayout::apply_to_players` and
  `PlayerRecord::set_classic_results_chain` are the only writers of the
  pointers. The engine (`build_results_dat`, `apply_results_reviewable_flags`),
  the `nc-compat` export, the scenario harness, and the nc-game startup
  controller use them. Harness `results-block player=N` rows now belong to
  empire N.
- The `nc-compat` import tags each report with the empire whose chain reaches
  it, and the classic decoder now ignores stale bytes past a record's declared
  length when byte 0 carries the record count, as ECGAME does. Before, ECMAINT output with stale padding fell back
  to the legacy decoder and lost the report boundaries.
- The PLAYER.DAT accessors are renamed to `classic_results_first_header_raw`
  and `classic_results_last_header_raw`, with the contradicting comment
  replaced.

[classic-results-dat-compat.md](../../dev/classic-results-dat-compat.md) holds
the confirmed contract.

## Validation and remaining work

Regression tests, all failing on the previous code and passing now:

- `rust/nc-compat/tests/classic_results_export.rs`: per-viewer rows built from
  the two-human oracle file export byte for byte to the oracle `RESULTS.DAT`
  and the same PLAYER.DAT pointers; rows from the interleaved oracle export
  to the same per-empire chains; importing the interleaved oracle directory
  tags rows with empires 1 to 3 and re-exports every chain.
- `rust/nc-engine/tests/results_routing.rs`: with broadcast and targeted
  reports for several empires, every empire's chain from `+0x38` is absolute,
  ends at `+0x3c`, and every report belongs to exactly one chain.

Remaining limits:

- Rust writes contiguous per-empire blocks, not ECMAINT's event-order
  interleave. ECMAINT itself writes contiguous blocks when events fall in
  empire order, so ECGAME reads that shape, but an ECGAME read-back of a
  Rust-laid-out file with several empires is still unrun.
- Deleting reports in the runtime store marks broadcast rows (viewer `0`)
  deleted for every empire, and deleting one of an empire's reports by
  `block_index` also deletes the broadcast row with the same index, because
  block indexes restart per viewer. Both live in
  `nc-data/src/storage/report_blocks.rs` and are outside this task.

Tracked as t008 in [todo.md](../../../todo.md).

## Connections

[classic-results-dat-compat.md](../../dev/classic-results-dat-compat.md) holds
the current record contract. Found during the
[October 2026 bug sweep](../milestones/ym5u2c1x-engine-and-door-bug-sweep-of-october-2026.md).
