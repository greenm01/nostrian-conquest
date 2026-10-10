# VBAM 3rd Edition vs Nostrian Conquest combat

Source: *Victory by Any Means*, 3rd ed. v1.0.2 (VBM 3001, April 2025), read
from `~/Documents/VBAM-3rd-Edition/VBM 3001 Victory by Any Means V1.0.2.pdf`.
Page numbers below are the book's printed page numbers (the ones in its table
of contents); PDF page = printed page + 1. Section numbers (e.g. 4.6.2.5) are
the book's.

NC side: `docs/dev/nc-combat-spec.md`, the player manual's Combat Mechanics,
Planetary Combat, Missions, and Appendix B chapters
(`docs/manuals/nc_player_manual.typ` lines 397-660, 901-1080), and
`docs/dev/ec-encounter-spec.md` sections 1-5, and the 2.5D movement model in
`docs/dev/ec-movement-spec.md` ("Classic Movement Semantics"), which section 5
takes as given. Where the manual is vaguer than the engine, I checked
`rust/nc-engine/src/maint/combat/{exchange,state,retreat,fleet_battle}.rs` and
say so.

---------------------------------------------------------------------------

## 1. VBAM's combat system, precisely

### 1.1 Units and combat factors (pp. 11-13)

- Four military unit types: ships (jump-capable), fighters (no jump drive,
  carried), bases (fixed; "more powerful than ships of an equivalent size"),
  troops (p. 11, 2.3.1).
- Combat factors (2.3.3, p. 11): **DV** (Defense Value, "the number of hits
  that are required to damage the unit"), **AS** (Anti-Ship; also the
  bombardment stat), **AF** (Anti-Fighter; cannot hurt ships). Troops have DV
  and ATK only.
- **Command Rating (CR)**: how many units a ship can lead as task-force
  flagship. **Command Cost (CC)** ≈ CR/2, used by the Grand Fleets option and
  by capture (2.3.4, p. 12).
- **Crippled** (2.3.5, p. 12): a damaged military unit is crippled; all combat
  factors halve, rated special abilities halve, unrated ones stop working; a
  second damage destroys it and its cargo. Armored (DV not halved), Gunship
  (AS not halved), Carronade (AF not halved) are exceptions (p. 12-13).
- **Civilian units** (Convoys, Shipyards, Supply Depots) have factors but
  "cannot be crippled and are instead simply destroyed when they are damaged"
  and cannot command (2.4, p. 12).
- Scale check: a Human Prometheus BC is cost 10, DV 6, AS 4, AF 2, CR 7
  (p. 82); Raider Battlewagon BC DV 7 AS 7 (p. 32); an Independent Large Base
  BS2 is DV 9 AS 7 against their biggest ship, the War Cruiser at DV 5 AS 5
  (p. 73). A base is roughly 1.4-1.8x the best hull on the same list.

### 1.2 Fleets, encounters, and who may attack whom

- A **fleet** is an administrative formation for movement only: "a group of
  units located in the same system that are operating together as a single
  administrative unit for movement purposes" (4.2.1, p. 22; Basic Terms p. 6).
  Combat is never fought by fleets. It is fought by **task forces** built
  fresh from everything a power has in the system (1.4 below).
- An **encounter** "occurs when opposing powers have assets in the same system
  at the start of the Combat Phase" (4.6.1, p. 37), including a colony, base
  or troops. One encounter per system per turn; "All encounters occur
  simultaneously ... resolved in a random order ... Resolve each encounter
  completely before proceeding to the next" (p. 37).
- Encounters need not fight: "Combat actions are only resolved when at least
  one of the participants declares hostile intentions" (p. 37). Within a
  hostile encounter the order is fixed: all space combat scenarios, then
  orbital bombardment (only with orbital superiority), then ground combat
  (p. 37). Ownership changes take effect at the end of the encounter.
- **Political state gates attacks** (4.5.1, pp. 33-34): War = attack, bombard,
  invade anywhere; Hostilities = attack/bombard/invade only frontier worlds
  (Pop <= 4), may blockade anything; Neutral = may attack their forces in
  *neutral* systems, and "may attack any opposing Neutral fleet that
  accidentally strays ... into one of their own controlled systems" (p. 33);
  Non-Aggression or higher = cannot attack at all (p. 37). A **sneak attack**
  unilaterally declares war and gives the victim -4 on Surprise that turn
  (4.5.8, pp. 36-37).

### 1.3 Space combat scenarios, initiative, refusal (4.6.2, pp. 38-39)

Within an encounter, players "take turns declaring and resolving space combat
scenarios". Three scenario types:

- **Interception**: pitched battle, no bases on either side, no Surprise mods.
- **Defensive**: the Defender may include bases; one base is free of command
  cost; Attacker gets +1 Surprise.
- **Pursuit**: declared against a Defender that refused a scenario and is
  withdrawing. Defender: -1 Surprise, must fill non-flagship slots with its
  crippled ships, +2 to retreat rolls. Attacker: +1 Surprise, flagship CR
  halved, may not bring crippled units. Cannot be refused.

