# ECMAINT Canonical Turn Cycle

This document records the current best recovery of the original
`ECMAINT.EXE` turn cycle.

The goal is to specify the recovered `ECMAINT.EXE` / oracle phase order, not
Rust policy. The reverse-engineering effort is complete for implementation
purposes; where minor gaps remain, this document notes them explicitly instead
of filling with guessed semantics.

For the Rust-native companion that describes the same process as a
runtime-state, simulation, and event-pipeline problem, use
[rust-turn-cycle-implementation.md](rust-turn-cycle-implementation.md).

For the current claim-by-claim Rust conformance snapshot against this spec,
use [ec-turn-cycle-compliance-audit.md](ec-turn-cycle-compliance-audit.md).

## Confidence Levels

- `High`: directly supported by static RE, black-box probes, or both
- `Medium`: strongly suggested by multiple clues, but still missing one key
  link in code or fixtures
- `Low`: useful working hypothesis only; do not promote into Rust as settled

## High-Level Result

Current best model:

1. maintenance schedule / token wait gate
2. `Move.Tok` crash recovery and `.SAV` restore, if needed
3. cross-file integrity validation and file-count/link checks
4. yearly maintenance simulation over the loaded game state
5. late summary canonicalization / sorting
6. report emission across an internal `1..52` weekly timeline
7. final report flush / cleanup / token cleanup

The yearly simulation core (step `4`) is now substantially recovered:

- **movement is annual** — fleet positions are updated once per year
  (one-time advance), not per-week. Fractional travel state is stored in
  the fleet tuple_c field (Real48) for multi-year journeys.
- **the 52-week fleet loop is event scheduling, not physics** — it
  schedules encounter detection, combat resolution, and report emission
  from post-movement positions. Stardates come from timing codes, not
  physical arrival time.
- fleet visit order is **sort-by-random-priority** per game state
  (Borland Pascal LCG confirmed as `$08088405`; the implementation uses
  per-fleet random priorities rather than an in-place shuffle)
- **mission resolution requires start-of-year position** — bombard,
  colonize, invade resolve only when the fleet is at its target at the
  start of the year
- combat reports are emitted **inline** during the weekly loop
- combat is triggered by the first co-located hostile fleet processed
- fleet destruction and capture are dynamic mid-pass
- a **pre-loop fleet setup phase** handles captures/reassignments before
  the 52-week loop (non-combat scenarios skip it entirely)
- the yearly driver (`0824:29c9..29d9`) runs, in this order, the
  CONQUEST previous-year snapshot (`0824:26a5`), the orders pass for rogue
  and autopilot empires (`24a3:3cc6`), the planet pass that completes
  builds and settles the economy (`1852:0000`), and only then the 52-tick
  year driver (`0824:1004`); units finished this year are in the stardock,
  or on the surface, before the first tick
- colonization is **atomic on arrival** (ownership, armies, name, status,
  production all set in one pass)
- **timing-window constants** are fully recovered: 8 codes with fixed
  week offsets (+2/+7/+21/+0/+0/+0/+0/+30), minimum week floors, and
  scheduling priorities. Kind-1 producer assigns codes 3-6 by fleet
  composition (starbase/BS/CA-TT-army/scout-DD).

No material turn-order gaps remain in the yearly simulation core. The
turn-cycle ordering is now closed to implementation depth; mission/combat
behavior is specified separately in
[nc-combat-spec.md](nc-combat-spec.md).

## Practical Rust Consequences

For implementation guidance, use the companion
[rust-turn-cycle-implementation.md](rust-turn-cycle-implementation.md).

Key points:

- movement is annual (pre-loop), not per-week
- the 52-week loop is event scheduling, not physics simulation
- builds complete and the economy settles before the ticks (`1852:0000`
  before `0824:1004`); the rogue and autopilot orders pass precedes both
- the late tail (`8652 → 1da6 / 0c06 / 2db3 / 56be`) is output/report
  generation, not simulation — do not shape Rust gameplay order around it
- the durable event pool (`0x2f72 / 0x2f76`) has two layers:
  - transient validation scratch from `5ee4` (cleared before return)
  - durable entries from `1000:dddb` (kind-1) and `1000:e31b` (kind-2),
    later consumed by the canonicalizer and weekly report scheduler

## Evidence Backbone

This spec is built from four independent evidence sources:

- token and restore-path RE in
  [token-investigation.md](archive/token-investigation.md)
- startup integrity/load RE in [RE_NOTES.md](archive/RE_NOTES.md)
- timing/log analysis in
  [ec-timing-spec.md](ec-timing-spec.md)
- late summary/report pipeline RE in
  `artifacts/ghidra/ecmaint-live/summary-post-canonical.txt` and
  `artifacts/ghidra/ecmaint-live/late-report-pipeline.txt`

## Canonical Phase Order

### 1. Schedule Gate And Token Wait

Confidence: `High`

Settled facts:

- `ECMAINT` has a real schedule/date gate
- the clearest current static anchor is the string cluster at `2000:6fc6`:
  `Today is ... maintenance is not scheduled to run.`
- before doing real work, the engine checks the token-file set
- the master token loop at `2000:997C` walks:
  - `Planets.Tok`
  - `Fleets.Tok`
  - `Player.Tok`
  - `IPBMs.Tok`
  - `Conquest.Tok`
  - `Message.Tok`
  - `Results.Tok`
  - `Database.Tok`
- this is an active wait/cleanup gate, not a passive existence check

Practical meaning:

- maintenance begins with node-coordination and schedule eligibility, before
  simulation or validation

### 2. Crash Recovery From `Move.Tok`

Confidence: `High`

Settled facts:

- `Move.Tok` is the crash marker for a previous run that halted during the
  movement phase
- after the token wait logic, `ECMAINT` checks for `Move.Tok`
- if present, it prints the recovery messages at the block around `2000:6DAA`
- it restores `.SAV` backups over the working `.DAT` files before continuing
- this restore happens before the main integrity check

Practical meaning:

- the engine treats movement as a distinct critical phase
- backup/rollback exists specifically to recover from interruption during that
  phase
- `Move.Tok` is evidence that movement is not just one tiny helper call inside
  a flat pass; it is a named crash boundary in the real engine

### 3. Cross-File Integrity Validation And Input Loading

Confidence: `High`

Settled facts:

- after recovery, `ECMAINT` runs a broad cross-file integrity validator
- helper `0x25EE4` already has recovered passes over:
  - `PLAYER.DAT`
  - `PLANETS.DAT`
  - `FLEETS.DAT`
  - `BASES.DAT`
  - `IPBM.DAT`
- the validator checks linkage/count relationships, not just file lengths
- recovered examples:
  - `PLAYER[0x44]` links into `BASES.DAT`
  - `PLAYER[0x48]` is the `IPBM.DAT` record count
- the nearby startup/status string cluster at `2000:841b..855a`
  (`main.tok`, `Performing integrity check of game files...`,
  `Creating main work file...`, `Merging joint fleets and setting required
  speeds...`) currently has no direct scalar xrefs in the live dump
  - current best interpretation is that this outer startup/status path is
    reached indirectly, likely through a table/pointer-driven emitter

