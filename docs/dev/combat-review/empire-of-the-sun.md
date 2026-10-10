# Empire of the Sun vs. Nostrian Conquest combat

Research notes, 2026-10-10. Read-only; nothing under `~/dev` was edited.

Sources

- *Empire of the Sun*, 2021 edition, rules v3.22 (Mark Herman, GMT):
  `/home/niltempus/Documents/game_books/EOTS_Rules-2021-LR.pdf` (56 pages;
  printed page numbers equal PDF page numbers). Page text extracted with
  pypdf to `eots-pages/pNN.txt` beside this file. Citations below are
  `rule, p.N`.
- NC: `docs/dev/nc-combat-spec.md` (cited by section heading),
  `docs/manuals/nc_player_manual.typ` (cited by line: Combat Mechanics
  400-554, Missions 556-663, Appendix B 904-1081; line numbers as of the end of this session, during which the manual gained the t036 and t043 text),
  `docs/dev/ec-encounter-spec.md` sections 1-5, `docs/dev/ec-movement-spec.md`
  "Classic Movement Semantics" (the 2.5D model, taken as given), and, for the
  retreat and ROE claims in section 3, `rust/nc-engine/src/maint/combat/
  {retreat,fleet_battle,exchange,state}.rs`,
  `rust/nc-engine/src/maint/encounter/rules.rs`, and the port checklist in
  `docs/dev/ec-battle-spec.md` section 7.
- Section 5 maps EotS's battle hex, reaction, retreat, and intelligence onto
  NC's two depth layers.

Actor mapping used throughout