Initiative to declare scenarios goes "in descending order by their total
Anti-Ship factors", ties by total DV, then random (p. 38). A targeted
Defender may accept or refuse-and-withdraw; refusal invites a Pursuit. A
Defender with bases may refuse an Interception and demand a Defensive
instead (its fleet "is effectively retreating to the protection of its
bases"); the Attacker must then accept or stop attacking that power this
encounter (p. 38). A Defender with no jump-capable ships left cannot refuse a
Defensive scenario. A scenario "ends once one side or the other is eliminated
or successfully retreats" (p. 38). A power may pass without declaring
anything (p. 39). The worked example (p. 39): three powers in one system; the
one with the NAP cannot attack; the Attacker could run a Pursuit on the
fleeing fleet and *then* a Defensive on the bases left behind, "so it is in
the Human's best interests to stand and fight".

### 1.4 Task force setup, reinforcements, surprise, formation (pp. 39-41)

- **Flagship and command limit** (4.6.2.1, p. 39): flagship = highest-CR
  unit (crippled units may be skipped); task force = flagship + up to CR
  other units. Civilian units must be included but do not count, except an
  Attacker at Normal-or-better readiness may hold them in reserve. Bases only
  in Defensive scenarios; the Defender may also field Atmospheric fighters up
  to the system's Fortification value free ("sorties ... from planetary
  bases", p. 39). Carrier-based fighters do not count against the limit.
- **Allied task forces** (p. 40): only Mutual Defense or Alliance partners
  at war with the same enemy may combine; the flagship owner's traits apply.
- **Reinforcements Pool** (4.6.2.2, p. 40): everything over the limit waits
  out of the battle, contributes nothing, and is rotated in during the
  Recovery Phase. You may not under-fill the task force voluntarily.
- **Surprise** (4.6.2.3, p. 40): one d10 per side per scenario on the
  Surprise Table: 1- Bad (-2), 2-3 Poor (-1), 4-7 Normal (0), 8-9 Good (+1),
  10+ Superb (+2). Mods: +1 base in TF, +1 Scout or Fast unit, +1 Attacker in
  Defensive/Pursuit, -1 Defender in Pursuit, -4 sneak-attack victim. The
  result is a combat die modifier locked for the whole battle (p. 47).
- **Formation bonus** (4.6.2.4, pp. 40-41): one ship per TF (plus one per
  Guardian factor) gets DV +50%; not fighters, attack boats or bases.

### 1.5 The combat round (4.6.2.5, pp. 41-44)

Five steps per round:

1. **Assignments** (p. 41): formation bonuses; fighter missions (Anti-Ship,
   Anti-Fighter, Defense); **screening**: "If your task force includes more
   ships than the enemy task force you may move a number of units from your
   task force back into the Reinforcements Pool equal to the difference";
   Jammer/Disruptor targets; electronic warfare: the side with fewer Scout
   factors takes -1, the side with more Stealth units gets +1.
2. **Fire** (pp. 41-43), three sub-steps: Fighter vs Fighter, Ship vs Fighter,
   Ship vs Ship. Each sub-step: total the relevant factors, roll d10 on the
   **Combat Results Table** (p. 42): 1- 10%, 2-3 20%, 4-7 30%, 8-9 40%, 10+
   50% "combat effectiveness rating", add the task force's combat modifier
   (readiness, EW, stealth, empire trait, -1 for a new flagship), multiply by
   the AS total, **round up** (12 x 20% = 2.4 -> 3 hits, p. 49). Fire is
   simultaneous: in the example both sides fire with their pre-damage AS and
   losses apply afterwards (p. 49).
3. **Special Operations** (p. 43): capture with Assault ships (4.6.2.12,
   p. 45: roll <= total Assault factors to capture one unit destroyed this
   round; captured units are crippled with AS/AF 0 until repaired), ramming
   (optional 5.14, p. 80).
4. **Retreat** (p. 43): Defender first, then Attacker, every round. The
   opponent may contest: each rolls d10 + combat modifier; the retreater must
   roll **strictly higher**. Mods: +2 Defender in Pursuit, -2 if any Convoy
   is in the TF (not in the pool), +2 all-Fast TF / Fast Drive Systems trait.
   A successful retreat ends the scenario; failure means at least one more
   round. **Retreating fleets must immediately declare a movement order for
   next turn: one jump into an adjacent system, not a contested or
   enemy-controlled one "unless no other movement options are available";
   the order is public to the encounter so the enemy can pursue.** Bases
   cannot retreat; fighters only with their carriers; unbased fighters are
   left behind. Retreated forces are "disengaged and cannot attack or be
   attacked for the remainder of the encounter" (p. 43; also p. 38).
5. **Recovery** (pp. 43-44): if neither side is destroyed or retreated,
   rebuild the TF from the pool (rotate crippled ships out, fresh ships in);
   a new flagship costs -1 next round; civilians with no military units left
   surrender.

**Hit application** (p. 42): the *receiving* player assigns hits, one target
at a time, each costing its current DV; undamaged -> crippled, crippled ->
destroyed; civilians are destroyed outright. The receiver may over-pay onto a
higher-DV unit to protect a cheaper one. Leftover hits: if greater than half
the DV of the weakest target, that unit is damaged; otherwise lost. The firer
may instead spend hits as **Directed Damage** (4.6.2.6, p. 44): pick a target
at **2x its DV**, and may hit it again the same round at 2x its new (halved)
DV to destroy it. Disruptors halve a target's DV for the round (p. 45);
Jammers halve a target's AS/AF (p. 44). Fighters on Defense missions may
voluntarily absorb hits (p. 41).

**Stealth** (4.6.2.8, p. 44): a Stealth ship may withdraw before a scenario
on a d10+mod of 4+, and may roll separately in the Retreat Phase.

Worked example (pp. 47-53): Jain vs Human Defensive scenario, three rounds,
Jains retreat on a contested roll (7 vs 3) and declare a move to the adjacent
Andromeda system. Losses: Jains 24 EP + 2 EP repairs; Humans 16 EP + 8 EP
repairs. The after-action text stresses the base's value: "included at no
additional command cost, and it supplemented their firepower" (p. 53).

### 1.6 Orbital bombardment (4.6.3, pp. 53-55)

- **Orbital superiority** (4.6.3.1, p. 53): bombardment only after
  "eliminating all the defender's bases in the system". Fleets are not named
  as blockers; bases are.
- **Bombardment Value** (4.6.3.2, p. 53): BV = total AS of all attacking
  ships, fighters and bases; Bombardment ability doubles a unit's share; Out
  of Supply units contribute nothing (p. 54).
- **Missions** (4.6.3.3, p. 54), spend BV in any mix:
  - Fortification: -1 Fort costs 10 x current Fort.
  - Troop: kill one random ground unit for 5 x Population (min 10); on a d10
    of 1 per kill, -1 Pop and -1 Morale (collateral).
  - Infrastructure: 5 x Pop, -1 RAW next Economic Phase.
  - Population: -1 Pop, -1 Morale, -1 Intel for 10 x Pop.
  - All non-Fortification costs are **doubled while Fort > 0**.
- **Devastation** (4.6.3.4, p. 54): per Pop lost, roll d10; each roll below
  starting Carrying Capacity permanently reduces CC by 1; losing 2+ CC in a
  turn also permanently reduces RAW by 1.
- **Sustained bombardment** (4.6.3.5, p. 54): half of unspent BV carries to
  next turn's Combat Phase while superiority is held; lost if the fleet is
  destroyed or forced to retreat.
- Fortification never shoots at the orbiting fleet. Its combat effect is
  (a) the free Atmospheric fighter sortie in a Defensive scenario (p. 39),
  (b) doubling bombardment costs (p. 54), (c) blocking landings (p. 56).

### 1.7 Ground combat and invasion (4.6.4, pp. 55-57)

- **Procedure** (4.6.4.1, pp. 55-56): all ground combat in an encounter is
  simultaneous and is **one CRT exchange per turn**: total ATK, roll on the
  same CRT (10-50%), hits assigned by the receiver at DV each; leftovers
  below the weakest unit's DV are lost (no half-DV rule on the ground).
  Embarked troops fight at **half** factors (Marines at full); disembarked
  troops must fight; embarked troops may be held back; crippled troops
  cannot invade.
- **Invasions** (4.6.4.2, p. 56): a **beachhead** is secured when invading
  troops damage at least one defender and one invader survives, or the
  landing is unopposed. With a beachhead, more troops may land in later
  Movement Phases. A beachhead is lost if all disembarked friendly troops
  die or re-embark. **"Troops cannot disembark and land in an enemy
  controlled system if the system's Fortification value is greater than
  zero"** - the fleet must bombard Fort to 0 first.
- **Economic disruptions** (4.6.4.3, pp. 56-57): invader troops >= defender
  troops on the ground halves system output (quarter if in Opposition), -1
  Morale Check.
- **Conquest** (4.6.4.4, p. 57): when all defending disembarked troops are
  gone, Morale -> 0, control to the power with the most disembarked troops
  (ties: highest troop DV, then random); construction orders cancelled.
- Garrison aftermath (4.10.4, p. 69): systems with enemy units present must
  roll Morale Checks; +1 for a full garrison (troops >= Pop); Martial Law
  needs troops >= Pop; a 0-Morale system spawns rebel Militia (4.10.5,
  p. 70).

### 1.8 Repair, supply, blockade

- **Repairs** (4.8.8, p. 61): 25% of build cost, at a Shipyard (Atmospheric
  units at a planet), bases anywhere in supply, troops only disembarked on a
  friendly system; a unit under repair cannot move or fight that turn and is
  captured if its system falls.
- **Field repairs** (4.8.9, p. 62): Supply/Hospital units repair up to their
  rating in EP per turn, in supply only, partial repairs accumulate.
- With Extended Construction Times (5.15, p. 81) repairs still finish in one
  turn: "This incentivizes keeping crippled ships alive during a battle so
  they can return home for repairs."
- **Out of Supply** (4.7.6, p. 58): factors halved (crippled + OOS: 25%),
  no bombardment or field repairs; **attrition** (4.7.7, p. 58) cripples,
  then destroys, random units.
- **Blockade** (4.7.8, p. 59): automatic when enemy ships outnumber defending
  ships; output halved, trade zero, defenders trapped (Fast ships excepted),
  supply cannot be traced through.

### 1.9 Multi-faction battles

- N powers in one encounter; AS-order initiative; each declares scenarios
  against any non-NAP opponent; a third party at war with the same enemy
  "cannot intervene" unless Mutual Defense/Alliance (p. 39). Withdrawn forces
  are disengaged for the rest of the encounter (p. 38). A power may pass
  (p. 39).
- Deep Space Encounters (optional 5.9, pp. 78-79): fleets crossing a lane in
  opposite directions stop in the lane; Interceptions only; refusal sends you
  back where you came from.

### 1.10 Detection and intelligence

VBAM has **no sighting roll inside a system**: if opposing assets share a
system at the start of the Combat Phase, the encounter exists (p. 37). Hidden
information lives elsewhere: Long Range Scanning (4.4.7, p. 29: d10 6+ gives
only the *count* of units in an adjacent system; a natural 1 drags the scout
into the system), Espionage covert missions (p. 25: full attribute and unit
list), Contested Movement (4.4.8, p. 30: a fleet passing through an
enemy-held system must stop or leave behind as many ships as the enemy has;
Fast ships pass), Stealth withdrawal (p. 44), Q-Ships hidden in convoys
(p. 45). The Surprise roll (1.4) is VBAM's stand-in for "who was ready".

### 1.11 Rule-shaped NPC rules of engagement

These matter for NC because NC's engine decides every fight automatically.

- Independent Systems (5.1.5, p. 74): attack chance = 25% + 1% per EP of
  their ships/fighters in-system - 1% per EP of the intruder's; they refuse
  to pursue unless at War/Hostilities.
- Hostile Galaxy raiders (5.5, pp. 75-76): always attack, "the opponent with
  the lowest total Defense Value first", always pursue; never repair, "so
  they will get progressively weaker after each engagement".
- Ravagers (5.6, p. 76): demand Defensive if bases present else Interception;
  pursue only if their total EP >= the opponent's.
- Harbingers (5.7, pp. 76-77): attack everything, then Fortification
  Bombardment, then Population Bombardment until Pop 0.
- Raiders (4.4.10, p. 31): 3d10 EP on a natural 1; Convoy Raid order (4.4.11,
  p. 31-32): move in, fight one Interception against the convoy's escort with
  no reinforcements for either side, return to the origin system the same
  turn.

In every case the single yardstick for "should I fight / pursue" is
construction cost (EP) or DV, and it is the same measure before and during
the encounter.

---------------------------------------------------------------------------

## 2. What in NC traces to VBAM, and where NC deviates

| NC rule | VBAM source | Match / deviation |
| --- | --- | --- |
| AS/DS per unit; hits = ceil(total AS x multiplier) | AS/DV (2.3.3, p. 11); CRT % x AS, rounded up (p. 42, p. 49) | Same shape. NC adds a 10x scale and a ratio-dependent column (see 4.1). VBAM's multiplier is 10-50% of AS; NC's Even column averages 115% of AS. |
| d10 "CRT" | d10 Combat Results Table (p. 42) | Name and die match; VBAM has one column with roll modifiers, NC has five ratio columns with column shifts. |
| Nominal -> Crippled -> Destroyed; crippled AS halved | 2.3.5 p. 12 (all factors halve) | NC halves AS only (`state.rs::crippled_as`); finishing a crippled ship costs 1x DS (`exchange.rs`), where VBAM costs the halved DV (0.5x). NC therefore pays 2x DS to kill through suppression, VBAM 1.5x DV. |
| "All nominal ships must be crippled before any crippled ship is destroyed" | Receiver-assigns rule (p. 42) | VBAM leaves the choice to the receiver (who may let a crippled corvette die to save a cruiser, as in the example p. 51: "destroy 2 Sentry CT (3 hits each, 2 to cripple and a third to destroy)"). NC hard-codes the receiver's usual choice. |
| Execution fire: DD/SB/batteries destroy nominal ships at 2x DS, bypassing crippled | Directed Damage at 2x DV (4.6.2.6, p. 44) | Clear ancestor. VBAM gives directed damage to *any* firer as an option and a second 2x(DV/2) is needed to kill in the same round (3x total); NC fixes it by hull class and kills outright at 2x. |
| Leftover hits lost | p. 42 half-DV rule; p. 56 ground (lost) | NC drops VBAM's "more than half the weakest DV still damages it" rounding, so sub-threshold volleys do nothing. |
| Simultaneous fire | p. 49 example | Same. |
| Starbase AS 100/DS 120 > BB 90/100; +1 column when defending at home; base + guard fight together; bases never retreat | p. 11 bases > ships; Defensive scenario free base and +1 Surprise for a base (pp. 38-40); bases cannot retreat (p. 43); p. 53 after-action | Traces. VBAM's base edge is larger (1.4-1.8x the best hull). VBAM's Defensive scenario also lets Fortification sortie fighters for free (p. 39), which NC has no analog for. |
| Auxiliaries (SC/TT/ET) untouchable while a combat-line ship lives | Screening (p. 41), Attacker reserving civilians at Normal+ readiness (p. 39-40), Defense-mission fighters absorbing hits (p. 41) | VBAM's protection is conditional (numerical superiority or readiness) and the firer can still Direct Damage a convoy in the TF; convoys are destroyed, not crippled, when hit; and the Convoy Raid scenario exists specifically to get at them. NC's screen is absolute. |
| Mixed DD/CA/BB +1 column | Surprise/EW/trait modifiers (p. 40, 42, 44) | VBAM has no composition bonus. The closest is the Grand Fleets option making light hulls cheaper to field (p. 80). NC's shift is also larger than any single VBAM modifier (one column ~ +28% mean hits vs VBAM +1 on d10 ~ +10 points of effectiveness). |
| ROE thresholds as AS ratios (2:1, 3:2, ...) for break-off; classic strength test to start | No player ROE. NPC rules use EP or DV (1.11) | VBAM players decide; retreat is a contested die roll, not a ratio (p. 43). See 3.3. |
| 3-round commitment, withdrawal exchange at Pressed column | Contested retreat every round, Defender first (p. 43); Pursuit scenario (p. 38) | VBAM never locks a side in; it makes leaving uncertain (roll) and costly (Pursuit, -2 for convoys). See 3.1 and 4.3. |
| Three theaters: fleet -> orbit supremacy -> assault | Space combat -> orbital superiority -> bombardment -> ground, in that order within one encounter (p. 37, p. 53) | Traces. VBAM's superiority condition is "all bases eliminated"; NC requires no enemy combat fleet or starbase in orbit. |
| Bombardment damages batteries, stardock, armies, goods, factories; batteries shield everything until gone | Fortification Bombardment first at 10 x Fort; other missions doubled while Fort > 0 (p. 54) | Traces as "defenses first". Deviation: VBAM's Fort never fires back; NC batteries (AS 90, execution fire) are the most lethal unit in the game. VBAM's bombardment is a resource spend with carry-over (3.4); NC's is three CRT exchanges per turn with no carry-over. |
| Invasion: suppress batteries, soften armies (<= half), land, fight to annihilation, defender +1 column | Fort must be 0 to land (p. 56); embarked troops at half strength (p. 55); one exchange per turn; beachhead (p. 56) | NC resolves the whole ground war in one maintenance; VBAM runs it over turns with reinforcement from orbit. VBAM has no defender bonus and no cap on pre-landing troop kills (Troop Bombardment can take the garrison to zero, at collateral risk). |
| Blitz: land through surviving batteries, transports at 2x DS | No analog; VBAM forbids landing while Fort > 0 (p. 56) | VBAM's "half factors when fighting from transports" is the nearest thing to a hasty-landing penalty. |
| Orbit holder runs one assault class per planet per turn; one ownership change per turn | Ownership changes at end of encounter (p. 37); one encounter per system per turn | Compatible. |
| Two sides only, no allies join, one attack per observer per tick | Only Mutual Defense/Alliance partners combine; others "cannot intervene" (p. 39) | NC has no treaty ladder, so "no allies" is VBAM's default too. The difference is unit-vs-unit (NC) against power-vs-power (VBAM); see 3.5. |
| Neutral fleet in my solar system is attackable | 4.5.1.3 Neutral, p. 33 | Same rule, independently arrived at by classic. |
| Declared enemy attacked anywhere | 4.5.1.1 War, p. 33 | Same. NC's declaration is one-directional; VBAM's state is mutual. |
| Sighting/identification/intercept rolls per tick | None in-system (1.10) | NC's encounter loop is classic heritage, not VBAM. VBAM's Surprise roll and Stealth pre-battle withdrawal are its only "readiness" and "slipped away" mechanics. |
| Defender wins ties | Retreater loses ties (p. 43); conquest ties by troop DV then random (p. 57); initiative ties by DV (p. 38) | VBAM has no general defender-wins-ties rule. |
| Crippled ships heal after battle | Crippled persists until repaired at 25% cost (p. 61-62) | Deviation; see 3.2. |
| Retreat = Seek Home to nearest owned planet, ROE forced to 0 (`retreat.rs`) | Retreat = one jump to an adjacent non-enemy system, declared publicly, disengaged for the encounter (p. 43) | Partially covered by the engine but not by the spec/manual; see 3.1. |

---------------------------------------------------------------------------

## 3. VBAM mechanics NC did not adopt that address NC's known holes

### 3.1 Retreat destination

What NC has: the manual says a broken-off fleet "retreat[s] and abort[s] its
current mission"; the spec says the same. The engine (`retreat.rs`) sets
order Seek Home to the nearest owned planet by Manhattan distance, speed =
max, mission target 0, **ROE 0**; with no owned planet it holds in place (the
ROE-retreat path uses `unwrap_or(task_force.coords)`, i.e. Seek Home to the
sector it is already in). Classic's rule for the target side is "a holding
target that flees stays put with its order" (encounter spec section 5),
and its only "disengaged" effect is the awareness-3 term in the intercept
roll (1% forced intercept; the target may still *choose* to fight by its own
ROE).

What VBAM does (p. 43): the retreating side must immediately declare a
single-jump move to an **adjacent** system, may not pick a contested or
enemy-controlled one unless nothing else exists, the order is announced to
the encounter, and the force is disengaged (cannot attack or be attacked)
for the rest of that encounter. Bases stay; unbased fighters are abandoned.
Blockaded defenders cannot leave at all except Fast ships (p. 59).

Concrete borrowings for NC:

- Define the destination in the spec and manual: nearest owned world is the
  VBAM "adjacent friendly system" analog, but add VBAM's exclusion: do not
  route *through or into* the sector you just lost or any sector where an
  enemy combat fleet or starbase was identified this year unless no other
  owned world exists; the manual's Seek Home already retargets "if that
  planet is captured en route", which is the same instinct.
- Do not zero ROE on retreat. VBAM's retreater keeps its readiness and may be
  pursued next turn; NC's ROE 0 makes the fleet a permanent non-combatant
  until the player notices, and NC's own rule "giving such a fleet any order
  sets its ROE to 0" is meant for non-combat hulls, not beaten warships. If
  the intent is "do not re-engage this year", VBAM's "disengaged for the
  remainder of the encounter" is the right scope: a per-year flag (or
  awareness 3 both ways, which classic already sets) rather than a change to
  the player's standing ROE.