Practical meaning:

- the engine refuses to simulate from structurally inconsistent state
- canonical turn processing starts only after this validation/load stage

Additional static tightening:

- startup helper `2000:9e1e` initializes the shared summary workspace before
  the later phases run:
  - stores a startup time tuple at `0x34fa/0x34fc`
  - zeroes summary count `0x2f76`
  - allocates `0xfa00` bytes through `2000:9b13`
  - stores the resulting far pointer at `0x2f72/0x2f74`

Practical meaning:

- the summary/event table later used by canonicalization and weekly report
  emission is not lazily invented at the end; it is seeded up front in the
  startup/token-side path
- that does not by itself spell out every later yearly subphase, but it
  narrows the boundary between startup plumbing and later summary/report
  processing
- helper `2000:5ee4` now has a firmer internal shape:
  - zeroes `0x16ae`, `0x1714`, and `0x190a`
  - loads `0x3278` records of size `0x6e` into the far-pointer table rooted at
    `0x16ac`, with count byte `0x16ae`
  - loads `0x2f78` records of size `0x61` into the far-pointer table rooted at
    `0x1712`, with count byte `0x1714`
  - then runs the already-recovered summary emitters over:
    - `0x3178` fleet records
    - `0x2ff8` base records
    - `0x31f8` IPBM records
  - finally frees the staged `0x3278` / `0x2f78` buffers before returning

Practical meaning:

- current best reading is that `0x3278` is the player-side staging collection
  and `0x2f78` is the planet-side staging collection
- within the currently recovered `5ee4` body, those collections act as inputs
  to the fleet/base/IPBM validation and summary-emission paths; they are not
  yet supported as separate direct summary producers
- wrapper `2000:6d9b` is now better bounded as restore/validation scaffolding:
  - `arg = 0` jumps into `0x6f20`, calls `5ee4`, and on failure emits
    recovery/error text before recursively re-entering `6d9b` with `arg = 1`
  - `arg = 1` brackets `5ee4` with two `0x3000:4f4c` registration waves over
    the stream anchors rooted at `0x2f78`, `0x2ff8`, `0x3078`, `0x30f8`,
    `0x3178`, `0x31f8`, `0x3278`, `0x32f8`, and `0x3478`
- this strengthens the reading that `6d9b` is integrity/restore framing around
  `5ee4`, not another hidden yearly gameplay-core stage
- `5ee4`'s writes into `0x2f72 / 0x2f76` now look transient rather than final:
  - fleet/base/IPBM branches inside `5ee4` do increment `0x2f76` and allocate
    `0x0c` entry records in the shared workspace
  - but tail `0x6ac3` immediately zeroes `0x2f76` before `5ee4` returns
  - practical reading: those validation-time entries are temporary scratch for
    integrity/link checks, not the durable late-report summary set consumed by
    later canonicalization/coalescing passes
- the first confirmed durable event emitters now sit later in segment `1000`:
  - `1000:dddb` / probe point `1000:e09d` appends kind-`1` `0x0c` entries
  - `1000:e31b` / probe point `1000:e569` appends kind-`2` `0x0c` entries
  - both write owner/coords/common key words into the exact pool later read by
    the `87f4 -> 8b15` matcher/coalescer
- the first recovered ordering between those durable producers is now better
  bounded:
  - sibling drivers `1000:00e8` and `1000:024d` both call `1000:f71d` first
  - `1000:f71d` reaches the kind-`1` writer via `1000:f8a9 -> 1000:dddb`
  - only after that do those same drivers call `1000:e31b` for kind-`2`
    emission
  - practical reading: at least this durable event family is not built by one
    unordered bulk pass; producer order matters before the later matcher pairs
    kind `2` against kind `1`
- this further narrows the gameplay-core evidence search away from the
  already-recovered `5ee4` tail exits

### 4. Yearly Simulation Core

Confidence: `High`

This is the recovered yearly simulation region that matters most for Rust.

Settled facts:

- this stage exists after validation and before late report emission
- it includes a real movement phase
- it includes hostile contact / battle / mission resolution behavior that can
  alter standing orders and fleet destinations
- it is not a single end-of-year text formatter, because later reports reflect
  different intra-year weeks for different outcomes

What is currently settled inside this core:

#### 4.0 The run driver and its yearly loop

Confidence: `High`

Source: the main program of the unpacked ECMAINT (`0824:2946..2a53`,
`tools/unlzexe/ecmaint_640k.bin`), read with capstone (2026-10).

The main program, once past the banner, does the following in order. Each
line names the call, and what the callee does.

| Call site | Callee | What it does |
| --- | --- | --- |
| `0824:294b` | `0824:0049` | Schedule gate: reads the date and refuses to run on a day the schedule excludes ("Today is ... maintenance is not scheduled to run"); 0 ends the run. |
| `0824:2955` | `2895:7d6f` | Token wait, `Move.Tok` recovery, load and validation of the `.DAT` files (sections 1 to 3); 0 ends the run. |
| `0824:2961..2970` | | After a `Move.Tok` restore (`[0x169a]`), steps the game year back by one so the interrupted year runs again. |
| `0824:29a5..29ec` | | The yearly loop, `[0x169e]` times (one for a normal `/R` run). Per year: |
| `0824:29c9` | `0824:26a5` | Copies PLAYER `+0x52` (production) and `+0x50` (planet count) of empires 1 to 25 into CONQUEST `+0x0A` and `+0x3C`, the "previous year" arrays the rankings print in parentheses. |
| `0824:29cc` | `24a3:3cc6` | The orders pass. First `24a3:0c2a` turns a civil-disorder empire with no planets into a rogue (and under `/C`, every civil-disorder empire). Then per empire: a rogue gets AI fleet orders and build commands (`24a3:2a29` with planets, `24a3:0dbc` without); a human empire gets its orders processed (`24a3:2e23`, `24a3:39fd`), its autopilot flag `+0x6d` set when it has missed `[0x2f70]` turns, and autopilot orders and build commands when the flag is set (`24a3:2a5a`). Ships commissioned here become fleets and unit-table entries before the year's ticks. |
| `0824:29d1` | `1852:0000` | The planet pass, one owned world at a time: `+0x5C` status steps 0 to 1 to 2; the starbase growth bonus test (`1852:011b`); build queue slots 1 to 8 move into the stardock, 9 into armies and 10 into batteries, each paid from the treasury (`1852:0206..0449`); unless Scorch Earth is set, growth and tax income (`1852:0457..082e`); the queue is zeroed (`1852:094a`). |
| `0824:29d6` | `0824:1004` | The year driver: sorts the unit table by `Random(0xffdc)` and runs 52 ticks of step, encounter and dispatch for every fleet and starbase ([ec-encounter-spec.md](ec-encounter-spec.md)). Movement, battles, colonization, hostile missions and their RESULTS reports all happen here. |
| `0824:29d9` | | Game year `+1`. |
| `0824:29ee..29fe` | `2895:726e` | When the game has an end year (`[0x42b4]`) and this is it (`[0x192]`): the game-over check ("The game is over and the winner is..."). |
| `0824:2a03` | `0824:1f8d` | Join fleets (order 13) whose host was destroyed hold position and report "In light of the destruction of the ... Fleet, we are holding our current position". |
| `0824:2a06` | `0824:2212` | Guard Starbase fleets (order 4) whose base was destroyed hold position and report the same for the starbase. |
| `0824:2a26..2a30` | `0824:252a`, `0824:25a3` | With the year stepped back to the year just played: "Updating intelligence database": a mode-2 DATABASE.DAT update for every owned world's owner (`2895:2ddc`), then a pass over the unit table that updates each unit's empire on the world in its sector. |
| `0824:2a34` | `0824:1724` | "Packing IPBM database": compacts IPBM.DAT through IPBM.TMP and removes the run's temporary files, the awareness file among them. |
| `0824:2a37` | `0824:1271` | Recounts PLAYER `+0x50` from PLANETS `+0x5D` and recomputes `+0x52` as `Trunc` of the summed present production (`2883:0000`); under `/C` converts civil-disorder empires to rogues. |
| `0824:2a3a` | `0824:1398` | Writes PLAYER.DAT, PLANETS.DAT and CONQUEST.DAT ("Unable to open & save file ..."). FLEETS.DAT and BASES.DAT were written record by record during the ticks. |
| `0824:2a3d` | `3159:0860` | Deletes `Move.Tok`. |
| `0824:2a49` | `0aca:07b5` | When rankings are enabled (`[0x16a1]`): writes RANKINGS.TXT ("Creating player-ranking file"). |
| `0824:2a4e` | `3159:05fd` | Deletes `Database.Tok`, `Planets.Tok`, `Fleets.Tok`, `Player.Tok`, `IPBMs.Tok`, `Conquest.Tok`, `Message.Tok`, `Results.Tok` and `Main.Tok`. |

