---
id: 9ir2qgc6
date: 2026-10-11
kind: plan
tags: [plan, door, helm]
---
# Use plain plus for table composition overflow

### t111

The owner approved plain `+` for omitted ships and asked for a table audit.
Keep `!n` only on complete class tokens that show crippled counts. Review
shows the full composition, including damage in omitted classes.

## Audit and change

The only damage-sensitive overflow formatter is `nc-data::fit_notation`.
Its runtime callers are the door fleet-table helper (fleet list and group
selection table) and the helm fleet list. Both now use the same plain `+`,
including a one-character column. A zero-width cell is empty. Whole class
tokens remain intact, and compositions that fit are unchanged.

Removed the formatter's obsolete damage argument. The door help, player
manual, notation note and regression expectations use the new rule. Searched
Rust table renderers and text documentation for the old marker; no other
`+!` overflow implementation remains. Unrelated name truncation and map
scrolling are unchanged.

## Validation

All 1,888 tests pass across nc-data, nc-game and nc-helm. Existing
regressions cover table geometry, visible and omitted damaged classes, tiny
widths and complete tokens. The player-manual PDF and release build pass.
Whitespace checks pass (excluding the PDF). The notebook link check retains
the same 24 pre-existing flagged notes.

## Connections

- [Ship notation](tafp4e55-compact-crippled-ship-notation-and-condition-aware-detach-and-transfer.md)