- Make the retreat visible: VBAM makes the destination public so the winner
  can pursue. NC's battle report already names both sides; adding "withdrew
  toward (x,y)" gives the winner the same information.
- When no owned world exists, VBAM's "unless no other movement options are
  available" permits retreating into enemy space; classic's
  `abort_to_seek_home` already climbs out of orbit (Move to own sector, TZ 0)
  in that case, which is better than NC's hold-in-place.
- The first leg of any NC retreat is a climb out of the layer, and that
  climb is where VBAM's Pursuit scenario lives; section 5.3 works this out.

### 3.2 Crippled damage healing between battles

What NC has: "surviving crippled ships are repaired automatically after
battle" (manual); "Only destroyed hulls are written back to the save file"
(spec). Crippled halves AS only, for the duration of one battle.

What VBAM does: crippled is a persistent state (2.3.5, p. 12) that halves all
factors and special abilities, is repaired at 25% of build cost at a shipyard
or by field repair (pp. 61-62), costs a turn of immobility while under repair,
and shapes every later decision: crippled ships cannot pursue, must lead a
pursued defender's line (p. 38), are skipped as flagships (p. 39), are rotated
to the reserve between rounds (pp. 43-44, 49), die first to supply attrition
(p. 58), and are worth keeping alive to repair (p. 81). Hostile Galaxy raiders
are explicitly balanced by "they will get progressively weaker after each
engagement" because they cannot repair (p. 76).

