# EC Movement And Pathfinding Spec

This document separates two concerns:

- classic EC movement semantics from the manuals and observed maintenance rules
- canonical Rust pathfinding policy for choosing safer routes

The first is a fidelity target. The second is an explicit extension.

## Source Authority

Primary sources:

- [ECPLAYER.DOC](../../original/v1.5/ECPLAYER.DOC)
- current movement RE and fixtures in
  [RE_NOTES.md](archive/RE_NOTES.md)

## Classic Movement Semantics

Movement is 2.5D. A fleet has a position on the flat sector map and a depth
that says how far it has descended toward the world at the center of its
sector, from 0 (deep space) to 1 (orbit). The model below was decompiled from
ECMAINT and checked against a classic trace matrix; the evidence is in the
[investigation note](../notes/investigations/9msqq5e0-classic-move-orders-complete-on-entering-the-target-sector.md).

The live implementation is in
[movement/](../../rust/nc-engine/src/maint/movement/) and
[navigation/](../../rust/nc-engine/src/navigation/).

Map coordinates are 1-based: an `N x N` map spans sectors `1..=N` on both
axes, so routes may use the last row and column but never row or column `0`.

### Fleet State

| Bytes | Field | Encoding |
| --- | --- | --- |
| `0x0a` | current speed | u8 |
| `0x09` | maximum speed | u8 |
| `0x0b`, `0x0c` | sector `x`, `y` | u8 |
| `0x0d..0x12` | in-sector `x` fraction `fx` | Turbo Pascal 6-byte real |
| `0x13..0x18` | in-sector `y` fraction `fy` | Turbo Pascal 6-byte real |
| `0x19..0x1e` | depth `Z` | Turbo Pascal 6-byte real |
| `0x20`, `0x21` | target sector | u8 |
| `0x22` | target depth `TZ`: `0` sector target, `1` world target | u8 |
| `0x23` | guarded starbase id (order 4) or join host fleet id (order 13) | u8 |

A 6-byte real stores a biased exponent in byte 0 (`0` means `0.0`) and a
39-bit mantissa in bytes 1 to 5, little-endian, with the sign in the top bit
of byte 5. The value is `(1 + m / 2^39) * 2^(e - 129)`. The fleet's
continuous position is `X = x + fx`, `Y = y + fy`; the sector center is
`fx = fy = 0.5`.

ECGAME writes `0x22 = 0` for Move, Patrol, Scout Sector, and Rendezvous and
`0x22 = 1` for every other order.

### Yearly Step

Classic runs 52 ticks a year. Each tick gives a fleet a budget of
`speed / 52` and applies one step toward `(tx + 0.5, ty + 0.5, TZ)`:

1. If the fleet is outside the target sector, or deeper than `TZ`, it climbs:
   the budget reduces `Z`, and `Z` stops at 0.
2. Any budget left moves the fleet straight toward the target sector center.
3. Once at the center, any budget left takes the fleet down toward `TZ`.

Arithmetic is in 6-byte reals. Rounding matters: a fleet can finish a climb
at `Z = 1 - epsilon` rather than `1.0`, and the arrival tests compare against
`1.0` exactly, so such a fleet arrives a year later. Rust steps fleets with the
same rounding and, like classic, interleaves them tick by tick in a random
order drawn once a year, with the encounter check between each fleet's step
and its arrival test ([ec-encounter-spec.md](ec-encounter-spec.md)).

A fleet at rest in orbit has `Z = 1`, so it spends its first sector of
movement climbing out: speed `v` covers `v - 1` sectors in its first year and
`v` a year after that.

The dispatcher refills `speed` from `0x09` when a fleet that has not arrived
has speed 0.

### Arrival

After each tick's step the order's arrival test runs; the first tick that
passes resolves the order.

| Orders | Arrival test |
| --- | --- |
| Move, Patrol, Guard/Blockade, View, Scout Sector, Scout System, Join, Rendezvous, Salvage | in the target sector and `Z == TZ` |
| Seek Home, Colonize | the test above, or `Round(Z) == 1` at the end of tick 52 |
| Guard Starbase | the starbase is in the same sector and depth layer (`Z >= 0.5`) and speed > 0 |
| Bombard, Invade, Blitz | ownership checks, then in the target sector with `Round(Z) >= 1` and `Z >= 1.0` |

