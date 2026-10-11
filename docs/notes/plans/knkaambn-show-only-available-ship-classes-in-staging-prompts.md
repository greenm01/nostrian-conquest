---
id: knkaambn
date: 2026-10-11
kind: plan
tags: [plan, door, helm]
---
# Show only available ship classes in staging prompts

## t106

The owner approved showing only classes available for the current action,
consistently across menus. The door's detach and transfer prompts and the
helm's transfer prompt currently list every class. Filter each using its
existing availability rule; update choices as ships are staged or cancelled.
Keep composition order and separate C/X actions from class choices.
Do not change transfer/detach eligibility or inventory rules.

See [prompt standard](../../dev/ec-game-prompt-standard.md).

Validation: all 1,231 nc-game and nc-helm tests passed. Focused checks cover
classes missing from the donor, loaded versus empty transports, choices
after staging, no further available ships, and cancellation restoring the
choices. Existing quantity and ship-selection rules remain unchanged.
