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
