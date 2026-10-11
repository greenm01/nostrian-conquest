---
id: tafp4e55
date: 2026-10-11
kind: plan
status: implemented
tags: [plan, door, combat]
---
# Consistent ASCII ship notation and condition-aware detach and transfer

The owner approved consistent ASCII ship notation for healthy and damaged compositions, plus an optional crippled-only quantity in the door's existing Detach and Transfer commands. Keep automatic RT2 recovery. Let ordinary operations stay simple, while allowing a player to send damaged ships away without a new menu or individual-hull management.

## t093

Implement this approved interface follow-up to [Recovery Turns](../decisions/p01cg7ll-rt-controls-automatic-ship-recovery.md#approved-door-interface-follow-up-2026-10-11). t066 is implemented; this task changes its display and default selection behavior. The owner accepted these rules in the conversation of2026-10-11 and requested documentation and a todo plan. The Implementation section at the end records what landed.

## Display contract

The authoritative formatting contract is [ASCII ship notation](../../dev/ship-notation.md), approved in the owner's follow-up. Apply it consistently to all compact ship compositions, not just damaged fleets: `DD8`, `CA12!3`, `SC1`, `TT5`, `TT*4!1`, `ETAC2`, `SB1!1`. This includes existing lists, details, report compositions and operation previews/summaries. Retain full names in prose and existing labels/numeric cells in dedicated columns. Reuse formatting rather than introducing another screen-specific dialect.

Document that grammar and its legends in the player manual and existing door help as part of rollout. Do not present implementation as complete just because the design reference exists.

- `CA12!3` means12 cruisers total, including3 crippled. `CA12` means all12 are healthy. Never append `!0`.
- A mixed composition can read `DD8 CA12!3 BB2!1`. Class totals always include crippled ships.
- A lone `!` beside a fleet identifies a fleet containing any crippled ships. Keep it visible even when the composition must be shortened; do not silently make a damaged fleet look healthy because the optional suffix did not fit.
- Use the notation in existing fleet details, ship/fleet report compositions and detach/transfer confirmations. Where the list has room for composition, use the same format there.
- Keep `TT*` for loaded transports. `!` identifies damage; `*` retains its cargo meaning. Respect the existing loaded/empty transport categories and preserve armies aboard.
- Provide a short legend in existing help/detail space: `!n = crippled ships included in total`. The fleet-level marker means at least one damaged ship, not that every ship is damaged.
- Preserve BBS table widths, screen dimensions, prompt rows and hotkeys. No RT column, per-hull list, recovery countdown or new screen. Adapt existing composition text and available marker space rather than widening the table.

## Detach and Transfer input

At the existing quantity prompt for a selected ship class:

| Input | Meaning |
| --- | --- |
| `3` | Move3 ships, healthy first, then crippled if needed. |
| `3!` | Move exactly3 crippled ships; do not substitute healthy hulls. |

Keep current handling of ordinary numeric limits, zero and cancellation. Extend it only with a trailing `!` on a valid quantity. An optional short hint is `Amount (3! = crippled only)`.

Reject `3!` if fewer than3 crippled hulls of the requested class/category are available. Explain the available count, leave state unchanged and allow correction. Do not silently cap the request or fill it with healthy ships. Malformed input must also leave state unchanged.

Whenever crippled hulls are selected, take those with the longest recovery remaining first, with newly crippled hulls first if that cohort is present. Move their existing recovery state intact. Selecting, detaching, transferring or merging must never heal a ship or reset its recovery time.

This **supersedes the implemented crippled-first default** for ordinary quantities documented under t066. Plain numbers now favor a ready outgoing force. `n!` deliberately gathers damaged hulls. The longest-remaining-first ordering within damaged hulls is unchanged. Apply the same contract to Detach and Transfer; do not make players learn different selection rules for them.

## Preview and confirmation

Show the actual selected composition in the existing confirmation/summary before committing the operation:

- With12 CA including3 crippled, plain `3` selects `CA3`.
- On that fleet, `3!` selects `CA3!3`.
- With4 CA including2 crippled, plain `3` selects `CA3!1`.
- With only2 crippled CA, `3!` is rejected.

If a confirmation combines classes, use the same compact notation for each. Selection must be computed by the same rule used by the mutation so the preview cannot promise different hulls. Keep this within the existing flow; do not add a damage-selection submenu or extra confirmation stage.

## Implementation boundaries

Use shared parsing/selection/validation where Detach and Transfer overlap. Keep the condition choice attached to the operation rather than introducing a permanent per-fleet preference. Preserve existing co-location, ownership, destination, fleet-size, mission and ROE rules.

Audit loaded and empty transports carefully: t066 stores their damage in a shared transport cohort. The requested cargo category must remain unchanged, and no path may duplicate damaged counts or move more crippled transports than exist. Account for this representation in shared selection and preview; do not silently change loaded transports into empty ones or discard armies to satisfy `n!`.

The door is the requested UI scope. Check shared mutation callers in the CLI and helm so changing the normal selection default does not leave contradictory behavior or documentation. Other clients do not gain a new menu or input syntax merely because the door does. Document any affected shared default explicitly.

Update the player manual's recovery, fleet composition, Detach and Transfer explanations when implementing. The current manual describes shipped behavior; this plan and the ADR follow-up record the approved pending change. Keep the RT value, recovery timing, starbase assistance and combat rules unchanged.

## Acceptance and validation

1. All compact composition surfaces use the documented code/count grammar, including healthy fleets. Rendering shows healthy, partly crippled and wholly crippled classes correctly, without counting damaged hulls twice. Fleet markers and legends fit the existing door layouts, including maximum supported counts and mixed-class compositions. `TT*` stays distinguishable.
2. Both commands accept plain quantities and `n!`; invalid or insufficient requests make no mutation. Existing cancellation behavior remains intact.
3. Preview and mutation agree for healthy-first fallback and crippled-only selection. Longest remaining recovery moves first among crippled hulls.
4. Counts, recovery cohorts and transport cargo are conserved across detach, transfer, merge and save/reload. Recovery after a move occurs on the same schedule it would otherwise have, subject to the already-approved location-dependent base assistance.
5. Existing mission/ROE behavior and healthy-only workflows remain correct. The manual and existing help document every ship code, totals including crippled counts, loaded `TT*`, fleet `!`, output `CA3!1` versus input `3!`, and matching examples. No hidden enemy damage is exposed. Dedicated numeric columns and prose retain appropriate labels.
6. Run focused shared-mutation and door-rendering tests, then the affected Rust crate suites through `cargo-slot nostrian-conquest cargo test ...` from `rust/`. Record counts and review representative fixed-width door screens. No tests or Rust builds are required merely to record this plan.

## Connections

- [ASCII ship notation](../../dev/ship-notation.md): common grammar, code table, examples and usage boundaries.

- [Recovery ADR](../decisions/p01cg7ll-rt-controls-automatic-ship-recovery.md): damage and recovery state; this follow-up changes only its display and default detach/transfer selection.
- [Implementation waves](x8arlb3c-implementation-waves-after-t039.md): this is follow-up work after the completed t066, coordinated with other fleet mutations.
- [Player manual](../../manuals/nc_player_manual.typ): update the existing player explanations when the behavior lands.

## Implementation (t093)

Done in t093. The sections above are the contract; this section records what
the code settled.

**One formatter.** `nc-data/src/ship_notation.rs` owns the grammar:
`ship_token`, `join_tokens`, `fleet_notation` (the fleet screens' class order,
`SC BB CA DD TT* TT ETAC`), `starbase_token`, `fit_notation`, and the
`NOTATION_LEGEND` text. `fit_notation` cuts between tokens and ends a cut
damaged fleet in ` +!`, so the lone `!` survives however far the cell is
cut. `ship_composition_table_summary` is now `fleet_notation` with the empty
fleet assertion; `ship_composition_summary` (`SC=1 ... AR=2`) and
`damaged_ships_table_summary` are gone. The grammar is used by the door's fleet
list and group screens, the review, ETA and order screens' ship rows, Detach and
Transfer (ship rows, staged, remaining and destination lines), the mass
commission notice, the helm's fleet list cell, transfer summary and commission
notice, the hosted state summary, the CLI's inspect and bundle output, and the
reports' force compositions, stardock scans and starbase counts. The review
screen's `Damaged:` row and the helm popup's `Damaged` line are replaced by
the notation itself; the review keeps the legend in that blank row.

**Transports.** A fleet keeps one crippled counter for loaded and empty
transports together, so the state cannot say which transports are hurt.
`crippled_by_category` splits the counter loaded first, the order
`remove_ships` already uses for transports lost in battle, and every
screen, selection and preview reads that one function. The split never
changes a ship count or an army: `TT*4!1` only says the one crippled transport
is counted among the four loaded. `TT 1!` is refused when every crippled
transport is counted loaded. The alternative, a separate counter per cargo
state, would have meant new rules in the engine's damage write-back, loading,
unloading and landing; the owner can ask for it if a loaded-first reading
ever plays wrong.

**Selection.** `FleetDetachSelection` gains `required_crippled` (per
category). `resolve_selection` picks the hulls: required crippled first, the
rest healthy first and then crippled, cohorts in `CrippledHulls::take` order.
Detach, Transfer and every preview call it, and an `n!` beyond the crippled
ships is refused by `GameStateMutationError::FleetDetachCrippledExceedsAvailable`
before anything changes. Plain and crippled quantities add up in any order to
the same ships. This also changes the shared default: the CLI, the KDL
turn files, the helm and the fuzzer have no `n!` syntax, so their plain
quantities now take healthy ships first as well.

**Door input.** The quantity prompt accepts digits and one trailing `!`; a
class with crippled ships left to stage says `n! = crippled only`. A refused
`n!` says `Enter a crippled quantity from 1 to N.` or `No crippled CA left to
stage.` and leaves the prompt open. Class codes show `ETAC`; `ET` still
works. Help for the fleet list, Detach and Transfer carries the legend and
the `3!` rule.

**Reports.** Force compositions and stardock scans use the grammar. A loss
record counts transports without their cargo, so casualty lists write
lost transports in words (`BB2 CA1 and 3 transports`) rather than give them a
cargo they may not have had. Reports show no crippled counts: the events
carry only totals, and an enemy's damage is not known to the reader.

**Left alone.** The helm's planet stardock and build-queue summaries and the
sector panel sample (`5BB 10CA`, from `planet_summary`) keep their form, and
so do the CLI's key=value feedback lines (`BB=2 CA=1`). Showing friendly
crippled counts in battle reports needs the combat events to carry them.


## Owner decisions after t093 (2026-10-11)

- **Transport damage is real state:** loaded and empty transports get
  separate crippled counters in the engine (damage write-back, loading,
  unloading, landing, and storage), replacing t093's loaded-first convention
  over one shared counter. Task t096.
- **Finish the rollout:** nc-helm's planet stardock and build-queue
  summaries and the CLI's `BB=2` feedback lines move to the notation, with
  one class order everywhere (SC BB CA DD TT* TT ETAC SB, the fleet screens'
  order). Output that scripts or tests parse changes only together with
  them. Task t097.

## Result (t097)

Done. Every compact ship composition now goes through `Composition`
(`nc-data/src/ship_notation.rs`), which writes `SC BB CA DD TT* TT ETAC SB`
and nothing else. A producer fills a `Composition` and calls `notation()`.

- **Planet lines.** The stardock and build-queue summaries in nc-helm (the
  planet panel and popup, the sector panel) and in the door's planet info
  screen read `BB5 CA10 DD4 TT6 ETAC3 SB2`. The door's `5-CA, 2-BB` form and
  the helm's `5BB 10CA` form are gone: `CompactUnitSummaryStyle` is now
  `Notation` or `Words`. The two non-ship build items follow the starbases
  as `AR` and `GB`. Docked and new transports are empty, so `TT`. The
  CLI's `inspect` planet summary (`build=`) uses the same text.
- **CLI.** `fleet-ships` and `fleet-detach` print the resulting fleet's
  notation (`Fleet 1 ships set: SC12 BB8 CA10 DD14 TT*4 TT5 ETAC2`), which
  also replaces `loaded_armies=`, `fullTT=` and `emptyTT=`. The scenario
  commands (`bombard`, `econ`, `invade`, `fleet-battle` and their batch
  manifests and `validate` output) print their ships the same way.
  Nothing in `tools/` or `scripts/` reads these lines; the CLI tests that
  did (`bombard`, `test_setup`) changed with them.
- **One order.** The commission notice (`CA2 DD4`, was `DD CA BB`), the
  reports' force compositions and loss lists (were `BB CA DD SC`) and the
  reports' stardock scan (was slot order) follow it. `starbase_token` is
  replaced by `Composition::with_starbases`.
- **Kept.** `Words` (the stored scouting-report stardock text, `2 cruisers`,
  which the helm also shows for foreign planets) keeps its prose and its
  slot order. The door's commission table keeps its `DD CA BB SC TT ET SB`
  column headings, and the harness bundle readme its labelled lists. The
  KDL turn files and the fuzzer's repro output (`bb=1 ca=0`) are input
  formats, not compositions.
- **Tests.** `nc-cli/tests/notation_order.rs` checks the fleet, commission,
  detach, planet and CLI producers; three tests in
  `nc-engine/src/maint/results/format.rs` check the report producers. The
  affected expectations moved to the new text. Workspace, with main at 48ab3e7a merged: 3133 passed, 0
  failed, 11 ignored.

## Result (t096)

Loaded and empty transports keep separate crippled counters:
`CrippledShips::loaded_transports` and `empty_transports`, the classes
`HullClass::LoadedTransport` and `EmptyTransport`. t093's loaded-first
reading of one shared counter is gone; `crippled_by_category` reads the
counters, so `TT*4!1 TT5!2` is the record's state and `TT 1!` is valid
whenever an empty transport is crippled.

**Combat.** A side's transports are still shared loaded first, but each
fleet's loaded and empty transports are separate holders in
`damage::apply_class_change`, so a hull crippled or finished is written to
the counter of the cargo state it was struck in; nothing is counted twice,
and no hull moves between the two in combat. `write_back_strike` shares a
lone fleet's change the same way.

**Loading and unloading.** `load_transports` takes healthy empty
transports first and crippled ones only when none is left, the nearest to
recovery first; `unload_transports`, which landing also uses, empties the
crippled loaded transports first, the most turns left first, then the
healthy ones. A cohort moves to the other counter with its hull. The
owner approved both orders on 2026-10-11 (armies ride the soundest hulls,
over crippled-first or a player's choice at the prompt); each lives in one
helper in `nc-data/src/fleet_ships.rs`, and the
[recovery ADR](../decisions/p01cg7ll-rt-controls-automatic-ship-recovery.md#transports-by-cargo-state-t096)
records it.

**Everything else.** Recovery, starbase assistance, merges, detach and
transfer, and the store treat the two classes as any other.
`RUNTIME_SCHEMA_VERSION` is 17 (class codes 4 loaded, 5 empty, 6 ETAC), with
no migration; the state hash feeds a class only when it has crippled
hulls, so the pinned fixture seeds stand. A classic import starts with no
damage, and a classic export writes whole hulls with t066's notice.

## Owner decisions after t097 (2026-10-11)

- **A scouted planet's `Docked` line uses the notation.** The planet
  intelligence row stores the stardock in notation (`BB1 CA2 TT3`), like the
  scout report and the owner's own planets. Rows written before the change
  keep their prose until the planet is scouted again; the text is NC's own
  store, not a classic file, so nothing migrates. The scout mission report
  prints the same stored line, so it now reads "Scanning the planet's
  stardock, we detected DD2." where classic ECMAINT writes the ships in
  words; the owner confirmed notation for both after Claude corrected an
  earlier claim that the report already used it. Task t098.
- **The door's commission picker follows the class order.** Its columns read
  `SC BB CA DD TT ET SB`, same widths and layout. Task t098.

## Result (t098)

Done. `intel.rs` writes the `Docked` summary with
`CompactUnitSummaryStyle::Notation` in both places (a scout's report and the
refresh of a full-tier row), and `planet_commission.rs` reorders the picker's
columns and cells. `docs/dev/ship-notation.md` says both. Four
expectations moved with the text: the picker render test (`│02│04│`), the
nc-cli scout report test (`we detected DD2.`), the nc-sysop runtime intel
test and the engine's stardock phase-order test (`DD2`).


## t103

The owner requested a shorter fleet-review legend with a blank line between
composition and legend. Use `! = crippled; * = loaded`. Keep the full grammar
in the manual and [notation reference](../../dev/ship-notation.md). On damaged
fleet reviews, move the dismiss prompt below the separated legend.

Use the same shared legend in fleet list, detach and transfer popup help.
The full notation examples remain in the manual; menu legends stay compact.

Validation: 1,319 nc-data and nc-game tests passed, including review spacing
and the shared legend on fleet list, detach and transfer help. The player
manual PDF rebuilt successfully. The legend is a shared nc-data constant,
so these screens cannot drift to different wording.
