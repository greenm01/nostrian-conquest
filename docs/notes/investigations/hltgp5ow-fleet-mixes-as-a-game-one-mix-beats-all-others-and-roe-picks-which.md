---
id: hltgp5ow
date: 2026-11-03
kind: investigation
status: resolved
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
- t254: the attack-and-defence game (see below; batteries dominate).
- t255: search rule settings for counters (see below; adopted).

## Search for counters (t255)

The owner asked on 2026-11-03 for composition to have counters. Hull-specific
bonuses are fine with them ("asymetry is not bad imo").

**Method.**
- Branch `t255-combat-search` (an experiment, never merged) adds two things:
  - **Per-hull fire.** Each hull class spends its own share of a volley,
    with its own first target and a damage bonus against it.
  - **A test-only `--tuning` option** for `combat_odds`.
- With no bonus, per-hull fire plays the same game as today: the same
  dominant mix, 60% battleships. With nothing set, the engine reproduces
  main battle for battle.
- `tools/combat-model/counter_search.py` on the branch ran 300 Optuna
  candidates on a 21-mix grid at 450 points, at ROE 10 and ROE 6. Each
  candidate was scored on three things:
  - whether every mix has a counter past two standard errors;
  - the smallest hull's share of the equilibrium;
  - a penalty for moving hull stats.
- Today's rules scored +0.014; the best candidates scored +0.29.
- The top five candidates were all the same family: destroyers aim at
  cruisers first, cruisers aim at battleships first, and battleships keep
  the line order.

**Verification** on the full 55-mix grids, at 1,000 trials per ordered
pair. "Force" means the first side at 125% strength in each of 441
pairings, at both ROEs.

| Candidate | Mix with no counter | Hull mass DD/CA/BB | More force worse |
| --- | --- | --- | --- |
| Today | one in every game | ROE 10: 22/11/67 | 0 |
| C: bonuses only, today's stats | one at ROE 10 (cruisers 0%) | ROE 10: 33/0/67 | 0 of 882 |
| R2: bonuses and three stat changes | none in all four games | 41–55 / 22–31 / 19–33 | 0 of 882 |

R, which also raised the engagement exponent to 4.3, played the same as R2,
so R2 keeps classic's engagement count.

**R2's rules:**
1. Each hull class fires its own share of the side's volley (in proportion
   to its AS).
2. Destroyers aim at cruisers first, and their hits count three times
   against cruisers.
3. Cruisers aim at battleships first, and their hits count twice against
   battleships.
4. Battleships keep the line order (BB, CA, DD) with no bonus.
5. Stat changes: BB AS 90 → 100, BB DS 110 → 105, CA DS 30 → 37. Prices
   are unchanged.

**Counters under R2** (405 points, ROE 10, win margin):
- Battleships beat destroyers by +0.85, destroyers beat cruisers by +0.41,
  and cruisers beat battleships by +0.92.
- Among the equilibrium's mixes, 27 DD + 6 BB beats 54 DD + 9 CA by +0.49,
  and 54 DD + 9 CA beats 45 DD + 12 CA by +0.23.
- Today, by contrast, 18 DD + 3 CA + 6 BB beats every mix.

Results: `tools/combat-model/nc_metagame_t255.json`. Raw runs:
`~/.local/state/nostrian-conquest/balance/t255/` on the Mac mini.

**Adopted.** The owner adopted R2 on 2026-11-03, with the ground battery
raised to AS 100 to match the battleship
([zq5xtv8l](../decisions/zq5xtv8l-each-hull-fires-on-its-own-destroyers-hunt-cruisers-cruisers-hunt-battleships.md)).
t256 re-checks the AI.

**R2 also affects:**
- Bombardment, which uses warship AS.
- Battery fire against fleets, which uses DS.
- The AI's build weights; land-grab builds no battleships.
- The trained AI, which learned under today's rules and would need
  retraining.
- The player manual's combat chapter.

## Attack and defence (t254)

How should a world be defended for its points, and how attacked? Run under
R2 at `7933bfa8`.

**Method.**
- `rust/nc-fuzz/examples/assault_odds.rs --grid 400 --attack A` puts every
  attack against every defence of empire 2's homeworld, through the real
  maintenance turn. 400 trials a pairing (seeds 1–400), up to 8 years each.