RESULTS.DAT is written as reports arise: inside the ticks by the encounter
and dispatch handlers, and after them by the two hold-position passes.
MESSAGES.DAT is opened, restored from `Messages.Sav` when needed, and
packed in the loading stage (the strings sit at `2895:0000..01a0`); no call
after the yearly loop touches it. The driver has no cull step of its own:
whatever fleet-record checks exist run inside the load and validation
routine, and the IPBM pack is the only compaction after the loop.

Practical meaning:

- every unit a build finishes this year is in the stardock, or on the
  surface, before the first tick; a bombardment, capture or scout report
  in any tick sees it
- the economy reads the world as the previous year left it: a world
  colonized or captured during the ticks gets its first income the next
  year, and Salvage credits land on top of the year's income
- the rogue and autopilot orders pass reads the state the previous run
  saved, including its hostile strikes and eliminations

#### 4a. Movement is an annual pre-loop phase

Confidence: `High`

- the crash marker is literally `Move.Tok`
- the restore message says the previous maintenance halted during the movement
  phase
- movement updates fleet positions once per year (not per-week)
- fractional travel state is stored in fleet tuple_c (Real48) for multi-year
  journeys
- this is the strongest recovered named phase boundary in the binary so far
- see also 4b (event scheduling model) and 4p (position-first evidence)

#### 4b. The 52-week fleet loop is event scheduling, not physics simulation

Confidence: `High`

Source: stardate analysis, tuple field analysis, and fixture diffs
(Phase C, 2026-03-17).

Key evidence:

- fleet-battle: fleet 8 (speed 3, 1 sector from (9,10) to (10,10))
  produces a contact report at stardate **50/3010**. If movement were
  per-week at speed 3 (arrival at week ~17), the contact would be at
  week ~19 (code 1 offset +2). Week 50 is inconsistent with per-week
  movement — the 52-week loop is scheduling report emission, not
  simulating fleet physics.

- fleet tuple_c field (`+0x19..+0x1E`) is a Borland Pascal Real48 that
  changes from `1.0` to `~0.9999` during movement tick and reverts to
  `1.0` when the mission resolves. Pre/post fixture comparison shows
  tuples are identical — they are **scratch state** used during yearly
  processing, not persistent fleet data.

- `Move.Tok` exists as a separate crash-recovery phase boundary, distinct
  from the 52-week loop.

- PLANETS.DAT is never accessed during the fleet loop — planet-side
  effects are annual, not per-week.

Structural conclusion:

- **movement is an annual position update**, not per-week incremental
  advance. Fleet positions are updated once per year (storing fractional
  travel state in tuple_c for multi-year journeys).
- **the 52-week loop processes encounter detection, combat resolution,
  and report scheduling** from post-movement positions.
- the weekly stardate assigned to each event comes from the timing-code
  system (codes 1-8 with their offsets and minimum-week floors), not
  from the physical arrival time.

Per-fleet-per-week inner body:

1. Read fleet record
2. Check if this fleet has events to emit this week (timing-window test)
3. If co-located hostile: resolve combat + emit RESULTS.DAT inline
4. Update weekly event state in fleet record
5. Write fleet record

This replaces the earlier "4b. Arrival and assault resolution" section
with a stronger structural model.

#### 4c. A real intra-year weekly scheduler participates in outcomes

Confidence: `High`

Black-box proof:

- shipped logs show report stardates in `1..52`
- logs are ordered nondecreasing by `(year, week)`
- the same fleet can generate multiple reports in one year at different weeks
- example from `ec2.txt`:
  - `3rd Fleet` reports at `12/3002`
  - then again at `21/3002`

Practical meaning:

- the yearly simulation core is not modeled as one atomic instant
- the engine assigns event times inside the year

#### 4d. Hostile contact/combat and administrative summaries feed the same timing stream

Confidence: `High`

Evidence:

- same-week bundles are common in historical logs, especially:
  - sensor contact
  - identification
  - interception
- same-week ordering is stable in the corpus, not just co-timestamped noise
  - repeated ordered `sensor contact -> identification` pairs are common
  - longer `sensor contact -> identification -> interception` chains also recur
- adjacent report transitions are dominated by gap `0` and gap `1` weeks
  - this fits a shared ordered weekly event stream
- Fleet Command Center loss summaries are interleaved into the same weekly
  ordering rather than appended as a separate yearly summary
- targeted recurring transitions now show that administrative summaries and
  follow-on mission consequences share that same stream
  - `identified -> fleet-lost` same week: `4x`
  - `attacked -> fleet-lost` next week: `2x`
  - `fleet-lost -> join-retarget` same week: `2x`
  - `fleet-lost -> planet-bombarded` same week: `4x`
  - `intercepted -> planet-bombarded` next week: `3x`

Practical meaning:

- combat/contact/admin summaries are likely produced from one shared event
  stream inside the yearly simulation
- at least some post-combat administrative and retargeting consequences are
  emitted immediately or on the very next weekly tick, not in a detached
  end-of-year appendix

#### 4e. Producer pass internal ordering (partially recovered)

Confidence: `Medium`

What is now settled:

- sibling drivers `1000:00e8` and `1000:024d` belong to the yearly producer
  family, not the late report tail
- both call `1000:f71d` first
- `1000:f71d` reaches the durable kind-`1` writer through
  `1000:f8a9 -> 1000:dddb`
- only after that do those same drivers call `1000:e31b` for durable
  kind-`2` emission
- `1000:024d` continues into the partially recovered `1000:03ff..0d53`
  interior, while `1000:00e8` stops earlier
- practical reading:
  this is not one flat unordered event dump; at least one real producer family
  inside step `4` has internal ordering and sibling specialization

#### 4f. `1000:024d` is a mixed planet-state producer pass

Confidence: `Medium`

Settled structure:

- `1000:024d` starts with the already known producer/event front half:
  - `cce7`
  - `f71d`
  - `d5d2`
  - `b6d8`
  - optional `db04(arg=0x0a)` when current planet `+0x5a > 0`
  - `f2c7`
  - `e31b`
  - `e1c0`
  - `f9ff`
  - `f914`
  - `c025`
  - `c9a0`
  - `fe73`
- it then enters a deeper owned-planet loop at `1000:03ff`
- that interior:
  - iterates staged planets from `0x1712`
  - skips `planet[+0x5d] == 0`
  - advances `planet[+0x5c]` through a small state ladder
  - gates on owner-player state
  - scans durable kind-`2` entries
  - folds `entry[+0x22]` into running planet-side accumulators
  - writes results into `planet[+0x58/+0x5a]`
  - directly transforms planet numeric fields at
    `+0x03/+0x05/+0x07` and `+0x09/+0x0b/+0x0d`
  - branches on `planet[+0x60]`

Practical meaning:

- `024d` is not just a late summary helper
- it is a genuine step-`4` bridge between planet-side state mutation and
  durable event creation
- some of that work is silent:
  - direct oracle probes can change `PLANETS.DAT`, `DATABASE.DAT`, and
    `RANKINGS.TXT`
  - while leaving `RESULTS.DAT`, `MESSAGES.DAT`, and `ERRORS.TXT` empty

#### 4g. Deep `024d` planet mutation is gated by more than ownership

Confidence: `Medium`

Current strongest black-box constraints:

- forcing `planet[+0x60] = 1` on an owned world consistently activates the
  deeper same-world rewrite at `+0x03..+0x0e`
- forcing nearby visible fields without `+0x60` does not reproduce that path:
  - `+0x0e`
  - `+0x58`
  - `+0x5a`
  - simple combinations
- forcing `+0x60 = 1` on an unowned world does not reproduce the broad rewrite
- forcing `+0x5c = 0` or `1` together with `+0x60 = 1` still triggers the
  rewrite and then normalizes `+0x5c` back to `2`

Payload bisect result:

- ownership plus `+0x60 != 0` is still not sufficient by itself
- the branch also depends on a richer established-world payload in
  `planet[+0x03..+0x0d]`
- that prerequisite is not one opaque blob:
  - lower block `+0x03..+0x08` can drive the lower half of the rewrite
  - upper block keyed by `+0x09..+0x0d` can drive the upper half
  - byte `+0x09` alone is already enough to activate the upper-half rewrite
    shape at `+0x09..+0x0e`
  - copying `+0x03..+0x09` together reproduces the full broad rewrite

Practical meaning:

- the current best raw gate is:
  owned world plus `planet[+0x60] != 0` plus an established-world numeric
  payload
- for Rust modeling, treat this as evidence for at least two coupled
  planet-state numeric groups inside step `4`, not one undifferentiated world
  blob

#### 4h. Ready hostile world-resolution family is selected by mission family plus start-of-year readiness

Confidence: `Medium`

Current direct timing evidence:

- in preserved invasion and bombardment pre-fixtures, forcing target-world
  `+0x60 = 1` causes the deep `+0x03..+0x0e` world rewrite to begin on tick
  `1`
- in those same forced probes, `RESULTS.DAT` is still empty on tick `1`
- in a fleet-battle probe, the same kind of world rewrite also lands on tick
  `1`, but `RESULTS.DAT` is non-empty that tick

Practical meaning:

- at least some `024d`-side planet mutation can occur earlier in step `4`
  than later visible mission/combat consequences, especially for delayed
  families like invasion and bombardment
- this is enough to reject the old "producer passes are only late aftermath"
  model
- later sections now tighten the outer-order picture directly

What is now constrained:

- the weekly aftermath delay is not one universal rule across all missions
- current corpus evidence shows:
  - bombardment `attacked -> bombing-run` at gaps `0`, `5`, `6`, and `7`
  - invasion `attacked -> invaded` at gap `7`
  - colonization `attacked -> arrived-target` at gap `2`
  - guard/blockade `arrived-world -> intercepted` at gaps `1`, `6`, `27`, and
    `43`

Current rule:

- do not model combat aftermath in Rust as one uniform post-combat delay; it
  is increasingly clear that mission family matters
- keep the hostile world-resolution distinction in the later mission-family
  pass, not in the weekly fleet-combat loop

Replayable correction:

- the preserved `invade-pre` fixture is not a clean invade-order oracle:
  `FLEET[3].order = 0x0a`
- cleaner replayable generated probes from `fixtures/ecmaint-post/v1.5` with
  copied `ECMAINT.EXE` now bound the selector directly against the same
  target world:
  - delayed `BombardWorld` (`raw[0x19] = 0x81`, one sector away):
    target world unchanged on tick `1`; bombardment family lands on tick `2`
    (`potential 0x87 -> 0x81`, `econ 12 -> 0`, armies/batteries -> `0`)
  - ready `BombardWorld` (`raw[0x19] = 0x80`, already at target):
    the same bombardment family lands on tick `1`
  - delayed `InvadeWorld` (`raw[0x19] = 0x81`, one sector away):
    target world unchanged on tick `1`; invasion family lands on tick `2`
    (`potential 0x87 -> 0x80`, `econ 4 -> 0`, armies/batteries -> `0`)
  - ready `InvadeWorld` (`raw[0x19] = 0x80`, already at target):
    a distinct ground-assault family lands on tick `1`
    (`potential 0x87 -> 0x83`, batteries -> `0`, armies `142 -> 65`,
    owner/status unchanged)
- the older preserved comparison against `fleet-battle-pre` mixed weekly
  fleet-vs-fleet combat with the later hostile world-resolution family and
  should not be used as the selector oracle for target-world aftermath

Practical meaning:

- the selector is the ready hostile mission family, not watched-world
  payload/class
- fleets that are still one sector away / `raw[0x19] = 0x81` do not mutate the
  target world in tick `1`
- ready `BombardWorld` and ready `InvadeWorld` produce different
  target-world aftermath families from the same starting world
- mission family matters, but that distinction lives in the later hostile
  world-resolution region, not inside the weekly fleet-combat loop

#### 4i. The yearly simulation is a weekly fleet-processing loop

Confidence: `High`

Source: enriched file-I/O trace analysis across 6 scenarios (bombard, econ,
fleet-order, fleet-battle, invade, planet-build).

