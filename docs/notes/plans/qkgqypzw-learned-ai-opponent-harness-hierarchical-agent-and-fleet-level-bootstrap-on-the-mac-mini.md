---
id: qkgqypzw
date: 2026-10-14
kind: plan
tags: [plan, milestone, ai]
---
# Learned AI opponent: harness, hierarchical agent, and fleet-level bootstrap on the Mac mini

## Scope and exit

The owner wants a neural-network opponent, not an LLM. Its first job is to
fill empty seats while the game has few human players. Its second is balance
testing: finding dominant strategies before players do.

The work runs in two stages:

1. **A hierarchical agent.** A small network makes the strategic choices: tax,
   build mix, sector goals and enemy declarations. A scripted executor turns
   them into fleet orders. The executor's strategy weights are first tuned by
   evolution, so the executor is a usable seat-filler on its own.
2. **A fleet-level agent bootstrapped from it.** The steps are:
   - imitate the executor's orders, conditioned on the strategic goals;
   - correct drift with DAgger, which works because the executor can be asked
     for orders in any state;
   - copy the strategy and value weights across;
   - fine-tune with PPO, held near the imitation policy by a KL penalty that
     fades over time;
   - hand over control one mission family at a time;
   - keep both teachers in the league as frozen opponents.

**Exit:** the fleet-level agent beats the hierarchical agent head to head, and
both can take seats in a game.

The background lives in
[Training an AI opponent](../investigations/lenngda1-training-an-ai-opponent-for-nostrian-conquest.md):
published systems that fit this game, the t025 throughput measurements, and
why the engine's autopilot is no sparring partner. That investigation proposed
an evolution-tuned scripted bot first. This plan folds that step into t154.

The tasks sit in the `+parallel` lane. They never displace the hosting
critical path.

## Hardware and placement

Everything runs on the Mac mini: an M4 Pro with 8 performance and 4 efficiency
cores, a 16-core GPU and 24 GB of memory shared by CPU and GPU. Rust plays the
games on the CPU cores. The learner uses the GPU through MLX or PyTorch's MPS
backend.

Game players write experience to Parquet shards, and the learner reads the
shards. Adding bean-void later (Ryzen 7 260, 16 threads, 15 GB under WSL)
then means adding a second source of shards. t158 decides whether that is
worth it.

Crunch is never used: it hosts the t249 latency measurements.

Long runs need the mini kept awake (`caffeinate`) and frequent checkpoints.

## What the code already provides

Paths are under `rust/`.

- **An in-memory game.** `nc_harness::ScenarioPlay` (`nc-harness/src/play.rs`)
  is `Clone`, with `submit_turn` and `run_maintenance`. Its maintenance seed is
  fixed at 0. `CoreGameData` (`nc-data/src/directory/types.rs`) derives
  `Clone`, so a start state can be built once and cloned, and a game can be
  branched for search.
- **Seeded maps.** `build_scenario` (`nc-harness/src/build.rs`) seeds the map
  from `ScenarioMetadata.seed`. Setup costs more than a hundred maintenance
  turns, so start states should be reused.
- **Orders.** `TurnSubmission` (`nc-data/src/turns/mod.rs`) applies in memory
  through `apply_to`.
  - Validators: `validate_fleet_order_payload` and
    `validate_fleet_player_inputs` (`nc-data/src/mutators_fleet.rs`),
    `validate_planet_player_inputs` (`mutators_planets.rs`),
    `set_player_tax_rate` and `validate_player_diplomacy_inputs`
    (`mutators_player.rs`), and the preflight checks in
    `nc-data/src/turns/apply.rs`.
  - Nothing enumerates legal actions. Legality is checked by trying to apply
    an order.
- **What each empire knows, computed in memory.**
  - `merge_player_intel_from_runtime` (`nc-data/src/intel.rs`), fed by
    `latest_planet_intel_grants_for_viewer`.
  - `CampaignMaintenance::run_turn` (`nc-engine/src/maint/campaign_run.rs`)
    carries the per-viewer intel from turn to turn.
  - `build_player_starmap_projection_from_snapshots` (`nc-data/src/starmap.rs`)
    builds a starmap from that intel.
  - Fleet sightings are held in `SightingMemory`
    (`nc-data/src/model/sighting.rs`).
- **Scripted play.**
  - In nc-fuzz: the `ChoiceSource` trait (`nc-fuzz/src/choice.rs`),
    `Policy::Aggressor`, `plan_turn`, and the action helpers in
    `nc-fuzz/src/policy/actions.rs`.
  - The price-campaign doctrines (`nc-harness/examples/price_campaigns.rs`).
    They read the whole map and so cannot train a fair agent as they stand.
- **Outcome.**
  - `WinnerState` and `campaign_outcome` (`nc-engine/src/maint/campaign/state.rs`).
  - Elimination events.
  - Production and planet rankings (`nc-data/src/directory/economy.rs`).
  - There is no numeric score. Rewards use final rank by production and
    planets, plus the result.

The repository has no Python bindings and no Python project files. Its Python
tools are standalone stdlib scripts. The training code is the first exception:
a `tools/ai/` project with a `pyproject.toml` managed by uv, and a PyO3 crate
built with maturin.

## Task details

### t150

Make the game environment seedable and cheap to restart.

- Add a maintenance-seed parameter to `ScenarioPlay`. Keep the current
  zero-seed behavior as the default for existing callers.
- Build start states once per map seed and clone them for each game.
- Add a parallel runner that plays many games, one per thread, as
  `price_campaigns` does with `thread::scope`.
- Benchmark on the Mac mini: games per hour and turns per second for 2- and
  4-player 18×18 maps, with Aggressor seats.

**Exit:** the benchmark is recorded, and the same map with different
maintenance seeds gives different outcomes.

**Result (2026-10-14):** `ScenarioPlay::with_campaign_seed` sets the seed for
every maintenance turn; the default stays zero. The `selfplay_throughput`
example clones prebuilt start states and runs games on parallel threads.
`nc-fuzz/tests/campaign_seed.rs` shows one seed replays exactly and different
seeds end games differently. On the mini, 12 threads play about 1.5 million
2-player or 565 thousand 4-player 100-turn aggressor games an hour. The full
table is in
[Training an AI opponent](../investigations/lenngda1-training-an-ai-opponent-for-nostrian-conquest.md#self-play-throughput-on-the-mac-mini-measured-2026-10-14-t150).

### t151

Build each seat's observation from what its empire knows, never from the
full state.

The inputs are:

- `merge_player_intel_from_runtime` intel, carried turn to turn;
- `SightingMemory`;
- the empire's own planets, fleets and starbases;
- the year's reports for that empire.

The output is fixed-layout entity arrays with padding masks, one row per
known planet, own fleet, contact and starbase, plus global values.

**Exit:** a test plays seeded games and shows that no seat's observation
holds a planet, fleet or value its empire has not learned.

**Result (2026-10-14):** the new `nc-ai` crate holds the view.

- `SeatMemory` keeps an empire's planet intel, merged each year as campaign
  maintenance merges it.
- It also keeps foreign forces for five years, taken from the scout,
  encounter, battle and starbase-sighting events addressed to that empire.
- `observe` builds five tables of fixed-width rows:
  - a global row: year, player count, tax, totals, and the empire's own enemy
    declarations;
  - one row per world: position is public, and the rest comes from intel
    with known flags and the age of the intel;
  - own fleets;
  - remembered contacts;
  - own starbases.
- Rows are unpadded; t153 pads them into batches.
- `SeatedGame` wraps `ScenarioPlay` and passes each empire's visible hazards
  to maintenance, through the new
  `ScenarioPlay::run_maintenance_with_visible_hazards`.

`nc-ai/tests/observation.rs` plays 4-player aggressor games on three maps for
40 years. Every year it:

- scrambles every record a seat doesn't own (other empires' worlds, fleets,
  starbases, tax and diplomacy) and checks the observation is unchanged;
- scrambles every report addressed to other empires and checks the seat
  learns the same.

Two planted leaks made the tests fail as they should: reading every world's
true armies, and learning other empires' scout reports.

Not yet observed: the build queue and messages. Each still needs a row
design.

### t152

Encode actions as a factored choice:

- per fleet: mission, target, ROE;
- per planet: build;
- per empire: tax and enemy declarations.

Build legal-action masks from the validators listed above. Translate a chosen
action straight into a `TurnSubmission`, with no turn KDL in the loop. KDL
rendering and re-parsing dominate the fuzzer's planning cost.

**Exit:** every action the mask allows is accepted by `apply_to`, and 1,000
seeded games of random masked play submit no rejected order.

**Result (2026-10-14):** `nc-ai/src/action.rs` covers this.

- **`SeatAction`**, by choice index:
  - tax in steps of 5;
  - keep, neutral or enemy toward each empire;
  - per world: no build or one of the nine build units, and a quarter to all
    of the remaining budget;
  - per fleet: keep orders or one of fourteen missions, a target world,
    speed in quarters of maximum, and ROE 0–10.
- **`seat_masks`** draws mission and target legality from
  `fleet_record_supports_mission_code` and `validate_fleet_order_payload`,
  builds from each world's remaining budget, and ROE from `is_support_only`.
  It reads only the empire's own records and public world positions.
