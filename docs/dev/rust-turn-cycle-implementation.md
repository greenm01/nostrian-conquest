# Rust Turn-Cycle Implementation Spec

This document is the implementation-facing companion to
[ec-turn-cycle-spec.md](ec-turn-cycle-spec.md).

For the current claim-by-claim Rust conformance snapshot against the canonical
spec, use
[ec-turn-cycle-compliance-audit.md](ec-turn-cycle-compliance-audit.md).

Read it together with:

- [nc-combat-spec.md](nc-combat-spec.md)
  for mission/combat behavior
- [economics.md](economics.md)
  for the Rust economy/build policy of the planet pass that precedes the
  tick loop
- [ec-timing-spec.md](ec-timing-spec.md)
  for weekly scheduler and report `Stardate` behavior
- [rust-architecture.md](rust-architecture.md)
  for the repository-wide data-oriented/module-boundary rules this engine
  implementation should follow

Ownership boundary:

- this document owns yearly phase placement and turn-order boundaries
- [nc-combat-spec.md](nc-combat-spec.md)
  owns combat and hostile world-resolution mechanics
- [ec-timing-spec.md](ec-timing-spec.md)
  owns weekly timing, report-week assignment, and `Stardate` header formatting
- [economics.md](economics.md) owns
  the Rust economy/build policy of the planet pass

Use it when designing or refactoring the Rust maintenance engine.

This companion is Rust-native first. It describes the maintenance cycle as a
runtime-state, simulation, and event-pipeline problem. The recovered
`ECMAINT`/classic behavior remains the historical reference, but classic
directories and `.DAT` outputs are compatibility projections at the edge, not
the main mental model for the engine itself.

Current conformance notes:

- phase `3` is now enforced at the engine entrypoint through
  `validate_maintenance_state()`, but the current structural rule set remains
  narrower than the full recovered classic validation surface
- phase `6` weekly report-emission parity remains the clearest explicit gap
- deterministic visit order and the simplified weekly loop shape remain
  intentional, documented Rust design choices rather than open regressions

This is not the raw RE notebook and not a byte-offset map. Its job is to
describe the turn-cycle as a practical engine/state-machine problem:

- which phases exist
- what each phase is responsible for
- what state each phase may read or write
- which boundaries are settled
- how the turn-order boundaries interact with the companion combat, economics,
  and timing specs

For raw oracle evidence and confidence notes, use the canonical turn-cycle
spec. For combat rules and timing/report formatting, defer to the dedicated
companion specs above.

## Scope

This document is for the Rust yearly maintenance engine.

It deliberately focuses on:

- human-digestible process flow
- engine boundaries
- durable state vs transient workspaces
- report/event generation boundaries
- safe implementation seams for Rust

It deliberately avoids:

- low-level byte-field inventories
- speculative semantic naming for still-unknown fields

## One-Page Model

The current best implementation model is:

1. decide whether maintenance may run now
2. recover from interrupted movement if needed
3. validate coherent runtime state and initialize transient workspaces
4. run the yearly simulation core
5. canonicalize the generated event pool
6. emit weekly reports and rebuild output projections
7. flush outputs, clean up, and end the tick

## Block Diagram

```text
Runtime State + Maintenance Context
    |
    v
+------------------------------+
| 1. Schedule / Token Gate     |
+------------------------------+
    |
    v
+------------------------------+
| 2. Recovery / Restore        |
|    if prior run halted       |
|    during movement           |
+------------------------------+
    |
    v
+------------------------------+
| 3. Validate Runtime State    |
|    + Init Workspaces         |
+------------------------------+
    |
    v
+------------------------------+
| 4. Yearly Simulation Core    |
|                              |
| 4a. snapshots + year advance |
| 4b. pre-move prep            |
| 4c. planet pass: builds,     |
|     then economy             |
| 4d. the year's ticks:        |
|     movement, encounters,    |
|     battles, strikes,        |
|     colonization             |
| 4e. autopilot/rogue orders,  |
|     player recompute,        |
|     campaign                 |
| 4f. conquest/header          |
|     normalization            |
+------------------------------+
    |
    v
+------------------------------+
| 5. Summary Canonicalization  |
|    / Matching / Sorting      |
+------------------------------+
    |
    v
+------------------------------+
| 6. Weekly Report Emission    |
|    + Output Projections      |
+------------------------------+
    |
    v
+------------------------------+
| 7. Final Flush / Cleanup     |
+------------------------------+
```