Why it matters for NC: with auto-heal there is no reason for a winner to go
home, no reason for a loser to have fled rather than fought to the last
round, no attrition across a 52-tick year of repeated encounters, and the
ROE 1 "enemy defenseless" state can never arise from damage. NC's fleet
record stores counts per class, not hulls, so VBAM's per-unit state maps to
**a crippled count per class** (BB_c, CA_c, DD_c, SB) carried in the Rust
model (the typed schema of t039 is the natural home; classic export can keep
healing at the .DAT edge, which preserves today's compatibility). Repair
rule candidates in VBAM's spirit: free at an owned world with a stardock
(VBAM's shipyard), at a fraction of build cost from the planet treasury (VBAM
25%), one maintenance turn; or decay-free but un-repaired in deep space.

### 3.3 Two strength measures (start vs break-off)

What NC has: the decision to *start* a fight uses classic's
`S = 21*BB + 7*CA + 2*DD`, +30 for a guard at rest with its base, and
`test_vs_base = 30 + S(guard)` (encounter spec section 4). The decision to
*break off* uses `rule_threshold_satisfied(roe, friendly_as, enemy_as)` on
total combat AS (90/30/10 per hull, 100 per starbase, crippled at half;
`exchange.rs`). The two weightings are close for ships (10.5 : 3.5 : 1 vs
9 : 3 : 1) but not for bases, and classic's measure depends on *which unit
the observer paired with first*: 2 BB (S 42) attacking a resting CA guard
(S 7 + 30 = 37) see odds 1.14; the same 2 BB attacking the base the guard
protects see E = 30 + 37 = 67, odds 0.63, so a ROE 7 fleet refuses a fight
that its own in-battle measure scores at 180 AS vs 130 AS = 1.38.

What VBAM does: there is no ratio ROE for players, and for every automated
actor the yardstick is one measure used at every decision point: EP of
ships and fighters for Independents (p. 74), EP for Ravager pursuit (p. 76),
lowest total DV for Hostile Galaxy targeting (p. 75-76). Initiative is total
AS (p. 38). And because the opponent in an encounter is a *power's* whole
presence in the system, the odds are computed once against everything that
will shoot back; VBAM has no "which unit did I bump into" ambiguity.

Borrowings: pick one function `strength(force)` and use it in `decide`,
`test_vs_base`, the intercept "target chooses to fight anyway" branch, and
the post-round check. Two defensible choices: (a) classic's S everywhere,
evaluated in battle on surviving hulls with crippled at half and a starbase
at 30 (keeps the oracle-matched start odds exactly, changes only the
break-off, which is already Rust's); or (b) AS everywhere, re-weighting the
unit table so 21:7:2:30 and 90:30:10:100 agree (BB 105, CA 35, DD 10,
SB 150 on the 10x scale would do it, or keep AS and set classic's test to
read AS). Whichever is chosen, VBAM's "odds against everything that will
shoot back" argues for evaluating the base-plus-guard pair as one opponent
in both tests, which also removes the 30-vs-67 discontinuity above.

### 3.4 One-exchange invasion suppression

What NC has: invasion = one CRT exchange against batteries; if any battery
survives, transports cannot land and the fleet retries next year; bombard =
three exchanges per turn with no carry-over of partial hits.

What VBAM does: bombardment is a **budget**, not a volley. BV = total AS;
Fort costs 10 x current Fort per point; half the unspent BV **carries over**
to next turn while superiority is held (p. 54, worked on p. 55: 41 BV takes
Fort 2 to 0 in one turn with 6 BV banked). Landing is a hard gate on Fort = 0
(p. 56). Troop bombardment can thin the garrison at 5 x Pop each with a 10%
collateral risk per kill (p. 54). Devastation makes over-bombing costly to
the attacker's own prize (p. 54).

Borrowings:

- Let invasion suppression run the same three-exchange turn as bombardment
  (the manual already says invasion's "first" stage is orbital suppression;
  giving it one exchange where bombard gets three makes Invade strictly
  worse at the job it must do first). VBAM's single BV pool used by all
  missions is the simpler model: one bombardment budget per turn, spent on
  batteries first, then on whatever the order allows.
- Adopt carry-over: a fleet that cannot clear the batteries this year banks
  half its unspent hits against them for next year while it holds orbit.
  This is the mechanism VBAM uses so "smaller fleets [can] perform orbital
  bombardment ... even if they can't generate enough bombardment value in a
  single turn" (p. 54), and it is the honest version of NC's "the attacker
  must commit enough firepower over enough turns".
- VBAM's cost-based deterrent (collateral Pop/Morale loss, Devastation)
  replaces NC's "softening may destroy at most half the garrison" cap with a
  price: the attacker may kill the whole garrison from orbit but risks the
  production it came for. NC already protects factories during Invade; the
  VBAM move is to let bombard-then-invade cost production instead of
  capping it.

### 3.5 Per-tick pairwise encounters vs theater / task-force battles

What NC has: every tick, one observer unit scans and attacks at most one
target; the battle is observer vs target (+ guard partner); "No other fleets
in the sector join (not allies, not other fleets of either empire). A second
target is fought only in a later tick" (encounter spec section 5). Five
friendly fleets in one orbit defend it one at a time, each fight a fresh
3-round commitment; three attacking fleets must fight three battles. The
observer also loses its tick of movement.

What VBAM does (1.2-1.5): one encounter per system per turn; each power's
entire in-system force is the pool; a **task force** of flagship + CR units
fights, the rest wait in the Reinforcements Pool and rotate in between
rounds; scenarios are two-sided but sequential, so a power can fight several
opponents in one turn; withdrawn forces are disengaged for the rest of the
encounter. The command limit is what keeps "everyone fights" tractable and
gives crippled ships somewhere to go.

The tension is real and VBAM does not resolve it for NC, because VBAM has no
tick loop: its encounter is defined at a phase boundary. Three ways to use
VBAM's structure without abandoning classic's loop (which the engagement ADR
commits to):

- **Keep the trigger, widen the sides.** Let classic decide *that* a fight
  starts (sighting, identification, ROE test, intercept, all unchanged), but
  when `battle(O, T)` fires, build each side as VBAM builds a task force:
  every fleet of O's empire and of T's empire in the same sector and layer
  that already has awareness >= 2 of the other side, plus the guard partner,
  joins its own empire's side. This keeps "exactly two sides" at the empire
  level and keeps allies out (VBAM's default too), but it changes the odds
  classic produces and so needs an ADR amendment, not a quiet change.
- **Keep the sides, add reinforcement between rounds.** VBAM's Recovery
  Phase is the narrower borrowing: between rounds, any fleet of either
  empire in the sector that identified the enemy this year may rotate in as
  reinforcements, and crippled hulls may rotate out. The battle still starts
  as classic's pair; the per-tick loop still runs; only the in-battle roster
  grows.
- **Keep everything, fix the cost.** If neither is wanted, at least drop the
  3-round lock for a *second* battle in the same sector and year against a
  side that already fought (VBAM's "disengaged for the remainder of the
  encounter"), so a defender is not forced through three fresh rounds per
  attacking fleet.

Whichever is picked, VBAM's "resolve each encounter completely before the
next, in random order" (p. 37) is already what NC's year loop does with its
once-a-year random unit order, so the two models agree on determinism and
isolation; they disagree only on roster. Section 5 shows that NC's depth
axis already enforces VBAM's space-then-orbit-then-ground order
geometrically, which is the strongest argument for keeping the tick loop
and widening only the roster.

### 3.6 Other VBAM pieces worth lifting

- **Surprise/readiness as a one-roll battle modifier** (p. 40) is a cheap way
  to express what NC's sighting data already knows: the side that identified
  the other first, or holds a base, or has scouts, fights at +1; the
  sneak-attacked side at -2 or worse. It would let NC's rich detection loop
  feed the battle instead of stopping at "who shoots first".
- **Pursuit scenario** (p. 38): a fleet that flees may be fought again next
  turn at its destination by a faster pursuer with its crippled ships
  excluded and the fugitive's crippled ships forced into the line. NC has
  the ingredients (max speed in the intercept roll, Seek Home target).
- **Bases cannot retreat; fighters/transports left behind** (p. 43): NC
  should say explicitly that a starbase's guard fleet retreating leaves the
  base to fight alone and that loaded transports that retreat keep their
  armies (the manual is silent).
- **Blockade economics** (p. 59): NC's Blockade order is only a watch; VBAM's
  rule "enemy ships > defending ships halves output and trade" is a
  one-line rule NC could add to give the order teeth.
- **Convoy Raid as an order** (pp. 31-32): move in, one battle, return the
  same turn. NC's View order already does the "dive in and climb out"
  movement; a raid variant of it would give light forces a job.

---------------------------------------------------------------------------

## 4. Where VBAM argues against choices NC made

### 4.1 Ratio-dependent CRT columns double-count the force ratio

VBAM's CRT is one column: effectiveness 10-50% of your AS, modified only by
qualitative factors (readiness, EW, stealth, new flagship, traits). A 3:1
force scores three times the hits of its victim and no more. NC's table
(Appendix B) picks a column from the AS ratio *and* multiplies the AS total:
a 3:1 force fires on Overwhelming (mean multiplier 1.725) while its victim
fires on Disadvantaged (mean 0.55), so the effective hit ratio is about
9.4:1, not 3:1. VBAM's design puts the whole weight of numbers into the AS
total and reserves the die for things numbers do not capture. If NC wants
the "mutual attrition, not one-sided wipeouts" its spec promises, VBAM says
to flatten the columns and move composition and base bonuses into +1/-1 roll
modifiers.

### 4.2 Per-round lethality is far above VBAM's, which hollows out the ROE check

VBAM's expected effectiveness is 30% of AS per round, and DV is comparable to
AS (Prometheus BC DV 6 AS 4; Human CL DV 5), so a battle lasts several rounds
and every round's retreat decision matters. NC's Even column averages 115% of
AS with DS = AS for cruisers, so (my arithmetic) an even cruiser fight
cripples both lines in round 1 and is largely decided by round 3, exactly when
NC first allows ROE to act. The 3-round commitment plus NC's lethality means
the post-round ROE check mostly fires on corpses. VBAM gets "fleets trade
meaningful blows" from low per-round effectiveness and a contested retreat,
not from forbidding retreat.

### 4.3 Forbidding retreat for three rounds versus making it uncertain

VBAM offers retreat every round, Defender first, but the opponent may contest
it with a die roll the retreater must win outright, convoys impose -2, and
refusing the scenario invites a Pursuit with the fugitive's crippled ships
forced to the front (pp. 38, 43). That is a cost model, and it produces the
same deterrent as NC's lock while leaving the choice with the commander.
NC's withdrawal exchange at a fixed Pressed column is a reasonable
deterministic substitute; the lock is the part VBAM argues against.

### 4.4 Absolute screening of auxiliaries

VBAM never makes civilians untouchable: they must be fielded unless you have
more ships than the enemy or are a well-prepared attacker (pp. 39-41), they
die to one hit (p. 12), the firer may direct damage at them (p. 44), and the
Convoy Raid exists to hunt them (p. 31). NC's rule that transports and ETACs
cannot be touched while one destroyer lives makes an escorted invasion
convoy or colony fleet risk-free until the escort is gone, and makes
"raid the transports" impossible as a tactic. VBAM's conditional screen
(the difference in line size, or readiness) is the better rule.

### 4.5 Planetary batteries as the most lethal unit in the game

In VBAM, fixed defenses are bases (which fight in space combat like ships,
with a free slot and +1 Surprise) and Fortification (which never fires, only
taxes bombardment and blocks landings). NC gives each ground battery AS 90
with execution fire, so a battery out-shoots a battleship and kills nominal
hulls outright. VBAM's position is that a planet's defense is a *siege cost*
(time, BV, collateral) rather than a fleet-killer; its "Harbinger"
bombardment fleets simply park and grind. If NC keeps lethal batteries, VBAM
suggests at least charging them the way it charges attackers: batteries that
fire are targets (they are, at 20 hits), and a fleet that holds orbit should
accumulate progress (carry-over, 3.4) rather than restart each year.

### 4.6 One-turn ground war to annihilation

VBAM resolves ground combat as one exchange per turn with beachheads,
reinforcement from orbit, economic disruption while contested, and a Morale
collapse on conquest (pp. 55-57). NC fights to the last army in one
maintenance (64-round cap) with a +1 defender column and a no-draw rule.
Given NC's turn is a year of 52 ticks, one-turn resolution is defensible on
pacing grounds; but VBAM's beachhead rule is the natural fix for NC's
"hostile fleet holds just outside orbit and attacks again each year": a
partial success (armies landed, garrison not eliminated) should leave a
foothold that the next year's landing builds on, instead of resetting.

### 4.7 Hard rules where VBAM uses choices or prices

"Cripple all before destroying any" (VBAM: receiver chooses), "softening
destroys at most half the garrison" (VBAM: no cap, collateral risk), "defender
wins ties" (VBAM: situational), "mixed fleet +1 column" (VBAM: no composition
rule) are all places where NC substituted a fixed rule for a choice or a cost.
That is appropriate for an engine that resolves battles without a human at
the table, but each should be documented as NC's own simplification rather
than as inherited from VBAM; only the AS/DV frame, the crippled step, the
directed-damage price, the base advantage, the encounter order, and the
"orbital superiority first" gate actually come from VBAM.

### 4.8 Where VBAM and NC agree, for the record

Simultaneous fire; ceil-rounded hits; d10 CRT; crippled before destroyed as
the normal path; 2x price for picking a target; bases stronger than the best
hull and unable to retreat; space, then orbit, then ground; one ownership
change per encounter; a neutral fleet in your own system is fair game; allies
do not join a fight; encounters resolved one at a time in random order. NC's
sighting/identification/intercept loop has no VBAM counterpart and should not
be measured against it.

---------------------------------------------------------------------------

## 5. VBAM's theaters, retreat, and pursuit on NC's depth axis

NC's geometry is taken as given (ec-movement-spec "Classic Movement
Semantics"): a unit has a sector and a depth `Z` in 0..1; `layer 0` is deep
space (`Z < 0.5`), `layer 1` is the solar system (`Z >= 0.5`), orbit is
`Z = 1.0`; units meet only in the same sector *and* layer; a fleet climbs to
`Z = 0`, crosses, then descends to its `TZ`; Bombard/Invade/Blitz strike on
the tick `Z` reaches 1.0 and the fleet stops there at speed 0; a fleet at
speed `v` spends `v/52` of depth or distance per tick, so the descent from
the layer boundary to orbit takes about `26/v` ticks (speed 3: 9 ticks;
speed 6: 4-5). Starbases also have a depth and move at speed 1; a base with
no owner's world beneath climbs to deep space, and its guards take its layer.

### 5.1 The three VBAM theaters on two layers plus the strike

| VBAM theater (one system, one Combat Phase) | NC location | Notes |
| --- | --- | --- |
| Interception scenario: pitched battle, no bases (p. 38) | Layer 0 of a sector, or layer 1 with no starbase on either side | NC's deep-space fights are pure Interceptions. VBAM's optional Deep Space Encounters (5.9, pp. 78-79: fleets meet *in the lane*, space combat only, nothing to bombard) is the closest thing VBAM has to NC's layer 0; VBAM's default has no such place. |
| Defensive scenario: bases included, one free, +1 Surprise, Fortification fighters sortie (pp. 38-40) | Layer 1 battle that includes a starbase in orbit, with its guard fleet as the partner | NC's "base + guard are one side" is exactly the Defensive task force. VBAM has no base in deep space; NC does (a base over a foreign world climbs out), which makes a "Defensive scenario in deep space" NC can have and VBAM cannot. |
| The Defender refuses an Interception and "retreats to the protection of its bases", forcing a Defensive scenario (p. 38) | A fleet descending to `Z = 1` beside its starbase (Guard Starbase order) | Geometry does what VBAM's refusal rule does: once the fleet is at the base, any attacker fights both. |
| "All space combat scenarios are resolved first" (p. 37) | The descent window: a hostile-order fleet is in layer 1 from `Z = 0.5` to `Z = 1.0`, visible at 20x (order_vis 20 once at its target) to every defender in the layer, for about `26/v` ticks before it can strike | VBAM's sequence is a phase order; NC's is a window of ticks. The defender's in-system fleets get `26/v` sighting rolls at hostile-mission visibility before the strike tick. |
| Orbital superiority: "eliminating all the defender's bases" (p. 53) | NC: no enemy combat fleet or defending starbase holding orbit (`Z = 1.0`, layer 1) | NC's gate is stricter (fleets block too). Both say the orbit theater opens only after the space theater is settled. |
| Orbital bombardment (pp. 53-55) | The strike at `Z = 1.0` on a Bombard order, fired in that tick's dispatch, repeated each year from orbit | VBAM's BV is spent in the Combat Phase; NC's three exchanges fire in the strike tick. |
| Fort must be 0 before troops land (p. 56); ground combat (pp. 55-57) | The strike at `Z = 1.0` on Invade (batteries first, then landing) or Blitz (landing through batteries) | Same gate for Invade; Blitz is NC's own. VBAM's beachhead would be "armies on the ground at `Z = 1`" persisting across years. |
| Encounter ends; ownership changes at the end (p. 37) | End of the strike tick; one ownership change per planet per turn | Agree. |

Two consequences of the mapping:

- VBAM's "space first, then orbit, then ground" is enforced in NC by the
  depth axis rather than by a phase list, but only *probabilistically*: a
  descending attacker is attackable for `26/v` ticks, not guaranteed to be
  engaged. A fast fleet (`v >= 26` crosses the whole layer in one tick) can
  strike before any defender's encounter check runs against it. VBAM
  guarantees the space battle happens if either side wants it; NC
  guarantees only the window. That is a feature of the tick loop, not a
  bug, but it should be stated in the combat spec next to "orbital
  supremacy".
- Within the strike tick itself, NC's order is step -> encounter(i) ->
  dispatch(i) for the *arriving* fleet, so the attacker's own scan (and any
  battle it starts, which costs it the tick) runs before its strike, while
  a defender whose turn in the random order falls later in the tick cannot
  hit it until the next tick. VBAM has no analog; the Attacker's +1 Surprise
  in a Defensive scenario (p. 38) is the nearest expression of "the attacker
  chose the moment".

### 5.2 What VBAM's retreat rule becomes on the depth axis

VBAM (p. 43): retreat = one jump to an adjacent, non-enemy system, declared
now, executed next Movement Phase, disengaged meanwhile. NC has no adjacent
system to jump to inside a sector; it has a layer to leave.

- **Retreat = climb out.** The NC form of "one jump to the adjacent system"
  is `TZ = 0` in the current sector (what classic's `abort_to_seek_home`
  already issues when no friendly world exists), followed by Seek Home. The
  fleet leaves layer 1 when `Z` drops below 0.5, i.e. after about `26/v`
  ticks from orbit, and reaches deep space (`Z = 0`) after `52/v`.
- **Deep space is VBAM's "non-enemy system".** NC's own rule says a sector's
  deep space is nobody's system: the "neutral fleet in one of our solar
  systems" trigger (N2) requires `layer(O.Z) == 1`, so a fleet in layer 0
  above an enemy world is attacked only by declared enemies. VBAM's "cannot
  retreat into a contested or enemy controlled system" is therefore
  satisfied automatically by climbing: the retreater reaches neutral ground
  without choosing a destination. The destination choice (which owned world
  to seek) only matters after the climb, and VBAM's "not through enemy
  systems" then means "not through sectors where enemy combat units were
  identified this year", which NC's shared awareness table can answer.
- **"Disengaged for the remainder of the encounter"** maps onto two NC
  facts. While the retreater is still in layer 1, both sides hold awareness
  3 of each other, and the intercept roll gives a pursuer only a 1% forced
  intercept (the rest is the retreater's own ROE choice), so it is nearly
  disengaged by classic's rule. When it crosses `Z = 0.5`, its awareness
  record is zeroed (every empire forgets it), so it enters deep space as a
  fresh, unknown contact at one-fifth of in-system visibility. That is a
  reasonable analog of VBAM's "safe this encounter, can be hunted next
  turn", and it falls out of the existing rules.
- **Bases cannot retreat (p. 43).** In NC a starbase over its owner's world
  stays; a base elsewhere climbs at speed 1, so an NC base can retreat,
  slowly, and VBAM's rule becomes "a starbase in orbit over its own world
  never climbs". The guard fleet retreating leaves the base alone in orbit,
  which the spec should say.
- **Blockaded defenders cannot leave; Fast ships can (p. 59).** NC's
  blockade-departure rule (encounter spec section 6) is the probabilistic
  version: a Blockade at post (`F = 5`) rolls to sight a climber at layer-1
  visibility times `(1 + speed/10)` each tick of the `26/v` climb, and the
  intercept roll auto-catches if the blockader's max speed exceeds the
  target's. VBAM's Fast exemption is NC's speed comparison.

### 5.3 What VBAM's Pursuit scenario becomes on the depth axis

VBAM's Pursuit (p. 38) is a second battle against a fleet that refused or
lost: the fugitive puts its crippled ships in front, takes -1 Surprise, gets
+2 on retreat rolls; the pursuer takes +1 Surprise, halves its flagship's
command limit, and may bring no crippled ships. It cannot be refused.

NC already has the window a Pursuit needs: the `26/v` ticks of climb-out
during which the beaten fleet is still in layer 1 with the winner. What NC
lacks is any rule that lets the winner use it, because awareness 3 drives the
forced-intercept chance to 1% and ROE 0 (set by `retreat.rs`) makes the
fugitive refuse every fight. The VBAM-shaped rule set is:

- **A winner may pursue during the climb.** For the fleets that held the
  field, treat the retreater as awareness 2 (identified, not fought) for the
  intercept roll while it is still in the layer, so the roll becomes the
  ordinary `100 * sin(pi*u) * O.max_speed / T.speed` chase instead of 1%.
  This is VBAM's "cannot be refused" with NC's speed-based escape chance
  standing in for the +2 retreat bonus.
- **Only undamaged pursuers.** VBAM's "may not include any crippled units"
  becomes: a pursuing side fires with its nominal hulls only (its crippled
  counts sit out), which is cheap once crippled counts persist (3.2).
- **Crippled in front.** The fugitive's target order in the pursuit battle
  puts its crippled counts first, which also matches VBAM's "fill
  non-flagship positions with crippled ships".
- **The chase continues at the destination.** VBAM announces the retreat
  order so the enemy can follow next turn. NC's Seek Home target is a
  sector the pursuer can be ordered to, and the battle report can carry it;
  the pursuer's awareness is reset when the fugitive crosses the layer, so
  the follow-up is a fresh sighting contest in layer 0 or at the
  destination, which is VBAM's "pursue them to their destination system
  next turn" without extra state.
- **Do not zero ROE.** With a pursuit rule in place, the fugitive's own ROE
  is what decides whether it turns to fight when the chase roll fails (the
  intercept roll's `test(T, S(O))` branch); forcing ROE 0 removes that
  decision and is not VBAM's rule.

### 5.4 Where the depth axis argues against a VBAM import

- VBAM's Defender may refuse an Interception and demand a Defensive scenario
  (p. 38). In NC the attacker's order fixes its `TZ`: a Patrol (`TZ 0`) can
  never be drawn into layer 1, so it can never be forced to fight a base;
  only orders with `TZ 1` descend. NC therefore needs no refusal rule; the
  order already chooses the theater.
- VBAM's "orbital superiority" requires eliminating bases, not fleets. On
  NC's axis an enemy combat fleet sitting at `Z = 1.0` is in the same layer
  and place as the attacker's strike, so NC's stricter gate (no enemy
  combat fleet in orbit) is the natural one; importing VBAM's base-only
  gate would let a bombard fleet strike past an untouched defending fleet
  that merely failed its sighting rolls.
- VBAM's mid-lane Deep Space Encounters are optional and off by default.
  NC's "Current Rust Simplification" (contact from co-location per tick, no
  crossed-path interception between sectors) is VBAM's default position,
  not a shortfall against it.
