## 2026-10-10

### Completed This Session

**Engine and door bug sweep (2026-10-09).** A six-slice review found 41
confirmed bugs in combat, maintenance, economy, reports, and the door; all are
fixed with failing-first regression tests. Details, decisions, and validation
are in the
[milestone record](../notes/milestones/ym5u2c1x-engine-and-door-bug-sweep-of-october-2026.md).

**Work tracking adopted.** Open work now lives in [todo.md](../../todo.md),
tracked with todo.txt and zk per [work-tracking.md](work-tracking.md). Start
with `zk queue`.

**v1.0.0-beta.3 (2026-10-10).** The Linux nc-sysop archive is published. The
Windows archives (t024) and the Windows console check (t019) are next, on
Windows after `git pull`; the Windows download links 404 until then. The new
`nc-fuzz` crate runs Hegel and proptest drivers over engine invariants.

**Classic 2.5D movement (t031, 2026-10-10).** Fleets now move as ECMAINT
does: 52 ticks a year, a depth axis for orbit, per-order arrival tests, and
positions stored as Turbo Pascal 6-byte reals with bit-exact arithmetic.
Mission aux bytes match ECGAME. The model and its evidence are in
[ec-movement-spec.md](ec-movement-spec.md) and the
[investigation note](../notes/investigations/9msqq5e0-classic-move-orders-complete-on-entering-the-target-sector.md).
Next on the critical lane: t033 (fleet ship layout), t035 (classic encounter
loop, [ec-encounter-spec.md](ec-encounter-spec.md)), t036 (classic planetary
battle), and t037 (starbase layout and movement). The policy is to match the
classic binaries exactly, except for clear classic bugs approved by the user.

## 2026-05-05

### Completed This Session

**nc-helm starmap UX overhaul** — zoom removed (unused, no gameplay value);
center-follow viewport replaced with 2-sector dead-zone margin scrolling;
middle-click drag-to-pan ("grab the map") with middle-click re-center on
crosshair; crosshair overlay hidden when outside viewport. All nc-helm tests
green.


## 2026-04-23

### Completed This Session

**nc-helm starmap rendering regressions (from `037784a5`)** — all fixed and
committed (`69005f7e`), all 379 nc-helm tests green.

Perf/fix bundle committed in one shot:

- `world_index_for_projection`: replaces O(V×W) per-sector linear scan with a
  `BTreeMap` built once per draw.
- `viewer_fleet_sector_coords_fast`: replaces O(V×F) nested scan with a single
  O(F) pass into a `BTreeSet`.
- `cached_projection_for_app` (`pub(crate)`): `PlayerStarmapProjection` now
  cached on `DashApp.starmap_projection_cache` keyed by
  `(game_data_revision, player)`. Used by `starmap::draw`,
  `jump_planet_target_for_app`, `known_galaxy::body_rows`,
  `planet_view::projected_sector_details`, and
  `planet_view::selected_planet_record_index`.
- `apply_crosshair_overlay` applied after every panel-cache hit/miss.
- `msg_could_change_desired_geometry`: gates `sync_geometry_for_current_window`
  in `Runtime::dispatch` so Wayland compositor round-trips
  (`window.is_maximized()`, `window.fullscreen()`) are skipped on high-frequency
  `Mouse`/`Key` events. Eliminates the ~270ms stall and `Broken pipe` crash on
  `map45-p25`.


- Public gameplay is now centered on `nc-game`, `nc-door`, and `nc-sysop`.
- The old SSH/Nostr hosted path has been cut out of the active gameplay and
  sysop surfaces.
- `nc-sysop` now exposes only `new-game`, `maint`, and `settings`.
- `nc-sysop new-game` no longer seeds hosted seats in `ncgame.db`.
- `nc-game` no longer accepts hosted-only launch flags such as
  `--session-token` or `--hosted-invite-code`.
- `ncgame.db` has dropped the retired hosted/session tables entirely.
- `nc-gate` is no longer part of the active Rust workspace build.
- `nc-client` now exists as the shared hosted client core.
- `nc-dash` now has a partial hosted lobby/client path in the same binary.
- That hosted lobby path now covers public `30500` discovery, hosted join
  request/decision, runtime-backed `30520` state refresh,
  turn submit/receipt, public `30516` notices, encrypted `30518` direct
  contact threads, and encrypted `30523` anonymous per-game diplomacy.
- `nc-dash` lobby now keeps a live hosted observer session instead of doing
  full reconnect/fetch cycles for catalog, notices, direct threads, and
  game-inbox updates.
- `30520 GameState` now uses typed hosted snapshot payloads instead of opaque
  JSON blobs on the Rust side.
- `nc-dash` hosted-game route now builds and renders a real `DashApp` from
  typed hosted snapshots instead of using the old separate mini summary view.
- The old hosted mini-view path has been removed; hosted play now enters the
  real dashboard shell through the shared launch path.
- Hosted dashboard edits now stage a generated `turn.kdl` draft for supported
  fleet-order and planet-build actions, and `T` reviews/sends that staged
  `30522` payload instead of starting from an empty raw-text box.
- `nc-host` now exists as the relay-native hosted server name and localhost dev lab target.
- `nc-host` now exposes `notices` and `threads` operator commands for the
  hosted lobby communication surfaces.
- The BBS door client is verified on Mystic and ENiGMA½.
- Latest local baselines after the hosted-path cut:
  - `cargo test -q -p nc-session`
  - `cargo test -q -p nc-game`
  - `cargo test -q -p nc-sysop`

