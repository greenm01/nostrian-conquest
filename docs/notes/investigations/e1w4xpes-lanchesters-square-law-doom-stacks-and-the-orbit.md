---
id: e1w4xpes
date: 2026-10-31
kind: investigation
status: investigating
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
reaches the cripple-first buffer. If heavy stacks should also bleed in fleet
battles, that needs a damage rule as well: for example, letting suppression
destroy crippled hulls before every healthy hull is crippled. Hughes's
break-off data suggest ending battles earlier, but Helmbold found fixed
break points don't fit history, so NC's ROE already plays that role. That is
a separate decision.

t232 tracks the choice. No rule has changed.

## Validation and remaining work

- Deterministic: the orbital sweep (500 seeds per case) and the fleet
  sweep (300 seeds per case), run on today's engine and on temporary
  patches. All the patches have been reverted.
- Not yet measured: multi-year sieges with reinforcement; invasion
  suppression and blitz fire, which share the battery exchange; and fleet
  battles between different hull mixes or with a starbase present.
- Not playtested. Either rule needs a manual change, the "three times the
  batteries' cost" paragraph rechecked, and the pending AI retrain for
  [bd8trz18](../decisions/bd8trz18-battles-pair-one-fleet-with-one-target-as-classic-does.md)
  run on the chosen rule.
- Open decision: t232 in [todo.md](../../../todo.md).

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