## Process Flow

```text
START
  |
  v
Check schedule and token files
  |
  +--> not allowed to run -> exit without simulation
  |
  v
Check crash marker for interrupted movement
  |
  +--> recovery needed -> restore movement-safe state
  |
  v
Validate coherent runtime state
  |
  +--> validation failure -> abort with error outputs
  |
  v
Create / reset transient workspaces
  |
  v
Run annual movement update
  |
  v
Run pre-loop fleet setup if needed
  |
  v
Determine fleet visit order (sort-by-random-priority; Rust may use slot order)
  |
  v
52-week fleet-processing loop
  |
  +--> per week: process each fleet in visit order
  |    +--> read fleet, timing-window check
  |    +--> resolve fleet-vs-fleet combat and record timed outcomes
  |    +--> resolve the fleet's planetary strike when its test passes
  |         (bombard / invade / blitz / aftermath)
  |    +--> update weekly fleet event state
  |    +--> write fleet, remove destroyed/captured
  |
  v
Post-loop world/player update region:
  |
  +--> autopilot / rogue orders (classic runs these at the start of the
  |    next year, before its planet pass)
  |
  +--> player planet-count / production recompute
  |
  v
Producer/mutator + campaign passes
  |
  v
Derived-state / header updates
  |
  v
Canonicalize and sort summary/events
  |
  v
Walk internal weekly timeline and build player-visible outputs
  |
  v
Rebuild output projections
  |
  v
Cleanup tokens / work files and finish
  |
  v
END
```

## Rust Maintenance Turn Order

This is the current Rust-facing turn order. Use
[ec-turn-cycle-spec.md](ec-turn-cycle-spec.md) for the recovered
`ECMAINT`/oracle phase order; use this section when reasoning about the code
that exists today.

1. schedule/token gate
2. interrupted-run restore if needed
3. validate the runtime campaign state
4. run the yearly simulation core
   - snapshot start-of-turn state needed by delayed missions/campaign logic
   - advance year
   - merge co-located friendly fleets before movement
   - sanitize invalid player inputs
   - refresh join-host and guard-starbase targets
   - apply autopilot fleet orders
   - run the planet pass: build completion, then planet economy
   - intercept fleets ordered away from blockaded worlds
   - run the year's ticks: movement, encounters, fleet battles, and
     planetary strikes
   - merge mission fleets
   - apply colonization
   - run autopilot/rogue orders and player recompute
   - apply campaign transitions, starbase flags, join-host updates, and
     conquest-header normalization
5. assemble and canonicalize the event pool
6. build report/output projections and apply delivery-side flags
7. flush outputs, clean up, and end the tick

Combat location summary:

- fleet-vs-fleet combat happens after annual movement and before colonization
- departure interceptions at blockaded worlds (a Rust engagement rule) happen
  before annual movement, so the departing fleets are still at the world
- build completion and economy happen before the ticks, so before fleet
  battles
- ready hostile world resolution happens later than both fleet battles and
  build/economy work

Recovered hostile-world rule:

- delayed hostile world missions do not mutate the target world on tick `1`
  after arrival
- ready hostile world missions select their target-world resolution family by
  mission kind (`BombardWorld` vs `InvadeWorld`) in the later hostile
  world-resolution region
- ready hostile world missions read post-build planet state

## Engine State Model

The Rust engine should model four distinct state layers.

| Layer | Meaning | Examples | Lifetime |
| --- | --- | --- | --- |
| Durable runtime state | The real campaign state that survives the turn | players, planets, fleets, bases, IPBMs, conquest/setup state | persisted |
| Transient staging/workspaces | Scratch collections used while validating or simulating | staged tables, temporary counters, intermediate working sets | one maintenance run |
| Durable summary/event pool | Intermediate event records that survive long enough to be canonicalized and turned into reports | summary/event entries later matched, sorted, and emitted | one maintenance run |
| Output projections | Rendered/report-facing artifacts rebuilt from runtime state and events | results rows, preserved mail payloads, rankings, database projections | regenerated each run |

