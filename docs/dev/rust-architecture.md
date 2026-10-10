# Rust Architecture

This document defines the standing Rust-side architecture rules for this repo.
It is the code-ownership and module-boundary companion to the behavior specs.

Read it together with:

- [rust-turn-cycle-implementation.md](rust-turn-cycle-implementation.md)
  for yearly maintenance phase ordering
- [nc-combat-spec.md](nc-combat-spec.md)
  for combat and hostile world-resolution mechanics
- [ec-timing-spec.md](ec-timing-spec.md)
  for weekly timing and report `Stardate` behavior
- [economics.md](economics.md)
  for post-loop build/economy policy
- [bbs_door_client_rust.md](bbs_door_client_rust.md)
  for player-client direction

Authority boundary:

- the subsystem spec docs own gameplay behavior and turn ordering
- this document owns Rust crate boundaries, data flow, and module structure
- if a behavior spec and this doc ever disagree on rules, the behavior spec
  wins; this doc should then be updated to match

## Core Rules

The Rust workspace should follow pragmatic data-oriented design:

- model game state as plain, typed Rust structs with named fields, and keep
  rules in focused free functions that operate on them
- store that state in SQLite as typed columns, one column per field
- keep classic byte layouts and `.DAT` codecs in `nc-classic` and
  `nc-compat` only; the engine, store, and clients never read or write
  classic record bytes
- keep binary layout explicit inside those codecs
- keep logic DRY by centralizing shared validation, normalization, and report
  helpers
- keep source files lean; when a file starts feeling crowded, split it before
  growing it further
- avoid deep object hierarchies, framework-style indirection, and giant
  monolithic modules
- carry a classic field whose meaning is unknown as raw bytes only inside the
  codecs, until docs, oracle diffs, or repeated observation explain it
- treat preserved fixtures, 1992 manuals, and the 1992 binaries as the
  acceptance oracle

## Workspace Ownership

### `nc-data`

`nc-data` is the shared runtime/store/model crate.

It is responsible for:

- the typed game state under `model/`: `Player`, `Planet`, `Fleet`,
  `Starbase`, `Ipbm`, `Setup`, and `Conquest`, plain structs with `pub`
  fields named by meaning; each field's doc comment gives its classic offset
- the classic `.DAT` codecs under `classic/`: `decode_*` and `encode_*` per
  record and parse/serialize per file, following
  [ec-file-formats.md](ec-file-formats.md). `classic::ignored_bytes` lists
  the bytes no classic program reads; the model does not carry them, and
  `encode` writes zeros or the documented constant there. t039 phase 4 moves
  the codecs to `nc-classic`, which nc-data cannot depend on
- the `*Record` shims under `records/`: thin wrappers over the model that
  keep the accessor API call sites still use. They go in t039 phase 5;
  shims that only exist for that are marked `// t039: remove in phase 5`
- `CoreGameData` and shared multi-file directory semantics
- semantic SQLite runtime persistence and snapshot/history loading
- shared validators, normalizers, report/mail/intel state, and typed helpers
  used by more than one frontend

It should stay focused on runtime/state semantics rather than owning the normal
classic import/export workflow or the external engine API surface.

Operationally:

- the live runtime DB is authoritative for normal play
- snapshot storage should prefer explicit typed columns over opaque grouped
  residue
- classic byte layouts belong in the codecs, not in normal gameplay code;
  a byte comparison against a classic file masks `classic::ignored_bytes`

### `nc-engine`

`nc-engine` is the public Rust engine/rules boundary.

It is responsible for:

- yearly maintenance behavior
- economy, movement/pathfinding, setup/map generation, and combat-facing rule
  surfaces
- the stable public API that CLI/client/harness code should use for gameplay
  logic

Engine callers should depend on `nc-engine` rather than reaching directly into
`nc-data` for rules.

### `nc-classic`

`nc-classic` is the low-level classic-file support crate.

It is responsible for:

- raw classic record types that are still shared across the compat boundary
- classic byte-level parse/encode helpers
- keeping classic byte mechanics out of the runtime/store API surface

It should stay small and dumb:

- explicit layouts
- minimal parsing/encoding helpers
- no runtime-store policy
- no gameplay rules

### `nc-compat`

`nc-compat` owns the explicit classic compatibility boundary.

