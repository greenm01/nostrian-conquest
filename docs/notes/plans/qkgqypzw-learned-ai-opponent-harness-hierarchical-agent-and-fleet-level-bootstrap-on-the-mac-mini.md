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

**Signed off (owner, 2026-10-18).** The owner signed off the readiness gate
without the recorded playtests. t191 and t193 stay open as improvements to
the seated agent, not blockers.

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

**League run (2026-10-17): it loses to the fixed mix.** `fleet_ppo.py
--league` ran the t202 retrain's recipe (from `fleet-final-pre-t202`,
100-year games, `--limit-credit 0.5`, `--kl-floor 0.02`, 200 iterations,
`fleet-league/ppo`). The league held the seven scripted styles, the
previous two agents, the fixed-mix snapshot 200 and the learner's last
three snapshots. The first attempt crashed at iteration 75 on a join-mask
bug (fixed in `ec6ceabe`); the run that followed saved `resume.pt` each
iteration. Against three executors on 1,024 maps from 100000, paired with
the fixed-mix snapshot 200 (now `fleet-final`):

| League snapshot | 60 years | 100 years |
|---|---|---|
| 200 | 0.460 (-0.086 +/- 0.019) | 0.683 (-0.056 +/- 0.018) |
| 160 | 0.478 (-0.069 +/- 0.019) | 0.644 (-0.095 +/- 0.018) |

Both still beat the previous agent (0.355 and 0.498). Their behaviour
differs little from snapshot 200's (more bombarding fleet-years, a few more
worlds lost). In training, the learner's running win share against
snapshot 200 ended at 0.113, against 0.25 for an even game. The league
spreads its games over thirteen opponents, so the learner meets the
default executor less often, and the test above is against that executor.
Robustness, the league's purpose, was not measured, because the exploiter
already found no hole in snapshot 200. The fixed mix stays the training
setup; t191 trains on it.

**Second league run (2026-10-26), against a known hole.** t213's exploiter
now beats `fleet-final` 0.348 +/- 0.015 against an even 0.25, so the
league has something to close. The run is `t184/run`: t213's recipe from
`fleet-final-pre-t213`, so it differs from `fleet-final` only in its
opponents. The league is smaller than last time, to keep games on the
executor. It holds the fixed mix's three scripted seats, `fleet-final`,
the exploiter and the learner's last three snapshots. Frozen agents now
play their own build heads, so the exploiter keeps the builds it learned.
An agent saved before the build head builds as the executor would.

**Exit:** paired against `fleet-final` (1,024 maps from 100000, three
executors, 60 and 100 years), the interval includes zero or better. A
fresh 80-iteration exploiter, made the same way as t213's, scores below
0.348 against it.

**Result: stopped at iteration 100; it loses strength again.** In-run
evaluation against three executors (EVAL_MAPS, 128 games):

| Iteration | t184 league | t213 fixed mix |
|---|---|---|
| 25 | 0.180 +/- 0.024 | 0.207 |
| 50 | 0.199 +/- 0.025 | 0.250 |
| 75 | 0.172 +/- 0.024 | 0.332 |
| 100 | 0.180 +/- 0.024 | 0.336 |

The league's running shares at iteration 94 show why. The learner beat
the aggressor (0.86), so the draw almost never chose it, and it lost to
everything else: `fleet-final` 0.045, the executor 0.12, the exploiter
0.15, its own snapshots 0.06-0.09. Prioritized self-play then fills the
iteration with games the learner loses three to one, whose rewards are
nearly all zero, and it learns nothing. t213 grew from 0.21 to 0.34 over
the same span on the fixed mix. With the strength test failed, the exploit
check was not run; no snapshot is a candidate.

Two league runs have now lost to the fixed mix. The exploit hole (0.348)
stays open, and t184 stays open. The next league attempt needs a
different draw before more compute: a fixed share of fixed-mix games (say
75%) with the league only as the rest, and opponents mixed within a game
(one exploiter seat among executors) rather than three copies of one
opponent.

Two fixes from this run stay: frozen league agents play their own build
heads (`c0d440d1`), and PPO minibatches trim their rows to their own
widest seat (`c31947ba`), which took the update from 200-600 s back to
about 50 s when one seat holds many fleets.

**t184b (2026-10-26).** `--league-share 0.25`: three games in four are
the fixed mix, so the learner keeps t213's diet, and the fourth draws
from a league of `fleet-final`, the exploiter and the learner's last three
snapshots (no scripted styles, which the fixed mix already plays). Same
recipe and exit as above; `t184b/run`.

**The minibatch, not the league, sank the first attempts.** t184b's first
50 iterations ran at minibatch 128, as t184 had, and scored 0.180 and
0.195 against the executor at 25 and 50. Restarted at t213's 256 (the row
trim keeps it in memory), it tracked t213: 0.191, 0.266, 0.250, 0.273,
0.344, 0.312, 0.383 and 0.344 at 25 to 200, against t213's 0.207, 0.250,
0.332, 0.336, 0.328, 0.309, 0.291 and 0.371. So t184's full-league result
above was taken at the wrong minibatch and is not a fair verdict on the
full league.

In its league quarter, the learner's running share against the t213
exploiter rose from 0.14 to 0.29; against `fleet-final` it stayed near
0.07.

Paired against `fleet-final` (1,024 maps from 100000, three executors,
builds handed over; `behaviour_report`, bean-void):

| Snapshot | 60 years | 100 years |
|---|---|---|
| 150 | -0.065 +/- 0.019 | -0.090 +/- 0.017 |
| 175 | -0.055 +/- 0.020 | -0.071 +/- 0.017 |
| 200 | -0.038 +/- 0.020 | -0.055 +/- 0.017 |

Snapshot 200 loses by about three standard errors at 100 years, so it
fails the strength half of the exit. The exploit check runs anyway, to
learn whether the league bought robustness: 80 iterations against
snapshot 200 as t213's was made (`exploit-t184b` on bean-void), and t213's
exploiter scored against it too.

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

**Done (2026-10-26): `tools/ai/value_trace.py`.** One greedy seat of a
fleet agent plays three scripted seats or three copies of a frozen agent.
The tool undoes the critic's discount (`V / gamma ** (years - year)`)
and plays with training's `--limit-credit 0.5`, so the estimate and the
result share a scale. It prints the estimate, worlds and production share
by decade, the largest one-year falls over all games and over lost games,
and with `--csv` every game-year. There is no chart: matplotlib is not
in the AI environment, and the CSV plots anywhere. Map, seat and year
name a position exactly, since greedy play from that map reaches it
again.

`fleet-final`, 100 years (`t186/` in the state directory):

| Year | Executors (1,024): estimate | worlds | share | Exploiter (256): estimate | share |
|---|---|---|---|---|---|
| 0 | 0.228 | | | 0.228 | |
| 20 | 0.217 | 4.5 | 0.239 | 0.280 | 0.256 |
| 40 | 0.290 | 6.7 | 0.306 | 0.311 | 0.320 |
| 60 | 0.366 | 8.6 | 0.432 | 0.329 | 0.384 |
| 80 | 0.417 | 10.4 | 0.549 | 0.317 | 0.405 |
| result | 0.527 | | | 0.340 | |

Three findings:

1. **The critic underrates easy games.** Against three executors the
   early estimate is 0.23 for a seat that ends with 0.53; it learned on
   the harder fixed mix. Against the exploiter it is close (0.34).
2. **Its biggest falls are noise in won games.** Over all games, 19 of
   the 20 largest falls (0.2 to 0.36 in a year) come in games the seat
   won or led at the limit, often with production share unchanged and
   the estimate above 1. The critic is unsure of dominant late positions
   it rarely trained in, as t217's explained-variance table suggested.
3. **In lost games the falls are real.** They come with a lost world or a
   drop in production share (0.29 to 0.14 on map 100504 in year 35), and
   they spread from year 19 to 97, most between 25 and 70. These are
   t185's start positions: the lost-games list, not the overall one.

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

**Signed off (owner, 2026-10-18).** The owner approved the three styles.

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

**Combined home gate and a retune (2026-10-16).** `Doctrine::combined_gate`
judges a strike by the strength of every warship fleet gathered at home,
not by each fleet alone (Muse finding 3). Paired on the tuned weights it is
neutral: +0.012 ± 0.024 at 60 years (512 games); at 100 years (384 games)
+0.025 ± 0.027 for land-grab, -0.073 ± 0.034 for the rusher and -0.005 ±
0.032 for the turtle. `tune_executor` now takes `--with SWITCHES` and
`--out FILE.kdl`, and `executor_matches --weights FILE.kdl` reads the
result, so each variant can be retuned with the same budget (40
generations of 24 candidates over 32 maps, 60 years, each generation's
best becoming the champion against the default style) and scored on 1,024
fresh maps against three default seats:

| Style | 60 years | 100 years |
| --- | ---: | ---: |
| `TUNED` (control, an even seat) | 0.243 | 0.237 |
| retuned, no switch | 0.229 | 0.374 |
| retuned, `combined_gate` | 0.266 | 0.394 |

Two findings. Retuning the current doctrine at all lifts the executor at
100 years by 0.14 (the weights were fitted before `supply` and `defend`).
It also moves the weights toward the variety asked for: scouts 0.14 to
about 1 (so they are replaced), destroyers 0 to 0.1-0.2, war year 21 to
29. And the combined gate on top of that is a small gain (+0.037 at 60
years, +0.020 at 100; each standard error about 0.02), not yet distinct
from noise. Neither retuned set is adopted yet: the tuner overfits each
generation's 32 maps, so the next step is to retune on rotating maps and
check the result through the network seat.

**Scouts replaced (`Doctrine::keep_scouts`, accepted 2026-10-16).** Muse
found that the default style's scout weight of 0.14 rounds to zero, so a
lost scout is never rebuilt. Rounding a positive scout target up lifts the
default style paired by +0.039 ± 0.019 at 60 years and +0.150 ± 0.020 at
100 (1,024 maps each; +0.169 ± 0.027 on a first 512). The rusher (1.31) and
turtle (0.66) already rounded to one, so they are unchanged. It is on for
every personality and `Doctrine::CURRENT`. Because every executor now keeps
its scouts, the bots the agents meet are much stronger, and the agents'
margins shrink to what they are against real opposition (512 maps, paired
difference of the promoted agent from the one it replaced in brackets):

| Agent | 60 years | 100 years |
| --- | ---: | ---: |
| Previous `fleet-final` | 0.402 | 0.516 |
| Snapshot 140 (promoted) | 0.527 (+0.125 ± 0.026) | 0.708 (+0.192 ± 0.027) |

The earlier figures (0.657 and 0.774 at 60 years, 0.868 and 0.927 at 100)
were against scoutless bots. The recorded-game fixture
(`tools/ai/export_onnx.py --fixture`) was regenerated. The agent has not
been trained against the stronger bots (t191).

**Retuned trials, with scouts on (2026-10-16).** Three retunes of 40
generations over 64 maps at 80 years (best of each generation becoming the
champion against the default style), scored on 1,024 fresh maps against
three default seats (an even seat is 0.25):

| Style | 60 years | 100 years |
| --- | ---: | ---: |
| default weights | 0.247 | 0.244 |
| retuned | 0.307 | 0.310 |
| retuned with `combined_gate` | 0.318 | 0.280 |
| retuned with `mixed_hulls` | 0.235 | 0.158 |

Retuning lifts the executor about 0.06 at either horizon. The combined gate
is no better than retuning alone (ahead at 60 years, behind at 100; each
standard error about 0.014), and mixed hulls lose even after a retune, the
worst of the four at 100 years. So `mixed_hulls` is rejected for good (the
switch goes with t197) and `combined_gate` stays off unless the staging
trials change that. The route to variety is hull roles, not a mixed pool.

**Early-warning stations (`Doctrine::watch`, accepted 2026-10-16).** After
the land grab, a lone picket (one or two destroyers) or a spare scout
stands in the approach sector between the seat's nearest exposed world and
its nearest rival's world, up to one watcher per approach, using the new
`FleetChoice.station` (the network never sets it). Paired over 1,024 maps
at 60 and 100 years, every personality gains:

| Personality | 60 years | 100 years |
| --- | ---: | ---: |
| default | +0.023 ± 0.017 | +0.051 ± 0.018 |
| rusher | +0.010 ± 0.016 | +0.029 ± 0.015 |
| turtle | +0.066 ± 0.016 | +0.031 ± 0.016 |

It is on for every personality. With scouts and stations in, the promoted
agent scores 0.773 at 60 years and 0.920 at 100 against the new default
bots (the previous agent 0.603 and 0.811; paired difference +0.171 ±
0.026 and +0.109 ± 0.020, 512 maps). One limit: a station has no array
form, so a network cannot order one, and tests that pass executor orders
through arrays use `tests/common::tuned_without_stations`. If the network
is ever handed the scout family (`Families::ALL`), stations need an action
(t190).

**Built pickets, rejected (2026-10-16).** A trial switch had the home
world keep one destroyer for each approach a watch station covers (up to
three) so the stations are staffed. Paired over 512 maps at 100 years it
is neutral for the executor (default +0.012 ± 0.024, rusher 0.000 ±
0.011, turtle -0.012 ± 0.015), and through the network seat it costs the
promoted agent: win share 0.875 against 0.920 and maps swept 37% against
48% (512 maps at 100 years, standard error about 0.012). The opening
escorts and spare scouts are enough to staff the stations, so the switch
was removed rather than left off.

**Why hulls stay cruisers: battleship speed (2026-10-16).** Two forced-hull
styles were added (`armada`, battleships first; `raider`, destroyers first,
04198f27), and tuned like the others against three default seats on 1,024
fresh maps (an even seat is 0.25). At today's rules both are weak:

| Style | 60 years | 100 years |
| --- | ---: | ---: |
| default (cruiser-led) | 0.271 (512 maps, style_stats) | |
| armada, tuned | 0.187 | 0.146 |
| raider, tuned | 0.172 | 0.160 |

Scratch copies of the engine (not committed) then changed one rule at a
time and retuned the armada for each:

| Change | 60 years | 100 years |
| --- | ---: | ---: |
| battleship cost 35 | 0.198 | 0.166 |
| cost 30 | 0.232 | 0.212 |
| cost 25 | 0.237 | 0.209 |
| cost 20 | 0.209 | 0.210 |
| **battleship speed 5, cost 45** | **0.307** | **0.290** |

Price alone never gets the armada past an even seat, even at under half
cost. Speed does, with the price unchanged: the same style goes from 0.187
and 0.146 to 0.307 and 0.290, ahead of the cruiser-led default. In a pure
battle battleships are already the strongest hull per point (the crunch
Claude's 20,000-battle test: 10 battleships beat 30 cruisers about 60% of
the time), so the campaign cost of speed 4 is what keeps them out of the
AI's fleets: a fleet moves at its slowest ship, and a battleship-led
force with transports is held to 4 while a cruiser force moves at 5.
Either the AI keeps cruisers because the rules make them better (speed),
or the owner moves battleship speed to 5 (as Empire of the Sun does for
all naval units) and the AI starts building battleships. This is a rules
decision for the owner (combat-balance lane), not an AI change; the trial
is in the scratch results only.

**Correction: it is largely the AI, not the rules (2026-10-16).** The
owner pointed out that 6/5/4 is classic EC's speed ladder, so the speed
finding above does not make speed 4 wrong. A diagnostic (a scratch
example, 256 maps at 100 years) played the same tuned armada weights at
battleship speed 4 and 5, so only the engine differs:

| Weights | Speed | Worlds taken | Worlds at end |
| --- | ---: | ---: | ---: |
| armada | 4 | 16.0 | 3.6 |
| armada | 5 | 20.8 | 5.3 |
| armada tuned at speed 5 | 4 | 16.6 | 3.6 |
| armada tuned at speed 5 | 5 | 20.1 | 5.1 |

A slow stack costs the same AI about a fifth of its conquests, and the AI
never adapts: after each strike its warships and transports sail back to
the home world to regroup, so a speed-4 force pays a long round trip every
cycle. `Doctrine::forward_muster` (accepted) makes warships and
transports gather at the seat's world nearest a known rival world, so a
force that has taken a world launches its next strike from the front. It
is the staging the owner described. Paired over 512 maps:

| Style | 60 years | 100 years |
| --- | ---: | ---: |
| default | +0.085 ± 0.027 | +0.156 ± 0.027 |
| rusher | +0.091 ± 0.025 | +0.101 ± 0.027 |
| turtle | +0.046 ± 0.025 | +0.025 ± 0.025 |
| armada at today's speed 4 | +0.051 ± 0.024 | +0.111 ± 0.024 |

The battleship-led armada, which scored 0.207 and 0.143 against the
default, now scores 0.258 and 0.254: an even seat at the real speed. So the
rules were not the cause; battleships were held back by an AI that did not
stage forward. It is on for every personality and `Doctrine::CURRENT`.
The deployed agent's strike orders come from the network, so it gains only
through retraining (t191: imitate the improved executor, then PPO). Against
the newest default bots the promoted agent scores 0.696 at 60 years and
0.862 at 100, and the previous agent 0.536 and 0.696 (paired difference
+0.160 ± 0.028 and +0.166 ± 0.024). The armada and raider need a retune
under the new doctrine before they are judged.

**Re-baseline under the t198/t200 combat rules (2026-10-16).** The figures
above were measured before t198 removed the mixed-fleet bonus and t200 set
the punch ladder (DD 12/7, CA 33/30, BB 90/110, SB DS 130). The Python
binding and examples were rebuilt at `5e3b236c`. The executor's
`strength` (1/3/9) still matches the new stats on attack x defence (DD : CA
: BB about 1 : 3.4 : 10.8, against 1 : 3.4 : 10.6 before), so it was left as
is; the network observes raw ship counts.

Agent (`behaviour_report`, 512 maps, against the default bots, same command
as before):

| Agent | 60 years | 100 years |
| --- | ---: | ---: |
| `fleet-final`, before / after | 0.696 / 0.686 | 0.862 / 0.875 |
| previous agent, before / after | 0.536 / 0.568 | 0.696 / 0.724 |
| paired gain, after | +0.117 ± 0.027 | +0.151 ± 0.023 |

Styles against three default seats (`style_stats`, 1,024 maps, the same
maps under both rule sets; before = `63dec4e1`, after = `5e3b236c`):

| Style | 60 years, before / after | 100 years, before / after | Main hull |
| --- | ---: | ---: | --- |
| land-grab | 0.278 / 0.253 | 0.277 / 0.255 | cruisers 59-65% |
| rusher | 0.191 / 0.199 | 0.198 / 0.184 | cruisers 61-65% |
| turtle | 0.200 / 0.181 | 0.174 / 0.161 | cruisers 57-62% |
| armada | 0.187 / 0.178 | 0.153 / 0.142 | battleships 48-54% |
| raider | 0.143 / 0.107 | 0.125 / 0.116 | destroyers 59-64% |

The rules change moved neither the AI's builds nor the order of the styles:
the default still builds cruisers almost only, and the battleship- and
destroyer-led styles stay well behind it (raider fell most, 0.036 at 60
years). The agent is as strong as before, so no early retrain is needed.
In a pure battle the engine now has cruisers and battleships about even per
point, so the missing variety is an AI problem: one preferred hull and
weights tuned under the old rules. Next: hull roles and a retune under the
new rules (t189), then the order set (t190), then one retrain (t191).

**Under the t202 planetary rules (2026-10-16).** Two-step batteries, their
full-strength return fire, and classic capture damage cut the agent's lead:
`fleet-final` against the default bots wins 0.354 at 60 years and 0.521 at
100 (0.686 and 0.875 before), +0.039 +/- 0.026 and +0.089 +/- 0.028 over the
previous agent; it loses about 20 worlds a game in 100 years and never
blitzes. The styles against three default seats at 100 years: land-grab
0.262, raider 0.184, rusher 0.162, armada 0.125, turtle 0.119. A `blitz`
switch (blitz when the loaded armies cover twice the known armies plus 2.5 a
known battery; branch `t189-blitz`, not merged) lost at every threshold
tried, -0.033 +/- 0.023 at 2x and -0.007 +/- 0.022 even when limited to
worlds with no known batteries (100 years, 512 paired maps), and is
rejected for now. A retune under t202 is running.

The owner's plan, bombard until the batteries are gone and then blitz,
beats bombard-then-invade in the engine (`assault_odds`, 400 trials): the
same capture rate and ship losses, a few more armies lost, and two to four
times the industry kept (for example 10 BB + 40 armies against 8 batteries:
76% and 90% of production kept, against 77% and 45%). A large fleet still
burns some industry in the breakthrough rounds of the year the batteries
fall. It is the executor's next trial: once the batteries are known gone,
blitz instead of invading.

**Retune adopted (owner, 2026-10-16).** `tune_executor --personality
land-grab --against-default` (30 generations, 24 candidates, 64 fresh maps
a generation, 100 years, bean-void) moved the attack ratio from 2.43 to
3.80, ETACs from 3.9 to 5.7, scouts from 0.14 to 1.4 and starbases from
1.7 to 2.0. On 1,024 untouched maps against three seats of the old weights
it wins 0.317 at 60 years and 0.308 at 100 (production share 0.283 and
0.296). These are now `Weights::TUNED` and the land-grab style. Bombard
then blitz on top of them is neutral (+0.018 +/- 0.011 at 60, -0.010 +/-
0.014 at 100) and stays off. Next: retune the other styles, then retrain
the agent (t191).

**Retrain under t202 promoted (owner, 2026-10-17).** PPO from
`fleet-final` on the fixed mix, 100-year games with `--limit-credit 0.5`,
200 iterations (`fleet-t202/ppo`). Snapshot 200 against `fleet-final`
(behaviour report, 1,024 maps from 100000, paired): win share 0.355 to
0.547 at 60 years (+0.192 +/- 0.019) and 0.498 to 0.739 at 100 (+0.241 +/-
0.019). Exploit check (bean-void, 80 iterations of `--exploit` against
snapshot 200, scored greedy on 1,024 maps from 100000): the exploiter wins
0.192, 0.182 and 0.200 at iterations 40, 60 and 80, against 0.235 +/- 0.013
for snapshot 200 in its own seat, so it found no hole. Snapshot 200 is now
`fleet-final` (ONNX export: 0 mismatches on 64 recorded seats); the
previous agent is kept as `fleet-final-pre-t202`. The t184 league run
starts from that previous agent, through its own copy at
`fleet-league/ai/fleet-final.pt`, which a `--resume` must name in place of
`fleet-final.pt`.

**Owner decision (2026-10-16): variety is required.** The AI builds every
hull, as roles, and the verdict on a variant waits for a retune. The
mixed-hull and Join trials above ran on weights tuned for the old
behaviour, so they stay provisional until `tune_executor` has run with the
switch on and the tuned weights are compared with the tuned baseline. The
order of work: (1) roles for the hulls, with battleships staged as a
group, destroyers as raiders and pickets, and cruisers as the line;
(2) Muse's combined-home-strength gate, which gives every home fleet one
target and speed, in place of Join; (3) round positive scout and ETAC
targets up, and keep picket stations after the land grab; (4) retune each
with `tune_executor` and judge it against the tuned baseline at 60 and 100
years, through the network seat as well as executor against executor.

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

**Hull roles under today's rules (2026-10-18): cruisers are the right
fleet.** Measured on the Mac mini at `6702e1ce`.

Equal-cost battles (`nc-fuzz` `combat_odds`, 450 points a side, ROE 6,
1,000 trials): 30 cruisers beat 27 cruisers + 9 destroyers 66.3% to
33.5%, and 24 cruisers + 18 destroyers by the same margin; 30 cruisers
beat 10 battleships 56.2% to 43.0%, and 5 battleships + 15 cruisers
54.4% to 44.4%. The manual's pairing, heavy ships crippling and
destroyers finishing, does not make up for the destroyer's lower DS per
point: adding destroyers to a cruiser fleet makes it weaker.

Campaigns agree. `mixed_hulls` with battleships zeroed, so only cruisers
and destroyers mix (no speed cost, since destroyers are faster), paired on
512 maps against three default seats:

| Destroyer share of the hull weight | 60 years | 100 years |
| --- | ---: | ---: |
| 10% | -0.029 ± 0.022 | -0.011 ± 0.025 |
| 25% | -0.046 ± 0.022 | -0.015 ± 0.024 |
| 40% | -0.052 ± 0.024 | -0.022 ± 0.026 |

Battleships do have a role against batteries. `siege_odds`, with
cruisers swapped in at equal cost for a one-off copy (not committed): a
single force worth three times the batteries' cost clears them 72-88% of
the time with battleships, 46-73% with cruisers, and loses 1.3-1.7 against
1.9-2.1 times the batteries' cost. But a trial `siege_hulls` switch
(battleships instead of the preferred hull while the strike target has at
least N known batteries or a starbase) lost in campaigns, more the more
often it fired: 1,024 paired maps for land grab, N = 1, 4 and 8, -0.005 to
-0.007 at 60 years and -0.020 to -0.032 (± 0.010) at 100. The battleships
join the cruiser force and slow it to speed 4, which costs more than the
better siege gains, as the earlier armada trials found. The switch was
removed rather than left off.