- **A defence spends 400 points:** no starbase or one (50), batteries in
  steps of 5 (100 points), a guard fleet on Guard/Blockade a World in
  90-point shares of one hull, and the rest in armies (2 each). That's 57
  defences, from 200 armies alone to 20 batteries alone.
- **An attack spends A points:** a warship mix in 90-point shares of DD, CA
  or BB, two shares or more, and the rest in loaded transports (7 points an
  army). It besieges: it bombards until the batteries are gone and its
  armies outnumber the garrison, then invades.
- **Attack budgets:** 400, 600 and 800 points, which give 31, 80 and 161
  attacks.
- `tools/combat-model/assault_game.py` solves the zero-sum game on the
  chance the world falls. It also solves a second table on the cost
  exchange. It bootstraps each pairing's trials 200 times.
- Results are in `tools/combat-model/nc_assault_game_t254.json`. Raw grids
  are in the mini's `~/.local/state/nostrian-conquest/balance/t254/`.
- **Checks:** the solver's rectangular check (unbalanced matching pennies)
  passes. The harness's fixed cases reproduce their earlier numbers exactly.

**Batteries alone are the best defence.** Below, each defence's capture
chance is taken against the attack that does best against it:

| Defence (400 points) | Attack 400 | Attack 600 | Attack 800 |
| --- | ---: | ---: | ---: |
| 20 batteries | 0.00 | 0.01 | 0.31 |
| 15 batteries, guard 6 CA, 5 armies | 0.00 | 0.17 | 0.86 |
| 15 batteries, guard 2 BB, 5 armies | 0.00 | 0.24 | 0.88 |
| 15 batteries, a starbase, 25 armies | 0.00 | 0.28 | 0.97 |
| 15 batteries, 50 armies | 0.00 | 0.35 | 0.97 |
| 200 armies | 0.04 | 1.00 | 1.00 |
| Any defence with 5 batteries or fewer | 0.04–0.98 | 0.98–1.00 | 1.00 |

1. **The defence is solved.** At attacks of 600 and 800 points, 20
   batteries do as well as every other defence against every attack
   (within 3 points). They are the whole equilibrium in all 200 bootstrap
   runs. At 400 points several battery-heavy defences hold completely, so
   the game's value is 0.
2. **Points moved off batteries are wasted.** A starbase, a guard fleet or
   armies all do worse, point for point. A battery fires like a battleship
   for less than half the price (AS 100 for 20 points against 45). Armies
   are weak once bombardment strikes them at full strength (t252): 200 of
   them hold against 400 points but fall to every 600-point attack.
3. **Attacking takes about twice the points.** Against the best defence,
   twice its points take the world 31% of the time, and 1.5 times take it
   1%. The attacker loses more points
   than the defender in every equilibrium (exchange value −0.99, −1.41 and
   −1.21 of the defence budget). This matches the manual's siege guide,
   which says the cheapest attack is four to five times the batteries'
   cost.
4. **The best attacks are heavy.** Against batteries, the equilibrium
   attacks at 600 and 800 points are cruisers and battleships with no
   destroyers: 30–34% of the
   points in cruisers and 56–60% in battleships. The rest is troops, kept
   to a tenth. Destroyers enter only the exchange table at 600 points.

**Limits.**
- One world, with no rebuilding during the siege, no relief fleet and no
  reinforcements.
- The attacker never blitzes, and lands only once it outnumbers the
  garrison. A blitz might punish a world with no armies.
- Guard fleets are one hull each, at ROE 10.
- Starbases and guards have other uses (economy, mobility) that this game
  doesn't price.

**Finding.** Point for point, world defence comes down to batteries, and
taking a battery world costs about twice its points. That fits classic's
"huge firepower for cost" and the manual's siege guide. Starbases, guard
fleets and armies earn their points in other ways, not in holding a
besieged world. Whether batteries should share that role is the owner's
call. No rule change is proposed.

## Connections

- [e1w4xpes](e1w4xpes-lanchesters-square-law-doom-stacks-and-the-orbit.md):
  the t243 duels this generalizes.
- [combat-balance-workflow.md](../../dev/combat-balance-workflow.md): where
  this method sits beside exact enumeration and seeded sampling.
- [tdkitij6](../decisions/tdkitij6-heavies-cover-the-destroyers-which-finish-cripples-at-half-price.md):
  t206's execution fire, which is why destroyers belong in every winning
  mix.
