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
- **A classic floor of 1.0 production.** Not needed for the owner's goal,
  and left out to keep the rule small.

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
