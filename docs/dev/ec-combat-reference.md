# EC Combat Reference

Classic Esterian Conquest v1.5 combat, as `ECMAINT.EXE` runs it, in one
place. This is a record of classic, not a contract for Rust: NC keeps its own
combat system ([NC combat stays](../notes/decisions/b5u6lekb-rust-keeps-its-own-combat-system.md)).
The detail lives in two specs, decoded from the unpacked binary and checked
against thousands of classic runs:
[ec-encounter-spec.md](ec-encounter-spec.md) (who meets whom, and when they
fight) and [ec-battle-spec.md](ec-battle-spec.md) (the fights, and what they
leave behind). This page summarises them, points into them, and ends with
how NC combat differs. [ecmaint-combat-reference.md](ecmaint-combat-reference.md)
holds the black-box notes that came before the decode.

## How classic combat works

**The year.** ECMAINT runs a year as 52 ticks. Before the ticks it runs the
economy and the build queue. Each tick, every fleet and starbase, in a random
order drawn once a year, moves one step, scans its sector for alien units,
and then runs its order's arrival test. Units meet only in the same sector
and the same depth layer: deep space, or the solar system. IPBMs are units
too, but they never scan, never fight, and do nothing on arrival.

**Sighting.** A scan rolls once per alien unit in the sector. The chance is
`10 × awareness + visibility × sensor factor` percent. Visibility grows with
hull count, with speed, and with being in orbit. Patrol, Scout, and Guard
fleets at their posts, and starbases, have stronger sensors. Awareness (0
unknown, 1 contact, 2 identified, 3 fought) belongs to the whole empire: once
any unit of an empire identifies a target, every unit of that empire knows
it. It is forgotten when the target changes sector or layer. A fleet with
ROE 0 that is not on a Patrol, Scout, or Guard post only ever reaches contact
on its own.

**Decision.** An identified fleet is attacked when the observer's empire
declared its owner an enemy; when it sits in one of the observer's own
systems; when a blockade meets an assault fleet, or a colonizer of an unowned
world; when two colonizers race for the same world; or when it guards the
world the observer came to assault. At first identification any ROE above 0
commits. Later re-checks compare strengths against the ROE table: NC
weighs attack strength (DD 10, CA 30, BB 90, a starbase 100), where classic
weighed `21 BB + 7 CA + 2 DD` plus 30 for a guarded base (combat
decision 2). An assault fleet whose ROE forbids
the fight aborts to Seek Home. A starbase is attacked only as a declared
enemy or when it protects an assault's target world; starbases never start a
fight.

**Intercept.** An observer whose maximum speed exceeds the target's catches
it every time. Otherwise the target empire's awareness of the observer
decides whether the observer gets to try at all, with `100 - 33 × awareness`
percent: on that roll a target at speed 0 is caught, and a moving target is
caught on a sine draw scaled by the observer's maximum speed over the
target's speed. When the roll fails there is no catch, and the fight happens
only if the target's own ROE accepts the observer's strength. So a
stationary target is caught only by an observer with the higher maximum
speed, or on the awareness roll; at awareness 3 that roll passes 1% of the
time, and a target whose ROE declines is otherwise left alone.

