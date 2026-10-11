---
id: wn3swwhh
date: 2026-10-11
kind: plan
tags: [plan, door]
---
# Keep build queue deletion explicit and its table stable

## t102

The owner requested explicit `D` deletion in the build queue. Enter must not
start deletion. It still submits a quantity once the player presses `D`.
Use `COMMAND <- Delete <A> or 1-N <Q> ->` and keep the table width unchanged
while editing the quantity or confirming deletion. Update the popup help
and [prompt standard](../../dev/ec-game-prompt-standard.md).

## Validation

Check key handling, the compact prompt and unchanged table borders through
quantity and confirmation states at local and BBS screen heights. Run the
nc-game tests before staging the updated door for playtesting.

## Result

Only `D`/`d` starts deletion; Enter and the Delete key do nothing on the
queue list. Enter still submits an entered quantity. The popup help and
prompt standard match. Table sizing uses its title and cells, independent
of quantity or confirmation footer text.

Validation: all 670 nc-game tests passed. After tightening the rendering
fixture to a short planet name and formatting, the 13 planet_build tests
passed again. Checks cover 24-row door and local rendering, fixed borders,
compact quantity text, D-only entry and Enter quantity submission. The
release build passed, and all three sandbox socket-door launch checks
passed on disposable copies. No gameplay or queue accounting changed.