It is responsible for:

- importing classic directories into normalized SQLite runtime state
- exporting normalized SQLite snapshots back to classic `.DAT` directories
- classic report/database projection helpers used only for oracle and hybrid
  workflows
- keeping classic file handling out of normal engine/client code

This is the only layer that should need to think in terms of classic
directories as a workflow boundary.

### `nc-sysop`

`nc-sysop` is the public sysop/admin surface.

It is responsible for:

- new-game creation and setup/admin flows
- yearly maintenance execution
- live campaign operator actions that belong in normal play

It should stay thin:

- `main.rs` is process boundary only
- dispatch owns top-level routing
- command modules orchestrate workflows but do not own rules

Game rules should not be reimplemented in command modules. If a command needs a
shared rule, that rule belongs in `nc-engine` (backed by shared runtime/store
types from `nc-data`).

### `nc-cli`

`nc-cli` is the internal developer/oracle/inspection surface.

It is responsible for:

- command dispatch and workflow orchestration
- oracle and compliance sweeps
- import/export and runtime/storage bridge commands
- inspection, reporting, and scenario materialization helpers
- temporary compatibility shims during public CLI transitions

It should stay thin:

- `main.rs` is process boundary only
- `dispatch.rs` owns top-level routing
- `commands/` is grouped by workflow or command family
- `support/` holds shared parsing/path helpers
- `workspace.rs` owns shared directory/bootstrap helpers

Game rules should not be reimplemented in command modules. If a command needs a
shared rule, that rule belongs in `nc-engine` (backed by shared runtime/store
types from `nc-data`).

### `nc-game`

`nc-game` is the player-facing application layer and currently ships as the
`nc-game` binary.

It is responsible for:

- rendering, input, startup flow, and screen/domain navigation
- player-facing review/edit flows over runtime state
- terminal/layout/theme concerns
- player-report presentation

It should not duplicate game rules. The client consumes `nc-data` runtime/store
types and `nc-engine` rule surfaces instead of re-deriving combat, movement,
build, or maintenance semantics locally.
It also should not own classic `.DAT` projection; if a workflow needs classic
files, that belongs in `nc-cli` export/materialization code.

Keep the client structure similarly explicit:

- `src/app/` is the thin shell:
  - root app state
  - top-level action enum
  - reducer/update loop
  - shell-wide helpers
- `src/domains/<domain>/` owns domain-specific:
  - screen state
  - render/update logic
  - any `App` methods specific to that domain
- `src/screen/` owns shared rendering primitives and screen IDs

Do not leave large domain controllers parked under `src/app/` once a real
domain module exists for them.

### `nc-fuzz`

`nc-fuzz` is the test-only engine fuzzer. It plays seeded games through
`nc-harness` with generated turns submitted through the real turn KDL path,
checks invariants after every maintenance turn, and writes failures as
`nc-harness` scenarios. Drivers (Hegel and proptest, in its `tests/`) only
supply choices. It owns no rules: invariants reuse `nc-engine` and `nc-data`
helpers, and a rule it needs belongs in those crates.

## Current Structural Direction

The current workspace shape is:

```text
rust/
├── nc-classic
│   └── src/          # low-level classic record/codecs
├── nc-data
│   ├── src/model/     # typed game state
│   ├── src/classic/   # .DAT codecs (moving to nc-classic)
│   ├── src/records/   # accessor shims over the model (pending removal)
│   ├── src/storage/   # SQLite campaign store + snapshot bridge modules
│   └── shared runtime/support modules
├── nc-engine
│   └── src/          # public engine/rules surface
├── nc-compat
│   └── src/          # classic import/export and compat projections
├── nc-sysop
│   └── src/          # public sysop/admin/maintenance workflows
├── nc-cli
│   ├── src/commands/  # developer/oracle/runtime/compat workflows
│   └── src/support/   # shared CLI helpers
└── nc-game
    ├── src/domains/   # feature/domain slices + domain controllers
    ├── src/app/       # thin app shell/state/update/action seams
    ├── src/screen/    # screen/layout primitives
    └── terminal/startup/theme helpers
```

This doc intentionally describes the module families rather than a frozen file
inventory. The structure should keep evolving when a cleaner split buys clarity,
but the ownership boundaries above should remain stable.