Practical rule:

- do not collapse these layers into one giant mutable pass
- especially do not mix:
  - validation scratch
  - durable simulation outcomes
  - late report formatting

## Recommended Rust Phase Map

| Phase | Responsibility | Reads | Writes | Confidence |
| --- | --- | --- | --- | --- |
| 1. Gate | Decide whether maintenance may run now | schedule config, token files | token/work status only | High |
| 2. Recovery | Recover from interrupted movement | crash marker, recovery markers/backups | restored durable state | High |
| 3. Validate/init | Build coherent runtime state and reject impossible campaigns | runtime state, structural links, existing orders | validated state, transient workspaces, error outputs on failure | High |
| 4. Simulation core | Apply the yearly game rules | validated state, staged work data, existing orders | durable runtime state, durable summary/event pool | Medium |
| 5. Canonicalize events | Match, coalesce, sort, and normalize event records | durable summary/event pool | canonicalized event pool | High |
| 6. Emit outputs | Convert canonical events into report rows and rebuilt projections | canonical event pool, durable runtime state | player-visible reports, rankings, database/mail projections | High |
| 7. Flush/cleanup | Finish the tick cleanly | work markers, generated outputs | final files, token cleanup | High |

## Recommended Rust Subsystems

The Rust engine should stay split by responsibility, not by one giant
"maintenance" function.

| Subsystem | Responsibility |
| --- | --- |
| Gate/recovery | schedule check, token coordination, interrupted-run recovery |
| Loader/validator | runtime-state loading, structural linkage checks, structural normalization |
| Simulation driver | orchestration of yearly simulation subphases |
| Movement/contact/combat | fleet motion, encounters, combat outcomes, retreat-vs-hold routing for invalidated missions, retargets |
| Producer passes | state-mutator/event-producer families inside step `4` |
| Event pool | typed durable summary/event records |
| Canonicalizer | matching/coalescing/sorting of event records |
| Report emitter | weekly timeline walk and player-visible report/message generation |
| Projection builder | rankings, database, compat-edge output artifacts |

## Step 4: What Rust Should Assume Today

Step `4` is now substantially recovered. The right Rust posture is:

- implement it as a structured sequence of subphases
- movement is annual, the 52-week loop is event scheduling
- fleet-vs-fleet combat happens during the simulation core, with player-visible
  reports built later from canonicalized events
- hostile world resolution happens after the ticks
- the planet pass (builds, then economy) runs before the ticks

### Step 4 Block Diagram

```text
Validated Durable State
    |
    v
+------------------------------+
| 4a. Prepare workspaces       |
+------------------------------+
    |
    v
+------------------------------+
| 4b. Annual movement update   |
|     (one-time position       |
|      advance for all fleets) |
+------------------------------+
    |
    v
+------------------------------+
| 4c. Pre-move fleet prep      |
|     (friendly merges, input  |
|      sanitization, retarget) |
+------------------------------+
    |
    v
+------------------------------+
| 4d. Annual movement          |
+------------------------------+
    |
+------------------------------+
| 4e. Fleet merge + battle +   |
|     colonization             |
+------------------------------+
    |
    v
+------------------------------+
| 4f. World/player update      |
|     region: autopilot/rogue  |
|     orders and player        |
|     recompute (the planet    |
|     pass ran before 4c)      |
+------------------------------+
    |
    v
+------------------------------+
| 4g. Campaign / conquest      |
|     normalization            |
+------------------------------+
    |
    v
Updated Durable State + Durable Event Pool
```

### What Is Settled

