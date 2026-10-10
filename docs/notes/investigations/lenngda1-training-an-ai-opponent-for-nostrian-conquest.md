---
id: lenngda1
date: 2026-10-09
kind: investigation
status: investigating
tags: [investigation, ai, pinned]
---
# Training an AI opponent for Nostrian Conquest

## Question

How could we build an AI opponent that is strong and fun to play against,
beyond the scripted autopilot in `nc-engine/src/maint/economics/ai.rs`? Which
published game-AI systems fit this game's shape, and what is the cheapest
path to a good opponent?

## Evidence

### What makes the game hard to learn

- **Simultaneous moves:** every empire submits orders for the same year and
  maintenance resolves them together.
- **Hidden information:** each empire sees only its own intel and reports.
- **Many players:** up to 25 empires, with alliances that form and break.
- **A huge action space:** orders for every fleet and its target, builds for
  every planet, tax, and diplomacy, every turn.
- **Long horizon, sparse reward:** the outcome is known decades of game time
  later.

These rule out AlphaZero-style tree search, which handles neither
simultaneous moves nor hidden information well. The engine itself suits
training: it is deterministic, seeded, fast Rust with no rendering.

### Published systems (from a web search on 2026-10-09)

| System | Game | What carries over |
| --- | --- | --- |
| AlphaStar (DeepMind, 2019) | StarCraft II | A transformer over units, an LSTM core, and an autoregressive policy head with a pointer network for huge action spaces. Imitation of human replays, then league self-play with exploiter agents to avoid strategy cycling. Ran 14 days on 16 TPUs per agent; the final agent runs on one desktop GPU. A Tencent reimplementation (TStarBot-X) found a faithful copy fails with limited compute. |
| CivRealm (ICLR 2024) | Freeciv, turn-based 4X | Closest analogue: imperfect information, a changing player count, diplomacy. RL agents did reasonably on mini-games, but both RL and LLM agents struggled to make substantial progress in the full game. |
| DORA and CICERO (Meta) | Diplomacy | Closest in mechanics: simultaneous moves and a combinatorial action space. DORA reached superhuman play from pure self-play, but self-play agents did badly against human-data agents and vice versa. CICERO paired human-data planning with a language model for negotiation. |
| DeepNash (DeepMind, 2022) | Stratego | R-NaD, a model-free self-play method for hidden information that converges to an approximate Nash equilibrium instead of cycling, with no search and no opponent modeling. Reached an all-time top-3 rank on a human platform. |
| Vox Deorum (Dec 2025) | Civilization V | Hybrid: an LLM sets strategy and the game's scripted AI handles tactics. Over 2,327 games it tied the strong scripted AI on win rate, with distinct play styles. |
| CivBench (2026) | Civilization V | Estimates each player's win probability from turn-level state, because final win or loss is too sparse in long, many-player games. |

### Engine throughput (measured 2026-10-10, t025)

Machine: AMD Ryzen 9 7950X3D, 16 cores and 32 threads, 61 GiB RAM, Linux
6.18, rustc 1.99.0-nightly, release profile (thin LTO, one codegen unit).
Other work kept the load average near 4 during the run, so the multi-thread
rows carry some noise. Command, from `rust/`:

```bash
cargo run --release -p nc-fuzz --example maint_throughput
```

The example builds each game with the fuzzer's `CaseSetup` (no rally, war,
armadas, or collapsed empire; the map seed is the game number) and plays it
through `nc_harness::ScenarioPlay` until the turn limit or a winner. In
`autopilot` games every seat is on the engine's autopilot and nothing is
submitted. In `policy` games the seats cycle the fuzzer's aggressor, random,
and swarm policies, which plan through turn KDL and submit every turn. None
of the fuzzer's checks run. Setup is timed per game; planning and
maintenance per turn.

Single thread, 100-turn games (20, 10, 4, 2, and 1 games per row from 2 to 25
players; 2-player policy games end in a win after 78.5 turns on average):

