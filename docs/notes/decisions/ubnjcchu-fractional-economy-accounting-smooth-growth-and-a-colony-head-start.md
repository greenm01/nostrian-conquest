---
id: ubnjcchu
date: 2026-10-10
kind: adr
status: accepted
tags: [adr, economy, tax, growth]
---
# Fractional economy accounting, smooth growth, and a colony head start

## Context

NC's economy rounds production and revenue to whole points at several
steps and grows every world by at least one point a year. Together they
created a hidden optimum: a mature potential-100 world taxed at exactly 70%
settles at production 99 and income 69, but at 71% at 86 and 61, because
the one-point minimum offsets the emergency-tax penalty. Classic itself
keeps production and stored goods as Real48 values, fractional internally.
This is the first decision of the economy review (t051); the owner's
constraints stand: the tax rate is the only economic control, with no
investment menus or research budgets.

## Decision

The owner approved Claude and Codex's joint recommendation on 2026-10-10.

1. **Fractional accounting.** Production and stored goods carry fractions
   internally. Spending uses whole points from the non-negative part of
   stored goods; fractions and debts carry over. Build capacity uses
   production rounded to the nearest whole point, so the last build point
   of a fully developed world is never denied.
2. **Forecasts.** Next year's revenue appears in the existing planet and
   empire Revenue fields and in the tax feedback, assuming the tax rate,
   owner, and starbase stay as they are. The budget still shows what can be
   spent now. No new columns or menus, and no years-to-full estimate, since
   growth approaches potential without reaching a fixed date. Exact labels
   and rounding are checked against the BBS layouts.
3. **Smooth growth.** Today's growth and emergency-tax coefficients apply
   without their intermediate rounding and without the one-point yearly
   minimum. The 70% optimum disappears: at 70% a mature world settles at
   88.2 production and 61.8 income, at 71% at 85.8 and 60.9.
4. **Colony head start.** A newly founded colony starts at production 3, or
   its potential if lower, instead of 1. The head start applies only when a
   colony is founded, never on capture, recapture, or recovery, and does not
   depend on the tax rate.

## Alternatives

- Keep whole-point rounding and the minimum: keeps the 70% optimum.
- Smooth growth with no head start: small colonies develop noticeably
  slower (potential 10: about 27 income over ten years at 50% tax, against
  34 today; first cheap build in year 4 against 3).
- Keep the one-point minimum only below the 65% emergency threshold:
  rejected. It creates a new cliff (a potential-10 colony earns 41 over ten
  years at 64% tax and 28 at 65%) and a repeatable tax cycle (75%, 64%, 64%
  earns 3% more than a steady 65%), which pushes players to micromanage.

## Consequences

At 50% tax over ten years, with the head start: a potential-10 colony earns
31.9 (34 today) and makes its first 5-point build in year 3 as today; a
potential-20 colony earns 56.2 (56 today), its first cruiser in year 9
(year 8 today); a potential-100 colony earns 249.8 (252 today) with its
first 5, 15, and 45-point builds in years 1, 2, and 5 as today. Emergency
taxes keep a real later cost: one year at 100% raises that year's income
from 65 to 93 and the world recovers. Lowering taxes to grow colonies still
pays later (a home world and four potential-100 colonies at 35% for five
years, then 65%, trail a steady 65% by 195 points at year 5 and lead by 238
at year 20). These are exact model controls with no enemies; campaign
effects are not measured.

## Implementation

Done in t084. `nc-data/src/economy.rs` holds the rule in `f64`: `yearly_growth`,
`yearly_high_tax_penalty`, `next_production`, `tax_income`, and `next_pass`,
the forecast. The yearly pass (`nc-engine`, `maint/economics/planets.rs`)
stores production and goods back into Real48 without rounding. The build
pass and Salvage subtract and add against the exact treasury, so fractions
and debts carry over: a negative treasury stays negative and income is added
to it. `stored_production_points` is the floor of the non-negative part and
`present_production_points` rounds to the nearest point, which build capacity
uses. Colony founding (`maint/merging/colonization.rs`) sets production to
`min(3, potential)`; capture and recovery do not touch it. The planet list's
Rev and Gro columns, planet information's Expected Revenue, Empire Revenue,
and the tax confirmation ("Revenue next year") show the forecast to the
nearest point, with no new column and no change to any screen's width or row
count. The forecast reads 0 for a scorched or still-converting world.

`nc-data/tests/economy_model.rs` compares the engine with the exact models
within a relative error of 1e-8 (observed about 1e-10): the 70% and 71%
equilibria (88.24 and 61.76, 85.80 and 60.92), and the potential-10, 20, and
100 ten-pass rows with the head start. It also covers spending from a
fractional treasury and debt carried through income. Existing tests whose
numbers moved are listed in the t084 commit messages.

## Acceptance and connections

Accepted by the owner on 2026-10-10. Evidence: Codex's controls in
[1e8xu4w6](../investigations/1e8xu4w6-fractional-economy-accounting-and-tax-development-controls.md)
and `tools/economy-model/accounting_review.py`; scope in
[0t08owtu](../investigations/0t08owtu-economy-review-decisions-and-campaign-pacing-scope.md).
Earlier input: [tkce84f0](../investigations/tkce84f0-a-simple-nc-economy-informed-by-classic-ec-vbam-and-ec4x.md).
