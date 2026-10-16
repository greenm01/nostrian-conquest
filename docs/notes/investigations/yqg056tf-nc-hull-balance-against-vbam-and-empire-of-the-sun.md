---
id: yqg056tf
date: 2026-10-16
kind: investigation
status: investigating
tags: [investigation, combat, balance, hulls]
---
# NC hull balance against VBAM and Empire of the Sun

## Question

How do NC's destroyer, cruiser, and battleship compare with the hull ladders
of *Victory by Any Means* (VBAM, 3rd edition, v1.0.2) and *Empire of the
Sun* (EotS, GMT 2021 rules)? What does each source use to make a large ship
worth building, and does the learned-AI evidence agree?

The [combat design review](ebrogywq-nc-combat-design-review-against-its-sources.md)
and the [independent balance analysis](rolr9ti0-independent-nc-combat-balance-analysis.md)
cover combat procedure. Neither compares the cost and value curve across
hulls. This note does.

## Evidence

### Sources read

- VBAM core rules: Hull Type Table (2.3.2, p. 11), Unit Tech Level Upgrade
  Table (4.9.3, pp. 65-66), maintenance (4.1.3, p. 20), task force command
  limit (4.6.2.1, p. 39), damage and directed damage (4.6.2.5-6, pp. 42-44),
  and the Human Union force list.
- EotS rules: counters (1.24), naval stacking (8.24), air-naval combat
  (9.12, 9.2), and replacements (11.0-11.33). The rulebook prints no counter
  values, so EotS hull strengths come only from rule examples (a 16-attack BB
  in 9.2.C; the Yamato scoring 18 hits in an offensive example in section 7). EotS conclusions here concern
  structure, not stat ratios.