| Game | Seats | Setup ms/game | Plan ms/turn | Maint ms/turn | Maint turns/s | Wall turns/s | Games/h |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 2p 18x18 | autopilot | 3.6 | - | 0.0035 | 285k | 25.4k | 914k |
| 2p 18x18 | policy | 3.4 | 0.12 | 0.0050 | 199k | 5.8k | 265k |
| 4p 18x18 | autopilot | 14.8 | - | 0.0063 | 159k | 6.5k | 233k |
| 4p 18x18 | policy | 14.6 | 0.32 | 0.012 | 85k | 2.1k | 76k |
| 9p 27x27 | autopilot | 81 | - | 0.013 | 79k | 1.2k | 44k |
| 9p 27x27 | policy | 85 | 0.64 | 0.023 | 44k | 664 | 24k |
| 16p 36x36 | autopilot | 349 | - | 0.024 | 41k | 284 | 10k |
| 16p 36x36 | policy | 386 | 1.30 | 0.055 | 18k | 192 | 6.9k |
| 25p 45x45 | autopilot | 1,272 | - | 0.048 | 21k | 78 | 2.8k |
| 25p 45x45 | policy | 1,190 | 1.80 | 0.078 | 13k | 73 | 2.6k |

Per-turn costs in 50-turn games match; with setup spread over fewer turns,
50-turn policy games run at 365k games/h for 2 players and 113k for 4.

Independent 4-player 18x18 100-turn games on parallel threads, four games
per thread:

| Threads | Autopilot turns/s | Autopilot games/h | Policy turns/s | Policy games/h |
| ---: | ---: | ---: | ---: | ---: |
| 1 | 6.5k | 234k | 2.3k | 82k |
| 2 | 13.7k | 493k | 4.4k | 159k |
| 4 | 26.6k | 957k | 8.8k | 318k |
| 8 | 49.8k | 1.79M | 16.3k | 588k |
| 16 | 79.0k | 2.84M | 30.2k | 1.09M |
| 32 | 105k | 3.79M | 39.8k | 1.43M |

The fuzz loop itself (`run_case` on drawn 2-4 player setups, up to 50 turns,
with its twin maintenance run, invariants, and SQLite round trip) ran 1,544
turns/s, 0.65 ms a turn including setup over 25.5-turn games, about the same
as 50-turn policy games without the checks.

What the numbers show:

- **Maintenance is not the cost.** A maintenance turn takes 4 to 80
  microseconds from 2 to 25 empires, 85k to 285k turns/s on one thread for
  small maps.
- **Setup and turn planning are the cost.** `build_scenario` takes 3.5 ms for
  2 players, 15 ms for 4, and 1.2 s for 25, growing much faster than the
  player count; it outweighs all 100 maintenance turns of a game. The fuzz
  policies spend 0.07 ms per empire per turn, because the turn builder
  renders and re-parses turn KDL and re-applies the whole turn for every
  action. A training loop should build start states once and clone them, and
  a trained policy should build `TurnSubmission`s directly with the engine
  validators as its action mask.
- **Autopilot-only games are idle.** In 100-year all-autopilot games no
  planet changed owner and no fleet record was added: the autopilot sets tax,
  queues builds, and recalls idle fleets, but never expands or attacks. It is
  not a sparring partner for the evolution step below; tune against the
  aggressor policy or a scripted expansion bot instead.
- **Threads scale nearly linearly to 8** and reach 76 to 87% of linear at
  16, the physical core count, with other load on the machine; SMT adds
  about a third more.

For the training plan: policy-driven 4-player 100-turn games run about 1.1M
an hour on 16 threads, about 26M a day. An evolution generation of 100
candidates at 200 games each (20k games) takes about a minute, so hundreds
of generations a day fit this machine. For a learned policy the engine's
share of a turn is negligible next to any neural forward pass: at an assumed
1 ms of inference per empire turn, a 4-player 100-turn game costs 0.4 s,
about 9k games an hour per thread, so inference and the Python crossing set
the budget, not maintenance. Even 25-player games cost about 8 ms of
maintenance per 100 turns once setup is reused.

## Finding and resolution

Not started; this is a design direction, not a result.

### Suggested order of work

