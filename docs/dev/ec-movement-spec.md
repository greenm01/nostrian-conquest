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
| `0x23` | guarded starbase id (order 4) or join host fleet number, the host's `0x00` word (order 13) | u16 |

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
| Move, Patrol, Guard/Blockade, View, Scout Sector, Scout System, Join, Rendezvous | in the target sector and `Z == TZ` |
| Seek Home | the empire owns the target world (see below), then the test above, or `Round(Z) == 1` at the end of tick 52 |
| Colonize | the test above, or `Round(Z) == 1` at the end of tick 52 |
| Salvage | the empire owns the target world (see below), then in the target sector with `Z >= 1.0` |
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
in [ec-starbase-spec.md](ec-starbase-spec.md). Rust moves starbases the same
way, so a guard at a base in deep space settles in deep space.

Fleet ETA replays the same step from the fleet's current state and reports
the year the order's arrival test first passes. ECGAME's ETA screen asks
whether to include time to enter the system, which selects `TZ`.

### Seek Home And Salvage Check Their World

"[C]" marks a rule read from ECMAINT's code, "[P]" one confirmed by a classic
probe, as in [ec-encounter-spec.md](ec-encounter-spec.md); addresses are
seg:off in the unpacked ECMAINT.

Seek Home and Salvage look at the owner of their target world every tick, in
the fleet's dispatch after its step and before the arrival test (`0b5d:01f9`
Seek Home, `0b5d:4a44` Salvage). Nothing looks before the year, and only a
rogue empire's orders pass (below) ever re-picks a Seek Home fleet's world.
[C][P]

- While the fleet's empire owns the target world, the order runs on
  unchanged. Seek Home keeps the world it was given even when another owned
  world is nearer. [P: a fleet at (13,13) seeking (16,13) flew on toward it
  although (12,12), also owned, was nearer.]
- In the first tick the world is not the empire's, whether it was lost or
  never owned, the fleet turns for the nearest friendly world: the picker
  `24a3:0876` with flag 1 ([ec-encounter-spec.md](ec-encounter-spec.md)),
  measured from where the fleet has just stepped, so it has already moved one
  tick toward the old target. That tick reports no arrival. A target sector
  with no world counts as not owned (classic reads a garbage owner there; see
  the encounter spec's note on worldless sectors). [P: a Seek Home fleet whose
  target belonged to empire 2 turned at stardate 1 and ended the year over
  (12,12) at depth 0.670, which its first tick toward the old target
  explains; a fleet seeking an empty sector turned the same way.]
  - Seek Home changes only its target sector; the order, `TZ`, and speed
    stay. Report: "Since we have lost control of planet "*name*" in
    System(*x*,*y*), we are changing course and heading for a different
    friendly world. Our new destination is planet "*name*" in
    System(*x*,*y*)."
  - Salvage changes its target sector, refills a zero speed from `0x09`, and
    keeps the Salvage order, so the fleet salvages at the new world. Report:
    "Since we no longer own the world in System(*x*,*y*), we are aborting our
    salvage mission. Until ordered otherwise, we are going to seek to safety
    of a friendly controlled world. Our new destination is planet "*name*" in
    System(*x*,*y*)." [P: the fleet salvaged at (12,12) in tick 44 of the
    same year.]
- When the empire owns no other world, the fleet holds: order Hold, speed 0,
  target its own sector, `TZ = Round(Z)`. Seek Home leaves the depth as it
  is; Salvage also sets `Z` to `Round(Z)`, so a fleet between layers jumps to
  the nearer one. Reports: "Since we have lost control of planet "*name*" in
  System(*x*,*y*), we are holding our current position and awaiting new
  orders." and "Since we no longer own the world in System(*x*,*y*), we are
  aborting our salvage mission. Until ordered otherwise, we are going to hold
  our current position and await new orders." [P: Seek Home kept depth
  0.692; Salvage went from 0.692 to 1.0 and from 0.135 to 0.0.]

A Salvage that arrives (`0b5d:4d65`) adds to the world's stored goods
(PLANETS `0x09`) 23 per battleship, 8 per cruiser, 3 per destroyer, 3 per
transport empty or loaded, 10 per ETAC, and 8 per scout: each ship's half
cost rounded up, summed as a u16. Each loaded transport lands its army on the
world (PLANETS `0x58`), and the fleet is deleted. The report ends "We estimate
that our fleet will yield *N* production point(s)." and, with loaded
transports, "In addition, we have transfered the *N* armies we were
transporting to the planet's surface." ("army" for one). [C][P: one
battleship, cruiser, destroyer, empty transport, ETAC, and scout and two
loaded transports yielded 61 and landed 2 armies.]

Rust still retargets every Seek Home fleet before the year by squared
distance, lets a Salvage whose world was lost fail on arrival, and credits
an arriving Salvage with half the fleet's summed cost, counting each army at
2 and landing none; t064 brings it to this rule.

### Rogue And Autopilot Orders

Once a year, before the planet pass and the ticks, ECMAINT's orders pass
(`24a3:3cc6`) gives fleets new orders in `24a3:25f7` for rogue empires that
own a world (PLAYER `0x00 = 0xFF`, planet count `0x50` as the last run saved
it, nonzero) and for human empires on autopilot (PLAYER `0x6D` nonzero, which
ECMAINT also sets once a human empire has not signed on for SETUP's
`autopilot_after_turns` years). [C][P]

