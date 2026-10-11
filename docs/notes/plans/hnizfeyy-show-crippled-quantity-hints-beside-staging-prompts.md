---
id: hnizfeyy
date: 2026-10-11
kind: plan
tags: [plan, door]
---
# Show crippled quantity hints beside staging prompts

### t110

The owner approved a short `1! = 1 crippled` hint above the quantity prompt
in Detach and Transfer. Show it only when the selected class has crippled
hulls still available after staging. Use the shared selection calculation;
do not infer availability from the whole fleet's damage flag.

The hint occupies the existing blank row between the action line and the
quantity prompt. It adds no rows and changes no prompt or preview positions.
Class selection, confirmation and speed entry have no hint. Confirmation
keeps its compact layout, with one blank line on either side of the prompt.
The player manual and prompt standard explain the placement.

## Validation

All 682 nc-game tests pass. The release build and player-manual PDF
compilation pass. Rendering
regressions cover both screens in 80 by 25, healthy classes in damaged fleets,
depleted crippled cohorts, and the compact confirmation layout. Diff checks
pass (excluding the PDF); the notebook retains its 24 existing flagged notes.

## Connections

- [Detach confirmation](4f5fgzyu-confirm-staged-ship-detachments-directly.md)
- [Transfer confirmation](hywhbdzd-confirm-staged-ship-transfers-directly.md)
- [Prompt standard](../../dev/ec-game-prompt-standard.md)
