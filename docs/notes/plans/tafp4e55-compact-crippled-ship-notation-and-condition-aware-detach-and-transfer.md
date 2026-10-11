---
id: tafp4e55
date: 2026-10-11
kind: plan
status: planned
tags: [plan, door, combat]
---
# Consistent ASCII ship notation and condition-aware detach and transfer

The owner approved consistent ASCII ship notation for healthy and damaged compositions, plus an optional crippled-only quantity in the door's existing Detach and Transfer commands. Keep automatic RT2 recovery. Let ordinary operations stay simple, while allowing a player to send damaged ships away without a new menu or individual-hull management.

## t093

Implement this approved interface follow-up to [Recovery Turns](../decisions/p01cg7ll-rt-controls-automatic-ship-recovery.md#approved-door-interface-follow-up-2026-10-11). t066 is implemented; this task changes its display and default selection behavior. The owner accepted these rules in the conversation of2026-10-11 and requested documentation and a todo plan. This plan does not claim they are already implemented.

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
