---
id: h8sc14bj
date: 2026-11-01
kind: adr
status: accepted
tags: [adr, combat, balance, orbital-bombardment]
---
# Some bombardment gets past standing batteries, more on a larger world

## Context

Bombardment strikes the batteries first. While any battery stands, the hits
they can't take expire
([mk1ie4e3](mk1ie4e3-batteries-shield-the-stardock-and-failed-landings-hold-orbit.md)).
The owner worried that a world could rebuild batteries every year and keep
its industry untouched. Classic never shielded industry: its bombers got
random passes behind the battle, weighted toward the dock and goods.

## Decision

The owner chose this rule on 2026-11-01. In every bombardment round that
starts with a battery standing, `round(hits · share)` extra hits land
behind the batteries, on top of the hits they take.

- `share = (0.10 + 0.30 · (P − 10) / 140) · (roll + 1) / 5.5`:
  - `P` is the world's potential, clamped to 10–150. A larger world is
    harder to cover.
  - `roll` is the round's own `d10`. Its factor averages one, so the rule
    adds no new draw.
- Two fifths of the extra hits strike the stardock, a tenth the armies, and
  the rest goods and factories. Hits a target can't spend pass down.
- Worlds with no batteries are unchanged, and so are invasion and blitz
  fire.

## Alternatives

- **Diverting the leak from the batteries' share.** Rejected: it made big
  fortresses tougher, and first-year break rates fell to 0.87.
- **Other ranges.** 5–25% was milder and 15–55% harsher. 15–55% puts every
  hit through on the best rolls at the largest worlds.
- **No change.** Sieges at 1.5:1 or more already break fortresses in year
  one. But an undersized siege kept 77–88% of the world's production after
  ten years.
- **A fixed share.** The owner preferred variance and a share that scales
  with the world's size.

## Consequences

Battery break rates, the attacker's losses and every world without
batteries are exactly as before. A 1:1 siege with the AI's mix now leaves
59–65% of the world's production after ten years, not 78–88%. The loss as
a share of production is about the same for small and large worlds.
Bombardment ruins more of what an invader later captures. The AI must be
retrained on the rule.

## Acceptance and connections

Accepted by the owner on 2026-11-01 ("10–40%", after the simulation). Task
t234. Normative text: the player manual's bombardment rules and
[nc-combat-spec.md](../../dev/nc-combat-spec.md). Evidence:
[e1w4xpes](../investigations/e1w4xpes-lanchesters-square-law-doom-stacks-and-the-orbit.md#simulation-bombardment-leak).

Amended by [vv8rql6r](vv8rql6r-bombardment-ends-once-the-batteries-fall-and-strikes-armies-and-industry-at-half-strength.md)
(t236): the leak's armies and industry shares now strike at half strength.
