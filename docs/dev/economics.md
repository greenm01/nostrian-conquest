# Esterian Conquest Canonical Economics Spec

This document defines the canonical Rust economy model for Esterian Conquest.
It is the project’s auditable economy rule. While the 1992 `ECMAINT.EXE` formulas
remain opaque, this spec translates the documented manuals into an explicit
Rust rule set.

Read this together with:
- [ec-turn-cycle-spec.md](ec-turn-cycle-spec.md)
- [rust-turn-cycle-implementation.md](rust-turn-cycle-implementation.md)
- [nc-combat-spec.md](nc-combat-spec.md)

## Status

This is the source of truth for:
- Production terminology
- Opening homeworld setup
- Empire-wide tax behavior
- Yearly revenue and growth
- Starbase effects on build capacity and growth
- Scorch Earth

The starbase **5x** rule is settled:
- The manuals tie **5x** to build capacity.
- Ghidra analysis of the 1992 binaries found no evidence for a starbase growth
  multiplier.
- Black-box sweeps in [starbase-economy-oracle-audit.md](starbase-economy-oracle-audit.md)
  confirm the commissioned starbase grants a **5x** build-capacity bonus.
- Starbases improve the yearly growth allowance at low and moderate tax.
- This document defines the canonical Rust policy where classic recovery is
  incomplete.

This spec does not replace the 1992 manuals as historical sources. It does
not define the full maintenance turn order. It defines the economy and build
policy that runs inside the recovered post-loop world update region.

The decisions behind it are
[ubnjcchu](../notes/decisions/ubnjcchu-fractional-economy-accounting-smooth-growth-and-a-colony-head-start.md)
(fractional accounting, smooth growth, colony head start),
[iq6sfxde](../notes/decisions/iq6sfxde-starbases-keep-their-growth-and-capacity-bonuses-without-stacking.md)
(starbases), and
[nrvey7jk](../notes/decisions/nrvey7jk-lighter-bombardment-of-goods-and-classic-scorch-earth.md)
(Scorch Earth). The arithmetic matches the exact models in
`tools/economy-model/` (`accounting_review.py`, `starbase_review.py`,
`scorch_review.py`); the engine tests compare against them within a relative
error of 1e-8.

## Turn-Order Placement

ECMAINT runs its planet pass (`1852:0000`, called at `0824:29d1`) before
the year's 52 ticks (`0824:1004`, called at `0824:29d6`); see
[ec-turn-cycle-spec.md](ec-turn-cycle-spec.md) section 4.0. For each owned
world, in this order: the ownership status steps 0 to 1 to 2, the starbase
growth bonus is settled, the build queue completes, the economy runs (or, on a
world under Scorch Earth, the decay). Rust keeps that order:
`run_maintenance_turn` completes builds and settles the economy right before
the tick loop. The rogue and autopilot orders pass,
which classic runs at the start of the year before the planet pass, Rust
runs at the end of the previous turn, which is the same point in the cycle.

### Impact:
- Ships and starbases enter the stardock before the first tick, so a
  bombardment or capture in any tick of the year, and any scout's report,
  sees them.
- Armies and batteries completed this year defend the planet from the first
  tick.
- The economy reads the world as the previous year left it: a world
  colonized during the ticks takes its first income next year, and Salvage
  credits land on top of the year's income.
- A Scorch Earth order given this turn first takes effect in this pass, so it
  is already in force when an assault strikes during the ticks.
- The build budget is read before the year's income is credited; income is
  spent the following year.

## Source Basis

Primary 1992 manual references:
- [original/v1.5/ECPLAYER.DOC](../../original/v1.5/ECPLAYER.DOC)
- [original/v1.5/ECQSTART.DOC](../../original/v1.5/ECQSTART.DOC)

Manual-facing requirements:
- Each player begins with one homeworld at current production **100**.
- The tax rate is empire-wide.
- Taxes generate yearly production points.
- Newly colonized planets start below potential.
- Lower taxes improve planetary development.
- Taxes above **65%** can harm current production.
- Starbases:
  - Help planets endure tax burden.
  - Grant a **5x** build-capacity multiplier.
  - Help underdeveloped planets grow faster.

## Canonical Terms

Use these player-facing terms:
- **Production**: Current productive capacity, shown to the nearest whole point.
- **Potential Production**: Maximum productive capacity.
- **Revenue**: The tax income a planet's next yearly pass will credit, shown to
  the nearest whole point. It assumes the tax rate, the owner, and the
  starbase stay as they are. A planet under Scorch Earth, and a captured
  planet still converting, show 0.
- **Treasury**: Accumulated production points stored on a planet (per-planet).
  The screens show the whole points it can spend.
- **Budget**: `min(treasury, build_capacity)` — what a planet can spend this turn.
- **Empire Revenue**: Sum of per-planet Revenue across all owned planets.

Avoid low-level storage nicknames like `factories` in player surfaces.

## Opening Economy