## Accomplished
- Completed the reverse engineering of the token gate mechanism and the `16A4` integrity bypass flag.
- Exhaustively proved that `DS:16A4` is never set to 1 due to a likely developer typo (command line `/B` sets `16A2`, but the integrity check tests `16A4`).
- Discovered the true reason `.TOK` files "bypass" the crash: the presence of `Move.Tok` triggers an automatic restore of `.SAV` backups over the `.DAT` files prior to the integrity check, causing the repaired files to pass naturally.
- Confirmed the planet stardock capacity: it is a 10-slot array of ship counts (`0x38..0x4B`) and corresponding ship kinds (`0x4C..0x55`), matching the 10-slot build queue.
- Parameterized the Guard Starbase generator to dynamically copy `tuple_a`, `tuple_b`, and `tuple_c` payloads directly from the guarding fleet, completely removing hardcoded byte arrays.
- Documented findings in `token-investigation.md`.
- Rust scenario-writer milestone:
  - `nc-cli` is now split further by feature area:
    - `src/commands/compare.rs`
    - `src/commands/compliance.rs`
    - `src/commands/fleet_order.rs`
    - `src/commands/planet_build.rs`
    - `src/commands/guard_starbase.rs`
    - `src/commands/inspect.rs`
    - `src/commands/ipbm.rs`
    - `src/commands/scenario.rs`
    - `src/commands/setup.rs`
  - `fleet-order` and `planet-build` no longer live as one-off logic inside
    `main.rs`; they now follow the same module/report/init pattern as the
    starbase/IPBM workflows
  - scenario catalog / init / preserved-validation logic also now lives in its
    own command module instead of being embedded directly in `main.rs`
  - compliance/batch-report, compare/preserved-diff, inspect/header dumping,
    and setup/config editing are also now extracted from `main.rs`
  - current refactor checkpoint:
    - `rust/nc-cli/src/main.rs` is now `767` lines, down from the original
      multi-thousand-line monolith
    - full `cargo test` is green after the extraction
  - `nc-data` now parses `BASES.DAT` (`35`-byte records) and `IPBM.DAT`
    (`0x20`-byte records) to the current structural level
  - `nc-cli fleet-order <dir> <fleet_record> <speed> <order_code> <target_x> <target_y> [aux0] [aux1]`
    rewrites decoded fleet-order fields in place
  - `nc-cli planet-build <dir> <planet_record> <build_slot_raw> <build_kind_raw>`
    rewrites decoded build-queue bytes in place
  - latest Rust scenario CLI milestone:
    - `nc-cli scenario <dir> fleet-order`
    - `nc-cli scenario <dir> planet-build`
    - `nc-cli validate <dir> fleet-order`
    - `nc-cli validate <dir> planet-build`
    - `nc-cli validate <dir> all`
    - `nc-cli validate-preserved <dir> <scenario>`
    - `nc-cli validate-preserved <dir> all`
    - `nc-cli compare-preserved <dir> <scenario>`
    - `nc-cli compare-preserved <dir> all`
    - `nc-cli scenario <dir> list`
    - `nc-cli scenario <dir> show <scenario>`
    - `nc-cli scenario-init-all [source_dir] <target_root>`
    - `nc-cli scenario-init [source_dir] <target_dir> fleet-order`
    - `nc-cli scenario-init [source_dir] <target_dir> planet-build`
    - these now turn the two already-preserved accepted fleet/build rewrites
      into named runnable scenarios rather than only low-level mutator calls
    - latest parser/usability correction:
      - `nc-cli init <target_dir>` now correctly uses the documented default
        source `original/v1.5`
      - `nc-cli scenario-init <target_dir> <scenario>` now correctly uses the
        documented scenario baseline `fixtures/ecmaint-post/v1.5`
  - `cargo test -p nc-cli` now proves two exact fixture recreations from the
    compliant `fixtures/ecmaint-post/v1.5` baseline:
    - `FLEETS.DAT` for `fixtures/ecmaint-fleet-pre/v1.5`
    - `PLANETS.DAT` for `fixtures/ecmaint-build-pre/v1.5`
  - new accepted starbase scenario command:
    - `nc-cli scenario <dir> guard-starbase`
    - verified against `fixtures/ecmaint-starbase-pre/v1.5` for:
      - `PLAYER.DAT`
      - `FLEETS.DAT`
      - `BASES.DAT`
    - latest Rust milestone:
      - `BASES.DAT` is no longer emitted from a raw 35-byte template constant
      - `nc-data::BaseRecord` now exposes named setters for the currently
        mapped integrity-critical fields:
        - local slot
        - active flag
        - base ID
        - link word
        - chain word
        - coords
        - tuple A/B/C payload blocks
        - trailing coords
        - owner empire
      - `cargo test` is green for the full Rust workspace after this change
  - `nc-cli validate <dir> guard-starbase` now checks the currently-known
    accepted one-base Guard Starbase invariants across `PLAYER.DAT`,
    `FLEETS.DAT`, and `BASES.DAT`
    - latest refinement:
      - it now validates the one-base linkage keys directly instead of only
        comparing a preserved accepted base blob
      - current explicit checks cover:
        - player starbase-count word
        - fleet local-slot word
        - fleet ID word
        - base chain word
        - guard starbase index/enable bytes
        - base/fleet coordinate agreement
      - Rust-side explicitly linked Guard Starbase encoder milestone:
        - Replaced the hard-coded single-base template with an explicit encoder that generates `BASES.DAT` linkage fields directly from the `PLAYER.DAT` and `FLEETS.DAT` states
        - Enforced strict linkage-key validation checking that `base.summary_word_raw() == fleet.local_slot_word_raw()` and matching `base.chain_word_raw() == fleet.fleet_id_word_raw()`
        - Demonstrated that the kind-1 / kind-2 linkage semantics described by the `3558/355A` decode logic fully align with the preserved one-base fixture values
      - Rust-side next-step milestone already landed:
        - `nc-cli guard-starbase-onebase <dir> <target_x> <target_y>`
        - this emits a parameterized one-base guard-starbase directory while
          keeping the currently-known accepted linkage keys fixed
        - `nc-cli guard-starbase-report <dir>`
        - this prints the current player/fleet/base linkage words and the
          current validator verdict, which is now the quickest Rust-side
          inspection tool before handing a directory to `ECMAINT`
        - `nc-cli guard-starbase-init [source_dir] <target_dir> <target_x> <target_y>`
        - this now materializes a coordinate-parameterized one-base Guard
          Starbase directory in one command, defaulting to
          `fixtures/ecmaint-post/v1.5`
        - `nc-cli guard-starbase-batch-init [source_dir] <target_root> <x:y> <x:y>...`
        - this now materializes multiple coordinate-parameterized one-base
          Guard Starbase directories plus a manifest in one run
        - `nc-cli ipbm-report <dir>`
        - `nc-cli ipbm-zero <dir> <count>`
        - `nc-cli ipbm-record-set <dir> <record_index> <primary> <owner> <gate> <follow_on>`
        - `nc-cli ipbm-validate <dir>`
        - `nc-cli ipbm-init [source_dir] <target_dir> <count>`
        - `nc-cli ipbm-batch-init [source_dir] <target_root> <count> <count>...`
        - these now cover the first practical Rust-side `IPBM.DAT` workflow:
          inspect the current count/length state, then materialize a zero-filled
          valid record family that satisfies the known `PLAYER[0x48]` gate,
          then start populating the decoded record prefix fields from Rust
        - latest refinement:
          - `nc-data::IpbmRecord` now exposes the currently mapped tuple tags,
            tuple payload groups, and trailing control bytes
          - `nc-cli ipbm-report` now prints those groups directly, so Rust-side
            scenario shaping no longer needs raw hex inspection for the mapped
            parts of the record
        - `nc-cli compliance-report <dir>`
        - `nc-cli compliance-batch-report <root>`
        - this now gives the quickest Rust-side preflight for the currently
          mapped integrity-sensitive areas:
          - Fleet Order
          - Planet Build
          - Guard Starbase linkage
          - `IPBM.DAT` count/length state
        - the batch form now gives a compact per-directory verdict for roots
          produced by the Rust scenario generators
        - `nc-cli fleet-order-report [dir] [fleet_record]`
        - `nc-cli fleet-order-init <target_dir> <fleet_record> <speed> <order_code> <target_x> <target_y> [aux0] [aux1]`
        - `nc-cli fleet-order-batch-init <target_root> <fleet_record:speed:order:target_x:target_y[:aux0[:aux1]]>...`
        - `nc-cli planet-build-report [dir] [planet_record]`
        - `nc-cli planet-build-init <target_dir> <planet_record> <build_slot_raw> <build_kind_raw>`
        - `nc-cli planet-build-batch-init <target_root> <planet_record:build_slot_raw:build_kind_raw>...`
        - these now make fleet/build experiment directories as easy to
          materialize and inspect as the existing starbase/IPBM flows
        - the new batch forms now make it practical to generate multiple
          parameterized fleet/build experiment roots without hand-copying the
          same baseline directory repeatedly
  - `nc-cli scenario-init [source_dir] <target_dir> guard-starbase`
    materializes a runnable scenario directory in one command from a
    compliant baseline
  - practical meaning:
    - Rust can now emit accepted pre-maint scenario files from decoded fields,
      not just copy preserved fixture trees wholesale
    - Rust can now generate and validate accepted fleet/build/starbase
      scenarios through a consistent scenario-oriented CLI
    - Rust can now also materialize and inspect parameterized fleet/build
      experiment directories directly from the compliant post-maint baseline
    - Rust can now classify a directory against all currently-known accepted
      scenarios with one command
    - Rust can now also distinguish "matches the currently-known rule shape"
      from "exactly matches the preserved accepted fixture bytes"
    - Rust can now diff a generated scenario directly against the preserved
      accepted fixture files for the same scenario
    - Rust now has a first-class scenario catalog and can materialize all
      currently-known accepted scenarios under one target root in one run
    - Rust can also reject obviously non-compliant starbase snapshots before
      running the original engine
    - Rust can now generate a ready-to-run preserved scenario directory, not
      just mutate an existing one in place
    - the next Rust-side milestone should build on this by replacing more
      scenario-specific raw blobs with explicit field encoders/validators

## Next Steps
The token-gate investigation is complete. The next work should return to the
main maintenance engine and focus on the remaining port-critical unknowns.

Primary project goal:

- reverse engineer enough of the original formats and engine rules to generate
  100% compliant gamestate files from Rust
- treat the original game and `ECMAINT` as the acceptance oracle for that
  milestone
- use that milestone as the first concrete step toward a full Rust port

Later serialization-layer milestone:

- add a KDL-backed scenario/order format only after the internal Rust
  gamestate/order model stabilizes
- treat that as a layer on top of the compliant generator, not as the next RE
  milestone

Preservation TODO:

- preserve original `ECGAME` ANSI opening/menu/report screens for reuse as
  Rust-client reference material
- once the local startup harness is reliable enough, capture:
  - raw ANSI stream output when available
  - rendered text/screen dumps as a fallback
- this is not the immediate next debugging task, but it is now an explicit
  project goal

Rust architecture note:

