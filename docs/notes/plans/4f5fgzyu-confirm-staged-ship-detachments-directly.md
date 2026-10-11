---
id: 4f5fgzyu
date: 2026-10-11
kind: plan
tags: [plan, door]
---
# Confirm staged ship detachments directly

### t108

The owner approved replacing the final Commission step with a detach
confirmation. When staging leaves one hull on the donor, confirmation opens
automatically. D or an empty Enter finishes staging early. Enter or Y confirms;
N returns to editing without discarding the selection; Q cancels. X clears
staging while editing, and its label now says Clear. The confirmation hides
the staging action line and collapses its reserved space: the confirmation
sits one blank line below Ships, and the staged summary moves up with it.
Existing donor-speed checks still run before commit.

The implementation adds a Confirming state to the door detach flow. It does
not change ship selection, recovery, or fleet orders. Transfer keeps its own
workflow. Help, the player manual, and the prompt standard describe the flow.

The key-event regression also found that the detach quantity handler rejected
`!` before it reached the existing parser. The handler now accepts the same
quantity characters as that parser.

## Validation

All 677 nc-game tests pass. New key-event regressions cover BB1!1 DD1!1,
automatic confirmation, Enter and Y acceptance, N returning to editing, X
clearing the selection, and Q cancelling without changing ships. Existing
speed-adjustment and selection tests still pass with explicit confirmation.
The release build and player-manual PDF compilation pass. Diff whitespace
checks pass (excluding the PDF); the notebook check retains its 24 existing
flagged notes, with no new broken note links.

## Connections

- [Prompt standard](../../dev/ec-game-prompt-standard.md)
- [Damage notation and staging](tafp4e55-compact-crippled-ship-notation-and-condition-aware-detach-and-transfer.md)
- [Short-prompt audit](om9r4ejo-keep-command-prompts-short-and-input-focused.md)
