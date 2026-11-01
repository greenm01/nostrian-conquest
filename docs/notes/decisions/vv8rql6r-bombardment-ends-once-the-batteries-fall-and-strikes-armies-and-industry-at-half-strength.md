---
id: vv8rql6r
date: 2026-11-01
kind: adr
status: accepted
tags: [adr, combat, balance, orbital-bombardment]
---
# Bombardment ends once the batteries fall and strikes armies and industry at half strength

## Context

Under t234 a bombardment turn ran three full-strength rounds. Once the
batteries were gone, or at a world with none, that emptied a world in one
year: ten cruisers left an open potential-80 world with nothing. A siege at
1.5:1 left a fortress with nothing, too. Classic's run did far less: ten
cruisers left 79% of the factories. The owner judged t234 too harsh once the
batteries fall.

## Decision

The owner chose these on 2026-11-01:

1. **One round once no battery stands.** A bombardment turn ends with the
   round that leaves no battery. That is the round that destroys the last
   battery, or the first round at a world with none. That round's leftover
   hits still flow on to the dock, armies and industry.
2. **Half strength behind the dock.** Hits that pass the stardock are
   halved, rounded down, before they strike the armies and industry. The
   leak past standing batteries
   ([h8sc14bj](h8sc14bj-some-bombardment-gets-past-standing-batteries-more-on-a-larger-world.md))
   is halved the same way. The stardock takes full fire.

Fire at the batteries, return fire, invasion and blitz are unchanged.

## Alternatives

- **Losing the leftover hits when the last battery falls.** Gentler at
  fortresses: 25% of production lost at 1.5:1 instead of 48–62%. But a
  bigger force would gain nothing from breaking the batteries early, which
  classic rewards with more passes.
- **One round without half strength.** It still emptied a mid-size world in
  three years against ten cruisers.
- **A classic floor of 1.0 production.** Left out at first, then adopted
  by the owner the same day (t237; see Amendment).

## Consequences

Battles are exactly as before. A cruiser removes about 2.4 points of
production a year from an open world, near classic's 2.1. Ten cruisers
leave a potential-80 world 77% of its production after one year and 9%
after three. A doom stack still levels a world in a year, as in classic.
Bombardment destroys less of what an invader later captures. The AI must be
retrained (t235).

## Acceptance and connections

Accepted by the owner on 2026-11-01 ("Keep, at half"). Task t236. Normative
text: the player manual's bombardment rules and
[nc-combat-spec.md](../../dev/nc-combat-spec.md). Evidence:
[e1w4xpes](../investigations/e1w4xpes-lanchesters-square-law-doom-stacks-and-the-orbit.md#simulation-fewer-rounds-and-half-strength-behind-the-batteries),
`tools/combat-model/collateral_t236.py`.

## Amendment (t237, 2026-11-01)

The owner asked to match classic's production floor. Bombardment never
takes a world's production below 1.0. A world already under 1.0 keeps
what it has, where classic raises it to 1.0. A world kept at 1.0 regrows
from there once the siege lifts, rather than from nothing.

## Amendment (t238, 2026-11-01)

Under the strict order, armies shielded the factories. An army costs 2 and
soaked 20 half-strength hits, so a world could stack cheap armies to keep
its industry. The owner chose to split the main fire that passes the dock
as the leak does: a sixth to the armies, the rest to goods and factories.

Simulated over five years at a potential-80 world with no batteries, under
10 cruisers, the defender spending half its production on armies:
- **Strict order:** starting with 50 armies, the world kept 100%, 96% and
  80% of its production after years 1, 3 and 5. With 100 it kept all of
  it.
- **The split:** 70%, 12% and 1%, whatever the garrison. That's the same as
  a world with no armies.

Armies still wear down slowly, so they keep their value against invasion.
Battle results are unchanged.