- recent refactor milestone:
  - `nc-cli` Guard Starbase/IPBM logic now lives in command submodules
  - shared argument/path helpers now live under `nc-cli/src/support/`
  - `nc-cli` workspace/file-copy helpers now live in `src/workspace.rs`
  - top-level CLI usage/help text now lives in `src/usage.rs`
  - top-level command routing now lives in `src/dispatch.rs`
  - `nc-cli` integration tests are now split by command family with shared
    helpers under `tests/common/mod.rs`
  - `nc-data` integration tests are now split by concern:
    - `tests/formats.rs`
    - `tests/mutators.rs`
    - `tests/ecmaint.rs`
    - `tests/common/mod.rs`
  - `nc-data` and `ec-tui` tests now live in crate `tests/` directories
  - latest multi-file compliance milestone:
    - `nc-data::CoreGameData` now provides a typed core-directory load/save
      layer for:
      - `PLAYER.DAT`
      - `PLANETS.DAT`
      - `FLEETS.DAT`
      - `BASES.DAT`
      - `IPBM.DAT`
      - `SETUP.DAT`
      - `CONQUEST.DAT`
    - `guard_starbase`, `ipbm`, and `compliance-report` paths now use that
      shared directory model instead of duplicating per-file load/save logic
    - this is the first step from scenario-specific file choreography toward
      a general compliant gamestate model in Rust
  - latest scenario-composition milestone:
    - `nc-cli scenario <dir> compose <scenario> <scenario>...`
    - `nc-cli scenario-init-compose [source_dir] <target_dir> <scenario> <scenario>...`
    - these now let Rust materialize combined experiment directories from the
      compliant post-maint baseline without hand-chaining multiple commands
    - current verified example:
      - composing `fleet-order` + `planet-build` yields a directory that
        satisfies both validators
  - latest convergence milestone:
    - `fleet-order` and `planet-build` now also operate through
      `nc_data::CoreGameData`
    - all current scenario transforms now share the same typed directory model:
      - `fleet-order`
      - `planet-build`
      - `guard-starbase`
      - `ipbm`
    - practical effect:
      - future multi-step scenario generation can compose these transforms
        without introducing new per-file choreography
  - latest DRY workspace milestone:
    - the repeated `INIT_FILES` overlay loop now lives in the shared
      `nc-cli` workspace layer
    - fleet/build/starbase/IPBM/scenario init paths now reuse that helper
      instead of duplicating the same file-copy boilerplate
  - latest inspection milestone:
    - `nc-cli inspect` now loads through `nc_data::CoreGameData`
    - this gives generated experiment directories a single typed report path
      for players/planets/fleets/bases/IPBM instead of ad hoc per-file loads
    - `nc-cli headers` intentionally stays on the lighter `SETUP.DAT` /
      `CONQUEST.DAT` path so it still works against `original/v1.5`
  - latest core-state milestone:
    - `nc-cli core-report [dir]`
    - `nc-cli core-validate [dir]`
    - these now provide a typed structural report/validator over the shared
      `CoreGameData` model
    - current-known validation scope is intentionally narrow:
      - `PLAYER[1].starbase_count_raw` vs `BASES.DAT` record count
      - `PLAYER[1].ipbm_count_raw` vs `IPBM.DAT` record count
    - this is a reusable structural preflight surface for generated
      experiment directories, not a full `ECMAINT` integrity reimplementation
  - latest normalization milestone:
    - `nc-cli core-sync-counts [dir]`
    - this repairs the current-known player-1 structural count invariants from
      the actual directory contents:
      - `PLAYER[1].starbase_count_raw <- BASES.DAT` record count
      - `PLAYER[1].ipbm_count_raw <- IPBM.DAT` record count
    - practical use:
      - after manual or scripted directory shaping, run `core-validate`
      - if only the current-known count words are wrong, `core-sync-counts`
        can normalize them before higher-level scenario or `ECMAINT` testing
  - latest shared-model milestone:
    - current-known core-state semantics now live on `nc_data::CoreGameData`
    - shared methods now cover:
      - `player1_starbase_count_current_known()`
      - `player1_ipbm_count_current_known()`
      - `current_known_core_state_errors()`
      - `sync_player1_current_known_counts()`
    - practical effect:
      - the CLI core report/validate/sync commands are now thin wrappers over
        the shared data model instead of owning those rules themselves
  - latest shared-mutation milestone:
    - `CoreGameData` now owns the current-known scenario mutation helpers for:
      - fleet-order shaping
      - planet-build shaping
      - one-base guard-starbase shaping
    - practical effect:
      - the CLI mutation commands now delegate to the shared data model instead
        of owning those multi-file transforms themselves
      - this is the first point where the Rust shared model is acting like a
        real gamestate mutation engine, not just a parser/serializer
  - current-known cross-file semantics milestone: complete
    - `CoreGameData` now owns the current-known validation helpers for:
      - fleet-order
      - planet-build
      - one-base guard-starbase
      - IPBM count/length
      - player-1 structural count words
    - direct `nc-data` tests now cover those validation helpers against the
      preserved accepted fixtures
    - practical effect:
      - the CLI is now a thin wrapper for the current-known mutation,
        validation, inspection, and normalization logic instead of the source
        of truth for those rules
  - latest `IPBM` centralization follow-on:
    - `CoreGameData` now also owns the current-known `IPBM` mutation helpers
      for zero-filled record families and mapped prefix-field updates
    - `nc-cli ipbm-zero` and `nc-cli ipbm-record-set` now delegate to the
      shared directory model instead of mutating decoded `IPBM` state in
      CLI-only code
  - latest compliance-report centralization follow-on:
    - `CoreGameData` now owns the current-known compliance status aggregate
      and key-word summary used by the compliance report surfaces
    - `nc-cli compliance-report` and `nc-cli compliance-batch-report` now
      consume that shared model output instead of reassembling current-known
      rule groups in CLI-only code
  - latest starbase-linkage expansion:
    - `CoreGameData` now owns a broader current-known guard-starbase linkage
      helper that follows the selected guarded base by index instead of only
      validating the accepted one-base fixture shape
    - the shared model now exposes selected-base linkage summary fields for:
      - guard index / enable
      - selected base presence
      - selected base ID / summary word / chain word
      - selected base coords / trailing coords / owner
    - `nc-cli` guard-starbase validation still preserves the one-base scenario
      output, but it now does so on top of the broader shared selected-base
      linkage checker
  - latest guarding-fleet aggregation follow-on:
    - `CoreGameData` now exposes the current-known set of guarding fleets and a
      shared aggregated guard-starbase linkage validator for all of them
    - current-known guard-starbase compliance is no longer implicitly tied to
      fleet 1 inside the shared model; it now means “all guarding fleets satisfy
      the selected-base linkage checks”
    - `nc-cli compliance-report` now reports guard-starbase status through that
      shared all-guarding-fleets validator and prints the current guarding
      fleet count
    - this is the current-best shared-model view for guard-starbase compliance:
      not “fleet 1 happens to match the preserved fixture,” but “all guarding
      fleets satisfy the currently-known selected-base linkage rules”
  - phase-close refinement:
    - `CoreGameData` now also exposes full per-guarding-fleet linkage summaries
      for the current-known selected-base rule set
    - `nc-cli compliance-report` now prints those shared guarding-fleet
      summaries directly instead of inferring them in CLI-local code
  - next-phase core-count correction:
    - the shared model now treats `PLAYER[1].starbase_count_raw` as a count of
      player-1-owned base records, not total `BASES.DAT` length
    - `core-validate`, `core-report`, and `core-sync-counts` now expose and use
      `player1_owned_base_record_count_current_known()`
  - follow-on count generalization:
    - the shared model now exposes per-player starbase counts and per-player
      owned-base counts across all five player records
    - starbase-count synchronization now updates all players from owned-base
      counts, while `IPBM` synchronization remains intentionally scoped to
      player 1
  - follow-on fleet topology validation:
    - the shared model now validates the deterministic initialized/post-maint
      four-fleet block topology directly from `FLEETS.DAT`
    - this checks fleet IDs, local slots, and prev/next links across the
      `4 x player_count` roster
    - it also validates the preserved block payload pattern:
      - slots `1/2` => `CA=1 ET=1` and max speed `3`
      - slots `3/4` => `DD=1` and max speed `6`
      - current speed `0`
      - constant location and mission bytes within each empire block
    - important caveat: non-player-1 `PLAYER.DAT.fleet_chain_head` words are
      not currently reliable in the preserved 88-byte player model, so generic
      cross-file validation should continue to prefer fleet-table truth over
      those player-side words
  - follow-on owner-range validation:
    - the shared model now validates `PLANETS.DAT.owner_empire_slot_raw()` and
      `BASES.DAT.owner_empire_raw()` against `CONQUEST.player_count`
    - `core-report` now exposes per-player owned-planet counts alongside the
      existing owned-base and starbase counts
    - this gives a stronger explicit-owner foundation for broader multi-player
      compliance work than the weaker player-side chain words
  - follow-on homeworld alignment validation:
    - the shared model now validates the initialized/post-maint homeworld-seed
      invariant:
      - each active empire has exactly one owned `Not Named Yet` planet
      - that planet's coordinates match the owning empire's fleet-block
        location and mission target bytes
    - `core-report` now exposes per-player `homeworld_seed_coords`
  - follow-on initialized planet-ownership baseline:
    - the shared model now validates that, in the preserved initialized/post
      baseline, owned planets are restricted to those homeworld seeds
    - non-homeworld planets remain unowned
    - owned homeworld seeds carry ownership status `2`
  - follow-on homeworld-seed payload baseline:
    - the shared model now validates the preserved owned homeworld-seed payload:
      - header markers `100 / 135`
      - `developed_value_raw = 10`
      - `likely_army_count_raw = 4`
      - zeroed build queue, stardock, and population payloads
    - `core-report` now exposes `homeworld_seed_payloads`
  - follow-on unowned-planet payload baseline:
    - the shared model now validates the preserved non-homeworld baseline:
      - status/name `Unowned`
      - owner slot `0`
      - ownership status `0`
      - developed value `0`
      - likely-army count `0`
      - zeroed build queue, stardock, and population payloads
    - `core-report` now exposes `unowned_planet_payloads`
  - `nc-data` is now split into domain modules:
    - `src/records/player.rs`
    - `src/records/planet.rs`
    - `src/records/fleet.rs`
    - `src/records/base.rs`
    - `src/records/ipbm.rs`
    - `src/records/setup.rs`
    - `src/records/conquest.rs`
    - `src/support.rs`
  - `nc-data/src/lib.rs` is now crate wiring and reexports instead of another
    monolithic record dump
  - `nc-cli/src/main.rs` is now just the process boundary
  - `nc-cli/src/dispatch.rs` carries the top-level command switchboard
- preserve this direction:
  - data-oriented layout
  - feature/domain submodules
  - thin `main.rs`

Milestone ladder:

1. Known accepted scenarios
   - current state: active and productive
   - Rust can already emit, validate, and materialize preserved accepted
     `fleet-order`, `planet-build`, and `guard-starbase` scenarios

2. Parameterized scenario generation
   - current state: partially started
   - next work here is to replace more accepted-value blobs and
     scenario-specific assumptions with explicit field encoders/validators

3. General compliant gamestate generation
   - blocked on the remaining `ECMAINT` linkage semantics, especially
     `3502` / `3558` / `355A`

4. Full Rust `ECMAINT` replacement
   - deferred until the first three milestones are substantially complete

