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

Current-known movement rules in Rust already model:

- simultaneous yearly movement
- persistent in-transit progress toward a target location
- order-driven arrival and mission resolution
- contact and combat based on post-movement position

The live movement implementation is in:

- [mod.rs](../../rust/nc-engine/src/maint/mod.rs)
- [movement/mod.rs](../../rust/nc-engine/src/maint/movement/mod.rs)
- [navigation/mod.rs](../../rust/nc-engine/src/navigation/mod.rs)
- [geometry.rs](../../rust/nc-engine/src/navigation/geometry.rs)
- [fleet_motion_state.rs](../../rust/nc-data/src/fleet_motion_state.rs)

The current implementation keeps the recovered annual `speed * 8 / 9` movement
budget, persists exact in-transit position between yearly maintenance passes,
and rounds only when writing the visible sector coordinates.

Map coordinates are 1-based: an `N x N` map spans sectors `1..=N` on both
axes, so routes may use the last row and column but never row or column `0`.

### Movement Fraction Byte

Each year the stepper adds `speed * 8` to a sub-unit accumulator, moves
`accumulator / 9` units, and carries `accumulator % 9` to the next year in the
classic `FLEETS.DAT` `raw[0x0F]` byte:

- encode: `(carry - 9) * 2 / 3` as a signed byte
- decode: `9 + raw * 3 / 2`, clamped to `0..=9`

This classic encoding is kept as the compatibility boundary even though it is
lossy: carries other than `0`, `3`, and `6` decode slightly high, so speeds
that are not multiples of 3 run a little fast (speed 1 covers 12 sectors in
13 years instead of 14). A carry of `8` encodes as `0` and decodes as `9`. The clamp only guards
malformed imported bytes and leaves every value the engine writes unchanged.

Fleet ETA estimates replay the same encode/decode round trip each simulated
year, so the predicted ETA matches the actual arrival year on axis-aligned
moves. The starbase travel-time estimate keeps the exact accumulator because
it reproduces the classic ECGAME display (`Travel time is 16 years` for a
speed-1 starbase from (15,13) to (2,12)).

For unresolved classic movement internals, Rust follows a pragmatic boundary:

- preserve player-facing travel geometry, arrival timing, and mission behavior
  where the oracle evidence is clear enough to matter
- keep internal continuity state explicit inside Rust when it improves ETA and
  movement behavior
- do not treat hidden scratch-byte layouts or every low-level numeric quirk as
  a required fidelity target by themselves

So `MoveOnly` is still a one-shot order that eventually falls back to `Hold`,
but Rust is not obligated to mirror every hidden classic transit byte on the
way there.

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

3. **Delayed-resolution hostile orders**

- these orders may require travel first, but arrival is not the end of the
  mission
- after the fleet reaches the target, the order remains armed until the ready
  hostile-world step resolves it or invalidates it
- current classic coverage shows that the arrival tick preserves both the
  standing order and the fleet's current travel speed; the fleet does **not**
  drop to `speed=0` until the later hostile-world step consumes or invalidates
  the order
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
| `BombardWorld` | Delayed-resolution hostile | `BOMBARD A WORLD` | transit to target world if needed | persists through arrival, keeping the arrival-tick travel speed, until the ready bombardment step resolves or is invalidated | yes |
| `InvadeWorld` | Delayed-resolution hostile | `INVADE A WORLD` | transit to target world if needed | persists through arrival, keeping the arrival-tick travel speed, until the ready invasion step resolves or is invalidated | yes |
| `BlitzWorld` | Delayed-resolution hostile | `BLITZ A WORLD` | transit to target world if needed | persists through arrival, keeping the arrival-tick travel speed, until the ready assault step resolves or is invalidated | yes |
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
