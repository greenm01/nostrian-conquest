---
id: 7h5r6ne8
date: 2026-10-10
kind: plan
tags: [plan, storage, architecture]
---
# Typed game state in Rust and SQLite

## Context

The runtime is SQLite-authoritative in name only. Core game state lives in
memory as classic `.DAT`-shaped byte buffers: `CoreGameData`
(`rust/nc-data/src/directory/types.rs`) holds `PlayerDat`, `PlanetDat`,
`FleetDat`, `BaseDat`, `IpbmDat`, `SetupDat`, `ConquestDat`, each a
`raw: [u8; N]` with accessors (`rust/nc-data/src/records/*.rs`). The SQLite
`snapshot_*` tables mirror those records with 26 hex columns and dozens of
raw-int columns (`rust/nc-data/src/storage/mod.rs` 426-604,
`snapshot_core.rs`). The March 2026 work (`55803026`, `9e7ec06c`,
`ead3979b`, `a5dd9d90`, docs `e813d167`) stopped after splitting records
into columns. Today's oracle work showed wrong accessor offsets were gameplay
bugs, not file bugs.

User decisions (2026-10-10):
- Game state is plain typed Rust; SQLite uses typed columns; classic byte
  layouts live only in the `.DAT` codecs (`nc-classic`, `nc-compat`). Already
  written into `AGENTS.md` and `docs/dev/rust-architecture.md`.
- Decode every remaining classic byte before t039 finishes; no opaque
  "unmapped" storage.
- Migrate with typed fields plus temporary accessor shims, then move call
  sites to fields crate by crate.
- Immutability (state-in, state-out maintenance) is out of scope.

Outcome: plain structs with named, typed fields for every entity; rules as
free functions; typed SQLite tables; classic records only in the import and
export codecs. Gameplay does not change: every test, the byte-for-byte
fixture round trips, and the classic oracle sweeps stay green throughout.

Scale (from exploration): ~3,000 non-test and ~5,000 test call sites on
record accessors; nc-engine never touches `.raw`; `.raw` outside `records/`
is in `storage/snapshot_core.rs` (32), nc-cli `inspect.rs`/`compare.rs`/
`fleet_order.rs`/`bombard.rs`, and `nc-compat/src/database_projection.rs`.

## Phase 0 — land the in-flight layout work

Resume and merge t036 (classic battle; also fixes the PLANETS.DAT production,
goods, and stardock layout), t037b (starbase movement; PLAYER 0x44/0x46 as
base chain), and t038 (fleet id/link words, speed change). They fix field
meanings the typed model is built from. Full suite green on main after each.

## Phase 1 — decode every remaining byte (parallel RE agents, no Rust edits)

One agent per file, using the decompiled ECMAINT/ECGAME (`tools/unlzexe/ecmaint_640k.bin`, `ecgame_640k.bin`; ECUTIL for SETUP) and
classic probes (`tools/ncmaint_oracle.py`, DOSBox-X via `nix shell`):

- CONQUEST.DAT: the ~2,000 bytes after the control header (0x55..0x824),
  plus the unnamed control words/bytes 0x0A..0x54.
- SETUP.DAT: bytes 13..511 and 514/519/521 (ECUTIL/ECGAME config).
- IPBM.DAT: the whole 32-byte record (only owner is semantic today).
- PLAYER.DAT gaps: 0x2F, 0x3A-3B, 0x3E-3F, 0x47, 0x4A-4B, 0x4D, the
  0x52 overlap, the two readings of 0x1A-1B.
- PLANETS.DAT gaps after t036: name suffix 0x1D-23, population 0x52-57,
  economy marker 0x0E, 0x59, 0x5B, 0x5F-60, build queue layout (t037b found
  ECGAME reads ten u16 counts indexed by kind), conversion countdown 0x5E.
- FLEETS.DAT: the 0x34 word's meaning (strength cache?).

Deliverable: `docs/dev/ec-file-formats.md`, one table per file covering every
byte (offset, width, type, meaning, [P]/[C] evidence). Any classic bug found
is reported for the user's decision, not silently copied.

## Phase 2 — typed model and codecs (additive, behaviour unchanged)

- New `rust/nc-data/src/model/`: `Player`, `Planet`, `Fleet`, `Starbase`,
  `Ipbm`, `Setup`, `Conquest` with `pub` typed fields from Phase 1. Domain
  types where they earn their keep (`Order`, `Coords`, `FleetId`,
  `StarbaseId`, `EmpireId`, ship-count struct, build-queue and stardock
  structs, `Real48` kept for bit-exact motion). Links and ids stay explicit
  fields (classic chain semantics are needed for the oracle).
