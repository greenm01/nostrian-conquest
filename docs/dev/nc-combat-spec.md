# Nostrian Conquest Canonical Combat Spec

This document defines the canonical Rust combat model for Nostrian Conquest.
While the 1992 `ECMAINT.EXE` internal formulas remain unrecovered, this spec
translates the documented manuals into an auditable, seeded, and reproducible
rule set.

The intent is to preserve the spirit of the original player manuals while
adopting a clean simultaneous-resolution structure inspired by *Empire of the
Sun*. Forces meet, strike at the same instant, and survivors move forward.

Read this together with:
- [rust-turn-cycle-implementation.md](rust-turn-cycle-implementation.md)
- [ec-turn-cycle-spec.md](ec-turn-cycle-spec.md)
- [ec-timing-spec.md](ec-timing-spec.md)

## Status

This is the source of truth for:
- Fleet-vs-fleet combat
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
- Handles multi-empire battles without arbitrary pairwise ordering.
- Resolves simultaneous arrivals deterministically.

## Core Principles

### 1. Simultaneous Resolution
Fleet combat, orbital fire, and ground battles resolve at the same instant.

### 2. ROE Thresholds
Rules of Engagement gate his willingness to engage, his decision to break off,
and his guard posture. ROE does not inject randomness.

#### Pre-Combat Sensor Check
Before combat rounds begin, each task force performs a sensor sweep. If a fleet's
ROE threshold is not met against the detected enemy force, it will abort the
engagement and seek home immediately. This "clean retreat" happens before any
fire is exchanged and avoids the damage of a withdrawal exchange.

*Note: Sensor checks do not trigger for fleets forced into engagement (e.g.,
defended system entry) or for patrols, which exist to intercept.*

Guards, incumbents (a planet owner's forces at its own world), and starbases
are anchored to their system. When their ROE threshold is not met against a
contact that is *not* entering their system, they **hold formation**: they
neither sortie nor retreat, keep their orders, and fire only if a willing
enemy attacks them. A guard with a high enough ROE breaks formation and
intercepts. Against an intruder the engagement is forced and they always
fight.

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

## Contact and Hostility

The model treats contact as a single contested event from a shared board state.

### Enemy vs. Hostile
- **Enemy**: A stored diplomatic stance set in the client.
- **Hostile**: A tactical state triggered by ROE, intrusions, or attacks.

Fleets attack declared enemies. They also attack when a player enters one of
his solar systems or tries to enter/leave a world he is blockading. Initiating
an attack escalates diplomacy to **Enemy** automatically.

### Engagement Rule
The 1992 player guide states it directly: "Your fleets may attack if they
encounter the fleet of a player you've declared to be an enemy... Your fleets
will also attack if another player's fleets enter one of your solar systems,
or if they try to enter or leave a planet that your fleet is blockading."

- **Declared enemies** (either direction) are intercepted wherever they meet,
  in deep space or at a guarded system, gated by ROE as normal (sensor check,
  hold formation, round-4 withdrawal). A guarded system does not let an enemy
  pass unchallenged.
- **System intrusion.** Any other player's fleet whose own order takes it
  *into* the solar system at its location is hostile: Guard Starbase,
  Guard/Blockade World, Bombard, Invade, Blitz, and Scout Solar System. This
  applies against the world's owner, and against any empire blockading that
  world. Intrusion forces the engagement (no sensor check), whether the
  intruder is a declared enemy or not.
- **View World is not an intrusion.** It is a long-range scan from the system
  edge; the fleet then backs off into deep space.
- **Deep space is neutral ground.** A non-enemy fleet in the sector's deep
  space (Move, Patrol, Scout Sector, Hold, Join, Rendezvous, Seek Home, View
  World, and so on) is never attacked, even in a sector with an owned world,
  a starbase, or guards. Contact is still reported.
- Initiating an attack escalates diplomacy to **Enemy** automatically.

### Who Takes Part
Only forces that actually exchange fire are combatants. Only combatants can
hold the field, be forced to retreat, lose their mission, or receive a battle
report, and a report names only the hostile combatants it fought. A neutral
force that happens to share the sector is a bystander and is untouched.

**Unarmed contact does not abort missions.** If no side can fire (for
example, an enemy scout and an enemy transport fleet meeting in deep space),
nobody retreats and every mission continues; contact reports still emit. A
non-combat fleet attacked by an *armed* enemy still flees as ROE 0.

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
   the field with a remnant army; exact equality favors the defender.

## Fleet Combat Sequence

1. **Identify Participants**: Combine all participating fleets into task forces,
   one per empire, ordered by empire number.
2. **Pre-Combat Sensor Check**: Fleets failing ROE attempt to seek home safely;
   anchored defenders hold formation instead. Fleets are evaluated by ROE
   ascending, equal ROE by empire number.
3. **Simultaneous Fire**: Both sides generate and apply hits (Rounds 1-3 mandatory).
   Withdrawals for a round are declared before any volley resolves: a force
   breaking off takes only the withdrawal exchange from the opponent it
   leaves, never that opponent's full volley as well.
4. **Post-Round ROE**: Survivors check if they can break off (Starting Round 4).
   A withdrawn force is out of the battle: it is no longer targeted and its
   strength no longer counts in anyone's ROE odds.
5. **End State**: Combat ends when one side is destroyed or retreats. A
   battle still undecided after 64 rounds ends as a stalemate: survivors keep
   their ships and the normal post-battle rules (retreat of forces that did
   not hold the field, mission aborts, reports) apply.

Every collection whose order affects outcomes or event order is ordered, so
the same seed always produces the same battle.


## Simultaneous Arrival at a Planet

No empire executes an assault (**Bombard, Invade, Blitz**) until it achieves
orbital supremacy. If multiple hostile empires arrive at once, they must
contest the orbit before attacking the world. A planet can change ownership at
most once per turn.

One assault class executes per planet per turn, in priority Blitz > Invade >
Bombard. Only the orbit holder's fleets whose own order is that class take
part (fire, land armies, reset orders). Its fleets on other assault orders keep
their orders, loaded armies, and arrival state, and execute on a later turn.

Blitz has one narrow exception: a fleet of loaded `TT*` may still execute a
Blitz without escorting combat ships. That transport-only force does not
generate orbital firepower and may proceed only if no enemy combat force or
defending starbase holds orbit. Enemy non-combat support fleets alone do not
block the attempt.

## Summary

This model provides a clear, auditable alternative to the 1992 engine's opaque
RNG. It preserves the classic mechanics while ensuring outcomes are
reproducible and fair.