## Boundary Sketch

```text
+--------------------------------------------------------------+
|                         Frontends                            |
|--------------------------------------------------------------|
|  nc-game      nc-sysop         nc-cli         nc-harness   |
|  player TUI   sysop/admin    dev/oracle/compat  scenarios/tests |
+--------------------------------------------------------------+
                |          |                |
                | normal   | normal         | explicit classic/oracle
                | runtime  | runtime        |
                v          v                v
+--------------------------------+   +-------------------------+
|           nc-engine            |   |        nc-compat        |
|--------------------------------|   |-------------------------|
| gameplay rules                 |   | classic DAT workflows   |
| maintenance                    |   | import/export bridge    |
| mapgen / movement / economy    |   | oracle materialization  |
+--------------------------------+   +-------------------------+
                 |                              |
                 v                              v
+--------------------------------+   +-------------------------+
|            nc-data             |   |       nc-classic        |
|--------------------------------|   |-------------------------|
| semantic runtime store         |   | raw classic records     |
| shared model                   |   | byte codecs             |
| snapshots / reports / mail     |   | DAT parsing/encoding    |
| fog of war                     |   +-------------------------+
+--------------------------------+               |
                 |                               v
                 v                    classic directories / DOS oracles
         SQLite / ncgame.db          DATABASE.DAT / RESULTS.DAT / MESSAGES.DAT
         authoritative state         ECGAME / ECMAINT / DOSBox-X
```

The key visual idea is:

- left side = normal runtime stack
- right side = compat/oracle stack

Even simpler:

`NORMAL PLAY / RUNTIME`

`frontend -> nc-engine -> nc-data -> SQLite`

`CLASSIC / ORACLE`

`frontend -> nc-compat -> nc-classic -> .DAT files / DOS binaries`

Read this sketch with the ownership rules above:

- `nc-game` does not parse classic `.DAT` files
- `nc-engine` owns gameplay rules, not classic file workflows
- `nc-data` owns shared runtime/store/model state
- `nc-classic` owns low-level classic byte/record helpers only
- `nc-sysop` owns public maintenance/setup flows
- `nc-cli` orchestrates explicit compat flows through `nc-compat`
- SQLite is authoritative; `.DAT` is the compatibility/oracle edge

## Maintenance Engine Structure

The Rust yearly maintenance engine is exposed and implemented through
`nc-engine/src/maint/`.

Shared maintenance result payloads remain in `nc-data::maintenance_types` so
multiple crates can consume the same plain event data without duplicating the
rule code.

Use [rust-turn-cycle-implementation.md](rust-turn-cycle-implementation.md)
as the ordering spec, then reflect that ordering in code by phase-oriented
submodules rather than one giant driver file.

Current direction:

- `maint/mod.rs` owns orchestration and shared turn context only
- focused submodules own stable mechanic families such as:
  - sanitize
  - retarget
  - movement
  - merging
  - combat
  - economics
  - campaign
  - events

Guidelines for maint code:

- keep phase ordering explicit
- keep per-phase read/write scope clear
- return typed events from phase helpers instead of burying report-side facts in
  ad hoc string assembly
- move reusable rule logic into focused helpers when two phases or frontends
  need the same behavior
- do not let UI/client concerns leak into maint code

If `maint/mod.rs` starts accumulating mechanic-specific detail again, split that
detail into submodules instead of extending the driver further.

## Shared Model Boundary

`CoreGameData` remains the canonical gameplay-state snapshot model inside
`nc-data`.
`CampaignStore` is the persisted source of truth for active campaigns.

Use it when the code needs:

- shared cross-file validation
- the canonical state snapshot that engine code mutates
- classic directory load/save
- reusable scenario/setup/report helpers
- plain shared event/result payloads

If a transform expresses shared game-directory/runtime-store semantics rather
than one frontend's interaction policy, it should live on `CoreGameData` or in
a closely related `nc-data` helper module.

Examples:

- cross-file validators
- build/fleet/player input validation
- maintenance events
- shared report/intel projections
- raw record-layout helpers such as fleet motion scratch-field access/reset

The CLI and client should orchestrate those helpers, not replace them.

## Storage Boundary

The runtime storage direction is now active, not deferred:

- `CampaignStore` / `CampaignRuntimeState` in `ncgame.db` are the runtime
  source of truth for active campaigns
