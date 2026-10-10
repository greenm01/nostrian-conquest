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

## Finding and resolution

Not started; this is a design direction, not a result.

### Suggested order of work

1. **Evolution-tuned scripted bot.** Expose a dozen strategy weights on the
   autopilot or the fuzzer's policies (expansion versus defense, tax, attack
   threshold) and tune them by evolutionary search against the current
   autopilot. Cheap, explainable, and possibly a good enough opponent alone.
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
  through PyO3. Measure turns per second first.
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
- **Compute:** small games fit this machine; 25-player training likely needs
  a GPU and days of self-play. Unmeasured.

## Validation and remaining work

Nothing built yet. t025 in [todo.md](../../../todo.md) tracks the first step;
on 2026-10-10 the user approved the throughput measurement only, with the
scripted opponent left for after beta.3 playtest feedback. A useful
first measurement is maintenance throughput in turns per second for small
games. A CivBench-style win-probability model would serve as a training
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
