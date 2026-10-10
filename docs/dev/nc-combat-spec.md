# Nostrian Conquest Canonical Combat Spec

This document defines the canonical Rust combat model for Nostrian Conquest.
While the 1992 `ECMAINT.EXE` internal formulas remain unrecovered, this spec
translates the documented manuals into an auditable, seeded, and reproducible
rule set.

The intent is to preserve the spirit of the original player manuals while
adopting a clean simultaneous-resolution structure inspired by *Empire of the
Sun*. Forces meet, strike at the same instant, and survivors move forward.

Read this together with:
- [ec-encounter-spec.md](ec-encounter-spec.md) (who meets and attacks whom)
- [rust-turn-cycle-implementation.md](rust-turn-cycle-implementation.md)
- [ec-turn-cycle-spec.md](ec-turn-cycle-spec.md)
- [ec-timing-spec.md](ec-timing-spec.md)

## Status

Approved change, implementation pending (2026-10-10): [combat decision 1](../notes/decisions/ld2f252j-flat-combat-effectiveness-with-battle-local-carry.md) selects EotS 25/50/100% effectiveness without ratio columns, battle-local carry per side and fire pool, and full DS for finishing crippled ships. The ratio CRT below describes the existing implementation until the engine and player manual are updated together. Other combat decisions remain separate.

This is the source of truth for:
- Fleet-vs-fleet battle resolution (who fights whom follows classic ECMAINT;
  see Contact and Engagement)
- Bombardment, Invasion, and Blitz
- Retreat and ROE interaction

It does not replace the 1992 manuals as historical sources. It defines the Rust
rule set used for maintenance runs. The turn-cycle specs decide where these
mechanics sit in the yearly loop.

## Source Basis

Primary 1992 manual references:
- [original/v1.5/ECPLAYER.DOC](../../original/v1.5/ECPLAYER.DOC)
- [original/v1.5/ECQSTART.DOC](../../original/v1.5/ECQSTART.DOC)
- [docs/ecmaint-combat-reference.md](ecmaint-combat-reference.md)

Manual-facing requirements:
- Combat fleets are governed by ROE thresholds (**0–10**).
- Bombardment damages production, stardock assets, and defenses.
- Invasion follows a three-stage attack:
  1. Destroy ground batteries.
  2. Soften resistance with orbital fire.
  3. Land armies to eliminate survivors.
- Blitz is a risky landing favoring overwhelming force.
- Starbases have more firepower than a battleship and absorb more hits.

## Design Goals

The canonical EC combat model:
- Preserves manual concepts: ROE, Bombard, Invade, Blitz, and Starbase defense.
- Uses a seeded campaign RNG for reproducibility.
- Produces plausible mutual attrition, not just one-sided wipeouts.
- Resolves simultaneous arrivals deterministically.

## Core Principles

### 1. Simultaneous Resolution
Fleet combat, orbital fire, and ground battles resolve at the same instant.

### 2. ROE Thresholds
Rules of Engagement decide whether a fleet attacks (following classic
ECMAINT; see Contact and Engagement) and, once a battle starts, when it breaks
off. Inside the battle ROE does not inject randomness.

#### 3-Round Commitment
Once combat begins, all participating fleets are committed to the engagement
for a minimum of **three rounds**. ROE-based withdrawals and retreats are
disabled until **Round 4**. This ensures that fleets trade meaningful blows
and prevents "bailing" before any attrition occurs.

### 3. Aggregate Combat
EC uses ship counts, not per-hull damage state. The Rust engine uses a
**Nominal -> Crippled -> Destroyed** step-loss model during a battle. Only
destroyed hulls are written back to the save file.

### 4. Defender Wins Ties
If a tie occurs and no side has a clear edge, the defender wins.

### 5. Combined Arms
The model rewards mixed fleets (**DD/CA/BB**) with a small effectiveness bonus
over mono-type swarms.

## Combat Actors

The Rust engine uses a **10x internal scale** for combat values to ensure high-precision damage resolution and to prevent crippled light ships from rounding to zero effectiveness.

### Fleet and Orbital Units