1. `2000:5EE4` / integrity validator completion
   - Fully map the accepted and rejected structure rules in the early
     cross-file validator, especially the `PLAYER -> BASES -> FLEETS` path.
   - Treat the token question as closed; the remaining task here is the actual
     validation logic, not more `.TOK` experiments.
   - Save additional function names and branch notes in the live Ghidra project
     as the validator becomes clearer.
   - Current correction:
     - artifact: `artifacts/ghidra/ecmaint-live/5ee4-fleet-branch.txt`
     - script: `tools/ghidra_scripts_tmp/Report5EE4FleetBranch.java`
     - confirmed:
       - `2000:6040..6368` is the `FLEETS.DAT` validator branch, not the
         direct `BASES.DAT` loader
       - it opens stream `0x3178` with record size `0x36` and copies the
         active fleet record into local scratch at `[BP+0xFF3E]`
       - it emits kind-`1` summary entries through `0x2F72` / `0x2F76`
       - the early synthetic two-base integrity abort is therefore distinct
         from the later `Fleet assigned to an unknown starbase` behavior
   - Immediate next target:
     - recover the remaining kind-`1` / kind-`2` linkage semantics around
       `3502`, `3558`, and `355A`
     - goal: replace the current accepted single-base scenario encoder with a
       more general Rust-side base/fleet linkage validator instead of
       scenario-specific accepted values
   - Current kind-`1` follow-up:
     - artifact: `artifacts/ghidra/ecmaint-live/kind1-scratch-function.txt`
     - script: `tools/ghidra_scripts_tmp/ReportKind1ScratchFunction.java`
     - confirmed:
       - `0000:02ED..03D5` is the kind-`1` mirror of the generic summary
         loader, using scratch block `0x3502`
       - it consumes:
         - `350D`, `350F..3513`
         - `350E`, `3515..3519`
         - `3522`, `3523`
         - `351B..351F`
         - capped byte `3524`
         - selector/count byte `350C`
       - in the initial kind-`1` load path, the only explicit summary input
         passed to the `3502` loader is `ES:[DI+0x06]`
       - summary bytes `+0x01` / `+0x02` emitted by the fleet branch are not
         read there; they are overwritten later by the shared canonicalization
         stage
       - `5EE4` fleet-scratch offsets now correlate cleanly to the known fleet
         layout:
         - `[BP+0xFF40]` -> `record[0x02]`
         - `[BP+0xFF41]` -> `record[0x03..0x04]`
         - `[BP+0xFF43]` -> `record[0x05..0x06]`
         - `[BP+0xFF49]` -> `record[0x0B]`
         - `[BP+0xFF4A]` -> `record[0x0C]`
       - the second sub-branch is therefore following the per-empire
         `next fleet` link, not selecting another starbase-side record
       - working hypothesis:
         - summary `+0x06` is carrying a fleet-chain identifier
         - `player[0x40]` in the first sub-branch is likely the empire's
           head-of-chain fleet ID, not a count
     - correction:
       - raw-import entry `2000:C067` is not yet a trustworthy semantic
         function start; it decodes as a fragment inside a larger helper region
       - treat `3502` field correlation, not `C067` naming, as the next
         productive task
     - milestone:
       - the kind-`2` path at `0000:03DF..06AE` actively scans the summary
         table for a matching active kind-`1` entry before it finalizes the
         current base-side summary
       - candidate summary requirements:
         - same summary `+0x00`
         - kind `+0x04 == 1`
         - active/status `+0x03 != 0`
         - and then either:
           - direct word match `candidate +0x0A == [0x3558]`
           - or same `+0x01`, `+0x02`, `+0x05` plus helper-decoded `+0x06`
             matching `[0x355A]` with decoded kind `4` and flag `0`
       - practical consequence:
         - the later `unknown starbase` behavior is now best modeled as a
           failed kind-`1` / kind-`2` summary pairing problem
         - Rust-side compliant gamestate generation will need fleet/base
           linkage values that survive this summary pairing, not just
           individually plausible records
     - new base-side emitter mapping:
       - artifact: `artifacts/ghidra/ecmaint-live/5ee4-base-branch.txt`
       - script: `tools/ghidra_scripts_tmp/Report5EE4BaseBranch.java`
       - confirmed:
         - `2000:63D3..6759` is the `BASES.DAT` validator / kind-`2` summary
           emitter
         - kind-`2` summary `+0x01` / `+0x02` come from base coords
           `0x0B` / `0x0C`
         - kind-`2` summary `+0x0A` comes from base `0x02..0x03`
         - kind-`2` summary `+0x06` comes from:
           - `player[0x44]` in the primary branch
           - base `0x07..0x08` in the follow-on linked-base branch
       - practical consequence:
         - the remaining unknown pairing keys around `3558/355A` are now
           narrowed to values derived from `player[0x44]` or base `0x07..0x08`,
           not the coordinate or flag fields already mapped
     - new matcher decode milestone:
       - artifact: `artifacts/ghidra/ecmaint-live/kind2-matcher.txt`
       - script: `tools/ghidra_scripts_tmp/ReportKind2Matcher.java`
       - confirmed:
         - the kind-`2` matcher at `0000:03DF..06AE` does not compare `3558`
           and `355A` as raw base fields
         - it first pushes base-side summary `+0x06`, then decodes it through
           helper `0x2000:c09a` into scratch rooted at `3558`
         - the direct accept path compares candidate kind-`1` summary `+0x0A`
           against decoded `[0x3558]`
         - the structural accept path decodes candidate kind-`1` summary `+0x06`
           through helper `0x2000:c067` and compares decoded word against
           `[0x355A]` with decoded kind `4` and flag `0`
       - practical consequence:
         - `3558` / `355A` are now best modeled as helper-decoded linkage keys
           derived from summary `+0x06`, not as raw persistent fields
     - correction after helper dump:
       - artifact: `artifacts/ghidra/ecmaint-live/kind2-decode-helpers.txt`
       - script: `tools/ghidra_scripts_tmp/ReportKind2DecodeHelpers.java`
       - `0x2000:c067` and `0x2000:c09a` are still not trustworthy semantic
         function starts in the raw live import
       - they currently sit inside a dense helper island that includes
         arithmetic and counted-string helpers
     - caller-pattern milestone:
       - artifact: `artifacts/ghidra/ecmaint-live/kind2-helper-callers.txt`
       - script: `tools/ghidra_scripts_tmp/ReportKind2HelperCallers.java`
       - confirmed shared contract:
         - callers push source summary word `ES:[DI + 0x06]`
         - then push a destination far pointer
         - then call either `0x2000:c067` or `0x2000:c09a`
       - concrete high-value cases:
         - `0000:0307` decodes summary `+0x06` into `DS:3502`
         - `0000:03fe` decodes summary `+0x06` into `DS:3558`
         - `0000:0681` decodes candidate kind-`1` summary `+0x06` into a
           local buffer and then checks:
           - decoded `+0x1f` as kind byte
           - decoded `+0x23` as word matched against `[0x355A]`
           - decoded `+0x0a` as flag byte
     - decoded-field-layout milestone:
       - artifact: `artifacts/ghidra/ecmaint-live/kind2-decoded-field-uses.txt`
       - script: `tools/ghidra_scripts_tmp/ReportKind2DecodedFieldUses.java`
       - confirmed:
         - `3502` and `3558` are sibling decoded summary-`+0x06` structures
           with the same tuple-oriented layout
         - shared shape:
           - tag byte `+0x0b` with payload words `+0x0d/+0x0f/+0x11`
           - tag byte `+0x0c` with payload words `+0x13/+0x15/+0x17`
           - control/scalar group rooted at `+0x20`
         - the local structural-match buffer from `0000:0681` confirms:
           - decoded kind byte at local `+0x1f`
           - decoded word at local `+0x23`
           - decoded flag byte at local `+0x0a`