Settled structure:

- the fleet write block inside step `4` consists of **exactly 52 passes**
  through the fleet table in scenarios without fleet destruction (bombard,
  econ, fleet-order, planet-build all show `832 writes / 16 records = 52`)
- each pass reads-then-writes every active fleet record once
- 52 passes = 52 simulated weeks
- the simulation phase always starts at event index `507`, consistent across
  all 6 scenarios tested

Fleet visit order is **data-dependent, not sequential**:

- bombard:     `[11,15,0,10,4,3,2,1,14,5,13,8,7,6,9,12]`
- econ:        `[11,1,4,14,12,8,3,15,0,5,7,6,9,13,2,10]`
- fleet-order: `[6,3,7,1,2,9,13,12,4,5,0,15,10,14,11,8]`
- planet-build: `[15,12,9,4,0,3,7,8,11,14,2,1,13,6,10,5]`
- fleet-battle: `[1,14,7,4,9,8,12,15,0,13,10,5,11,6]` (14 records)
- invade:       not a fixed sequence (variable due to destruction mid-pass)

Fleet visit order is stable within a scenario: all 52 passes visit the
same records in the same order (confirmed by pass-order consistency check
in bombard, econ, fleet-order, and planet-build).

Fleet destruction reduces record count and pass count:

- fleet-battle: 15 records × 49 passes = 735 writes (one fleet absent)
- invade: 15 records × ~45.7 passes = 685 writes (non-integer ratio
  implies fleet destruction mid-pass, reducing write count for later
  passes)

#### 4j. Combat reports are emitted inside the weekly fleet loop, not after

Confidence: `High`

Source: fleet-battle file-I/O trace interleave analysis.

Key finding:

- in the fleet-battle scenario, **RESULTS.DAT writes are interleaved inside
  fleet write pass 7** (events 640-677)
- 11 RESULTS.DAT records (84 bytes each) are written in two bursts:
  - first burst: 5 records (events 648-652)
  - second burst: 6 records (events 663-676), with a seek-read-rewrite
    pattern suggesting report insertion/reordering
- this happens between fleet record writes within the same weekly pass

Practical meaning:

- report generation is not deferred to a separate post-simulation phase
- the weekly fleet-processing loop itself generates combat reports inline
- the Rust engine should allow report emission during fleet processing, not
  only after all fleet passes complete
- the RESULTS.DAT write at pass 7 correlates with the Stardate `1/3` seen
  in the report text (7th weekly pass maps to week 3 via some scheduling
  offset or initialization)

#### 4k. Fleet-battle has a pre-loop fleet setup phase before the 52-week loop

Confidence: `High`

Source: fleet-battle trace pass-by-pass record indices vs non-combat traces.

Non-combat scenarios (bombard, econ, fleet-order, planet-build) all show
exactly **52 passes** with all 16 fleet records from pass 1. Fleet-battle
shows **57 passes**: 5 pre-loop passes with varying fleet subsets, then 52
passes with a stable 14-fleet set.

Pre-loop passes (fleet-battle only):

- pass 1: `[1]` (1 record)
- pass 2: `[1,3]` (2 records)
- pass 3: `[1]` (1 record)
- pass 4: `[1,0]` (2 records)
- pass 5: `[1,11,6]` (3 records)
- pass 6-57: stable 14-record set `[1,14,7,4,9,8,12,15,0,13,10,5,11,6]`

The final pass (57) shows 12 records — fleets 11 and 6 were dropped
(destroyed during combat in pass 7, which is weekly pass 2).

Fleet 2 (empire 1's bombard fleet, speed 3) and fleet 3 (captured in
pre-loop) are absent from the stable 52-week set.

Practical meaning:

- passes 6-57 = exactly 52 weekly passes = the main simulation loop
- passes 1-5 are a **pre-loop fleet setup phase** that handles fleet
  captures, reassignments, and initial fleet-set construction
- this phase only produces fleet writes when there are fleets to
  capture/reassign (non-combat scenarios skip it entirely)
- the "incremental activation" previously attributed to movement arrival
  is better understood as a capture/setup phase that runs before the weekly
  loop, not as gradual fleet awakening during the loop itself

#### 4l. Fleet visit order is determined by PRNG-assigned sort priority

Confidence: `High`

Source: cross-fixture comparison, PRNG reverse-engineering, and
disassembly of the System.Random and fleet-processing functions
(Phase C/D, 2026-03-17).