- A rogue empire's fleets all get new orders, whatever their order; an
  autopilot empire's only when they are on Hold. [P: autopilot left a Move
  fleet and a Seek Home fleet alone.] Fleets go in unit-table order, each
  empire's fleet chain.
- The destination is the picker with flag 2: every owned world, the one in
  the fleet's own system included, by travel distance divided by present
  production of at least 1.0 ([ec-encounter-spec.md](ec-encounter-spec.md)).
  When the empire owns no world, the fleet is left as it is.
- The fleet's target becomes that world with `TZ = 1`; a Guard Starbase
  fleet first clears its base's escort word (`2895:30f4`). A fleet in that
  world's sector with `Round(Z) = 1` and any battleship, cruiser, or
  destroyer takes Guard/Blockade (5) with speed 0 and its maximum speed
  recomputed (`24a3:036b`), and its ROE becomes 10 for a rogue, or 6 on
  autopilot when it was 0; without warships its order stays. Any other fleet
  takes Seek Home (2) at its maximum speed, and a rogue's ROE becomes 0.
  [P: an autopilot Hold fleet in orbit with a cruiser and ROE 0 took
  Guard/Blockade with ROE 6; one with only an ETAC stayed on Hold; a rogue
  fleet on a Move order went home within the year with ROE 0, and the rogue's
  Guard/Blockade fleet at home kept its order with ROE 10.]
- A fleet that is now on Guard/Blockade at speed 0 then takes the first
  starbase of its empire in its sector, in the system layer, at rest at its
  target, with no escort (BASES `0x00 = 0`): order Guard Starbase (4),
  `0x23` the base id, and the base's escort word the fleet's `0x00` word.
  [P: an autopilot Hold fleet in orbit under an unescorted Starbase 1 took
  order 4 with `0x23 = 1`, and the base's escort word became 1.]
- A rogue's starbases (`24a3:28fe`) take the same pick from their own
  position: target the world with `TZ = 1`, moving flag 0 if the base is
  already in orbit there, else 1. A rogue with no world sends its fleets on
  random patrols instead (`24a3:0dbc`). [C]

Rust's autopilot recall (`economics/ai.rs`) sends only autopilot Hold fleets
outside an owned world's sector home by squared distance, and gives rogue
fleets and starbases no orders; t064 brings it to this rule.

### Hostile Missions

Bombard, Invade, and Blitz attack on the tick the fleet reaches `Z = 1.0`, in
that year's maintenance. The fleet stops there, in orbit with speed 0, and
the attack resolves in that tick, in the fleet's slot of the year's unit
order, before the next unit moves. A fleet that starts the year in orbit at
its target on a hostile order attacks on the first tick. NC combat
([nc-combat-spec.md](nc-combat-spec.md)) decides the attack and what it
leaves on the order:

- Bombard keeps its order, and the fleet bombards again each year from orbit
  until the player changes the order.
- Invade and Blitz resolve, win or lose, and the surviving fleet holds in
  orbit.

Because each strike resolves in its tick, the rest of the year sees its
result: a fleet the batteries destroy is gone from the encounter loop, a
captured world has its new owner from the next tick (a starbase of the old
owner lifts, a later attacker meets the new owner), and fleets reaching the
same world strike in the order they arrive, whatever their orders. NC
policy, not classic: there is no class priority among Bombard, Invade, and
Blitz, and no limit on how often a world changes hands in a year.

Classic does not save the strike tick after a failed attack, so its fleet
keeps its order and speed one tick short of orbit and attacks again each
year. That is a classic bug; Rust does not copy it.

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
- Bombard stays armed and bombards each year until replaced; Invade and Blitz
  resolve, win or lose, and fall back to `Hold` in orbit
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
| `BombardWorld` | Hostile | `BOMBARD A WORLD` | transit to target world if needed | attacks the year it reaches orbit, stops in orbit, and bombards again each year | yes |
| `InvadeWorld` | Hostile | `INVADE A WORLD` | transit to target world if needed | attacks the year it reaches orbit; win or lose, then `Hold` in orbit | no |
| `BlitzWorld` | Hostile | `BLITZ A WORLD` | transit to target world if needed | attacks the year it reaches orbit; win or lose, then `Hold` in orbit | no |
| `ViewWorld` | One-shot completion | `VIEW A WORLD` | transit to target world edge, perform long-range scan, then back off | completes and returns to deep-space `Hold` | no |
| `ScoutSector` | One-shot completion | `SCOUT A SECTOR` | transit to target sector, perform sector reconnaissance | completes on arrival/report, then `Hold` | no |
| `ScoutSolarSystem` | One-shot completion | `SCOUT A SOLAR SYSTEM` | transit to target world, perform close reconnaissance | completes on arrival/report, then `Hold` | no |
| `ColonizeWorld` | One-shot completion | `COLONIZE A WORLD` | transit to raw world | completes on arrival if colonization succeeds; otherwise fails/aborts and awaits new orders | no |
| `JoinAnotherFleet` | Persistent standing | `JOIN ANOTHER FLEET` | transit toward the designated host fleet, with retargeting as the host moves | remains a join mission until fleets meet and merge; abandons if the host is lost | yes |
| `RendezvousSector` | Persistent standing | `RENDEZVOUS` | transit to the rendezvous sector, then wait there for more rendezvous fleets | lowest fleet ID host absorbs later arrivals and remains on rendezvous until replaced | yes |
| `Salvage` | One-shot completion | `SALVAGE` | transit to target world; if the world is lost, to the nearest friendly world | scraps the fleet at the world; holds when no friendly world is left | no |

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