With `TZ = 0` the base test passes on entering the target sector, anywhere in
it. With `TZ = 1` the fleet must reach the center and then descend fully, so a
world order usually takes a year longer than a move to the same sector.

When View World resolves, classic writes order Move, refills the speed from
`0x09`, and sets `0x22 = 0`, so the fleet climbs back to deep space and the
Move completes when it reaches depth 0. If the climb runs past tick 52, the
file shows order Move at full speed for a year. Rust does the same.

Guard Starbase takes its target depth from the starbase. Classic moves
starbases at speed 1: a base in orbit with no world of its owner's beneath it
climbs to deep space, and every time a base changes sector or depth layer,
classic rewrites its guards' target sector and sets their `0x22` to the
base's layer. A guard that arrives takes the base's depth, and a stopped
guard in the same sector and layer rides along with the base; the rules are
in [ec-starbase-spec.md](ec-starbase-spec.md). Rust does not move starbases
yet (t037), so a guard at a base in deep space settles in orbit instead.

Fleet ETA replays the same step from the fleet's current state and reports
the year the order's arrival test first passes. ECGAME's ETA screen asks
whether to include time to enter the system, which selects `TZ`.

### Hostile Missions

Bombard, Invade, and Blitz attack on the tick the fleet reaches `Z = 1.0`, in
that year's maintenance. If the attack fails, classic does not save that
tick's state, so the fleet keeps its order and speed and holds at the
previous tick's depth, one tick short of `1.0` (for example `0.988129`). It
attacks again the next year, on the first tick it reaches `1.0`. Classic's
battle is random, so a weak fleet can retry for many years. Rust decides each
attack with its own combat model and applies the same retry rule.

## Standing Order Model

A practical way to think about fleet orders is:

- a fleet always has exactly one current standing order
- there is no separate zero-order state distinct from `HoldPosition`; order
  code `0` is still a real standing order
- maintenance advances that order until it completes, persists, aborts, or is
  retargeted by game rules
- if the player issues a new order before maintenance, that new order replaces
  the old one

`Hold` is therefore not special in terms of editability. It is just the
idle/default standing order. A fleet on `Move`, `Patrol`, `Guard`, `Bombard`,
or `Hold` can all be given a new order; the newly entered order is the one
maintenance will process next.

The main nuance is that fleet orders fall into three categories:

1. **One-shot completion orders**

- these orders mean "go there, do the thing, then stop"
- after they resolve, the fleet normally falls back to `Hold`
- examples: `MoveOnly`, `SeekHome`, `ViewWorld`, `Scout*`, `ColonizeWorld`,
  `Salvage`

2. **Persistent standing orders**

- these orders remain armed until the player replaces them or game rules
  explicitly invalidate them
- some are static watch/guard orders, while others wait, chase, or assemble
  fleets over multiple maintenance turns
- current classic coverage shows that "persists" does not mean "keeps moving":
  `PatrolSector`, `GuardStarbase`, and `GuardBlockadeWorld` keep their standing
  order on arrival, but the fleet's `current_speed` still drops to `0`
- they remain armed if the player does not replace them
- examples: `HoldPosition`, `PatrolSector`, `GuardStarbase`,
  `GuardBlockadeWorld`, `JoinAnotherFleet`, `RendezvousSector`

3. **Hostile orders**

- these orders travel to the target world, go into orbit, and attack in the
  year they reach orbit
- a failed attack leaves the order, the speed, and the fleet just short of
  orbit, and the fleet attacks again each year
- examples: `BombardWorld`, `InvadeWorld`, `BlitzWorld`

Player-level rule: "new order overrides whatever the fleet was doing" is the
correct model.

Manual-backed merge specifics:

- `JOIN ANOTHER FLEET` persists until the join resolves or the host fleet is
  lost
- if a join host is itself absorbed into another surviving fleet, joiners
  retarget to that survivor instead of abandoning the mission
- a joiner already co-located with the surviving fleet completes the join that
  same maintenance turn