Four scenarios with identical CONQUEST.DAT, SETUP.DAT, and nearly identical
FLEETS.DAT (only fleet 2's ship counts differ) but different PLANETS.DAT
produce completely different visit orders:

- bombard:     `[11,15,0,10,4,3,2,1,14,5,13,8,7,6,9,12]`
- econ:        `[11,1,4,14,12,8,3,15,0,5,7,6,9,13,2,10]`
- fleet-order: `[6,3,7,1,2,9,13,12,4,5,0,15,10,14,11,8]`
- planet-build:`[15,12,9,4,0,3,7,8,11,14,2,1,13,6,10,5]`

**Mechanism (confirmed from disassembly):**

The visit order is NOT a permutation shuffle. It is a **sort-by-random-
priority**: during fleet loop setup, each fleet is assigned a priority
value `Random(N) + 1` written to fleet record offset `0x08` (a transient
field, zero in both pre- and post-maintenance states). Fleets are then
processed in ascending priority order; ties are broken by slot index
(stable sort). The Range `N` for each `Random(N)` call is dynamic,
read from per-player arrays.

The code was found at segment `16AE:CA50+` in the unpacked memdump.
The assignment uses conditional logic to select between two per-player
arrays for the Range, depending on game state flags.

**PRNG details:**

- LCG: `RandSeed = RandSeed * $08088405 + 1` (standard Borland Pascal)
- `Random(Range)` extraction: `(RandSeed >> 16) % Range` — confirmed by
  disassembly of System.Random at CS:15EA. This uses unsigned 16-bit
  `DIV` (remainder), NOT the TP7 multiply-extract often documented
- `RandSeed` at `DS:0x03A6`, `Randomize` (DOS clock seed) immediately
  after the Random function body
- `RandSeed = 0x000E000E` at the post-load bridge (`96c4`)

The seed at priority-assignment time is the **accumulated RandSeed after
all Random() calls during validation** (step 3). Different planet data
takes different validation branch paths, advancing the PRNG differently,
producing completely different priority assignments.

**Why exhaustive search failed to find a standard shuffle:**

Full 2^32 seed search was run against Fisher-Yates (forward, reverse,
off-by-one), inside-out FY, simple swap, rejection sampling, reservoir
sampling, and sort-by-fixed-key — with 5 extraction variants each
(TP7 multiply, full-32, lo16, mod, hi16mod). All returned no match
because the mechanism is not a permutation shuffle at all: it is a
sort-by-random-key with **dynamic per-fleet Range** and **+1 offset**.

**Practical meaning:**

- the visit order is cosmetic for non-combat scenarios — all fleets see
  the same world state each week regardless of iteration order
- it only affects **combat report ordering** when hostile fleets share a
  sector: the lower-priority fleet triggers combat resolution first
- exact replication would require tracing the full PRNG call chain from
  program start through validation to the priority-assignment point,
  which is infeasible without replicating the entire validation phase
- Rust uses deterministic slot order, which produces byte-identical
  results against the oracle for all tested scenarios

#### 4m. Combat resolution is triggered by the first co-located hostile fleet processed

Confidence: `High`

Source: fleet-battle pass 7 inner event analysis.

When fleet 4 (empire 2, PatrolSector, 100 BS at (10,10)) is processed in
weekly pass 7:

1. read fleet 4
2. read fleet 0 (empire 1, co-located hostile, 50 BS/CA/DD at (10,10))
3. open RESULTS.DAT
4. write 5 report records (84 bytes each)
5. close RESULTS.DAT
6. read fleet 0 again
7. open RESULTS.DAT, read 1 record back, write 7 more records
8. close RESULTS.DAT
9. write fleet 4

The engine reads the opposing fleet's state during the processing fleet's
iteration, resolves combat, emits reports inline, then writes the
processing fleet's updated state. Fleet 0's state update happens later in
the same pass when it reaches its own position in the visit order.

#### 4n. Fleet slots can be reassigned between empires during the simulation

Confidence: `High`

Source: fleet-battle preserved pre/post fixture comparison.

Observed fleet ownership changes:

- fleet 2: empire 1 → empire 2 (reassigned)
- fleet 3: empire 1 → empire 2 (reassigned)
- fleet 8: empire 3 → empire 4 (reassigned)

Fleet 2 (empire 1's bombard fleet) is absent from the entire weekly visit
set (only 14 of 16 fleet slots participate in the 52-pass loop). This
suggests fleet slots that are being reassigned/captured are excluded from
the weekly processing loop, or they are processed through a separate path.

#### 4o. File write ordering is stable across scenarios

Confidence: `High`

Source: cross-scenario first-write ordering comparison.

All 6 scenarios follow the same first-write file ordering:

```
FLEETS.DAT -> [RESULTS.DAT in combat scenarios] -> DATABASE.DAT ->
PLAYER.DAT -> PLANETS.DAT -> CONQUEST.DAT -> RANKINGS.TXT
```

RESULTS.DAT appears only in fleet-battle (the only single-tick scenario
with active combat at the start). The remaining files always appear in
the same order in the flush phase.

The preserved traces also show a stable late close sequence after the
final ranking writes:

```
close RANKINGS.TXT -> close PLAYER.DAT -> close PLANETS.DAT ->
close BASES.DAT -> close IPBM.DAT -> close FLEETS.DAT ->
close DATABASE.DAT
```

So the late file tail is now recovered to the level of exact observed
write/close ordering for the traced scenarios. Raw DOSBox file-I/O logs
also expose a stable token deletion tail, so this cleanup region is now
recovered at the ordering level needed for implementation guidance.

DATABASE.DAT writes correlate with specific planet record indices:

- bombard/econ: planets `[44, 65, 32, 33, 14, 13]`
- fleet-battle: planets `[44, 65, 32, 33, 14]`
- fleet-order: planets `[44, 65, 32, 13, 14]`
- invade: planets `[44, 65, 32, 33, 14]`
- planet-build: planets `[44, 65, 32, 14]`

Planets 44, 65, and 32 appear in every scenario — they are likely
homeworld or structurally significant planets whose database entries are
always refreshed.

#### 4p. Movement is position-first, mission-resolution-next-year

Confidence: `High`

Source: black-box ordering probes across econ, bombard, and colonization
fixtures (Phase C probes, 2026-03-17).

A speed-3 fleet traveling 1 sector updates its position in tick N but does
not resolve its mission (bombardment, colonization) until tick N+1:

- econ fixture: fleet 2 (bombard, speed 3) at (16,13) targeting (15,13)
  - tick 1: location_x changes 16→15, but speed and order remain set
  - tick 2: speed clears to 0, order clears, planet damage applied
- colonization probe: fleet 2 (colonize, speed 3) at (16,13) targeting (18,15)
  - tick 1: location moves to (17,14) — one sector of two
  - tick 2: location reaches (18,15), order clears, planet colonized

Contrast with co-located fleets (fleet-battle fixture): fleets already at
the same sector at the start of a tick resolve combat within that tick.

Practical implication: the 52-week loop processes movement and resolves
missions within the same yearly pass, but a fleet must already be at its
target at the start of the year for its mission to resolve that year.
Position updates within the 52-week loop are visible at end-of-year but
mission resolution uses start-of-year position.

#### 4q. Colonization is atomic on arrival

Confidence: `High`

Source: ETAC colonize probe on econ fixture (Phase C, 2026-03-17).

When a colonize fleet arrives at an unowned planet, all colonization
effects happen in the same tick:

- planet ownership status: 0→2 (homeworld-style)
- planet owner empire: 0→colonizer empire
- planet army count: 0→1
- planet name: updated to "Not Named Yet"
- planet potential_prod_hi: set to 0x81 (129)
- planet stardock/build fields: unchanged

Economy starts on the newly colonized planet in the following tick
(factories_raw initialized on tick 3).

#### 4r. The planet pass gates on ownership, status and Scorch Earth, not on the player mode byte

Confidence: `High`

Source: static RE of `1852:0000` (2026-10), replacing the 2026-03-17
PLAYER byte-0 mutation probes.

The planet pass (`1852:0000`) visits every world with an owner
(`+0x5D != 0`). It reads PLAYER `+0x44` (the base chain, for the
starbase bonus) and `+0x51` (the tax rate), never `+0x00`: a
civil-disorder empire's worlds complete their builds and take growth and
income like anyone else's. The 2026-03-17 probe compared a civil-disorder
empire with the same empire made rogue; the rogue's new armies and dock
changes are what the orders pass (`24a3:3cc6`) adds, since it gives build
commands only to rogue (`0xFF`) and autopilot empires. The gates inside
the planet pass are:

- `+0x5C` status: stepped 0 to 1 to 2 at the top of the pass, and the
  year's income is scaled by 0, 0.5 and 1.0 for the value found
- `+0x60` Scorch Earth: growth and income are replaced by the decay in
  [ec-file-formats.md](ec-file-formats.md); the build queue still completes

#### 4s. The planet pass runs before the year's ticks

Confidence: `High`

Source: the yearly driver (`0824:29c9..29d9`, section 4.0).

`1852:0000` is called at `0824:29d1`; the year driver `0824:1004` at
`0824:29d6`. The pass therefore reads the state the previous run saved and
the orders pass just changed; nothing the ticks do (movement, battles,
colonization, hostile strikes, Salvage) is visible to it.

The 2026-03-17 fleet-battle comparison that read "economy after the loop"
was comparing two fixtures whose starting fleet layout differed, and the
rogue orders pass, which runs before the ticks too, reads that layout. The
difference in armies came from the AI's build commands, which the planet
pass completed in the same run.

#### 4t. Build completion is the first half of the planet pass

Confidence: `High`

Source: static RE of `1852:0206..0449` (2026-10) and the preserved
`planet-build` fixture.

For every owned world, after the status step and the starbase bonus test:

- queue slots 1 to 8 (`+0x22 + 2k`) are added to stardock slots
  (`+0x34 + 4k`), each saturating at 65535
- slot 9 is added to armies (`+0x58`) and slot 10 to batteries (`+0x5A`),
  both saturating
- each addition subtracts the units' cost from the treasury (`+0x09`, a
  real); the queue slot is zeroed as it is paid
- the economy then runs on the same world (4s), and the ten queue slots
  are zeroed again at the end of the world's visit (`1852:094a`)

Practical meaning:

- a unit finished this year is in the stardock, or on the surface, before
  the first tick; a bombardment or capture in any tick, and a scout's
  report, see it
- the `planet-build` fixture (queue `03/01` becoming `stardock_count=03`
  after one run, RESULTS.DAT empty) and the hostile probes that found the
  dock already filled when a ready bombard or invade struck on tick 1 are
  both this placement

### 5. Late Summary Canonicalization And Sort

Confidence: `High`

Recovered structure from
`artifacts/ghidra/ecmaint-live/summary-post-canonical.txt`:

- `0000:1104..123E` performs generic post-processing over summary entries
- it seeds summary word `+0x08`
- it sorts/swaps the 12-byte summary records by that derived key
- `0000:123E..12FD` then emits generic report/header staging

Practical meaning:

- major simulation outcomes are first collapsed into a summary table
- only after that summary table is canonicalized and sorted does the later
  report/timing pipeline consume it
- the post-validate call chain recovered so far now supports that separation
  more strongly:
  - `1da6`, `0c06`, and especially `56be` are increasingly report/message
    oriented rather than core simulation routines
  - `2db3` still looks more like derived-output regeneration than combat or
    movement logic
  - the newly probed helper `33f7` inside `2db3` is tied to
    `Backing up intelligence database...`, which pushes `2db3` even further
    toward derived database handling rather than simulation-core work

### 6. Weekly Report Emission Loop

Confidence: `High`

Recovered structure from
`artifacts/ghidra/ecmaint-live/late-report-pipeline.txt`:

- `0000:127A..1361` runs an outer loop controlled by `[BP-0x2]`
- that loop starts at `1`
- it repeats until `0x34`
- `0x34` is decimal `52`
- inside each outer iteration, `0000:1302..1356` scans active summary entries
- each active entry runs through:
  - `0000:02C0`
  - `1000:a26e`
  - `2000:c057`
  - `1000:0b51`
  - `2000:c057`
- after the `52` boundary, the loop flushes through `0x3000:32e0`

Practical meaning:

- the binary contains a real late-stage `1..52` loop
- this is the first strong static confirmation that the weekly structure is in
  engine code, not only in report text
- current best interpretation:
  - the yearly simulation has already produced canonicalized summary entries
  - the later report pipeline then walks those summaries across a 52-step
    weekly presentation / emission schedule
- the late timing side is now better split into internal layers:
  - `0000:02c0` decodes kind-`1` summary entries through `2000:c067` into
    stack-resident local timing state
  - `1000:9fa1 / 1000:a26e` derives timing windows from a local `0x0a`-byte
    code table using fixed offsets like `+2`, `+7`, `+0x15`, and `+0x1e`
  - `1000:c102 / 1000:9c0e` then score/test the current weekly slot against
    those windows and raise a rejection flag when the candidate falls outside
    the acceptable range

#### 6-table. Recovered timing-window constants

Confidence: `High`

Source: full disassembly of the `1000:a26e` switch-case (2026-03-17).

The scheduler reads a code byte from each `0x0a`-byte timing entry and
applies these fixed parameters:

| Code | Week Offset | Min Week | Priority | Corpus Match |
| ---: | ---: | ---: | ---: | :--- |
| 1 | +2 | ≥10 | 6 (low) | sensor contact |
| 2 | +7 | ≥15 | 5 | fleet identification |
| 3 | +21 | ≥20 | 4 | interception / engagement |
| 4 | +0 | ≥0 | 6 (low) | immediate, low priority |
| 5 | +0 | ≥0 | 5 | immediate, medium priority |
| 6 | +0 | ≥0 | 5 | immediate, medium priority |
| 7 | +0 | ≥0 | 3 (high) | immediate, high priority |
| 8 | +30 | ≥25 | 1 (highest) | unfed consumer-side case in preserved image |

Key relationships:

- contact → identification: +7 − +2 = **5-week gap**
- identification → interception: +21 − +7 = **14-week gap**
- these match the dominant corpus transition gaps of ~5 and ~14 weeks
- codes 4–7 are all immediate (offset +0) with varying priority for
  scheduling conflicts
- code 8 is the latest placement (+30 weeks from base, earliest week 25)
  with highest scheduling priority, but preserved evidence now bounds it as
  an unfed consumer-side case rather than an observed runtime class

The `min_week` floor prevents early events from being scheduled before
their minimum week. The priority value resolves conflicts when multiple
events compete for the same weekly slot (lower number = higher priority).

Important caution:

- this should not be read as a claim that every gameplay mechanic is itself
  simulated week-by-week in the same loop
- what is proven is that the late report/timing stage explicitly iterates over
  `1..52`
Code-to-source mapping (current preserved evidence):

The preserved timing-code scans show that the durable producer-side
`entry[+0x09]` writes in the recovered producer regions come only from
the kind-1 writer at `1000:dddb`, and the only observed values there are
**3, 4, 5, 6**:

| Condition | Code | Meaning |
| :--- | ---: | :--- |
| `source[+0x30] > 0` (ETACs) | 3 | ETAC fleet (+21 weeks) |
| `source[+0x26] > 0` (battleships) | 4 | battleship fleet (immediate) |
| `source[+0x28] > 0` or `[+0x2c] > 0` or `[+0x2e] > 0` (cruisers, empty or loaded transports) | 5 | cruiser/transport fleet (immediate) |
| else (destroyers or scouts) | 6 | light fleet (immediate) |

The source offsets are FLEETS.DAT fields, and the codes are the ship speeds
ETAC 3, BB 4, CA and transports 5, DD 6: this chain is ECMAINT's max-speed
write (`+0x09`) for a fleet it creates (t033). The timing reading below
predates that layout and is unconfirmed.

No scanned kind-1 or kind-2 durable producer writes `1`, `2`, `7`, or
`8` to `entry[+0x09]`.

However, the preserved decoder scan for `0000:02c0` does more than leave
those codes hypothetical:

- it assigns local timing-state value `1` in an earlier decoded branch
- it assigns local timing-state value `7` in the later branch around
  `0000:079a..07da`
- the `a26e` scheduler still carries explicit `7` and `8` cases

So the recovered durable producer mapping is definitive for `3..6`, and
code `7` is now bounded as a real decoder-local class rather than an
unknown producer-side write. The preserved `0000:02c0` dispatch notes
also now tighten its semantic family:

- `0000:02c0` sets code `7` only in the kind-`3` branch
- archived RE already bounds kind `3` as the `IPBM` summary family

So code `7` is now best treated as the decoder-local `IPBM` timing
class. Timing code `8` is now also bounded more tightly:

- whole-image timing-entry scans show consumer-side comparisons against
  `ES:[DI-0x0a] == 8` in helpers such as `1000:a59c`, `1000:b67e`, and
  `1000:c3d5`
- the same scans do **not** show any preserved ES-side writer feeding
  `entry[+0x09] = 8`
- the only newly found `[-0x0a]` writes outside the durable producers are
  SS-local scratch writes at `0000:f203` / `0000:f20d`, not entry-table
  writes

So code `8` is best treated as an unassigned dead consumer case in the
preserved image, not a reachable timing class.

The practical durable producer-side timing system is therefore:

- **Code 3** (+21 weeks, min week 20, priority 4): ETAC fleets
- **Code 4** (+0 weeks, immediate, priority 6): battleship fleets
- **Code 5** (+0 weeks, immediate, priority 5): cruiser/transport fleets
- **Code 6** (+0 weeks, immediate, priority 5): light fleets (scouts/DD only)

Only ETAC fleets get a delayed timing offset. All other fleet types
produce immediate events. The contact→identification→interception report
chain visible in the corpus logs is generated by the `02c0` decoder
from these immediate events, not by separate timing codes.

The kind-2 producer at `1000:e31b` writes `entry[+0x04] = 2` but does not
write `entry[+0x09]` — kind-2 entries use the default (zero-initialized)
code value, which falls through the `a26e` switch without matching any
case (no timing offset applied)

### 6a. Fixed post-validate report/output tail

Confidence: `Medium`

New static anchor:

- a larger driver region around `2000:861d` now shows a fixed late call order
  after successful restore/validation:
  - `2000:1da6`
  - `2000:0c06`
  - `2000:2db3`
  - `2000:56be`
  - conditional `2000:7659` when `0x169a != 0`

What is already clear:

- this is a late-phase tail, not the whole gameplay simulation core
- `2000:56be` is strongly report-oriented:
  - it references mission-report text families including invasion,
    colonization, scouting, seek-home, and starbase/guard-blockade reports
- `2000:0c06` also looks report/output-oriented:
  - it references player-facing starbase/crew loss text
- `2000:2db3` is the strongest current `DATABASE.DAT` rebuild candidate:
  - it sizes work by `planet_count * 100`
  - that matches the already recovered `DATABASE.DAT` `100`-byte slot model

Practical meaning:

- after validation, `ECMAINT` enters a structured late-output tail before the
  explicit weekly `1..52` emission pass completes
- the core simulation evidence sits earlier than this `861d` tail, or partly
  behind helpers that feed it

### 6b. Kind-`2` summary coalescing happens in the late weekly/report side

Confidence: `Medium`

New static tightening from `2000:87f4 -> 2000:8b15`:

- this region iterates over the summary pointer table at `0x2f72` / `0x2f76`
- it selects summary kind `2` entries and scans for matching kind `1` entries
- the structural match keys are currently:
  - owner byte `+0x00`
  - X byte `+0x01`
  - Y byte `+0x02`
  - flag/mode byte `+0x05`
- after that match, it decodes summary payload word `+0x06` through helper
  pair `2000:3f5a` / `2000:3f27`
- matched entries then feed more late text/output helpers rather than obvious
  movement/combat/economy code

Practical meaning:

- this is another late summary coalescing / report-prep stage, not the yearly
  gameplay simulation core itself
- the middle simulation-order evidence is therefore even less likely to live
  in the `861d -> 8b3d` region and more likely earlier in the run or behind
  helpers that populate the summary table before this pass

### 7. Final Flush, Writes, And Cleanup

Confidence: `Medium`

Settled facts:

- after the weekly loop, the pipeline flushes report output
- the observed final file tail is stable across traced scenarios:
  `DATABASE -> PLAYER -> PLANETS -> CONQUEST -> RANKINGS` for first
  writes, then `RANKINGS -> PLAYER -> PLANETS -> BASES -> IPBM ->
  FLEETS -> DATABASE` for the final close sequence
- the observed token deletion order is stable in the traced logs:
  `Move.Tok -> Database.Tok -> Planets.Tok -> Fleets.Tok ->
  Player.Tok -> Conquest.Tok -> Main.Tok`
- token deletion / cleanup exists elsewhere in the run
- `Conquest.Tok` has explicit management code in the live image

## Residual Uncertainty

The canonical turn order is now recovered to implementation depth.

This document no longer carries open turn-order items. Mission/combat rules for
the Rust target belong in
[nc-combat-spec.md](nc-combat-spec.md),
not here.

## Current Working Canonical Spec

This is the tightest oracle-backed statement today:

1. `ECMAINT` first performs schedule/token gating.
2. If `Move.Tok` exists, it restores `.SAV` backups before validation.
3. It validates and loads the linked `.DAT` state.
4. It runs the yearly loop (`0824:29a5..29ec`), once per year to run:
   a. CONQUEST previous-year snapshot: PLAYER `+0x52` and `+0x50` of every
      empire into CONQUEST `+0x0A` and `+0x3C` (`0824:26a5`).
   b. Orders pass (`24a3:3cc6`): civil-disorder empires without planets
      become rogues; rogue and autopilot empires get fleet orders and
      build commands; human orders are processed. Commissioned ships are
      fleets before the ticks.
   c. Planet pass (`1852:0000`), per owned world: `+0x5C` status step,
      starbase bonus test, build queue into stardock, armies and batteries
      with the treasury debited, then growth and tax income (Scorch Earth
      decays instead), then the queue zeroed.
   d. Year driver (`0824:1004`): the unit table sorted by
      `Random(0xffdc)`, then 52 ticks of step, encounter and dispatch for
      every fleet and starbase. Movement, battles, colonization, hostile
      strikes and their RESULTS reports happen here; a fleet's record is
      saved by its dispatch each tick.
   e. Game year `+1`.
   After the loop: the game-over check at the end year (`2895:726e`); Join
   fleets whose host died and Guard Starbase fleets whose base died hold
   position and report it (`0824:1f8d`, `0824:2212`); DATABASE.DAT
   updates for the year just played (`0824:252a`, `0824:25a3`); IPBM pack
   and temporary-file cleanup (`0824:1724`); PLAYER `+0x50`/`+0x52`
   recount (`0824:1271`).
5. Report weeks come from the tick a report arises in (4d) and the
   timing-window constants of section 6 (8 codes with offsets
   +2/+7/+21/+0/+0/+0/+0/+30 and minimum-week floors); the main program
   has no report loop of its own after the year driver.
6. It writes PLAYER.DAT, PLANETS.DAT and CONQUEST.DAT (`0824:1398`),
   deletes `Move.Tok`, writes RANKINGS.TXT when rankings are enabled
   (`0aca:07b5`), and deletes the remaining tokens (`3159:05fd`).

That is the current oracle-backed canonical turn cycle.