- NC: the player manual's force table
  ([nc_player_manual.typ](../../manuals/nc_player_manual.typ), "Forces at
  Your Command") and the engine constants in
  [state.rs](../../../rust/nc-engine/src/maint/combat/state.rs) (DD 10/8,
  CA 30/30, BB 90/100, SB 100/120).

### The ladders

| Measure | NC DD / CA / BB | VBAM DD / CA / BB (Perseus / Alexander / Nike) |
| --- | --- | --- |
| Cost | 5 / 15 / 45 (1 : 3 : 9) | 5 / 8 / 10 (1 : 1.6 : 2) |
| Attack per cost | 2.0 / 2.0 / 2.0 | 0.8 / 0.75 / 0.8 |
| Defense per cost | 1.6 / 2.0 / 2.22 | 0.8 / 0.88 / 0.8 |
| AS x DS / cost^2 | 3.2 / 4.0 / 4.4 | 0.64 / 0.66 / 0.64 |
| Defense spread, DD : BB | 1 : 12.5 | 1 : 2 |
| Speed | 6 / 5 / 4 | equal for all hulls; only the Fast ability differs |
| Command rating | none; fleet cap 3,000 ships | 5 / 8 / 10 |

AS x DS / cost^2 is a rough measure of line power per point: firepower times
durability, each scaled by price. The Zeus DN (cost 12, DV 8, AS 12) scores
0.67, so VBAM's curve stays flat through the dreadnought.

### VBAM prices hulls by ability points

Each hull class has an ability-point budget at TL1: DD 10, CL 12, CA 16,
BB 20, DN 24. Defense, anti-ship, and anti-fighter cost one point each, and a
rated special ability costs two per level. Build cost is about half the
budget (Perseus 4+4+2 = 10 AP at cost 5; Nike 8+8+4 = 20 AP at cost 10).
Maintenance is a flat 10% of build cost. Value per point is therefore equal
across hulls by construction.

A large hull pays off through command. A task force holds its flagship plus
as many units as the flagship's Command Rating, so a battleship puts 20 AP
into a slot where a destroyer puts 10, and only large hulls can lead large
task forces. Damage costs one DV to cripple; a crippled unit's factors halve,
so a normal kill costs 1.5 DV. Directed damage, open to any firer, costs
twice the target's DV. NC's execution fire descends from it.

### EotS balances by scarcity, not price

Forces arrive on a fixed historical schedule. One replacement point restores
one step of any naval unit, battleship or light cruiser alike, and Pre-War
units are never replaced. All naval units move 5. Outside an offensive a hex
holds at most six naval units of any type, so per-counter strength matters.
Type matters through function: carriers have air range and a zone of
influence, CA, CL, and APD units carry brigades, and surface ships stacked
with carriers act as escorts that must be reduced first. The designer's note
after 9.14 says the system rewards "a balanced mix of forces" through these
hit-application rules. There is no composition bonus on the die.

### Learned-AI results (Mac mini session, 2026-10-16)

From the t189 section of the
[learned-AI plan](../plans/qkgqypzw-learned-ai-opponent-harness-hierarchical-agent-and-fleet-level-bootstrap-on-the-mac-mini.md),
as reported by the Mac mini session:

- The deployed agent's builds come from the rule-based executor. The default
  style builds about 67% cruisers and almost no battleships; the evolutionary
  tuner always ends with cruiser weight above battleship weight.
- Forced-hull styles (`04198f27`), against three default seats over 512
  100-year games, where 0.25 is even:

  | Style | Fleet mix | Win share |
  | --- | --- | --- |
  | default | about 67% cruisers | 0.271 |
  | battleship-led | 55% battleships | 0.135 |
  | destroyer-led | 66% destroyers | 0.125 |

- A mixed pool (`259b2f60`) loses 0.04-0.12 win share across the three
  personalities (default -0.075) over 512 paired 60-year games. A retuned
  mixed style scored 0.235 at 60 years and 0.158 at 100, against 0.307 and
  0.310 for a plain retune on 1,024 maps.
- Join reinforcement (`f842ffe6`) loses 0.06-0.19 in every variant.
- Tuned battleship-led style on 1,024 fresh maps, at 60 / 100 years
  (plan section t189, `5b3efbf6`): current rules 0.187 / 0.146; BB cost 35
  0.198 / 0.166; cost 30 0.232 / 0.212; cost 25 0.237 / 0.209; cost 20
  0.209 / 0.210; **BB speed 5 at cost 45: 0.307 / 0.290**, ahead of the
  default. Price never lifts the style past an even seat; speed alone does.
  A battleship force carrying transports moves at 4, while cruisers and
  transports both move 5.
- Forward muster (`d59d1061`): with warships and transports gathering at
  the world nearest a known rival instead of sailing home after each
  strike, the battleship-led style at speed 4 scores 0.258 / 0.254, an even
  seat. The speed-4 shortfall above was the AI's round trip home.
- Destroyer and scout pickets in approach sectors lift every personality by
  0.01-0.07 (`b82f82ab`).

These are executor-driven strategy results across whole campaigns, not
isolated combat tests. None isolates the mixed-fleet +1. Commits `04198f27` and
`b82f82ab` were on the Mac mini and not yet on `main` when this note was written.

### What the AI sees

The executor on `main` (`rust/nc-ai/src/executor.rs`) values a fleet as
DD 1, CA 3, BB 9: build cost, which is also AS. It ignores DS, fire type, and
crippled state, and it builds either the top-weighted hull or each hull
toward its weight's cost share. Nothing pairs a suppression line with
destroyers. The strategist network sees per-class counts but its postures
never touch hull weights.

### Equal-cost battles without the AI

To separate rules from AI, nine 450-point fleets fought a round robin on the
combat model's `engine-t065` rules (the preset checked against the engine in
t091), with the mixed-fleet +1 on and off. Both sides fought at ROE 10;
20,000 trials per pairing, about ±0.7 points. Script:
[hull_mix.py](../../../tools/combat-model/hull_mix.py); output:
[nc_hull_mix.json](../../../tools/combat-model/nc_hull_mix.json). Mean win
rate against the other eight:

| Fleet (450 points) | +1 on | +1 off |
| --- | --- | --- |
| 90 DD | 0.253 | 0.299 |
| 30 CA | 0.425 | 0.500 |
| 10 BB | 0.551 | **0.621** |
| 27 CA + 9 DD | 0.480 | 0.467 |
| 24 CA + 18 DD | 0.461 | 0.436 |
| 9 BB + 9 DD | 0.594 | 0.572 |
| 8 BB + 18 DD | 0.565 | 0.529 |
| 5 BB + 15 CA | **0.596** | 0.551 |
| 4 BB + 15 CA + 9 DD | 0.558 | 0.509 |

