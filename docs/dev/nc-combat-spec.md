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
Fleet combat and orbital fire resolve at the same instant. The landing battle
is the exception: the defenders fire first.

### 2. ROE Thresholds
Rules of Engagement decide whether a fleet attacks (see Contact and
Engagement) and, once a battle starts, when it breaks off. Both decisions
weigh the same yardstick, NC attack strength, and both weigh the sides as
they would fight: an empire's fleets in the sector and layer together, not
the one fleet alone (combat decision 6). Inside the battle ROE does not
inject randomness.

#### One Compulsory Exchange
The first exchange of a battle is compulsory. After it, and after every
exchange that follows, each fleet checks its own ROE against the odds as
they now stand, its side's attack strength against the other side's, and
breaks off when they fail. A fleet whose ships have no attack strength left
attempts to escape at once, whatever its ROE, including a fleet that has
just lost its last armed ship. A fleet that breaks off may be pursued (see
Fleet Combat Sequence).

### 3. Aggregate Combat
EC uses ship counts, not per-hull records. The Rust engine uses a
**Nominal -> Crippled -> Destroyed** step-loss model. Crippled hulls stay
crippled after the battle, counted by class and by how long each still needs
to recover (see Damage That Lasts); destroyed hulls are gone.

### 4. Defenders Fire First on the Ground
In the landing battle the defenders fire before the attackers. That is the
defender's whole advantage: there is no defender modifier, no homeworld
bonus, and no tie rule, because the attackers fire only with what survives
and both sides cannot be destroyed.

### 5. Combined Arms
The model rewards mixed fleets (**DD/CA/BB**) with a small effectiveness bonus
over mono-type swarms.

## Combat Actors

The Rust engine uses a **10x internal scale** for combat values to ensure high-precision damage resolution and to prevent crippled light ships from rounding to zero effectiveness.

### Fleet and Orbital Units

| Unit | AS (Attack) | DS (Defense) | Notes |
| ---- | -- | -- | ----- |
| Destroyer | 10 | 8 | Fast raider and line support (Execution) |
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
  assault aborts if its ROE forbids the fight). Every attack passes the ROE
  test on the sides as they would fight, at first identification as at
  every later re-check: classic's first-identification bypass, under which
  any ROE above 0 attacked whatever the odds, is gone (combat decision 6).
  Then an intercept roll decides whether the target gets away.
- Every ROE test, the decision to attack, the in-battle check, and the
  orbital gate, weighs effective NC attack strength: each warship at its AS
  (destroyer 10, cruiser 30, battleship 90), a participating starbase once at
  100, and scouts, transports, and ETACs at 0. Die bonuses do not inflate it.
  Classic's 21/7/2 weights and its +30 for a guard at rest with its base are
  not used; the ROE thresholds, sighting, and intercept are classic's. A
  crippled hull counts at its crippled AS (half) and a crippled starbase at
  50, on every one of these paths.
- Starbases scan and report but never start a fight. A fleet attacks a
  starbase only as a declared enemy or when the base protects its assault
  target; in both cases it weighs the base's whole side, the base, its guard,
  and its owner's other fleets there (classic weighed a declared enemy's base
  alone).
- A battle is between two empires, and each side is its empire's fleets in
  the sector and layer (see Who Takes Part). The attacker loses that tick's
  movement. Battles leave stored diplomacy alone.
- There is no departure strike: a blockade catches a fleet leaving its world
  only through the rules above, while the fleet is still in orbit.

Rust departs from classic in three places. A sector with no world is nobody's
system (classic reads a garbage owner there). The strength a fleet weighs is
NC's, so classic decides who may meet whom and when, and NC decides whether a
fleet agrees to fight
([decision 2](../notes/decisions/kiswrzos-withdrawal-pursuit-and-one-strength-yardstick.md)).
And the sides are assembled and first contact obeys ROE
([decision 6](../notes/decisions/gzly58c5-fleets-of-one-empire-fight-together-and-first-contact-obeys-roe.md)):
encounter parity with classic holds up to those decisions. Rust matches
classic's odds, not its random sequence.