| EotS | NC |
| --- | --- |
| naval units (BB, CA, CV...) in a battle hex | DD / CA / BB in a fleet |
| land-based air at a base (defender's local striking power, ZOI) | ground batteries (AS 90, "shield wall") |
| ground units; intrinsic defense step | armies |
| friendly port / airfield | owned world, starbase |
| battle hex | sector + depth layer |
| offensive (card play) | a fleet's order and arrival tick |
| reaction move | (nothing: nobody else joins) |
| intelligence condition | (nothing: sighting/awareness decides only whether a fight starts) |

---------------------------------------------------------------------------

## 1. EotS combat, precisely

### 1.1 Frame

- One ten-sided die; a 0 is zero, never ten (1.22, p.3). Every fraction
  rounds up (Rounding, p.5).
- Units are two-step counters (full / reduced) with separate attack and
  defense values; a few are single-step and "considered to be on their
  reduced side" (counter diagrams p.2; 6.24, p.12). Counters on the map are
  always visible to both players (1.24, p.3): fog of war is in hands and
  intelligence, not in hidden units.
- The engine is the strategy card: played as an Operations Card (value 1-3)
  or as an Event (5.0, p.6). Movement = ops value x base allowance (naval 5,
  ground 1, air = range) (7.22, p.13; 8.1, p.15). Activations = HQ efficiency
  + OC value or event logistics value, from one HQ (7.21, p.13). An OC
  offensive may declare one battle hex; an EC any number (5.31, p.7; 7.24,
  p.14). Design note: an OC is a localized, operationally secure action; an
  EC is larger and "more likely to tip off the opposition" (7.1, p.13).
- Inactivated units defend if attacked but cannot initiate or move (6.0,
  p.9). Supply "has no effect on battle resolution"; its effects are in
  activation and the attrition phase (9.14, p.20).

### 1.2 Offensive sequence (7.2, p.13)

1 activate; 2 move; 3 declare battle hexes; 4 weather cancel; 5 pre-reaction
event bonuses; 6 Special Reaction rolls; 7 intelligence condition; 8 reaction
activation and move (skipped under Surprise); 9-10 attack reaction cards and
post-reaction bonuses; 11 resolve all battles; 12 post-battle attack cards;
13 post-battle movement, reaction player first; 14 emergency naval and air
moves.

### 1.3 Intelligence condition (5.2, p.7; 7.25-7.27, pp.14-15)

- Default is **Surprise Attack** for every offensive. The reaction player may
  change it once, for all battle hexes of that offensive, by either
  - playing an Intelligence or Counteroffensive reaction card (to Intercept
    or Ambush; if both are possible the result is Ambush; 5.32.A, p.8), or
  - one intelligence die roll <= the offensive card's printed intelligence
    value (OC or EC value as played), which can only yield **Intercept**,
    never Ambush. Offensive units that entered, moved within, or exited a
    reaction air ZOI (neutralized or not) give the roll -2; an unmodified 9
    always fails (7.25.D-E, p.14). A card printed "Surprise Attack" forbids
    the roll (5.31.B, p.7).
- **Special Reaction** (7.27, pp.14-15): a landing on an empty
  reaction-controlled city, resource, port, or airfield hex inside a reaction
  HQ range and a reaction air ZOI lets the reaction player roll to create a
  battle hex there; carriers assigned to cover the landing must fight it.
- Effects:
  - Surprise: no reaction move at all (7.26, p.14); the offensive player
    applies all hits first and only survivors reply (9.2.D, p.21); +3 to the
    offensive air-naval die roll (9.2.B, p.20).
  - Intercept: the reaction player may activate one in-supply HQ's worth of
    units (efficiency + the offensive card's OC value, or a counteroffensive
    card's logistics value) which may join any declared battle they can
    reach (7.26); hits are simultaneous (9.2.C, p.21).
  - Ambush: the reaction player fires first (9.2.E, p.21), Allies +4 (9.2.B,
    p.20); only two Allied cards allow it; some make any modified 9+ a
    critical hit (design note, p.21: "This is how a Midway like outcome would
    occur").

### 1.4 Who takes part (7.24, p.14; 9.1, pp.19-20)

- The attacker declares battle hexes after moving; any hex holding units of
  both sides must be declared; hexes with only reaction units may be declared
  if in range of activated air/carriers. All units in a battle hex must take
  part; no unit fights in more than one battle per offensive (9.1).
- Air and carrier units join from any friendly airfield / sea hex within air
  range (9.11, p.19). Air at extended range fights at half strength (9.2.A,
  p.20). Carriers that did not end in a friendly port must fight or cover a
  landing (9.11, p.20).
- Non-carrier naval units outside the hex but stacked with a participating
  carrier add no attack strength but are in the loss pool as escorts (9.12,
  p.20; design note p.20).
- The reaction side also counts its inactive air and naval units in the hex
  (9.2.A, p.20).
- Amphibious ground units of the offensive take part in the ground battle
  only if their side wins the air-naval battle (9.13, p.20).
- Aircraft ZOI (6.4, p.12): every in-supply air or carrier unit projects a
  2-hex ZOI; un-neutralized enemy ZOI blocks amphibious and strategic moves
  and sea supply/activation paths; even neutralized ZOI gives the -2
  intelligence modifier and enables Special Reaction.

### 1.5 Air-naval combat (9.2, pp.20-22)

- Each side sums attack strength (9.2.A). Each side rolls d10 + modifiers;
  the result is a **combat effectiveness multiplier**, hits = strength x
  multiplier, rounded up (9.2.B, p.20; CRT p.21):

  | modified roll | multiplier |
  | --- | --- |
  | 0, 1, 2 | x0.25 |
  | 3, 4, 5 | x0.5 |
  | 6, 7, 8 | x1 |
  | 9 | x1 and, if the 9 was unmodified, a **critical hit** |
  | 9 or greater (modified) | x1 |

  Air-naval modifiers (9.2.B, p.20): Ambush Allies +4; Surprise +3; 1943 +1
  and 1944-45 +3 for the Allies when US air/carriers are present; event
  modifiers. There is **no odds or force-ratio table**: "the game system
  gives equal weight to all factors brought into a battle" (player note,
  p.20).
- Applying hits (9.2.F, p.21-22). The roller distributes hits as he likes
  within:
  1. hits equal to a unit's defense strength flip it, or eliminate it if
     already reduced;
  2. **all full-strength units must be reduced before any unit is
     eliminated** (escorts outside the hex included);
  3. hits that cannot be legally applied are lost;
  4. air/carrier units not in a hex with enemy naval units can be hit only
     one-for-one by friendly air/carrier units in the battle; surface ships
     can always be hit;
  5. ground units may be hit only if the enemy had no air or naval units in
     the battle, and **the last ground step in a hex can never be eliminated
     by air-naval hits**; a hex with intrinsic defense is always the last
     step;
  6. **critical hit** (unmodified 9, or a card): the roller may ignore rule 2
     and eliminate units while full-strength units remain; and if he is
     mathematically unable to cause even one step loss, one step loss is
     assessed anyway on the lowest-defense unit that can take it (ties:
     reaction player's choice);
  7. Japanese carrier range advantage: absent an Allied critical hit, one
     received step may be moved between Japanese carriers (p.22).
- **Winner** (9.3, p.22): sum the attack strengths of the surviving units
  that contributed (reduced units at their reduced value; halved air stays
  halved; escorts outside the hex do not count). Higher total wins; **ties go
  to the reaction player**; if the reaction player has any surviving air or
  carrier unit and the offensive has none, the reaction player wins
  automatically; if nothing survives on either side it is an offensive
  victory (9.31).
- Consequences: reaction victory means amphibious units do not land, do not
  capture even an empty hex, and must leave in post-battle movement (9.32,
  p.23); offensive victory leads to ground combat if both sides have ground
  units, otherwise the side with ground units controls the hex, and with no
  ground units at all the reaction player keeps it (9.33). "If not, it is
  assumed that amphibious assault forces are turned back as at Coral Sea"
  (design note, p.23; also p.19).

### 1.6 Ground combat (9.4, p.23)

- **Always simultaneous, whatever the intelligence condition** (9.4.A).
  Each side sums ground attack strength (active and inactive units in the
  hex) and rolls:

  | modified roll | multiplier |
  | --- | --- |
  | <0, 0, 1, 2 | x0.5 |
  | 3-6 | x1 |
  | 7-8 | x1.5 |
  | 9+ | x2 |

  Offensive modifiers: +2 shore bombardment if only the offensive side has
  naval units in the hex after the air-naval battle; +2 air superiority if no
  reaction air/carrier survives; terrain -1 jungle, -2 mixed, -3 mountain.
  Reaction modifier: **+3 if it had any land or HQ unit in the hex before the
  amphibious assault**. Both: event modifiers; +1 British armor.
- Hits (9.4.B): only ground units; full before reduced; excess lost;
  **amphibious attackers have their defense strength halved**.
- Conclusion (9.4.C): sole survivor wins; otherwise the side that lost more
  steps retreats; **a tie means the reaction player wins and the offensive
  retreats**; if both sides are wiped out **the reaction player keeps the
  hex** ("a remnant force remains in the defender's hex", design note
  pp.23-24). Control passes to the offensive only if it is the sole
  survivor.
- Before any of this, a defender may **disengage** from a hex an attacker just
  entered if its attack strength is strictly greater and a free adjacent hex
  exists (8.43, p.18).

### 1.7 Retreat and post-battle movement (9.5-9.6, p.24; 8.21-8.22, p.16)

- An offensive ground unit that came by land retreats **into the hex it
  entered from**; one that came by sea retreats like a naval unit and must
  end in a friendly port or coastal hex or is eliminated (9.5; 8.45.C, p.19).
- A retreating reaction ground unit is **moved by the offensive player** into
  an adjacent hex that holds no offensive units, is not the attacker's entry
  hex, and does not overstack; a friendly named location if possible;
  if no such hex exists, or the battle hex is a one-hex island, the unit is
  eliminated (9.5).
- Post-battle movement (9.6): every activated unit that did not use strategic
  movement may move its offensive allowance, reaction player first. Naval
  units **must end in a friendly port, air units at a friendly airfield,
  otherwise they are eliminated** (8.21, p.16; 9.61-9.62, p.24). Reaction
  units must also end in supply within HQ range if possible.
- Inactive units in a hex that changed hands make an **emergency move**:
  naval to a friendly port within 10 hexes, air to an airfield within range,
  else eliminated (8.22, p.16; 8.32, p.17).
- Hex control changes "just prior to post battle movement" (6.5, p.12).

### 1.8 Amphibious assault (8.45-8.46, pp.18-19)

- Costs amphibious shipping points per unit or per step for corps/armies;
  one ASP at most in reaction (8.45.A).
- The path may not enter or leave a hex containing an enemy naval unit
  unless a friendly naval unit escorts the whole way, nor an un-neutralized
  enemy air ZOI (8.45.B).
- **Unescorted amphibious units are turned back, each losing one step, if
  enemy naval forces react into the battle hex**; with nothing left to fight,
  the battle is lost and cancelled (8.45.B, pp.18-19).
- Japanese organic transport: if the carrying naval unit is eliminated before
  ground combat the embarked unit dies with it; a step loss on the ship has
  no effect (8.46, p.19).
- Amphibious units only land after winning the air-naval battle (9.13,
  9.32-9.33) and defend at half strength on the beach (9.4.B.5).

### 1.9 Damage persistence: replacements and attrition

- Reduced units stay reduced until repaired. The Replacement Segment (4.12,
  p.6; 11.0, p.25): one replacement point flips a supplied reduced unit not in
  an un-neutralized enemy ZOI, or returns an eliminated unit at reduced
  strength; two points for a full-strength return. Allied points not used in
  the turn are lost; Japanese naval points can be banked; pre-war units can
  never be repaired (11.1). Replacements are scarce (e.g. two Allied ground
  and five air per turn, 11.31-11.32, p.26).
- Attrition (4.4, p.6; 6.24, pp.11-12): unsupplied full-strength air and
  ground units flip; unsupplied reduced ones out of any HQ's range are
  eliminated; naval units are immune.
- Outside an offensive no more than six naval units of a side may share a
  hex (8.24, p.16): forces disperse between battles.

### 1.10 Design philosophy (pp.6, 13, 20, 51-52)

- Card-driven: "The cards are the mechanism through which all movement,
  battle, and events are initiated" (p.6). The player "is not focused on the
  battles, but on resourcing and prosecuting the major axes of advance"
  (p.51). "Combat is a necessary way to show who has done a better job at
  resourcing their offensives ... meant to reward the player who can maintain
  a combined arms force with more mass than your opponent" (p.52).
- Combat "tends to be bloody, given the time scale ... some battles actually
  represent a series of battles" (p.20); the intelligence system plus the
  two-tier (air-naval then ground) battle gives "broad-brush combat outcomes
  ... with as little mechanical overhead as possible" (p.52).
- Decisive moments come from intelligence (Surprise / Ambush) and the
  critical hit, not from the odds (p.21).
- Attrition strategy depends on damage persisting: "The Allied units quickly
  recover full strength, while the Japanese get weaker over time. If the
  Allies fail to be aggressive and use up their replacements, they will face a
  stronger than historical Japanese air force" (p.52).

---------------------------------------------------------------------------

## 2. What in NC traces to EotS, and where NC deviates

| NC rule | EotS source | Verdict |
| --- | --- | --- |
| Simultaneous fire, "no first strike advantage" (spec Core Principle 1; manual 408) | Simultaneous only under **Intercept** (9.2.C); Surprise and Ambush are sequential first strikes (9.2.D-E); ground is always simultaneous (9.4.A) | Partial. NC took the Intercept case and dropped the condition that selects it. |
| `hits = ceil(AS x multiplier)` from a d10 (spec Seeded CRT; manual 1009) | hits = strength x effectiveness, round up (9.2.B, p.20; p.5) | Direct trace. |
| Column by **force ratio** (Disadvantaged ... Overwhelming), multipliers 0.00-2.50 over ten rows (manual 957-996) | No ratio table at all; three or four effectiveness bands keyed to the die, with additive DRMs (pp.21, 23) | NC invention; see 4.1. |
| Column shifts for mixed fleet and defending starbase (manual 998-1002) | Additive die modifiers for surprise, tech, terrain, shore bombardment, air superiority, prepared defender (9.2.B, 9.4) | Same role (qualitative edge), different arithmetic. |
| Critical hit on an unmodified 9 forcing one extra bypass loss (spec; manual 1013; `exchange.rs:264`) | 9.2.B and 9.2.F.6 (p.21): unmodified 9 lets hits bypass "reduce all before eliminating" and guarantees one step loss on the lowest-DS unit | Direct trace, slightly narrower (EotS lets *all* leftover hits bypass). |
| Defender wins ties (spec Core Principle 4; manual 415; ground equality favors defender) | 9.3 ties to the reaction player, plus auto-win with any surviving air vs none; 9.4.C.1 ground ties | Direct trace. |
| Nominal -> Crippled -> Destroyed; "all nominal ships must be crippled before any crippled ship is destroyed" (spec Core Principle 3; manual 410) | Full -> reduced -> eliminated; 9.2.F.1-2, 9.4.B.2-3 | Direct trace. |
| Execution fire: DD, starbases and batteries bypass the crippled state at 2x DS (spec Hit Allocation; manual 506-512, 542; `exchange.rs:293-315`) | Bypass exists only as the critical hit (9.2.F.6) | Deviation; see 4.2. |
| Auxiliaries (SC/TT/ET) screened until the combat line collapses (manual 410) | Escorts stacked with carriers are in the loss pool and must be reduced first (9.2.F.2, 9.12); carriers outside the hex hittable only 1:1 by air (9.2.F.4) | Same intent (p.20 design note on escorts), simpler mechanism. |
| Crippled ships repaired automatically after battle; only destroyed hulls persist (manual 410; spec Core Principle 3) | Reduced state persists; repair costs scarce replacement points (11.0) | Deviation; see 3.2. |
| Three mandatory rounds, ROE re-check from round 4, withdrawal exchange at the Pressed column, 64-round stalemate (spec Core Principle 2, Fleet Combat Sequence; manual 530-536) | One exchange per battle; winner by surviving attack strength; loser retreats (9.3, 9.4.C, 9.5) | NC invention (closer to classic's `5+Random(20)` round loop, ec-encounter 5.4). |
| Combined-arms +1 column (manual 412) | No flat bonus; balance is rewarded structurally (air needed to hit carriers, auto-loss without air, shore-bombardment and air-superiority DRMs) | Intent traces (p.20, p.52); mechanism does not. |
| Defending starbase +1 column; defender +1 column on the ground (spec Invasion Priority 3) | Reaction +3 on the ground only if it held the hex before the landing; amphibious attackers' DS halved (9.4) | Loose analogue. |
| Orbital supremacy before any assault; TT-only blitz only if no enemy combat force or starbase holds orbit (spec Simultaneous Arrival; manual 657) | Amphibious units land only after winning the air-naval battle (9.13, 9.33); unescorted landings turned back by reacting naval units (8.45.B) | Direct trace. EotS also charges the turned-back units a step each; NC's blocked blitz is free. |
| Transport destroyed takes its army with it (spec Bombardment, blitz landing fire) | 8.46: carrier ship eliminated before ground combat eliminates the embarked unit | Direct trace. |
| Softening may destroy at most half the garrison; capture only by landing (spec Invasion Priority 2; manual 655) | Air-naval hits can never remove the last ground step; capture requires ground units (9.2.F.5, 9.33) | Trace in spirit. NC bombardment's round-3 breakthrough can still kill every army (but cannot capture). |
| Ground combat cannot draw: on mutual annihilation the larger entering force holds with a remnant army; equality to the defender (spec Invasion Priority 3; manual 655) | Mutual annihilation: the **reaction player** keeps the hex, always (9.4.C.2) | Deviation in the attacker's favour; see 4.5. |
| One assault class per planet per turn, Blitz > Invade > Bombard; a planet changes hands at most once per turn (spec Simultaneous Arrival) | One battle per hex per offensive; control changes once, just before post-battle movement (9.1, 6.5) | Compatible; the priority ladder is NC's. |
| ROE 0-10 thresholds, S = 21 BB + 7 CA + 2 DD (+30 guard) (manual 518-528, 908-930; ec-encounter 4) | Nothing comparable: who fights is decided by activation and declared battle hexes; a defender can only disengage (8.43) or be reacted into a battle | Classic ECMAINT, not EotS. |
| Exactly two sides: attacker alone vs target + guard partner; "nobody else in the sector joins, not even allies" (spec Who Takes Part; manual 483; ec-encounter 5.3) | Everyone in the hex fights; in-range air joins; the defender reacts with an HQ's worth of units when not surprised (9.1, 7.26) | Classic ECMAINT, opposite of EotS; see 3.5. |
| Seeded RNG, 10x internal scale, per-tick/per-unit keyed draws | n/a | NC engineering. |

---------------------------------------------------------------------------

## 3. EotS mechanics NC did not adopt that bear on the known holes

### 3.1 No defined retreat destination

Where NC stands. The spec says a battle "ends when one side is destroyed or
retreats" and that "retreat of forces that did not hold the field, mission
aborts, reports" apply after a stalemate; the manual says a fleet that breaks
off "actually retreat[s] and abort[s] its current mission" (manual 536). No
document says where it goes. The code decides: `retreat.rs`
`retreat_task_force` / `apply_roe_retreat_to_task_force` set **Seek Home to
the nearest owned planet by Manhattan distance** (`nearest_owned_planet`,
lines 5-21), speed = max, mission target cleared, and **ROE forced to 0**
(`apply_retreat_order`, lines 32-38). A fleet with no owned planet gets Seek
Home aimed at its own sector (`unwrap_or(task_force.coords)`, line 53) or, via
`abort_mission_to_seek_home_or_hold`, a Hold in place inside the enemy
system. The classic port checklist already flags the gap: classic *scatter*
is Seek Home + ROE 0, classic *retreat* is Seek Home only for assault,
colonize, and guard fleets, and everyone else keeps their orders
(`ec-battle-spec.md` section 7, "Retreat outcomes").

What EotS does (1.7 above). Three principles worth lifting:

1. A retreat has a **legal destination set** (friendly port / airfield /
   named location reachable within this offensive's allowance), and an
   attacker retreats **to where it came from** (9.5, p.24).
2. The **winner constrains the loser's retreat**: the offensive player picks
   the reaction unit's retreat hex among legal ones, never through offensive
   units nor into the attacker's entry hex (9.5).
3. **No legal destination means elimination** (9.5; 8.21, p.16; 8.22, p.16).
   A hex that changes hands forces an emergency move within ten hexes or the
   ships are lost.

Concrete proposal for NC. Write the destination rule into the spec and
manual and make the code follow it rather than the reverse: an attacker that
loses in a solar system retreats up to the deep-space layer of the same
sector first (its "hex it entered from"; NC already has layers, manual
"Movement and Arrival") and then Seeks Home; a defender retreats to the
nearest owned world or friendly starbase reachable within N weeks at its
current max speed; with no destination it is *scattered* (classic's term)
and takes EotS's consequence, elimination or at least a further loss step.
Keep the player's ROE (EotS never changes a posture; classic zeroes it only on
scatter) so a retreating fleet is not silently rewritten into a pacifist.
Resolve the retreat move before the loser can be re-attacked, which is
EotS's "post-battle movement, reaction player first" (9.6), and which also
blunts the faster pursuer's automatic re-intercept (ec-encounter 4,
`intercept`: a faster observer always catches).

### 3.2 Crippled damage heals between battles

Where NC stands. "Surviving crippled ships are repaired automatically after
battle" (manual 410); "Only destroyed hulls are written back to the save
file" (spec Core Principle 3). `FleetCombatState.crippled` (`state.rs:38`) is
a per-battle array; the fleet record only has hull counts (ec-encounter
notation: `+0x26` BB, `+0x28` CA, `+0x2a` DD). Consequences: a battle that
cripples every enemy ship without a kill has no lasting effect; Suppression
fire's own stated purpose, "wins the immediate field, but does not always
result in permanent kills" (manual 506), evaporates next tick; the round-4
ROE check discounts crippled ships (`state.rs:65`, `crippled_as`), so a
mauled fleet is *more* likely to withdraw and then returns at full strength;
and "produces plausible mutual attrition" (spec Design Goals) is true only
inside the battle.

What EotS does (1.9 above). Reduced is a persistent state; repair is a
separate, rationed phase (11.0, p.25) conditioned on supply and on not being
under enemy air; unused points are lost for the Allies; pre-war units never
repair (11.1). Reduced units fight at their reduced printed values and cost
fewer shipping points (8.45.A). Unsupplied reduced units die in attrition
(6.24). The designer calls this the hinge of the whole air war (p.52).

Concrete proposal. Persist crippled counts per class in the typed game state
that t039 introduces (the schema no longer has to fit classic's two-byte
slots; at the `.DAT` export edge, crippled hulls either round to full or to
destroyed, documented). Repair happens in yearly maintenance only at an owned
world with a stardock or a starbase, at a treasury cost or capped per world
per year (EotS's replacement-point rationing); crippled ships elsewhere stay
crippled, fight at `crippled_as`, die at 1x DS, and, if the empire has no
world within supply range, are lost after a year (6.24). This gives
Suppression fire a reason to exist and gives the player an attrition lever.

### 3.3 Two different strength measures for ROE

Where NC stands. The encounter decision uses classic's `S = 21 BB + 7 CA + 2
DD`, +30 for a guard at rest, crippled hulls at full value
(`encounter/rules.rs:162 roe_permits(roe, own: u16, enemy: u16)`;
ec-encounter 4; manual 473). The in-battle re-check from round 4 uses
the battle's own 10x AS (BB 90, CA 30, DD 10, SB 100, batteries 90) with
crippled ships discounted (`fleet_battle.rs:588,788` ->
`exchange.rs:108 rule_threshold_satisfied(roe, friendly_as, enemy_as)`). The
CRT column is a third ratio, AS vs AS, over a different set of units (the
guard's +30 becomes a starbase's AS 100; DD:CA is 2:7 before the battle and
10:30 inside it). Appendix B documents only the first yardstick, while the
manual's "If the post-loss ratio no longer meets his threshold" (536) is
computed with the second. A fleet can be committed by one measure and
released at round 4 by the other, and a crippled-heavy fleet's threshold
drifts in a way the player cannot see.

What EotS does. One vocabulary: attack strength makes hits and decides the
winner (9.3), defense strength absorbs hits, reduced units use their reduced
printed values everywhere, and the one "do I have to fight" test in the game,
ground disengagement, compares attack strengths (8.43). There is no separate
odds scale.

Concrete proposal. Pick the AS/DS table as the single yardstick: the
encounter ROE test compares the two sides' total AS (starbase as AS 100, not
+30; crippled at `crippled_as` once 3.2 lands), and Appendix B shows one
table. If classic `S` must stay at the encounter boundary for oracle parity,
say so in both documents and show both tables, and derive the in-battle check
from `S` too so the two agree. Note t042 ("zero ROE on composition
recompute") is adjacent.

### 3.4 One-exchange invasion suppression

Where NC stands. Bombard runs three exchanges per turn with a breakthrough
round (spec Bombardment Resolution); Invade runs **one** suppression exchange
and "transports cannot land until all batteries are destroyed" (manual 655;
spec Invasion Priority 1). An invasion is "a single assault: whether it
takes the world, loses its armies, or is called off because the batteries
survived, the surviving ships hold position in orbit, and you must order any
new attack", and a fleet re-ordered against the world it orbits "attacks at
the next maintenance" (manual 649, 655; movement spec "Hostile Missions";
t036). So a fortified world is easier to bombard than to invade, the invader
takes battery focus fire for one try per year, a garrison with batteries
that survive one exchange cannot be invaded that year regardless of ground
odds, and the retry costs a year and a fresh order.

What EotS does (1.5-1.6). An assault is one combined battle: the air-naval
exchange decides supremacy by surviving strength (9.3); the ground battle
follows immediately, with the orbital result expressed as modifiers (+2
shore bombardment, +2 air superiority) and the landing's cost as modifiers
(defender +3 if in place before the landing; attackers' DS halved). Fixed
defenses are intrinsic ground steps that naval fire cannot remove (9.2.F.5)
and are fought on the ground. There is no "kill every battery first" gate,
so the landing can be defeated but never stalled indefinitely; and a landing
turned back by a naval reaction costs a step (8.45.B).

Concrete proposals, in order of ambition:

1. Minimal consistency: give invasion suppression the same three-exchange
   structure as bombardment, and let a failed attempt retry next tick, not
   next year.
2. EotS-shaped: batteries fight as part of the defender's side in the orbital
   battle (they already have AS 90); decide the orbit by winner determination
   (surviving AS, ties to the defender); if the invader wins orbit it lands
   against the garrison with surviving batteries acting as intrinsic ground
   steps that shift the column toward the defender (EotS's +3), instead of a
   hard gate. Keep the "softening kills at most half" rule, which is 9.2.F.5's
   spirit.
3. If the gate stays, charge a turned-back landing one transport step
   (8.45.B) so a failed invasion is not free, and make the retry cadence per
   tick.

### 3.5 Per-tick pairwise encounters vs theater / task-force battles

Where NC stands. The encounter loop is classic's: one observer per tick, one
target, one attack per observer per tick, a battle of exactly the attacker
versus the target and its guard partner; "Nobody else in the sector joins,
not even allies" (spec Who Takes Part; manual 483; ec-encounter 2 and
5.3). Yet the combat engine is built around `TaskForce { empire,
fleet_indices }` and `dominant_empire_after_battle` copes with any number of
empires (`retreat.rs`, `dominant_empire_after_battle`), and the spec's "Identify Participants: the
two sides of the encounter, each a task force, ordered by empire number"
reads like a sector battle with the two-sides rule bolted on afterwards
(`ec-battle-spec.md` section 7, "Participants": "never task forces per
empire"). Consequences: three fleets of one empire in a sector fight the
same enemy serially over three ticks, each attacker losing its tick's
movement; a fleet orbiting beside its own starbase does nothing when the base
is attacked unless it holds the Guard order; and the detection model
(awareness 0-3, sighting, intercept) decides *whether* a fight starts but
never *who* is in it or *how* it opens.

What EotS does (1.3-1.4). The battle hex is the unit of combat: everyone in
it fights (9.1), in-range air joins, and the defender's wider forces join
only by **reaction**, which exists only if the intelligence condition is
Intercept or Ambush (7.25-7.26). Surprise means the target fights alone and
takes the first strike. The attacker chooses which hexes become battles and
how many (one per OC, many per EC, 7.24).

Concrete proposal, which uses what NC already computes. Keep the per-tick
scan to decide *that* a battle starts and *who the attacker is*. Then
resolve one **sector/layer battle**:

- Side A: the attacker plus other fleets of its empire in the same sector and
  layer whose ROE passes (activation).
- Side B: the target and its partner, plus other fleets and bases of the
  target's empire in the sector and layer **only if that empire's awareness
  of the attacker is >= 2** (Intercept: it identified the attacker). With
  awareness 0-1 it is Surprise: side B is the target and partner only and
  side A fires first (9.2.D) or gets a column shift (9.2.B). Awareness 3
  ("fought") on the defender's side could play Ambush: side B fires first.
- Several attacks in one sector in one tick collapse into one battle (9.1:
  one battle per hex per offensive), with the attacker order deciding who is
  the offensive side.

This turns the awareness table from a gate into the battle's intelligence
condition, resolves the ally-in-the-same-orbit oddity, and keeps classic's
encounter odds untouched for the oracle because the scan is unchanged. It
also removes the "attacker loses its tick's movement" quirk for every fleet
but one.

---------------------------------------------------------------------------

## 4. Where EotS argues against a choice NC made

1. **Force-ratio CRT columns** (manual 957-996). EotS has no odds table on
   purpose: "the game system gives equal weight to all factors brought into a
   battle" (p.20), and the weakest result is still x0.25 (air-naval) or x0.5
   (ground), rounding up (p.5), so a weak side always draws blood. NC counts
   mass twice, once in AS and again in the column, and its Disadvantaged
   column has a 0.00 row: a 2:1 underdog can deal nothing, while
   Overwhelming reaches x2.5. This amplifies the doom-stack incentive that
   3,000-ship fleets (manual 516) already create, where EotS caps stacks at
   six naval units between offensives (8.24, p.16). EotS would reserve
   column shifts or DRMs for qualitative edges (surprise, starbase, mixed
   fleet, terrain) and let mass speak through AS alone.

2. **Routine bypass of step reduction ("Execution fire")**. In EotS the
   reduce-before-eliminate rule is the core of hit allocation and the only
   bypass is the critical hit: "There are only two Allied cards that enable
   the Ambush condition, so it is not the normal state of affairs unless you
   can roll a lot of 9s" (p.21). NC hands the bypass to whole unit classes
   (DD, starbases) and to every planetary battery (manual 506-512, 542), so
   the step model is the exception for DD-heavy fleets and absent from all
   planetary combat, and the 10/5 destroyer becomes the game's executioner.
   If the aim is that fortresses kill rather than cripple, EotS expresses
   lethality as a modifier (+3 prepared defender, 9.4) and as "the defender
   chooses the last step" (9.2.F.5), not as a bypass.

3. **Dropping the intelligence condition** ("There is no first strike
   advantage", manual 408). In EotS detection is the single most consequential
   battle variable: it decides first strike, a +3/+4 modifier, and whether the
   defender may reinforce at all. NC has a richer detection model than EotS
   but spends it only on whether a fight starts; classic's own battle entry
   even takes a `surprise` flag that the encounter path always passes as 0
   (ec-encounter 5.4, ec-battle 3). Simultaneity as a fixed rule is the one
   EotS case (Intercept) promoted to always.

4. **Round loop to annihilation with a 64-round stalemate**. EotS is one
   bloody exchange, a winner by surviving strength, and a retreat (9.3-9.5):
   "Combat tends to be bloody ... some battles actually represent a series of
   battles" (p.20). NC's loop makes outcomes bimodal (wipe-out or ROE
   withdrawal) and needs three patches to behave: the three-round commitment,
   the withdrawal exchange, and the 64-round stalemate. A single mandatory
   exchange already forbids bailing, which is what the three-round rule was
   added for (manual 534).

5. **Mutual annihilation goes to the larger force** (spec Invasion Priority
   3; manual 655). EotS rules that when both sides are wiped out "the Reaction
   player maintains control of the hex" (9.4.C.2), because a remnant always
   belongs to the side that was there. NC lets an invader capture a world
   with "a remnant army" after both sides die, which is the one place NC
   breaks its own "defender wins ties" principle.

6. **Automatic repair** (3.2). The designer's notes make damage persistence
   the engine of the historical air war (p.52): attrition is a strategy only
   if the enemy stays hurt. With auto-repair only kills matter, so NC's
   Suppression / Execution distinction collapses into "who has more
   destroyers".

7. **A lost battle rewrites the player's ROE to 0** (`retreat.rs:37`). EotS
   never changes a unit's posture as a battle result; post-battle movement
   only relocates. Classic zeroes ROE only on scatter. An order the player set
   being silently overwritten is a surprising side effect and is not stated
   in the manual.

8. **A blocked transport-only blitz is free** (spec Simultaneous Arrival).
   EotS's analogous turn-back (8.45.B) costs each unit a step. Cheap probing
   landings have no downside in NC.

9. **Starbase defense as a flat +1 column** while batteries get a bypass.
   EotS treats fixed defenses as the last, unremovable ground step (9.2.F.5)
   and gives the dug-in defender +3 on the ground; it never lets the fortress
   itself ignore the loss rules. NC's two fixed-defense types are modelled on
   opposite principles.

10. **A beaten assault fleet holds position in orbit** (manual 649, 655,
    657, landed with t036). EotS is explicit that an amphibious force that
    loses the air-naval or ground battle "must later conduct post-battle
    movement out of the hex" (9.32, p.23; 8.45.C, p.19) to a friendly port
    or coastal hex, or be eliminated; a naval unit that cannot end an
    offensive in a friendly port is eliminated (8.21, p.16). Holding in the
    enemy's orbit after losing is the one thing EotS never allows, and it
    leaves the loser parked under the batteries that just beat it, at x20
    visibility, with no retreat and no new order until next year.

Where EotS supports NC as written: defender wins ties (9.3, 9.4.C.1);
critical hit on an unmodified 9 (9.2.F.6); reduce all before eliminating
(9.2.F.2); auxiliaries screened by the combat line (9.12, 9.2.F.4);
transports die with their cargo (8.46); supremacy before landing (9.13,
9.33); softening cannot by itself take a world (9.2.F.5); control changes at
most once per battle (6.5).

---------------------------------------------------------------------------

## 5. Mapping EotS onto NC's 2.5D movement

Taken as given (`ec-movement-spec.md` "Classic Movement Semantics";
ec-encounter 1): a fleet has a sector and a depth `Z` in `0..1`; the layer is
`Z >= 0.5` (system) or `Z < 0.5` (deep space); units meet only in the same
sector and layer; a tick's budget is `speed / 52` and is spent first on
climbing to `Z = 0`, then on crossing, then on descending toward `TZ`; a
hostile strike fires on the tick `Z` reaches `1.0` and the fleet stops in
orbit; a fleet that starts the year in orbit on a hostile order strikes on
tick 1. Planets and IPBMs are not units and never sight anything; only
fleets and starbases do. Visibility in the system layer is x5, and x20 for an
assault fleet once it is in its target sector's system layer (ec-encounter 3).

### 5.1 Battle hex = (sector, layer)

EotS has one kind of hex; NC stacks two on each sector. The split already
matches a distinction EotS draws in words:

- **Deep space is EotS's sea hex.** Naval units there "are in constant motion
  and occupy a common hex only as an artifact of the hexgrid" and pass
  through each other freely (8.2, 8.11, pp.15-16). NC's "no departure strike"
  and "transit fleets ignore each other unless sighted" are this rule.
  Battles in deep space are fleet-only: no batteries, no armies, and (as in
  EotS) they never change who owns anything, because "Air and/or naval units
  alone can never change the hex control" (Control, p.4).
- **The system layer is EotS's coastal hex with a port and an airfield.** The
  world is the port (Seek Home's destination, the stardock, repair), the
  batteries are the land-based air that defends it and cannot be bypassed
  (9.2.F.5), the starbase is a fortified port, the armies are the ground
  units. EotS's "all units in the battle hex must participate" (9.1)
  therefore says: an orbital battle involves every unit in that system layer,
  batteries included, which is section 3.5's proposal with the layer as the
  hex.
- **The descent from `Z = 0.5` to `1.0` is EotS's amphibious move into the
  battle hex** (8.45). The attacker is inside the defender's "hex" for
  `26 / speed` ticks before it can strike (speed 2: 13 ticks; speed 5: 5;
  speed 10: 3), during which orbiting defenders, guards and starbases may
  sight and attack it (the encounter check runs between step and arrival
  test every tick). EotS forbids an unescorted amphibious path through a hex
  holding an enemy naval unit and turns it back with a step loss if enemy
  naval units react in (8.45.B); NC's "a transport-only blitz may proceed only
  if no enemy combat force or starbase holds orbit" is the same gate, minus
  the step loss (section 4.8).
- **The strike at `Z = 1.0` is EotS's declared battle hex** at the end of the
  offensive's movement (7.24): the order is the offensive, arrival is the
  declaration, and EotS then resolves air-naval and ground in one battle
  (9.0). NC resolves the orbital encounter and the assault as separate events
  that can fall on different ticks; the layer model does not require that,
  and 3.4's EotS-shaped invasion (batteries in the orbital battle, landing on
  winning orbit) fits one strike tick exactly.

### 5.2 Reaction, across and within layers

EotS reaction units are those the reacting HQ can activate that can *reach*
the declared battle (7.26). In NC the reachable set is:

- **Same sector, same layer:** EotS's "already in the hex": they fight,
  activated or not (9.1, 9.2.A). This is the minimal reaction rule and needs
  no movement: the defender's other fleets and its starbase in the layer join
  the target's side when the defending empire is not surprised (5.4).
- **Same sector, other layer:** EotS's adjacent sea hex. A deep-space fleet
  could "react into" the system by descending, but at `speed / 52` per tick
  it needs `26 / speed` ticks to cross the layer boundary and `52 / speed`
  to reach orbit, so it cannot join a battle resolved in one tick. EotS's
  answer is that reaction is movement *before* battle resolution (7.2 steps
  8 and 11), so either (a) NC keeps battles one-tick and limits reaction to
  the same layer, or (b) a deep-space guard of a world is modelled as EotS
  models a carrier covering a landing from range (7.24, 9.11): a fleet on
  Guard/Blockade in deep space with the world as target counts as "in range"
  of the system layer and joins the orbital battle without moving. (b) is
  the cheaper way to let a blockade protect its world against a descending
  assault that it has identified, which today it can do only by catching the
  assault in the same layer first.
- **Other sectors:** out of HQ range; no reaction. EotS's HQ range has no NC
  analogue and need not get one.

Guard Starbase already is EotS's "escort stacked with the carrier" (9.12): it
rides with the base, takes the base's layer, and is in the base's loss pool.
That rule should survive any widening of participation.

### 5.3 Retreat, with the layers as the destination grid

EotS's retreat rules (9.5-9.6, 8.21-8.22) translate almost one-for-one once
"adjacent hex" is read as "the other layer of this sector" and "friendly
port" as "the system layer of an owned world or a friendly starbase's layer":

| EotS | NC reading |
| --- | --- |
| Attacker that came by land retreats into the hex it entered from (9.5) | An attacker beaten in the system layer climbs back to deep space in the same sector (`TZ = 0`), which is what classic's abort-with-no-home already does (ec-encounter 4, `abort_to_seek_home`: "and are leaving the solar system"), then Seeks Home. |
| Amphibious attacker retreats like a naval unit; must end in a friendly port or coastal hex, else eliminated (8.45.C, 9.62) | A beaten Invade/Blitz fleet climbs out and Seeks Home; no owned world or friendly base reachable means *scattered*. NC's current rule is the opposite: "Win or lose, the fleet then holds position in orbit" (manual 649, 655, 657), see 4.10. |
| Winner moves the losing defender to an adjacent hex with no attacker units, preferring a friendly named location; none means elimination (9.5) | A defender beaten in the system layer of a world it still owns stays in orbit under its batteries (the port is friendly; EotS lets post-battle movement end in the same hex's port). Beaten in a system it does not own, or in deep space: Seek Home to the nearest owned world or friendly starbase within reach; none means scattered. The winner's units block the layer they hold. |
| Inactive naval units in a hex that changed hands: emergency move to a friendly port within **10 hexes**, else eliminated (8.22) | Fleets in orbit of a world that was just captured: Seek Home to an owned world or friendly base within 10 sectors, else scattered. This is the natural cap for "nearest owned planet" in `retreat.rs`, which today has no limit and falls back to holding in place. |
| Post-battle movement happens before the next offensive (7.2 step 13) | The loser's layer change is resolved in the battle tick, so the pursuer's next encounter check finds it in the other layer and must descend or climb to follow; a faster pursuer still catches it in deep space (ec-encounter 4), as EotS naval units can chase into a sea hex. |

The one EotS rule with no layer reading is "the winner chooses the loser's
hex"; in NC the engine chooses, and the only choice is layer and then home.

### 5.4 Intelligence and surprise for sighting-gated battles

NC already computes an intelligence condition per tick and per empire; it
only lacks a consumer. The pieces line up with 7.25 (p.14):

- **The intelligence die roll is the sighting roll.** EotS rolls once per
  offensive against the card's intelligence value, -2 if the attacker moved
  through a reaction air ZOI, auto-fail on an unmodified 9. NC rolls
  `10 * awareness + vis * F` per tick per observer. Entering the system
  layer is entering the ZOI: visibility x5, and x20 once in the target
  system on an assault order, so a descending assault fleet is up to 100x
  more visible than in transit. The number of rolls the defender gets is the
  descent window, `26 / speed` ticks, so a fast strike is "operationally
  secure" the way an OC offensive is (7.1 design note, p.13), while classic's
  `x (1 + speed / 10)` partially taxes speed. The probability EotS prints on
  the card emerges in NC from speed, fleet size, and who is watching.
- **Who is watching decides the condition.** A starbase in the system layer
  (`F = 25`) sees one cruiser at 13% per tick and an assault BB at
  essentially 100%; a Hold fleet in orbit (`F = 1`) sees that BB at about
  19% per tick; a world with no starbase and no fleet in orbit sees nothing,
  because planets are not units. The starbase report "The aliens appear to
  be headed at our planet X! We are alerting all fleets in the area" is
  today text only (ec-encounter 2, starbase observer); it is the Intercept
  trigger EotS would make it. Proposal: at the strike tick, if the
  defending empire's awareness of the attacker is `>= 2`, the condition is
  **Intercept** (everyone in the layer fights, simultaneous, 9.2.C); if it is
  `0` or `1`, it is **Surprise** (only the target and its partner and the
  batteries respond, and the attacker's first volley lands before theirs,
  9.2.D, or takes a column shift in place of EotS's +3). Awareness `3`
  ("fought") or identification while the attacker was still in deep space,
  i.e. before it crossed `Z = 0.5`, is the rare **Ambush**: the defender
  fires first. A fleet that starts the year in orbit on a hostile order has
  been watched for a year and is never a surprise.
- **Surprise is bounded the EotS way.** 9.2.F.5 (the last ground step cannot
  be removed by naval fire) and the batteries' own return fire keep a
  surprise strike from deciding a world by itself; it decides the first
  exchange, which is the "Midway" the designer wanted (p.21) and the only
  thing that makes scouts and starbases worth more than their guns.
- **The intercept roll is ground disengagement.** NC's target may notice the
  attacker and run, or choose to fight if its ROE accepts (ec-encounter 4,
  `intercept`); EotS lets a defender step aside before battle only if its
  attack strength is strictly greater and a free hex exists (8.43, p.18).
  Both are pre-battle escapes keyed to awareness; NC's is the better one for
  a two-layer map because "the free adjacent hex" is always the other layer.
- **Reports already carry the condition.** "We successfully intercepted ..."
  and "We were attacked by ..." (ec-encounter 5.8) are the two sides of
  Intercept and Surprise; the reports need no new vocabulary.

EotS's lesson for the sighting gate is therefore not to change how fights
start but to let the awareness the gate computes shape the fight it starts:
who else is in the layer, who fires first, and where the loser goes.
