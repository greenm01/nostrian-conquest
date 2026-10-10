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

The starbase **5x** rule is settled:
- The manuals tie **5x** to build capacity.
- Ghidra analysis of the 1992 binaries found no evidence for a starbase growth
  multiplier.
- Black-box sweeps in [starbase-economy-oracle-audit.md](starbase-economy-oracle-audit.md)
  confirm the commissioned starbase grants a **5x** build-capacity bonus.
- Starbases improve the yearly **grow** allowance under tax pressure.
- This document defines the canonical Rust policy where classic recovery is
  incomplete.

This spec does not replace the 1992 manuals as historical sources. It does
not define the full maintenance turn order. It defines the economy and build
policy that runs inside the recovered post-loop world update region.

## Turn-Order Placement

The recovered yearly turn order constrains economy and build behavior:
- Build completion and economy run in the **post-loop world update region**.
- This occurs **after** the weekly 52-pass fleet combat loop.
- This occurs **before** the hostile world-resolution region (Bombard, Invade,
  Blitz).

### Impact:
- Ships and starbases enter stardock before a hostile assault hits the planet.
- Bombardment and invasion see post-build planet state.
- Armies and batteries completed in this region go straight to the planet surface.
- Later hostile actions read these units as planet state, not stardock inventory.

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
- **Production**: Current productive capacity.
- **Potential Production**: Maximum productive capacity.
- **Revenue**: Per-planet current-turn tax income (`floor(present_production * tax_rate / 100)`).
- **Treasury**: Accumulated production points stored on a planet (per-planet).
- **Budget**: `min(treasury, build_capacity)` — what a planet can spend this turn.
- **Empire Revenue**: Sum of per-planet revenue across all owned planets.

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

Yearly revenue per planet:
`revenue = floor(present_production * tax_rate / 100)`

Empire Revenue:
`total = sum(revenue)` across all owned planets.

## Production Growth and Tax Pressure

Every owned planet grows toward its potential production each maintenance turn.

### The Canonical Rust Rule:

1. **Growth Gap**: `gap = potential_production - present_production`
2. **Tax Headroom**: `headroom = 100 - min(tax_rate, 95)`
3. **Base Growth**: `base = ceil(gap * headroom / 400)`
4. **Starbase Bonus**:
   - Apply a bonus if a friendly starbase is in orbit.
   - Tax <= 50%: **+50%** bonus.
   - Tax 51%–64%: Linearly taper bonus to 0%.
   - Tax >= 65%: No bonus.
   `growth = base + ceil(base * bonus_percent / 100)`
5. **Clamp**: Minimum growth is **1**; maximum is the remaining gap.
6. **New Production**: `production = present_production + growth`
7. **High-Tax Penalty**:
   - If tax > 65%, calculate penalty: `penalty = ceil(production * (tax - 65) / 500)`
   - Minimum penalty is **1**.
   - Penalty cannot exceed current production.

Final result:
`present_production = (present_production + growth) - penalty`

Yearly revenue is credited after this update, on the new
`present_production`. Growth comes first, then revenue.

### Impact:
- Lower taxes produce faster growth.
- Higher taxes yield immediate revenue but slow development.
- Taxes above **65%** directly reduce production.
- Starbases accelerate development at low and moderate tax.
- Starbases do not grant an exemption from high-tax penalties.

## Starbase Effects

### 1. Growth Bonus
A starbase at rest in orbit provides a tax-sensitive growth bonus. It applies
**+50%** of base growth at tax <= 50%, tapering to 0% at tax 65%. The amount
is the canonical Rust policy.

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

## Treasury (Per-Planet Stored Production)

Yearly tax revenue is added to each planet's treasury. This is
separate from the empire's Empire Revenue total.

When maintenance processes a build queue, the planet spends from its treasury
by the number of build points actually applied that year. If a build cost
is larger than the planet's current per-turn build capacity, only that yearly
processed amount is deducted and the remaining cost stays queued for later
turns. If a build is blocked and no work is applied, the treasury is not
consumed.

The planet's **budget** for a given turn is:
`budget = min(treasury, build_capacity)`

Maintenance never spends more than the budget, so a planet with an empty
treasury completes nothing that year. New orders, from the build screen or a
turn file, may commit only the budget less the points already queued.

## Special Cases

### Homeworlds
Homeworlds start at full production from turn one. A homeworld seed still
named "Not Named Yet" reads at its potential (classic seeds store 50), and
naming it stores that value. A named homeworld reports its stored production,
so high-tax penalties compound on it like any other planet.

### Newly Colonized Planets
Fresh colonies do not receive same-turn revenue or growth on the maintenance
tick that establishes ownership. They begin at production 0 with no
treasury, then start growing on the next maintenance turn according to the
normal tax and growth formulas. Growth is applied before revenue is credited,
so at moderate tax (35-65%) a colony with potential 15 or more has a nonzero
budget after its first full turn. Very small worlds (potential 5-10) or low
tax (25% or less) can leave a colony at zero budget for a turn or more, because
`floor(production * tax_rate / 100)` rounds to zero.

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
- build spending is applied during maintenance from the planet's treasury,
  up to the planet's budget
- partial progress carries the remaining build cost into later turns
- units appear as enough points are applied to complete them, even if other
  units from the same order remain queued
- blocked builds remain queued and do not consume the treasury that year

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