- Codecs in `nc-classic` (add a dependency on `nc-data`; nc-data does not
  depend on nc-classic): `decode_*`/`encode_*` per file, lossless.
- Round-trip tests: every `.DAT` under `fixtures/` and `original/v1.5`
  decodes and re-encodes byte for byte (extend `nc-data/tests/fleet_layout.rs`
  / `base_layout.rs` pattern into nc-classic tests).
- `records/*` become thin shims: each `*Record` wraps the typed struct and
  keeps its existing accessor methods (now reading fields), so all call sites
  compile unchanged. `raw` disappears; `parse`/`to_bytes` delegate to the
  codecs. `rng.rs` `derive_campaign_seed_from_runtime` keeps hashing the
  classic encoding (via the codec) so seeds don't change.

## Phase 3 — typed SQLite schema

- Rewrite the seven `snapshot_*` tables as one typed column per field
  (child tables for list-shaped data: diplomacy rows, build queue, stardock,
  maintenance schedule) in `storage/mod.rs` and `snapshot_core.rs`; drop
  `hex.rs` use for core state. Bump `RUNTIME_SCHEMA_VERSION` (unreleased;
  pre-1.0 policy, no migration).
- Update `nc-data/tests/storage.rs` schema pin (298) to assert no hex/raw
  columns in core tables; keep every byte-for-byte import/export test green.
- Hosted JSON (`nc-nostr/src/state_sync.rs`, built in
  `nc-host/src/game/state.rs`) reads the typed model.

## Phase 4 — move `.DAT` I/O to the compat edge

- Move `CoreGameData::load`/`save` (`nc-data/src/directory/io.rs`) into
  `nc-compat`. Route the non-compat callers through the store or nc-compat:
  `nc-data/src/builder.rs:447`, `nc-data/src/turns/runtime.rs:87,101`,
  nc-host `commands/maint.rs`, `game/state.rs`, `game/worker.rs:1208`,
  `support/runtime.rs`, nc-cli `core.rs`, `setup.rs`, `sysop.rs`,
  `runtime.rs` (incl. the raw `fs::write` at 95-100), `ipbm.rs`,
  `inspect.rs`, `compare.rs`, `maint.rs`.
- `current_known_diff.rs` byte diffs move to nc-compat.

## Phase 5 — remove the shims, crate by crate

Move call sites from accessor methods to fields in order nc-data, nc-engine,
nc-cli, nc-game, nc-helm, nc-host, nc-harness, nc-fuzz, nc-sysop, tests;
rename `_raw` names to domain names; delete each shim when its last caller is
gone. Tests that poke `.raw` (e.g. `nc-cli/tests/maint.rs` 126,
`nc-data/tests/maint_fleet.rs` 72) set fields or go through the codec.
Extend `nc-data/tests/source_policy.rs` and `nc-engine/tests/source_policy.rs`
to forbid classic record types outside nc-classic/nc-compat.

## Phase 6 — docs and tracking

`rust-architecture.md` (current state), `ec-file-formats.md` linked from the
specs, `fixtures.md`, next-session note; close t039.

## Execution

Phase 1 agents can start as soon as this plan is approved (they edit no
Rust), alongside Phase 0. Phases 2-5 run as worktree agents in sequence, each
phase split into several signed commits; every commit keeps
`cargo test --no-fail-fast` green. Builds use `CARGO_TARGET_DIR` under
`~/.cache`, never `/tmp`.

## Verification

- `cd rust && cargo test --no-fail-fast` green after every commit.
- Byte-for-byte: every `.DAT` in `fixtures/` and `original/v1.5` round-trips
  through codec and through SQLite import/export.
- Oracle: `tools/ncmaint_mission_sweep.py` and the four `ncmaint_*_audit.py`
  tools give the same results as before t039.
- Schema check: no `*_raw_hex` or opaque columns in core tables
  (`sqlite3 ncgame.db .schema`).
- Source policy tests: no classic record types outside the codec crates.

## Connections

- t039 in [todo.md](../../../todo.md).
- [rust-architecture.md](../../dev/rust-architecture.md): the Core Rules this
  plan implements.
- `docs/dev/ec-file-formats.md`: the Phase 1 byte maps (written in Phase 1).