Head to head, 10 BB beat 30 CA 59-60% and 30 CA beat 90 DD 68-69% under
either setting. With the bonus off, adding destroyers to a cruiser or
battleship line lowers its win rate: 27 CA + 9 DD loses to 30 CA 46-54.
With the bonus on, the same mix wins 56-44. A single battle counts
destroyed build points only; crippled ships recover, speed acts only in
withdrawal, and the model is not the engine.

ROE 10 hides execution fire: in a fight to the finish the loser dies
whatever fired at it. New fleets start at ROE 6, and the tuned personalities
run at ROE 4.2-8.0, so the same round robin was rerun at ROE 6
([nc_hull_mix_roe6.json](../../../tools/combat-model/nc_hull_mix_roe6.json)).
There the loser usually withdraws, and crippled ships escape to recover.
Mean field-win rate, and destroyed build points dealt over lost, summed over
the eight opponents:

| Fleet (450 points) | Win, +1 on | Dealt : lost, +1 on | Win, +1 off | Dealt : lost, +1 off |
| --- | --- | --- | --- | --- |
| 90 DD | 0.292 | **1.44** | 0.353 | **1.67** |
| 30 CA | 0.435 | 0.65 | 0.494 | 0.83 |
| 10 BB | 0.601 | 0.85 | **0.648** | 1.03 |
| 27 CA + 9 DD | 0.439 | 0.94 | 0.423 | 0.85 |
| 24 CA + 18 DD | 0.382 | 1.06 | 0.373 | 0.98 |
| 9 BB + 9 DD | **0.627** | 1.05 | 0.588 | 0.95 |
| 8 BB + 18 DD | 0.569 | 1.16 | 0.554 | 1.07 |
| 5 BB + 15 CA | 0.605 | 0.85 | 0.567 | 0.71 |
| 4 BB + 15 CA + 9 DD | 0.551 | 0.87 | 0.500 | 0.75 |

**Engine check.** The same 36 ROE 6 pairings, +1 on, ran through the
engine's `combat_odds` harness (4,000 trials each, every trial fought;
cases added to
[combat_odds.rs](../../../rust/nc-fuzz/examples/combat_odds.rs), output
[nc_engine_odds_roe6_mix.json](../../../tools/combat-model/nc_engine_odds_roe6_mix.json)).
The rankings and the attrition pattern hold: field-win rates 90 DD 0.271,
30 CA 0.427, 10 BB 0.615, 9 BB + 9 DD 0.630; dealt-to-lost ratios 1.39,
0.65, 0.91, and 1.08. Single pairings differ from the model by up to 4.7
points, and 13 of 36 lie outside sampling error (|z| > 2.5). The mean
difference is -0.003. The model slightly favors destroyer-heavy fleets at
ROE 6, by 2-5 points, so its destroyer numbers are an upper bound.
Destroyed build points agree within 11 per battle.

Within a battle both fire types remove guns at the same rate per hit:
30 suppression hits cripple a cruiser (15 AS), 60 execution hits destroy
it (30 AS). Execution's edge is permanence. Destroyers lose the field
because they are more fragile per point.

### Speed, economics, and bombardment

Break-even counts on the same rules, fought to the finish (ROE 10), 20,000
trials each ([hull_breakeven.py](../../../tools/combat-model/hull_breakeven.py),
[nc_hull_breakeven.json](../../../tools/combat-model/nc_hull_breakeven.json)):
10 BB (450 points) match about 32 CA (480) and about 107 DD (535); 30 CA
(450) match about 100 DD (500). Per point, a battleship is worth about 1.07
cruisers in a fleet battle and a cruiser about 1.11 destroyers. At ROE 6 the
loser usually withdraws, so losses shrink and the edges stay the same.

The fleet-battle edge is small next to the other differences:

| Per build point | DD | CA | BB |
| --- | --- | --- | --- |
| Fleet battle (CA = 1) | 0.90 | 1.00 | 1.07 |
| Bombardment AS (`bombard_attack_as`, 0.5x / 1x / 1.5x) | 1.0 | 2.0 | 3.0 |
| Trip time, any distance (CA = 1) | 0.83 | 1.00 | 1.25 |
| Smallest build | 5 | 15 | 45 |