- `RENDEZVOUS` persists at the specified sector so additional rendezvous fleets
  can keep merging there
- when multiple rendezvous fleets merge, the fleet with the lowest fleet ID is
  the host/survivor

## Order Reference

The table below is the player-facing order model that Rust should preserve.
"Persists" means the mission is intended to remain armed if the player does not
replace it. "Completes" means the fleet falls back to `Hold` when the mission
finishes.

| Order | Category | Classic label | Travel shape | Normal post-arrival behavior | Persists if not replaced? |
| --- | --- | --- | --- | --- | --- |
| `HoldPosition` | Persistent standing | `NONE` / Hold position | no travel | stays idle at current position | yes |
| `MoveOnly` | One-shot completion | `MOVE FLEET` | direct transit to sector | completes on arrival, then `Hold` | no |
| `SeekHome` | One-shot completion | `SEEK HOME` | transit to nearest owned world, with retargeting if that world is lost | completes on arrival, then `Hold` | no |
| `PatrolSector` | Persistent standing | `PATROL A SECTOR` | transit to patrol sector if needed, then intercept/watch posture | order persists on arrival, but fleet stops at the patrol sector | yes |
| `GuardStarbase` | Persistent standing | `GUARD A STARBASE` | transit to base if needed, then escort/guard posture | order persists on arrival, but fleet stops at the guarded base | yes |
| `GuardBlockadeWorld` | Persistent standing | `GUARD/BLOCKADE A WORLD` | transit to target world if needed, then guard/blockade posture | order persists on arrival, but fleet stops at the guarded world | yes |
| `BombardWorld` | Hostile | `BOMBARD A WORLD` | transit to target world if needed | attacks the year it reaches orbit; after a failed attack it keeps its order and speed and attacks again each year | yes |
| `InvadeWorld` | Hostile | `INVADE A WORLD` | transit to target world if needed | attacks the year it reaches orbit; after a failed attack it keeps its order and speed and attacks again each year | yes |
| `BlitzWorld` | Hostile | `BLITZ A WORLD` | transit to target world if needed | attacks the year it reaches orbit; after a failed attack it keeps its order and speed and attacks again each year | yes |
| `ViewWorld` | One-shot completion | `VIEW A WORLD` | transit to target world edge, perform long-range scan, then back off | completes and returns to deep-space `Hold` | no |
| `ScoutSector` | One-shot completion | `SCOUT A SECTOR` | transit to target sector, perform sector reconnaissance | completes on arrival/report, then `Hold` | no |
| `ScoutSolarSystem` | One-shot completion | `SCOUT A SOLAR SYSTEM` | transit to target world, perform close reconnaissance | completes on arrival/report, then `Hold` | no |
| `ColonizeWorld` | One-shot completion | `COLONIZE A WORLD` | transit to raw world | completes on arrival if colonization succeeds; otherwise fails/aborts and awaits new orders | no |
| `JoinAnotherFleet` | Persistent standing | `JOIN ANOTHER FLEET` | transit toward the designated host fleet, with retargeting as the host moves | remains a join mission until fleets meet and merge; abandons if the host is lost | yes |
| `RendezvousSector` | Persistent standing | `RENDEZVOUS` | transit to the rendezvous sector, then wait there for more rendezvous fleets | lowest fleet ID host absorbs later arrivals and remains on rendezvous until replaced | yes |
| `Salvage` | One-shot completion | `SALVAGE` | transit to target world | completes when salvage resolves or fails | no |

## Contact And Hostility

The manuals support this distinction:

- `enemy` is a stored diplomatic relation
- `hostile` is the broader encounter state used for actual combat

Canonical Rust direction:

- fleets that encounter one another should always generate a contact/intel
  report
- fleets should automatically engage only when the encounter is hostile
- declared enemies meeting in one final location should auto-engage
- non-enemy contacts should not fight unless:
  - one side attacks
  - a defensive rule makes the contact hostile
  - blockade / system-entry rules apply

## Current Rust Simplification

The current Rust maint path resolves contact from final post-movement
co-location. It intentionally does not model true mid-course interception
geometry for "crossed paths" between sectors.

That simplification should remain explicit in docs and tests.

## Canonical Rust Pathfinding Extension

