---
id: g6uo9u0x
date: 2026-10-23
kind: investigation
status: investigating
tags: [investigation, ai, t207]
---
# Training techniques for the learned agent's builds, drift and rare orders

## Question

t207 step 2 found three failures in the learned agent (see the
[t207 plan](../plans/qkgqypzw-learned-ai-opponent-harness-hierarchical-agent-and-fleet-level-bootstrap-on-the-mac-mini.md#t207)):

1. The build head never overrides the executor in greedy play. Sampled
   play deviates at 6-9% of worlds, and the first slot's probability of
   `BUILD_EXECUTOR` moved only from 0.94 to 0.91 in 200 iterations of 64
   games.
2. With the KL at 0, the tax and join heads drift, and an 80-iteration
   exploiter wins 0.43 where 0.25 is even. With a 0.02 floor, the agent
   holds strength but learns little.
3. Rare but useful orders (Blitz, Salvage, starbases, varied tax) are never
   the greedy choice.

What does published work on RL for strategy games offer for each, within
the owner's rule that the agent finds what wins (results-only reward)?

## Evidence

A literature search (2026-10-23) covered four areas, each with its own
notes, and one report combined them. They are kept outside the repo,
under `~/.local/state/nostrian-conquest/research/` on the Mac mini:
residual policies and action spaces; credit assignment and exploration;
league training and drift; and strategy-game projects. The search ran
before checking the code. On checking, three of its open questions are
already settled here:

- The reward is terminal only: win share, worth 1.0 for a sweep and
  `--limit-credit` (0.5) for a production lead at the year limit
  (`rust/nc-ai/src/batch/mod.rs`). There is no per-turn production term,
  so the report's worry that production share tilts the objective does
  not apply.
- Every head is masked to legal choices, and the fleet target head is
  conditioned on the chosen mission (`target_query` over
  `mission_embed`).
- Per-head KL exists in part: `--free-heads` takes a head out of the KL.
  There is no per-head coefficient or smoothed anchor yet.

No source applies any of these techniques to a four-player turn-based 4X
game. The mappings below are inferences, and every equilibrium guarantee
cited holds only for two-player zero-sum games.

## Finding and resolution

Two causes, not a lack of exploration, explain most of all three
failures.

**The anchor charges for every deviation.** The imitation policy puts
almost all its weight on the executor's choice. With the KL-regularized
optimum π(a) ∝ π₀(a)·exp(Q(a)/λ) (piKL, [Jacob et al., ICML
2022](https://arxiv.org/abs/2112.07544)), a deviation becomes the mode
only when its value advantage beats λ·log(π₀(default)/π₀(a)). That is
about 7λ when π₀(a) is near 1e-3; applying the formula to our numbers is
the report's own estimate, not a measured figure. OpenAI Five warned of
the same trap when handing scripted logic to the model ([Berner et al.
2019](https://arxiv.org/abs/1912.06680)).

**Credit is diluted.** One world's build in one year gets the whole
turn's advantage, and noise from every other head, world and player
swamps it.

What the literature offers, by failure:

- **Builds (1):**
  - Per-head KL weights, with a smoothed anchor on the build head
    ([Galashov et al. 2019](https://arxiv.org/abs/1905.01240);
    [Kickstarting](https://arxiv.org/abs/1803.03835)).
  - Per-head PPO clipping and advantage normalization.
  - Counterfactual branch rollouts from saved states, using the engine as
    the simulator: TRPO's "vine" estimator ([Schulman et al.
    2015](https://arxiv.org/abs/1502.05477)) and COMA's counterfactual
    baseline ([Foerster et al. 2018](https://arxiv.org/abs/1705.08926)).
    Learned action-dependent baselines often add bias rather than cut
    variance ([Tucker et al.
    2018](https://proceedings.mlr.press/v80/tucker18a/tucker18a.pdf)), so
    check any against branch rollouts.
  - A gated head ("override?", then the kind), or a doctrine-level build
    mix held for several turns over the executor's builds.
    - TStarBot1 learned 165 scripted macros from win/loss alone
      ([arXiv 1809.07193](https://arxiv.org/pdf/1809.07193)).
    - CherryPi's choice among 25 build orders raised held-out win rates by
      14-25 points ([Gehring et al.
      2018](https://ar5iv.labs.arxiv.org/html/1811.08568)).
    - CivRealm's flat PPO over Freeciv production learned a myopic "build
      units for score" policy ([Qi et al., ICLR
      2024](https://arxiv.org/pdf/2401.10568)).
  - A 2026 µRTS preprint found learned commands that met their training
    targets but changed nothing in paired tests, because the default
    already did what they asked ([Leenders et al.
    2026](https://arxiv.org/html/2610.11663)). That matches our build
    head.
  - Keeping scripted builds is a legitimate outcome: OpenAI Five scripted
    item purchases.
- **Drift (2):** keep an anchor, but move it or vary its strength.
  - In 7-player Diplomacy, the unanchored agent ranked 32nd of 62 human
    players. The anchored agents ranked 1st and 3rd. A KL weight sampled
    each game from {1e-4, 1e-3, 1e-2, 1e-1} beat any fixed weight
    ([Bakhtin et al., ICLR 2023](https://arxiv.org/html/2210.05492v1)).
  - TStarBot-X replaced its weak supervised anchor with a KL to its own
    earlier checkpoint (DAPO) for a significant Elo gain
    ([arXiv 2011.13729](https://arxiv.org/html/2011.13729v2)).
  - Magnetic Mirror Descent anchors to a trailing average of the policy
    and lowered exploitability where PPO did not; in deep RL it is a
    one-line change ([Sokota et al., ICLR
    2023](https://arxiv.org/pdf/2206.05825)).
  - A best response to one target is not population strength: Diplomacy's
    best-response bot scored only 18% in the wider population. Report the
    exploiter score next to a fixed-panel score.
- **Rare orders (3):**
  - Self-imitation or UPGO, which copies only steps that beat the value
    estimate; AlphaStar and the Lux AI 2021 winner used it ([Oh et al.
    2018](https://arxiv.org/abs/1806.05635)).
  - Order-level committed tries (εz-greedy, [Dabney et al.
    2021](https://arxiv.org/abs/2006.01782)).
  - Per-head entropy targets.
  - At deployment, sampling at a low temperature or a margin rule instead
    of each head's argmax. Practice is mixed: AlphaStar reportedly sampled
    at 0.8, and the Lux winner took the argmax.
- **Shaping:** the only optimality-preserving shaping is potential-based,
  F = γΦ(s') − Φ(s) ([Ng et al.
  1999](https://people.eecs.berkeley.edu/~pabbeel/cs287-fa09/readings/NgHaradaRussell-shaping-ICML1999.pdf)).
  It must have Φ = 0 at game end ([Grześ
  2017](https://www.ifaamas.org/Proceedings/aamas2017/pdfs/p565.pdf)). An
  economic-lead potential fits the owner's rule but needs approval.
  Rewards for following a chosen strategy (AlphaStar's z pseudo-rewards,
  adherence rewards) break the rule and are rejected.
- **Budget:** our ~12,800 games is small next to every comparable
  success.
  - µRTS: about 60 hours on one GPU ([Huang et al.
    2021](https://arxiv.org/pdf/2105.13807)).
  - Lux AI 2021 winner: weeks on one PC
    ([README](https://github.com/IsaiahPressman/Kaggle_Lux_AI_2021)).
  - Structural changes should come before more iterations.

## Validation and remaining work

Ranked experiments, cheapest first. "Panel" means 1,000 games against
the promoted agents (0.25 even, standard error about 0.014). Steps 0-5
fit the Mac mini; 6-8 suit crunch once the owner frees it.

| # | Experiment | Cost | Success |
|---|---|---|---|
| 0 | Paired test: forced learned against executor builds on the same seeds; advantage with and without a deviation; critic explained variance by year | Hours | Shows whether an override changes outcomes at all. If within 2 SE of zero, drop 6-7 for builds |
| 1 | Deployment sweep: temperature {0, 0.3, 0.5, 0.8} or a "non-default if p > 0.25" rule on the build, tax and mission heads | Hours, no training | Within 2 SE of greedy with rare orders used; adopt if 2 SE better |
| 2 | Per-head KL: 0.02 floor on tax and join; build head with an ε = 0.1 smoothed anchor; per-head clip and normalization | Hours + one run | Exploiter ≤ 0.25; first-slot p(executor) < 0.80; panel ≥ 0.25 |
| 3 | Moving anchor (DAPO snapshot or MMD trailing average) | About a day | Beats the 0.02-floor agent > 0.27; exploiter ≤ 0.25 |
| 4 | Per-head entropy targets for rare orders | 1-2 days | Rare-order sampling ×3; some become greedy in legal states |
| 5 | Self-imitation plus a KL weight sampled per game | About a day | Panel +2 SE over step 3 |
| 6 | Branch-rollout advantages for builds | 2-4 days | Nonzero deviation values that move greedy overrides |
| 7 | Gated, then doctrine-level, build head | 2 days to a week | Paired delta > 2 SE and a panel gain |
| 8 | League: exploiters seeded from executor styles, quality-score PFSP; ask approval for an economic-lead potential | Days, GPU | Exploiter ≤ 0.25 across resets while the panel rises |

Open work (owner, 2026-10-23), in `todo.md`:

- t212: steps 0 and 1.
- t213: step 2.
- t214: step 3.
- t215: step 5.
- t216: steps 6 and 7, only if t212 shows overrides matter.
- t217: the shaping decision.
- t184: step 8's league changes.

Step 4's masking and mission-conditioned target already exist; its
entropy targets go with t213. The task details are in the plan's t212 to
t217 sections.

The exploit check on the KL-floor run's snapshot 200 (`exploit-t207c` on
bean-void) was still running when this note was written.

## Connections

- [t207 plan section](../plans/qkgqypzw-learned-ai-opponent-harness-hierarchical-agent-and-fleet-level-bootstrap-on-the-mac-mini.md#t207):
  the runs, gates and results this research answers.