| Unit | AS (Attack) | DS (Defense) | Notes |
| ---- | -- | -- | ----- |
| Destroyer | 10 | 5 | Agile glass cannon (Execution) |
| Cruiser | 30 | 30 | Balanced brawler (Suppression) |
| Battleship | 90 | 100 | Fleet anchor (Suppression) |
| Scout | 0 | 10 | Non-combatant |
| Troop Transport | 0 | 10 | Vulnerable landing craft |
| ETAC | 0 | 20 | Colony ship |
| Starbase | 100 | 120 | Orbital fortress (Execution) |

### Ground and Planetary Defenses

| Unit | AS | DS | Notes |
| ---- | -- | -- | ----- |
| Ground Battery | 90 | 20 | Anti-orbital cannon (Execution) |
| Army | 10 | 10 | Surface combatant |

## Contact and Engagement

Who meets and who attacks follows classic ECMAINT's encounter loop exactly,
rules and odds; [ec-encounter-spec.md](ec-encounter-spec.md) has the full
rule and its constants, and the
[engagement ADR](../notes/decisions/gtkmxw3l-engagement-follows-the-classic-ecmaint-encounter-rule.md)
the decision. In short:

- The year is 52 ticks. Each tick every fleet and starbase, in a random order
  drawn once a year, moves one step, scans the alien units in its sector and
  depth layer, and runs its order's arrival test. Units in different layers
  (deep space, the solar system) never meet.
- Each scan rolls a sighting chance per unit: `10 * awareness + visibility *
  sensor factor`. Awareness (0 unknown, 1 contact, 2 identified, 3 fought) is
  shared by the observer's whole empire and forgotten when the target changes
  sector or layer.
- An identified fleet is attacked when the observer's empire declared its
  owner an enemy, when it is in one of the observer's systems, when a
  blockade meets an assault or a colonizer of an unowned world, when two
  fleets race to colonize, or when it guards an assault's target world (the
  assault aborts if its ROE forbids the fight). First identification commits
  any ROE above 0; later re-checks use classic's strength test. Then an
  intercept roll decides whether the target gets away.
- Starbases scan and report but never start a fight. A fleet attacks a
  starbase only as a declared enemy or when the base protects its assault
  target.