| Point | Practical meaning for Rust |
| --- | --- |
| **Movement is annual, not per-week** | fleet positions are updated once per year. Keep movement as a distinct pre-loop subphase. See [movement-oracle-audit.md](movement-oracle-audit.md): the controlled classic `MoveOnly` traces now match, and the active remaining uncertainty is the exact raw-byte encoding of classic in-transit scratch state rather than the visible movement geometry |
| **Mission resolution requires start-of-year position** | bombard, colonize, invade resolve only when the fleet is at its target at the start of the year. Co-located fleets resolve within the same tick |
| **The 52-week loop is event scheduling, not physics** | the loop schedules encounter detection, combat resolution, and report emission from post-movement positions. Stardates come from timing codes, not physical arrival time |
| **Timing-window constants are recovered** | the scheduler constants are recovered; kind-1 producer assignment is recovered for codes `3..6`; code `7` is the decoder-local `IPBM` timing class; code `8` is an unfed consumer-side case in the preserved image. Only starbase fleets get a delayed producer-side timing offset |
| **Fleet visit order is sort-by-random-priority** | Classic assigns `Random(N)+1` to each fleet as a sort key (extraction: `(seed>>16) % N`), then processes in ascending key order. The Range `N` is dynamic per player. Exact replication requires the full PRNG call chain from validation, which is infeasible. **Rust uses deterministic slot order**, which produces byte-identical results against the oracle for all tested scenarios |
| **Recovered weekly scheduling evidence remains important** | timing codes and weekly report ordering still matter, but current Rust models them through typed events plus canonicalization rather than a literal 52-pass fleet-processing driver |
| **Rust report generation is late projection work** | the current engine records combat/mission outcomes during simulation, canonicalizes them, and builds reports later; weekly ordering is preserved at the event/projection boundary rather than by inline file writes |
| **Fleet-vs-fleet combat triggered by first co-located hostile fleet** | current Rust still resolves local hostile contact after annual movement and before colonization/economy |
| **Some hostile world-resolution paths can destroy stardock contents** | preserved evidence shows at least bombardment-side hostile resolution can remove planet-owned stardock contents on the target world. Rust must model those losses as real planet-state mutation and mirror the matching player-facing turn reports, but should not overclaim the exact stardock-damage mechanics yet |
| **Ready hostile world-resolution family is mission-specific** | replayable oracle probes from the same target world show delayed bombard/invade fleets leave the world unchanged on tick `1`, while ready `BombardWorld` and ready `InvadeWorld` produce different target-world mutation families. Model this as a ready mission-family dispatch in the late hostile world-resolution region |
| **Ready bombard/invade hostile resolution already sees completed builds** | paired oracle probes show a target-world build queue that becomes stardock inventory in delayed control is already consumed before ready `BombardWorld` or ready `InvadeWorld` world resolution. Rust may therefore treat ready hostile world-resolution as reading post-build world state |
| **Fleet destruction is modeled; fleet reassignment is not yet** | destroyed fleets are removed/neutralized by the current engine, but the preserved fleet-battle owner-reassignment behavior is not currently modeled in Rust |
| **Current Rust pre-move setup is narrower than the recovered classic phase** | Rust performs friendly merges and retarget/input normalization before movement; the recovered capture/reassignment setup from the oracle is still unresolved |
| **Colonization is atomic on arrival** | ownership, armies (=1), name, status, production all set in one pass; economy starts the following tick |
| **The planet pass precedes the ticks** | ECMAINT completes builds and settles the economy (`1852:0000`) before the year driver (`0824:1004`); Rust does the same, right before `encounter::run_year` |
| **Recovered classic economy/autopilot gate is `player[0] == 0xFF`** | the currently recovered classic pass applies to rogue-mode empires; civil-disorder `0x00` empires are frozen |
| **Current Rust also does broader post-loop planet/economy recompute work here** | the Rust engine currently places normal planet-economy updates and per-player production/count recomputation in the same post-loop world/player region, even though the oracle-backed classic gate currently only settles the `0xFF` autopilot/rogue pass |
| **File write ordering is stable** | FLEETS → RESULTS → DATABASE → PLAYER → PLANETS → CONQUEST → RANKINGS |
| **Administrative summaries share the weekly event stream** | contact/combat summaries and at least some follow-on administrative consequences belong to one timed event stream, not a detached year-end appendix |
| **`00e8/024d` are yearly producer passes with internal order** | they are part of the yearly producer family, not late report helpers; `f71d -> dddb` runs before `e31b`, and `024d` mixes real planet mutation with durable event production, sometimes silently |
| **The `861d` tail is late report/output work** | treat `1da6 / 0c06 / 2db3 / 56be` as late output/report processing, not as the place to infer gameplay-core phase order |

