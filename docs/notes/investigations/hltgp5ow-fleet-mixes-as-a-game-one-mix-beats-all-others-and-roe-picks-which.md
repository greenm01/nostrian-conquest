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
- t254: the attack-and-defence game (see below; batteries and armies counter
  different attacks).
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
  or BB, the rest in loaded transports (7 points an army), and one of three
  plans:
  - *Siege:* bombard until the batteries are gone and the armies aboard
    outnumber the garrison, then invade.
  - *Bombard, then blitz:* blitz once the batteries are gone.
  - *Blitz:* land at once, past the batteries. A blitz may bring no
    warships; the other plans take two shares or more.
- **Attack budgets:** 400, 600 and 800 points, which give 97, 244 and 487
  attacks.
- `tools/combat-model/assault_game.py` solves the zero-sum game on the
  chance the world falls. It also solves a second table on the cost
  exchange. It bootstraps each pairing's trials 200 times.
- Results are in `tools/combat-model/nc_assault_game_t254.json`. Raw grids
  are in the mini's `~/.local/state/nostrian-conquest/balance/t254/`. A
  first run with sieges alone is in its `siege-only/`.
- **Checks:** the solver's rectangular check (unbalanced matching pennies)
  passes. The harness's fixed cases reproduce their earlier numbers exactly.

**Batteries stop sieges, and armies stop blitzes.** Below is each
defence's capture chance against the best siege and the best blitz (either
blitz plan):

| Defence (400 points) | 600: siege | 600: blitz | 800: siege | 800: blitz |
| --- | ---: | ---: | ---: | ---: |
| 20 batteries | 0.01 | 1.00 | 0.31 | 1.00 |
| 15 batteries, 50 armies | 0.35 | 0.25 | 0.97 | 0.58 |
| 15 batteries, a starbase, 25 armies | 0.28 | 0.34 | 0.97 | 0.96 |
| 15 batteries, guard 6 CA, 5 armies | 0.17 | 0.90 | 0.86 | 1.00 |
| 10 batteries, 100 armies | 0.99 | 0.04 | 1.00 | 0.25 |
| 200 armies | 1.00 | 0.00 | 1.00 | 0.00 |

At 400 points, 15 batteries and 50 armies hold against every attack. Twenty
batteries with no armies fall to a 400-point blitz of 57 armies, with no
warships, 41% of the time.

1. **The two ground defences counter different attacks.** Batteries make a
   siege slow and costly, but a blitz lands past them. Armies are what a
   blitz must beat, but a siege bombards them away. Neither alone holds
   against twice its points.
2. **The best defence mixes them.** When the attacker sees the defence (a
   scout counts batteries and armies), the best one is about three-quarters
   batteries and a quarter armies: 15 batteries with 50 armies, or 15
   batteries, a starbase and 25 armies.
   - At 1.5 times its points, either falls 34–35% of the time.
   - At twice its points, either falls 97% of the time.
3. **A defender who keeps the mix secret does better.** The equilibrium
   switches between all-batteries and all-armies at 800 points, and the
   world falls 59% of the time. The secret is hard to keep in play, so
   the seen-defence numbers above are the practical ones.
4. **Attacks mix too.** Both plans stay in the equilibrium:
   - heavy sieges with few troops (cruisers and battleships, about a tenth
     of the points in troops);
   - blitzes that are mostly troops (60–114 armies with a light escort).

   Bombard-then-blitz never enters it, in either table.
5. **The attacker still pays.** In every equilibrium of the cost-exchange
   table, the attacker loses more points than the defender: 0.46, 0.34 and
   0.25 of the defence budget at 400, 600 and 800 points. These attacks are
   all blitzes.
6. **Guard fleets and starbases add little to holding a world.** A
   starbase in place of 25 armies plays about the same. A guard fleet in
   place of armies leaves the world open to a blitz.

**Limits.**
- One world, with no rebuilding during the siege, no relief fleet and no
  reinforcements.
- A siege invades only once its armies outnumber the garrison.
- Guard fleets are one hull each, at ROE 10.
- Starbases and guards have other uses (economy, mobility, keeping
  bombardment off production) that this game doesn't price.

**Finding.** World defence has a counter cycle. Batteries counter sieges,
armies counter blitzes, and the best seen defence is mostly batteries with
a quarter of the points in armies. The original player guide gives the
same advice: batteries give the most "firepower per dollar", but "Keep a
number of armies on your planet or your opponents will BLITZ and overrun
your armies." Taking such a world costs about twice its points.
Starbases, guard fleets and armies past a blitz-proof garrison earn their
points in other ways. No rule change is proposed.

A first run with sieges alone found batteries alone the best defence. That
was an artifact of leaving out the blitz.

Re-run under t258's battery targeting (batteries share their fire by ship
count), the finding holds: the best defence is unchanged, and sieges are
a little harder (capture value 0.245 → 0.224 at 1.5 times, 0.589 → 0.583
at twice). Siege fleets shift from battleships to cruisers. Figures are in
[kazzj70c](../decisions/kazzj70c-batteries-pick-their-targets-at-random-among-the-warships.md).

## Connections

- [e1w4xpes](e1w4xpes-lanchesters-square-law-doom-stacks-and-the-orbit.md):
  the t243 duels this generalizes.
- [combat-balance-workflow.md](../../dev/combat-balance-workflow.md): where
  this method sits beside exact enumeration and seeded sampling.
- [tdkitij6](../decisions/tdkitij6-heavies-cover-the-destroyers-which-finish-cripples-at-half-price.md):
  t206's execution fire, which is why destroyers belong in every winning
  mix.
