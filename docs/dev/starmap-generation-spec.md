# Starmap Generation Spec

This document defines the intended Rust starmap-generation model for new games.

The original manuals define the hard semantic constraints:

- map size depends on player count
- total solar systems = `5 * player_count`
- each solar system contains one planet
- positions are randomly generated at game start

The manuals do not define a good algorithm for fair or interesting maps. The
Rust port should therefore use a canonical generation algorithm that preserves
those constraints while improving map quality.

This is an intentional Rust policy choice, not an unresolved oracle gap:

- preserve the original hard constraints from the manuals
- improve the original game's weaker homeworld and planet distribution quality
  inside those constraints

## Goals

The generator should produce maps that are:

- faithful to the manuals
- reproducible from a seed
- fair for all starting empires
- strategically interesting
- more elegant and less frustrating than the original EC maps

## Hard Constraints

For player-count tiers:

- `4 -> 18 x 18 map, 20 planets`
- `9 -> 27 x 27 map, 45 planets`
- `16 -> 36 x 36 map, 80 planets`
- `25 -> 45 x 45 map, 125 planets`

These counts are not tuning suggestions. They are gameplay rules.

## Generation Pipeline

Recommended pipeline:

1. choose player-count tier and map size
2. choose RNG seed
3. place homeworlds with fairness constraints
4. build Voronoi regions from homeworlds
5. generate a density field over the map
6. place remaining solar systems using region-aware weighted sampling
7. score the resulting map
8. accept or reroll

## Homeworld Placement

Homeworld placement should be engine-generated, not normally hand-authored in
sysop config.

Required properties:

- exactly one homeworld per player
- no homeworld on the extreme edge
- strong separation between homeworlds
- roughly balanced territorial spread

Concrete Rust rules:

- map coordinates are 1-based: an `N x N` map spans sectors `1..=N` on both
  axes, and the map centre is `(N + 1) / 2`
- every homeworld keeps at least `2` clear sectors to each edge, so homeworld
  coordinates stay within `3..=N-2`
- every pair of homeworlds is at least `6` sectors apart (straight-line
  distance), for every player count `1..=25`; the region grid gives each player
  roughly `9` sectors of width, so a ratio of the map size cannot be met above
  `4` players while a fixed `6` holds for every tier
- if no sampled sector in a player's region meets the spacing rule, the
  generator picks the free sector of that region farthest from the homeworlds
  already placed, never the bare region anchor
- neutral worlds are sampled from the interior `2..=N-1`; if no well-spaced
  sector is found, the fallback is still an unused sector

Recommended method:

- place homeworld candidates first
- partition the map into broad player regions
- score candidate placements by:
  - distance from edges
  - distance from other homeworlds
  - distribution balance
  - likely access to nearby neutral expansion

For `4` players, quadrant-based jitter is acceptable as an early policy.
For larger counts, use region seeds that approximate Voronoi territories.

## Planet Distribution

The engine should place exactly `5 * player_count` systems total, including the
homeworld systems.

Pure uniform random placement is discouraged because it tends to produce bad
clumps and dead zones. Pure Voronoi is also discouraged because it looks too
geometric.

Recommended hybrid:

- Voronoi regions for macro territory shape
- Perlin/fractal noise for local density variation
- spacing/rejection rules for clean distribution

### Voronoi Role

Voronoi regions should define:

- each empire's rough local space
- frontier boundaries between empires
- safe interior vs contested border space

### Noise Role

Perlin-style noise should modulate:

- local star density
- rich/sparse bands
- cluster shapes
- void corridors

Noise should not be the only placement algorithm. It should shape density, not
replace fairness rules.

### Spacing Role

Use minimum-distance / blue-noise / rejection rules so systems do not bunch up
ugly or overlap unrealistically.

## Desired Map Shape

Good maps should have:

- a few nearby expansion targets around each homeworld
- some medium-distance contested worlds
- some frontier friction between likely neighbors
- some sparse or low-density regions for maneuver
- no player boxed into a dead corner

## Fairness Metrics

After generating a candidate map, score it before accepting.

Minimum fairness checks:

- each player has a similar count of nearby planets within an early expansion
  radius
- each player has similar reachable production potential in the early game
- no player is isolated from the rest of the graph
- no player gets an obviously dominant local cluster
- contested worlds exist between likely neighboring empires

Reject and reroll bad maps.

## Production Distribution

Planet quality should also be distributed with structure.

Recommended shape:

- many middling worlds
- some poor worlds
- a few rich worlds
- no heavy concentration of rich worlds around one homeworld

This may also use the density/noise field, but fairness scoring should evaluate
reachable value, not just total count.

## Seed And Reproducibility

The generator should be seed-driven.

That means:

- same seed + same setup inputs => same generated map
- sysop config may specify a seed
- if no seed is provided, generate one and report it

This gives both:

- reproducibility for testing/admin work
- randomness for ordinary new games

## Current Implementation Scope

The current Rust implementation now covers the active canonical generator for
the documented manual player tiers:

1. deterministic fair homeworld placement across the supported player-count
   tiers
2. region-based homeworld placement with quadrant/sector ownership seeds
3. region-aware neutral-planet placement after homeworld placement
4. fairness scoring and reroll selection
5. noise-shaped density weighting for frontier placement
6. explicit one-planet-per-system enforcement by unique coordinates

Future work here is tuning, not rule recovery:

1. richer void/corridor shaping beyond the current lightweight noise field
2. deeper reachable-production graph scoring for larger galaxies
3. future tuning of region/frontier weights based on playtesting rather than
   only oracle acceptance

## Relationship To KDL

`setup.kdl` should describe setup intent such as:

- player count
- year
- setup mode
- optional seed

It should not normally hardcode homeworld coordinates. The engine should own
placement.