Rust starts encode the intended opening economy:
- Homeworld Potential Production: **100**.
- Homeworld Production: **100**.
- Default Tax Rate: **50%**.
- First-Turn Empire Revenue: **50**.
- Homeworld Defenses: **10** armies, **4** ground batteries.

## Empire-Wide Tax

The tax rate is stored on the player record. He chooses one rate for all his
planets.

Income a planet earns in a yearly pass:
`income = production * tax_rate / 100`, on the production after that year's
growth and penalty, with no rounding.

Revenue, the figure the screens show, is the income the next pass will earn,
rounded to the nearest whole point. Setting the tax rate reports the new
Empire Revenue in its confirmation.

Empire Revenue:
`total = sum(revenue)` across all owned planets.

## Production Growth and Tax Pressure

Every owned planet grows toward its potential production each maintenance turn.
Production and stored goods are Real48 values and keep their fractions from
pass to pass. Nothing in the pass rounds them, and growth has no yearly
minimum. A world's production only reaches its potential in the limit.

### The Canonical Rust Rule:

1. **Growth Gap**: `gap = potential_production - production`
2. **Tax Headroom**: `headroom = 100 - min(tax_rate, 95)`
3. **Growth**: `growth = gap * headroom / 400`
4. **Starbase Bonus**: if the starbase rule below applies,
   `growth = growth * (1 + bonus)`, where `bonus` is `0.5` at tax 50% or
   less, `(65 - tax_rate) / 30` from 51% to 64%, and `0` at 65% and above.
5. **Clamp**: growth never exceeds the gap.
6. **High-Tax Penalty**: `penalty = production * max(0, tax_rate - 65) / 500`
7. **New Production**: `production = clamp(production + growth - penalty, 0, potential)`
8. **Income**: `income = production * tax_rate / 100`, added to the world's
   treasury.

Growth comes first, then income, on the new production.

### Where worlds settle:

Up to 65% a world reaches its potential. Above that, growth and penalty
balance where `production = potential * a / (a + b)`, with
`a = headroom / 400` and `b = (tax_rate - 65) / 500`. At 70% a mature world
with potential 100 settles at production 88.2 and income 61.8; at 71% at 85.8
and 60.9. Settled income peaks at 65% and falls with every point above it.

### Impact:
- Lower taxes produce faster growth.
- Higher taxes yield immediate revenue but slow development.
- Taxes above **65%** directly reduce production.
- Starbases accelerate development at low and moderate tax.
- Starbases do not grant an exemption from high-tax penalties.

## Starbase Effects

### 1. Growth Bonus
A starbase at rest in orbit of its owner's world makes that world grow half
again as fast at tax <= 50%. The extra growth fades in a straight line to
nothing at tax 65%: the bonus is `(65 - tax_rate) / 30` between 51% and 64%
(`0.467` at 51%, `0.167` at 60%, `0.033` at 64%). It is applied to the growth
term only and does not change the high-tax penalty. A mature world gains
nothing from it.

Which base counts follows ECMAINT (`1852:011b`). Classic settles it before the
year's 52 ticks, by scanning the unit table: a base in the world's sector at
depth layer 1 (`Z >= 0.5`) whose BASES moving flag (`0x1f`) is 0. A base in
deep space or under way gives nothing, even one that arrives in orbit during
the year. Rust takes the same test before movement. Classic also requires
the world's owner to have at least one base (PLAYER `0x44`), but not that the
base in orbit is the owner's; Rust requires the owner's base, since a base at
rest over another empire's world lifts away within a tick.

### 2. Build Capacity Multiplier
A starbase lets a planet spend up to **5x** its current production on units
in a single turn. Without a starbase, capacity is **1x**. This affects build
completion, not tax revenue.