### Who Takes Part

A battle is between two empires. Each side is assembled from its empire's
units in the battle's sector and layer when the attack starts; nobody
arriving later joins, and a third empire never joins either side.

**The defenders.** The target's empire's fleets there form a fixed roster,
whatever their ROE: a fleet that is attacked fights the compulsory first
exchange, ROE 0 included. A fleet escaping from a battle there that the
attacking empire has already caught once is left out: it has got away from
that empire. Starbases never volunteer to attack; they defend only under the
target rule and the orbital gate: the base attacked, the base each guard
fleet at rest on the roster guards, and at the gate every base the world's
owner has in orbit.

**The attackers.** The observer's empire's fleets there volunteer under
their own ROE: each candidate has ROE above 0 and warships, and each tests
its ROE on the volunteers' assembled effective attack strength against the
defenders' roster; whoever refuses is dropped and the rest re-test until
none refuses, and a dropped fleet is not re-admitted. Two cruiser pairs at
ROE 6 that would each refuse four cruisers take them on together. The
observer's own decision is the same test: if admission removes the fleet
whose sighting triggered the encounter, the trigger is cancelled, its
sighting stands, and another willing fleet attacks only through its own
contact and intercept check. The intercept's fallback, an unnoticed target
choosing to fight, weighs the same assembled sides.

Only the two sides fight. Only they can hold the field, be forced to
retreat, lose their mission, or receive a battle report. Other fleets in the
sector are untouched.

### Surprise

Combat decision 6, surprise
([ADR atdsvb5y](../notes/decisions/atdsvb5y-surprise-uses-local-sighting-memory.md)).
An attack surprises a defending empire that has not sighted the attacking
one in the battle's sector and layer. Classic's awareness cannot carry that:
it is reset every year, so a standoff would hand a fresh surprise to
whoever sights first each year. Surprise therefore reads a memory of its own.

**Sighting memory.** Each empire keeps, for every sector and depth layer
where it has a fleet or a starbase, the set of opponent empires it has
sighted there. A successful sighting roll by any of its fleets or starbases
there adds the sighted unit's empire, whether or not the sighter went on to
identify, attack, or decline; a combat there adds the other empire to both
sides' sets (a fleet battle, and a planetary strike on a world another empire
holds). The set is shared by all of the empire's fleets and starbases in that
sector and layer. It is local readiness, not empire-wide knowledge: a scout's
sighting in another sector adds nothing here. A sighting by the fleet whose
trigger admission cancelled still counts, though its catch is not inherited.

**Lifetime.** The set is kept across ticks and years while the empire has a
fleet or starbase in that sector and layer, and cleared when that presence
ends: its last unit there moves to another sector or layer, is destroyed, or
leaves play (merged, salvaged). The encounter loop checks after every unit's
turn, so a gap within a year clears it even when another unit of the empire
arrives later the same year. The engine also settles the memory against the
records when a pass begins and after the pass's colonization and defections,
which covers what happens outside the ticks. An opponent leaving and
returning does not clear it.

**The bonus.** Before an attack's combat adds anything to anyone's memory,
the engine asks whether the defending empire's set there includes the
attacking empire. If not, the attacking side gets **+1** on its die in the
first exchange of the battle, an ordinary exchange. It counts within the
+2 cap on die modifiers (see Fleet Battle Effectiveness), leaves an
unmodified natural 9 the only critical, and never applies to a pursuit
exchange. Only the attacking side can earn it: a defender, hidden or not,
gets no counter-bonus, because surprise means catching the defending side
unprepared. It applies at the orbital gate like any other attack. Detection,
awareness, the ROE tests, and the intercept roll are unchanged. A battle's
combat then puts each empire in the other's set, so no later battle between
them there is a surprise while both stay.

**Storage and export.** The sets are typed state, `CoreGameData::sightings`,
with an SQLite table of their own, `snapshot_sightings` (viewer, sector,
layer, and the opponents as a bit mask, one row per place), saved and loaded
with the rest of the state; the campaign seed hashes them only when any
exist, so a game that has none keeps its seed. The classic files cannot
hold them: a classic export omits them, says how many places it left out,
and never alters the store, and a classic import starts with none.