- A battle has exactly two sides: the attacking fleet, and the target with
  its guard partner (a guarded starbase, or a starbase's guard fleet). The
  attacker loses that tick's movement. Battles leave stored diplomacy alone.
- There is no departure strike: a blockade catches a fleet leaving its world
  only through the rules above, while the fleet is still in orbit.

Rust departs from classic in one place: a sector with no world is nobody's
system (classic reads a garbage owner there). Rust matches classic's odds, not
its random sequence.

### Who Takes Part
Only the two sides of an encounter fight. Only they can hold the field, be
forced to retreat, lose their mission, or receive a battle report. Other
fleets in the sector are untouched.

## Seeded CRT Resolution

Combat derives its results from the `campaign_seed` plus battle context. Each
exchange rolls a `d10` against the Combat Results Table (CRT).

### The CRT
The model selects a column based on the force ratio and applies modifiers:
- **Disadvantaged**: Ratio < 0.5
- **Pressed**: Ratio 0.5 – 1.0
- **Even**: Ratio 1.0 – 1.5
- **Advantaged**: Ratio 1.5 – 3.0
- **Overwhelming**: Ratio >= 3.0

Modifiers shift the column (e.g., **+1** for mixed fleets or starbases). An
unmodified **9** is a critical hit, forcing at least one real loss.

## Hit Allocation

### Tactical Roles and Split Fire
Hits generated by a fleet are divided proportionally based on the Attack Strength (AS) contribution of its ships into two pools:
1. **Suppression Fire (Dispersed):** Generated by **Cruisers** and **Battleships**. These hits spread across the enemy combat line, reducing nominal ships to crippled status before any ships are destroyed.
2. **Execution Fire (Focus Fire):** Generated by **Destroyers** and **Starbases** (and all Planetary Return Fire). These hits bypass the crippled state and allocate directly to the destroyed pool, paying the full `2x DS` cost to eliminate ships one by one.

This creates a deadly synergy where heavy ships suppress the enemy line, while light screens and fortresses execute them.

### Target Priority
1. Fire targets the combat line (**DD, CA, BB, SB**) first.
2. Auxiliaries (**SC, TT, ET**) are screened until the combat line collapses.
3. Target selection favors the lowest **DS** first.

### Bombardment Resolution (Per Turn)

Each bombardment turn resolves three sequential exchanges (rounds). Ground
batteries act as the planet's shield wall --- while they stand, they draw
orbital fire and shoot back, protecting armies, production, and industry.

1. **Round 1 (Suppression):** Attacker fires at stardock contents, then
   batteries. Batteries fire back. Armies, goods, and factories are shielded.
2. **Round 2 (Suppression):** Same targeting. Batteries fire back again with
   whatever survives round 1.
3. **Round 3 (Breakthrough or continued suppression):** If batteries reached
   zero before this round, attacker hits cascade into armies, stored goods,
   and factories. If batteries still remain, this round is another suppression
   exchange --- batteries continue to shield everything behind them and fire
   back.

#### Planetary Hit Costs
Orbital hits are on the same 10x scale as fleet combat, so each target costs
its defense strength:

| Target | Hits per unit destroyed |
| ------ | ----------------------- |
| Ground battery | 20 |
| Army | 10 |
| Docked ship (stardock) | Its fleet DS (DD 5, CA 30, BB 100, SC 10, TT 10, ET 20, SB 120); docked batteries 20, armies 10 |
| Stored goods / factories | Remaining hits ÷ 10, then 100 goods and 1 factory per point |

Hits below one unit's cost do not destroy it, and partial hits do not carry
over between rounds. Hits too few to destroy the next docked item move on to
the following stardock slot. Invasion suppression and blitz cover fire pay the
same battery cost.

Blitz landing fire is one CRT exchange of surviving battery AS against the
escorts' bombardment AS. It is focus fire aimed at the descending transports
first: each transport destroyed costs 20 hits (2x DS) and takes its army with
it, and leftover hits fall on the escorts.

The attacker must commit enough firepower over enough turns to grind through
the battery shield before reaching anything valuable. A small raiding fleet
may take several turns of three-round suppression before breaking through,
while a heavy bombardment fleet can clear batteries and break through in the
same turn.

### Invasion Priority
1. **Orbital suppression:** Ships exchange fire with batteries (1 exchange).
2. **Softening fire:** If batteries cleared, orbital fire targets armies only.
   Factories and stored goods are not damaged during invasion --- the goal is
   to capture the planet with its production intact. The CRT column compares
   bombardment AS against the garrison's strength on the same scale (armies x
   army AS 10), and each army destroyed costs 10 hits. Softening may destroy at
   most half of the defender's starting armies.
3. **Landing battle:** Armies fight in repeated simultaneous rounds until one
   side is destroyed. The defender receives a **+1** column bonus in ground
   combat. Ground combat cannot end in a draw: if a simultaneous exchange
   would annihilate both sides, the larger force entering that exchange holds
   the field with a remnant army; exact equality favors the defender. If the
   64-round cap is reached with both sides standing, the larger remaining force
   holds the field, again with ties to the defender.

## Fleet Combat Sequence

1. **Identify Participants**: The two sides of the encounter, each a task
   force, ordered by empire number. The attacker side reports that it
   intercepted the enemy; the other side reports that it was attacked.
2. **Simultaneous Fire**: Both sides generate and apply hits (Rounds 1-3 mandatory).
   Withdrawals for a round are declared before any volley resolves: a force
   breaking off takes only the withdrawal exchange from the opponent it
   leaves, never that opponent's full volley as well.
3. **Post-Round ROE**: Survivors check if they can break off (Starting Round 4).
   A withdrawn force is out of the battle: it is no longer targeted and its
   strength no longer counts in anyone's ROE odds.
