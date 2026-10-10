---
id: mnb1phgg
date: 2026-10-09
kind: adr
status: superseded
tags: [adr, combat]
---
# Engagement follows the 1992 player guide

## Context

The fleet battle code had two engagement errors. A declared enemy passing a
guarded system without an assault order was never fought. A neutral fleet in
the deep space of an owned sector was attacked, because the owner's starbase
alone counted as a system entry. The Rust player manual said nothing about who
fights whom. The developer combat spec allowed only neutral assault fleets to
count as intruders. The 1992 player guide says that "your fleets will
automatically attack his/her fleets when they are encountered" for declared
enemies, and that "your fleets will also attack if another player's fleets
enter one of your solar systems, or if they try to enter or leave a planet
that your fleet is blockading."

## Decision

Adopt the 1992 rule, called option A in the review:

- Declared enemies are intercepted wherever they meet, gated by ROE.
- Any foreign fleet whose own mission takes it into your system is attacked,
  at war or not: Guard Starbase, Guard/Blockade World, Bombard, Invade, Blitz,
  and Scout Solar System.
- View World is exempt, because it scans from the system's edge.
- Fleets on every other mission stay in deep space and are reported, not
  attacked, unless their owner is a declared enemy.

## Alternatives

Option B kept the developer spec's rule: neutrals fight only on assault
orders, so a neutral scout could spy on a system freely. It was rejected
because the Rust manual was silent, the agent guide makes the 1992 docs the
next authority, and spying becomes a hostile act that escalates diplomacy.

## Consequences

Guards, starbases, and fleets at their own world now hold formation when an
enemy is only passing through and their ROE declines the fight. A lone
starbase therefore no longer attacks passing enemies. The user confirmed this
on 2026-10-09 (task t006). The 1992 player guide supports it: a starbase
"cannot attack since it moves very slowly ... but it can defend itself," and a
guarding fleet "may leave it if they have a high ROE and an enemy fleet comes
close to their solar system."

Leaving a blockaded world is now modeled as well (task t021). A fleet ordered
away from a world under blockade, including ships just launched from its
stardock, is attacked by the blockade before movement.

## Acceptance and connections

Accepted by the user on 2026-10-09. The normative text is the "Who Fights
Whom" section of the [player manual](../../manuals/nc_player_manual.typ) and
the engagement rule in [nc-combat-spec.md](../../dev/nc-combat-spec.md). The
work is recorded in the
[October 2026 bug sweep](../milestones/ym5u2c1x-engine-and-door-bug-sweep-of-october-2026.md).

Superseded on 2026-10-10 by
[Engagement follows the classic ECMAINT encounter rule](gtkmxw3l-engagement-follows-the-classic-ecmaint-encounter-rule.md),
after t034 read the rule from ECMAINT itself.