- **`decode_action`** builds the `TurnSubmission` directly. Two of the
  empire's own fleets choosing the same colony world is the one conflict a
  static mask cannot rule out. The later fleet's new orders are dropped, and
  `DecodedTurn` lists it.
- **`commission_submission`** commissions every finished ship before the seat
  chooses, so the masks see the fleets that creates.
- **Not offered yet:**
  - Guard a Starbase and Join, which need a base or fleet target;
  - targets in deep space;
  - detach, transfer, and army loading.

`nc-ai/tests/action.rs` plays random masked 4-player games. The ignored
exit test ran 1,000 games of 30 years: 30,000 game-years, 944,270 fleet
orders and 122,941 builds, with no rejected submission. 156 second colonizers
were dropped, and the test asserts that a drop is never anything else.

`masks_reveal_nothing_about_other_empires` scrambles other empires' records,
including who owns every world the seat does not, and checks the masks are
unchanged. A planted leak, letting the colonize mask see a world's true
owner, made it fail.

Random play cost 1.9 ms per game-year for four seats on one bean-void
thread. Mask cost per seat should be measured in t153 once the batch API
exists.

### t153

Expose the environment to Python on the mini.

- A PyO3 crate built with maturin: `reset(seeds)`, `observe()`, `masks()`,
  `step(actions)` and per-seat rewards, over a batch of games, returning numpy
  arrays without copying.
- A Parquet shard writer for experience.
- The `tools/ai/` uv project, with MLX or PyTorch MPS.

**Exit:** a Python random agent plays seeded games on the mini, and a
learner script reads the shards it wrote.

**Result (2026-10-14):** the mini plays and learns end to end.

- **`BatchEnv`** (`nc-ai/src/batch.rs`) steps many seeded games on
  parallel threads.
  - Each seat's observation and masks are padded arrays indexed
    `[game, seat, ...]`, one row per world and per own fleet.
  - Between steps, finished ships are already commissioned, so the
    fleet rows are the fleets the seat orders.
  - `step` checks every choice against the masks before it touches any
    game, so a bad action raises an error and changes nothing.
  - A finished game pays each seat its `MatchResult::win_share`.
- **`nc-ai-py`** (`rust/nc-ai-py`) is the PyO3 module `nc_ai_env`, built by
  maturin.
  - It provides `reset`, `observe`, `masks` and `step`, run with the GIL
    released.
  - Arrays take over the Rust buffers without a copy.
  - It sits outside the `rust/` workspace, so the game never needs Python.
- **`tools/ai/`** is a uv project with PyTorch on MPS: a Parquet shard
  writer, a random agent and a learner. PyTorch was chosen over MLX for the
  ONNX export t158 would need.
  - Shards store entity tables and masks unpadded, one row per seat-year.
  - The target mask is kept only for the chosen mission.

`nc-ai/tests/batch.rs` shows three things:

- executor choices encoded into the batch arrays play the same game as
  `executor_turn` played by hand, on two maps for 25 years;
- random play within the masks ends every game with exactly one win
  shared out;
- out-of-mask and wrongly sized choices are refused without changing any
  game.

`tools/ai/tests/test_shards.py` reads shards back and finds every stored
choice inside its stored mask.

The exit run on the mini:

- 256 four-seat games of 60 years: 15,360 game-years.
- Reset took 0.34 s. Play took 5.7 s, about 2,700 game-years a second
  end to end.
- Wall time per game-year:

| Phase | ms per game-year |
| --- | ---: |
| Rust step, 12 threads | 0.237 |
| observe | 0.029 |
| masks | 0.013 |
| numpy sampling | 0.057 |
| shard writing | 0.037 |

- `seat_masks` cost 45 µs of thread time per seat, 0.18 ms per four-seat
  year. That is about a tenth of t152's 1.9 ms random game-year.
- The single Python thread takes about a third of the wall time. Overlapping
  it with the step is t158's concern.
- The 19 MB shard held 891,945 fleet choices. The learner read it in
  0.3 s and trained on MPS. Its masked mission loss settled at the
  uniform-within-mask baseline (2.2698 against 2.2702), as it should on
  random play.

**macOS 27 linker bug.** Xcode 27's `ld` (ld-27037.1) writes the indirect
symbol table straight before the string pool without padding. With an odd
entry count the pool lands 4 bytes off 8-byte alignment, and macOS 27's
dyld refuses to load the dylib. Whether a build loads therefore depends on
symbol-count parity.

- The fix: `rust/nc-ai-py/macos-link.sh` links with `cc`, then moves the
  pool into the padding before the code signature and re-signs the file.
- The crate's release profile turns off `strip`, which would recreate the
  bad layout.
- Neither `-ld_classic` (ignored), chained fixups, a newer deployment
  target, `strip`, nor rust-lld (it cannot read the macOS 27 SDK) avoids
  the bug.

Not done here:

- The batch has no scripted seats yet. t155's imitation needs the
  executor's choices as labels, and its PPO pool needs executor and
  Aggressor seats beside learned ones.
- Start states are rebuilt on each reset rather than cached.

### t154

Write the scripted executor on the seat view from t151. It takes strategic
goals and issues fleet orders through the nc-fuzz and price-campaign helpers,
rewritten to read only the seat's observation.

Expose about a dozen strategy weights, such as:

- expansion versus defense;
- tax target;
- attack threshold;
- build mix.

Tune them by evolution against Aggressor seats. The tuned executor is the
first usable seat-filler and the teacher for t155 and t157.

**Exit:** the tuned executor beats Autopilot seats and wins at least half its
games against Aggressor, over seeded 4-player games with seats rotated.

**Result (2026-10-14):** `nc-ai/src/executor.rs` holds the executor. It
chooses within `seat_masks` and builds a `SeatAction`, so a learned seat can
imitate it choice for choice. Each year it:

- sets tax;
- declares war after `war_year` years;
- builds each world's first unmet need: armies, batteries, and at home
  ETACs, scouts, starbases and transports, then the preferred hull;
- colonizes the most valuable world not known to be owned, one ETAC fleet
  per world;
- scouts the world it knows least about;
- gathers warships and transports at home by Rendezvous, loads armies there,
  and strikes the best known enemy world at `attack_ratio` times its
  estimated defense.

Two additions to t152's actions support it:

- a per-fleet choice to load armies, decoded so two fleets never draw more
  than a world holds;
- target masks shared between fleets with the same kinds of ships.

The 1,000-game random test still passes: 30,000 game-years, no rejected
submission, and 171 second colonizers dropped.

`nc-ai/tests/executor.rs` shows three things:

- the executor's choices do not change when other empires' records are
  scrambled, including who owns each world;
- it never needs an order dropped;
- it beats Autopilot and Aggressor seats.

`play_match` in `nc-ai/src/arena.rs` scores seat policies by declared winner,
or else by production at the end.

**Baseline is already enough for the exit.** Even the hand-set weights win
every game against three Autopilot or three Aggressor seats. They conquer
every homeworld within 42–57 years, or hold most worlds at year 60. Neither
policy is a real test. Aggressor makes six fuzz-style actions a year, and
Autopilot never expands.

**Tuning therefore ran against the executor itself.** The
`tune_executor` example ran for 40 generations on bean-void, about 8 minutes.
Each generation had 32 candidates, and each candidate played one seat
against three seats of the current champion on 32 fresh maps over 60 years.
A candidate became champion when its win share plus production share
reached 0.6, where 0.5 is even. The champion changed 29 times. Picking the
best of 32 noisy scores favours luck, so the final weights were checked on
256 maps no candidate had played (maps 100000–100255):

| One seat vs three | Win rate | Production share |
| --- | ---: | ---: |
| Tuned vs baseline | 0.717 | 0.394 |
| Baseline vs tuned | 0.000 | 0.111 |
| Tuned vs Aggressor | 1.000 | 0.943 |
| Tuned vs Autopilot | 1.000 | 0.954 |

An even seat would win 0.25. Compared with the baseline, `Weights::TUNED`:

- taxes at 64%;
- keeps 16 armies at home, 10 at each colony and 9 batteries everywhere;
- keeps 6 ETACs and 16 transports;
- builds no starbase;
- favours cruisers and battleships;
- strikes only at 3.4 times a target's estimated defense, with ROE 4;
- declares war at year 10.

On the Mac mini the same tuning run took the same wall time and found the
same champion. The tuner is deterministic. The mini's cores are about twice
as fast, but 16 candidates split unevenly over its 12 cores.

Limits:

- The executor never uses Guard a Starbase, Join, detach or deep-space
  targets.
- It sends every warship in one home strike force.
- Its defense estimates are rough.

The tuned weights may exploit the baseline's habits more than they play
well in general; t156's league ratings are the broader check.

### t155

Train the hierarchical agent. It is a small entity-transformer strategy
network over the executor, with a value head.

1. Imitate the tuned executor's strategic choices.
2. Fine-tune with PPO self-play, against a pool that includes the tuned
   executor and Aggressor.
3. Reward final rank, with a small production-and-planets shaping term that
   fades out during training.

**Exit:** it wins at least 60% of seeded, seat-rotated 4-player games against
the tuned executor and Aggressor.

**Result (2026-10-14): closed at the plateau, by the owner's decision.**

