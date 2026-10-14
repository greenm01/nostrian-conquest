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

### t156

Build the league and evaluation.

- SQLite tables for agents, checkpoints, matches and results.
- TrueSkill ratings with seat rotation.
- Replays stored as a seed plus the order stream. The engine is deterministic,
  so this rebuilds any state.

**Exit:** the rating table can be rebuilt from the stored seeds and orders.

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

### t158

Measure whether the game players keep the GPU busy during t157's self-play.

Add bean-void as a second actor only if both hold:

- the GPU waits on games more than half the time;
- a training run takes longer than about a day.

bean-void would run the policy on its CPU through ONNX, write shards, and
rsync them to the mini, with a limit on how stale its policy may be.

**Exit:** utilisation is measured, and the decision is recorded here either
way.

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
