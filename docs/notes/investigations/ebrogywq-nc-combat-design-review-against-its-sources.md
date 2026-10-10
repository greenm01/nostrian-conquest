---
id: ebrogywq
date: 2026-10-10
kind: investigation
status: investigating
tags: [investigation, combat, design]
---
# NC combat design review against its sources

## Question

NC combat ([nc-combat-spec.md](../../dev/nc-combat-spec.md)) was built from
Empire of the Sun, Victory by Any Means, and the user's ec4x design. Since
then the engine gained classic's 2.5D movement and per-tick encounter loop.
Does the spec still hold together, which of its rules trace to which source,
and what should change?

## Evidence

Three reviews, each with page and section references:

- [Empire of the Sun](../../dev/combat-review/empire-of-the-sun.md)
- [Victory by Any Means](../../dev/combat-review/victory-by-any-means.md)
- [ec4x](../../dev/combat-review/ec4x.md), a rule-by-rule table

Plus the classic decodes: [ec-encounter-spec.md](../../dev/ec-encounter-spec.md)
and [ec-battle-spec.md](../../dev/ec-battle-spec.md).

## Lineage

| NC rule | Source |
| --- | --- |
| AS/DS frame, crippled step, 2x price to pick a target, bases stronger than any hull and unable to retreat, space then orbit then ground | VBAM |
| Hits = ceil(AS x multiplier), d10, critical on an unmodified 9, defender wins ties, reduce all before eliminating, auxiliaries screened, transports die with their cargo, supremacy before landing | EotS |
| ROE table (0-10 as AS ratios), cripple-first for CA/BB fire, retreat to the nearest owned world, starbase as a non-retreating partner, three-round bombardment behind a battery shield, blitz landing under fire, batteries transferring on capture, three theaters | ec4x |
| Ratio-column CRT, suppression/execution split, the 3-round commitment, the withdrawal exchange, the 64-round cap, the 10x scale, "mutual annihilation goes to the larger force", auto-repair | NC's own |

NC kept the frame of all three and added its own resolution loop. The
sources agree with each other more than any of them agrees with NC's
additions.

## Findings

### 1. The CRT counts mass twice

VBAM's CRT has one column: 10-50% of AS, and the die carries only
qualitative edges. EotS has no odds table at all, and its weakest result
still draws blood. NC picks a column from the AS ratio *and* multiplies the
AS total, so a 3:1 force hits about 9.4:1 (VBAM review 4.1). Its
Disadvantaged column has a 0.00 row, so an underdog can deal nothing. Both
sources say: flatten the columns, let mass speak through AS, and keep column
shifts for surprise, starbases, mixed fleets, and ground.

### 2. Per-round lethality hollows out ROE

NC's Even column averages 115% of AS per round against VBAM's 30%. An even
cruiser fight cripples both lines in round 1 and is decided by round 3,
exactly when NC first allows ROE to act. The 3-round commitment, the
withdrawal exchange, and the 64-round stalemate are patches on that
lethality. EotS gets "no bailing" from one mandatory exchange; VBAM from a
contested retreat roll. Neither forbids retreat.

### 3. Crippled damage heals between battles

All three sources persist step damage; EotS's designer notes call it the
engine of attrition. NC repairs after every battle, and the tick loop now
gives several battles a year, so suppression fire (CA/BB) is wasted and
"who has more destroyers" decides fights. Fix: crippled counts per class in
the typed fleet (t039), repaired at an owned world for a cost (ec4x: 25% of
build cost, one turn).

### 4. Retreat has no destination and rewrites ROE

The spec never says where a withdrawn fleet goes. The engine
(`retreat.rs`) sends it to the nearest owned world by Manhattan distance,
sets ROE 0, and falls back to Hold in enemy orbit. Classic zeroes ROE only
on scatter; ec4x and EotS never. ec4x's rule: pursuit volley, then move to
the nearest friendly system; auxiliaries lose the escort's loss ratio. On
2.5D the move is a climb out of the layer, which ends the pairing and hands
the chase to classic's speed-based intercept.

### 5. Two strength yardsticks for one ROE number

Starting a fight uses classic's `21 BB + 7 CA + 2 DD`; breaking off uses
the CRT AS ratio. Worked case: 2 BB against a base with a resting cruiser
guard scores 0.63 on one scale and 1.38 on the other. Use AS for both, with
crippled hulls at half, so the attack decision and the battle agree.