Trip time is `(d + 2) / speed` years for an orbit-to-orbit trip of `d`
sectors (climb and descent cost one unit each; see the
[movement spec](../../dev/ec-movement-spec.md)), so the ratios hold at every
distance. NC charges no upkeep. A world without a starbase spends up to its
Production a year, and a ship is usable only when fully paid: at Production
20 a cruiser finishes every 0.75 years, a battleship every 2.25. Unspent
points stay in the treasury and partial builds carry over, so leftover
budget is never wasted.

**Speed in battle.** Speed acts in a battle only through withdrawal and
pursuit. The ROE 6 round robin with the +1 off, rerun with BB 5 and with BB
and DD both 5 ([hull_speed.py](../../../tools/combat-model/hull_speed.py),
[nc_hull_speed_roe6.json](../../../tools/combat-model/nc_hull_speed_roe6.json)),
leaves every field-win rate within 0.003. It moves attrition: BB 5 raises
10 BB from 1.02 to 1.17 dealt-to-lost, since battleships can now pursue
cruisers; DD 5 lowers 90 DD from 1.68 (1.59 with BB 5) to 1.43, still the
best attrition fleet. Speed's larger effects, reach, interception, and
pickets, are campaign effects this model does not see.

## Finding and resolution

1. **For holding the field, size wins.** At equal cost the order is
   BB > CA > DD at both ROE 10 and ROE 6, which follows from NC's rising DS per point (1.6 / 2.0 /
   2.22). This is a property of the rules, not of the AI.

2. **At the default ROE, destroyers win the attrition war and lose the
   field.** At ROE 6 a destroyer swarm holds the field least often (0.29-0.35)
   yet destroys 1.4-1.7 times the build points it loses, the only fleet that
   comes out well ahead. Adding destroyers to a line raises its attrition
   ratio (30 CA 0.65 to 24 CA + 18 DD 1.06; 10 BB 0.85 to 8 BB + 18 DD 1.16,
   +1 on). Heavy hulls hold the field; destroyers make losses stick. That is
   the trade the manual describes. At ROE 10, where everything dies,
   execution earns nothing and the mixes pay only through the +1. Which
   matters more, the field or the attrition, is a campaign question: missions
   need the field, the economy feels the attrition.

3. **Speed 4 is the battleship's price, and it is sound.** Per point
   against a cruiser, a battleship is 7% stronger in battle, 11% more
   durable, and bombards at 1.5 times. At the same speed and price it would
   beat the cruiser on every axis but build size. Speed 4 pays for that, as
   in classic EC, and suits the siege role: battleship fleets bombard a world
   before the invasion. The AI's earlier shortfall at speed 4 came from an
   executor that sailed home after every strike; with forward muster,
   battleship-led seats reach an even seat, which confirms the design. Game
   balance comes from this design analysis; AI results confirm it and do not
   drive it.

4. **Mixed fleets losing in campaigns is likely the AI or speed.** With the
   bonus on, which the trials used, 27 CA + 9 DD beats 30 CA in battle, yet
   the cost-share mix loses in campaigns. The executor's mixes move at their
   slowest hull and are not built as deliberate pairings, so the trials
   tested something other than the manual's pairing.

5. **The destroyer's economic niche is gone.** In classic EC the owner
   built destroyers with each world's leftover budget and used them as
   pickets and explorers. NC keeps leftover points in the treasury and
   carries partial builds over, so a destroyer is no longer the way to use
   spare points. What remains is the picket role (speed 6, cost 5, against
   a 15-point scout) and attrition: making losses permanent (finding 2).

6. **The sources do not use NC's levers.** VBAM separates hulls by command
   and special abilities, EotS by function and scarcity. Neither makes size
   cost speed or ties fire type to hull class. NC's +39% line power per point
   from DD to BB has no counterpart in VBAM, whose curve is flat. A flatter
   NC curve (for example BB DS 90) would make battle balance more even but
   would push battleships further out of campaign use unless speed or price
   moves with it. NC's 1 : 12.5 defense spread, against VBAM's 1 : 2, also
   makes remainder rules matter more per hull.

