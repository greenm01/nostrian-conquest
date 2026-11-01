---
id: e1w4xpes
date: 2026-10-31
kind: investigation
status: resolved
tags: [investigation, combat, balance, simulation, doom-stacks]
---
# Lanchester's square law, doom stacks and the orbit

## Question

In the owner's AI playtest on crunch (`games/ai2`, year 3031), Empire 2's
9th Fleet grew to CA33 DD154 on Rendezvous and broke worlds one after
another. Its empire held CA113 DD355 against the human's BB3 CA41 DD34. Why
does NC reward one huge stack so strongly? How did classic Esterian
Conquest limit it? Which simple rule, true to the original, would give
planets a fighting chance without forbidding large fleets? The owner wants
large fleets kept for fleet-on-fleet battles, real consequences when they
attack ground batteries, and fortresses that can still be broken.

## Lanchester's square law

Frederick Lanchester showed in 1916 that when every unit on both sides can
aim at the enemy, a force's fighting strength grows with the *square* of its
size. Each side's losses per round are proportional to the other side's
numbers. Double a force and it deals twice the damage, while its own losses
are spread over twice the hulls. Over a battle that compounds. Two fleets
of 100 and 200 equal ships do not fight at 1:2. They fight at 1:4. The
larger fleet wins with about 173 ships left, since `sqrt(200² − 100²) ≈ 173`.
It loses 27 ships to kill 100.

Under the *linear* law, by contrast, only a fixed front can fight at once, as
in a narrow pass. Extra ships add staying power but no firepower. The same
200 ships beat 100 and lose about 100.

The square law is why concentration pays so well in most strategy games.
Ernest Adams's [Designer's Notebook on Lanchester's laws](https://www.gamedeveloper.com/design/the-designer-s-notebook-kicking-butt-by-the-numbers-lanchester-s-laws)
puts it this way: a force twice as large is four times as strong at twice
the cost.

## How NC follows the square law

NC combat is square-law combat with an amplifier.

- **Every ship fires every exchange.** A side's hits are
  `ceil(AS × effectiveness)`, and AS is the sum over every hull in the battle
  ([manual, How a Fleet Battle Is Fought](../../manuals/nc_player_manual.typ)).
  Nothing limits how much of a side can fire.
- **Orbital fire reads a ratio column for each side.** Bombardment,
  invasion suppression and battery return fire pick their Bombardment Table
  column from the force ratio (`base_combat_column` in
  [exchange.rs](../../../rust/nc-engine/src/maint/combat/exchange.rs)). A
  bigger fleet moves its own fire up toward Overwhelming and pushes the
  batteries' return fire down to Disadvantaged. At the planet, size wins twice.
- **No cost scales with size.** There is no upkeep, no supply limit and no
  command limit. A fleet holds up to 3,000 ships and pays only by moving at
  its slowest ship's speed.

The playtest shows the result. A rendezvous stack is the best plan for the
executor, and both the land-grab and rusher personalities build it from
cruisers and destroyers alone. Under today's rules, a fleet more than 2.5
times the batteries' AS breaks any fortress in one year, and the batteries
cost it no more than they would cost a fleet a quarter its size (simulation
below).

## How classic EC limited it

Classic ECMAINT fought shooter by shooter, but it did not let a larger side
fire with everything. Its *engagement count*
([ec-battle-spec §1.3](../../dev/ec-battle-spec.md)) commits only the
first units of the larger side each round, heaviest first. They add up to
about the enemy's value times a damped ratio:

```
A = value of the larger side, B = value of the smaller side
y = (A/B)^(1/e)              fleet e = 3.3, planetary e = 4.0
if y > m: y = (y + m) / 2    fleet m = 1.3, planetary m = 1.5
fleet battles only: y = min(y, 2.0)
the larger side fires with about B × y of value; the rest hold their fire
```

| Larger side ÷ smaller side | Fleet battle fires | Planetary battle fires |
|---|---|---|
| 1.5 | 1.13 × enemy | 1.11 × enemy |
| 2 | 1.23 × enemy | 1.19 × enemy |
| 4 | 1.41 × enemy | 1.41 × enemy |
| 10 | 1.65 × enemy | 1.64 × enemy |
| 100 | 2.00 × enemy | 2.33 × enemy |

So classic was close to linear-law combat at large ratios. A stack ten times
the enemy's size fired with about 1.65 times the enemy's value. Its extra
ships kept it in the fight, but they did not make each round deadlier.
Concentration still won in classic, but every extra ship returned less.
This was the original game's own limit on doom stacks, and it applied to
fleets and to fortresses alike.

NC kept classic's road to the first shot and replaced the battle itself
([b5u6lekb](../decisions/b5u6lekb-rust-keeps-its-own-combat-system.md)).
The [combat reference](../../dev/ec-combat-reference.md) lists "the larger
side holds part of its force back each round" as a difference. No decision
records dropping it on purpose. It went with the classic battle model.

