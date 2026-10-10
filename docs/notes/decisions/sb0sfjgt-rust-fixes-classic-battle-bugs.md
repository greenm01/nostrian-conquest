---
id: sb0sfjgt
date: 2026-10-10
kind: adr
status: superseded
tags: [adr, combat, oracle]
---
# Rust fixes classic battle bugs

## Context

Rust matches the classic binaries exactly, except where classic is clearly
broken. The t036 decode of ECMAINT's battles
([ec-battle-spec.md](../../dev/ec-battle-spec.md), section 8) found ten
defects. The largest explains a rule Rust had copied: classic marks a
destroyed assault fleet inactive just before deleting it, the delete skips
inactive fleets, and the fleet comes back the next year with all its ships
and attacks again. Every fleet that survives an assault, whatever the
result, goes to Hold. The "hostile fleets retry each year" rule that t031
built and documented was this bug.

## Decision

Rust does not copy these:

- B1: a destroyed assault fleet is deleted. Surviving assault fleets go to
  Hold, as classic does, so there is no yearly retry.
- B2: a destroyed bombard fleet stops bombing.
- B3: blitz landing damage is clamped at 0.
- B4: the landing's transport targeting window uses the right indices.
- B5: the stardock loss pick uses the full width.
- B6: the stardock value counts cruisers once (`9 * CA`), not twice.
- B7: the range check against a scattering fleet uses elapsed rounds.
- B8: a lone starbase's side has a defined speed.
- B9: the production division guards against a potential of 0.
- B10: invasion's 4:1 early stop uses the surviving loaded transports.

## Alternatives

Copy B6 and B10, which change balance rather than break the game. The user
chose to fix both.

## Consequences

t036 implements these with the classic battle. It also removes the yearly
hostile retry from the engine, ec-movement-spec.md, and the player manual.
Rust results differ from classic wherever these defects fire; the battle spec
marks each one.

## Acceptance and connections

Accepted by the user on 2026-10-10. The normative text is
[ec-battle-spec.md](../../dev/ec-battle-spec.md). Related:
[engagement ADR](gtkmxw3l-engagement-follows-the-classic-ecmaint-encounter-rule.md).

Superseded on 2026-10-10 by [NC combat stays](b5u6lekb-rust-keeps-its-own-combat-system.md): Rust does not port classic battles, so these fixes have nothing to apply to.
