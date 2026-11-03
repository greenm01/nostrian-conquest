---
id: hltgp5ow
date: 2026-11-03
kind: investigation
status: investigating
tags: [investigation, combat, balance, game-theory, hulls]
---
# Fleet mixes as a game: one mix beats all others, and ROE picks which

## Question

Is fleet combat balanced across hulls? t243
([e1w4xpes](e1w4xpes-lanchesters-square-law-doom-stacks-and-the-orbit.md#simulation-battleships-starbases-invasion-and-recovery))
found battleships winning every equal-cost duel it tried. A few duels can't
show whether some mix beats everything, so t253 treats every equal-cost
DD/CA/BB mix as a strategy and solves the game between them. Ship prices
stay classic, so a fix, if one is needed, must be a combat rule.

## Evidence

**Method.**
- `rust/nc-fuzz/examples/combat_odds.rs --grid BUDGET [--step POINTS] --roe R`
  plays every ordered pair of mixes through the real maintenance turn. The
  scenario: one fleet each, at rest in an empty sector, at war, on Hold.
- Budgets: 405 points in 45-point shares, and 1,215 in 135-point shares.
  Either way that's 55 mixes, 3,025 ordered pairs.
- 1,000 trials per ordered pair (seeds 1–1,000), so 2,000 per pairing
  across both seatings. Both sides at ROE 10, then both at ROE 6.
- `tools/combat-model/metagame.py` builds the payoff
  `M[i][j] = P(i wins) − P(j wins)`, a trial without a battle counting as a
  draw. It then:
  - solves the symmetric zero-sum game by linear program;
  - checks for a mix that beats every other;
  - bootstraps each pair's trials 200 times;
  - runs α-Rank (OpenSpiel) as a cross-check.

  A second table scores the cost exchange instead of wins.
- Results: `tools/combat-model/nc_metagame_t253.json`. Raw grids and logs:
  `~/.local/state/nostrian-conquest/balance/t253/` on the Mac mini.

**Every game has a single mix that beats all the others**, with α-Rank
putting 65% of its mass on the same mix:

| Budget, ROE | Winning mix | Points DD / CA / BB | In bootstrap support |
| --- | --- | --- | --- |
| 405, ROE 10 | 18 DD + 3 CA + 6 BB | 22 / 11 / 67% | 93% |
| 1,215, ROE 10 | 54 DD + 9 CA + 18 BB | 22 / 11 / 67% | 97% |
| 405, ROE 6 | 27 DD + 12 CA + 2 BB | 33 / 44 / 22% | 100% |
| 1,215, ROE 6 | 81 DD + 36 CA + 6 BB | 33 / 44 / 22% | 100% |

- **The answer doesn't depend on scale.** The same point shares win at both
  budgets.
- **ROE picks the winner.** In a fight to the end (ROE 10), a battleship
  line with a destroyer screen wins. At the default ROE 6, where fleets
  break off and are pursued, a cruiser-led mix with a destroyer screen and
  a few battleships wins.
- **No hull is dead weight.** Every winning mix uses all three hulls, and
  destroyers always take a fifth to a third of the points.
- **No pure-hull fleet is optimal.**
- **The cost-exchange table** gives the same mix at ROE 10. At ROE 6 it
  mixes three neighbouring mixes, with more battleships (39–44% of points).
- **The AI's mix loses clearly.** 45 DD + 12 CA, nearest the AI's 3 DD : 1
  CA, loses to its best response by 0.66–0.69 win margin at ROE 10 and
  0.28 at ROE 6.

**Agreement with t243.** At 405 points and ROE 10:

| Pairing | This run | t243 |
| --- | --- | --- |
| Cruisers against destroyers | 86/13% | 86/12% |
| Battleships against cruisers | 70/30% | 62/30% |
| Battleships against destroyers | 94/6% | 88/7% |

The direction holds, with the battleship margins higher here. t243's
harness met moving fleets with 300 seeds and isn't in the tree, so the gap
isn't pinned down.

**Seat check.** The left seat won mirror matches by 3–8 points in three of
the four grids. That's the shared seed range, not the engine. One mirror
run over 4,000 trials in three seed ranges split 50.9/48.4, 50.4/49.0 and
49.2/50.1. The yearly unit order is drawn at random. Pooling both seatings
cancels the effect in every payoff above.

## Finding and resolution

The fleet-mix game is transitive: at a given ROE, one mix is the right
answer, and there is no rock-paper-scissors among mixes. That's expected
when every hull fights by the same rules with costs that scale linearly.
Lanchester's square law then rewards the best blend of defence per point
(battleships) and cheap execution fire (destroyers).

Whether this needs a rule change is the owner's call:
- Hulls are balanced in the sense that each has a role and none dominates
  alone.
- But composition is solved, and the stance (ROE) changes the answer more
  than any counter-build does.

## Validation and remaining work

Checks run:
- `metagame.py` checks itself on rock-paper-scissors (⅓ each).
- Mirror matches split evenly over fresh seed ranges.
- Cruisers against destroyers reproduces t243.

Not covered here:
- Mixed ROE pairings, such as ROE 10 against ROE 6.
- Starbases, planets, movement and speed.
- Fleets that meet while moving.

Remaining work:
- t254: the attack-and-defence game (fleet mixes against batteries, armies,
  starbases and guards).
- t255: refresh the Python fleet model and search rule settings, only if the
  owner wants composition to have counters.

## Connections

- [e1w4xpes](e1w4xpes-lanchesters-square-law-doom-stacks-and-the-orbit.md):
  the t243 duels this generalizes.
- [combat-balance-workflow.md](../../dev/combat-balance-workflow.md): where
  this method sits beside exact enumeration and seeded sampling.
- [tdkitij6](../decisions/tdkitij6-heavies-cover-the-destroyers-which-finish-cripples-at-half-price.md):
  t206's execution fire, which is why destroyers belong in every winning
  mix.