### Residual Uncertainty

Turn-order placement is now recovered strongly enough for implementation.

Mission/combat behavior is specified separately in
[nc-combat-spec.md](nc-combat-spec.md).
This document is only the turn-order/phase-boundary companion.

## Current Practical Step-4 Shape

The current best implementation shape for step `4` is:

```text
4a. Prepare transient simulation workspaces / start-of-turn snapshots
4b. Advance year
4c. Merge co-located friendly fleets before movement
4d. Sanitize invalid player inputs and refresh retarget/autopilot fleet orders,
    then intercept fleets ordered away from blockaded worlds
4e. Planet pass: build completion (ships and starbases into the stardock,
    armies and batteries onto the planet), then planet economy; then the
    year's ticks (movement, encounters, battles, and planetary strikes:
    bombard / invade / blitz in the tick the fleet reaches orbit)
4f. Mission-fleet merging and local fleet battles
4g. Colonization
4h. World/player update region:
    - autopilot / rogue orders (tax, commissioning, build commands)
    - per-player planet-count / production recomputation
4i. Producer/mutator + campaign region
4j. Join-host updates and conquest-header normalization
```

Key structural evidence:

- **movement is annual**: fleet positions update once per year, not
  per-week. Rust now preserves exact in-transit position between turns and
  rounds only when writing the visible sector coordinates
- the current movement audit in [movement-oracle-audit.md](movement-oracle-audit.md)
  now matches the controlled horizontal / diagonal / shallow / steep
  `MoveOnly` traces and clears `MoveOnly` to `hold` / `speed=0` on arrival
- **the 52-week loop is event scheduling**: stardates come from timing
  codes (+2/+7/+21/+30 week offsets), not from physical arrival time.
  A speed-3 fleet traveling 1 sector shows contact at week 50 (timing-code
  scheduled), not week 19 (physical arrival)
- at least some hostile world-resolution paths can mutate the target world's
  stardock contents, and the corresponding stardock-loss reports belong in the
  same player-visible turn-report stream
- immediate co-located bombard-side hostile resolution already sees target-world
  build completion output in the same yearly tick
- current Rust build completion semantics are explicit:
  - ships and starbases stage into stardock awaiting later commission
  - armies and ground batteries apply directly to the planet and never enter
    stardock
- in current Rust, economy-facing world updates are therefore **before the
  ticks**, not interleaved inside movement/combat resolution
- the remaining hostile world-resolution family is later than the fleet-battle
  step and should not be collapsed into the same “combat” box in
  diagrams
- the flush order: PLAYER → PLANETS → CONQUEST → RANKINGS

## The `024d` Implication For Rust

The most important recent finding is that at least one producer pass is not
just "report prep." It mutates real world state and also contributes durable
events.

Implementation consequence:

- Rust should have a first-class concept of a producer pass that may do both:
  - mutate durable game state
  - append durable event records
- preserve producer ordering inside that family:
  - `f71d -> dddb` kind-1 work happens before later `e31b` kind-2 emission
  - `024d` is not an unordered late helper; it is part of the simulation core

That means the engine should not be shaped like this:

```text
simulate all durable state
then
generate all events from the finished state
then
format reports
```

It should instead allow this:

```text
run subphase
  -> mutate state
  -> emit durable event records

run later subphase
  -> mutate more state
  -> emit more durable event records

canonicalize the resulting event pool
emit reports from the canonicalized pool
```

## Shared Write Ownership Inside Step 4

The current practical evidence is strong enough to guide one more engine rule:

- do not model step `4` as a set of fully disjoint passes where each subphase
  owns a separate slice of world state