- The strategist reached about twice an even seat against its own
  executor, short of 0.60. Its best checkpoint (run B, iteration 140) wins
  0.539 against three land-grab executors on 1,024 held-out maps, and
  1.000 against Aggressor. It is kept on the mini as
  `~/.local/state/nostrian-conquest/ai/strategist-t155.pt`.
- Choosing only goals, it is limited by the executor that carries them
  out. Longer PPO and Monte Carlo returns did not move it.
- The owner approved moving on: first the executor doctrine tasks
  (t159–t164), then t157's fleet-level agent. Only the fleet-level agent
  goes into real games, so the 0.60 bar now belongs to the t165 readiness
  gate.

**Progress log:** the owner chose goals as the
strategist's actions, and two builds per world per year for escorts.

- **Scripted seats in the batch.** A lineup gives each seat a kind:
  - learned (the whole turn);
  - strategist (goals carried out by the executor);
  - scripted: executor, baseline, Aggressor or Autopilot.

  A batch of scripted seats plays exactly the games `play_match` plays.
- **Goals** (`nc-ai/src/goals.rs`): tax, diplomacy, build posture, a strike
  world (or hold) and a colony world (or auto).
  - The executor's turn is now `execute_goals(plan_goals(..))`. Its
    executed goals reproduce its turn by construction, so `plan_goals`
    gives exact imitation labels.
  - The refactor left 96 recorded `play_match` results unchanged.
  - Goal masks read only the seat's memory. A test scrambles other
    empires' records and checks the masks are unchanged.
- **Commission order.** Strategist seats commission at the start of the
  year, scripted executors at their own turn. Fleet record order differs as
  a result, and maintenance follows it. The effect is negligible: the
  executor's own goals, played as a strategist against three executors,
  win 0.258 against an even 0.25. Against three baselines they win 0.730
  (t154 measured 0.717).
- **Training** (`tools/ai`):
  - `policy.py`: an entity transformer, 3 layers of width 128, with pointer
    heads for strike and colonize.
  - `imitate.py`: online DAgger, 40 rounds of 128 games, about 9 minutes.
    Final accuracy: tax and posture 1.00, diplomacy 0.93, strike 0.87,
    colonize 0.92. Greedy, the student wins 0.230 against three executors.
  - `ppo.py` and `evaluate.py`: PPO and evaluation.
- **MPS shape cache.** MPS compiles a graph for every input shape. Varying
  batch shapes filled 28 GB and crashed the first imitation run. Padding
  batches to multiples of 128 seats and 16 rows holds memory at about
  6 GiB.
- **PPO from the imitation checkpoint** (200 iterations of 128 games, about
  20 s each). Win share of one learner seat against three tuned executors,
  on 256 held-out maps:

  | Iteration | 10 | 40 | 70 | 90 | 110 | 130 | 140 | 150 |
  | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
  | Win share | 0.230 | 0.383 | 0.514 | 0.535 | 0.533 | 0.578 | 0.559 | 0.525 |

  - Against three Aggressors it won every game.
  - It plateaued between 0.50 and 0.58, just short of the exit.
  - Iteration 160 read 0.611 on 256 maps. On 1,024 maps its snapshot won
    0.533 (halves 0.555 and 0.512) against the executor, and 1.000 against
    Aggressor. Single 256-map readings swing by about ±0.05, so the exit
    must be judged on 1,024 maps.
  - The run ended at iteration 200 (readings 0.578, 0.516, 0.543 at
    180–200). On 1,024 maps the final checkpoint won 0.568 against three
    executors (0.546 on a second sampled run), and 1.000 against Aggressor.
    That is more than twice an even seat, short of the 0.60 exit.
  - The checkpoints and log are kept in
    `~/.local/state/nostrian-conquest/ai/` on the mini, as the baseline for
    the land-grab executor.
  - Policy entropy rose after shaping ended (0.97 to 1.29), so the entropy
    bonus should be lower.
- **Land-grab executor.** The owner, the manuals and VBAM all say expansion
  and intelligence come first. The executor now:
  - builds ETACs with same-year escort destroyers, then scouts, before
    garrisons;
  - sends colony fleets at ROE 0;
  - uses lone destroyers as pickets.

  The doctrine and its sources are in
  [v6jegy7f](../investigations/v6jegy7f-strategy-doctrine-for-ai-seats-from-the-manuals-vbam-and-owner-guidance.md).
- **Two builds per world per year** (`BUILD_SLOTS`). The action arrays
  gained a slot axis. t152's 1,000-game random test still passes:
  - 30,000 game-years, 262,838 builds and 1,191,938 fleet orders;
  - no rejected submission;
  - 282 second colonizers dropped.
- **Re-tuned weights.** `tune_executor` ran as in t154: 40 generations
  of 32, about 7 minutes on the mini. The champion changed 36 times.
  `Weights::TUNED`, on held-out maps 100000–100255:

  | One seat vs three | Win rate | Production share |
  | --- | ---: | ---: |
  | Tuned vs baseline | 0.697 | 0.357 |
  | Baseline vs tuned | 0.008 | 0.097 |
  | Tuned vs Aggressor | 1.000 | 0.929 |
  | Tuned vs Autopilot | 1.000 | 0.920 |

  The new weights tax 61%, keep about 5 ETACs and 2.6 scouts, build a
  home starbase, keep 5 batteries, strike at 2.9 times a target's defence
  with ROE 10, and declare war at year 3. The old and new doctrines cannot
  meet in one game, because the old build order is gone. The retrained
  strategist is the comparison. Its executor's own goals win 0.250 against
  three new executors on 1,024 maps.
- **Land-grab retrain.** Imitation on the new teacher ended at strike
  0.88 and colonize 0.91, but diplomacy fell to 0.89. Greedy, the student
  won 0.188. PPO with entropy 0.001 then climbed more slowly than the first
  run: 0.195 at iteration 10, 0.330 at 60, 0.451 at 100 and 0.535 at 180.

  | Checkpoint, 1,024 maps vs three land-grab executors | Win share | Production share |
  | --- | ---: | ---: |
  | Land-grab run, iteration 180 | 0.515 | 0.339 |
  | Land-grab run, iteration 200 | 0.490 | 0.339 |
  | First run's final network | 0.489 | 0.323 |

  Both networks won every game against Aggressor.

  The first run's network fell from 0.568 against the old executor to
  0.489 against the land-grab one. It drives the new executor too, so the
  new teacher is the tougher opponent. The retrained network only matches
  the old one, still short of 0.60. Checkpoints are kept in
  `~/.local/state/nostrian-conquest/ai/landgrab/` on the mini.

- **Continuations from iteration 180.** Both started with shaping off and
  entropy 0.001.
  - **A** kept the settings. It sat near 0.51 and was stopped at
    iteration 47: two PPO runs at once pushed the mini into heavy swap,
    using 13.6 GB of 14 GB, with each process waiting on memory.
  - **B** used Monte Carlo returns (λ = 1, γ = 1). It held the plateau,
    and its entropy rose to 1.6–1.8.

  On 1,024 maps against three land-grab executors:

  | Checkpoint | Win share | Production share |
  | --- | ---: | ---: |
  | B, iteration 80 | 0.503 | 0.346 |
  | B, iteration 140 | 0.539 | 0.350 |
  | B, iteration 200 | 0.532 | 0.354 |
  | A, iteration 40 | 0.497 | 0.343 |

  Run PPO experiments one at a time on the mini.
- **Remaining work toward the exit.** Judge every result on 1,024 maps.
  In order:
  1. Finish the land-grab retrain: imitation, then PPO with entropy 0.001.
  2. If it stays below 0.60, try one change at a time:
     - longer PPO from the best checkpoint;
     - Monte Carlo returns in place of GAE, which suit a final-result
       reward;
     - a small KL penalty toward the imitation policy;
     - two learner seats in more games.
  3. Executor improvements come from the
     [doctrine follow-up plan](kvqg7wv5-executor-doctrine-follow-ups-from-strategy-research.md)
     (t159–t164). Retrain the strategist on the final executor.

### t156

Build the league and evaluation.

- SQLite tables for agents, checkpoints, matches and results.
- TrueSkill ratings with seat rotation.
- Replays stored as a seed plus the order stream. The engine is deterministic,
  so this rebuilds any state.

**Exit:** the rating table can be rebuilt from the stored seeds and orders.

**Result (2026-10-14):**

- **Recording.** `SeatedGame` keeps every submission it applies, from
  learned, strategist and scripted seats alike. `nc_ai::replay` rebuilds a
  game from its map and campaign seeds, its autopilot seats and those
  turns.
  - `nc-ai/tests/replay.rs` replays mixed batch games exactly, also after a
    round trip through turn KDL.
  - The same test shows that a stream missing one turn does not
    reproduce the game.
- **The league.** `tools/ai/league.py` keeps a SQLite store:
  - agents, with a checkpoint's SHA-256;
  - matches, with seeds, result, and the git commit they were played at;
  - seats, with win share, production and rank;
  - replays, as zlib-compressed turn KDL;
  - ratings.

  `play` runs seat-rotated matches on maps from 5,000,000. `verify`
  replays every match, checks each stored result, and rebuilds the rating
  table from the replays.
- **OpenSkill, not TrueSkill.** Ratings use OpenSkill's Plackett-Luce
  model, the same Bayesian approach for free-for-all games, without
  TrueSkill's patent and trademark. Seats rank by win share, then
  production.