2. `IPBM.DAT` resolution
   - Practical status: structurally complete enough for Rust-side compliant
     file generation.
   - Confirmed:
     - record size `0x20`
     - `PLAYER[0x48]` is the record count gate
     - the raw record copies contiguously into `DS:3538`
     - the overlapping field map is documented in `docs/dev/archive/RE_NOTES.md`
   - Remaining work is semantic naming of some tuple payloads, not file-layout
     discovery or integrity-critical structure.
     - script: `tools/ghidra_scripts_tmp/ReportIPBMTailTransition.java`
     - confirmed:
       - common writeback always updates summary offsets `+0x01`, `+0x02`,
         `+0x05`
       - kind `2` has an extra side path through helper `0x2000:c100`
       - kind `3` writes finalized tuples back into:
         - tuple A -> `3541`, `3543..3547`
         - tuple B -> `3542`, `3549..354d`
         - tuple C -> `354f..3553`
     - practical implication:
       - `3555..3557` are outside the main tuple A/B/C writeback and should be
         treated as a separate trailing control group
   - Current boundary result:
     - `353D` is only consumed by the second `5EE4` `IPBM` branch
     - `3555..3557` are only visible inside the kind-`3` path in `0000:02c0`
     - practical implication:
       - kind `3` has a primary normalized field group
         (`3541`, `3543..3547`, `3542`, `3549..354d`, `354f..3553`)
       - plus a trailing group (`3555..3557`)
       - while `353B/353D` likely belong to the second-branch follow-on result
         path rather than the generic trailing group
   - First live baseline capture:
     - artifacts:
       - `artifacts/ecmaint-ipbm-debug/registers-6870.txt`
       - `artifacts/ecmaint-ipbm-debug/scratch-3538-6870.txt`
     - setup:
       - valid one-record baseline with `PLAYER[0x48] = 1`
       - zero-filled `IPBM.DAT` of length `0x20`
       - breakpoint at live `2814:6870`
     - key observed normalized values:
       - `353D = 0x0001`
       - `3543 = 0x0080`
       - `3549 = 0x0080`
       - observed `3541`, `3542`, and `354F..3557` bytes/words are zero
     - practical implication:
       - this is the baseline shape to compare against future mutated `IPBM`
         records
   - First mutated correlation point:
     - artifacts:
       - `artifacts/ecmaint-ipbm-debug/off_00_val_01-registers.txt`
       - `artifacts/ecmaint-ipbm-debug/off_00_val_01-scratch.txt`
     - setup:
       - `IPBM.DAT[0x00] = 0x01`
       - all other bytes zero
       - same breakpoint at live `2814:6870`
     - observed delta vs baseline:
       - `3538` changed from `0x0000` to `0x0001`
       - baseline `353D = 0x0001` cleared to `0x0000`
       - baseline `3543 = 0x0080` cleared to `0x0000`
       - baseline `3549 = 0x0080` cleared to `0x0000`
     - practical implication:
       - raw `IPBM` offset `0x00` definitely feeds tuple C / the summary-`+0x0A`
         word path
       - it also suppresses the zero-record default normalization that
         previously produced `353D = 1` and the paired `0x0080` defaults
   - Second mutated correlation point:
     - artifacts:
       - `artifacts/ecmaint-ipbm-debug/off_01_val_01-registers.txt`
       - `artifacts/ecmaint-ipbm-debug/off_01_val_01-scratch.txt`
     - setup:
       - `IPBM.DAT[0x01] = 0x01`
       - all other bytes zero
       - same breakpoint at live `2814:6870`
     - observed delta vs baseline:
       - `3538` changed from `0x0000` to `0x0100`
       - baseline `353D = 0x0001` cleared to `0x0000`
       - baseline `3543 = 0x0080` cleared to `0x0000`
       - baseline `3549 = 0x0080` cleared to `0x0000`
     - practical implication:
       - raw `IPBM[0x00..0x01]` maps directly into `3538` as a little-endian
         word
       - tuple C / early summary `+0x0A` is therefore confirmed to derive from
         the first `u16` in the raw record
   - Expanded prefix mapping:
     - artifacts:
       - `artifacts/ecmaint-ipbm-debug/off_02_val_01-scratch.txt`
       - `artifacts/ecmaint-ipbm-debug/off_03_val_01-scratch.txt`
       - `artifacts/ecmaint-ipbm-debug/off_04_val_01-scratch.txt`
       - `artifacts/ecmaint-ipbm-debug/off_05_val_01-scratch.txt`
       - `artifacts/ecmaint-ipbm-debug/off_06_val_01-scratch.txt`
       - `artifacts/ecmaint-ipbm-debug/off_07_val_01-scratch.txt`
       - `artifacts/ecmaint-ipbm-debug/off_09_val_01-scratch.txt`
       - `artifacts/ecmaint-ipbm-debug/off_0a_val_01-scratch.txt`
     - confirmed:
       - `IPBM[0x02]` copies to scratch `353A`
       - `IPBM[0x03]` copies to scratch `353B`
       - `IPBM[0x04]` copies to scratch `353C`
       - `IPBM[0x05]` copies to scratch `353D`
       - `IPBM[0x06]` copies to scratch `353E`
       - `IPBM[0x07]` copies to scratch `353F`
       - `IPBM[0x09]` copies to scratch `3541`
       - `IPBM[0x0A]` copies to scratch `3542`
     - semantic read with `2000:5EE4`:
       - `353A` is the player / empire byte copied to summary `+0x00`
       - `353B..353C` is the non-aligned `u16` that gates the second `IPBM`
         branch
       - `353D..353E` is the non-aligned `u16` later written to summary `+0x06`
       - `3541` and `3542` are the kind-`3` tag bytes written to summary
         `+0x01` and `+0x02`
     - practical implication:
       - the front of the raw `IPBM` record copies contiguously into scratch,
         then `ECMAINT` interprets overlapping byte/word fields over that copy
       - baseline all-zero defaults like `353D = 1` and `3543 = 3549 = 0x0080`
         are derived normalization, not the raw on-disk values
   - Group-start confirmation:
     - artifacts:
       - `artifacts/ecmaint-ipbm-debug/off_0b_val_01-scratch.txt`
       - `artifacts/ecmaint-ipbm-debug/off_11_val_01-scratch.txt`
       - `artifacts/ecmaint-ipbm-debug/off_17_val_01-scratch.txt`
       - `artifacts/ecmaint-ipbm-debug/off_1d_val_01-scratch.txt`
     - confirmed:
       - `IPBM[0x0B]` copies to scratch `3543` (tuple-A payload block start)
       - `IPBM[0x11]` copies to scratch `3549` (tuple-B payload block start)
       - `IPBM[0x17]` copies to scratch `354F` (tuple-C payload block start)
       - `IPBM[0x1D]` copies to scratch `3555` (trailing control group start)
     - static follow-up from `0000:0723..0797`:
       - `3555` and `3556` are read as scalar bytes and expanded through
         helper `0x3000:486B`
       - `3557` is clamped to at most `1`, so raw `0x1F` behaves like a
         boolean / capped mode byte
     - dynamic clarification:
       - `artifacts/ecmaint-ipbm-debug/off_1e_val_01-scratch.txt` confirms
         `IPBM[0x1E]` copies to `3556`
       - `artifacts/ecmaint-ipbm-debug/off_1f_val_02-scratch.txt` shows the
         first `5EE4` stop still sees raw `3557 = 0x02`
       - the clamp to `1` therefore happens later in shared summary logic,
         not during the initial record-to-scratch copy
     - practical implication:
       - the coarse full-record layout is now stable enough for Rust-side
         binary encoding even though several gameplay semantics remain unnamed

3. Build queue / stardock encoding
   - Continue the partially solved `PLANETS.DAT[0x38]` / `0x4C` work.
   - Determine how completed production is stored in the stardock, including
     multiple ship types/counts and the exact handoff to later commissioning.
   - Goal: enough format/behavior detail to reproduce build completion without
     depending on the original executable.

4. Maintenance phase ordering around `Move.Tok`
   - Now that `Move.Tok` is understood as the crash-recovery marker, map the
     broader maintenance phase boundaries around it.
   - Determine what runs before movement, what runs after movement, when
     backups are written, and when token files are created/deleted.
   - Goal: recover the high-level phase order for faithful reimplementation of
     the maintenance engine.

Suggested execution order:
- First finish `2000:5EE4` branch mapping with emphasis on `BASES.DAT` and
  `IPBM.DAT`.
- Then move to build queue / stardock mechanics.
- Finally map movement-phase ordering and backup/token lifecycle around
  `Move.Tok`.
## ECGAME Dropfiles

- `tools/ecgame_dropfiles.py` is now the shared writer for local `ECGAME` door files.
- `setup_commission_test.py` and `fix_dropfiles.py` were normalized to use the same 32-line WWIV-style `CHAIN.TXT` with explicit DOS CRLF endings.
- This removes the earlier repo inconsistency where `CHAIN.TXT` helpers disagreed on both line count and line endings.
- Non-active `ECGAME` pexpect harnesses were also fixed to pass argv correctly via `pexpect.spawn(cmd[0], cmd[1:], ...)` instead of `pexpect.spawn(" ".join(cmd), ...)`.
- Root cause: the old join-based form caused `/L` to be consumed by DOSBox-X as an outer option rather than passed through to `ECGAME`.
- Confirmed effect:
  - `tools/dump_ecgame_memory.py` once again produces `/tmp/ecgame-dump/MEMDUMP.BIN`
  - the old bogus `Unknown option l` warning disappears under the corrected argv path
