---
id: 0dt7sw73
date: 2026-10-09
kind: adr
status: accepted
tags: [adr, setup]
---
# Homeworlds keep a fixed six-sector spacing

## Context

Map generation required homeworlds to sit at least 0.28 times the map size
apart. Map size grows with player count, so every tier's region grid works out
to about 9 sectors per player, and the target exceeded the grid above 4
players. When no candidate qualified, placement fell back to the region
center without a distance check. A 12-player map put two homeworlds 4
sectors apart against a target of about 10.

## Decision

Use a fixed minimum homeworld distance of 6 sectors for every player count.
When no sampled sector meets it, pick the free sector that maximizes the
minimum distance to homeworlds already placed, never the bare region center.

## Alternatives

Redesigning the region layout was rejected as a larger change for the same
result. Keeping a map-relative ratio with a smaller factor was rejected
because the grid cell, not the map, is what bounds the spacing.

## Consequences

Seeded maps changed in the same release because map generation also moved to
1-based coordinates. A sweep over every player count from 1 to 25 confirms the
spacing holds.

## Acceptance and connections

Accepted by the user on 2026-10-09. The normative text is
[starmap-generation-spec.md](../../dev/starmap-generation-spec.md). The work
is recorded in the
[October 2026 bug sweep](../milestones/ym5u2c1x-engine-and-door-bug-sweep-of-october-2026.md).