The base must be the empire's, in the world's sector at `Z >= 0.5`, with the
world's sector as its target and orbit as its target layer. This is ECGAME's
build-screen test (image offset `0xa3fe`, "There is a starbase orbiting
planet"); it ignores the moving flag, so a base descending toward the world
counts.

So the two bonuses use different tests. A base arriving at a world can raise
its budget a pass before it raises its growth. A docked base, and a base of
another empire, count for neither.

### 3. No Stacking, and Damage
More bases at one world add defense and redundancy, not more growth or
capacity: the world has the bonus or it does not. A base has no persistent
damage. A crippled base that survives a battle keeps both bonuses.

The price of a starbase stays **50**.

The Revenue forecast assumes the base stays, and takes the capacity test above
for it.

## Treasury (Per-Planet Stored Production)

Yearly income is added to each planet's treasury as it stands, fraction
included. This is separate from the empire's Empire Revenue, which is a
forecast.

Spending takes whole points. The spendable part of a treasury is the whole
points of its non-negative part, `floor(max(0, treasury))`. The fraction left
over stays in the treasury. A negative treasury is a debt: it stays negative
and income pays it down. The screens show a debt as 0 spendable points. Bombardment
and Scorch Earth reduce only a positive treasury and leave a debt alone. Debt
comes from imported classic states, where ECMAINT charges builds without
checking the treasury; builds under Rust's budget rule never create it.

When maintenance processes a build queue, the planet spends from its treasury
by the number of build points actually applied that year. If a build cost
is larger than the planet's current per-turn build capacity, only that yearly
processed amount is deducted and the remaining cost stays queued for later
turns. If a build is blocked and no work is applied, the treasury is not
consumed.

Build capacity uses production rounded to the nearest whole point, so the
last build point of a fully developed world is never denied.

The planet's **budget** for a given turn is:
`budget = min(spendable treasury, build_capacity)`

Maintenance never spends more than the budget, so a planet with an empty
treasury completes nothing that year. New orders, from the build screen or a
turn file, may commit only the budget less the points already queued.
Cancelling a build releases the points still unpaid; points already spent on
a part-built unit are not returned.

## Special Cases

### Homeworlds
Homeworlds start at full production from turn one. A homeworld seed still
named "Not Named Yet" reads at its potential (classic seeds store 50), and
naming it stores that value. A named homeworld reports its stored production,
so high-tax penalties compound on it like any other planet.

### Newly Colonized Planets
A new colony starts at production 3, or at its potential if that is lower,
with no treasury. The head start applies when the colony is founded and never
on capture, recapture, or recovery, whatever the tax rate. A colony does not
receive revenue or growth on the maintenance tick that establishes ownership;
it first grows and earns in the next yearly pass, by the normal rules. Growth
is applied before income is credited. A potential-10 colony at 50% tax makes
its first 5-point build in the third pass; a potential-100 colony its first
5-, 15-, and 45-point builds in the first, second, and fifth.

### Captured Planets
A captured world's status steps 0, 1, 2 over its next two yearly passes. It
neither grows nor earns until the status is 2, so it pays nothing in those two
passes and full income from the third. Its Revenue shows 0 until then.

### Scorch Earth
The Scorch command sets the planet's Scorch Earth order, and the same command
rescinds it. The order changes nothing at once. It stays set until the player
rescinds it or the world is captured. Each yearly pass while it is set, after
the build queue completes and before the ticks:

- production keeps `0.30 + Random(20) / 100` of itself, never less than 1 and
  never more than the potential; a world with potential 0 stays at 0;
- positive stored goods become `min(goods / 10, sqrt(goods))`; a debt is left
  as it is;
- the world neither grows nor earns tax, and its Revenue shows 0.

Potential is kept, so the world recovers by the normal rules once the order is
rescinded. `Random(20)` is drawn from the campaign seed, one draw per world
per year. A converting world is scorched too. The maintenance report tells the
owner each year what the order cost the world and that it stays in force until
rescinded.

### Civil Disorder and Autopilot
The classic autopilot pass handles disorder. Empires in disorder (**0x00**) are
economically frozen. Normal-planet economy policy does not apply to them.
Rogue empires (**0xff**) run the normal economy pass.

The autopilot (rogue empires and human players on autopilot) plans builds
against the same budget, keeps existing queue entries and only tops them up,
and commissions the starbases it completes.

## Unit Build Completion

When maintenance applies build spending, completed units are dispatched:

- orders are whole units: queued points must be an exact multiple of the
  unit cost
- the planet's budget is the only limit on orders: orders of one kind share a
  build slot, and a slot holds any number of points, so one order may call
  for any number of units, starbases included
- build spending is applied during maintenance from the planet's treasury,
  up to the planet's budget
- partial progress carries the remaining build cost into later turns
- units appear as enough points are applied to complete them, even if other
  units from the same order remain queued
- blocked builds remain queued and do not consume the treasury that year

### Classic Files

Classic keeps a count of units by kind and charges for all of them at
ECMAINT's next run. Rust keeps the points still owed. Import reads a count
`n` of a kind as `n * cost` points owed and leaves the treasury alone, so
neither side has charged for the order. A file with orders in the reserved
slot 8 does not import. Export writes `ceil(points / cost)` units per kind and
adds the paid part of any part-paid unit, `ceil(points / cost) * cost -
points`, to the treasury; ECMAINT's charge for whole units then leaves the
treasury where Rust's own pass would.

### Ships and Starbases (Stardock)
Destroyers, cruisers, battleships, scouts, transports, ETACs, and starbases
enter the planet's stardock.
- The player must commission them before use.
- Uncommissioned units can be destroyed by bombardment.
- If stardock is full, the build remains in queue. Rust avoids the classic
  stardock corruption bug.

### Armies and Batteries (Planet Surface)
Armies and ground batteries go directly to the planet surface. They do not
require commission.
- A full stardock does not block them.
- Planet armies and batteries are capped at **255**.
- If a build would exceed the cap, it stays in the queue.

## Validation

The canonical model is backed by regression tests and the economy tax probe
tool. Refine this rule only when stronger 1992 evidence becomes available.