7. **The mixed-fleet +1 has no precedent in either source.** At ROE 6,
   destroyers keep an attrition role without it (90 DD reach 1.67 with the
   bonus off), so dropping the bonus, as
   [rolr9ti0](rolr9ti0-independent-nc-combat-balance-analysis.md)
   recommends, would not leave them useless. It would cost mixes about
   0.02-0.05 field-win rate. Balance work should use ROE 6, the default,
   with ROE 10 only as a bound.

## Validation and remaining work

- **Executor pairing test:** a style that keeps 10-20% of cost in
  destroyers inside the strike stack, with a strength estimate that counts
  crippled ships at half and weighs DS, under the no-bonus rule.
- **Campaign value of execution:** a multi-battle model in which crippled
  ships recover after two turns, to price destroyers' permanent kills.
- Measure the per-hull effect of the three remainder treatments before
  changing CA or BB stats.

**Decisions:** the mixed-fleet bonus is removed
([r4hd41gi](../decisions/r4hd41gi-mixed-fleets-get-no-combat-bonus.md),
t198). Battleship speed 5 was proposed and then rejected; every speed stays
classic ([mga5ezbw](../decisions/mga5ezbw-battleships-move-at-speed-5.md)).

## Approved ship table

The owner approved this table on 2026-10-16. Every stat stays as it is;
the only rule change is the removal of the mixed-fleet bonus (t198).

| Ship | Cost | Speed | AS | DS | Fire | Bombard |
| --- | --- | --- | --- | --- | --- | --- |
| Destroyer | 5 | 6 | 10 | 8 | execution | 0.5x |
| Cruiser | 15 | 5 | 30 | 30 | suppression | 1x |
| Battleship | 45 | 4 | 90 | 100 | suppression | 1.5x |
| Scout | 15 | 6 | 0 | 10 | none | none |
| Transport | 5 | 5 | 0 | 10 | none | none |
| ETAC | 20 | 3 | 0 | 20 | none | none |
| Starbase | 50 | 1 | 100 | 120 | execution | none |

Ground batteries (20, AS 90, DS 20, execution) and armies (2, AS 10,
DS 10) are unchanged. Scout, transport, ETAC, and starbase values were not
tested here; scout price against a destroyer picket is worth a later look.

## Proposed punch ladder (t200)

Equal AS per point leaves cruisers and battleships close in a fleet battle
(battleship +7% per point) and distinct only by role. The owner proposed
giving smaller hulls more punch per point, and approved testing two
ladders on 2026-10-16; battleships stay 45 / 4 / 90 / 100:

| Ladder | DD | CA | AS per point, DD / CA / BB | Line power proxy, CA = 1 |
| --- | --- | --- | --- | --- |
| A | 11 / 8 | 33 / 30 | 2.2 / 2.2 / 2.0 | 0.80 / 1.00 / 1.01 |
| B (preferred) | 12 / 8 | 33 / 30 | 2.4 / 2.2 / 2.0 | 0.87 / 1.00 / 1.01 |

Durability per point rises with size (1.6 / 2.0 / 2.22), so punch falls and
staying power rises from destroyer to battleship. The proxy is
`AS x DS / cost^2`; it overstated the measured break-evens slightly (BB +11%
proxy against +7% measured).

Tests, with the bonus off and ROE 6, on the model and then the engine: the
nine-fleet round robin, break-even counts, and attrition ratios. Ladder B
passes if an all-destroyer fleet does not beat cruiser or battleship fleets
on field wins and its attrition ratio stays near today's (1.4-1.7);
otherwise ladder A. Cruisers and battleships should come out roughly even
per point in battle.

## Connections

- [ebrogywq](ebrogywq-nc-combat-design-review-against-its-sources.md): the
  procedure-level review of the same sources; this note adds the hull curve.
- [rolr9ti0](rolr9ti0-independent-nc-combat-balance-analysis.md): set
  DD 10/8 at cost 5 and recommended dropping the mixed-fleet bonus; this note
  supplies source and AI context for both.
- [qkgqypzw](../plans/qkgqypzw-learned-ai-opponent-harness-hierarchical-agent-and-fleet-level-bootstrap-on-the-mac-mini.md):
  source of the learned-AI numbers.
- [VBAM review](../../dev/combat-review/victory-by-any-means.md) and
  [EotS review](../../dev/combat-review/empire-of-the-sun.md): rule-by-rule
  comparisons of combat procedure.