- **First league.** 1,024 matches of 60 years among eight agents, four per
  match. `verify` rebuilt all 1,024 and the rating table exactly in 2
  minutes, from a 10.6 MB database. The matches are tagged `9b9f8f0d+dirty`:
  the league code was not yet committed when they ran.

  | Agent | mu − 3σ | mu | Win share |
  | --- | ---: | ---: | ---: |
  | Final strategist | 41.80 | 47.85 | 0.729 |
  | Executor (current) | 30.87 | 36.29 | 0.384 |
  | Final strategist, imitation only | 28.38 | 33.83 | 0.354 |
  | t155 strategist | 26.44 | 31.89 | 0.390 |
  | Classic (t154 executor) | 22.08 | 27.54 | 0.133 |
  | Baseline | 15.81 | 21.49 | 0.036 |
  | Autopilot | −3.58 | 2.89 | 0.000 |
  | Aggressor | −3.81 | 2.77 | 0.000 |

  The league is kept on the mini at
  `~/.local/state/nostrian-conquest/ai/league.db`.

### t157

Bootstrap the fleet-level agent from the hierarchical one.

1. Imitate the executor's fleet orders, conditioned on the strategic goals.
2. Run DAgger: query the executor on states the student reaches.
3. Copy the strategy and value weights across.
4. Fine-tune with PPO and a KL penalty toward the imitation policy that fades
   on a schedule.
5. Hand over control one mission family at a time, starting with scouting
   and colonization. A family moves to the network only after it beats the
   executor head to head.
6. Keep the executor and the hierarchical agent in the league as frozen
   opponents.

**Exit:** the fleet-level agent beats the hierarchical agent head to head,
with the hand-over result logged for each mission family.

**Progress (2026-10-14, in progress):**

- **Hybrid seats.** `Seat::Hybrid(weights, families)` takes goals, plus
  fleet orders from the network for the handed-over families. Families
  follow the executor's branches: ETAC fleets are `Colonize`; scouts and
  one or two lone destroyers are `Scout`; the rest are `Strike`. The
  executor orders the other fleets and does all building.
  `BatchEnv::fleet_labels(goals)` gives the executor's fleet orders for any
  goals: labels in any state, which DAgger needs.
  `nc-ai/tests/hybrid.rs` shows three things:
  - a hybrid fed those labels plays exactly the strategist's game, every
    step;
  - out-of-mask fleet choices are refused;
  - families not handed over stay with the executor.
- **The model** (`tools/ai/fleet_policy.py`). The strategist network gains
  per-fleet heads for mission, target, speed, ROE and load. The target
  points at worlds, conditioned on the chosen mission and on the worlds the
  seat's goals name. The agent starts from the final strategist's weights
  (step 3).
- **Imitation with DAgger** (`fleet_imitate.py`, steps 1–2). 40 rounds of
  128 games, about 10 minutes. A KL term toward the frozen strategist
  holds the goal heads; it ended at 0.16. Final accuracy: mission 0.89,
  target 0.86, ROE and speed 1.00, load 0.997.
- **Starting point,** one hybrid seat against three final strategists on
  1,024 maps (an even seat is 0.25):

  | Families handed to the network | Sampled | Greedy |
  | --- | ---: | ---: |
  | None (goals only) | 0.232 | 0.252 |
  | Colonize and scout | 0.161 | 0.156 |
  | All | 0.068 | 0.163 |

  The goals survive imitation. Fleet orders at about 88% agreement with
  the executor still cost a lot: misplaced ETACs compound over decades.
- **PPO** (`fleet_ppo.py`, steps 4–5). The objective covers the goal heads
  and the handed-over fleets' heads, with a KL penalty toward the imitation
  policy that fades over 100 iterations.
  - **First attempt collapsed** to zero wins by iteration 3. The penalty
    used log p(now) − log p(imitation) on the actions already taken, which
    is not a KL for actions the current policy did not sample. Driving
    those actions' probability toward zero minimised it.
  - **Fix:** the exact KL between each head's current and imitation
    distributions on the same states. Masked logits are zeroed before
    subtracting, since −∞ − −∞ gives NaN, and NaN poisons gradients even
    through an untaken `torch.where` branch. The learning rate is
    3 × 10⁻⁵.
  - **Stage A** (colonize and scout handed over), 200 iterations. On 256
    maps every 20 iterations, one seat against three final strategists
    read 0.184, 0.229, 0.203, 0.227, 0.285, 0.203, 0.242, 0.225, 0.258
    and 0.248. Against three executors it rose from 0.363 to 0.545 at
    iteration 140. `kl_ref` grew past 1.2 once the penalty faded out at
    iteration 100.
  - **On 1,024 maps against three final strategists:**

    | Snapshot | Sampled | Greedy |
    | --- | ---: | ---: |
    | Iteration 100 | 0.223 | |
    | Iteration 140 | 0.254 | 0.187 |
    | Iteration 180 | 0.255 | 0.230 |
    | Iteration 200 | 0.240 | |

  - **Parity, not a win.** PPO won back what imitation lost (0.16 up to
    0.255), but the network's colonization and scouting only match the
    executor's. They fall short of the 0.28 hand-over bar, so those
    families stay with the executor for now. Checkpoints are kept in
    `~/.local/state/nostrian-conquest/ai/fleet/`.
  - **Stage B** (strike handed over), from the imitation checkpoint, KL
    fading to a 0.02 floor, 200 iterations. On 256 maps every 20
    iterations, one seat against three final strategists read 0.262,
    0.266, 0.312, 0.344, 0.430, 0.449, 0.453, 0.484, 0.453 and 0.502.
    Against three executors it peaked at 0.707.
  - **The world changed during the run.** The executor gained t178's
    intel, scaled garrisons and re-tuned weights. Every seat in a head to
    head uses the current executor: the learner's builds and non-strike
    fleets, and the strategist opponents throughout. On 1,024 maps against
    three final strategists, with the current executor, the stage B
    snapshots won 0.358 (iteration 160), 0.326 (180) and 0.383 (200).
  - **The strike family passes the hand-over.** The network's strike
    orders beat the executor's, well above 0.28. Colonize and scout stay
    with the executor.
- **Final runs (2026-10-15)**, on the final executor after t170–t175
  (blockader on; Guard a Starbase, Detach and deep-space waypoints in the
  actions) and from t175's strategist with the allocation goal
  (`strategist-alloc-final.pt`).
  - **Imitation,** 40 rounds. Final agreement: mission 0.87, target 0.89,
    waypoint 1.00, speed 1.00, ROE 1.00, load 0.996, detach 0.975. The
    goal KL ended at 0.21.
  - **Stage B PPO,** strike handed over, KL fading to a 0.02 floor, 200
    iterations at about 28 s each. On 256 maps every 20 iterations, one
    seat against three of the new strategists read 0.188, 0.277, 0.273,
    0.281, 0.275, 0.285, 0.322, 0.307, 0.375 and 0.344. Against three
    executors the readings rose from 0.40 to 0.63.
  - **On 1,024 maps** (block 100000 unless noted):

    | Snapshot | vs 3 strategists | vs 3 executors |
    | --- | ---: | ---: |
    | Iteration 160, sampled | 0.330 | 0.540 |
    | Iteration 180, sampled | 0.334 | 0.541 |
    | Iteration 200, sampled | 0.345 | 0.580 |
    | Iteration 200, sampled, block 200000 | 0.356 | 0.558 |
    | Iteration 200, greedy | 0.379 | 0.674 |
    | Iteration 200, greedy, block 200000 | 0.381 | 0.672 |

    The same network with no families handed over (goals only) won 0.264
    against three strategists. Iteration 200 won every game against
    Aggressor and Autopilot, sampled and greedy.
  - **Kept:** iteration 200 as
    `~/.local/state/nostrian-conquest/ai/fleet-final.pt`, played greedy
    with the strike family handed over. Colonize and scout stay with the
    executor (stage A's parity). Greedy, it clears the readiness gate's
    strength bar against executors (0.60) on both blocks. The league
    rating and the soundness checks are t165's.
  - Checkpoints are in `~/.local/state/nostrian-conquest/ai/fleet-final/`.

### t158

Measure whether the game players keep the GPU busy during t157's self-play.

Add bean-void as a second actor only if both hold:

- the GPU waits on games more than half the time;
- a training run takes longer than about a day.

bean-void would run the policy on its CPU through ONNX, write shards, and
rsync them to the mini, with a limit on how stale its policy may be.

**Exit:** utilisation is measured, and the decision is recorded here either
way.

**Result (2026-10-15): bean-void does not join.** `fleet_ppo.py` now logs
`env_s`, the seconds spent in the games themselves: observing, masking,
labelling and stepping. Over t157's final stage B run (200 iterations,
5,696 s, 28.5 s an iteration):

- **Games:** 284 s, 5.0% of the run.
- **Rollout:** 1,394 s, 24.5%. This is the games plus the networks
  choosing moves.
- **The rest:** 75%, the PPO update and evaluations on the GPU.

So the GPU waits on games far less than half the time. A whole training
run takes about 1.6 hours, not a day. Neither condition holds. A second
actor on bean-void's CPU would speed up the 5% of the run spent in games,
not the rest. The strategist's PPO run (t175) also spent 39% of its time
in rollouts, network moves included.

