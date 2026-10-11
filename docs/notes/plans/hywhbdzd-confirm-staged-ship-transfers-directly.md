---
id: hywhbdzd
date: 2026-10-11
kind: plan
tags: [plan, door]
---
# Confirm staged ship transfers directly

### t109

The owner approved matching the new Detach flow in the BBS Transfer screen.
T replaces C as the finish-staging command. An empty Enter also finishes
staging. When only one hull remains on the source, confirmation opens
automatically. Enter or Y accepts, N returns to editing with the selection
intact, and Q cancels without moving ships. X clears staging while editing.

Confirmation hides the action row and collapses its space. The screen keeps
both fleet summaries and shows the staged ships, source remainder, and
destination after transfer. Existing source-retention, selection, speed and
ROE rules remain unchanged. The helm popup is outside this door-only change.

As with Detach, the quantity key handler now accepts `!` so actual keyboard
input reaches the existing crippled-only parser.

## Validation

All 680 nc-game tests pass, and the player-manual PDF compiles. Key-event regressions
cover loaded crippled transports, automatic confirmation, compact layout,
Enter/Y acceptance, N editing, X clearing, Q cancellation and empty staging.
The release build passes. Diff whitespace checks pass (excluding the PDF);
the notebook link check retains the same 24 pre-existing flagged notes.

## Connections

- [Detach confirmation](4f5fgzyu-confirm-staged-ship-detachments-directly.md)
- [Prompt standard](../../dev/ec-game-prompt-standard.md)
- [Damage notation and staging](tafp4e55-compact-crippled-ship-notation-and-condition-aware-detach-and-transfer.md)