- New `ECGAME` launch clarification:
  - once the corrected harness truly reaches `ECGAME`, `/L` is interpreted as a door-file path and fails with `C:\/L\`
  - the best current local-launch assumption is plain `ECGAME` / `ECGAME.EXE` with normalized `CHAIN.TXT` auto-detection, not `ECGAME /L`
- Another stale-script bug:
  - several old `DEBUGBOX` helpers never issued `RUN`, so their later `send()` calls were targeting the debugger prompt instead of the game
- New reliable `ECGAME` pause point:
  - on the corrected no-`/L` path, `BPINT 21 3D` breaks inside live `ECGAME`
  - `DOS MCBS` at that stop shows `ECGAME` as PSP `0814`
  - that is currently the best hook for startup RE
- Important current limitation:
  - equivalent non-debug launches of `ECGAME.EXE` and `ECGAME.EXE C:\CHAIN.TXT` currently return immediately with no visible `ERRORS.TXT` and no useful file-I/O log entries
  - so the productive path remains debugger-assisted startup, not plain headless execution
- Current open-break snapshot:
  - `EV AX BX CX DX SI DI BP SP DS ES SS` confirms:
    - `AX=3D02`
    - `DX=A506`
    - `SI=FABE`
    - `DS=44A1`
  - dumping `DS:DX` (`44A1:A506`) yields `Setup.dat`
- First concrete startup-open fact:
  - corrected no-`/L` `ECGAME` opens `Setup.dat` first
- Current startup file-op sequence:
  - artifact:
    - `artifacts/ecgame-startup/startup-fileops.txt`
  - script:
    - `tools/capture_ecgame_startup_fileops.py`
  - confirmed debugger-assisted sequence:
    1. open `Setup.dat` with mode `0x02`
    2. read `0x20A` bytes from handle `5`
    3. close handle `5`
    4. open `C:\CHAIN.TXT` with mode `0x00`
    5. read `0x80` bytes from handle `5`
    6. close handle `5`
    7. exit with code `0x1C`
  - consistency:
    - `fixtures/ecutil-init/v1.5/SETUP.DAT` is `522` bytes (`0x20A`)
    - local generated `CHAIN.TXT` is `107` bytes, so `ECGAME` only reads the
      first `0x80` bytes before deciding to exit
- Next task:
  - investigate the `CHAIN.TXT` parser/decision path that leads to exit code
    `0x1C` after the `0x80`-byte prefix read
  - likely productive methods:
    - dump the `0x40BC` chain buffer after the read returns
    - or set a code breakpoint on the post-read caller path instead of
      re-breaking on raw DOS file I/O
- New result on that path:
  - artifact:
    - `artifacts/ecgame-startup/chain-buffer-summary.txt`
    - `artifacts/ecgame-startup/chain-buffer-prefix.bin`
  - script:
    - `tools/capture_ecgame_chain_buffer.py`
  - confirmed:
    - the first `107` bytes of the live `0x40BC` read buffer exactly match the
      generated `CHAIN.TXT`
    - only the bytes beyond EOF are stale scratch data
  - implication:
    - the remaining blocker is not low-level `CHAIN.TXT` I/O or CRLF handling
    - it is the semantic decision path after that successful prefix read
- Additional negative result:
  - artifact:
    - `artifacts/ecgame-startup/chain-variant-matrix.json`
  - script:
    - `tools/test_ecgame_chain_variants.py`
  - changing any of the following does **not** alter the early startup path:
    - `first_name = HANNIBAL`
    - `remote = Y`
    - padding `CHAIN.TXT` to exactly `128` bytes
  - all tested variants still:
    - read `SETUP.DAT`
    - open and read the first `0x80` bytes of `CHAIN.TXT`
    - close `CHAIN.TXT`
    - exit with code `0x1C`
  - implication:
    - stop spending time on obvious `CHAIN.TXT` shape/length tweaks
    - move to post-read semantic tracing instead
- New positive result on dropfile routing:
  - artifact:
    - `artifacts/ecgame-startup/dropfile-probe.json`
  - script:
    - `tools/test_ecgame_dropfile_probe.py`
  - confirmed:
    - if both files are present, `ECGAME` chooses `C:\CHAIN.TXT`
    - if `CHAIN.TXT` is absent, it falls back to `C:\DOOR.SYS`
  - `DOOR.SYS` fallback is not identical to the `CHAIN.TXT` path:
    - `CHAIN.TXT` path:
      - `3F00`
      - `3E00`
      - `3D00`
      - `3F01`
      - `3E01`
      - `4C00`
    - `DOOR.SYS` fallback:
      - `3F00`
      - `3E00`
      - `3D00`
      - `3FFF`
      - `3F30`
      - `3E01`
      - `4C00`
    - both still exit `0x1C`
  - implication:
    - there are at least two distinct startup parser paths available locally
    - the `DOOR.SYS` path is now the highest-value local harness lead
- New `DOOR.SYS` read-level result:
  - artifact:
    - `artifacts/ecgame-startup/door-buffer-summary.txt`
    - `artifacts/ecgame-startup/door-buffer-first.bin`
    - `artifacts/ecgame-startup/door-buffer-second.bin`
  - script:
    - `tools/capture_ecgame_door_buffers.py`
  - confirmed:
    - `DOOR.SYS` is read in two completed chunks:
      - first `128` bytes
      - then remaining `122` bytes
    - both chunks land in the same `0x40BC` buffer
    - bytes beyond the second chunk are stale scratch data
  - implication:
    - the `DOOR.SYS` fallback is not failing at the raw read level either
    - next work should target the post-read semantic decision path, not more
      dropfile formatting or chunk-size guesses
- Updated next task:
  - stop iterating easy `CHAIN.TXT` edits
  - trace the semantic decision path after:
    - `CHAIN.TXT` read/close -> `4C00 1C`
    - or the distinct `DOOR.SYS` fallback read path -> `4C00 1C`
  - likely productive next pass:
    - capture the live `DOOR.SYS` read buffers the same way we already did for
      `CHAIN.TXT`
    - or set a post-read code breakpoint keyed from the `3F01` / `3FFF` /
      `3F30` return path
- New best local-harness lead:
  - the legacy `DOOR.SYS` shape from `tools/test_fossil_commission.py` gets
    materially farther into the fallback parser than the current shared
    `write_door_sys()` output
  - observed dynamic difference:
    - shared `DOOR.SYS`:
      - `3FFF`
      - `3F30`
      - `3E01`
      - `4C00`
    - legacy fossil-style `DOOR.SYS`:
      - `3FFF`
      - `3F05`
      - `3F06`
      - `3F07`
      - `3F08`
      - `3F09`
      - `3F0A`
      - ...
      - later `3F10`, `3FFF`, `3F1A`, then close/exit
  - implication:
    - the current shared `DOOR.SYS` writer is probably not faithful enough for
      deeper local `ECGAME` startup
    - next pass should start by preserving/probing the legacy fossil-style
      `DOOR.SYS` shape, not by iterating more on the current shared one
- New parser-structure clue on that legacy path:
  - artifact:
    - `artifacts/ecgame-startup/legacy-door-reads.json`
  - script:
    - `tools/capture_ecgame_legacy_door_reads.py`
  - confirmed:
    - the legacy path repeatedly hits `AH=3F` with the same:
      - `BX=5`
      - `CX=0x80`
      - `DX=0x40BC`
    - while the low byte in `AX` walks through:
      - `3F05`
      - `3F06`
      - `3F07`
      - ...
      - `3F10`
      - later `3FFF`
      - `3F1A`
  - implication:
    - this is a deeper iterative parser/validator loop, not just one or two
      simple file reads
    - next work should target the code/counter logic around that recurring
      legacy `3F` loop
- New semantic state result inside that loop:
  - artifact:
    - `artifacts/ecgame-startup/legacy-door-locals.json`
  - script:
    - `tools/capture_ecgame_legacy_door_locals.py`
  - confirmed:
    - during the stable `3F05..3F10` run, `BP = F6A4`
    - local word `SS:[BP+0x0C]` increments in lockstep with the loop:
      - `3F05` -> `0x0006`
      - `3F06` -> `0x0007`
      - ...
      - `3F10` -> `0x0011`
  - implication:
    - this is likely the parser-progress / field-index counter
    - next work should target how that `BP+0x0C` counter is compared and why
      the loop still eventually exits `0x1C`
- New fixed-limit result:
  - artifact:
    - `artifacts/ecgame-startup/legacy-door-tail-matrix.json`
  - script:
    - `tools/test_ecgame_legacy_door_tail_matrix.py`
  - confirmed:
    - adding extra trailing `90` lines changes the early low-byte `3Fnn`
      pattern and starting index
    - but the stable loop limit stays fixed at `17` in every tested case
  - implication:
    - the deeper local `DOOR.SYS` parser is validating a fixed field window,
      not an arbitrary-length tail
    - next work should focus on which early fields matter up through
      index/field `17`, not on the variable `90` tail
- New representative field-sensitivity result:
  - artifact:
    - `artifacts/ecgame-startup/legacy-door-field-subset.json`
  - script:
    - `tools/test_ecgame_legacy_door_field_subset.py`
  - representative mutations to line `1`, `2`, `6`, `10`, `13`, `16`, and
    `18` did **not** change:
    - the `3FFF -> 3F05..3F10 -> 3FFF -> 3F1A -> 3E01 -> 4C00` shape
    - the stable loop-local `6 -> 17` progression
    - the final exit `0x1C`
  - implication:
    - those representative transport/flag/name/numeric fields are not the main
      discriminator for the current local startup gate
    - next mutations should target the remaining untested early lines inside
      the fixed window, not the already-tested representatives or the `90` tail
- Focused flag-cluster result:
  - artifact:
    - `artifacts/ecgame-startup/legacy-door-flag-cluster.json`
  - script:
    - `tools/test_ecgame_legacy_door_flag_cluster.py`
  - changing line `7`, `8`, `9`, or `17` also did **not** change:
    - parser sequence shape
    - stable loop-local `6 -> 17`
    - exit `0x1C`
  - implication:
    - the dense `Y/Y/Y` flag run is now largely deprioritized too
    - next likely productive targets are:
      - lines `3`, `4`, `5`, `11`, `12`, `14`, `15`
      - or a direct code/counter trace on the `BP+0x0C` comparison path
- New phase-boundary result:
  - artifact:
    - `artifacts/ecgame-startup/legacy-door-transition.txt`
  - script:
    - `tools/summarize_ecgame_legacy_door_transition.py`
  - confirmed:
    - stable parser loop ends at `3F10` with:
      - `[BP+0x0A] = 0x0011`
      - `[BP+0x0C] = 0x0011`
    - then the next `3FFF` and later `3F1A` stops use different frame shapes
      and no longer carry that loop-limit pair
  - implication:
    - the most productive next work is now the handoff after `3F10`, not more
      broad pre-`3F10` sweeps
    - specifically: trace how the completed parser loop repacks state and
      decides to continue toward `0x1C`
- New live-object anchor:
  - artifact:
    - `artifacts/ecgame-startup/legacy-door-handoff.json`
  - script:
    - `tools/capture_ecgame_legacy_door_handoff_buffers.py`
  - confirmed:
    - `DI=403C` is stable across `3F10`, `3FFF`, and `3F1A`
    - `DS:403C` looks like a live Borland-style file/stream object with:
      - handle `0x0005`
      - magic-like word near `0xD7B1`
      - buffer size `0x0080`
      - buffer pointer `44A1:40BC`
      - pointers back into live code near `4294:060C` and `4294:06E8`
  - implication:
    - `4294:060C` / `4294:06E8` are now the best concrete post-`3F10` anchors
      for static/dynamic RE of the local startup gate
    - this is a better next target than more blind dropfile text mutation
- New post-handoff code-hit result:
  - artifact:
    - `artifacts/ecgame-startup/legacy-door-code-hits.json`
  - script:
    - `tools/capture_ecgame_legacy_code_hits.py`
  - confirmed:
    - after arming breakpoints from the first live `BPINT 21 3D` stop,
      `4294:` code breakpoints are usable
    - `4294:06FC` hits first on the post-`3F10` handoff path with the same
      `AX=3FFF`, `DX=40BC`, `DI=403C`, `BP=F6A8` handoff state
    - `4294:076D` later hits on a close/error path and carries inline frame
      text:
      - `ECGAME: found invalid data in file: C:\DOOR.SYS`
    - `4294:01A3` is the final EOF-report/termination path, carrying stack
      text:
      - `ECGAME: found an unexpected End Of File in File: C:\DOOR.SYS`
      - `AX=4C67`
  - implication:
    - startup RE is now "complete enough" for the current project phase
    - the blocker has been narrowed to a semantic parser rule inside the
      legacy `DOOR.SYS` validator, not low-level file I/O or launch plumbing
    - unless local `ECGAME` ANSI/session preservation becomes immediate
      priority, return focus to Rust gamestate/compliance work
- Remaining blocker if local startup is revisited later:
  - recover the exact parser comparison between post-loop handoff `06FC` and
    the invalid-data / EOF reporters at `076D` and `01A3`

## Latest Rust Milestones

- Preserved initialized/post-maint auxiliary baseline is now explicit in the
  shared model:
  - `BASES.DAT` empty
  - `IPBM.DAT` empty
  - guarding-fleet count `0`
  - `core-report`, `core-validate`, and `core-sync-counts` now expose
    `empty_auxiliary_state`
  - count-sync helpers intentionally normalize only the proven count words;
    they do not erase these baseline errors in synthetic non-baseline states

- Preserved initialized/post-maint planet economy baseline is now explicit in
  the shared model too:
  - homeworld seeds keep tax rate `12`
  - homeworld seeds keep factories payload `[0, 0, 0, 0, 72, 134]`
  - homeworld seeds keep stored goods `0`
  - unowned planets keep tax rate `0`
  - unowned planets keep factories all zero
  - unowned planets keep stored goods `0`
  - this closes the remaining obvious planet-side gap in the deterministic
    initialized/post-maint baseline

- The current 88-byte `PLAYER.DAT` parser model is still too weak/noisy for a
  comparably broad initialized/post-maint player baseline. Continue extending
  broader cross-file rules from `PLANETS.DAT`, `FLEETS.DAT`, `BASES.DAT`, and
  `IPBM.DAT` first unless new player-side RE lands.

- Preserved initialized/post-maint fleet mission baseline is now explicit too:
  - every fleet in the four-fleet empire blocks uses standing order `5`
    (`Guard/blockade world`)
  - every fleet in a block targets that empire's homeworld-seed coordinates
  - every fleet in a block keeps mission aux bytes `[1, 0]`
  - this closes the remaining obvious gap in the fleet-side initialized/post-
    maint baseline: we were already validating topology, payloads, and
    homeworld alignment, but not the exact mission-byte semantics

- Preserved initialized/post-maint control-file baseline is now explicit too:
  - `SETUP.DAT.version_tag = EC151`
  - `SETUP.DAT.option_prefix = [4, 3, 4, 3, 1, 1, 1, 1]`
  - snoop + remote timeout enabled
  - local timeout disabled
  - max-time-between-keys `10`
  - minimum granted time / purge-after-turns / autopilot-inactive-turns `0`
  - `CONQUEST.DAT.player_count = 4`
  - `CONQUEST.DAT.maintenance_schedule = [1, 1, 1, 1, 1, 1, 1]`
  - `CONQUEST.DAT.game_year` stays within the preserved initialized/post-maint
    pair `3000` / `3001`

- New shared repair path:
  - `CoreGameData::sync_current_known_baseline_controls_and_counts()`
  - CLI: `nc-cli core-sync-baseline [dir]`
  - this now repairs the deterministic current-known control/count fields:
    - player starbase counts
    - player-1 `IPBM` count
    - `SETUP.DAT` baseline control fields
    - `CONQUEST.DAT` player count + maintenance schedule
    - invalid `CONQUEST.DAT` years normalized to `3001`
  - intentionally does **not** attempt full baseline reconstruction of
    planets/fleets/player records; it only touches fields the current RE
    supports confidently

- New shared fleet repair path:
  - `CoreGameData::sync_current_known_initialized_fleet_baseline()`
  - CLI: `nc-cli core-sync-initialized-fleets [dir]`
  - this rebuilds the deterministic preserved four-fleet-per-player baseline
    from current homeworld-seed coordinates:
    - fleet block topology
    - slot loadout payloads
    - mission bytes `[5, x, y, 1, 0]`
    - current fleet locations
  - intentionally does **not** claim to reconstruct arbitrary live fleets;
    it only reconstructs the current-known initialized/post-maint fleet block

- New shared planet repair path:
  - `CoreGameData::sync_current_known_initialized_planet_payloads()`
  - CLI: `nc-cli core-sync-initialized-planets [dir]`
  - this repairs the deterministic payload fields for:
    - owned homeworld seeds
    - unowned planets
  - rewritten fields include:
    - status/name summary bytes
    - tax
    - stored goods
    - factories
    - build queue
    - stardock
    - population
    - developed value
    - likely-army count
    - ownership status
  - intentionally does **not** guess new homeworld ownership or arbitrary
    planet topology; it only normalizes the payload fields the current model
    already validates

- New composed baseline synchronizer:
  - `CoreGameData::sync_current_known_initialized_post_maint_baseline()`
  - CLI: `nc-cli core-sync-current-known-baseline [dir]`
  - this composes the three bounded repair surfaces now available:
    - empty auxiliary-state repair
    - control/count repair
    - initialized fleet baseline repair
    - initialized planet payload repair
  - practical meaning:
    - Rust can now take a directory with drift across those current-known
      surfaces and normalize it back into the validated initialized/post-maint
      baseline model in one command

- New baseline constructor command:
  - `nc-cli core-init-current-known-baseline [source_dir] <target_dir>`
  - copies a source tree, then applies the composed current-known baseline
    synchronizer before writing the target directory
  - defaults the source to `fixtures/ecmaint-post/v1.5`
  - practical meaning:
    - Rust now has a one-command way to materialize a validated baseline
      experiment directory, not just repair one already in place

- New current-known drift report:
  - `CoreGameData::current_known_baseline_diff_counts()`
  - CLI: `nc-cli core-diff-current-known-baseline [dir]`
  - reports per-file differing-byte counts between a directory and the
    Rust-generated current-known baseline normalization
  - this answers: "what would the current-known normalizer still change?"
  - practical meaning:
    - this exposes where the current-known model is still not byte-complete,
      which is useful for deciding the next RE targets instead of only knowing
      whether the structural validator passes

- New current-known drift offset report:
  - `CoreGameData::current_known_baseline_diff_offsets()`
  - CLI: `nc-cli core-diff-current-known-baseline-offsets [dir]`
  - reports exact differing byte offsets per core `.DAT` file versus the
    Rust-generated current-known baseline normalization
  - practical meaning:
    - this turns the remaining baseline drift into concrete byte-level RE
      targets, especially in `PLAYER.DAT`, `PLANETS.DAT`, and `FLEETS.DAT`
  - current-known post-maint hotspot run:
    - `PLAYER.DAT`: `[156, 157, 244, 245, 332, 333]`
    - `PLANETS.DAT`: concentrated clusters including `23..28`, `120..125`,
      `217..222`, `314..319`, and later repeated six-byte runs
    - `FLEETS.DAT`: now clean after promoting the missing initialized owner-byte
      and tuple-marker semantics into the shared baseline model
  - resolved `FLEETS.DAT` follow-up:
    - old repeated hotspot stride: `2,13,19,25`, `56,67,73,79`, continuing
      across the initialized 16-record block
    - missing semantics were:
      - `owner_empire` byte at offset `0x02`
      - tuple A `[128, 0, 0, 0, 0]`
      - tuple B `[128, 0, 0, 0, 0]`
      - tuple C `[129, 0, 0, 0, 0]`
    - `fixtures/ecmaint-post/v1.5` now has `FLEETS.DAT: differing_offsets=[]`
  - resolved `PLAYER.DAT` follow-up:
    - old hotspot words were `[156, 157, 244, 245, 332, 333]`
    - those came from rewriting non-player-1 starbase count words as if the
      current 88-byte player-record model were validated for every player
    - corrected boundary:
      - player 1 starbase count word is current-known
      - player 1 `IPBM` count word is current-known
      - player 2..5 corresponding words are currently preserved as raw /
        uninterpreted
    - `fixtures/ecmaint-post/v1.5` now has `PLAYER.DAT: differing_offsets=[]`
  - resolved `PLANETS.DAT` follow-up:
    - the old repeated six-byte clusters were the hidden tail bytes after the
      visible `"Unowned"` name prefix in non-homeworld records
    - corrected boundary:
      - `"Unowned"` prefix is current-known
      - trailing bytes in that 13-byte name/status window are preserved raw
        per-planet payload
    - `fixtures/ecmaint-post/v1.5` now has `PLANETS.DAT: differing_offsets=[]`
  - current state:
    - the current-known baseline normalizer is now byte-complete for the full
      core post-maint fixture set:
      - `PLAYER.DAT`
      - `PLANETS.DAT`
      - `FLEETS.DAT`
  - new canonical drift reports:
    - CLI: `nc-cli core-diff-canonical-current-known-baseline [dir]`
    - CLI: `nc-cli core-diff-canonical-current-known-baseline-offsets [dir]`
    - these compare a directory directly against the preserved canonical core
      post-maint oracle in `fixtures/ecmaint-post/v1.5`
    - this answers: "how far is this directory from the preserved oracle
      bytes?"
    - important distinction:
      - a directory can be a valid fixed point of the Rust normalizer without
        being canonical
      - `original/v1.5` normalized through the current-known synchronizer still
        differs from the canonical baseline in `PLAYER.DAT`, `PLANETS.DAT`,
        `FLEETS.DAT`, and `CONQUEST.DAT`
  - new exact baseline oracle:
    - CLI: `nc-cli core-validate-current-known-baseline [dir]`
    - this is the first byte-level pass/fail oracle for the canonical preserved
      post-maint core baseline, not just a structural validator or drift report
  - new exact baseline classification path:
    - `nc-cli match <dir>` now emits
      `MATCH current-known-post-maint-baseline-core` when the directory's core
      `.DAT` set is an exact byte match for the canonical preserved post-maint
      core baseline
  - important boundary:
    - normalizing `original/v1.5` directly now works because missing baseline
      auxiliary files are seeded, but that result is not yet the canonical
      preserved post-maint baseline
  - new exact canonical constructor/synchronizer:
    - CLI: `nc-cli core-sync-canonical-current-known-baseline [dir]`
    - CLI: `nc-cli core-init-canonical-current-known-baseline [source_dir] <target_dir>`
    - these overlay the exact preserved post-maint core `.DAT` set from
      `fixtures/ecmaint-post/v1.5`
    - practical meaning:
      - use the current-known synchronizer when the goal is bounded repair from
        validated shared rules
      - use the canonical synchronizer/constructor when the goal is to
        materialize the preserved oracle baseline exactly while keeping top-level
        programs and non-core files from another source tree
      - `BASES.DAT`
      - `IPBM.DAT`
      - `SETUP.DAT`
      - `CONQUEST.DAT`

## Archived 2026-10-11 (superseded by the Claude-to-Codex handoff)

## Completed This Session

**Typed game state (t039, 2026-10-10).** Game state is plain typed Rust
structs (`nc-data/src/model/`); SQLite stores them in typed columns; the
runtime reads and writes only the store; the classic `.DAT` codecs live in
`nc-compat/src/classic/`, guarded by `nc-data/tests/classic_boundary.rs`.
`.DAT` files appear only through `nc-cli db-export`/`db-import` and the
oracle tools. See the [plan](../notes/plans/7h5r6ne8-typed-game-state-in-rust-and-sqlite.md).

**Classic fidelity (t031-t064).** 2.5D movement, the per-tick encounter loop,
starbase movement, the orbital gate, and the classic home pickers match
ECMAINT; deviations are owner-approved classic bugs, listed in the specs.

**NC combat redesign.** The owner approved all seven combat decisions with
Claude and Codex's joint recommendations (ADRs in `docs/notes/decisions/`).
Decision 1 (the flat 25/50/100 table with carried hits) is in the engine;
the rest are tasks t065, t066, t071, t074, t078, t079, t080.

**Door and mail fixes.** Windows console input, output, arrow keys, and
flicker (t019); `?` types text in editors, Ctrl+G opens help; queued mail is
readable and cancellable and is delivered at maintenance (t067).

**Next:** the `*_raw` identifier cleanup, then the implementation waves in
[x8arlb3c](../notes/plans/x8arlb3c-implementation-waves-after-t039.md);
`zk queue` for the order.

## Current Goal

- Stabilize the localhost and BBS release surfaces for public beta use.
- Fix real playtest bugs and workflow rough edges quickly.
- Keep docs aligned with the local/BBS product story.
- Treat future Nostr work as a separate `nc-host` / `nc-helm` track.

## Biggest Blockers

- No known major engine/storage blocker remains.
- The main remaining risk is field bugs found by real BBS and localhost players.
- The hosted `nc-host` / `nc-helm` track now exists locally, but it is still a
  dev-only path and not the public shipped product story.
- The hosted client now stages and replays build, fleet, tax, diplomacy, and
  quick message drafts. Remaining gaps are narrower unsupported actions such as
  starbase moves and fleet renumbering.
- The BBS door renderer redraws every row each frame (no flash since t019)
  instead of using the retained-frame diffing already in local `nc-game`.

## Immediate Next Steps

1. Keep running real BBS and localhost playtests and tighten the rough edges.
2. Regenerate manuals/PDFs whenever the public command surface changes.
3. Keep runtime/storage code pointed at SQLite and away from raw offset-style
   regressions.
4. Keep the runtime DB scoped to localhost/BBS play; future hosted work should
   use its own schema rather than reviving retired hosted tables in `ncgame.db`.
5. Keep the localhost `nc-host` lab reproducible with the user-service install
   script and dev docs, but keep the public docs centered on local/BBS play.
6. Keep the shared local/hosted dashboard launch path stable while real users
   exercise the hosted dashboard.
7. Finish the remaining unsupported hosted dashboard actions or explicitly
   scope them out of the hosted flow.
8. If modal drag pacing still feels rough in `nc-helm`, evaluate a future
   GPU/vsynced native renderer path as follow-up work.
9. After `nc-helm` is stable on the shared ratatui render contract, replace
   the temporary `crossterm` event types with a small local input model.

## Claude handoff before takeover, 2026-10-11

**Heads.** `main` = `origin/main` at this commit; nothing uncommitted on
main. The last full suite ran on the t061 merge (`a9c2a710`): 3195 passed,
0 failed, 11 ignored. Commits after it change only the sysop PDF, todo.md and
this file.

**Live work.** Three Claude subagents were running at handoff. They stop
when Claude stands down; their work stays in the worktrees. The worktrees
are in Claude's session scratchpad under `/tmp` (`git worktree list` shows
the paths), which is on **tmpfs: a reboot loses uncommitted work**. Each
brief is at `briefs/<task>.md` beside the worktrees.

| Task | Branch, worktree | State at handoff |
| --- | --- | --- |
| t099 | `t099`, `wt-t099` (from `7348955f`) | ~20 files uncommitted, no commits |
| t090 | `t090`, `wt-t090` | `0c1c8f6f` (price campaign driver) + 3 uncommitted; also detached worktrees `variants/{bb30,bb60,etac15,etac25}` |
| t101 | `t101`, `wt-t101` (from `9a9ca001`) | 1 file uncommitted |

To take one over: read its brief, review the diff, finish, commit signed,
merge. After t090, remove the four variant worktrees and their cargo pools
(`~/.cache/cargo-slots/nostrian-conquest@{bb30,bb60,etac15,etac25}`).

**Merge routine.** For each branch: every commit `%G? = G`; `git merge
--no-ff -S`; full suite from `rust/` with `nice -n 10 cargo-slot
nostrian-conquest cargo test --workspace --no-fail-fast`; if a `.typ`
changed, `python3 scripts/refresh_manual_revision_date.py --doc
player|sysop` and commit the PDF; close with `zk tasks -p ls id:tNNN` then
`zk tasks do N` (re-read the line number after each close), `zk index`;
`zk list docs/notes --broken-links` reports 22 pre-existing notes; push;
`git worktree remove` and delete `~/.cache/cargo-slots/nostrian-conquest@wt-tNNN`.
`zk tasks add` prepends the date itself. The highest task ID is t101.

**Signing.** Every commit is GPG-signed by Mason Green's key; the agent
cache expires (default 10 min, max 2 h), and a cold cache makes `git
commit -S` time out at pinentry. The GitHub SSH key sits in the same
gpg-agent. When either is cold, ask the owner to unlock it in a terminal
(`echo x | gpg --clearsign >/dev/null`, `ssh -T git@github.com`). Never
commit unsigned. To sign an agent's unsigned commits, cherry-pick them with
`-S` onto the branch's base and compare trees; a rebase would also rewrite
main commits the branch merged.

**Queue after the three tasks.** t024: the Windows nc-sysop packages
(x64, x86, Win7+ x86, as beta.2 was), built on `ssh bean` (PowerShell;
sync by git bundle over scp, since `git pull` hangs there), signed and
uploaded from crunch; ask the owner before uploading. Then the beta.3
re-release: rebuild and replace its assets and deploy the website, keeping
the version v1.0.0-beta.3.

**Owner decisions.** None block the queue. For the owner's review:
t061's choices (a new hosted game arms its timer instead of running at
once; a host that was down runs one year on return; hosted league games
put absent players on autopilot after three turns, as local games do), and
t090's price recommendation when it lands (prices change only with the
owner's approval). Rulings for t099 are in
[q0a0mhm2](../notes/plans/q0a0mhm2-classic-follow-ups-from-the-october-2026-oracle-work.md#t099).

**Other agents.** The owner's own agent pushes signed commits to nc-bridge,
nc-herald and nc-dropfile; fetch before each merge.