**Reports.** The battle report says "We attacked by surprise." to the
attacker and "The enemy attacked by surprise." to the defender.

## Seeded Resolution

Combat derives its results from the `campaign_seed` plus battle context. Each
side's volley rolls a `d10`. Fleet battles and the landing battle read the
roll on a flat effectiveness table; planetary support fire reads it on the
ratio Combat Results Table (CRT).
[Combat decision 1](../notes/decisions/ld2f252j-flat-combat-effectiveness-with-battle-local-carry.md)
records the fleet rule and
[decision 4](../notes/decisions/59v787ut-defender-first-ground-combat-on-the-shared-table.md)
the landing battle.

### Fleet Battle Effectiveness

| Modified d10 | Effectiveness |
| --- | ---: |
| 0–2 | 25% |
| 3–5 | 50% |
| 6–9 | 100% |

A volley deals `hits = ceil(AS × effectiveness)`, where AS is the firing
side's total attack strength. The force ratio plays no part.

Die modifiers add to the roll:
- **+1** for a mixed fleet: at least two of destroyers, cruisers, and
  battleships.
- **+1** for a side with a starbase.
- **+1** for the attacking side's first exchange when it surprises the
  defending empire (see Surprise).

The total is at most **+2**, and a modified roll above 9 reads as 9. The
pursuit exchange is an ordinary exchange and takes the same modifiers
except surprise, which it never gets. An unmodified **9** is a critical
hit: it destroys one more unit outright, a crippled one first.

### Carried Hits

Each damage step costs the target's full DS: crippling a nominal ship, and
finishing a crippled one. Execution fire that destroys a nominal ship pays
both steps.

A volley's hits rarely buy a whole number of steps. What a fire pool cannot
spend stays carried against that side and joins the same pool's hits in the
side's next exchange of the battle, the pursuit exchange included.
Suppression and execution hits carry separately. Carried hits buy new steps
only; they never undo a cripple or a kill.

Carried hits belong to the battle and to the side, not to its fleets. They
are dropped when the battle ends for any reason: a side destroyed, a side
breaking off, or the 64-round cap. The next battle starts at zero, even
between the same fleets in the same tick: a pursuit on a later tick is a new
encounter, and a fleet that breaks off forfeits whatever was carried against
its enemy.

### Damage That Lasts

Combat decision 3 ([RT controls automatic ship recovery](../notes/decisions/p01cg7ll-rt-controls-automatic-ship-recovery.md)).
A hull that a battle cripples stays crippled until it recovers on its own.
Recovery needs no order, payment, shipyard, or owned world, and it goes on
while the fleet moves. A crippled hull fights at its crippled AS (half) and
changes nothing else: speed, DS, and mission ability are as before.

**RT, Recovery Turns, is 2 for every class**, starbases included. It is one
game-wide rule, not a stat of a ship class. A hull crippled in pass N stays
crippled through passes N+1 and N+2, recovers when pass N+2 ends, and fights
whole from N+3. The pass that cripples a hull does not count toward its
recovery. An encounter does not advance it.

**The counters.** Each fleet keeps, for every ship class, its crippled
hulls counted by turns remaining (1 to RT), and a separate count of the
hulls crippled during the current pass. A starbase keeps the same for its
one hull. The classic files have no field for them: a classic export writes
whole hulls and says the recovery state is lost, a classic import starts
with nothing crippled, and the campaign store keeps them
(`snapshot_fleet_crippled`, `snapshot_base_crippled`).

**No recovery inside a pass.** A hull crippled at tick 10 is still crippled
at tick 30 and in every later battle until it recovers. A battle's result is
written back to the fleets and starbases of each side with its damage: the
next battle, bombardment, or strike starts from it.

