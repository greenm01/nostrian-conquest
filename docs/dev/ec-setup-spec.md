# EC Setup And Starmap Spec

This document defines the intended Esterian Conquest setup model for the Rust
port.

The original manuals are the semantic authority here. Exact byte-for-byte map
recreation from one historical game is not required. What matters is faithful
adherence to the documented setup rules while preserving classic-compatible
save directories.

The important split is:

- original EC docs settle the hard setup constraints
  - square grid
  - map size by player-count tier
  - `5 * player_count` solar systems / planets
- the Rust clone is free to improve poor classic homeworld and neutral-world
  distribution inside those constraints

## Source Authority

Primary source:

- [ECPLAYER.DOC](../../original/v1.5/ECPLAYER.DOC)

Relevant manual statements:

- the galaxy is a square grid
- map size depends on player count
- solar systems are randomly generated at game start
- each solar system contains one colonizable planet
- the total number of solar systems is `5 * player_count`
- each player begins with one planet producing `100` points per year
- each player begins with four fleets:
  - two fleets containing `1 cruiser + 1 ETAC`
  - two fleets containing `1 destroyer`

## Canonical Setup Goals

Rust setup should:

- follow the player-count and map-size rules from the manuals
- generate a valid star map and initial empire state without violating classic
  file compatibility
- preserve interchangeability with the original DOS binaries at the `.DAT`
  directory boundary
- avoid treating the current generalized builder as the final gameplay setup
  model
- keep sysop setup flows distinct from the player client workflow

## Player Count Tiers

The original game supports these canonical player-count tiers:

- `4 players -> 18 x 18` map
- `9 players -> 27 x 27` map
- `16 players -> 36 x 36` map
- `25 players -> 45 x 45` map

The intended long-term Rust initializer should treat those as the primary
supported setup modes.

The current generalized builder may continue to support narrower experimental
counts for oracle and compatibility work, but that flexibility should not be
confused with the documented gameplay setup rules.

## Star Map Rules

The map is a square sector grid. Each sector is either:

- empty deep space
- one solar system containing one planet

The total number of solar systems is:

- `5 * player_count`

Since each solar system contains one planet, the total number of planets is
also:

- `5 * player_count`

That means the documented galaxy totals are:

- `4 players -> 20 planets`
- `9 players -> 45 planets`
- `16 players -> 80 planets`
- `25 players -> 125 planets`

The positions of solar systems are randomly generated at game start.

This implies:

- star-system count is a first-class gameplay rule, not a scenario convenience
- planet count is a first-class gameplay rule, not a tuning suggestion
- a faithful Rust setup flow needs explicit map-generation rules, not just
  hand-placed known scenarios

The intended map-generation algorithm is documented separately in
[starmap-generation-spec.md](starmap-generation-spec.md).

## Planet And Empire Start Rules

Each player begins with:

- one owned homeworld
- current production `100`
- four initial fleets:
  - two `CA + ETAC`
  - two `DD`

The long-term setup model should make the following explicit:

- homeworld placement constraints
- homeworld separation constraints
- initial fleet placement relative to the homeworld
- initial ownership and development fields required for classic compatibility

## Compatibility Rule

The Rust setup flow does not need to reproduce the original game’s hidden map
RNG sequence exactly.

It does need to:

- generate manual-faithful map sizes and system counts
- generate sane starting empires and planets
- emit classic-compatible `.DAT` directories accepted by original tools

## Sysop Workflow Boundary

Game setup is a sysop/admin responsibility, not a normal player action.

For the Rust port, that means:

- setup/map-generation rule execution should live in `nc-engine`
- shared setup config parsing and baseline state-construction helpers may live
  in `nc-data`
- admin-facing setup commands may live in `nc-cli` or a future sysop UI
- the player client should not be the primary place where a new game is
  initialized or rewritten

This follows the original `ECUTIL` / `ECGAME` separation even if the Rust
implementation later shares more code under the hood.

## Implementation Direction