- some neighboring subphases appear to touch overlapping target-world state
- in at least some probes, a producer-side branch can overwrite target-world
  changes that also appear in natural hostile-resolution cases

Implementation consequence:

- keep step-`4` world updates ordered and explicit
- avoid hidden mutation from helper calls that makes overwrite order hard to
  audit
- prefer one of these shapes:
  - subphase functions that write directly in known order
  - or subphase-local change sets that are applied in known order

Avoid this shape:

```text
collect a bag of unordered world mutations from many helpers
merge them later with no explicit precedence
```

Prefer this shape:

```text
run hostile-resolution subphase
  -> apply its target-world updates

run producer/mutator subphase
  -> apply its target-world updates
  -> allow documented overwrite where needed
```

That keeps the engine faithful to current evidence without hiding precedence
inside unordered helper effects.

## Weekly Event Stream Implication

Current spec evidence is now strong enough to guide one more implementation
rule:

- contact/combat reporting and at least some administrative follow-on
  consequences share the same weekly event stream
- do not treat Fleet Command Center loss summaries or retarget consequences as
  a detached end-of-year appendix by default

Implementation consequence:

- the event pool and weekly scheduler should be capable of carrying both
  hostile-contact/combat outcomes and later administrative consequences in the
  same timed stream
- when a hostile world-resolution path destroys stardock contents, emit the
  matching player-facing losses through that same event/report path rather
  than as a detached late summary
- standing mission/status families can also recur across years:
  - fleets already in extended orbit commonly emit `orbit-world` again at week
    `1` of later years
  - standing bombardment can resume with an immediate week-`1` bombing report
    after hostile contact when the fleet was already in bombard posture at
    round start

## Concrete Scheduler-Family Summary

This is the Rust-facing summary of the concrete report-family placement rules
already strong enough to encode. For corpus evidence and edge cases, use
[ec-timing-spec.md](ec-timing-spec.md).

| Report family / transition | Observed weekly placement | Rust implication |
| --- | --- | --- |
| `sensor-contact -> identified` | same week consistently in the shipped corpus | emit these as an ordered same-week pair; do not force a week advance between contact and identification |
| `identified -> intercepted` | same week where directly chained | direct interception can continue inside the same weekly batch |
| `entered-system -> attacked` | same week or next week | treat attack timing as a separate hostile-contact event in the shared weekly stream; do not derive it mechanically from system-entry week |
| `identified -> orbit-world` | same source/year gaps `0`, `1`, or `4` in the shipped corpus; the zero-gap cases are all week `1` | treat `extended orbit` as a standing mission/status family; fleets already orbiting at round start may emit a week-`1` orbit report in the same yearly stream |
| `orbit-world -> sensor-contact` | wide-gap periodic same-source family | while extended orbit persists, later `sensor-contact` is an independent weekly-stream event driven by hostile presence/traffic, not by one internal orbit timer |
| `attacked -> bombing-run` | same source/year gaps `0`, `5`, `6`, or `7` in the shipped corpus; the zero-gap case is week `1` | standing bombardment can continue after hostile contact without one fixed delay table; support same-year continuation and the round-start immediate variant |
| `intercepted -> bombing-run` | one direct same-source case at gap `6` | generalize the bombardment continuation rule to hostile encounter during standing bombardment, not only to the literal `attacked` wording |
| `identified -> Fleet Command Center fleet-lost` | same-week cross-source interleaving is common but not universal | treat loss summaries as separate weekly-stream events, not as same-source mission progression |
| `attacked -> Fleet Command Center fleet-lost` | next-week cross-source interleaving in the observed corpus | do not force immediate same-week loss-summary emission after every attack report |
| `fleet-lost -> join-retarget` | same-week cross-source interleaving is observed | administrative retarget consequences can share the same weekly stream as the loss summary |
| `fleet-lost -> planet-bombarded` | same-week cross-source interleaving is observed, but delayed variants also exist | bombard aftermath belongs to the same scheduler stream, but not as one fixed same-source delay rule |

## Target-World Resolution Families

Current oracle evidence now supports a tighter implementation rule than before.

Replayable bombard/invade probes from the same generated target world show:

- delayed `BombardWorld` / `InvadeWorld` fleets leave the target world
  unchanged on tick `1`
- ready `BombardWorld` applies the bombardment damage family
- ready `InvadeWorld` applies a different ground-assault family
- both ready families read post-build planet state

Implementation consequence:

- dispatch these world effects by mission kind in the tick the fleet's
  strike test passes, inside the year's ticks and not in a later pass,
  so each reads the fleet and the world as they stand in that tick
- keep the damage mechanics/report phrasing inside mission-specific helpers,
  but keep the phase boundary itself explicit and shared

## Recommended Driver Skeleton

This is the current recommended engine shape for the Rust maintenance engine.

```text
run_turn(directory):
  gate_result = run_gate_and_recovery(directory)
  if gate_result.skip:
      return

  state = load_and_validate(directory)
  events = create_event_pool()
  snapshots = capture_start_of_turn_state(state)

  advance_year(state)
  merge_co_located_friendly_fleets(state)
  sanitize_invalid_player_inputs(state, events)
  refresh_retargets_and_autopilot_recalls(state, events)
  run_build_completion(state)
  run_planet_economy(state)
  run_year_ticks(state, events)        # movement, encounters, battles, strikes
  merge_mission_fleets(state, events)
  apply_colonization(state, events)

  run_economy_autopilot(state)
  recompute_player_planet_stats(state)

  run_campaign_state_updates(state, snapshots, events)
  update_player_starbase_flags(state)
  apply_join_host_updates(state, events)
  normalize_conquest_headers(state)

  assemble_events(events)
  canonicalize_events(events)
  report_rows = build_report_rows(state, events)
  apply_reviewable_flags(state, report_rows)
  rebuild_output_projections(state, events, report_rows)
  flush_and_cleanup(directory, state, events)
```

Use that as the current implementation skeleton. For combat mechanics, defer to
[nc-combat-spec.md](nc-combat-spec.md).

## Current Rust Driver Snapshot

This is a non-authoritative snapshot of the current refactored Rust maint
driver. Keep it separate from the target turn order above. If the code and the
spec disagree, the spec wins and the code should move. The same is true if this
snapshot drifts from the companion combat, timing, or economics specs.

1. validate structural maintenance state
2. cull empty fleet records and take the start-of-turn snapshots
3. advance the game year
4. merge co-located friendly fleets before movement
5. sanitize invalid player inputs
6. refresh retarget / seek-home / join-host / guard-starbase targets and
   autopilot fleet recalls
7. run the planet pass: settle which worlds take the starbase growth bonus,
   complete builds (ships and starbases into the stardock, armies and
   batteries onto the planet), then run the planet economy
8. run the year's ticks: movement, encounters, fleet battles, and planetary
   strikes
   - if hostile action strips the ship class that makes the mission possible,
     abort the mission immediately
   - invalidated mission fleets seek home when possible, otherwise hold in
     place
   - a Bombard / Invade / Blitz fleet strikes in the tick its strike test
     passes, with the ships and against the world of that tick
9. process mission-fleet merging and join-host updates
10. apply colonization
   - only from current post-combat fleet state; do not execute stale
     pre-combat colonization arrivals after ETAC loss or forced retreat
11. run autopilot / rogue orders (tax, commissioning, build commands)
12. update player starbase flags
13. recompute per-player planet count / production totals
14. apply campaign-state transitions and related player/fleet consequences
15. normalize conquest header fields
16. finalize pending observations and on-station scout reports
17. assemble maintenance events, apply stored diplomatic escalations, and canonicalize

That is the ordering the Rust diagrams should describe when they are trying to
show the current driver, not just the recovered classic outer boundaries.

## Allowed Writes By Phase

This table is the practical guardrail for implementation.

