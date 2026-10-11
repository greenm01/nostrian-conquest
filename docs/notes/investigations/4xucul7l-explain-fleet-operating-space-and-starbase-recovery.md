---
id: 4xucul7l
date: 2026-10-11
kind: investigation
status: resolved
tags: [investigation, docs, movement]
---
# Explain fleet operating space and starbase recovery

## Scope and decision

t115 answers the owner's questions about Hold, world/system positioning, and
starbase-assisted recovery. The owner approved a separate Where Missions
Operate table, rather than another column in the existing orders tables.
Updated both the quick-start and full player manual. No runtime or mission
picker change is included.

## Code findings

- A sector is the map coordinate. Deep space and the system are separate
  layers within that sector. Full orbit is the end of the descent, not a
  third encounter layer. The system layer starts halfway down.
- Hold preserves position. Move, Patrol, Scout Sector and Rendezvous target
  deep space. See [movement stepper](../../../rust/nc-engine/src/maint/movement/stepper.rs)
  and [movement/arrival rules](../../../rust/nc-engine/src/navigation/motion.rs).
- Guard Starbase dispatch uses the base's actual layer, then puts the escort
  at the base's depth. It works with deep-space and system bases. See
  [guard dispatch](../../../rust/nc-engine/src/maint/movement/guard.rs) and
  [starbase movement](../../../rust/nc-engine/src/maint/movement/starbase.rs).
- Join is not simply a same-layer rendezvous. The player order targets the
  host's sector at system depth; the merge tests the joiner's settled target
  depth and the host's sector, then keeps the host's position and orders.
  See [order writer](../../../rust/nc-data/src/directory/mutators_fleet.rs),
  [retargeting](../../../rust/nc-engine/src/maint/retarget.rs), and
  [mission merging](../../../rust/nc-engine/src/maint/merging/mission.rs).
  Its table row is Special, with this qualification. No rule changed.
- Recovery assistance requires old damage, an owned world, and a qualifying
  base and fleet stationary throughout the tick year. The base cannot assist
  itself. Same coordinates alone do not qualify. See
  [recovery snapshot](../../../rust/nc-engine/src/maint/ship_recovery.rs) and
  [regressions](../../../rust/nc-engine/src/maint/ship_recovery_tests.rs).

## Documentation and validation

The [shared table](../../manuals/includes/mission_operating_space.typ) covers
all 16 missions. Both guides include it, keeping labels and exceptions aligned.
Existing mission-order tables retain their columns. The full manual corrects
its older Guard Starbase arrival wording and explains the Join exception.

The recovery example contrasts Move to Sector followed by Hold with Guard
Starbase directed at an owned base inside an owned system. It explains the
full resting turn, base ownership, and why an unarmed fleet cannot use Guard
Starbase. Seek Home is identified as nearest-world routing, not a choice of
repair destination. Hold is optional after reaching the base.

Both PDFs compile without warnings. The quick-start has four gameplay pages
and a license page. Its new movement page was rendered and visually checked.
The full manual's movement explanation, separate table, and recovery example
were rendered for pagination checks; the operating-space table starts on its
own page so its ordinary missions and exceptions stay together. Text diff
checks pass. No Rust changed or new runtime test was required.

## Connections

- [Quick-start accuracy review](jl2o2ym3-quick-start-guide-accuracy-and-release-inclusion.md)
  records the earlier content and packaging work.
- [Quick-start guide](../../manuals/nc_quick_start.typ) gives the compact explanation.
- [Player manual](../../manuals/nc_player_manual.typ) gives the full timing and mission detail.