Refine setup in this order:

1. codify manual-driven setup invariants
2. audit the current builder against those invariants
3. separate "compatibility builder" from "faithful game initializer"
4. add a dedicated Rust initializer for canonical EC game start
5. validate generated starts against original-tool acceptance and gameplay
   sanity rather than byte-perfect RNG recreation

## Current Rust Audit

The current Rust setup/builder layer is a compatibility-oriented baseline
constructor that now satisfies the manuals' hard setup constraints, while still
leaving distribution quality as an intentional Rust policy layer.

Recent Rust changes removed the old fixed `4 player / 20 planet` storage cap:

- `PLAYER.DAT`, `PLANETS.DAT`, and `DATABASE.DAT` now use dynamic record counts
- setup generation now supports the documented `4`, `9`, `16`, and `25`
  player tiers in one shared path
- current preflight validation now accepts `1..=25` players and enforces the
  `5 * player_count` planet rule

Those changes are now sufficient for the manuals' full player-count tier model
at the setup/storage level.

So the current code should now be understood as:

- compatible setup/storage infrastructure for all documented player-count tiers
- a manual-faithful generated new-game path on the original square-grid /
  player-tier map model
- map shape and player-count-based map sizing are now closed constraints
- future setup work is now about generator quality and play balance, not about
  recovering more core setup rules

## Audit Consequences

The setup roadmap is now split differently:

1. preserve the accepted generated setup path across all documented player-count tiers
2. refine generator quality inside that fixed square-grid / player-tier model

## Current Generated Path

The Rust sysop path now has a seeded generator for the documented player
tiers:

- public sysop path: `nc-sysop new-game <target_dir> [--players <1-25>] [--seed <u64>]`
- internal dev preset path: `nc-cli sysop new-game <target_dir> --config nc-cli/config/setup.example.kdl`

Current behavior:

- homeworld placement is engine-generated rather than KDL-authored
- the generated map is seed-reproducible
- the generator populates exactly `5 * player_count` planets for the active
  player count
- the generator enforces one planet per system by unique coordinates after
  homeworld placement
- coordinates are 1-based (`1..=map_size`), homeworlds stay at least `2`
  sectors from every edge and at least `6` sectors from each other for every
  player count (see [starmap-generation-spec.md](starmap-generation-spec.md))
- player counts outside `1..=25` are rejected by every setup entry point
- homeworlds keep the documented fixed starting production of `100`
- the default `sysop new-game` path now materializes a joinable pre-player
  baseline for `ECGAME`:
  - inactive player slots
  - `Not Named Yet` homeworld seeds
  - pre-join fleet blocks already parked at those homeworld coords
- neutral worlds are distributed by a fairness-scored generated map rather than
  by pure random placement
- the data model and generator now cover the documented `4/9/16/25` setup
  tiers rather than only the old 4-player bridge case
- the explicit `setup_mode="builder-compatible"` config path remains available
  for tests/sweeps that need the older post-join active-campaign baseline
- the seeded `sysop new-game` path now has broader oracle coverage:
  - `4/9/16/25` players
  - seeds `1515`, `2025`, `4242`
  - `12/12` ECMAINT oracle passes via
    `python3 tools/oracle_sweep.py --mode seeded`

This is a deliberate bridge:

- more manual-faithful than fixed sysop-authored homeworld coordinates
- intentionally better distributed than the original game's weaker random
  homeworld/planet placement
- still interoperable with the classic `.DAT` compatibility boundary
- already accepted by the original `ECMAINT` oracle across multiple seeds and
  all currently supported manual player tiers

## Non-Goals

This spec does not try to reproduce:

- the exact procedural RNG used by the DOS binaries
- the exact classic solar-system distribution algorithm
- exact homeworld placement heuristics used by `ECUTIL`

Those are not prerequisites for a faithful manual-driven Rust setup
implementation, because the Rust clone intentionally improves map quality while
preserving the manuals' hard setup constraints.