- `CoreGameData` is the canonical snapshot shape carried through storage and
  shared engine helpers
- classic `.DAT` files remain the compatibility boundary and oracle artifact set
- runtime-facing state should stay structured:
  - `CoreGameData`
  - report blocks
  - queued mail
  - per-player intel
  - campaign seed
- classic compatibility projections such as `DATABASE.DAT` belong to explicit
  import/export helpers, not to the normal runtime/client API surface

Practical rule:

- `nc-game` and normal Rust maintenance/mutator paths read and write SQLite
  runtime state
- explicit compatibility paths such as `db-export`, scenario materialization,
  and oracle setup are the only places that should intentionally write classic
  `.DAT` outputs, normally through `nc-compat`
- explicit import paths such as `db-import` are the only places that should
  rebuild runtime state from a classic directory, normally through `nc-compat`
- read-only inspection/report commands must not create or update `ncgame.db`
  as a side effect
- SQLite is the runtime source of truth; classic files remain the compatibility
  and oracle projection boundary

Do not bypass classic compatibility just because Rust-native storage exists, and
do not couple client/runtime logic directly to `DATABASE.DAT` or other classic
report artifacts. `CampaignRuntimeState` should not require compat-shaped
fields like `DatabaseDat`, `RESULTS.DAT` bytes, or `MESSAGES.DAT` bytes just to
drive the Rust engine or player client.

## Command And Report Ownership

Keep ownership clear:

- rule calculation belongs in `nc-engine`
- operator command selection / argument parsing belongs in `nc-sysop`
- developer/oracle command selection / argument parsing belongs in `nc-cli`
- screen flow / interaction belongs in `nc-game`
- player-visible report timing and header rules belong to the dedicated specs,
  then to shared `nc-engine` / `nc-data` helpers, not to CLI or client-only
  string logic

If the same report/intel/business rule is needed in more than one frontend,
promote it into `nc-engine` or a shared `nc-data` helper, depending on whether
the code is rule logic or runtime/store structure.

## Setup, Movement, Economy, And Combat Boundaries

Subsystem behavior should follow the companion specs:

- setup/map-generation constraints:
  [ec-setup-spec.md](ec-setup-spec.md)
- movement/pathfinding behavior:
  [ec-movement-spec.md](ec-movement-spec.md)
- build/economy behavior:
  [economics.md](economics.md)
- combat/hostile world resolution:
  [nc-combat-spec.md](nc-combat-spec.md)

The architecture consequence is straightforward:

- keep gameplay/rule execution in `nc-engine`
- keep shared models, invariants, and plain event/result payloads in `nc-data`
- keep the specs authoritative
- keep CLI/client layers as consumers of those rules

## Testing Direction

Testing rules:

- keep tests in crate `tests/` directories by default
- prefer regression tests tied to preserved fixtures and oracle-accepted
  directories
- split tests by concern rather than growing one giant integration test file
- after meaningful Rust changes, run `cargo test -q` from `rust/`
- for maint-sensitive behavior, keep using the oracle sweep and Rust maint sweep
  documented in [next-session.md](next-session.md)

The test split should mirror the code split. If a feature area deserves its own
module, it usually deserves its own test surface too.

## Avoid

Do not:

- grow giant `main.rs`, `mod.rs`, or catch-all utility files
- duplicate rules between `nc-engine`, `nc-data`, `nc-cli`, and `nc-game`
- bury classic byte semantics in UI or command code
- treat scenario-specific scripts as the long-term home for shared mechanics
- collapse maint ordering, combat rules, timing rules, and economy rules into
  one mega module
- use speculative semantic names in typed APIs before the evidence supports
  them

## Practical Heuristic

When adding or refactoring behavior, ask:

1. is this gameplay/rule execution, a shared data invariant, or a plain shared
   payload type?
2. is this a frontend workflow over an existing rule?
3. is this a presentation concern only?

Then place it accordingly:

- gameplay/rule execution -> `nc-engine`
- shared model/invariant/plain payload -> `nc-data`
- public sysop/admin workflow -> `nc-sysop`
- developer/oracle/compat workflow -> `nc-cli`
- player interaction/rendering -> `nc-game`

That placement rule matters more than preserving any one historical file tree.