| Phase | May write durable game state | May write durable event pool | May write player-visible reports | May rebuild derived outputs |
| --- | --- | --- | --- | --- |
| Gate | No | No | No | No |
| Recovery | Yes, by restore only | No | No | No |
| Load/validate | No durable gameplay mutation | Validation scratch only | Error outputs only on failure | No |
| Simulation core | Yes | Yes | Usually no direct final report emission | Sometimes indirectly, but prefer deferring |
| Canonicalization | No new gameplay mutation | Yes, normalize existing events | No | No |
| Report emission | No new gameplay mutation | No new core events | Yes | Yes |
| Flush/cleanup | No new gameplay mutation | No | Final file writes only | Final file writes only |

## Report And Timing Model

Rust should treat report generation as a separate consumer of already-produced
events, not as the place where gameplay outcomes are decided.

### Report Flow Diagram

```text
Simulation subphases
    |
    v
Durable event pool
    |
    v
Canonicalize / match / sort
    |
    v
Weekly timeline walk (1..52)
    |
    +--> report blocks / report rows
    +--> message projections
    +--> rankings / database / compat-edge projections
```

Practical rule:

- a state mutation may be real even if it produces no immediate visible report
- conversely, late report helpers are not evidence for gameplay-core ordering

## State-Machine View

```text
Idle
  |
  v
GateChecked
  |
  +--> Skipped
  |
  v
RecoveredOrClean
  |
  v
Validated
  |
  v
Simulating
  |
  v
EventsCanonicalized
  |
  v
ReportsEmitted
  |
  v
Flushed
```

## Implementation Rules

1. Keep validation outside the simulation core.
2. Keep the event pool as a first-class runtime structure.
3. Allow simulation subphases to mutate state and emit events in the same pass.
4. Keep canonicalization separate from event production.
5. Keep report emission separate from gameplay mutation.
6. Keep step `4` subphases explicit and ordered.
7. Treat mission-family timing as data/logic attached to the scheduler, not as
   one universal post-combat delay.
8. Prefer typed event records and explicit phase functions over giant
   cross-cutting mutation code.

## What This Document Does Not Claim

This document does not claim:

- the full mission/combat rulebook; use
  [nc-combat-spec.md](nc-combat-spec.md)
  for that
- the semantic meaning of every still-raw planet/player field
- the original combat RNG or full Pascal-era implementation structure

It does claim:

- the major outer turn-cycle boundaries are strong enough to guide Rust
- **movement is annual** (one-time position update), not per-week
- **the 52-week fleet loop is event scheduling**, not physics simulation
- current Rust models that weekly timing through typed events and
  canonicalization rather than a literal 52-pass fleet-processing driver
- stardates come from timing codes (+2/+7/+21/+30), not physical arrival
- kind-1 timing-code production for codes 3-6 is recovered; code 7 is
  decoder-local `IPBM`, and code 8 is an unfed consumer-side case
- fleet visit order is sort-by-random-priority, with Rust free to use slot
  order pragmatically
- mission resolution requires start-of-year position
- the recovered planet pass (builds, then economy) is before the ticks; the
  rogue/autopilot orders pass precedes it
- colonization is atomic on arrival
- current Rust builds combat reports from the canonicalized event pool after
  simulation rather than as inline file writes
- at least some hostile world-resolution paths can destroy stardock
  contents and must produce matching player-facing turn reports
- ready hostile world-resolution is selected by mission family in the later
  hostile world-resolution region
- build completion precedes ready hostile world-resolution in the recovered
  bombard/invade probes
- producer/mutator passes are part of gameplay state mutation, not just
  report formatting
- the late `861d` tail is output/report oriented, not gameplay-core ordering
- event production, canonicalization, and report emission are distinct
  responsibilities and should stay distinct in Rust

## Relationship To Other Docs

- use [ec-turn-cycle-spec.md](ec-turn-cycle-spec.md)
  for oracle-backed phase evidence
- use [nc-combat-spec.md](nc-combat-spec.md)
  for mission/combat behavior and hostile world-resolution mechanics
- use [economics.md](economics.md)
  for the canonical Rust economy/build policy of the planet pass
- use [ec-timing-spec.md](ec-timing-spec.md)
  for weekly report/timing evidence and `Stardate` report-header shape
- use [approach.md](approach.md)
  for project-level preservation and RE policy
- use [rust-architecture.md](rust-architecture.md)
  for codebase structure and DOD rules