**The battle.** Exactly two sides fight: the attacker, and the target with
its partner (a guarded starbase, or a base's guard fleet). Each ship,
battery, and base is a unit with hit points, a hit chance, a damage roll,
armor, range, and speed. Nothing persists past the fight: a survivor is
whole, there are no crippled ships and no repairs. The sides start
`5 + Random(20)` apart and close at their speeds; a shooter fires when its
range covers the distance. The larger side commits only part of its force
each round, about the enemy's value scaled up a little, heaviest ships first.
Each shooter picks a target, keeps it while it lives, and 30% of the time
joins a neighbour's target. After every round each side checks its ROE: it
may retreat (back off at its speed and keep shooting) or scatter (every ship
runs for itself, and escapes once it is clear of pursuit). The fight ends
when a side is wiped out, when neither side has warships, when a retreating
side gets out of range, when the distance passes 30, or when equal speeds
leave nothing in range. There is no round cap.

**Planetary assault.** Bombard, Invade, and Blitz act on the tick the fleet
reaches orbit. A starbase of the world's owner in orbit must be beaten
first, in the same tick. Bombard: a 10-round battle against the ground
batteries from distance 25, then collateral damage scaled by the damage
dealt and the batteries killed, then up to 20 bombing passes in which each
warship picks a target class at random (armies, factories, stored goods, the
stardock), weighted 10/20/30/40. Invade: the same battle; if any battery
survives there is no landing. Otherwise 12 softening passes (weighted
20/25/25/30), then the landing, then ground combat: the defenders shoot
first at 40-49%, the attackers at 30%, until one side is gone. Blitz: no
battle and no softening; the transports descend from 25 to 0 in steps of 5
under fire from every battery, land, and fight the same ground combat while
the fleet withdraws under fire. A battery picking a fresh target aims at a
transport 40% of the time and otherwise at any ship up to the last
transport, so transports draw `40% + 60% × transports / ships` of fresh
picks, and all of them when no warship escorts them. A captured world is
wrecked: an invasion keeps 20-40% of factories and 10-50% of goods and loses
its batteries; a blitz keeps most of both and a share of its batteries,
scaled by the square root of the surviving attackers over the defending
armies at the start of the assault, capped at 1; a world flagged
Scorch-Earth is halved again and its dock emptied.

**Aftermath.** A fleet that survives an assault goes to Hold with the ships
it has left, win or lose. A fleet destroyed in an assault is never saved
(bug B1): it comes back next year and attacks again. A fleet wiped out in a
fleet battle is deleted; a scattered fleet goes Seek Home with ROE 0; a
retreating assault, colonize, or guard fleet aborts to Seek Home, and any
other retreating fleet keeps its orders. A dead starbase is deleted. Fights
leave stored diplomacy unchanged, with one exception that follows assaults
only: after the capture step, if the world's owner is a civil-disorder
empire whose worlds stand below 75% of their potential, that empire becomes
the AI "Rogues", and its diplomacy is rewritten to enemy toward the attacker
and nothing toward anyone else. The test reads the owner after the capture,
so a captured world belongs to the attacker by then, and a capture never
converts the old owner. Every assault and battle updates DATABASE.DAT, the
per-empire intelligence file, and a per-run status table that later Scout
and View reports read.

**Reports.** Both sides get reports in RESULTS.DAT under "Stardate
tick/year": sighting and identification, the decision and the intercept, the
battle with its outcome sentence and losses, and for assaults the attacker's
mission report and the owner's "We have been bombarded / We have just
repelled / We have been invaded and conquered" report, with the damage in
percentages.

## Where the detail is

| Topic | Spec |
| --- | --- |
| Year and tick loop, unit table, awareness | encounter spec §1 |
| Sighting handler by observer and target kind | encounter spec §2 |
| Visibility and sensor formulas, measured odds | encounter spec §3 |
| Decision rules H, E, N; ROE table; starbase targets; intercept | encounter spec §4 |
| What happens when an attack starts; the lost tick of movement | encounter spec §5 |
| Blockade departures | encounter spec §6 |
| The random number generator | encounter spec §7 |
| Unit stats, unit order, engagement count | battle spec §1 |
| Assault arrival, starbase gate, orbital battle, collateral, passes | battle spec §2.1-2.6 |
| Invade, the blitz landing, ground combat, capture, Scorch-Earth | battle spec §2.7-2.9 |
| The fleet record after an assault; bug B1 | battle spec §2.10 |
| Rogues, the planet status table, DATABASE.DAT | battle spec §2.11 |
| Fleet battle setup, rounds, retreat, scatter, escape | battle spec §3.1-3.3 |
| Starbase fights; IPBMs | battle spec §3.4-3.5 |
| Report texts | battle spec §4 |
| Order and record effects | battle spec §5 |
| Probe results against the model | battle spec §6 |
| Classic bugs B1-B11 | battle spec §8 |
| What is still unchecked | battle spec §9 |

## How NC differs

NC combat is defined by [nc-combat-spec.md](nc-combat-spec.md) and the
player manual's combat chapters. The decision to keep it rather than port
classic's battles is [NC combat stays](../notes/decisions/b5u6lekb-rust-keeps-its-own-combat-system.md).
NC takes from classic the road to the first shot, and these contracts that
cross it:

- Survivor speed. Classic recomputes a survivor's maximum speed from the
  hulls it has left. NC does the same after every battle and strike
  (`distribute_fleet_losses`).
- Strike timing. Both strike in the tick the fleet reaches orbit, right
  after the orbital gate (below) in that tick.
- Intelligence. Classic gives each side in layer 1 at a world its empire
  does not own an owner-and-year update after a fleet battle, and the
  attacker and the old owner a detailed update after an assault. NC records
  planet intel events at the same points.
- Ownership consequences. Classic's capture, Rogues conversion,
  Scorch-Earth, and planet status table follow the fight. NC applies its
  own capture rules; Rogues, Scorch-Earth, and DATABASE.DAT are economy and
  intelligence rules outside this comparison.
- Aborts and the lost tick. A driven-off attacker goes Seek Home, a holding
  target stays, destroyed units are deleted, and the observer loses its
  movement for the tick. NC keeps these (encounter spec §5).

**Same as classic.** The tick loop, sighting, awareness, the decision rules
and the ROE table, the intercept roll, the two-empire battle, the lost tick of
movement, and unchanged diplomacy (classic's one exception, the Rogues
conversion, is an economy rule, below). Three differences by choice: a sector
with no world is nobody's system, where classic reads a garbage owner; the
strength a fleet weighs is NC attack strength (combat decision 2); and first
contact obeys ROE, with no first-identification bypass (combat decision 6,
[gzly58c5](../notes/decisions/gzly58c5-fleets-of-one-empire-fight-together-and-first-contact-obeys-roe.md),
item 4). Battles pair one fleet with one target as in classic
([bd8trz18](../notes/decisions/bd8trz18-battles-pair-one-fleet-with-one-target-as-classic-does.md)).
Rust matches classic's odds, not its random sequence.

**Different.**

- Units. NC gives each hull an attack and a defense strength on a 10x scale
  and a nominal, crippled, destroyed ladder. Classic gives hit points, hit
  chance, damage, armor, range, and speed, and nothing persists.
- Resolution. In a fleet battle each NC side rolls one d10 for 25, 50, or
  100% of its attack strength, whatever the force ratio, with +1 to the die
  for combined arms and +1 for a starbase, and a critical on 9. Hits a side
  cannot spend carry to its next exchange in the same battle. Planetary
  combat keeps a combat results table by force ratio. Classic rolls per
  shooter.
- Fire. NC splits suppression fire (cruisers, battleships) from execution
  fire (destroyers, starbases, batteries) and shoots the combat line first.
  Classic shooters pick one target and keep it; batteries only ever target
  warships, except in a blitz landing; and the larger side holds part of its
  force back each round.
- Rounds. NC fights one compulsory exchange, then lets every fleet check
  its own ROE after every exchange, shares a side's losses among its
  fleets, lets a side that stays pursue each fleet of one that breaks off
  when it catches it, and ends an undecided battle unresolved at 64
  exchanges.
  Classic checks ROE after every round from round 1, fights across a
  separation distance with weapon ranges, and has no cap.
- Retreat. NC's retreat aborts the mission. Classic's scatter sends the fleet
  home at ROE 0; its retreat aborts only assault, colonize, and guard
  fleets.
- Bombard. NC fires three rounds per turn behind a battery shield, with hit
  costs per target, and persists each year until the player changes the
  order. Classic runs one battle, collateral damage, and up to 20 random
  passes, then goes to Hold.
- Invade. NC runs one suppression exchange and a softening that hits armies
  only, at most half of them, with a defender column bonus and no draws.
  Classic runs the full battle, lands only when every battery is dead,
  softens factories, goods, and the dock as well, and fights the ground
  battle by count.
- Blitz. NC runs one cover-fire exchange of the escorts' bombardment
  strength against the batteries and removes the batteries it destroys,
  then one exchange of the surviving batteries against the fleet,
  transports first, then destroyers, cruisers, and battleships, then the
  ground battle; a capture keeps the batteries that survived. The owner's
  starbase blocks it. Classic fires no cover fire: the transports descend
  and withdraw under fire from every battery, and only the owner's starbase
  blocks it.
- Arrival. Both resolve each fleet in its own dispatch slot, in the tick it
  reaches orbit, so a world can change hands more than once in a year.
- The orbital gate. Classic makes the attacker beat the owner's starbase,
  with its guard, in the strike tick before it touches the world, sighted
  or not, and leaves the owner's other fleets to the encounter loop. NC runs
  the same gate (`holds_orbit` in `encounter/handler.rs`,
  [bd8trz18](../notes/decisions/bd8trz18-battles-pair-one-fleet-with-one-target-as-classic-does.md)): an attacker whose ROE refuses the
  fight seeks home without a shot, and only the holder of the field
  strikes. A transport-only blitz passes any orbit without a starbase.
- Failed attacks. NC resolves invade and blitz, and the fleet holds. Classic
  resurrects a destroyed assault fleet (B1) and attacks again next year.
- Damage. NC writes only destroyed hulls back. Classic has no damage state
  at all. The two agree here by different routes.

Rogues conversion, Scorch-Earth, and DATABASE.DAT are classic economy and
intelligence rules that run after a fight. They are outside NC combat and
this comparison.