### 6. Invasion gets one suppression exchange

Bombard gets three; Invade gets one, then aborts if any battery stands.
With no retry (t036), most invasions against batteries waste the year. Give
Invade the same three rounds, or state plainly that batteries must be
bombarded first.

### 7. The orbital theater is gated by a sighting roll

Classic runs its starbase gate at the strike regardless of sighting. NC's
only gate is the encounter decision, which needs a sighting first, and
`select_orbital_supremacy_empire` ignores the owner's base whenever an
attacker has combat AS. An attacker that never rolled a sighting of the
base assaults straight past it. The gate must be deterministic.

### 8. Rules the sources argue against, in brief

- Execution fire hands the step-bypass to whole unit classes and every
  battery; EotS keeps it for critical hits only.
- A natural 9 targets the cheapest unit (`fleet_target_order` puts
  SC/TT/ET first), so criticals kill scouts and transports while the line
  stands. ec4x targets the highest-value asset.
- Mutual annihilation goes to the larger force; every source gives it to
  the defender.
- A blocked transport-only blitz costs nothing; EotS charges a step.
- Batteries (AS 90, execution fire) out-shoot a battleship; VBAM models
  fixed defenses as a siege cost, not a fleet-killer.
- A beaten assault fleet holds in enemy orbit at 20x visibility; EotS
  forces post-battle movement out.
- Auxiliaries are untouchable while one destroyer lives; VBAM's screen is
  conditional, and raiding transports is a tactic.
- Surprise is unused: the engine has the richest detection model of the
  four and spends it only on whether a fight starts.

### 9. The spec's stale text

"ECMAINT formulas remain unrecovered" (they are decoded); the hostile
retry (removed by t036); nothing on when a strike fires.

## 2.5D as the theater model

The depth axis does most of ec4x's theater work for free:

| ec4x theater | On 2.5D |
| --- | --- |
| Space combat | layer 0 of the target sector; Patrol and Scout Sector live here |
| Orbital combat | layer 1: the starbase, Guard Starbase and Guard/Blockade fleets, and fleets at rest in orbit; an attacker enters it as depth passes 0.5, over `26/speed` ticks |
| Planetary combat | the strike, when depth reaches 1.0 |

What it makes natural: no theater skipping (a descending fleet passes both
layers in order); retreat as a climb-out that breaks the pairing; pursuit
as classic's intercept during the climb window; blockade as sitting in layer
1; the awareness reset on a layer change as ec4x's fresh detection check at
orbit; a lost world's base lifting instead of dying. What it makes awkward:
task forces across ticks and layers, and sighting as a leaky gate for
orders that are not assaults.

## Proposal

Keep classic's loop untouched: ticks, sighting odds, awareness, the
decision rules, the intercept roll, the strike tick, the lift. Change only
what happens once classic has decided that a fight starts (ec4x review 4.3):

1. **Sides at the tick.** The observer plus every fleet of its empire in
   the sector and layer with ROE > 0, against the target plus every fleet
   and starbase of its empire there. Each fleet keeps its own ROE.
2. **Theater by layer and ownership.** Layer 0, or layer 1 at a world
   neither side owns: space rules. Layer 1 at an owned world: orbital
   rules for the owner (starbase column shift while the base is undamaged;
   a side with a live base never withdraws).
3. **Rounds.** Simultaneous; flat CER with column shifts for qualitative
   edges; cripple-first; criticals on the highest-DS unit in the line;
   cascading overkill above 1.5x the enemy's DS; ROE per fleet after every
   round from round 1; no 3-round lock; end at no combat line, all
   withdrawn, or 64 rounds.
4. **Withdrawal = climb.** Seek Home at max speed, ROE unchanged; the
   victor pursues during the climb with the intercept rule; crossing 0.5
   ends the chase.
5. **Strike tick gate.** Before any assault, fight the orbital battle
   against everything of the owner's in layer 1, sighted or not. Only the
   field holder assaults; a beaten attacker climbs out.
6. **Assault.** Bombard: three rounds. Invade: the same three rounds, land
   only at zero batteries. Blitz as now. Ground: defender +1, +1 more at a
   home world; mutual annihilation to the defender. A beaten assault fleet
   leaves orbit.