**The recovery step runs once per pass,** after the 52 ticks, the post-year
merges, and colonization, and before the AI and the reports. Every
turns-remaining count drops by one, hulls that reach zero are whole, and the
hulls crippled this pass enter at RT. No battle happens between the end of
one pass and the start of the next, so for combat this is also the start of
the next pass. Reports say how many ships recovered, and how many of them
came back a pass early with a starbase's help when any did. Players see only
nominal or crippled; no table shows a recovery time.

**Starbase assistance.** A hull recovers one extra turn early when it was
crippled before the pass began, belonged to the fleet when the ticks began,
and the fleet stayed at rest in orbit of the same owned world, beside one of
the owner's starbases that stayed there, through all 52 ticks. The engine
decides this from a snapshot taken when the ticks begin and dropped when the
pass ends, so it stores nothing. After every unit's turn it strikes off
a fleet that moved (even if it came back), lost its world, lost every base
of the snapshot, or changed hands, and it narrows the assisted hulls to
those the fleet still has. Hulls a loss removes are the unassisted ones
first; hulls that merge in or arrive during the pass never receive the
turn, and neither do hulls crippled this pass. A starbase does not assist
its own recovery. Assistance matters only for hulls with two turns left:
those with one recover at the end of the pass either way.

**Finishing and splitting.** Hit allocation still picks the class and
whether fire lands on nominal or crippled hulls. Within that choice, fire
that finishes a crippled hull takes the one with the most turns remaining,
and the hulls crippled this pass before any other, so a fleet's survivors
are the closest to recovery. A surviving crippled hull keeps its count.
Merging fleets adds the counts, this pass's included. Detaching or
transferring ships moves a plain quantity healthy hulls first, then crippled
ones only if the class has too few healthy hulls; `n!` moves exactly `n`
crippled hulls and is refused, with nothing changed, when the class has
fewer. Crippled hulls leave the donor in the same order as finishing fire
(this pass's first, then the most turns left) and take their counts with
them; nothing is reset or sped up. One rule, `resolve_selection`, picks the
hulls for both the move and the door's preview. Loaded and empty transports
keep separate counters, so a fleet's crippled transports are known by cargo
state. Loading an army takes healthy empty transports first and crippled
ones only when no healthy one is left, the nearest to recovery first;
unloading and landing empty the crippled loaded transports first, the most
turns left first, then the healthy ones. A cohort moves to the other
counter with its hull. Both orders are a default that awaits the owner's
confirmation, each kept in one helper in `nc-data/src/fleet_ships.rs`. A
fleet that changes owner would keep its counts; the engine has no path that
moves a fleet between empires (a civil-disorder defection removes the
fleet).

**Strength sees damage.** The attack decision, the intercept's fallback, the
orbital gate, and the in-battle ROE check weigh a crippled warship at half
its AS and a crippled starbase at 50. Bombardment and assault fire keep
their own weights (5, 30, 135 per hull) and their rounding for crippled
hulls: 2, 15, and 67. A crippled starbase keeps the growth and capacity
bonuses of its world and the +1 a starbase adds to the die.

### Landing Battle

The armies that land fight on the same table as fleets, one army against one
army, AS 10 and DS 10 each. A round has two volleys, the defenders' first:

1. The defenders roll once: unmodified `d10` 0-2 gives 25%, 3-5 gives 50%,
   6-9 gives 100% of their armies' AS. The attackers' losses come off.
2. The surviving attackers roll once and fire back the same way; the
   defenders' losses come off.
3. Rounds repeat until one side has no armies.

Damage is counted in quarter armies: a side with `n` armies deals `n × 1`,
`n × 2`, or `n × 4` quarters on a roll of 25%, 50%, or 100%, and four
quarters destroy one army. Damage short of an army carries, per side, into the
next round of the same landing, and is lost when the landing ends. Nothing
carries from one landing to the next. The quarters are exact, with no
rounding up.

There is no die modifier for either side, no homeworld bonus, and no critical.
The attackers fire only with what survives their opponents' volley, so both
sides cannot be destroyed and no tie rule is needed. Each round costs the
attackers at least a quarter army for every defending army, so a landing is
over within four rounds per landed army; it has no round cap. The surviving
attackers of a won landing become the new garrison.

Reports show whole armies lost and remaining. The carried quarters are not
reported.

Capture odds (the armies that land against the armies that defend, after any
softening; exact enumeration in `tools/combat-model/ground_calibration.py`,
checked against sampled engine landings):

| Attackers vs defenders | Capture |
| --- | ---: |
| 1 vs 1 | 33.6% |
| 2 vs 1 | 85.1% |
| 3 vs 2 | 55.6% |
| 3 vs 1 | 99.4% |
| 5 vs 5 | 22.6% |
| 10 vs 10 | 19.9% |
| 11 vs 10 | 27.6% |
| 15 vs 10 | 56.1% |
| 20 vs 10 | 91.8% |
| 20 vs 20 | 19.0% |
| 40 vs 20 | 92.3% |

Capture odds follow the force ratio and are roughly the same at every scale.
Overwhelming ratios still guarantee victory.

### Planetary Support Fire CRT

Bombardment, invasion suppression and return fire, softening, and blitz cover
and landing fire use the ratio CRT. The column comes from the force ratio:
- **Disadvantaged**: Ratio < 0.5
- **Pressed**: Ratio 0.5 – 1.0
- **Even**: Ratio 1.0 – 1.5
- **Advantaged**: Ratio 1.5 – 3.0
- **Overwhelming**: Ratio >= 3.0

A mixed fleet in bombardment and invasion suppression shifts the column by
**+1**. An unmodified **9** is a critical hit, forcing at least one real loss.
Planetary support hits do not carry (see Planetary Hit Costs).

## Hit Allocation

### Sharing Losses Among a Side's Fleets

A side fires and is fired on as one aggregate. Hit allocation chooses the
class, the damage state (nominal or crippled), the recovery cohort, and the
cargo subgroup it strikes; within that choice the losses are shared across
the side's fleets in proportion to their eligible hulls by largest remainder
(combat decision 6): fleets of 8 and 2 eligible cruisers share 3 losses as
2 and 1 (exact shares 2.4 and 0.6). Ties at the cutoff are broken by draws
from the battle's seeded roll stream, without replacement, only when a tied
group straddles the losses still to allocate; a completed allocation is
never redrawn, and roster order gives no fleet priority. Splitting or
merging fleets can shift a rounding loss by one hull per allocation.

Transport losses, crippling included, fall on loaded transports first across
the side, so moving cargo into another fleet of the side does not shelter it:
six loaded transports in one fleet and four empty in another lose four
loaded ones to four losses, as one fleet of ten would. Each fleet's loaded
and empty transports are separate holders with their own crippled counters,
so a hull crippled or finished is written to the counter of the cargo state
it was struck in; nothing is counted twice, and no hull moves between the
two in combat. A strike on a lone fleet shares its transport change the same
way. Starbases are members of the roster like fleets, one hull each, and the
base class is shared the same way.

The allocation happens after every exchange, before the ROE check, so a
fleet's decision to stay sees its own hulls. Carried hits belong to the
side, not to its fleets (see Carried Hits): a fleet that leaves takes none
with it and duplicates none.

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
orbital fire and shoot back, protecting the stardock, armies, production, and
industry.

**Batteries shield the stardock.** An exchange that begins with batteries
standing hits batteries only, and the hits beyond what destroys them expire.
Docked ships are exposed from the next exchange, the first one that begins
with no batteries standing
([combat decision 5](../notes/decisions/mk1ie4e3-batteries-shield-the-stardock-and-failed-landings-hold-orbit.md)).

1. **Round 1 (Suppression):** If batteries stand, the attacker's hits go to
   batteries only; otherwise they go to the stardock. Batteries fire back.
   Armies, goods, and factories are shielded.
2. **Round 2 (Suppression):** Same targeting by the batteries standing at the
   start of the round. Batteries fire back again with whatever survives
   round 1.
3. **Round 3 (Breakthrough or continued suppression):** If batteries reached
   zero before this round, attacker hits go to the stardock first and then
   cascade into armies, stored goods, and factories. If batteries still
   remain, this round is another suppression exchange --- batteries continue
   to shield everything behind them and fire back.

#### Planetary Hit Costs
Orbital hits are on the same 10x scale as fleet combat, so each target costs
its defense strength:

| Target | Hits per unit destroyed |
| ------ | ----------------------- |
| Ground battery | 20 |
| Army | 10 |
| Docked ship (stardock) | Its fleet DS (DD 8, CA 30, BB 100, SC 10, TT 10, ET 20, SB 120); docked batteries 20, armies 10 |
| Stored goods / factories | Remaining hits ÷ 10, then 25 goods and 1 factory per point |

Hits below one unit's cost do not destroy it, and partial hits do not carry
over between rounds. Once the stardock is exposed, hits too few to destroy the
next docked item move on to the following stardock slot. Invasion suppression
and blitz cover fire pay the same battery cost, and neither reaches the
stardock.

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
3. **Landing battle:** The armies fight the Landing Battle above, defenders
   first, until one side has no armies. Capture requires destroying every
   defending army; the surviving attackers become the new garrison.

Invade supplies its own softening once suppression has cleared the batteries,
so a separate Bombard order is not required first.

#### Capture
A world that falls passes to the new owner with what the fight left standing:
stored goods, production, docked ships, and queued build work. There is no
capture bonus, no loss by mission, and no forgiven debt. An Invade capture
leaves no batteries; a Blitz capture keeps the surviving ones. The status
resets to 0, the start of conversion.

A world whose owner had set Scorch Earth is halved as it falls
(`110a:6e6a..6f02`): production and positive goods are halved, the stardock
is emptied, the order is lifted, and production is raised to at least 1.0.
Debt is untouched, and a world with no potential stays at production 0. The
build queue is not burned. A failed assault leaves the order set.

The attacker's capture report lists the goods, docked ships, and queued work
the world passed on, and says when the defenders had scorched it.

#### Failure Is Not Defeat
A fleet whose landing fails, or whose Invade is blocked because batteries
survive suppression, keeps its survivors in orbit on Hold and files a failure
report. A blocked Invade lands nothing and its armies stay aboard. The fleet
does not blockade the world automatically and is subject to ordinary
encounters like any fleet in orbit. A fleet beaten in the orbital battle
leaves under the ordinary withdrawal rules (see Simultaneous Arrival at a
Planet), with no extra battery volley.

## Fleet Combat Sequence

1. **Identify Participants**: The two sides of the encounter (see Who Takes
   Part), each a task force whose roster is fixed when the battle starts,
   ordered by empire number. The attacker side reports that it intercepted
   the enemy; the other side reports that it was attacked.
2. **Compulsory Exchange**: Both sides fire one volley from the same moment,
   and the hits land together with what each side carried; the attacker's
   die gets surprise's +1 here when it applies (see Surprise). Each side's
   losses are then shared among its fleets (see Sharing Losses).
3. **Post-Exchange ROE**: After every exchange, the first included, both
   sides drop every fleet whose ROE fails the odds as they stand, its side's
   attack strength against the other side's, and every fleet whose ships
   have no attack strength left, all at once; the odds are recomputed and
   the rest re-check until none fails. A dropped fleet is out of the battle
   for good, even if the odds later favour it: it is no longer targeted and
   its strength no longer counts in anyone's odds. A starbase never leaves;
   the fleets beside it leave under their own ROE like any other. The
   carried hits stay with the side.
4. **Pursuit**: Fleets that break off while their side fights on leave
   unpursued; the enemy is still engaged. When a whole side breaks off and
   the other stays, the other may pursue each fleeing fleet. It needs armed
   ships (a starbase cannot chase), it must not have had its exchange with
   that fleet's escape already (see Pursuit and Escape), and it must catch
   the fleet by classic's intercept rule: a pursuer whose ships are faster
   catches for certain; otherwise the speed roll decides, `100 * sin(pi *
   u) * pursuer speed / fleeing speed` percent with `u` uniform, each at the
   speed of its slowest surviving class, one draw per fleeing fleet. The
   fleeing fleet is in contact and running, so the intercept's awareness
   term does not apply. The fleets caught stand for one more ordinary
   exchange, both sides firing with their die modifiers and carried hits.
   When both sides break off, nobody pursues. The battle then ends.
5. **End State**: Combat ends when one side is destroyed or breaks off. A
   battle still undecided after 64 exchanges ends unresolved: survivors keep
   their ships, their damage, and their orders, nobody holds the field,
   nobody retreats, no mission aborts, and an attacker at the orbital gate
   gains no permission to strike. The next tick's ordinary encounter rules decide whether they
   fight again; nothing remembers the capped battle.
6. **Retreat**: Once the losses are on the fleet records (each fleet's own
   share, as the battle allocated it), each surviving fleet's maximum speed
   is recomputed from the ships it has left, as classic's `24a3:036b` does.
   A fleet that did not hold the field, broke off under its ROE or unarmed,
   or lost the ships its mission needs then gets the Seek Home order as the
   order path writes it: its target is classic's nearest
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
   with no ships left is culled. The mission report of a fleet that broke
   off says the orbital defence repelled it when the world's owner held the
   field, or that it lost the ships its mission needs.

### Pursuit and Escape

A fleet that breaks off is escaping: from its retreat order on it is in an
escape episode bound to the sector and layer it is leaving. The episode ends
when the fleet actually leaves that sector or layer or receives a new order
(a player's order, a Join, or an order of ECMAINT's orders pass for a rogue
or autopilot empire), and it outlives the year: it
is runtime state on the fleet record and in the SQLite store (the
`escape_*` columns of `snapshot_fleets`), and a classic export, which has
no field for it, drops it. A fleet that holds in place because its empire
owns no other world is in no episode.

Each opposing empire gets at most one exchange with the escaping fleet while
the episode lasts: the pursuit exchange that caught it inside the battle, or
failing that its next attack on the fleet on a later tick, which is a new
encounter starting at zero carried hits and whose compulsory exchange is
that empire's pursuit, so the fleet, breaking off again, is not pursued. An
empire that has had its exchange cannot attack the fleet again there, and
the fleet does not join a defending roster against that empire: it has got
away from it. Pursuit stays per fleeing fleet and per opposing empire:
joining a side gives nobody a second pursuit. An escaping fleet attacks, or
volunteers, only when its ROE accepts the assembled odds, like any other.
Splitting or merging fleets gives nobody a second pursuit: a detached fleet
keeps its donor's episode, and a merge keeps every pursuer either fleet had
recorded.

Screening stays line-first during a pursuit exchange: auxiliaries are not
exposed because a retreat occurred. The free critical keeps its exception:
it destroys one more unit in the order DD, SC, TT, ETAC, CA, BB, SB,
ignoring the screen, so a critical can take an auxiliary while the line
still stands. That is the one way through the screen, documented here rather
than called an absolute screen; changing it needs its own approval.

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
above, with the attacker's side the striking fleet and its empire's other
fleets in orbit that volunteer under their own ROE (see Who Takes Part), and
the owner's units the target side, so losses, re-rating, retreat, and the
awareness both sides gain (3, fought) are the battle's. The attacker's ROE
decides first, as it does for a starbase blocking its path: a fleet
unwilling to take on the owner's combined attack strength (each starbase at
100) with its side's assembled strength aborts to Seek Home without a shot,
and so does a fleet without warships, whatever its ROE. Only the holder of the field strikes, in the same tick. A
beaten attacker has retreated with its battle, or died in it, and its
mission report says the orbital defence repelled it. A gate battle the
64-round cap leaves unresolved gives no permission: the attacker keeps its
order, and the next tick's gate decides again. The world's batteries take no part in the gate
and fire only on a fleet that strikes. A second attacker of the same empire
finds the orbit the first cleared empty and strikes without refighting
anyone; rival attackers of different empires strike in the year's unit
order, each against whatever holds the orbit when its turn comes. A transport-only
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