1. **Evolution-tuned scripted bot.** Expose a dozen strategy weights on the
   autopilot or the fuzzer's policies (expansion versus defense, tax, attack
   threshold) and tune them by evolutionary search against the fuzzer's
   aggressor policy, since the current autopilot never expands or attacks
   (see the throughput measurement). Cheap, explainable, and possibly a good enough opponent alone.
2. **LLM plus scripted executor (the Vox Deorum shape).** An LLM reads the
   empire's reports and sets strategy; a scripted executor turns it into
   orders through `turn.kdl`. The LLM side already exists in
   [llm-player-guide.md](../../dev/llm-player-guide.md). It is the only option
   that can negotiate through in-game messages.
3. **A learned policy, if still wanted.** AlphaStar's shape scaled down:
   entities in, actions chosen one piece at a time with pointer-style target
   picks and engine validators as the action mask. Start from imitation of the
   scripted bots, then PPO self-play on 2-player 18x18 games, then DeepNash
   R-NaD or league self-play while growing to more players and larger maps.

### Design notes for a learned policy

- **Environment:** a thin gym-style Rust API (start from seed and player
  count, observe one empire, step with all empires' orders), exposed to Python
  through PyO3. See the measured engine throughput above.
- **Observation:** only what a human player sees, per the rule in
  [llm-player-guide.md](../../dev/llm-player-guide.md): the empire's intel
  database, its own planets and fleets, recent reports, and intel age.
- **Action interface:** the fuzzer's choice source ("fleet 2 of 7, mission,
  target") is nearly the same interface; a neural policy can be one more
  choice source over the same policy code.
- **Deployment:** inference in Rust through ONNX (`ort` or `tract`) or
  `candle`, with seeded sampling for reproducible games, difficulty through
  snapshot strength or temperature, and personality through reward weights.

### Risks

- **Exploits:** an RL agent will find engine bugs fast; the free-build bug
  from the October 2026 sweep is the kind it would exploit. Keep the fuzzer's
  invariants running during training and treat exploits as bug reports.
- **Fun over strength:** DORA's result means pure self-play may produce an
  opponent that plays oddly against people. Log human `turn.kdl` submissions
  from BBS games now, under the visible-information rule, as future training
  data.
- **Compute:** the engine runs about a million small games an hour on this
  machine; a learned policy's inference cost, not the engine, sets the
  budget, and 25-player training likely needs a GPU and days of self-play.

## Validation and remaining work

Candidate task t025 in [todo.md](../../../todo.md) tracks the first step.
Its throughput measurement is done (see Evidence); the evolution-tuned bot
is not built. A CivBench-style win-probability model would serve as a training
reward, an evaluation metric, and a strength meter for AI seats.

## Connections

The fuzzer's policies and choice source are in the
[fuzzer plan](../plans/lski78er-engine-fuzzer-with-shared-policies-and-invariants.md).
LLM play rules are in [llm-player-guide.md](../../dev/llm-player-guide.md) and
[llm-test-harness.md](../../dev/llm-test-harness.md).

Sources:

- [AlphaStar (DeepMind blog)](https://deepmind.google/blog/alphastar-mastering-real-time-strategy-game-starcraft-ii)
- [TStarBot-X (arXiv 2011.13729)](https://arxiv.org/pdf/2011.13729v1)
- [AlphaStar method summary](https://cs.paperswithcode.com/method/alphastar)
- [CivRealm on GitHub](https://github.com/bigai-ai/civrealm)
- [CivRealm paper (ICLR 2024)](https://www.alphaxiv.org/abs/2401.10568.md)
- [No-Press Diplomacy from Scratch (DORA)](https://ar5iv.arxiv.org/html/2110.02924)
- [Learning to Play No-Press Diplomacy](https://ar5iv.labs.arxiv.org/html/2006.04635)
- [Diplomacy and CICERO slides](https://www.cs.ubc.ca/~kevinlb/teaching/cs532l/student%20presentations/Ward%20-%20Diplomacy.pdf)
- [Mastering the Game of Stratego (DeepNash)](https://arxiv.org/pdf/2206.15378)
- [Vox Deorum (arXiv 2512.18564)](https://arxiv.org/pdf/2512.18564)
- [CivBench, Civilization V (arXiv 2604.07733)](https://arxiv.org/html/2604.07733)