7. **Persisted damage.** Crippled counts per class in the typed state;
   repair at an owned world for 25% of build cost.

Either keep classic's two-sided battles and apply the theaters only at the
strike gate (the smaller change, no new ADR), or join sides as in step 1
(supersedes the "exactly two sides" clause of the engagement ADR).

## Consultation with Codex (2026-10-10)

Codex reviewed this note independently
([rolr9ti0](rolr9ti0-independent-nc-combat-balance-analysis.md), with
Python models in [tools/combat-model](../../../tools/combat-model/README.md)). Its four code findings were
verified against main: strikes resolve at year end, not in their tick
(contrary to the combat ADR); survivors keep a dead hull's speed; retreat
targets deep space over the home sector by Manhattan distance; the mixed
bonus fires on any two hull types. It corrected this note in four places,
accepted here: destroyers do not dominate at equal cost; "joined vs two
sides" is a false choice (two empires per battle, each side joined, with a
willingness test); criticals should focus an existing damage budget, not
grant a free kill; the flat table should start at EotS's 0.625 mean. Its
strongest new finding is the ground-combat cliff: against ten defenders,
ten attackers capture 8% of the time and eleven 83%.

Agreed with Codex:

- Mechanical fixes, classic's domain or plain bugs, to do now (t052-t054):
  strikes in their eligible tick on top of t048; survivor speed recomputed
  after losses; retreat to the nearest owned world by the classic
  travel-distance picker, into orbit, with the climb as the first leg and
  ROE unchanged; then the deterministic orbital gate for the owner's base.
- Rule decisions, candidates until the user selects them: per-fleet ROE
  after one compulsory exchange with a bounded pursuit episode (one
  exchange per opposing empire, ended by leaving the sector or layer);
  persistent crippled counts with repair at 25% of build cost at an owned
  world, at rest in orbit, from a maintenance-start snapshot; a flat
  effectiveness table with three remainder variants compared in a sweep
  (discard, residual roll, carry forward); criticals as budget focus;
  dropping the mixed bonus; a conditional auxiliary screen; batteries
  before docked assets; a separate ground rule (defender-first with one
  effectiveness roll per side, or simultaneous per-army fire), compared
  across scale; chronological assaults instead of class priority and the
  annual ownership cap; joined sides with a willingness test and
  proportional loss allocation; surprise as a first-exchange +1 from a
  persisted alert state; DD 10/8 at cost 5 as the first hull playtest.
- Tooling: a combat-odds sweep with AS/DS/cost tables as explicit inputs
  and fresh combat seeds per trial (ScenarioPlay passes seed 0 today), plus
  exact enumeration for d10 pairs and ground combat; odds tables for the
  manual's Appendix B (t055).

The sequence: mechanical fixes, then persistence in t039's typed model,
then the sweep-driven resolution choice, then participation and surprise
with an ADR that supersedes the two-sides clause of the engagement ADR.

## Decisions for the user

The candidates above, grouped for a decision after the sweep (t055) has
numbers: (1) resolution table and remainder rule; (2) ROE, withdrawal, and
pursuit; (3) persistence and repair; (4) ground combat rule; (5) assault
ordering (chronological, no class priority, no annual cap) and the
batteries-first collateral order; (6) participation (joined sides) and
surprise; (7) hull values and prices, starting with DD 10/8 at cost 5.
Execution fire stays for DD and bases; batteries stay dangerous.

## Connections

- [NC combat stays](../decisions/b5u6lekb-rust-keeps-its-own-combat-system.md)
- [Engagement follows the classic encounter rule](../decisions/gtkmxw3l-engagement-follows-the-classic-ecmaint-encounter-rule.md)
- [Typed game state plan](../plans/7h5r6ne8-typed-game-state-in-rust-and-sqlite.md):
  crippled counts belong in the typed fleet.
- [ec-movement-spec.md](../../dev/ec-movement-spec.md): the depth axis.
- [Codex's independent analysis](rolr9ti0-independent-nc-combat-balance-analysis.md)
  and the models in [tools/combat-model](../../../tools/combat-model/README.md).
- [The t055 sweep review](qjev5wpq-review-of-the-t055-combat-sweep-semantics.md):
  solver defects, experimental assumptions, and mission-survival findings.
