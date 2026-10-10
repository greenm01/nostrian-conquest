# Fixture Workflow

Fixtures are the main bridge between reverse engineering and implementation.

They let us:

- preserve known-good states
- diff specific transitions
- turn observed behavior into tests

## Existing Fixture Families

### Baselines

- `original/v1.5/`
  - shipped sample state and bundled assets (year 3022, 13 fleets, 1 starbase)
- `fixtures/ecutil-init/v1.5/`
  - known-good initialized game state produced by `ECUTIL` (year 3000, 16 fleets, no starbases)
- `fixtures/ecmaint-post/v1.5/`
  - known-good post-maintenance state produced by `ECMAINT` (one maint pass from init)
- `fixtures/ecutil-f3-owner/v1.5/`
  - targeted ownership-change snapshot for `ECUTIL F3`

### Build Queue

- `fixtures/ecmaint-build-pre/v1.5/` / `fixtures/ecmaint-build-post/v1.5/`
  - single planet build queue transition

### Economics

- `fixtures/ecmaint-econ-pre/v1.5/` / `fixtures/ecmaint-econ-post/v1.5/`
  - economic maintenance pass (production, tax, population)

### Fleet Movement

- `fixtures/ecmaint-fleet-pre/v1.5/` / `fixtures/ecmaint-fleet-post/v1.5/`
  - fleet movement resolution
- `fixtures/ecmaint-fleet-battle-pre/v1.5/` / `fixtures/ecmaint-fleet-battle-post/v1.5/`
  - fleet-vs-fleet combat scenario

### Bombardment

- `fixtures/ecmaint-bombard-pre/v1.5/` / `fixtures/ecmaint-bombard-post/v1.5/`
  - basic bombardment scenario
- `fixtures/ecmaint-bombard-arrive/v1.5/`
  - fleet arriving at bombard target (single state)
- `fixtures/ecmaint-bombard-heavy-pre/v1.5/` / `fixtures/ecmaint-bombard-heavy-post/v1.5/`
  - heavy bombardment scenario
- `fixtures/ecmaint-bombard-army0-pre/v1.5/` / `fixtures/ecmaint-bombard-army0-post/v1.5/`
  - bombardment with 0 armies on target
- `fixtures/ecmaint-bombard-army0-dev0-pre/v1.5/` / `fixtures/ecmaint-bombard-army0-dev0-post/v1.5/`
  - bombardment with 0 armies and 0 development on target
- `fixtures/ecmaint-bombard-army1-pre/v1.5/` / `fixtures/ecmaint-bombard-army1-post/v1.5/`
  - bombardment with 1 army on target
- `fixtures/ecmaint-bombard-army1-dev0-pre/v1.5/` / `fixtures/ecmaint-bombard-army1-dev0-post/v1.5/`
  - bombardment with 1 army, 0 development
- `fixtures/ecmaint-bombard-army1-dev0-b08-pre/v1.5/` / `fixtures/ecmaint-bombard-army1-dev0-b08-post/v1.5/`
  - bombardment variant (b08 battleship count)
- `fixtures/ecmaint-bombard-army1-dev0-b09-pre/v1.5/` / `fixtures/ecmaint-bombard-army1-dev0-b09-post/v1.5/`
  - bombardment variant (b09 battleship count)
- `fixtures/ecmaint-bombard-army1-dev0-e0c-pre/v1.5/` / `fixtures/ecmaint-bombard-army1-dev0-e0c-post/v1.5/`
  - bombardment variant (e0c parameter)

### Invasion

- `fixtures/ecmaint-invade-pre/v1.5/` / `fixtures/ecmaint-invade-post/v1.5/`
  - heavy invasion scenario

### Starbases

- `fixtures/ecmaint-starbase-pre/v1.5/` / `fixtures/ecmaint-starbase-post/v1.5/`
  - Guard Starbase (order 0x04) scenario — init state with BASES.DAT added and
    three required patches: `FLEETS.DAT[0x1F]=0x04`, `FLEETS.DAT[0x23]=0x01` (starbase id 1),
    `PLAYER.DAT[0x44]=0x01`

### Reports With Several Human Empires

Both start from `ecmaint-fleet-battle-pre` with empire 1 joined as a human
(`PLAYER.DAT[0x00]=0x01`, handle `BAR`).

- `fixtures/ecmaint-results-two-human-pre/v1.5/` / `fixtures/ecmaint-results-two-human-post/v1.5/`
  - two human empires whose fleets meet; `RESULTS.DAT` holds one contiguous
    chain per empire
- `fixtures/ecmaint-results-interleaved-pre/v1.5/` / `fixtures/ecmaint-results-interleaved-post/v1.5/`
  - empire 3 also joined (handle `BAZ`) and the location, movement and order
    fields `FLEETS.DAT[0x0A..0x24]` of fleets 1 and 9 swapped, so empire 1
    arrives late; three chains interleave in event order

## Rules For New Fixtures

Only create a new fixture set when it captures a meaningful state transition.

Good examples:

- one `ECUTIL` option change
- one ownership change
- one maintenance run after one known order
- one battle-producing scenario

Avoid:

- redundant copies with no documented purpose
- mixed scenario states where too many variables changed at once

## Recommended Workflow

1. Start from a known baseline

- usually `fixtures/ecutil-init/v1.5/` or `fixtures/ecmaint-post/v1.5/`

2. Make exactly one controlled change

- one menu option
- one fleet order
- one build order
- one ownership edit

3. Save and preserve the result

- copy the changed files into a clearly named fixture directory
- note exactly what was changed and why the fixture exists

4. Diff against the baseline

- identify changed files
- identify changed offsets or records
- turn confirmed fields into Rust accessors/setters/tests

5. Validate through generated reports when useful

- if `ECGAME` can read the post-maint state cleanly, use it to view the reports
- treat report viewing as a secondary confirmation step, not the primary source of truth
- the primary evidence remains:
  - the pre/post fixture pair
  - the raw file diffs
  - the decoded state transitions
- preserved historical text captures are useful for interpreting `MESSAGES.DAT` and
  `RESULTS.DAT` when live playback is unavailable

6. Add tests

- parser tests for new fields
- CLI/TUI tests when behavior is exposed there

## Naming Guidance

Use descriptive fixture names tied to one observed operation.

Examples:

- `fixtures/ecutil-f4-snoop-off/v1.5/`
- `fixtures/ecutil-f5-flow-off/v1.5/`
- `fixtures/ecmaint-bombard-scenario/v1.5/`

## Key Idea

Fixtures are not just test data.

They are the durable record of observed original behavior, and they should be treated as first-class preservation assets.