Threat-aware pathfinding is a desirable Rust extension, but it should be
documented as an intentional improvement rather than implied to be a recovered
original mechanic.

### Why It Belongs

If a player sends a fleet to a destination, the route should not be stupid.
When known hostile systems, blockades, or starbases exist, a modern faithful
port should be able to avoid obvious destruction when the mission does not
require entering hostile space.

### Pathfinding Goals

Canonical Rust pathfinding should:

- preserve the same movement execution rules once a route is chosen
- choose safer routes when multiple legal paths exist
- avoid accidental entry into hostile systems unless the mission explicitly
  targets that system
- prefer reduced detection/contact risk for scouting and transit missions

### Suggested Cost Model

Use shortest-path routing with additive penalties for known danger:

- enemy-owned solar systems
- known starbases
- active blockades
- known hostile homeworlds

Do not treat transient deep-space fleet sightings as durable route hazards by
default. A fleet sighting in open space is perishable intel; by the time the
moving fleet reaches that sector, the observed force may already be gone.
Pathfinding should therefore avoid building stable route penalties from open
space contact reports alone.

Suggested policy:

- hard-avoid hostile systems for civilian or non-combat transit where possible
- soft-penalize risky sectors for scouts and movement missions
- ignore transient deep-space sightings unless a later, explicitly documented
  policy introduces a very short-lived penalty
- allow direct hostile routing when the target mission is itself hostile:
  - bombard
  - invade
  - blitz
  - guard/blockade hostile target

### Algorithm Direction

The intended long-term routing engine is:

- grid-based A* over the sector map
- deterministic tie-breaking
- route costs derived from visible game state and diplomatic hostility

This should be implemented as a route-planning layer above the current movement
formula, not as a replacement for the movement execution rules themselves.

## Fog Of War Rule

Threat-aware routing must preserve fog of war.

- route costs must be derived from player-visible intel, not hidden global
  truth from the whole gamestate
- the planner may use:
  - visible world ownership
  - visible starbases
  - known blockades
  - stored diplomatic hostility
- the planner should not treat deep-space fleet sightings as stable hazards by
  default
- the planner must not "cheat" by consulting unseen foreign planets, fleets,
  or defenses

Current Rust implementation status:

- the `nc-data` pathfinder now accepts explicit visible hazard intel as input
- the live maint path now accepts owner-scoped visible hazards and the CLI
  derives first-pass foreign-world hazard intel from each empire's
  `DATABASE.DAT` view
- the live visible-hazard path now also derives hostile blockade hazards for an
  empire's own worlds from current gamestate, because that information is
  player-visible without consulting hidden foreign-space intel
- only visible intel should feed the planner; unknown foreign worlds remain
  invisible to routing until discovered
- combat/contact handling now also has an explicit hostility decision seam
  rather than burying that choice inside the battle loop
- `nc-data` now exposes a typed stored-diplomacy seam, and the classic
  `PLAYER.DAT` enemy/neutral bytes are now mapped as a contiguous table:
  - `PLAYER.DAT[player].raw[0x54 + (target_empire_raw - 1)]`
  - `0x00 = neutral`
  - `0x01 = enemy`
- the contiguous `0x54..=0x6C` span provides 25 diplomacy slots
- the live Rust path now also accepts a `diplomacy.kdl` sidecar in the game
  directory as a migration/fallback source; persistable relations are absorbed
  into `PLAYER.DAT`
- the current hostility predicate therefore uses:
  - declared-enemy status from stored bytes
  - declared-enemy status from `diplomacy.kdl`, if present
  - defensive/manual hostility triggers
- foreign co-location by itself should generate contact intel but not force
  combat
- fleet encounters now generate contact/intel reports even when the encounter
  source is not a scout mission

## Compatibility Rule

Threat-aware routing may change which sectors a Rust-controlled fleet chooses to
travel through. That is acceptable if:

- the resulting orders and yearly movement remain classic-compatible on disk
- the pathfinding policy is documented and deterministic
- the policy remains faithful to player intent and manual combat/diplomacy
  rules

It is not acceptable to silently claim that such routing was proven to be the
original DOS behavior when it has not been recovered.