## What the square law limits in NC

- **One answer to every war.** The best plan is to mass everything and hit
  one target. Splitting forces is a mistake, and under
  [bd8trz18](../decisions/bd8trz18-battles-pair-one-fleet-with-one-target-as-classic-does.md)
  (one fleet meets one target) split fleets are beaten one at a time.
- **Defence stops scaling.** Batteries and starbases cost the same no matter
  what attacks them, while an attacker's strength grows with the square of
  its size. A fortified world can bleed a right-sized siege, but not a stack.
- **Cheap-hull spam.** With no cost for size, the cheapest AS per point wins
  the stack. Destroyers give the most AS per point, and the AI builds
  hundreds.
- **The leader snowballs.** An empire 2.5 times larger in production
  can field a stack that wins every battle at little loss. Smaller empires
  cannot trade efficiently, so the game is often settled by the first big
  battle (see [The Snowball and the Steamroller](http://www.big-game-theory.com/2015/02/the-snowball-and-steamroller.html)).
- **Less for the AI to learn.** Under these rules, the AI's best strategy
  really is one stack. That works against the owner's goal of an AI that
  uses every order (see the
  [AI opponent plan](../plans/qkgqypzw-learned-ai-opponent-harness-hierarchical-agent-and-fleet-level-bootstrap-on-the-mac-mini.md)).

## How other games handle it

Other games use three mechanical answers. A survey of the VBAM 3rd
edition rules and the web for the owner on 2026-10-31 found:

- **Cap how much fights.** VBAM's task-force command limit fields a flagship
  plus its Command Rating in ships, and the rest wait in reserve.
  Hearts of Iron IV applies terrain combat width with an overstacking penalty.
  Europa Universalis IV limits the front row to the combat width. Classic
  EC's engagement count belongs to this family.
- **Charge for size.** Master of Orion 2 command points, from starbases, with
  10 BC a turn for each point over. Stellaris naval capacity raises upkeep in
  proportion to the overage. VBAM charges 10% maintenance.
- **Punish standing stacks.** EU4 supply limits and VBAM supply attrition.
- Civilization IV gives siege units collateral damage against the whole stack.

## What the research says

Operations research has studied Lanchester's laws against real battles for
a century. Very few battles fit the square law. These sources were found on
2026-10-31. Items marked *unverified* could not be read in full.

- **Osipov (1915)** fitted 38 battles from 1805–1905 and found a 3/2-power
  law, not a square law. A force twice as large is about 2.8 times as
  strong, not 4. Translated by Helmbold and Rehm as
  [CAA-RP-91-2](https://apps.dtic.mil/sti/tr/pdf/ADA241534.pdf) and
  summarised by [The Dupuy Institute](https://dupuyinstitute.org/2017/07/03/osipov/).
  *Unverified: the 3/2 figure comes from secondary sources.*
- **Helmbold (1965)**, [A Modification of Lanchester's Equations](https://pubsonline.informs.org/doi/10.1287/opre.13.5.857),
  *Operations Research* 13(5). It corrects the square law for "inefficiencies
  of scale when force sizes are grossly unequal" by scaling each side's fire
  by a power of the force ratio, usually written
  `fire_A = a · A^W · B^(1−W)`. `W = 1` is the square law and `W = ½` the
  linear law. *Unverified: the exact notation is behind a paywall.*
- **Hartley and Kruse (1989)**, [Historical support for a mixed law
  Lanchestrian attrition model](https://www.osti.gov/biblio/5087936), and
  **Hartley (1990)**, [Confirming the Lanchestrian linear-logarithmic
  model](https://www.osti.gov/biblio/6066944). Hundreds of battles support a
  law between the linear and the logarithmic, not the square law.
- **Lucas and Turkes (2004)**, [Fitting Lanchester Equations to the Battles
  of Kursk and Ardennes](https://hdl.handle.net/10945/44169), *Naval Research
  Logistics* 51. They review fits of the Ardennes and Kursk: Bracken found the
  linear law, Fricker and Wiper the logarithmic law, and Wiper calls the
  square law "implausible". Engel's 1954 [Iwo Jima fit](https://www2.kenyon.edu/Depts/Math/Paquin/LanchesterVerification.pdf)
  is the rare battle that fits the square law.
- **Hughes (1995)**, [Two Effects of Firepower: Attrition and
  Suppression](https://hdl.handle.net/10945/38442). Citing McQuie's 80
  battles, the median attacker broke off at 4% casualties and the defender at
  8%. "The square law advantage of numbers has rarely been achieved."
  Hughes's salvo model resolves combat in discrete exchanges, as NC does.
- **Kuikka (2015)**, *A Combat Equation Derived from Stochastic Modeling of
  Attrition Data*, *Military Operations Research* 20(3), 49–69. Each force
  decays as a geometric Brownian motion, and a battle ends when one side
  crosses a decision boundary that falls over time. The boundaries track
  both sides' losses, not a fixed percentage. The winner depends on
  `S = (x_end/x₀) / (y_end/y₀)`, the ratio of the fractions each side keeps.
  Against Hartley's 593 battles it calls 438 of 552 decided battles (79%),
  the same as the Helmbold and Willard formulas. The attacker's edge fits
  best at about 1.1. It notes that sides "don't immediately use all the
  available force", so effective forces are smaller than documented ones.
  The model forecasts and explains battles; it is not a rule for resolving
  combat. For NC it supports two points. Classic's engagement count models
  that idle part of a force. ROE, which weighs odds rather than a fixed loss
  percentage, is the kind of relative break point the data fit, unlike the
  fixed break points Helmbold rejected.
- **Kuikka (2021)**, *Modelling of Military Capabilities, Combat Outcomes
  and Networked Systems with Probabilistic Methods*, doctoral thesis,
  National Defence University (Finland), Series 1 No. 44. It collects the
  2015 paper and a 2017 follow-up (*Journal of Applied Operational Research*
  9(1)) that also predicts how long battles last. In Hartley's data a side
  quits after losing about 10% of its starting force. Hotter fighting ends
  battles sooner, and more resilient sides make them longer. Matching both
  win rates and lengths needs a stubborner loser when one side is much
  stronger. A separate chapter on how many units to buy treats extra units
  as additive up to a threshold and as redundant above it
  (`1 − (1 − x)^n`). That is the overkill regime NC's orbit already shows
  past about 2.5:1, where extra ships neither help nor cost anything.
- **Kress (2020)**, [Lanchester Models for Irregular Warfare](https://www.mdpi.com/2227-7390/8/5/737).
  Aimed fire gives the square law. Area fire, whose hits depend on how many
  targets are present, gives the linear law.
- **Games.** Stellaris gives the smaller fleet "Force Disparity", a fire-rate
  bonus up to +100% against a fleet twice its size ([wiki](https://stellaris.paradoxwikis.com/Space_warfare);
  *unverified against the dev diary*). Civilization V's one unit per tile
  ended doom stacks but clogged the map, and its lead designer later called it a
  mistake ([postmortem](https://www.gamedeveloper.com/design/classic-postmortem-firaxis-i-civilization-v-i-)).
  Board-wargame combat results tables stop at about 6:1, so strength beyond
  that ratio is wasted.

**Classic EC's engagement count is a Helmbold factor.** The larger side
fires with `B · (A/B)^(1/e) = A^(1/e) · B^(1 − 1/e)`. That is Helmbold's
form with `W = 1/e ≈ 0.30` in fleet battles and 0.25 at planets, before the
damping and the 2.0 cap. Those values sit inside the linear-to-logarithmic
band that Hartley found in historical battles. In effect, classic ECMAINT
already ran on the empirical law, not the square law.

## Simulation: three orbital rules

**Setup.** The empty-board fixture of the combat tests
(`rust/nc-engine/tests/combat/support.rs`) puts a world at (15,13) with
10, 20 or 40 ground batteries (5 in a spot check). A fleet orbits it on
Bombard World at ROE 10. Each case ran one maintenance year on 500 seeds,
at the code of commit `85ed9b88`, in a release build. Fleets were all
destroyers, all cruisers, all battleships, or the AI's mix of 3 destroyers
to 1 cruiser. Each was sized so its bombardment AS was `r` times the
batteries' AS (battery 90, destroyer 12, cruiser 33, battleship 90). The
three rules ran on the same seeds, so they are compared on the same dice.
The harness and the experimental rule patches were temporary and are not
in the tree. Rerunning needs both patches, which this section describes in
full.

- **Now:** today's code.
- **Target-rich orbit:** battery return fire reads one column better for
  each doubling of `r` past 3, up to three steps: Pressed at 3–6, Even at
  6–12, Advantaged at 12 and up. Nothing else changes.
- **Classic cap:** classic's planetary engagement count, in AS: the fleet's
  AS that fires, and the ratio both sides' columns read, is
  `min(A, B × y)` with planetary `e = 4`, `m = 1.5`.

**Toll** is the attacker's mean loss in production points that year,
divided by the batteries' cost (20 a battery). Below 1, the batteries cost
less than the ships they killed; above 1, more. Results for 10 batteries.
The 20- and 40-battery runs match within a few hundredths, since the toll
scales with battery count.

| r | 1 | 1.5 | 2 | 2.5 | 3 | 4 | 6 | 8 | 12 | 24 |
|---|---|---|---|---|---|---|---|---|---|---|
| DD now | 1.74 | 1.39 | 1.37 | 0.89 | 0.89 | 0.89 | 0.89 | 0.89 | 0.89 | 0.89 |
| DD target-rich | 1.74 | 1.40 | 1.37 | 0.89 | 1.37 | 1.37 | 1.84 | 1.84 | 2.29 | 2.29 |
| DD classic cap | 1.75 | 1.69 | 1.61 | 1.59 | 1.57 | 1.50 | 1.40 | 1.40 | 1.38 | 1.37 |
| CA now | 1.71 | 0.95 | 0.60 | 0.60 | 0.60 | 0.60 | 0.60 | 0.60 | 0.60 | 0.60 |
| CA target-rich | 1.71 | 0.96 | 0.60 | 0.60 | 0.94 | 0.94 | 1.28 | 1.28 | 1.28 | 1.59 |
| CA classic cap | 1.73 | 1.14 | 1.10 | 1.08 | 1.07 | 1.03 | 0.96 | 0.96 | 0.95 | 0.94 |
| BB now | 1.27 | 0.80 | 0.79 | 0.52 | 0.52 | 0.52 | 0.52 | 0.52 | 0.52 | 0.52 |
| BB target-rich | 1.27 | 0.81 | 0.79 | 0.52 | 0.79 | 0.79 | 1.06 | 1.06 | 1.31 | 1.31 |
| BB classic cap | 1.28 | 0.91 | 0.88 | 0.86 | 0.86 | 0.85 | 0.81 | 0.80 | 0.79 | 0.79 |
| AI mix now | 1.81 | 1.34 | 1.35 | 0.89 | 0.89 | 0.89 | 0.89 | 0.89 | 0.89 | 0.89 |
| AI mix target-rich | 1.81 | 1.34 | 1.35 | 0.89 | 0.89 | 1.37 | 1.37 | 1.84 | 2.29 | 2.29 |
| AI mix classic cap | 1.82 | 1.57 | 1.57 | 1.57 | 1.57 | 1.50 | 1.40 | 1.40 | 1.38 | 1.37 |

Ship counts are rounded, so a column's real ratio can be a little off its
label. That is why a step sometimes lands one column early or late.

**Fortresses still fall.** Across every mix and battery count, a fleet at
`r ≥ 1.5` broke the batteries in the first year at least 96% of the time
under today's rule and the target-rich rule. The two rules are identical
below `r = 3`. Under the classic cap it was at least 81% at `r ≥ 1.5` and
98% at `r ≥ 3`. At `r = 1` a fleet broke them 24–88% of the time today
(battleships best), and 19–78% under the classic cap. No rule makes a
world unbreakable.

**Findings.**

1. **Today, overkill is free at the orbit.** The toll falls to its floor at
   `r ≈ 2.5` and stays there however large the fleet grows. A 24× stack
   pays exactly what a 2.5× fleet pays, because the batteries read
   Disadvantaged at any `r > 2`. A right-sized siege at `r = 1.5–2` pays
   *more* (1.4 times battery cost for destroyers) than the stack does.
2. **Target-rich orbit charges the stack and leaves sieges alone.** It
   matches today's rule below `r = 3` and raises the toll by steps after:
   destroyer stacks pay up to 2.3 times the batteries' cost, cruisers 1.6,
   battleships 1.3. The cheapest way in becomes a fleet at `r ≈ 2.5`, so a
   stack's best move is to send a detachment. Because battery fire kills
   destroyers first at twice their DS, the rule falls hardest on destroyer
   spam and favours battleships.
3. **Classic cap removes the reward for size but adds no penalty.** The toll
   is nearly flat in `r`: destroyers pay about 1.4–1.6, cruisers about 1.0,
   battleships about 0.8 at any size. It also makes mid-size sieges
   costlier than today, about 1.6 instead of 1.4 for destroyers at `r = 2`,
   and less certain: as low as 81% instead of 96% at `r = 1.5`. That changes the
   balance the manual's "three times the batteries' cost" rule describes.

## Simulation: classic's engagement count in fleet battles

**Setup.** On the same board, an empire 1 fleet and an empire 2 fleet, both
at ROE 10 and mutual enemies, meet in deep space at (8,8). The defender has 20
or 60 units of all cruisers, all destroyers, or the AI's mix (3 destroyers
and 1 cruiser per unit). The attacker has `r` times as many units of the
same kind. Each case ran one maintenance year on 300 seeds, under today's
rule and under classic's fleet engagement count (`e = 3.3`, `m = 1.3`,
capped at 2.0). The count limits the AS each side fires with, in proportion
to its suppression and execution shares. It doesn't change the die bonuses.
**Loss** means hulls destroyed. Crippled hulls recover on their own, so they
are not counted.

The winner's loss as a fraction of its fleet, 60 defending units. The square
law predicts `1 − sqrt(1 − 1/r²)`; the linear law predicts `1/r`.

| r | 1.25 | 1.5 | 2 | 3 | 5 | 10 |
|---|---|---|---|---|---|---|
| Square law | 0.40 | 0.26 | 0.13 | 0.06 | 0.02 | 0.005 |
| Linear law | 0.80 | 0.67 | 0.50 | 0.33 | 0.20 | 0.10 |
| CA today | 0.27 | 0.11 | 0.004 | 0.001 | 0.000 | 0.000 |
| CA classic count | 0.40 | 0.22 | 0.031 | 0.002 | 0.001 | 0.001 |
| DD today | 0.60 | 0.49 | 0.34 | 0.21 | 0.11 | 0.06 |
| DD classic count | 0.65 | 0.56 | 0.42 | 0.27 | 0.16 | 0.08 |
| AI mix today | 0.57 | 0.44 | 0.27 | 0.17 | 0.09 | 0.05 |
| AI mix classic count | 0.63 | 0.51 | 0.33 | 0.21 | 0.12 | 0.06 |

The attacker's chance of wiping out the defender barely moved: by at most
0.07, at `r = 1.25–1.5`.

**Findings.**

1. **Heavy stacks beat the square law.** At 2:1 a cruiser fleet loses 0.4%
   of its hulls, where the square law predicts 13%. The cause is the
   suppression walk: fire must cripple every healthy hull of a class before it
   can destroy one crippled hull. A stack's healthy hulls form a buffer the
   smaller side can't fire through, so its fire only cripples, and cripples
   recover for free. Above about 1.5:1, a heavy stack is close to immune.
2. **Destroyer fleets sit between the two laws.** Execution fire destroys
   outright, so destroyer stacks do take losses.
3. **Classic's engagement count is a mild correction.** It raises the
   winner's losses by about 20–40% for destroyers and the AI mix,
   and from 0.4% to 3% for cruisers at 2:1. It never changes who wins. It
   can't touch the cripple-first buffer, which is a damage rule, not a
   firepower rule.

## Measuring the law: Helmbold's regularities

Helmbold (1971), *Air Battles and Ground Battles: A Common Pattern?* (RAND
P-4548), argues that combat models should be tested by whether they
reproduce the regularities in historical battle data, not by replaying
single battles. He writes the square law `dx/dt = −D·y`, `dy/dt = −A·x`,
so each battle yields an activity ratio
`D/A = (x₀² − x_t²) / (y₀² − y_t²)`. With the fractions left, `a = x_t/x₀`
and `d = y_t/y₀`, he defines the relative advantage `μ`, where
`μ² = (1 − a²) / (1 − d²)`, the defender's advantage `V = ln μ`, and the
bitterness `ε = λT`, where `λ = sqrt(A·D)` and `T` is the battle's length.
His land-battle fits are:

- `ln(D/A) = 0.230 + 1.266 · ln(x₀/y₀)`. A pure square-law world would
  give a slope of 0. History says the larger force's activity falls steeply
  as its numerical advantage grows. Air battles give a slope of 1.544.
- `ln ε` is roughly normal with mean −2.16 and SD 0.83. A typical battle
  has `ε ≈ 0.12`, and one SD covers 0.05–0.26: battles end long before
  annihilation. Air battles: mean −3.65, SD 0.30.

He is also known for showing that fixed break-point models of when a battle
ends do not fit the data. So a simple "withdraw at X% losses" rule is not
supported by his evidence.

The same quantities from the fleet sweep, 60 defending units. The slope is
a least-squares fit of `ln(D/A)` on `ln r` over `r` = 1–10.

| Fleet | Slope today | Slope with classic count | `ε` at r = 1 / 2 / 10, today |
|---|---|---|---|
| Destroyers | 1.02 | 1.16 | 1.47 / 0.96 / 0.34 |
| AI mix | 0.96 | 1.05 | 1.49 / 0.84 / 0.32 |
| Cruisers | −4.16 | −1.14 | 0.99 / 0.09 / – |
| History (land) | 1.27 | | about 0.12 (0.05–0.26) |

- **Destroyers and the AI mix fit Helmbold's slope reasonably well.**
  Classic's count moves them closer to the land value of 1.27.
- **Cruiser fleets have a negative slope.** The bigger cruiser stack's
  activity *rises* with its advantage, the opposite of history. This is
  the cripple-first buffer again, and the count only halves it.
- **NC battles are 3–12 times bitterer than historical land battles.** At
  ROE 10, the setting the sweep uses, they run to annihilation within one
  year. Bitterness is mostly a matter of ROE and when retreat is allowed,
  not of the firepower rules.

`μ`, `ε` and `D/A` come from mean losses across seeds, ignore cripples, and
apply a continuous model to discrete exchanges. They show trends and
directions, not fitted constants.

### The effective exponent

Power-law forms of Lanchester's equations reduce to a conserved quantity
`A · x^n − D · y^n`, so a combat system can be summarised by one exponent
`n`. Two forms are quoted for Helmbold's 1965 modification.
`dx/dt = −D · y^p` (and `dy/dt = −A · x^p`) gives `n = p + 1`. The force-ratio
form `dx/dt = −D · x^(1−W) · y^W` gives `n = 2W`. In both, `n = 2` is the
square law, `n = 1` the linear law, and Osipov's historical fit is about
`n = 1.5`. The two forms differ in a way that matters for a game. The
`y^p` form taxes every large battle, including one between two equal stacks,
and needs a reference size to fix its scale. The force-ratio form, like
classic EC's engagement count, changes nothing between equal forces and
needs no reference size.

Fitting `n` to each case of the fleet sweep, with equal ships on both sides
(`r^n − (r(1−F_A))^n = 1 − (1−F_D)^n`), 60 defending units:

| r | 1.25 | 1.5 | 2 | 3 | 5 | 10 |
|---|---|---|---|---|---|---|
| CA today | 2.60 | 3.07 | 5.52 | 4.85 | ∞ | ∞ |
| CA classic count | 1.97 | 2.17 | 3.33 | 4.33 | 3.51 | 2.59 |
| DD today | 1.34 | 1.28 | 1.27 | 1.24 | 1.24 | 1.18 |
| DD classic count | 1.17 | 1.14 | 1.12 | 1.10 | 1.10 | 1.08 |
| AI mix today | 1.43 | 1.41 | 1.44 | 1.36 | 1.31 | 1.22 |
| AI mix classic count | 1.25 | 1.24 | 1.30 | 1.25 | 1.20 | 1.16 |

Destroyer fleets and the AI's mix already fight near Osipov's 3/2 law, and
classic's count pulls them toward the linear law. Cruiser fleets fight
steeper than the square law, unboundedly so (`n = ∞`) once the smaller side
can no longer destroy anything. NC's doom-stack problem in fleet battles is
therefore mainly about heavy hulls and the cripple-first rule, not about how
many ships fire.

## Finding and resolution

Doom stacks come from two rules. Every ship fires every exchange, which is
square-law combat. The orbital ratio columns make that worse at the planet.
In fleet battles, the cripple-first suppression walk makes a heavy stack
above about 1.5:1 nearly immune. Classic EC limited the first with its
engagement count, a Helmbold factor close to the historical evidence. NC
dropped it along with the classic battle model.

The owner proposed (2026-10-31) to combine:

- **classic's engagement count in fleet battles**, which is faithful,
  research-backed and mild. It raises the winner's losses without changing
  who wins;
- **the target-rich orbit for ground batteries**, which charges stacks that
  attack fortresses a toll that grows with their size, leaves ordinary
  sieges as they are, and never makes a world unbreakable.

The two rules act on different exchanges and don't interact. Neither
reaches the cripple-first buffer, so a third rule was simulated: dispersed
suppression, where suppression strikes healthy and crippled hulls in
proportion (see Simulation: dispersed suppression). Hughes's break-off data
suggest ending battles earlier, but Helmbold found fixed break points don't
fit history, so NC's ROE already plays that role.

The owner adopted all three rules on 2026-10-31:
[almb7dbt](../decisions/almb7dbt-a-larger-side-commits-part-of-its-force-and-batteries-punish-a-crowded-orbit.md),
task t232.

## Simulation: dispersed suppression

The fleet sweep again, 60 defending units, 300 seeds, with classic's count
in both columns. The figure is the attacker's loss as a share of its cost.
Who wins is the same in both columns, and destroyer fleets and the AI's mix
come out identical to three decimals: their crippled hulls are finished by
execution fire before suppression reaches them.

| Cruisers, r | Before t232 | Count only | Count + dispersed | Square law |
|---|---|---|---|---|
| 1.5 | 10.5% | 21.9% | 32.5% | 25.5% |
| 2 | 0.4% | 3.1% | 12.3% | 13.4% |
| 3 | 0.1% | 0.2% | 4.5% | 5.7% |
| 5 | 0.0% | 0.1% | 1.3% | 2.0% |

Dispersed fire brings cruiser stacks from steeper than the square law back
to about it, the same place destroyer fleets already were. The engine's
final rules reproduce this table and the orbital sweep's target-rich table
exactly.

## Simulation: bombardment leak

**Question (owner, 2026-11-01).** Bombardment strikes batteries first. Fire
reaches the dock, armies and industry only in a round after the last battery
falls. Can a world that rebuilds batteries every year keep its industry
untouched? Classic gave its bombers random passes behind the battle,
weighted 40% dock, 30% goods, 20% factories and 10% armies, whether or not
the batteries survived.

**Rule tested (t234).** In a round that starts with a battery standing,
`round(hits · share)` hits leak behind the batteries. `share = base ·
(roll + 1) / 5.5`, where `base` grows evenly with potential from `lo` at 10
to `hi` at 150 and `roll` is the round's `d10`. The leak splits 40% dock,
10% armies and 50% goods and factories, and whatever a target can't spend
passes down.

Two variants were tested:
- **Diverted:** the batteries take `hits − leak`.
- **Additive:** the batteries take every hit, and the leak comes on top.

Additive ran at `lo–hi` of 5–25%, 10–40% and 15–55%; diverted at 10–40%.

**Setup.** The empty-board fixture, with 10 armies, production equal to
potential, and goods of five times potential. All fleets were on Bombard at
ROE 10, and every variant ran on the same seeds. The harness and the rule
switch were temporary, run on bean-void.
- **One year:** 0 or 10 batteries; potential 10, 80 or 150; destroyers,
  cruisers, battleships or the AI's 3:1 mix; `r` from 1 to 25; 500 seeds.
- **Ten-year siege:** 10 batteries to start. Each year the defender spent
  50% or 100% of production on new batteries. The AI mix or battleships at
  `r` of 1, 1.5 or 3; 300 seeds.

**Findings.**
1. **Additive leaves combat alone.** In all 168 one-year cases and all 36
   siege cases, battery break rates, attacker losses and the year the
   batteries fall match the current rules exactly. Worlds with no batteries
   match exactly under every variant.
2. **Diverted fails.** At `r ≥ 1.5`, first-year break rates fall to 0.87,
   and attacker losses rise up to 45%. A battleship siege of a potential-150
   world at `r = 1` breaks the batteries within ten years in 19% of games,
   against 88% now. It makes big fortresses tougher, the opposite of the
   aim.
3. **The shield matters only to an undersized siege.** At `r ≥ 1.5` the
   batteries fall in year one and production goes to 0 that year, however
   much the defender rebuilds. At `r = 1` with the AI's mix, the siege
   breaks the batteries in 27% of games, loses about 90% of its fleet over
   ten years, and leaves the world with this much of its production:

| Potential | Now | Additive 5–25% | Additive 10–40% | Additive 15–55% | Diverted 10–40% |
|---|---|---|---|---|---|
| 30 | 78% | 68% | 59% | 49% | 64% |
| 80 | 81% | 71% | 63% | 55% | 72% |
| 150 | 88% | 74% | 65% | 57% | 75% |

   (Defender spending all its production on batteries. At half, the figures
   are within 4 points of these.) The scaling with potential evens out the
   damage as a share of production: the same hits are a smaller share of a
   big world's industry.

**Resolution.** The owner chose additive at 10–40% on 2026-11-01. The final
engine reproduces both of its tables exactly.

**Outside t234.** A fortress that falls loses all its production within the
same year of bombardment. t236 takes this up below.

## Simulation: fewer rounds and half strength behind the batteries

**Question (owner, 2026-11-01).** Once the batteries are gone, or at a world
that never had any, three full-strength rounds empty it in a year. A siege at
1.5:1 or more took all of a fortress's production in year one. So did ten
cruisers at an open world.

**Classic.** Each Bombard order is one run: the battle, collateral damage
scaled to the batteries destroyed, then up to 20 passes, less the rounds the
battle took. In a pass each warship fires one small shot at a random target.
A factory hit removes a tenth of the shot's damage. Production never drops
below 1.0, and the fleet then holds. Ten cruisers against an open world
leave 79% of the factories (ec-battle-spec, case B2).

**Rules tested (t236).** Every variant was run on the t234 engine.
- **One round once no battery stands.** The year ends with the round that
  destroys the last battery, and a world with none takes one round.
  - *Keep:* that round's leftover hits still flow on.
  - *Drop:* they are lost.
- **Half strength:** hits that pass the stardock are halved before they
  strike the armies and industry, the leak's included. The dock takes full
  fire.

**Setup.** The t234 harness and seeds again. One added run bombards an open
world with 10 armies for five years: potential 30, 80 or 150; cruisers,
battleships or the AI's mix; 33 to 2,700 AS; 300 seeds.

**Findings.**
1. **Battles are untouched.** Battery break rates and the attacker's losses
   match t234 exactly in every variant.
2. **One round alone is not enough.** Ten cruisers still leave a
   potential-80 world 41% of its production after a year, and none after
   three.
3. **One round at half strength is close to classic.** A cruiser removes
   about 2.4 points of production a year from an open world (exact model
   `tools/combat-model/collateral_t236.py`). Classic removes about 2.1.
   Production left after years 1, 3 and 5:

| World | Fleet | t234 | One round | One round + half |
|---|---|---|---|---|
| 80 | 3 CA | 51 / 0 / 0 | 92 / 50 / 10 | 100 / 82 / 63 |
| 80 | 10 CA | 0 / 0 / 0 | 41 / 0 / 0 | 77 / 9 / 0 |
| 150 | 10 CA | 4 / 0 / 0 | 68 / 3 / 0 | 88 / 50 / 13 |
| 150 | 27 CA | 0 / 0 / 0 | 12 / 0 / 0 | 55 / 0 / 0 |

4. **At a fortress of 10 batteries,** with the AI's mix and half strength,
   production lost in year one:

| Ratio | t234 | Keep | Drop |
|---|---|---|---|
| 1.5 | 99–100% | 48–62% | 25% |
| 3 | 100% | 84–91% | 48–50% |
| 25 | 100% | 100% | 94% |

   A doom stack still levels a world in a year, as one does in classic.

**Resolution.** The owner chose one round at half strength, keeping the
leftover hits, on 2026-11-01. The engine's `planetary.rs` model tests are
pinned to `collateral_t236.py`.

## Simulation: the army shield

**Question.** Under t236, fire that passes the dock strikes the armies
before the industry. An army costs 2 and soaks 20 half-strength hits. Can a
world shield its factories with cheap armies?

**Setup.** A potential-80 world with no batteries, starting with 0, 10, 25,
50 or 100 armies. It was bombarded for five years by 3, 10 or 27 cruisers,
300 seeds. In half the cases the defender spent half its production on new
armies each year. The variants were the strict order, and splits that send
a sixth or a tenth of the fire to the armies.

**Findings.** Production left after years 1, 3 and 5, under 10 cruisers,
with the defender buying armies:

| Starting armies | Strict | A sixth | A tenth |
|---|---|---|---|
| 0 | 64 / 22 / 2 | 64 / 8 / 1 | 64 / 6 / 1 |
| 10 | 77 / 40 / 7 | 70 / 12 / 1 | 68 / 7 / 1 |
| 50 | 100 / 96 / 80 | 70 / 12 / 1 | 68 / 7 / 1 |
| 100 | 100 / 100 / 100 | 70 / 12 / 1 | 68 / 7 / 1 |

- Under the strict order, the shield is real. 50 armies hold off 10
  cruisers for years, and buying armies makes the world nearly immune.
- Either split removes it: industry falls the same whatever the garrison.
- Under a sixth, a garrison of 50 still has 29 armies after five years
  without reinforcement, and 53 when the defender keeps buying them. Armies
  keep their value against invasion.

**Resolution.** The owner chose a sixth on 2026-11-01 (t238). It's the
armies' share of the leak and of classic's passes, leaving the dock out.
The engine's model tests are pinned to the updated `collateral_t236.py`.

## Validation and remaining work

- Deterministic: the orbital sweep (500 seeds per case) and the fleet
  sweep (300 seeds per case), run on the engine before t232, on temporary
  patches, and on the final engine. The final engine matches the patches
  exactly.
- Regression tests pin the count, the target-rich columns and the
  dispersed walk (`fleet_battle_tests.rs`). Tests that pinned old outcomes
  were updated: a withdrawing fleet now meets pursuit fire, a guard on ROE 0
  escapes a base it used to die beside, a guard's ETAC now outlives the
  line, and the AI seat fixtures were re-recorded.
- Not yet measured: multi-year sieges with reinforcement; invasion
  suppression and blitz fire, which share the battery exchange; and fleet
  battles with a starbase present or with battleships.
- Not playtested yet. The AI was retrained on these rules first (t233).

**The t233 retrain (promoted 2026-11-01).** The t233 run fine-tuned
`fleet-final` for 100 iterations on the t232 rules (`fleet_ppo.py --init`,
64 games, league). On those rules, the old agent's win share against three
executors fell from 0.58 to 0.375 over 60 years.

Paired `behaviour_report` results against `fleet-final`, 1,024 maps, at
`2ba010b6`:

| Snapshot | Win share, 60 years | Win share, 100 years | Start check |
|---|---|---|---|
| 50 | -0.016 ± 0.019 | -0.001 ± 0.020 | +0.024 ± 0.025 |
| 75 | +0.048 ± 0.020 | +0.051 ± 0.020 | +0.059 ± 0.022 |
| 100 | +0.055 ± 0.020 | +0.043 ± 0.021 | +0.049 ± 0.024 |

Its largest fleet holds 48% of its strength at 100 years, down from 52%.
Snapshot 75 is now `fleet-final`; the previous agent is
`fleet-final-pre-t233`. It is deployed with t232 to crunch's test BBS.
archvps keeps its release and model until a release ships t232.

The first reports ran on stale code: bean-void's checkout had no remote,
so its pull failed. They were rerun after the commits were copied over with
a git bundle.

## Connections

- [ec-battle-spec §1.3](../../dev/ec-battle-spec.md): classic's engagement
  count, decoded from ECMAINT and checked against classic runs.
- [ec-combat-reference](../../dev/ec-combat-reference.md): lists the dropped
  holding-back rule among NC's differences.
- [b5u6lekb](../decisions/b5u6lekb-rust-keeps-its-own-combat-system.md):
  why NC kept its own battle model.
- [bd8trz18](../decisions/bd8trz18-battles-pair-one-fleet-with-one-target-as-classic-does.md):
  one fleet meets one target, which punishes split fleets.
- [rolr9ti0](rolr9ti0-independent-nc-combat-balance-analysis.md): warned
  against "an arbitrary anti-stack penalty". The target-rich rule penalizes
  only overkill against batteries, and classic's count is not arbitrary.
- [tdkitij6](../decisions/tdkitij6-heavies-cover-the-destroyers-which-finish-cripples-at-half-price.md):
  the cover and execution rules that make the destroyer-cruiser mix the
  AI's stack.
