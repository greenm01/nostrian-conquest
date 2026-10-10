---
id: lzvpk22s
date: 2026-10-09
kind: adr
status: accepted
tags: [adr, combat]
---
# Planetary hits pay defense strength

## Context

Bombardment applied one hit per battery, army, or stardock item, although ship
fire runs on a 10x scale. A single cruiser's 30 hits could destroy 10
batteries in round 1. The combat spec's stated design is that "a small
raiding fleet may take several turns of three-round suppression before
breaking through." The manual's unit table already gives batteries DS 20 and
armies DS 10.

## Decision

Each planetary target costs its defense strength in hits: 20 per battery, 10
per army, and a docked ship's own DS. Hits that reach stored goods and
factories are divided by 10 first, keeping them on the same scale. Hits below
one unit's cost destroy nothing and do not carry over between rounds.
Invasion softening compares bombardment AS against armies times 10.

## Alternatives

Keeping one hit per unit and documenting it was rejected: it makes batteries
nearly worthless against a single warship, against the spec's design goal.

## Consequences

Fortified worlds now take sustained bombardment. One piece of classic
evidence fits the cost model, though weakly because the classic scale is
unknown: a 100-ship bombardment destroyed all 15 batteries but only 82 of 142
armies. Blitz landing fire still costs one transport per battery on the old
scale; that is task t007 in [todo.md](../../../todo.md).

## Acceptance and connections

Accepted by the user on 2026-10-09. The normative text is the combat appendix
of the [player manual](../../manuals/nc_player_manual.typ) and
[nc-combat-spec.md](../../dev/nc-combat-spec.md). The work is recorded in the
[October 2026 bug sweep](../milestones/ym5u2c1x-engine-and-door-bug-sweep-of-october-2026.md).