So under the current combat rules, a cruiser fleet beats every mix the
executor has tried, and variety costs strength. Per the owner's rule
above, this goes to the owner as a combat-balance finding: ship prices
stay classic, so variety needs either a combat-rule change (for example,
destroyers' execution fire counting for more, so the pairing pays) or
roles that keep slow battleships out of the cruiser force entirely (a
separate siege group with its own rendezvous, not yet tried).

The owner took the combat-rule route on 2026-10-18: heavies cover the
destroyers, execution finishes cripples at half price, and a heavy line
adds one to the die
([tdkitij6](../decisions/tdkitij6-heavies-cover-the-destroyers-which-finish-cripples-at-half-price.md)).
Adoption and the retune are t206 below.

**Re-baseline under t206 (2026-10-18).** At `59930a31`. The agents, one
network seat against three default executors, 1,024 maps from 100000
(`behaviour_report`, Mac mini):

| Agent | 60 years | 100 years |
| --- | ---: | ---: |
| `fleet-final` | 0.570 | 0.753 |
| t191 pre-t206 run, snapshot 175 | 0.649 (+0.080 ± 0.019) | 0.845 (+0.092 ± 0.016) |
| t191 pre-t206 run, snapshot 200 | 0.633 (+0.063 ± 0.019) | 0.813 (+0.060 ± 0.016) |

`fleet-final` held up (0.547 and 0.739 under the old rules), and the
retrain's lead carries over. The styles against three default seats
(`style_stats`, 1,024 maps, bean-void), 60 / 100 years: land-grab 0.258 /
0.245, raider 0.169 / 0.182, turtle 0.161 / 0.166, armada 0.192 / 0.139,
rusher 0.138 / 0.126. The cruiser-led styles still spend 64-69% of their
build points on cruisers.

**Do mixed fleets lose because destroyers sail alone? No.** `style_stats`
now counts destroyer-years spent in a fleet with a cruiser or battleship,
and runs one style file (`--style`). With `mixed_hulls` and battleships
zeroed, 512 maps against three default seats:

| Destroyer share of the hull weight | Destroyers with a heavy, 60 / 100 yr | Win share, 60 / 100 yr |
| --- | ---: | ---: |
| default style (no switch) | 20% / 20% | 0.264 / 0.258 |
| 10% | 74% / 82% | 0.264 / 0.234 |
| 25% | 85% / 90% | 0.244 / 0.238 |
| 40% | 88% / 92% | 0.232 / 0.242 |

Most destroyers already fight under cover, so no commissioning change is
needed. The next suspect is weights tuned for a cruiser fleet: both arms
are retuned under t206 (below).

**Mixed hulls adopted under t206 (owner's bar, 2026-10-18).** The owner set
the bar: adopt if the mixed executor does not lose to a cruiser executor
tuned the same way, and keep battleships out of the line this round. Both
arms were retuned from the land-grab style (`tune_executor --against-default`,
30 generations of 24 candidates, 64 fresh maps a generation, 100 years):
the cruiser arm on the Mac mini, the mixed arm on bean-void with
`mixed_hulls` on and the battleship weight pinned at 0.

| Weight | Cruiser retune | Mixed retune |
| --- | ---: | ---: |
| cruiser | 0.80 | 0.26 |
| destroyer | 0.33 | 0.32 |
| battleship | 0.05 | 0 (pinned) |
| attack ratio | 3.17 | 3.74 |
| scouts / starbases | 0.0 / 0.07 | 0.21 / 0.44 |

The cruiser arm builds only its largest weight, so its destroyer weight
has no effect. Paired, each against three default seats, 1,024 maps from
1500000 (`executor_matches --weights mixed --baseline cruiser --paired`):
the mixed retune wins 0.272 against 0.284 at 60 years (-0.012 ± 0.018) and
0.244 against 0.263 at 100 (-0.020 ± 0.018). Neither difference is
distinct from zero, so mixed hulls pass. `Doctrine::CURRENT` has
`mixed_hulls` on, and the mixed retune is `Weights::TUNED` and the
land-grab style, whose bounds now pin battleships at 0 and keep destroyers
at 0.1 or more (`dc979f43`). The executor builds for the network seat, so
the deployed agent's fleets mix too. The seat fixtures' games were
re-recorded; `tiny.onnx` stays the pre-t190 graph that tests old inputs.
The rusher and turtle are retuned next under the same doctrine.

**The other styles under mixed hulls (2026-10-18).** Each retuned the same
way (battleships pinned at 0, destroyers 0.1 or more) and judged paired
against its current style, three default seats, 1,024 maps from 1500000:

| Style | 60 years | 100 years | Verdict |
| --- | ---: | ---: | --- |
| rusher, retuned (bean-void) | +0.025 ± 0.013 | +0.021 ± 0.013 | adopted (`b16cb7e2`) |
| turtle, retuned (Mac mini) | -0.090 ± 0.014 | -0.088 ± 0.015 | rejected |
| turtle, own weights, destroyers a quarter | -0.037 ± 0.014 | -0.059 ± 0.015 | rejected |

The rusher's retune puts destroyers at 0.45 against cruisers' 0.56. The
turtle loses with destroyers either way, so it stays a cruiser fleet; its
retune ended on rules of engagement of 2.2, a cautious last pick from a
noisy generation. Why the turtle, which keeps warships home under the
allocation goal, does worse with destroyers is open. The armada
(battleship-led) and raider (destroyer-led) were not changed.

**Deep-space staging, rejected (2026-10-18).** A `stage` switch sent a
force bound for a world more than two sectors away to Rendezvous one sector
short of it in deep space, struck from there once the merged force was
`attack_ratio` times the target's defence, and sent warships at home to
reinforce a force waiting there. Paired against the adopted land-grab
executor, 1,024 maps: -0.048 ± 0.017 at 60 years, -0.001 ± 0.018 at 100.
Under `forward_muster` the force already gathers by Rendezvous at the
world nearest the enemy, so the stop adds a year without assembling more.
The switch was removed (`6faca951`). Salvage was not tried: it scraps ships
at half their price, and crippled ships mend on their own, faster beside a
starbase, so a rule that salvaged damaged fleets could only lose strength.

**Rejected switches removed (2026-10-18).** `reinforce` and
`combined_gate` lost or stayed neutral in their trials, and the staging
trial that might have revived the combined gate lost, so both are gone
with the code only they used (`da1d46e3`). Join stays available to the
network (t190).

**The agent cannot fight with mixed fleets (2026-10-18).** The executor
builds for the network seat, so adopting mixed hulls changed the agent's
own fleets as well as its opponents'. One network seat against three of the
adopted executors, 1,024 maps from 100000 (`behaviour_report`, bean-void,
at `492a55f8`):

| Agent, its seat's builds | 60 years | 100 years |
| --- | ---: | ---: |
| `fleet-final`, mixed (the new default) | 0.172 | 0.192 |
| t191 pre-t206 snapshot 175, mixed | 0.145 | 0.145 |
| `fleet-final`, cruisers (the old land-grab file) | 0.525 | 0.709 |

Against cruiser executors it scored 0.570 and 0.753, so mixed opponents
cost it only about 0.04; its own mixed fleets cost it 0.5. With them it
parks warships for 955 fleet-years a game against 186, and invades 13
times against 30: a network trained on cruiser fleets seems never to judge
a destroyer-heavy force ready to strike. **Until t191 retrains the agent on
mixed fleets, main's AI seats are far weaker than the released ones; do not
release main's AI.** The live community stack (`ea1a6b3b`) and the beta.3
archives predate the adoption.

### t203

Early warning: pickets on the approaches, threats tracked from what the
seat has seen, and warships that sail to meet them. The owner asked
(2026-10-17) whether the turtle should picket the sectors around its
worlds with destroyers, infer where an enemy fleet is headed, strengthen
defences in advance, and send fleets to intercept.

What the executor had: `watch` posts spare scouts and pickets halfway
between the most exposed world and the nearest rival world; a declared
enemy's force seen within 4 sectors in the last 2 years threatens a world,
which then builds its guard (under `allocate`). Gaps: the retuned turtle
buys no scouts or destroyers, so it is blind once its starting destroyers
are gone; only empires the seat itself declared enemies count, so the
turtle (war in year 27) ignores an attacker until then; and no waiting
warship ever sails to a threatened world.

Three trial switches, all off by default (`6182b67c`):

- `pickets`: after the land grab, while fewer than three watchers stand
  (scout or picket fleets, and destroyers waiting in stardocks with no
  larger hull), the colony nearest the open approach whose smallest
  destroyer build is one or two builds it alone, so it commissions as a
  picket.
- `track`: an empire that fought the seat's forces or bombarded or
  assaulted its worlds in the last five years is hostile, declared or not
  (`SeatMemory::attackers`, from the seat's own reports), and a hostile
  force also threatens the seat's world nearest the line from its
  empire's nearest known world through the sighting, within 12 sectors.
- `intercept`: warships waiting at the muster world, when it is not
  itself threatened, guard the nearest threatened world within 12 sectors
  whose strongest threat they outgun, and keep station while it lasts.

The agent hands the warship fleets to its network (`--families strike`),
so `intercept` changes only the scripted bots; pickets and `track`'s guard
builds reach the agent too.

On the 4-player maps the nearest rival world is 4-6 sectors from a
colony, so the 4-sector threat radius already covers most approaches and
a heading adds warning only for farther worlds.

Exit: paired tests (`executor_matches --paired --with`) on 1,024 maps at
60 and 100 years, turtle against the default and against the rusher, then
the other styles; keep a switch only if it does not lose, and remove the
ones rejected.

**First test (2026-10-17), on the turtle's weights tuned without the
switches.** Paired difference in the turtle's win share, 1,024 maps from
1500000 (standard error about 0.015):

| Switches | vs default, 60 / 100 yr | vs rusher, 60 / 100 yr |
| --- | --- | --- |
| pickets | -0.036 / -0.031 | -0.033 / -0.023 |
| track | -0.021 / -0.025 | -0.028 / -0.019 |
| intercept | -0.011 / -0.071 | -0.052 / -0.082 |
| track + intercept | -0.019 / -0.049 | -0.091 / -0.104 |
| pickets + track | -0.033 / -0.046 | -0.055 / -0.049 |
| all three | -0.036 / -0.100 | -0.088 / -0.104 |

Every switch loses, interception most. The weights were fitted without
them, as with `mixed_hulls` and `reinforce`, so the turtle is retuned
with pickets + track and with all three before a verdict.

**Revised (2026-10-17).** At the owner's suggestion a colony now buys its
picket from what is left after its garrison, in place of its warships, and
only when that buys one or two destroyers. Interception now uses only
fleets with nothing to strike that year. Paired, on the turtle's current
weights:

| Switches | vs default, 60 / 100 yr | vs rusher, 60 / 100 yr |
| --- | --- | --- |
| pickets from leftover budget, 3 posts | +0.003 / -0.039 | -0.032 / -0.010 |
| pickets from leftover budget, 2 posts | -0.010 / -0.023 | -0.027 / -0.026 |
| pickets from leftover budget, 1 post | -0.004 / -0.015 | +0.007 / +0.009 |
| intercept, idle fleets only | -0.020 / -0.020 | -0.003 / -0.019 |
| track | -0.021 / -0.025 | -0.028 / -0.019 |
| all three | -0.021 / -0.036 | -0.046 / -0.007 |

One post is kept (`c792dbb3`). Retuned with the switches on (30
generations, 64 games, 100 years), against the current turtle at 0.185 /
0.188 against the default and 0.316 / 0.313 against the rusher, on 1,024
maps from 1500000:

| Retuned turtle | vs default, 60 / 100 yr | vs rusher, 60 / 100 yr |
| --- | --- | --- |
| all three | 0.147 / 0.148 | 0.282 / 0.257 |
| pickets + intercept | 0.186 / 0.155 | 0.291 / 0.307 |

A retune does not rescue them. Pickets at one post are even; tracking and
interception lose as scripted rules. Interception remains a candidate for
the network to learn now that it can order the fleets that would do it
(t190).

**Decision (2026-10-17, owner approved).** `track` and `intercept` are
removed, with `SeatMemory::attackers`, which only `track` read; the threat
rule is t181's again. `pickets` stays as a switch, off: even at one post,
and early warning the network may yet use.

**The other styles (2026-10-18), closing t203.** `pickets` paired against
three default seats, maps from 1500000 (`executor_matches --with pickets
--paired`, on the Mac mini at `94c3acb8`):

| Style | 60 years | 100 years |
| --- | ---: | ---: |
| land grab (1,024 maps) | +0.003 ± 0.002 | +0.001 ± 0.002 |
| rusher (1,024 maps) | +0.001 ± 0.001 | +0.000 ± 0.000 |
| armada, raider (128 maps) | +0.000 ± 0.000 | |

The switch barely fires outside the turtle: the land grab, armada and
raider already keep a watcher (scouts, or the raider's destroyers), so the
one-post rule has nothing to add, and the rusher triggers it in a handful
of games. It loses nowhere, so it stays, off, as decided above. Nothing
else is left to remove.

### t206

Adopt the destroyer cover rules
([tdkitij6](../decisions/tdkitij6-heavies-cover-the-destroyers-which-finish-cripples-at-half-price.md)):
heavies in front, execution finishes cripples at half price, a heavy line
adds one to the die. The engine, model, tests and docs are on branch
`t206-destroyer-rules` (`bf3ffde5`), merged to main.

**Gate campaign (2026-10-18, Mac mini): fails.** Paired `mixed_hulls`
against three tuned seats, 1,024 maps from 1500000, battleships zeroed,
the t189 share files, main at `6702e1ce` against the branch:

| Build, destroyer share | 60 years | 100 years |
| --- | ---: | ---: |
| main, 25% | -0.025 ± 0.016 | -0.008 ± 0.017 |
| t206, 10% | -0.038 ± 0.016 | -0.021 ± 0.017 |
| t206, 25% | -0.025 ± 0.016 | -0.018 ± 0.017 |
| t206, 40% | -0.020 ± 0.017 | -0.002 ± 0.018 |

The rules win the equal-cost battles (mixes 70% against pure cruisers,
36% before) and reverse the campaign trend (under the old rules more
destroyers lost more: -0.029, -0.046, -0.052 at 60 years), but no mix
beats the pure cruiser executor, so the gate's condition (mixed-on ahead
with the interval above zero at both horizons) is not met. The owner
merged the rules anyway on 2026-10-18: the incentive now exists in the
game, and the executor's failure to use it is an AI problem (its weights
and the `mixed_hulls` doctrine were fitted under the old rules), carried
by t189 and the retrain in t191. t206 closed with the merge.

### t190

Give the network the whole order set.

The network cannot order Join Another Fleet (mission 13), or the turn
actions Join, Transfer and Unload Armies, because they need a fleet target
and none is offered (`nc-ai/src/action.rs`). Add a target that points at
the seat's own fleets, with masks from the game's validators, to the
model, the ONNX export and the Rust decoder.

**Exit:** a test shows each new order chosen, masked and submitted
legally, and the exported model plays with them.

**Progress (2026-10-17).** The network can now order Join and Unload.

- Masks (`action.rs`): each fleet row has a join row (none, then one flag
  per row of the seat's fleets) and an unload flag (at one of the seat's
  worlds, armies aboard, room for them). A hybrid seat's fleets may join
  only fleets its network orders (`network_joins`), so a strike force never
  absorbs the executor's scouts or pickets.
- Decoding: Join replaces the mission. Join with Detach leaves the detached
  ships where they are and sends the rest to the host, which covers most of
  Transfer, so Transfer is not offered. An unload is skipped when the
  fleet's new or kept order is Invade or Blitz (random play found the
  conflict), and a fleet that loads and unloads only unloads.
- Batch arrays, `nc_ai_env` (join and unload optional in `step`, default
  none) and imitation labels carry both.
- Model (`fleet_policy.py`): a join head pointing at the other fleet
  tokens, with a learned "none" logit, and an unload head. They start on
  "none" (bias 8, zero weights; the join key starts as the identity so the
  query can learn), so older checkpoints load and play as before and the
  PPO reference starts identical (KL 0). Sampled play joins in about 0.4%
  of fleet-years at first; greedy play never does.
- Export: the join and unload masks and noise follow every other input, so
  a pre-t190 graph keeps its input positions; the Rust seat feeds them only
  to a graph that has them (`OLD_INPUTS`) and reads join and unload only
  when they are output.

Validation: 1,000 random masked games (30,000 game-years) with 172,094
joins (4,903 with a detach) and 10,502 unloads, none rejected; the Rust
seat replays both fixtures turn for turn, `tiny.onnx` (exported before
t190) and `tiny-joins.onnx` (random join heads; 308 Joins in its game);
snapshot 200 exports with 0 mismatches on 64 recorded seats.

What it does not do yet: the agent will not learn these orders by itself.
The executor never unloads and joins only under `reinforce` (off), so
imitation labels them "none", and PPO's pull toward its reference and its
small entropy leave the new heads near their start. t191 needs either a
teacher rule that uses them or PPO that exempts these heads from the KL
penalty and raises their entropy.

**Closed (2026-10-18).** The exit is met: random masked play
(`tests/action.rs`) submits joins, joins with a detach, and unloads with
none rejected; `a_join_and_a_station_decode_into_legal_orders` covers the
decoder; and `nc-ai-onnx`'s `a_seat_with_join_heads_makes_the_recorded_turns`
replays an exported graph that joins. Transfer is covered by Join with
Detach. Teaching the agent to use the orders is t191's.

### t191

Retrain the agent on the new executor.

The network learned its fleet orders by imitating the executor (t157). It
learns t189's doctrine and t190's orders the same way: imitation with
DAgger from the current weights, then PPO on 100-year games (t182), on
crunch when Codex has no quiet hold.

**Exit:** the retrained agent beats the deployed one on 1,024 maps, and
its behaviour report shows the wider order set and the mixed fleet.

**Preparation (2026-10-17).** `fleet_ppo.py --free-heads join,unload
--free-entropy W` leaves the t190 heads out of the KL penalty and gives
their entropy an extra weight, so PPO can explore orders that imitation
labels only as "none". The retrain waits for t184's league run, which
decides whether it trains on the league or the fixed mix. The league run
lost (t184), so it trains on the fixed mix. Played greedy, no agent so far
orders a Join or an Unload (behaviour report, 0/0/0 joins, transfers and
unloads a game): the new heads start on "none".

**A retrain under the old rules (2026-10-18, before t206).** A full run
finished at 13:01, before the t206 merge, and was not recorded until now:
PPO from `fleet-final` on the fixed mix, 100-year games, `--limit-credit
0.5 --kl-floor 0.02 --free-heads join,unload --free-entropy 0.03`, 200
iterations of 64 games (`~/.local/state/nostrian-conquest/ai/t191-full/`;
the pilot that chose 0.03 is `t191-pilot2/`). Against three executors on
1,024 maps at 100 years, under the old rules, snapshot 175 led
`fleet-final` 0.841 to 0.739 (+0.102 ± 0.016). Under t206 it still leads
(t189's re-baseline). Sampled play joined about half the time by the end,
but greedy it orders no Join and a handful of unloads (0/0/3 a game), so
the free heads did not teach the order set. It was not promoted: the
retrain this task asks for comes after t189's executor work, on the new
rules.

**The retrain on mixed fleets passes the strength gate (2026-10-19).**
Imitation with DAgger from `fleet-final` and the alloc strategist (40
rounds; mission 0.897, target 0.778 teacher-forced), then PPO on the fixed
mix with the same flags as above, 200 iterations of 64 games
(`~/.local/state/nostrian-conquest/ai/t191-mixed/`, `run.sh`). PPO's
greedy checks against executors fell first, then climbed: 0.121, 0.090,
0.160, 0.229, 0.312, 0.262, 0.330 and 0.348 at each 25 iterations. One
network seat, building mixed fleets, against three adopted executors, 1,024
maps from 100000 (`behaviour_report`, bean-void, at `970cdbc5`):

| Agent | 60 years | 100 years |
| --- | ---: | ---: |
| `fleet-final` | 0.172 | 0.192 |
| t191-mixed snapshot 175 | 0.508 | 0.725 |
| t191-mixed snapshot 200 | 0.504 | 0.708 |

Snapshot 175 leads `fleet-final` by +0.335 ± 0.018 at 60 years and +0.532
± 0.018 at 100, back to what `fleet-final` scored with cruiser builds
(0.525, 0.709). It parks warships for 198 fleet-years a game against 955,
invades 27 times against 13, and ends with 11.4 worlds against 3.5.
Exploit check (bean-void, 80 iterations of `--exploit` against snapshot
175, scored greedy on 1,024 maps from 100000): the exploiter wins 0.217,
0.252 and 0.250 at iterations 40, 60 and 80, against 0.253 ± 0.014 for
snapshot 175 in its own seat, so it found no hole. ONNX export: 0
mismatches on 64 recorded seats, for both snapshots. Greedy it still
orders no Join, transfer or unload (0/0/0), so the exit's wider order set
is not met; the mixed fleet is.

**Promoted (owner, 2026-10-19).** Snapshot 175 is now `fleet-final`
(`.pt` 574ad237…, `.onnx` 077d9284…); the previous agent is kept as
`fleet-final-pre-t206`. t191 stays open for the order set.

**Why the agent never joins (2026-10-19).** On handed-over rows where a
join is legal (128 greedy games against executors at 100 years):

| Agent | P(join), mean | rows with P(join) > ½ | greedy argmax joins |
| --- | ---: | ---: | ---: |
| `fleet-final` (t191-mixed 175) | 0.0000 | 0% | 0% |
| t191-full snapshot 200 | 0.44 | 38% | 0.01% |

Three causes. (1) Greedy play split the join choice: one categorical over
none and about 17 hosts, so none wins the argmax while joining at all is
likelier. (2) The free-head bonus peaks at acting half the time, and t191-full's
0.44 sits at that optimum, so it shows the bonus, not that joins win. (3)
Imitation labels joins "none" until the head never acts, where the bonus
has no gradient, so t191-mixed sampled no joins at all. Unload is binary;
t191-full's greedy unloads on 3.7% of legal rows. Fixes (`8c591439`):
greedy play joins when P(none) < ½, then picks the likeliest host
(`greedy_join`, in the model and the exported graph at zero noise; noise
still samples the trained distribution); `--free-entropy-until N` fades the
bonus to 0; `--reset-new-heads` puts the join and unload heads back at
their start.

**The retrain that joins (2026-10-19).** PPO from the promoted agent with
the join and unload heads reset, the free-head bonus 0.03 fading to 0 by
iteration 120, otherwise t191-mixed's flags, 200 iterations
(`~/.local/state/nostrian-conquest/ai/t191-orders/`, `run.sh`). Sampled
joins rose to 0.41 of handed-over fleet orders by iteration 100 and held
at 0.37-0.41 through the 80 iterations with no bonus, so PPO keeps them
for the reward. Unloads stayed at 0. Greedy checks against executors:
0.254, 0.250, 0.207, 0.215, 0.230, 0.305, 0.348, 0.402. Paired behaviour
report against the promoted agent (bean-void, 1,024 maps from 100000, at
`0f629cd7`; the baseline exported with `greedy_join`, which changes
nothing for a head that never joins):

| Agent | 60 years | 100 years | joins a game, 60 / 100 |
| --- | ---: | ---: | ---: |
| promoted (t191-mixed 175) | 0.508 | 0.725 | 0 / 0 |
| t191-orders snapshot 175 | 0.542 | 0.762 | 57 / 113 |
| t191-orders snapshot 200 | 0.575 | 0.791 | 76 / 344 |

Snapshot 200 leads by +0.067 ± 0.019 at 60 years and +0.066 ± 0.017 at
100, ends with 1.6 more worlds, and rendezvouses less (772 orders a game
against 838). Both export with 0 mismatches.

**A ferry to teach Unload loses as written (2026-10-19).** The owner
pointed out that transports moving armies between the seat's own worlds is
how a player reinforces. The executor never did it, so imitation taught the
agent never to unload. A `ferry` switch (`e7cea4c2`, off) has transports
without warships load a world's armies beyond its garrison target and carry
them (Move Only) to the colony furthest short of its own, which `defend`
raises with exposure, and unload there; `style_stats` now counts it (18.5
armies unloaded a game, against none). Paired, one land-grab seat with it
against three default seats, 1,024 maps from 1500000 (Mac mini, owner's
leave): -0.032 +/- 0.016 at 60 years and -0.033 +/- 0.017 at 100. It stays
off. A likely cost: it takes the idle transports a strike force would load
at home.

A second version ferries only to a threatened colony (an enemy force seen
within reach in the last two years) and never starts from the home or
muster world, where strike forces load. Same trial: +0.006 +/- 0.008 at 60
years, -0.013 +/- 0.009 at 100, inside the "doesn't lose" bar, but it fires
rarely: 0.7 armies unloaded a game (256 games at 100 years), since
transports are built at home and seldom wait near a threatened colony. Too
rare to teach Unload by imitation; not adopted yet.

A third version lets one ferry a year leave home once the strike force
there carries the transports the weights want (`transports`, rounded); the
muster world still holds its transports for strikes. Same trial: -0.002
+/- 0.010 at 60 years, -0.018 +/- 0.013 at 100, inside the bar; 2.6 armies
unloaded a game. Transport-only fleets are strike family, so in a hybrid
seat the network orders them and DAgger would label these unloads. But
imitation from the current agent would also relabel its joins "none", since
the executor never joins; teaching Unload this way needs imitation limited
to the transport heads, or the ferry as a PPO teacher term.

**The ferry adopted; the agent taught by a teacher term (owner,
2026-10-19).** The third version is on in `Doctrine::CURRENT` and the
land-grab style (the other styles keep it off, untried). `fleet_ppo.py
--teach-ferry W --teach-until N` adds the executor's mission, target, load
and unload on the learner's convoy rows (transports without warships) as a
log-likelihood term, fading to 0 by iteration `N`, so joins are left as
PPO made them. A 64-game iteration at 100 years has about 3,100 convoy rows
and 120 taught unloads. Run: from t191-orders snapshot 200, `--teach-ferry
1.0 --teach-until 120`, the free-head bonus fading on the same schedule,
200 iterations (`~/.local/state/nostrian-conquest/ai/t191-ferry/`).

**t191-ferry fails the gate (2026-10-19).** Sampled unloads held at about
0.4% of eligible rows after the teacher term faded at iteration 120; the
executor eval rose to 0.463 at iteration 200 (t191-orders: 0.402). Snapshot
200 exports with 0 mismatches. Paired `behaviour_report`, 1,024 maps from
100000, land-grab seat against three executors (ferry now on in CURRENT, so
these numbers differ from the t191-orders gate above):

| Agent | Win share 60 / 100 y | Paired vs promoted 60 / 100 y | Greedy joins / unloads a game, 60 / 100 y |
| --- | --- | --- | --- |
| promoted `fleet-final` | 0.506 / 0.735 | - | 0 / 0 |
| t191-orders snapshot 200 | 0.544 / 0.763 | +0.038 +/- 0.019 / +0.027 +/- 0.018 | 71 / 319, 0 unloads |
| t191-ferry snapshot 200 | 0.493 / 0.657 | -0.014 +/- 0.020 / -0.078 +/- 0.018 | 265 / 898, 0 unloads |

It loses strength and makes no greedy unloads: the unload head's
probability stays under one half, so greedy play never fires it. Joins
roughly tripled. Under the stop rule this was the one variant; t191-orders
snapshot 200 remains the candidate (exploit check pending on bean-void), and
Unload is taken back to the owner.

**Convoys go to the executor's ferry (owner, 2026-10-19).** `Family::Convoy`
(transports without warships, no ETACs) is a fourth family; no agent seat
hands it to the network (`AGENT_FAMILIES`, `behaviour_report`), so the
ferry orders convoys and the network keeps warships and joins. Python's
`fleet_families` mirrors it (3 convoy), so `--families strike` no longer
hands convoy rows to the learner. The seat fixtures' games were re-recorded
with their networks unchanged. `behaviour_report` takes `A.onnx+convoy` to
hand a model its convoys as before, so both can be paired on the same maps:

| Seat, 1,024 maps from 100000 | Paired vs `fleet-final+convoy` 60 / 100 y | Joins / unloads a game, 60 / 100 y |
| --- | --- | --- |
| `fleet-final` (baseline today: `fleet-final+convoy`, 0.506 / 0.735) | -0.010 +/- 0.019 / -0.031 +/- 0.018 | 0 / 0 |
| t191-orders 200+convoy | +0.038 +/- 0.019 / +0.027 +/- 0.018 | 71 / 319, 0 unloads |
| t191-orders 200, convoys to the ferry | +0.083 +/- 0.020 / +0.051 +/- 0.017 | 48.7 / 298.5, 0.4 / 0.6 unloads |

Handing convoys to the ferry makes the candidate stronger (win share 0.589
/ 0.786) and gives it unloads, about one game in two. The plan's use
criterion (greedy joins plus unloads at least one a game) is met; Unload
alone is under one a game. Remaining for t191: the exploit check on
snapshot 200, then promotion with the owner's OK.

**Exploit check clean; promoted; t191 closed (2026-10-20).** 80 iterations
of `--exploit` against snapshot 200 (bean-void), scored greedy on 1,024
maps from 100000 against three copies of the target, in four batches of
256 (all 1,024 at once was killed for memory twice): the target itself
0.245 +/- 0.013; exploiter snapshots 40, 60, 80: 0.177, 0.177, 0.174 (+/-
0.012). No hole. The exploiter trained before convoys moved to the ferry;
the ferry is the executor's scripted code, so the check was accepted
rather than rerun. With the owner's OK, `fleet-final.pt`/`.onnx` are now
t191-orders snapshot 200 (`.onnx` sha256 `a4db043e64baf590...`, export 0
mismatches); the previous agent is kept as `fleet-final-pre-t191`. Models
are backed up with restic to archvps and crunch.

### t207

Teach the agent to use every order, and to find new uses for them.

The owner's goal (2026-10-19): the AI must take advantage of every game
option, above all the orders, and learn new ways to use them. The promoted
agent uses 9 of the 16 missions (behaviour report, 1,024 maps, 100 years):
never Hold Position, Move Only, Seek Home, Guard Starbase, Blitz World,
Join Another Fleet or Salvage. The masks already offer all 15 missions
wherever the game's validators allow them (`nc-ai/src/action.rs`,
`MISSIONS`), plus Join and Unload (t190). The agent was taught them away:
imitation copies an executor that never orders them, and PPO's KL penalty
holds the agent near that copy, so an order it never plays never earns the
reward that would show its use. t191 found the same three causes for Join.

Plan:

1. **Measure.** For each mission, how often it is legal on the agent's
   handed-over rows and how much probability the agent gives it, for the
   promoted agent and t191's result; and which orders the executors use.
   Read the manuals on Blitz, Salvage, Seek Home, Move Only and Guard
   Starbase, so each order's purpose is known before it is judged.
2. **Lift the suppression.** Generalise t191's fixes to the mission head:
   take the KL against a reference smoothed toward every legal mission
   rather than the imitation copy, and give rarely chosen missions an
   exploration bonus that fades over the run, so an order survives only if
   it wins games.
3. **Find new uses.** Self-play and exploiters reward play the scripted
   bots never show. Retry league training (t184 lost) once step 2 is in.
4. **Then the other options.** The same audit for builds, tax, diplomacy
   and starbases.

**Exit:** a retrained agent that does not lose the paired behaviour report
against the promoted agent at 60 and 100 years, and uses more of the order
set greedily: every mission whose use the manuals and the measurement
justify, each ordered at least once a game on average, with the exploit
check clean.


**Step 1, measured (2026-10-19).** The promoted agent, greedy, on its
handed-over strike rows (256 games against executors at 100 years, maps
from 100000, bean-void; it won 0.727):

| Mission | legal on | P where legal | greedy picks |
| --- | ---: | ---: | ---: |
| keep orders | 100% | 0.253 | 28.0% |
| Rendezvous | 100% | 0.615 | 63.8% |
| Bombard | 93.0% | 0.066 | 4.1% |
| Invade | 7.8% | 0.341 | 2.6% |
| Guard/Blockade | 93.7% | 0.046 | 1.5% |
| Blitz | 7.9% | 0.00006 | 0 |
| Patrol | 100% | 0.0008 | 0 |
| View | 100% | 0.0003 | 0 |
| Hold, Move Only, Seek Home, Salvage | 100% | 0.00002-0.00004 | 0 |
| Scout Sector, Scout System | 1.2% | 0.001-0.004 | 0 |
| Guard Starbase, Colonize | 0% | - | - |

The executor issues Rendezvous, Guard/Blockade, Invade, Bombard, Guard
Starbase, Colonize, Patrol, Scout and View, and never Hold, Move Only,
Seek Home, Blitz, Salvage or Join; the network's near-zero choices match
that list. Blitz is the plainest case: legal exactly where Invade is, and
given a two-thousandth of Invade's weight, though the manual makes it the
way to take a lightly defended world intact. Guard Starbase is never legal
on a strike fleet because the seat seldom has a starbase where its fleets
are; that is a build question, for step 4. Hold and Move Only overlap
Rendezvous, which merges on arrival.

**Step 2, design (2026-10-20; awaiting the owner's approval).** Why the
mission head stays shut, from `fleet_ppo.py`: `kl_to` holds every fleet
head, mission included, near the imitation reference, where Blitz has
probability 0.00006. Raising it to 5% costs about log(0.05 / 0.00006) = 6.7
nats of KL on each row where it is legal, far more than one invasion's
reward can repay, so PPO never samples it often enough to learn its worth.
t191's `--free-heads` (no KL, an act-or-not entropy bonus) fits Join and
Unload, which are yes/no choices; on a 16-way mission head an entropy bonus
spreads probability over every legal mission, as the t191 pilot did for
joins (97% of orders).

Which missions should earn a place, from the player manual:

- *Blitz:* lands at once, the warships firing on the batteries first; the
  world's factories and batteries are kept. For lightly defended worlds or
  when speed matters. Legal wherever Invade is. Expected to earn use.
- *Salvage:* scraps a fleet at an owned world for half its build cost,
  paid into that world's treasury. A plausible late-game use for obsolete
  or crippled hulls. Worth offering.
- *Patrol a Sector:* the executor uses it; the network gives it 0.0008.
  Worth offering.
- *View World:* a quick look for warships; scouts do most of it. Offered,
  not required.
- *Hold Position, Move Only, Seek Home:* overlap Rendezvous and the ROE
  retreat (a retreating fleet already seeks home). Offered, not required.
- *Guard Starbase:* never legal on strike rows (step 4). *Join:* t191.

Two changes to `fleet_ppo.py`, both for the mission head only:

1. **Smoothed reference, `--mission-smooth E`.** The KL on the mission head
   is taken against `(1 - E) * reference + E * uniform over the row's legal
   missions`. With `E = 0.05` and ten legal missions, every legal mission
   has at least 0.005 in the reference, so lifting Blitz to 5% costs about
   2.3 nats rather than 6.7, and the reward can decide. Other heads keep the
   plain reference and the KL floor.
2. **Capped, fading coverage bonus, `--mission-bonus C --mission-floor F
   --mission-until N`.** On handed rows, the loss gains `-C * sum over legal
   missions of min(p(m), F)`. It pushes each legal mission up to share `F`
   (0.03) and no further, so unlike an entropy bonus it cannot flatten the
   head; its weight falls linearly to 0 by iteration `N` (120 of 200), so
   the last 80 iterations are judged on reward and KL alone. A mission that
   does not win games sinks back.

Logged per iteration: each mission's sampled share on handed rows.
Python tests: the smoothed reference gives every legal mission at least
`E / legal` and none to illegal ones; the bonus's gradient is zero for a
mission already above `F`.

Run (Mac mini, about 3.5 hours): from the agent t191 promotes, with its
flags plus `--mission-smooth 0.05 --mission-bonus 0.02 --mission-floor 0.03
--mission-until 120`, 200 iterations, `--families strike` (convoys stay with
the ferry).

Gate, as t191: paired `behaviour_report` against the promoted agent (1,024
maps from 100000, 60 and 100 years) inside the bar or better; Blitz,
Salvage and Patrol each ordered greedily at least once a game on average
(the others counted, not required); the exploit check clean; export 0
mismatches. Stop rule: this run and one variant (`E = 0.1`, or the bonus
until 160); if Blitz is still unused, bring the owner the evidence and the
option of an executor rule for Blitz, as Unload went to the ferry.

**Step 2, first run fails (2026-10-20).** From the promoted agent with the
design's flags, 200 iterations (`~/.local/state/nostrian-conquest/ai/t207-missions/`).
The MPS backend ran out of memory twice (iterations 108 and 123); the run
resumed from its saves, the second time with `--minibatch 256`. Sampled
shares of handed rows at iteration 120 (bonus gone) and 200: Patrol 0.5% /
0.5%, View 0.5% / 0.5%, Salvage 0.1% / 0.3%, Hold 0.2% / 0.4%, Seek Home
0.2% / 0.4%, Blitz 0 / 0. Executor eval 0.426 at 200. Snapshot 200 exports
with 0 mismatches. Paired `behaviour_report` against the promoted agent,
1,024 maps from 100000: win share 0.470 / 0.657 at 60 / 100 years, -0.119
+/- 0.019 / -0.129 +/- 0.018; joins 438 / 1,523 a game (promoted 49 /
299); greedy Blitz and Salvage 0, the other missions as before.

Two findings. Orders the agent samples at under 1% never win the greedy
argmax, so greedy play (Hard difficulty) shows none of them; a rare,
situational order like Blitz must dominate its own rows to appear. And
the join head, free of the KL since t191, drifted: sampled joins rose from
0.40 to 0.56-0.60 of rows, and greedy joins multiplied by five, which is
likely most of the strength lost. Not a pass; taken to the owner before
the one allowed variant.

**Owner's call: executor rules first (2026-10-20).** Script Blitz and
Salvage in the executor, each behind its own `Doctrine` switch and
paired-tested as the ferry was; the RL variant waits.

**Blitz switch (2026-10-20).** `Doctrine.blitz`, off everywhere: a strike
lands with Blitz instead of Invade when the target's batteries are known to
be gone (`known_ground_batteries == Some(0)`). A blitz keeps 85-100% of
production times f; an invasion keeps 20-40%. Test: `nc-ai/tests/blitz.rs`.
Paired `executor_matches`, land-grab against land-grab, 1,024 games from
map 1500000: +0.009 +/- 0.016 at 60 years (0.269 against 0.260), +0.015
+/- 0.017 at 100 (0.260 against 0.245). `style_stats` (256 games, 60
years) now counts landings: land-grab orders 10.1 invasions and no
blitzes a game; with the switch, 9.3 blitzes and 1.2 invasions. The rule
fires on most landings and does not lose; it does not reach agent seats,
whose strike orders come from the network.

**Blitz adopted (owner, 2026-10-20).** On in `Doctrine::CURRENT` and the
land-grab style. Agent seats reach it through `hand_to_network`: when the
network's chosen mission is Invade and the seat's doctrine would blitz the
target, the seat orders Blitz; training (`nc-ai/src/batch`) and play
(`nc-ai-onnx`) share it. Test: handing the executor's own blitz-off plan to
the network under the blitz doctrine gives back the blitz-on plan.
`behaviour_report` takes `A.onnx@STYLE` per column for the pairing.
Promoted agent with land-grab against the same with blitz off, 1,024 maps
from 100000: -0.009 +/- 0.018 at 60 years (0.527 against 0.536), +0.011
+/- 0.016 at 100 (0.737 against 0.727); invasions 15.9 / 33.7 a game
become 15.3 / 32.6 blitzes. Neutral: the agent was trained to invade, and
the landing order changes only what a capture keeps. The seat fixtures
were re-recorded (their executors now blitz); the ferry test plays four
maps, since its first two no longer ferry.

**Salvage rule, rejected (2026-10-20).** Crippled hulls were left out:
cripples mend on their own, so scrapping one trades a ship for half its
cost. The rule tried instead: once no known world is colonizable (the land
grab is over), an ETAC fleet without warships salvages at the nearest owned
world, 10 points an ETAC. It fired on about 2.2 ETACs a game, and ETAC
builds did not change (0.42% of build points either way), so there was no
build-and-scrap loop. Paired `executor_matches`, land-grab against
land-grab with `--with salvage`, 1,024 games each: maps from 1500000,
+0.025 +/- 0.009 at 60 years and -0.025 +/- 0.011 at 100; maps from
2500000, -0.018 +/- 0.009 and -0.014 +/- 0.011. Pooled, +0.004 +/- 0.006
at 60 and -0.020 +/- 0.008 at 100: a small loss in long games, likely
because idle ETAC fleets still watch and soak up fire. It fails "doesn't
lose", so the switch was not kept in `Doctrine`; the patch is in the
private `t207-blitz/` run directory.

**Join to reinforce, adopted (owner, 2026-10-20).** The owner's doctrine:
Join Another Fleet reinforces a fleet already on an active mission, where
Rendezvous gathers fleets at a sector to merge there. t189's `reinforce`
trial lost because home warships joined any strike under way instead of
launching a second one. The new `Doctrine.reinforce` fires only for
warships at home that found no strike of their own (after the strike,
home-guard and blockade choices) and joins the nearest strike force on
Bombard, Invade or Blitz whose strength is below `attack_ratio` times its
target's known defence; empty transports load armies first. Test:
`nc-ai/tests/reinforce.rs`. Paired `executor_matches`, land-grab against
land-grab, 1,024 games from map 1500000 (bean-void):

| Rule | 60 years | 100 years | Joins a game (100 years) |
| --- | ---: | ---: | ---: |
| strict: the joiner must bring the host up to the need | +0.007 ± 0.004 | +0.012 ± 0.006 | 0.2 |
| loose: any host short of the need | -0.002 ± 0.011 | +0.010 ± 0.014 | 2.7 |

Both pass "doesn't lose". The owner chose the loose rule, which uses Join
regularly; it is on in `Doctrine::CURRENT` and the land-grab style.
`style_stats` now counts joins and takes `--with`. Joins reach only
executor seats: an agent seat's strike fleets are the network's.

The classic replay test (`the_classic_doctrine_replays_t154_exactly`) had
failed since t194, whose bombardment intel changes what executors learn; it
was not run then. Its values are re-recorded. The seat fixtures' games were
re-recorded too, since their executor seats now join.

**The seat's orders after Blitz and Join (2026-10-21).** An agent seat is
the network's strike fleets plus the executor's colonizers, scouts and
convoys, so the order set is judged per seat. Promoted `fleet-final` on
main (`d49fcf76`), `behaviour_report`, 1,024 maps from 100000, bean-void:
win share 0.523 / 0.715 at 60 / 100 years (opponents now blitz and
reinforce too); 9.7 / 10.0 of 16 missions; at 100 years per game, Blitz
32.3, Invade 1.0, Bombard 64.5, Guard/Blockade 48.8, Patrol 35.2, View
20.3, Scout Sector 2.0, Scout System 6.5, Colonize 13.3, Move Only 0.9,
Rendezvous 744, and 312 joins (the join head). Never ordered: Hold
Position (every fleet's resting state), Seek Home (retreats apply it
themselves), Salvage (rejected) and Guard a Starbase.

**Starbase trials, rejected (2026-10-21).** Guard a Starbase is never
legal for the agent seat's fleets because the land-grab style builds no
starbase: its weight, 0.44, rounds to zero. Paired `executor_matches`,
1,024 games from map 1500000:

| Change | 60 years | 100 years | Starbase guards a game |
| --- | ---: | ---: | ---: |
| land-grab, `keep_starbase` (round the target up: one home base) | -0.019 ± 0.017 | -0.023 ± 0.018 | 0 |
| land-grab, that plus `mend_at_base` | -0.013 ± 0.017 | -0.018 ± 0.018 | 0.4 |
| turtle, `mend_at_base` (it builds bases already) | -0.004 ± 0.004 | -0.005 ± 0.004 | 3.5 (3.3 without) |

`mend_at_base` sent warships at home with crippled ships to Guard a
Starbase there until they mended, the manual's repair use (older cripples
recover a turn sooner). The base costs the land-grab seat more than the
repair returns, and on the turtle the rule changes little because its
guards already post there. Every point estimate is negative, so neither
switch was kept; the patch is in the private `t207-starbase/` run
directory. Guard a Starbase stays in the game's AI through the turtle,
whose seats order it about 3.3 times a game. `style_stats` counts starbase
guards.

**The owner on starbases and Salvage (2026-10-21).** Starbases matter for
their economic and defensive benefits, and the AI should build them and
guard them. Salvage is for a player in deep trouble: under attack, scrap
the weaker ships (destroyers, ETACs, scouts) for quick money to build
batteries and armies against a blitz or invasion. Hold Position and Seek
Home (applied automatically) count as used.

Why the executor builds no bases: a base's growth bonus is half again at
50% tax or lower and fades to nothing at 65%, and the land-grab style taxes
at 62.7% (the agent seat at 60%), so its one base would sit at the home
world, already at its Potential. Its other benefit, 5x build capacity,
spends saved treasury quickly. The seat's actions cannot move a base, and a
world without one can order a base (50 points) only when a year's
Production covers it.

Three switches, each paired against the adopted land-grab executor (1,024
games from map 1500000, bean-void):

| Change | 60 years | 100 years |
| --- | ---: | ---: |
| `colony_starbases`: a colony with its garrison met builds a base before warships | -0.014 ± 0.016 | -0.021 ± 0.017 |
| tax 50% with `colony_starbases` | -0.127 ± 0.015 | -0.136 ± 0.015 |
| tax 50% alone | -0.093 ± 0.016 | -0.129 ± 0.016 |
| `emergency_salvage` (23.5 salvages a game) | -0.007 ± 0.016 | -0.035 ± 0.017 |

`emergency_salvage`: a world with an enemy at war seen in or next to its
sector in the last year, no guard of its own, and a budget short of the
armies and batteries it lacks salvages the fleets there made only of
destroyers, ETACs and scouts. A destroyer returns 3 points and a battery
costs 20, and without a base the world still spends only 1x its
Production a year, so the scrap money comes too slowly.

None passes as it stands. Lower tax is the large loss, and bases do not
recover it. As with mixed hulls (t189), the weights were tuned without
bases, so the next step proposed is a retune with `keep_starbase` and
`colony_starbases` on, letting tax, garrisons and the rest adjust around
them, then the paired check against the adopted style.

**Tax that follows development (2026-10-21).** The owner: tax is an
extremely important lever; keep it low while colonies develop, for growth
and the starbase bonus, and raise it as they mature. The manual says the
same (start at 50%, raise toward 65% near Potential). The executor sets
one rate all game, and the agent seat's strategist, imitated from it,
holds 60% every game. `Doctrine.growth_tax` (off) taxes from 50% up to the
style's rate in proportion to the empire's maturity. Paired against the
adopted land-grab executor, 1,024 games from map 1500000:

| Maturity measure | 60 years | 100 years |
| --- | ---: | ---: |
| mean of each world's Production / Potential | -0.045 ± 0.017 | -0.047 ± 0.017 |
| the same, with `colony_starbases` | -0.044 ± 0.016 | -0.053 ± 0.016 |
| total Production / total Potential (patch in `t207-tax/`) | -0.059 ± 0.016 | -0.042 ± 0.017 |

Each loses about 0.05, less than a flat 50% (about 0.1) but clearly. The
weights (build mix, attack ratio, war year) were tuned for the revenue of a
65% rate, so less money early costs ships the rest of the plan counts on.

**Retuned around starbases and development-driven tax (2026-10-21).** The
owner: low tax pays only when many undeveloped worlds have high Potential.
`growth_tax` now mixes a new tuned weight, `young-tax`, with `tax` by the
empire's total Production over total Potential, so rich undeveloped worlds
weigh most. Two retunes with one budget (`tune_executor --against-default`,
30 generations, 24 candidates, 64 maps, 100 years, bean-void): a control
of land-grab as adopted, and a candidate with `growth_tax`,
`keep_starbase` and `colony_starbases` on, starbases bounded 0.1-3 and
`young-tax` 20-65, starting at 50. The control ended at tax 65, starbases
0.15; the candidate at tax 65, `young-tax` 60.1 (up from 50), starbases
1.64. Paired on 1,024 untouched maps from 2500000:

| Comparison | 60 years | 100 years |
| --- | ---: | ---: |
| candidate against control | -0.044 ± 0.017 | -0.037 ± 0.017 |
| candidate against adopted | -0.031 ± 0.017 | -0.055 ± 0.018 |
| control against adopted | +0.013 ± 0.018 | -0.018 ± 0.018 |

The engine's growth matches the manual: a world closes (100 - tax) / 400
of its gap a year, half again with a base at 50% or less. One young world
(Potential 100, Production 10), cumulative revenue at years 5 / 10 / 20 /
40: 65% flat 101 / 284 / 788 / 2006; 50% until 80% grown, then 65%, 97 /
268 / 805 / 2055; the same with a base (50 points) 74 / 303 / 895 / 2162;
a base at 65% 51 / 234 / 738 / 1956. Low tax pays where young rich worlds
have bases, as the owner says. The executor cannot get them there: a world
without a base can order one only when a year's Production covers 50
points, so colonies get bases only once well grown, and the seat's actions
cannot move a base, so the home world (5x capacity) cannot build them for
its colonies. Tax is empire-wide, so lowering it costs the mature home
world with no growth in return. Next: give seats Starbase Command moves, so
the home world builds bases and sends them to its richest young colonies,
then retry `growth_tax`.

The owner chose a rule change instead (2026-10-21): a world with no
starbase in orbit or in its stardock may pay for one base from its own
treasury past its build capacity ([price decision](../investigations/y95fnqkm-unit-prices-and-opening-resources-under-the-approved-economy.md),
amended). `colony_starbases` now saves: a colony of Potential 80 or more
with its garrison met banks its budget until it can order the base. Paired
against land-grab as adopted, 1,024 maps from 1500000, bean-void:

| Trial | 60 years | 100 years |
| --- | ---: | ---: |
| saving `colony_starbases`, before the rule | -0.013 ± 0.017 | -0.009 ± 0.018 |
| saving `colony_starbases`, with the rule | -0.014 ± 0.017 | -0.002 ± 0.018 |
| `growth_tax` from `young-tax` 50 | -0.059 ± 0.016 | -0.042 ± 0.017 |
| the same, with `colony_starbases` | -0.023 ± 0.017 | -0.024 ± 0.017 |

Saving for colony bases now passes "doesn't lose" at both lengths. A low
young-empire tax still loses, though bases halve the loss.

Adopted with the owner's OK (2026-10-21): `colony-starbases` is now a
personality switch, on in land-grab and `Doctrine::CURRENT`, off in the other
styles. The ONNX seat fixtures were re-recorded.

Land-grab never guards its bases: its guard posts need the allocation to
hold a share back, and with `allocate` off it is always 0. The
`guard_bases` switch (off) sends a warship fleet at rest to Guard a
Starbase at the nearest threatened world of the seat's with a base and no
guard, whatever the allocation. Paired as above: -0.016 ± 0.015 at 60
years, -0.030 ± 0.017 at 100, with 10.5 starbase guards a game (none
before). Guards cost land-grab warships its strikes need; not adopted yet.
Its 95% interval at 100 years (-0.063 to +0.003) does include zero, so the
loss is not certain.

A fleet resting beside its own base for a whole turn mends its older
cripples a turn sooner (manual, Starbase Recovery). The executor never
used this. `repair_at_bases` (off) sends a crippled warship fleet at rest
or seeking home to Guard a Starbase at the nearest world of the seat's
with a base, holds it there while it mends, then returns it to Hold.
Paired as above:

| Trial | 60 years | 100 years |
| --- | ---: | ---: |
| `repair_at_bases` | -0.005 ± 0.008 | -0.001 ± 0.011 |
| `repair_at_bases` and `guard_bases` | -0.016 ± 0.016 | -0.019 ± 0.017 |

Repair is neutral and passes "doesn't lose"; with it, guarding loses less
(-0.030 to -0.019 at 100 years).
Adopted with the owner's OK (2026-10-21): `repair-at-bases` is a
personality switch, on in land-grab and `Doctrine::CURRENT`.

#### Step 2 plan: let the agent choose everything (2026-10-21)

The owner's aim: the learned agent tries every order, tax rate and build,
and keeps what wins, rather than playing what a script dictates. Today the
strategist sets tax and the other goals, the fleet agent orders the
handed-over families (colonize, scout, strike) with Join and Unload, and
the executor still builds and orders every other fleet. The masks already
offer every legal mission, target, tax rate and build. What holds the agent
to the script is training: it starts as an imitation of the executor, the
KL penalty never fades below `--kl-floor 0.02`, the network never builds,
and its rivals are scripted. Scripted doctrines that pass "doesn't lose"
stay available as executors to start from and play against, never as rules.

1. **A build head.** The env already takes per-world builds (`build`,
   `amount` in `BUILD_SLOTS` slots; see `random_agent.py`), with masks that
   include the first-starbase rule. Add the head to `fleet_policy.py`,
   imitate the executor's builds into it (`fleet_imitate.py`), hand builds
   to a learner seat with a `--hand-builds` switch, export it to ONNX and
   the Rust seat (`nc-ai-onnx`), and re-record the fixtures. Verify: build
   imitation accuracy reported; seat replays pass; export 0 mismatches.
2. **Hand over every family.** The hybrid hands the network all fleets,
   so the executor makes no choice for a learner seat.
3. **Free every head.** Generalise t191's `--free-heads` to the mission,
   target, tax and build heads: KL fades to 0 (`--kl-floor 0`), and a
   rarity bonus on legal choices the policy seldom takes fades out by
   `--free-entropy-until`. After that only winning keeps a choice.
4. **Reward results only:** win share and production share, as now. No
   bonus for any particular order.
5. **Varied rivals.** The t184 league: the scripted styles, frozen past
   snapshots, the learner's own recent snapshots, and exploiters trained
   against it.

Runs:

- **Mac mini (M4 Pro, 12 cores, 24 GB), first.** From promoted
  `fleet-final.pt`: steps 1-3 with the league off, 200 iterations of 64
  games at 100 years, `--free-entropy-until 120`, about 4-6 hours. This
  checks that the build head and freed heads train without collapse.
- **crunch (Radeon RX 7900, ROCm; 32 cores), when the owner frees it.**
  crunch is busy with other work, so nothing runs there until the owner
  says so. The long run there: the league on, 256 games an iteration on
  the 32 cores, the network on the GPU, 1,000 iterations or more, with
  exploiters every 200. Check ROCm PyTorch on the 7900 first with a two
  iteration smoke run.

Exit, per run, against promoted `fleet-final` on the paired behaviour
report (1,024 maps, 60 and 100 years, mixed builds):

- **Strength:** the interval includes zero or better.
- **Use:** the report counts every mission, tax rates by year and builds
  by kind; the run must use more of them than the promoted agent, with tax
  that varies within games and starbases built at colonies.
- **Exploit check:** 80 iterations of `--exploit` find no hole.
- **Export:** 0 mismatches.

Stop rule: the Mac mini run plus one variant (bonus 0.05, or a later
fade). If greedy play still uses no new orders or loses strength, bring the
evidence to the owner before the crunch run rather than tuning further.
Models stay private; promotion only with the owner's OK.

**Step 2, first results (2026-10-21).** The build head is exported to
ONNX, and the Rust seat builds when its families hand builds over
(`tiny-builds` fixture; 0 export mismatches). `build_imitate.py` taught
promoted `fleet-final`'s build head the executor's builds:

| Build head | Accuracy | Greedy, 60 y | Greedy, 100 y |
| --- | ---: | ---: | ---: |
| Linear on the world token | 0.50 | -0.222 +/- 0.020 | -0.369 +/- 0.019 |
| + global token, goals, hidden layer; factored greedy | 0.53 | -0.143 +/- 0.020 | -0.247 +/- 0.020 |

These come from `behaviour_report`, 1,024 maps from 100000, against
promoted `fleet-final` without builds. Factored greedy is `greedy_join`'s
rule: build when building at all is likelier than "none". A plain argmax
built about 15% too little. Accuracy stays near half because the executor
fills worlds in turn from one empire-wide stock (ETACs, scouts,
transports), which a per-world head cannot see. The learner ends with
fewer worlds: 7.3 at 100 years against 11.9.

Handing over every fleet family at once fell to 0.03 sampled wins at
iteration 1, against 0.22 for strike only and 0.13 for strike and
builds. So the Mac mini run (`t207-builds/run.sh`) hands over strike and
builds only.

**Build as the executor would (owner, 2026-10-21).** Imitation could not
close the gap, so the build head gained a choice, `BUILD_EXECUTOR`: a world
whose first slot takes it keeps the executor's builds. The head starts
there (about 94%), so greedy play equals promoted `fleet-final` exactly
(paired +0.000 on 96 maps). Sampled play tries other builds at a few worlds
a year, and the agent keeps those that win. The run starts from
`fleet-final.pt` with no build imitation; its first sampled win share was
0.19, against 0.22 for strike only.

**Mac mini run, 200 iterations (2026-10-21).** Sampled builds of a kind
other than the executor's rose from 0.5 to 3.8 per seat-year. Greedy evals
against the executor fell to 0.14 at iteration 75, then rose to 0.41 by
200, once the KL had faded. Paired `behaviour_report` against promoted
`fleet-final`, 1,024 maps from 100000:

| Snapshot | 60 years | 100 years | Tax rates a game | Joins a game, 100 y |
| --- | ---: | ---: | ---: | ---: |
| 150 | -0.002 +/- 0.020 | +0.013 +/- 0.018 | 1.3 | 931 |
| 175 | -0.005 +/- 0.020 | -0.021 +/- 0.019 | 1.9 | 1,094 |
| 200 | -0.072 +/- 0.020 | -0.056 +/- 0.019 | 2.4 | 1,328 |

Promoted `fleet-final` uses one tax rate a game and joins 441 times.
Snapshot 150 passes strength at both lengths, and builds more transports
(138 against 112 at 100 years). The starbase share stays at 2%. Later
snapshots vary tax more and lose: the strategist's tax head drifts once
nothing holds it, as the join head did in t191.

**Exploit check on snapshot 150 (2026-10-22): a hole.** 80 iterations of
`--exploit` from `fleet-final`, strike and builds handed over on both
sides, ran on bean-void. Two things had to be fixed first. Frozen agents
now choose their builds, since without that a target that hands over
builds built nothing. The scorer now plays 256 games at a time, since all
1,024 at once ran out of memory. Greedy, 1,024 maps from 100000, 60 years:

| Seat against three copies of snapshot 150 | Win share |
| --- | ---: |
| Snapshot 150 itself | 0.256 +/- 0.014 |
| Exploiter, iteration 40 | 0.385 +/- 0.015 |
| Exploiter, iteration 60 | 0.428 +/- 0.015 |
| Exploiter, iteration 80 | 0.416 +/- 0.015 |

Earlier exploiters held promoted agents to 0.17-0.25 (t191, t202). The
hole is not in the build head: with the executor building on both sides,
the iteration-60 exploiter still scores 0.426 +/- 0.022 (512 games). The
same exploiter scores 0.348 +/- 0.021 against promoted `fleet-final`, which
t183's exploiter also reached (0.377). So snapshot 150 is more exploitable
than the promoted agent in its goals and fleet orders, likely the tax and
join drift once the KL reached 0. It fails the gate and is not promoted.
Per the stop rule, the evidence goes to the owner before any variant or
crunch run.

**Variant: KL floor 0.02 off the build heads (owner, 2026-10-22).** The
same run with `--kl-floor 0.02`, and only join, unload, build and amount
free. The free-head bonus on builds now rests on `BUILD_EXECUTOR`, not
"none". Final KL to `fleet-final` was 1.1, against 15.8 without the floor.
Paired `behaviour_report` against promoted `fleet-final`, 1,024 maps:

| Snapshot | 60 years | 100 years | Joins a game, 100 y |
| --- | ---: | ---: | ---: |
| 150 | -0.094 +/- 0.020 | -0.120 +/- 0.019 | 1,053 |
| 175 | -0.107 +/- 0.021 | -0.120 +/- 0.019 | 1,082 |
| 200 | -0.014 +/- 0.020 | -0.010 +/- 0.018 | 141 |

Tax stays at one rate a game (60%), and starbases at 2% of builds.

**Exploit check on snapshot 200** (`exploit-t207c`, bean-void): 80
iterations of `--exploit`, each snapshot scored over 1,024 games in
chunks of 256. The target scores 0.246 +/- 0.013 against the same
panel. The exploiter scores 0.399 at iteration 40, 0.396 at 60 and
0.386 +/- 0.015 at 80, against 0.416 for the free run. The floor keeps
strength but does not close the hole, so the snapshot fails the gate.

**Neither run learned to build.** A new `behaviour_report` row counts
world-years the network builds its own way: 0.0% greedy for snapshot 200
here and snapshots 150 and 200 of the free run. The first slot's
probability of `BUILD_EXECUTOR` moved only from 0.94 to 0.91-0.94 in
year one. The run log's builds by kind had counted the second slot,
which is ignored when a world defers; it now counts only builds that take
effect. Sampled play builds its own way about twice a seat-year, but no
build wins often enough to move greedy play. So any gain here is in fleet
orders, not builds.

**Literature search (2026-10-23).** [Training techniques for the learned
agent's builds, drift and rare orders](../investigations/g6uo9u0x-training-techniques-for-the-learned-agents-builds-drift-and-rare-orders.md)
holds the findings and a ranked, cheapest-first set of experiments. In
short, the anchor's KL charges for every deviation from the executor, and
a single build's credit is lost in the turn's noise. The first step is a
paired test of whether forced build overrides change outcomes at all.

**Path forward (owner, 2026-10-23).** Diagnostics first (t212), then
the anchor changes (t213, t214), then self-imitation (t215). Build credit
and head changes (t216) wait on t212's paired test. Reward shaping (t217)
waits on the owner. League changes stay with t184 on crunch. Each run
uses the t207 gate: paired `behaviour_report` against promoted
`fleet-final`, the 80-iteration exploit check scored against a fixed
panel, and 0 export mismatches. Rows are from the [investigation's
ranked plan](../investigations/g6uo9u0x-training-techniques-for-the-learned-agents-builds-drift-and-rare-orders.md#validation-and-remaining-work).

### t212

Diagnose before training more (Mac mini, hours, no training).

1. Paired build test: on the same maps and seeds, play snapshot 200's
   seat with the executor's builds against forced network builds (the
   head's best non-executor kind) at a sampled share of worlds. Report
   the win share difference with its standard error.
2. Critic explained variance by game year, on the run's own games.
3. Deployment sweep on the build, tax and mission heads: temperature 0,
   0.3, 0.5, 0.8, and a margin rule (play a non-default choice when its
   probability passes 0.25). Paired `behaviour_report` against greedy.

Exit: the paired build delta, the explained-variance curve, and the
sweep table recorded here. If the build delta is within 2 standard
errors of zero, t216 is dropped for builds. A deployment rule 2 standard
errors better than greedy goes to the owner for adoption.

**Paired build test (2026-10-23).** Snapshot 200 of the KL-floor run,
greedy, one seat against three executors, 60 years. At a share of its own
world-years each year, its first build slot was forced to the build
head's likeliest unit kind in place of `BUILD_EXECUTOR`. The two arms
match exactly with nothing forced. The script was a scratch file,
`paired_builds.py`, wrapping `fleet_ppo.play_year`.

| Forced share | Maps | Win share change |
| ---: | --- | ---: |
| 0.05 | 100000-100511 | +0.037 +/- 0.028 |
| 0.05 | 100512-101023 | -0.042 +/- 0.027 |
| 0.10 | 100512-101023 | -0.060 +/- 0.027 |
| 0.25 | 100000-100511 | -0.193 +/- 0.028 |

Builds matter: overriding a quarter of world-years costs 0.19 of win
share. The head's own favourite kind is worse than the executor's build
on average, at about -0.008 of win share per 1% of world-years
overridden. Across all 1,024 maps the 5% result is near zero (-0.003 +/-
0.020). So a deviation's effect is large enough to learn from, but the
head has not yet learned which deviations help. t216 stays in.

**Critic explained variance** of the final reward, by year, in the same
games:

| Year | 0 | 10 | 20 | 30 | 40 | 50 | 55 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Explained variance | 0.02 | 0.05 | 0.07 | 0.14 | 0.23 | 0.37 | 0.43 |

The value estimate explains almost nothing of the outcome for the first
20 years, when most builds and colonies are chosen. So an early build's
advantage is mostly noise, which matches the research note's credit
diagnosis. A better critic (more inputs, such as every seat's state in
training) and per-decision credit (t216's branch rollouts) are the
levers.

**Deployment sweep (2026-10-23).** Snapshot 200 sampled at temperature T
on every head (`behaviour_report`'s `~T` suffix, seeded by map and
year), paired against its own greedy play on 1,024 maps from 100000,
one seat against three executors:

| Temperature | Win share, 60 y | Win share, 100 y | Own builds | Tax rates a game, 100 y | Salvage a game, 100 y |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 0 (greedy) | 0.543 | 0.723 | 0.0% | 1.0 | 0.0 |
| 0.3 | -0.006 +/- 0.020 | +0.025 +/- 0.018 | 0.3% | 1.0 | 0.0 |
| 0.5 | +0.006 +/- 0.019 | +0.000 +/- 0.018 | 4.5% | 1.2 | 0.0 |
| 0.8 | -0.145 +/- 0.019 | -0.189 +/- 0.019 | 21.4% | 2.6 | 1.1 |

Guard Starbase rises from 0.5 to 4.2 a game at 0.8. Temperature 0.5
plays as well as greedy while building its own way at 4.5% of
world-years. No temperature is 2 standard errors better than greedy,
so none goes to the owner for adoption. At 0.8 the rare orders appear
but cost 0.15-0.19 of win share, the same cost as forcing builds. The
margin rule was not run: the sweep already shows the low-probability
choices lose. Sampling helps only if training first makes those choices
good, which is t213-t216's job.

### t213

Separate KL weights per head (one Mac mini run). Tax and join keep the
0.02 floor. The build and rare-order heads take a KL to the anchor
blended with ε = 0.1 of an even spread over legal choices. PPO's ratio
clip and advantage normalization are taken per head.

Exit: an exploiter scores 0.25 or less; the first slot's probability of
`BUILD_EXECUTOR` falls below 0.80; paired strength within the t207 gate.

**Run (2026-10-23, Mac mini).** `~/.local/state/nostrian-conquest/ai/t213/`:
from promoted `fleet-final` with a fresh build head, strike and builds
handed over, `--kl-floor 0.02` on every head but unload,
`--build-smooth 0.1 --mission-smooth 0.1 --split-clip`, free entropy
0.03 fading to 0 by iteration 120, 200 iterations of 64 100-year games.
Final KL to the anchor was 1.4. Snapshots 150, 175 and 200 export with 0
mismatches. Paired `behaviour_report` against promoted `fleet-final`,
1,024 maps from 100000, three executors:

| Snapshot | 60 years | 100 years | Joins a game, 100 y |
| --- | ---: | ---: | ---: |
| 150 | +0.032 +/- 0.020 | +0.037 +/- 0.018 | 737 |
| 175 | +0.062 +/- 0.019 | +0.067 +/- 0.017 | 272 |
| 200 | +0.059 +/- 0.020 | +0.083 +/- 0.016 | 486 |

Snapshot 200 beats `fleet-final` by more than 2.9 standard errors at
both lengths. Its gain is in fleet orders: greedy play still builds its own way at 0.0% of
world-years, uses one tax rate a game, and puts 2% of builds into
starbases. Sampled play builds its own way at 11% of the builds that
take effect (4.5% at iteration 1), so the first slot's probability of
`BUILD_EXECUTOR` is still near 0.89 and the build exit is not met.

**Exploit check on snapshot 200** (`exploit-t213`, bean-void): 80
iterations of `--exploit`, scored over 1,024 games. A first scoring ran
against t207c's target by mistake (the scorer's directory was copied
unchanged) and is kept as `score-wrong-target.txt`. Rescored against
t213's snapshot 200, the target scores 0.259 +/- 0.014 against copies of
itself and the exploiter 0.281 at iteration 40, 0.326 at 60 and 0.348
+/- 0.015 at 80. That fails the 0.25 exit but is the least exploitable
of the t207-t213 runs (0.416 free, 0.386 with the floor). Promoted
`fleet-final` shows holes of the same size to exploiters trained in
t183 and t207 (0.377 and 0.348), so snapshot 200 is stronger than
`fleet-final` and no more exploitable by this measure. Promotion is the
owner's call.

**Promoted (owner, 2026-10-24).** Snapshot 200 is now `fleet-final`
(`.pt` eb9d2688…, `.onnx` ebbea298…); the previous agent is kept as
`fleet-final-pre-t213`, and the models are backed up with restic. t213
stays open for its build and exploit exits.

### t214

A moving anchor (one Mac mini run): a KL to the agent's own snapshot
from one save earlier (TStarBot-X's DAPO), or to a trailing average of
the policy (Magnetic Mirror Descent, η̃ about 0.05). It replaces the
fixed imitation floor; a variant keeps the floor on tax and join.

Exit: beats the 0.02-floor agent above 0.27 over 1,000 games; an
exploiter scores 0.25 or less; tax and join drift no more than with the
floor.

**Run (2026-10-23, Mac mini).** `fleet_ppo.py --anchor-ema H` moves the
KL reference `H` of the way toward the policy after each iteration (the
trailing-average form; the reference is saved for `--resume`).
`~/.local/state/nostrian-conquest/ai/t214/` repeats t213's flags with
`--anchor-ema 0.05`, so the anchor is the only change and t213's
snapshot 200 is the baseline to beat. The 0.02 floor now holds the policy
near its recent self rather than near the imitation copy.

Final KL to the moving reference was 0.59. Snapshots 150, 175 and 200
export with 0 mismatches. Paired `behaviour_report` against t213's
snapshot 200, 1,024 maps from 100000, three executors:

| Snapshot | 60 years | 100 years | Tax rates a game |
| --- | ---: | ---: | ---: |
| 150 | -0.076 +/- 0.020 | -0.076 +/- 0.017 | 1.2 |
| 175 | -0.063 +/- 0.019 | -0.083 +/- 0.017 | 1.0 |
| 200 | -0.006 +/- 0.019 | -0.026 +/- 0.017 | 1.4 |

The trailing anchor does not beat t213, so the strength exit fails and
no exploit check was run. It is the first run whose greedy play varies
the tax rate (1.4 rates a game), but builds are still the executor's at
every world-year. With a 0.05 trail the anchor follows the policy within
about 14 iterations, so the 0.02 floor restrains little; a smaller share
(0.01) or a DAPO snapshot anchor are the remaining variants.

**Slower trail, t214b (2026-10-24, Mac mini).**
`~/.local/state/nostrian-conquest/ai/t214b/` repeats t214 with
`--anchor-ema 0.01`, from the same start (`fleet-final-pre-t213`).
Snapshots 150, 175 and 200 export with 0 mismatches. Paired
`behaviour_report` (`+builds`) against `fleet-final` (t213's snapshot 200),
1,024 maps from 100000, three executors:

| Snapshot | 60 years | 100 years |
| --- | ---: | ---: |
| 150 | -0.011 +/- 0.019 | -0.047 +/- 0.017 |
| 175 | -0.027 +/- 0.019 | -0.053 +/- 0.016 |
| 200 | -0.002 +/- 0.019 | -0.025 +/- 0.016 |

`fleet-final` itself scores 0.617 and 0.816 here. No snapshot beats it,
and none varies the tax rate or builds its own way greedily, so the slower
trail loses t214's one gain. **t214 is closed:** a trailing anchor does not
beat the fixed 0.02 floor at either rate. The DAPO snapshot anchor was not
run; it is the same idea with a stepped trail and is not expected to do
better.

**t213 is closed (2026-10-24).** Its strength exit was met and promoted.
Its build exit (`BUILD_EXECUTOR` below 0.80) and exploit exit (0.25) were
not; they carry to t215 and t216, which target exactly those failures.

### t215

Self-imitation (UPGO) and a KL weight drawn each game from {0, 0.005,
0.02, 0.1}, as Diplomacy's DiL-piKL does.

Exit: 2 standard errors over t214's best on the panel, or more greedy
use of rare orders at equal strength.

**Split (2026-10-25).** A KL weight drawn per game only works as in
DiL-piKL when the network is told the weight; ours has no such input, so a
random draw would act as a fixed weight of about 0.03 plus noise. That half
waits for a weight input to the network (export and Rust seat changes) and
is built only if self-imitation shows promise.

**Self-imitation run (2026-10-25, Mac mini).** `fleet_ppo.py --upgo C`: each
seat-year's upgoing return `G_t = r_t + gamma * max(V_{t+1}, G_{t+1})`, and
`C * max(0, G_t - V_t)`, in units of the GAE advantage's spread, joins the
policy advantage; the value target is unchanged. At `C = 1` a smoke run's
policy loss was -2.4 (about 0.01 normally) and entropy fell; at 0.3 entropy
holds at the baseline's level. `~/.local/state/nostrian-conquest/ai/t215/`
repeats t213's flags with `--upgo 0.3`, from the same start; t214 is closed,
so `fleet-final` (t213's snapshot 200) is the baseline.

**Result (2026-10-25).** Snapshots 150, 175 and 200 export with 0
mismatches. Paired `behaviour_report` (`+builds`) against `fleet-final`
(0.617 / 0.816), 1,024 maps from 100000, three executors:

| Snapshot | 60 years | 100 years |
| --- | ---: | ---: |
| 150 | -0.039 +/- 0.019 | -0.070 +/- 0.016 |
| 175 | -0.029 +/- 0.020 | -0.034 +/- 0.016 |
| 200 | -0.013 +/- 0.019 | -0.018 +/- 0.017 |

Snapshot 200 ties; greedy play still builds its own way at 0.0%, uses one
tax rate a game and never salvages. **t215 is closed:** self-imitation does
not beat `fleet-final`; the per-game KL weight is not built.

### t216

Build credit and build head structure, only if t212's paired test shows
forced overrides change outcomes. First, branch rollouts from saved
states: about 5% of world-years, three branches differing only in that
world's build, about 10 years each, common seeds; their return
difference is the build head's advantage. Then, if still flat, a gated
head ("override?", then the kind), or a build mix the strategist holds
for several years. Branch rollouts need many CPU cores; crunch suits
them once the owner frees it.

Exit: a paired override delta over 2 standard errors, and a panel gain.

**Branching (2026-10-25).** `BatchEnv::branch` (Python `Env.branch`) copies
games mid-play, state, random streams and turn log included; a Rust test
shows a copy replays its source exactly and copies given different choices
part.

**Branch measurement (2026-10-25, Mac mini).** `fleet-final` greedy, one
seat against three executors, 512 maps from 100000. At year Y each game is
copied three times, differing only in one learner world's first build slot:
the executor's build, the head's likeliest other kind, or a random other
legal kind. The table gives the change in the seat's production-share lead
against the executor branch (mean +/- standard error, then the mean size of
the per-game difference), and the lead's spread across games:

| Fork | Horizon | Head's other kind | Random kind | Lead spread |
| ---: | ---: | --- | --- | ---: |
| 5 | +10 y | -0.0002 +/- 0.0005 (0.003) | +0.0000 +/- 0.0007 (0.006) | 0.070 |
| 5 | +30 y | -0.0014 +/- 0.0044 (0.057) | +0.0026 +/- 0.0056 (0.097) | 0.145 |
| 15 | +10 y | -0.0005 +/- 0.0019 (0.028) | -0.0019 +/- 0.0018 (0.026) | 0.102 |
| 15 | +30 y | -0.0039 +/- 0.0084 (0.145) | -0.0037 +/- 0.0081 (0.142) | 0.197 |

One build changes a game a lot within decades, but in no direction: the
mean effect is zero within a standard error at every fork and horizon,
while per-game divergence grows to most of the lead's spread. The
executor's build is not beaten by the head's favourite or by chance, so a
single world-year build carries almost no learnable signal, and branch
credit would measure chaos, not value. This explains t207-t217: the build
head stays on `BUILD_EXECUTOR` because nothing better is there to find at
that grain. **t216's branch-rollout route is closed.** Builds stay with the
executor; a build mix held over several years is the only build lever left,
and it is not pursued without the owner.

### t217

Owner decision: an economic-lead reward shaping term. The potential is
the seat's production share minus its rivals' mean, zero at game end and
on elimination. Potential-based shaping keeps the best strategy
unchanged (Ng 1999; Grześ 2017), but it is still a reward change.
Optional: a novelty bonus for unusual orders that fades to zero. Nothing
is built until the owner approves.

**Approved (owner, 2026-10-24).** Shaping yes; the potential is the
production-share lead (share minus the rivals' mean, 0 before the first
year, at game end and for a seat without worlds); no novelty bonus, since
free entropy already plays that part and t212 showed forced rare orders
lose; and run alone before t215, so a gain has one cause.
`fleet_ppo.py --shape-lead W` adds `W * (gamma * phi(next) - phi(now))` to
each learner's yearly reward, with the run's own `--gamma`; logged win
shares stay unshaped. Unit tests check the potential and that the terms
telescope to zero over a game.

**Run (2026-10-24, Mac mini).** `~/.local/state/nostrian-conquest/ai/t217/`
repeats t213's flags with `--shape-lead 1.0`, from the same start, so
t213's snapshot 200 (`fleet-final`) is the baseline. Exit: paired gain
over `fleet-final` above 2 standard errors at 60 or 100 years without a
loss at the other; report the critic's explained variance by year against
t212's (0.02-0.07 over years 0-20) and the share of builds made its own way.

**Result (2026-10-25).** Snapshots 150 and 175 export with 0 mismatches,
200 with 1 mismatched output of 64 recorded seats. Paired
`behaviour_report` (`+builds`) against `fleet-final` (0.617 / 0.816), 1,024
maps from 100000, three executors:

| Snapshot | 60 years | 100 years | Tax rates a game |
| --- | ---: | ---: | ---: |
| 150 | -0.036 +/- 0.019 | -0.085 +/- 0.017 | 1.1 |
| 175 | -0.019 +/- 0.019 | -0.040 +/- 0.017 | 1.0 |
| 200 | -0.002 +/- 0.019 | -0.031 +/- 0.016 | 1.2 |

The exit fails: weight 1.0 does not beat `fleet-final`, greedy play still
builds its own way at 0.0% of world-years and starbases stay at 2% of
builds. Training win share (0.232 over the last 25 iterations) ran slightly
above t214b's (0.219). The critic's explained variance was not measured.
The shaping's weight was not tuned against the final reward; that is the
open variant.

**Critic check (2026-10-25).** Greedy, one seat against three executors,
100 years, 1,024 maps from 100000; explained variance of the final win
share by year. `fleet-final`'s critic is scored as is; t217's predicts the
shaped return, so the lead is added back (`V + phi`). Squared correlations
with the lead added to both critics are in the last two columns.

| Year | fleet-final V | t217 V + phi | fleet-final r2(V + phi) | t217 r2(V + phi) |
| ---: | ---: | ---: | ---: | ---: |
| 0 | 0.036 | 0.030 | 0.047 | 0.038 |
| 5 | 0.071 | 0.093 | 0.115 | 0.108 |
| 10 | 0.092 | 0.110 | 0.127 | 0.127 |
| 20 | 0.140 | 0.155 | 0.182 | 0.178 |
| 40 | 0.334 | 0.386 | 0.375 | 0.403 |
| 60 | 0.517 | 0.644 | 0.540 | 0.646 |
| 80 | 0.639 | 0.727 | 0.655 | 0.733 |

Over years 0-20 the shaped critic knows no more than `fleet-final`'s critic
plus the lead itself; it gains only from year 40 on, after most builds are
made. At year 10 nothing measured explains more than about 13% of the
outcome, a limit set by the game's horizon, not the reward. **t217 is
closed:** shaping stays available as `--shape-lead` (off by default); no
weight variant is run. Early credit is t216's job (branch rollouts).

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

**Result (2026-10-18): named, and absent from the deployed agent.**
`behaviour_report` gained `--opponents MODEL.onnx`, which fills the other
three seats with that model (land-grab style) instead of executors. The
exploiter (iteration 80 of t183's crunch run, copied to the mini as
`~/.local/state/nostrian-conquest/ai/t192/exploit-0080.pt` and exported
with 0 mismatches) and a control each played one seat against three copies
of the target, on the same 512 maps of 60 years from 100000, greedy:

| Against three pre-sweep agents | Control (pre-sweep) | Exploiter |
| --- | ---: | ---: |
| win share | 0.239 | 0.319 |
| invasions, years 1-30 | 4.81 | 5.01 |
| invasions, after year 30 | 10.11 | 12.69 |
| Guard/Blockade World orders | 2.7 | 6.3 |
| Invade World orders | 18.4 | 22.9 |
| worlds lost | 12.6 | 14.6 |
| mean armies on a world when lost | 2.2 | 1.3 |
| worlds at end | 5.1 | 5.6 |

Paired: win share +0.080 ± 0.025, worlds at end +0.57 ± 0.13, worlds lost
+1.99 ± 0.19. **The weakness:** the opening is the same (early invasions
level), but after year 30 the exploiter blockades the target's worlds more
than twice as often and invades a quarter more. The pre-sweep agent did not
hold off this late pressure. The exploiter traded thin garrisons for tempo, losing more worlds
itself but ending with more. This is a reading of the measurements; no
ablation isolated the blockades as the cause.

Against today's `fleet-final` (the t202 retrain), on the same maps:
control 0.263, exploiter 0.151 (paired −0.111 ± 0.023). The exploiter
loses outright, and the fresh 80-iteration exploiter trained against that
agent on bean-void (t189's t202 section) found no hole either. The weakness
is absent from the deployed agent; no fix is needed. Reports are in
`~/.local/state/nostrian-conquest/ai/t192/`.

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

**Progress (2026-10-16).** Done: the league plays strategists greedy
(`league.py`); PPO evaluation plays a fresh block of maps from 5,000,000 at
each check and prints a standard error, so nothing is picked on the maps
used for final confirmations (`fleet_ppo.py`); `behaviour_report` prints
each model's paired difference from the first, with its standard error;
imitation prints free-running exact-row accuracy beside the teacher-forced
figures (`fleet_imitate.py`); `seat_matches`, `behaviour_report`,
`game_log` and `executor_matches` take `--style FILE.kdl` through
`personality_or_file`. Left: play a fresh league with both greedy to
re-rate.

**Exit:** the league is re-rated with both greedy, evaluation maps are held
out, and the report prints paired differences with their standard errors.

**Re-rated (2026-10-18).** A fresh league, every agent greedy, at
`8ca63bb9` on the Mac mini: the same twelve agents with today's
checkpoints (`fleet-final.pt` is the t202 retrain, so the old league's
fleet entry was stale anyway), 2,048 matches of 60 years from seed 1, in
`~/.local/state/nostrian-conquest/ai/league-t193.db`. `verify` replayed all
2,048 and rebuilt the table exactly.

| Agent | Rating | Win share |
| --- | ---: | ---: |
| fleet | 39.86 | 0.717 |
| executor | 30.10 | 0.393 |
| final (strategist) | 29.75 | 0.391 |
| allocation (strategist) | 28.76 | 0.367 |
| t155 (strategist) | 25.80 | 0.342 |
| final-imitation (strategist) | 25.75 | 0.252 |
| turtle | 25.53 | 0.290 |
| rusher | 22.93 | 0.208 |
| classic | 11.35 | 0.022 |
| baseline | 10.77 | 0.041 |
| aggressor | -3.80 | 0.000 |
| autopilot | -4.78 | 0.000 |

The fleet agent still leads, by 9.8 points over the next agent. Greedy
strategists no longer beat the tuned executor: in the old, sampled league
they rated 5-8 points above it, but greedy they rate level with it or
below. The old league was also played under earlier rules and the
pre-t202 fleet agent, so the two tables are not a paired comparison.
Closed on 2026-10-18.

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

**Progress (2026-10-17).** The fallback is logged: `CampaignAi::stale_seats`
names each seat whose saved memory is from another year than the campaign,
and `nc-sysop maint` warns "AI seat N's saved memory is from year Y, not
Z". `tests/ai_seats.rs` leaves memory a year behind, as a stop between the
two saves would, and sees the warning, then a quiet run after it. Saving
both in one transaction would mean threading the memory through the
snapshot save, or keying the table by year, a schema change on the live
Beta 1 store; the warning was the smaller change.

Intel after attacks: the two paths disagree after a bombardment. An
invasion or blitz grants the attacker planet intel (`AssaultSuccess` or
`AssaultFailure`), which the store keeps and hazard routing reads. A
bombardment grants none, so the attacker's stored intel and its hazard
routing keep the batteries from before it, while `SeatMemory` takes the
batteries and armies left from the bombardment report (t178). An AI seat
whose memory falls back loses that. Making bombardments grant intel would
change every player's intel display, so it waits for the owner.

**Done (2026-10-20).** ECMAINT writes the bombarded world's post-attack
owner, armies, batteries and production into the attacker's planet
database (the `ecmaint-bombard-heavy` fixture: the attacker's blank row
shows 0 batteries and 60 armies after, from 15 and 142). The owner chose to
match it in the engine only, leaving the classic codecs alone: a new
`PlanetIntelSource::Bombardment` grants the attacker the world's name,
owner, Potential, armies and batteries after the attack, as a successful
assault does. The classic `DATABASE.DAT` export writes it as ECMAINT
does: name, owner, Potential, current production, armies and batteries,
the 0x1e word 0, and the year words (`nc-cli` `maint` test
`maint_rust_bombardment_gives_the_attacker_the_worlds_post_attack_intel`,
which replaces the test that the row stayed blank). `maint_combat`'s bombardment test now checks the grant and that
the attacker's snapshot holds the world's post-attack figures, the ones
`SeatMemory` takes from the report. The player manual says so under the
hostile missions.

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