4. **End State**: Combat ends when one side is destroyed or retreats. A
   battle still undecided after 64 rounds ends as a stalemate: survivors keep
   their ships and the normal post-battle rules (retreat of forces that did
   not hold the field, mission aborts, reports) apply.
5. **Retreat**: Once the losses are on the fleet records, each surviving
   fleet's maximum speed is recomputed from the ships it has left, as
   classic's `24a3:036b` does. A fleet that did not hold the field, withdrew
   under its ROE, or lost the ships its mission needs then gets the Seek
   Home order as the order path writes it: its target is classic's nearest
   friendly world (`24a3:0876`, as every classic abort and retreat picks
   it), the owned world nearest by travel distance from the fleet's own
   position and depth, leaving out the world of the system the fleet is in
   ([ec-encounter-spec.md](ec-encounter-spec.md), section 4); the target is
   the world (TZ 1), so the fleet's first leg climbs out of the layer it
   fought in and its last leg descends into the home world's orbit; its
   speed is its new maximum. ROE is left alone. That is a mechanical
   matter, not a rule: classic zeroes ROE only on scatter, and ec4x never
   does, so a retreat is not where ROE changes. A fleet whose empire owns
   no other world holds where it is, position and depth untouched. A fleet
   with no ships left is culled.

Every collection whose order affects outcomes or event order is ordered, so
the same seed always produces the same battle. The battle's random draws are
keyed by the tick and the attacking unit, so two battles in one sector in one
year roll independently.

This battle is NC combat's, not classic's. Classic's battle is decoded in
[ec-battle-spec.md](ec-battle-spec.md) for reference only; see
[NC combat stays](../notes/decisions/b5u6lekb-rust-keeps-its-own-combat-system.md).
`fight_encounter` is the seam.


## Simultaneous Arrival at a Planet

A hostile fleet (**Bombard, Invade, Blitz**) strikes in the tick its strike
test passes ([ec-movement-spec.md](ec-movement-spec.md), Hostile Missions),
with the ships it has in that tick and against the world as it stands in
that tick. Each fleet's assault resolves alone, in the year's unit order:
fleets that reach the same world strike one after another, and two that
strike in the same tick go in the order the year drew for its units.
Nothing is pooled across fleets or ticks. A later attacker meets what the
earlier one left, including a new owner, so a world can change hands more
than once in a year.

Before the strike comes the orbital gate. Whatever the world's owner has in
orbit fights the attacker in the strike tick, whether or not either has
sighted the other: his starbases there and his fleets there with warships,
less any fleet already leaving on Seek Home. It is the ordinary fleet battle
above, with the attacker as the observer side and the owner's units as the
target side, so losses, re-rating, retreat, and the awareness both sides
gain (3, fought) are the battle's. The attacker's ROE decides first, as it
does for a starbase blocking its path: a fleet unwilling to take on the
owner's combined strength aborts to Seek Home without a shot. Only the
holder of the field strikes, in the same tick. A beaten attacker has
retreated with its battle, or died in it, and its mission report says the
orbital defence repelled it. A second attacker of the same empire finds the
orbit the first cleared empty and strikes without refighting anyone; rival
attackers of different empires strike in the year's unit order, each
against whatever holds the orbit when its turn comes. A transport-only
Blitz brings no warships and can fight nobody: it lands only when the owner
has nothing in orbit, and a fleet of his without warships does not stop it.

Classic runs this gate for the owner's starbase alone (`110a:67bd`),
sighted or not, and leaves his fleets to the encounter loop. NC extends the
mandatory fight to the owner's fleets in orbit. That is an NC policy
choice: a defended orbit cannot be assaulted past by an attacker that never
rolled a sighting.

Two earlier rules of this spec were NC policy, not classic behaviour, and
are withdrawn: one assault class per planet per turn, Blitz before Invade
before Bombard, and a world changing hands at most once per turn. Both
belonged to a year-end assault phase that pooled the year's strikes; with
each strike resolved in its own tick there is nothing to rank or cap.

## Summary

This model provides a clear, auditable alternative to the 1992 engine's opaque
RNG. It preserves the classic mechanics while ensuring outcomes are
reproducible and fair.