### t165

Put the finished AI into real games.

Everything before this task plays in-memory copies of games. The owner
chose (2026-10-14) to wait for the strong agent rather than seat the
scripted executor early. Only the fleet-level agent (t157) goes into real
games, and only once it passes the readiness gate below. The owner decided
on the strength bar and on which agent to release.

**Readiness gate.** Every part must pass. The owner's playtest verdict
decides.

1. **Strength against scripts**, measured on 1,024 held-out maps:
   - one seat against three tuned executors, at least 0.60 win share;
   - top of the t156 league ratings, above the strategist and every
     earlier checkpoint;
   - never loses to Aggressor or Autopilot seats.
2. **Soundness:**
   - thousands of full games with no rejected order and no crash;
   - no engine exploit it relies on (any found is fixed as a bug);
   - the fog-of-war tests still pass for its seat view.
3. **Human play.** The owner and a few testers play full games against
   it. It must be a real challenge, and its play must look sensible: it
   expands early, defends what it holds, attacks with purpose, and makes no
   absurd moves. A short written checklist records each game, and the
   owner signs off.
4. **Operations:**
   - it runs on the game server without Python or a GPU;
   - it finishes a turn within a few seconds;
   - a sysop can assign AI seats and choose from at least two difficulty
     levels.

The owner's playtests (part 3) are of the agent to be shipped. So t182
(promote the sweep-trained agent if it passes) and t183 (a main
exploiter's check for exploitable weaknesses, part of soundness) come
first; t165 depends on both.

Parts 1 and 2 come before any real-game work. Parts 3 and 4 are this
task's work:

- **AI seats in a campaign.** A sysop marks a seat as AI. Before each
  maintenance the seat submits a `TurnSubmission`, as any player does.
- **The seat's memory from stored records.** Rebuild its `SeatMemory` from
  the campaign's stored intel, sightings and reports, not the simulator's
  memory, and check it matches what t151 builds in play.
- **The network on the server.** Export the chosen agent to ONNX and run it
  from Rust, so hosts need no Python and no GPU. Offer difficulty through
  checkpoint and sampling temperature.

**Exit:** a hosted test campaign plays to its end with AI seats beside a
human seat. Every AI submission is accepted, and the AI's view of its
world matches the in-memory view on the same seeds.

**Progress (2026-10-15): gate parts 1 and 2.** The candidate is t157's
final agent, `fleet-final.pt`. It plays greedy, with the strike family
handed over and t175's strategist goals.

- **Strength.**
  - Against three tuned executors on 1,024 maps it won 0.674 (block
    100000) and 0.672 (block 200000), above the 0.60 bar.
  - Against Aggressor and Autopilot it won every game.
  - In the league it rates first, after 2,048 more matches:

    | Agent | Rating | Win share |
    | --- | ---: | ---: |
    | fleet (t157 final) | 43.47 | 0.708 |
    | allocation (t175 strategist) | 38.28 | 0.505 |
    | final (t155 strategist) | 38.23 | 0.536 |
    | final-imitation | 30.31 | 0.287 |
    | executor | 29.89 | 0.298 |
    | t155 | 29.07 | 0.335 |

    `league.py` gained a `fleet` agent kind for this. Strategist
    checkpoints from before t175 load with an allocation head that always
    chooses 0, as they played.
- **Soundness.**
  - Every evaluation and league game ran to its end. That is over 10,000
    full games for this agent, with no rejected order and no crash: a
    choice outside the masks or a rejected submission raises.
  - The fog-of-war tests (mask and observation leak tests) pass.
  - Exploits are left to the playtests.
- **The league's old replays.** `league.py verify` failed on 237 of the
  first 1,024 league matches. They were played at commit 9b9f8f0d, and
  later engine rule changes now reject some of their stored orders (fleet
  speeds, fleet ownership after record changes). Every match played from
  this commit on replays exactly. So stored turn KDL stays valid only
  within one engine version. A league that must verify across engine
  changes needs to store that version, or prune old matches.

**Progress (2026-10-15): the network in Rust.**

- **Export.** `tools/ai/export_onnx.py` exports a fleet-level agent to
  ONNX (opset 18, torch's dynamo exporter, with dynamic world, fleet,
  contact and starbase counts). The graph takes one seat's raw
  observation tables and masks, plus Gumbel noise per head. It returns
  each head's choice; the fleet target is chosen after the mission, as in
  training. ONNX Runtime matched PyTorch on 64 recorded seats, greedy and
  sampled. `fleet-final.onnx` is 3 MB.
- **Running it.** The `nc-ai-onnx` crate runs the graph through tract
  0.22, which is pure Rust. tract 0.23 needs `rand_core` 0.10, which
  conflicts with russh's pinned release candidate.
  - `NetworkSeat::turn` builds the inputs from `observe`, `seat_masks` and
    `goal_masks`. It hands the strike family's fleet orders to the
    network, and has the executor carry out the goals for everything else.
  - The test `nc-ai-onnx/tests/seat.rs` replays a game that a small seeded
    network played in Python. Every year, the Rust seat's turn must equal
    the recorded one.
- **The real model in Rust**, one seat against three tuned executors,
  1,024 maps from 100000:

  | Temperature | Win share |
  | --- | ---: |
  | 0 (greedy) | 0.674 |
  | 0.5 | 0.666 |
  | 1 | 0.590 |
  | 2 | 0.194 |

  The greedy result equals the Python agent's 0.674 exactly. That needed
  the batch environment's order: the network seat commissions at the start
  of the year, and executors at their own turns.
- **Speed.** A turn takes 2 ms on average and at most 37 ms.
- **Difficulty** can come from temperature: greedy for hard, 1 for normal
  (still above the executor), 2 for easy.

**Progress (2026-10-15): AI seats in campaigns (gate part 4).**

- **Setting seats.** `nc-sysop settings ai-seat --dir D --player N
  --difficulty easy|normal|hard` and `remove-ai-seat` set them. They are
  stored in `ncgame.db`'s `ai_seats` table. A reserved seat cannot be an
  AI seat.
- **Playing them.** Before each maintenance year, every AI seat that may
  still play does the following:
  - The first time it plays, it joins its empire as "AI Empire N". A new
    game's seats wait in civil disorder on autopilot until joined.
  - It commissions its ships.
  - It submits the network's turn through `apply_to_runtime_state`, as a
    player's turn is applied.

  After the year it reads its reports. Its `SeatMemory` is saved as JSON
  in `ai_seat_memory`. A memory saved for another year starts over from
  the stored planet intel.
- **Where it runs.** `nc-sysop maint` and `nc-host`'s Nostr and community
  maintenance all play AI seats with the agent named by `NC_AI_MODEL`.
  Community joins skip AI seats.
- **Memory deviation.** The seat's memory is kept, not rebuilt from stored
  records: contacts are not stored anywhere else, and the stored intel
  lacks what the seat learns from its own attack reports (t178).
- **The exit's view check.** `nc-ai-onnx/tests/campaign.rs` advances a
  campaign whose seats are all AI, one stored maintenance run a year, for
  20 years. Every year the campaign's game equals the in-memory
  simulator's on the same seeds (report record ranges aside). Each seat's
  restored memory equals the simulator's.
- **A full campaign.** `nc-sysop new-game` with four seats, seats 2–4
  hard AI and seat 1 never joined, ran from 3000 to the engine's year
  limit, 3100. Every AI turn was accepted. Seat 1's homeworld fell by
  3020. By 3100 the AI seats held 13, 6 and 1 worlds.
  - A first run stalled, its world counts unchanged from 3030 to 3100. The
    AI seats had never joined their empires, so the engine's autopilot
    overrode their orders. Joining on first play fixed it.

**Remaining:** gate part 3, the owner's playtests. The owner and a few
testers play full games against AI seats; a short checklist records each
game, and the owner signs off. To play: create a campaign with
`nc-sysop new-game`, mark seats with `settings ai-seat`, play your own seat
in `nc-game`, and run `NC_AI_MODEL=fleet-final.onnx nc-sysop maint`. The
model is on the mini at `~/.local/state/nostrian-conquest/ai/fleet-final.onnx`;
`export_onnx.py` rebuilds it from `fleet-final.pt`.

### t180

Let games run past year 3100.

The fleet agent sweeps the map in only 3 of 1,024 games when games stop at
60 years. Given 100 years, it sweeps in 205 of 1,024, in year 87 on
average. But Rust maintenance refused any year past 3100. That was a guess
in `nc-data`'s preflight ("a reasonable range … based on game context"),
not a rule; neither the manuals nor the original docs set a last year.

**Result (2026-10-15): fixed.** The real ECMAINT was run in DOSBox-X on
crunch. It advanced a 4-player game and a 25-player game, both brought to
year 3098 by Rust maintenance, through 3099, 3100, 3101 and 3102. Every
year wrote rankings, and no year wrote `ERRORS.TXT`. The preflight now
refuses only years before 3000, and a test runs maintenance from 3100 to
3101.

A game whose year was edited by hand (3001 to 3099) is refused by ECMAINT
before it writes anything. That is its consistency check, not a year
limit: other records no longer match the year.

### t182

Promote the 100-year sweep-trained agent.

A fine-tune of `fleet-final.pt` on 100-year games, where sweeping the map
pays 1 and a production lead at the limit pays 0.5 (t180), running in
`~/.local/state/nostrian-conquest/ai/fleet-sweep/`. The behaviour report
on its snapshots, against the current agent (100-year games):

| Snapshot | Maps swept | Win share |
| --- | ---: | ---: |
| Current agent | 22% | 0.88 |
| 80 | 50% | 0.945 |
| 100 | 63% | 0.926 |

It shows no new pathologies so far.

**Exit:** the best snapshot beats the current agent on 1,024 maps, at 60
and 100 years. Its behaviour report shows no regression, and it holds its
own against t183's exploiter. It is then exported as `fleet-final.onnx`.

**Result (2026-10-16): promoted snapshot 140.** On 1,024 maps against
the previous agent, with three executors in the other seats:

| Model | 60 years | Maps swept (60) | 100 years | Maps swept (100) |
| --- | ---: | ---: | ---: | ---: |
| Previous `fleet-final` | 0.657 | 1 | 0.868 | 182 |
| Snapshot 140 | 0.774 | 20 | 0.927 | 613 |

The 256-game behaviour report shows no new pathology: the same tax,
no declared wars, fewer worlds lost (15.4 against 17.2), more worlds at
the end (17.5 against 13.8), and fewer transports built (85 against 115).
Against t183's exploiter it scores 0.260 where its own control scores
0.237 (1,024 games, 60 years), so the hole the exploiter finds in the
previous agent is not there (t192 checks it properly). The previous agent
is kept beside it as `fleet-final-pre-sweep.pt` and `.onnx`, and
`fleet-final.pt` and `.onnx` are now snapshot 140. `cargo test -p nc-ai
-p nc-ai-onnx` passes.

### t183

Train a main exploiter against the deployed agent.

Adversaries trained against KataGo with under 5% of its compute won most
games by inducing blunders, and patched defences fell to new adversaries
(see [lenngda1](../investigations/lenngda1-training-an-ai-opponent-for-nostrian-conquest.md)).
Win share against our bots cannot show such holes.

**The method.** `fleet_ppo.py --exploit PATH` trains an agent that meets
only three frozen, greedy copies of the target, with no KL leash. It
started from the target itself.

**Exit:**
- the exploiter's win share against the target is measured;
- if it clearly beats 0.25, its games are read and the weakness named;
- the weakness is then fixed, in the executor or by training the agent
  against the exploiter (t184).

**Result (2026-10-16).** 100 iterations of 128 60-year games (20 on
bean-void, then 80 on crunch's RX 7900 under ROCm, from the bean-void
iteration-20 checkpoint), `--kl 0 --entropy-coef 0.003`. The exploiter's
win share against three copies of `fleet-final`, 128 games per check:

| Iteration | 10 | 20 | 30 | 40 | 50 | 60 | 70 | 80 | 90 | 100 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Win share | 0.211 | 0.242 | 0.234 | 0.219 | 0.316 | 0.289 | 0.266 | 0.250 | 0.227 | 0.270 |

An even seat is 0.25, and a 128-game check is uncertain by about ±0.04.
The iteration-50 peak fell back and the rest sit at an even share, so the
exploiter found no reliable weakness and there is none to name. This is
a small exploiter: the KataGo adversaries trained far longer, so t184's
league remains the guard against holes a larger search would find.

**Correction (2026-10-16): the exploiter did find a weakness.** The
table above scored the exploiter by sampling its choices during training,
which understates it. Scored greedy, as the target is deployed, on 1,024
60-year games against three copies of the target (the control is the
target itself in the learner's seat):

| Learner | Target | Win share |
| --- | --- | ---: |
| exploiter, iteration 80 of the crunch run | `fleet-final` | 0.377 |
| `fleet-final` (control) | `fleet-final` | 0.237 |
| exploiter, iteration 80 | sweep snapshot 140 | 0.260 |
| sweep snapshot 140 (control) | sweep snapshot 140 | 0.237 |

At 256 games the three saved exploiter snapshots scored 0.309, 0.336 and
0.371 against `fleet-final`, rising with training. So `fleet-final` has a
real hole (0.38 against an even 0.25), and the sweep-trained snapshot 140,
which the exploiter never met in training, does not show it (0.26 against
a 0.24 control, within noise). That favours promoting snapshot 140, but an
exploiter trained against snapshot 140 itself is the proper test (t184).
t192 names the weakness.

### t184

Choose training opponents from the league by difficulty.

AlphaStar's prioritized fictitious self-play samples opponents in
proportion to how often they beat the learner. Training today draws from
a fixed mix: strategist, executor and aggressor.

**Exit:** `fleet_ppo.py` draws opponents from the league, weighted toward
those the learner loses to. The pool includes t183's exploiter and past
snapshots. A run trained this way beats one trained on the fixed mix,
both in league ratings and against the exploiter.

### t185

Start some training games from saved mid-game positions.

DAGS (2026) and SACL start self-play episodes from intermediate states,
which speeds learning in imperfect-information games. DAGS warns this
biases the result unless normal starts are mixed in. Our late-game
weaknesses (t179, t181: about 17 worlds lost per 100-year game) appear
only in states the training rarely reaches.

**Exit:**
- `BatchEnv` can start a game from a stored position, such as a league
  replay at year 25–40, with each seat's memory rebuilt;
- training mixes such starts with normal ones;
- the late-game measures in the behaviour report improve without
  hurting the early game.

### t186

Track the win-probability estimate through games.

CivBench estimates each player's chance of winning from turn-level state,
because the final result is too sparse in long games. Our value head
already estimates it.

**Exit:** a tool plots the value estimate year by year for evaluation and
league games, and lists the years where it fell most, with the game
state. Those positions feed t185 now and t166's loss mining later.

**Against the critical path (2026-10-15).**

- **None of t182–t186 gates the critical path.** That path is the
  community hosting launch, t143–t121, and these stay `+parallel`.
- **Before t165:** t182 and t183, so that the owner playtests the agent to
  be shipped.
- **After t165:** t184–t186. t186 feeds t166, which also waits for t121's
  hosted pilot.

Not yet a task: deciding strategy only on events (a target taken, a fleet
lost, a new contact) instead of every year, from Black & Darken's options
framework. It is speculative, so it waits for t182–t186.

### t167

Give AI seats personalities.

The owner (2026-10-14) wants the land grab as the default AI style, with
other styles as personalities, starting with a rusher. Twice, tuning the
executor against itself found a rush that beats the land-grab weights
between executors:

- **Rush 1** (t162 tuning, with front scouting): beats the land-grab
  weights 0.266 and 0.295 with the switch off.

  | Tax | Home armies | Colony armies | Batteries | ETACs | Scouts | Starbases | Transports | BB | CA | DD | Attack ratio | ROE | War year |
  | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
  | 65.0 | 19.1 | 4.7 | 7.3 | 1.04 | 0 | 0 | 15.8 | 0.27 | 1.0 | 0.10 | 0.61 | 6.1 | 1.13 |

- **Rush 2** (t164 tuning, with scaled garrisons): beats them 0.326 and
  0.309 with the switch off.

  | Tax | Home armies | Colony armies | Batteries | ETACs | Scouts | Starbases | Transports | BB | CA | DD | Attack ratio | ROE | War year |
  | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
  | 59.8 | 19.6 | 7.5 | 6.8 | 2.97 | 0.06 | 0.14 | 16.0 | 0.21 | 0.84 | 0.81 | 0.67 | 5.7 | 1.88 |

The work:

- **Named executor styles:** land grab (the default, `Weights::TUNED`),
  rusher, and others found by tuning within bounds (for example a turtle).
  Each is tuned within its own bounds, so a style stays recognisable
  instead of drifting to whatever beats its own kind.
- **Learned seats keep the style.** A strategist drives an executor of
  that style, and is trained or fine-tuned with it, so the network does
  not undo the personality.
- **In real games:** a sysop chooses each AI seat's personality, beside its
  difficulty (t165). Seats can mix styles so a game has variety.

**Lessons from EC4X.** The owner's earlier game `~/dev/ec4x` had 12 rule-based
AI personalities. Its rule-based AI was deleted in December 2025, but its
records survive in `docs/archive/`:

- **Twelve traits.** Each personality was a vector of six traits from 0
  to 1, e.g. aggression and expansion (`2025-12-04-ai-docs/PERSONALITIES.md`).
  Traits shifted budgets set by the game's act, and gated behaviours by
  thresholds (`2025-12-10/ai/ARCHITECTURE.md`).
- **Only four were ever tested:** 292 twelve-player games
  (`2025-12-10/ai/budget/12-strategy-balance-report.md`).
  - Turtle and Economic competed (16.7% and 16.1% wins, against 8.3% by
    chance); Aggressive (0.6%) and Balanced (0%) failed.
  - The other eight, and the matchups the doc described, were never
    measured.
- **The Balanced bug.** Balanced built no ground batteries in 75% of games
  despite a large treasury. No trait reached the defence budget
  (`2025-12-10/ai/analysis/CRITICAL_BALANCED_STRATEGY_BUG.md`).
- **Threshold cliffs.** An aggression ≥ 0.5 gate blocked most
  personalities from invading at all.
- **Scouts.** Winners built far fewer scouts, as NC's tuning found.

**Personalities as risk-and-return preferences (owner, 2026-10-15).** Each
executor rule weighs risk, reward and return (see the
[doctrine plan](kvqg7wv5-executor-doctrine-follow-ups-from-strategy-research.md)).
A personality is a coherent set of preferences over those same
trade-offs:

| Trade-off | Rusher | Land grab (default) | Turtle | Raider |
| --- | --- | --- | --- | --- |
| Risk tolerance (odds needed to attack) | High | Moderate | Low | High, against weak targets only |
| Time horizon | Short | Long | Long | Short |
| Fleet held for defence (t175) | Little | Balanced | Much | Little, with hunters out |
| Attack mission (t168) | Blitz | Bombard, then invade | Invade after full suppression | Blitz; hit-and-run bombardment |
| Intelligence (t174 pickets) | Few | Moderate | Dense net | To find targets |
| Expansion | Minimal | Maximal early | Steady | Moderate |

Learned seats keep their personality through training:

- **The reward:** a cautious seat is penalised for losses; a rusher
  discounts the future more.
- **The executor:** its rules use the personality's preferences.

So a learned seat plays its character well, rather than just playing to
win.

**What follows for NC:**

- **A personality is a named profile:** `Weights` and `Doctrine`
  overrides, authored as typed KDL. There is no separate trait layer, the
  indirection where EC4X's Balanced bug hid. Every profile must set every
  weight; no knob may be left untouched.
- **No midpoint "balanced" profile.** Each profile is a distinct plan.
- **First profiles.**
  - Land grab (default).
  - Rusher: early war, few ETACs, strikes below a target's estimated
    defence (attack ratio about 0.6).
  - Turtle and economic as later candidates, since only they competed in
    EC4X.
- **No thresholds.** Prefer smooth, capped differences to switches that
  turn a doctrine off for a whole profile.
- **Tune within a profile.** Its identity weights stay fixed or bounded,
  so a land-grab seat cannot drift into the rush.
- **The strategist decides when to fight; the personality biases how.**
  This follows EC4X's later move from traits to its planner.
- **Measure every pairing with seats rotated** before claiming a matchup,
  and lock win-rate bands into regression tests. Ship only profiles that
  have been measured.

**Exit:**

- Each personality plays recognisably: its build mix, war timing and
  expansion rate, measured over 1,024 games.
- Each holds its own in a mixed league (t156): it never falls far below
  the default.
- The owner approves the styles in playtests.

**Progress (2026-10-15).**

- **Profiles.** `nc-ai/personalities/{land-grab,rusher,turtle}.kdl`, parsed
  by `nc_ai::personality`. Each sets every weight and doctrine switch once.
  Each also sets bounds for its identity weights:
  - land grab: ETACs 2.5–8, war year 12–30;
  - rusher: ETACs 0–3, attack ratio 0.5–1, war year 0–3;
  - turtle: colony armies and batteries 8–10, starbases 2–3, attack ratio
    3–4, war year 24–30.

  A test checks that `land-grab` equals `Weights::TUNED`, and that a
  profile missing a knob, setting one twice, or leaving its bounds is
  refused.
- **Tuning within bounds.** `tune_executor --personality NAME
  --against-default` tunes a style against three default seats, inside its
  bounds: 40 generations of 32. On 1,024 maps, one seat against three land
  grabs:
  - the rusher went from 0.195 (t164's rush weights) to 0.212;
  - the turtle went from 0.129 (hand-set) to 0.194.
- **Each style plays recognisably** (`style_stats`, 1,024 games, one seat
  against three land grabs):

  | Style | Win share | First war | First attack | Build mix |
  | --- | ---: | --- | ---: | --- |
  | Land grab | 0.242 | year 21.0, in 584 games | year 14.5 | starbases 2%, batteries 20% |
  | Rusher | 0.212 | year 2.0, every game | year 8.6 | destroyers 4%, batteries 16% |
  | Turtle | 0.194 | year 30.0, in 149 games | year 13.9 | starbases 4%, ETACs 1%, armies 9% |

  The turtle also holds fleet back through t175's allocation rule.
  Expansion is about five worlds at years 10, 20 and 40 for every style,
  because these are small 20-world maps.
- **Mixed league** (2,048 more matches): the default executor rated 31.15,
  the turtle 30.48 and the rusher 30.14. Neither falls far below the
  default.
- **Learned seats keep the style.** The fleet agent never sets a war goal:
  it strikes without declaring war. So a learned seat's style comes from
  its executor, and from goal bounds in the profile (`goals { allocation
  LO HI }`), which `NetworkSeat` applies to the network's goals. The
  rusher strikes with everything (allocation 0). The turtle always guards
  (allocation 1–2). The Rust seat, one against three executors on 1,024
  maps, greedy:

  | Style | Win share | First attack | Worlds at year 20 | Defence share of builds |
  | --- | ---: | ---: | ---: | ---: |
  | Land grab | 0.674 | year 12.8 | 5.4 | 35% |
  | Rusher | 0.616 | year 10.9 | 5.6 | 26% |
  | Turtle | 0.604 | year 13.2 | 5.4 | 35% |

  The network leaves the turtle closest to the default. Its difference is
  that it always guards. No per-style fine-tune was run: PPO rewards
  winning, not keeping a style, and the goal bounds keep the style
  directly.
- **In real games**, `nc-sysop settings ai-seat` takes `--personality`,
  stored with each AI seat. `CampaignAi` loads one network seat per
  personality in use, and seats can mix styles.

**Remaining:** the owner approves the styles in playtests.

### t166

Learn from real games once AI seats play people.

After t165, every hosted game with an AI seat is a seed plus each seat's
submitted orders. The engine is deterministic, so any game can be replayed
exactly, and each empire's fog-correct view rebuilt year by year, as t156's
replays do.

- **Mine the AI's losses.** Find the years where its estimated chance of
  winning fell most, rebuild those positions in the simulator, and train on
  them. This is the main use: a few hundred human games against millions of
  simulated ones point at weaknesses better than they fuel training.
- **A human-style model.** Imitate human turns to build a model of how
  people play. Use it as a league opponent, and anchor the AI toward
  human-like play with a piKL-style penalty (Diplodocus). It also gives a
  natural difficulty setting.
- **Human ratings.** Rate the AI on its games against people, beside its
  league rating against bots.
- **Consent.** Before any player's turns are used for training, the
  community rules must say so. This is the owner's decision.
- **Prerequisites.**
  - t165 must pass.
  - The hosted community must be running: t121's private pilot, on the
    critical path.
  - t186's win-probability trace, which finds the losses to mine.

**Exit:** one loop completed. Human games are collected with consent and
the AI's losses mined. A retrained agent passes the t165 readiness gate
again, and wins more often against the human-style model than the agent it
replaces.

### t189

Use the full order set and a mixed fleet in the executor.

The owner's review (2026-10-16): the AI must use every ship order a player
has, and build battleships, destroyers and scouts, not only cruisers. The
behaviour report's new order rows show, over 128 100-year games, that both
`fleet-final` and the sweep snapshot use about 5 of the 16 missions:
Rendezvous (about 1,000 a game, all at its own worlds, none in deep space),
Bombard, Invade, View and Colonize, with Guard/Blockade about once. No
Join, Transfer or Unload Armies. About 2 fleets attack a world, holding
30% of the seat's warship strength, and the largest fleet holds 45%.

`preferred_hull` builds only the affordable hull with the largest weight,
so the tuned weights (cruiser 0.59, battleship 0.38, destroyer 0) buy
cruisers and nothing else, and the personalities' different hull weights
make no difference. The executor builds for the network seat too, so a
change here reaches the deployed agent at once.

The owner's doctrine for massing:
- **Rendezvous** is for a planned assault: several large fleets meet at a
  staging sector, in deep space short of the target, and strike together
  once the assembled force is `attack_ratio` times the target's defence.
- **Join Another Fleet** is for a force already on the move: new ships and
  rear fleets join it to replenish or consolidate it.

The changes, each a `Doctrine` switch measured head to head as t159 does,
kept only if it doesn't lose:
1. **Mixed hulls.** Build each hull toward its weight's share of the
   fleet, so every personality's mix shows. Scouts and lone destroyers
   as pickets on Patrol or Scout a Sector between the seat's worlds and its
   nearest rivals, feeding t181's exposure.
2. **Staged assaults and reinforcement.** Rendezvous in deep space short
   of the target, strike together; Join for new and rear fleets.
3. **The rest of the order set** where it applies: Blitz when a world's
   batteries are down, Guard a Starbase, Seek Home and Salvage for wrecked
   fleets, Transfer and Unload Armies to move garrisons.

If a mixed fleet plays measurably worse than cruisers alone, that is a
balance finding about ship prices and goes to the owner, not a reason to
keep the cruiser fleet.

**Exit:** the behaviour report shows the executor using most of the order
set, staging in deep space and building every hull, and the changed
executor beats or matches its predecessor.

**Mixed hulls, first trial (2026-10-16): rejected as it stands.**
`Doctrine::mixed_hulls` builds each hull toward its weight's share of the
fleet by cost. Paired against the tuned executor, 512 60-year games each
(`executor_matches --with mixed_hulls --paired --first-map 200000`):

| Personality | With | Without | Difference |
| --- | ---: | ---: | ---: |
| land-grab | 0.159 | 0.234 | -0.075 ± 0.023 |
| rusher | 0.170 | 0.294 | -0.124 ± 0.025 |
| turtle | 0.178 | 0.221 | -0.043 ± 0.023 |

Per point, the hulls fire alike (AS 2 a point) and battleships are a
little tougher (DS 2.2 a point, cruisers 2.0, destroyers 1.6), so value
alone doesn't explain the loss. A fleet moves at its slowest ship's
speed: one battleship (speed 4) slows a cruiser force (speed 5) by a fifth,
and destroyers are the weakest in a slugging match. Mixing the hulls in
one force is the mistake; they need roles. Battleships form a heavy group
that stages by Rendezvous so the slow ships arrive together (step 2),
destroyers raid, screen and picket at speed 6, cruisers hold the line. The
switch stays off until it is retried with roles.

**Reinforcement by Join, first trial (2026-10-16): rejected as it stands.**
`FleetChoice` gains a `join` host (the decoder emits a Join turn action, so
the plumbing t190 needs exists but has no test of its own yet), and
`Doctrine::reinforce` sends warships at home to Join the nearest strike
force under way instead of gathering a second force. Paired, tuned
executor, 256-512 60-year games (`--with reinforce`):

| Join rule | Difference in win share |
| --- | ---: |
| any strike force | -0.186 ± 0.020 |
| within 10 sectors | -0.188 ± 0.028 |
| within 5 sectors | -0.121 ± 0.028 |
| joiner at least as big as the host | -0.088 ± 0.029 |
| only a force short of its target's need (kept) | -0.061 ± 0.020 |

Every rule loses, and the more it restricts joining the less it loses.
Likely cause: one growing stack conquers one world at a time, where
gathering at home launches several forces at several worlds, so conquest
runs in parallel and the cycle from home to target and back is shorter.
Replenishing a force that is short of its target is the least harmful
case, and still loses. Before retrying, measure the parallel-conquest
hypothesis directly: worlds taken per year while the home force is split
and while it is stacked.

### t190

Give the network the whole order set.

The network cannot order Join Another Fleet (mission 13), or the turn
actions Join, Transfer and Unload Armies, because they need a fleet target
and none is offered (`nc-ai/src/action.rs`). Add a target that points at
the seat's own fleets, with masks from the game's validators, to the
model, the ONNX export and the Rust decoder.

**Exit:** a test shows each new order chosen, masked and submitted
legally, and the exported model plays with them.

### t191

Retrain the agent on the new executor.

The network learned its fleet orders by imitating the executor (t157). It
learns t189's doctrine and t190's orders the same way: imitation with
DAgger from the current weights, then PPO on 100-year games (t182), on
crunch when Codex has no quiet hold.

**Exit:** the retrained agent beats the deployed one on 1,024 maps, and
its behaviour report shows the wider order set and the mixed fleet.

### t192

Name the weakness the exploiter found.

t183's correction above: the exploiter beats `fleet-final` 0.377 to 0.237.
Play its games against the target and read them (`game_log` with the
exploiter and the target in the network seats, or `behaviour_report` rows
for the exploiter against the executors), then name what it does that the
target does not answer: an opening, a timing, a world it takes. Check
whether sweep snapshot 140 answers it, and train a fresh exploiter against
snapshot 140 (the same `--exploit` run, about 10-15 s an iteration on the
RX 7900).

**Exit:** the weakness is named and shown to be fixed or absent in the
agent to be promoted, or fixed by training against the exploiter (t184).

### t193

Correct the evaluation flaws the Muse review found.

Fix before citing new strength numbers:
- the league plays fleet agents greedy but strategists sampled
  (`league.py:209` against `:222`), which flatters the fleet agent;
- PPO evaluation reuses maps 100000-100255 every 20 iterations, and the
  final 1,024-map confirmations include them; rotate evaluation blocks and
  keep one untouched;
- `behaviour_report` prints no paired difference or interval between
  models;
- imitation accuracy is teacher-forced (`fleet_imitate.py:127`).

**Exit:** the league is re-rated with both greedy, evaluation maps are held
out, and the report prints paired differences with their standard errors.

### t194

Save AI seat memory with the campaign, and check hazard intel.

`nc-sysop` saves AI seat memory in a store call after `maintenance.save`
(`main.rs:458-462`); a crash between them silently resets the memory
through the year-mismatch fallback (`campaign.rs:87-107`). Save them
together, or log the fallback. Also check that the campaign's hazard
routing, which uses stored intel, and the simulator, which uses
`SeatMemory` with attack intel, agree after attacks.

**Exit:** a test crashes between the two saves and the seat's memory
survives or the fallback is logged; the two paths agree after an attack.

### t187

Stretch goal: AI empires send in-character dispatches.

A language model turns an AI empire's turn into a short message in its
personality's voice, such as "Empire 2 masses near our border; we fortify
Rigel". It states only what the turn and the empire's reports support. It
makes no decisions, so the agent's strength is unchanged.

- **Hosting.** Turns are yearly, so a small quantized model (3-8B, via
  llama.cpp) on the host's CPU is fast enough, or a hosted API. No GPU is
  needed. If the model is unavailable, the turn still goes in without the
  dispatch.
- **Fairness.** Its input is the empire's own view (the same rule as the
  [LLM player guide](../../dev/llm-player-guide.md)), so a dispatch cannot
  leak what the empire hasn't seen.

**Exit:** in playtests, AI empires send dispatches that match their actual
moves, and the owner finds them worth keeping.

### t188

Stretch goal: AI empires that can be negotiated with.

Today an AI seat neither reads nor sends diplomatic messages. Meta's Cicero
reached human-level play at Diplomacy by splitting the work: a strategy
model chose moves and intents, and a language model talked to players,
conditioned on those intents. Here:

- a language model reads players' messages into structured signals (peace
  proposed, threat against a world, alliance against an empire);
- the agent takes those signals as inputs, and is retrained against bots
  that send them and keep or break their word;
- the language model writes replies consistent with what the agent will
  actually do.

The language model never chooses moves: the trained agent plays far better
than one, and overriding it would invalidate its training. The retraining
is the large part of this task.

**Exit:** in playtests, a player can propose peace or an alliance to an AI
empire, get a reply that matches its later conduct, and the agent's
strength against the bots doesn't drop.

## Cold-eye review (Muse Code, 2026-10-16)

Muse reviewed the AI work read-only on crunch (full report in
`~/muse-review-findings.md` there). It found no fog-of-war leak; the
observation, masks, goal masks, contacts and attack intel all trace to
player-visible sources, the exact-KL PPO fix and GAE are correct, and
Python and Rust classify families the same way. Its findings, with what
this plan does about each:

| # | Finding | Disposition |
| --- | --- | --- |
| 1 | AI seats break maintenance in a non-4-player campaign (the graph's diplomacy inputs are fixed at four) | Fixed: `CampaignAi::load` and `settings ai-seat` refuse such campaigns |
| 2 | No Join, Transfer or Unload in the action space | t190, covering all three, not Join alone |
| 3 | Strikes are piecemeal: yearly commissioning makes separate fleets and `can_take` judges each alone | t189: gate on the combined home strength and give every home fleet the same target and speed (replaces the rejected Join trial) |
| 4 | Cruiser-only builds; a scout weight of 0.14 rounds to 0, so scouts are never replaced | t189: roles for hulls, and round positive scout and ETAC targets up |
| 5 | Pickets recalled when the land grab ends | t189: keep Patrol and View stations |
| 6 | Diplomacy has no learning signal | t188; until then the head is inert |
| 7 | PPO ratio, KL and entropy include heads the decoder ignores | t191: a per-(mission, head) relevance mask |
| 8-9, 12 | League greedy against sampled; reused evaluation maps; thin evaluation | t193 |
| 10 | Memory saved in a separate call | t194 |
| 11, 13 | Rejected switches left in `Doctrine`; duplicated example loops; legacy t153 path | After t189 decides: remove the dead switches and merge the example loops |

## Risks

- **A leaky view.** If an observation shows anything the empire hasn't
  learned, the agent learns to cheat and its play won't carry over to real
  games. t151's test guards this.
- **Reward farming.** The shaping term fades out, and the final rank decides.
- **Engine exploits.** A learning agent finds bugs fast. Keep the fuzzer's
  invariants running during training, and treat any exploit as a bug report.
- **Strong but unfun.** Pure self-play can produce odd play. Judge
  seat-fillers in human playtests as well as by rating, and offer difficulty
  through checkpoint strength or sampling temperature.
- **The mini sleeping or overheating.** Run under `caffeinate` and checkpoint
  often.
- **New dependencies.** The PyO3 crate and the Python project are the
  repository's first. Keep them out of the default workspace build, so nobody
  needs Python to build the game.

## Connections

- [Training an AI opponent](../investigations/lenngda1-training-an-ai-opponent-for-nostrian-conquest.md):
  background, published systems and t025 throughput.
- [Engine fuzzer plan](lski78er-engine-fuzzer-with-shared-policies-and-invariants.md):
  the choice source, policies and invariants reused here.
- [LLM player guide](../../dev/llm-player-guide.md): the rule that a bot
  sees only what a human player sees.
