---
id: iq6sfxde
date: 2026-10-10
kind: adr
status: accepted
tags: [adr, economy, starbase]
---
# Starbases keep their growth and capacity bonuses without stacking

## Context

A starbase at a world raises its growth and lets it spend up to five times
its production. With [fractional accounting and smooth growth](ubnjcchu-fractional-economy-accounting-smooth-growth-and-a-colony-head-start.md)
approved, the economy review (t051, item 3) asked whether the bonus rates,
the price, stacking, damage, and eligibility still hold.

## Decision

The owner approved Claude and Codex's joint recommendation on 2026-10-10.

1. **Growth.** A starbase at rest in orbit of its owner's world makes that
   world grow half again as fast at taxes of 50% or less; the extra growth
   fades continuously to zero by 65%.
2. **Capacity and price.** A base lets its world spend up to five times its
   production; a starbase still costs 50.
3. **No stacking.** More bases at one world add defense and redundancy, not
   more growth or capacity.
4. **Damage.** A crippled base that survives keeps both bonuses; Recovery
   Turns assistance stays its own rule
   ([p01cg7ll](p01cg7ll-rt-controls-automatic-ship-recovery.md)).
5. **Eligibility.** Both inherited rules stay and are documented. Growth
   needs the owner's base stationary in orbit (ECMAINT `1852:011b..0206`,
   probed; Rust ignores classic's foreign-owner bug). Build capacity counts
   the owner's base in orbit or descending toward its own world's orbit
   (ECGAME's build screen, `0x12616..0x126D4`). So a base arriving at a
   world can raise its budget a pass before it raises growth. Docked and
   foreign bases count for neither.

In player terms: a starbase in orbit makes its world grow half again as
fast at normal taxes, less as taxes rise toward 65%, and lets the world
spend up to five times its production; more bases at one world add
defense, not growth.

## Alternatives

- One eligibility rule for both bonuses ("at rest in orbit"): clearer, but
  it would deliberately change classic's order admission.
- Halving a crippled base's bonuses with its combat strength: an extra rule
  for little gain.
- Changing the bonus or the price: the controls showed no need (a 25%,
  50%, or 100% bonus pays back a base at a fresh potential-100 colony in
  15, 8, or 5 passes).

## Consequences

Growth alone repays a base in about 8 passes at a fresh potential-100
colony, 22 if the base arrives five passes late, and never at a
potential-20 world, where a base is for defense and capacity. A mature world
gains nothing from the growth bonus. The two tax cycles tested with a base
present, 75%, 64%, 64% and 70%, 64%, fall behind a steady 65% in the long
run (after 20 cycles by 105 and 41 points from production 50, and by 96
and 35 from a fully developed world); that is evidence against cycling, not
a proof that no tax schedule can gain. Implementation is
pending, with the economy work (t084): the smooth fade, non-stacking, and
documentation in `docs/dev/economics.md` and the manual.

## Acceptance and connections

Accepted by the owner on 2026-10-10. Evidence: Codex's controls in
[w3vj4wti](../investigations/w3vj4wti-starbase-growth-capacity-and-economic-concentration.md)
and `tools/economy-model/starbase_review.py`; classic sources in
[ec-economy-reference.md](../../dev/ec-economy-reference.md).
