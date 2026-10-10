# ec4x combat vs NC combat: rule-by-rule comparison

Research note, 2026-10-10. Nothing under `~/dev` was edited.

## 0. Sources

ec4x (Nim, `/home/niltempus/dev/ec4x`):
- `docs/specs/07-combat.md` (1301 lines; the current rule set, quoted below as "spec")
- `docs/engine/combat.md` (theater orchestration, participant filtering, CON2 effects)
- `docs/architecture/combat-engine.md` (older squadron/phase design: Ambush → Fighters → Capitals, CER 0.75 step)
- `docs/player-manual/EC4X-Player-Manual.adoc` §7 (lines 2742-3701; also the older squadron design, with
  a morale-modifies-ROE table and "Victorious fleet does NOT automatically pursue", which the spec's
  pursuit volley supersedes)
- `docs/archive/testing/DESPERATION_IMPLEMENTATION_SUMMARY.md`, `docs/archive/specs_review.md`
- `/home/niltempus/dev/ec4x_assessment.md`

Where the ec4x manual and spec disagree, this note takes `07-combat.md` as ec4x's rule and flags the
manual variant.

Nostrian Conquest (`/home/niltempus/dev/nostrian-conquest`):
- `docs/dev/nc-combat-spec.md`
- `docs/manuals/nc_player_manual.typ`: "Combat Mechanics" (lines 397-553), "Mission Details" (618-660),
  "Appendix B" (896-1074)
- `docs/dev/ec-encounter-spec.md` §1-5 (classic's per-tick encounter loop); `docs/dev/ec-battle-spec.md`
  §3 (classic's own battle, for the conflict section only)
- `docs/notes/decisions/gtkmxw3l-engagement-follows-the-classic-ecmaint-encounter-rule.md`,
  `b5u6lekb-rust-keeps-its-own-combat-system.md`, `sb0sfjgt-rust-fixes-classic-battle-bugs.md`
  (superseded), `lzvpk22s-planetary-hits-pay-defense-strength.md`
- `rust/nc-engine/src/maint/combat/{fleet_battle.rs,assault.rs,exchange.rs,retreat.rs,state.rs,mod.rs}`,
  `rust/nc-engine/src/maint/encounter/rules.rs` (decide_vs_base)

Status vocabulary in the tables: **SAME**, **SIMPLIFIED** (how), **DROPPED**, **CONTRADICTED** (by the
classic encounter loop NC adopted), **NC-ONLY** (NC has a rule ec4x lacks).

## 1. Entity map: what ec4x combat needs that EC does not have

| ec4x entity / stat | EC equivalent | Note |
| --- | --- | --- |
| Capitals (DD, CA, BB, …) | DD, CA, BB | direct |
| Raiders (CLK cloak) | none | no stealth hull; a lone Scout is merely the least *visible* fleet |
| Fighters, Carriers | none | |
| Scouts with ELI | SC | EC scouts are sensor platforms with no ELI level |
| ETAC, Troop Transport (auxiliaries) | ET, TT | direct |
| Starbase (Kastra) | SB | direct |
| Planet-Breaker | none | |
| Planetary shields (SLD) | none | |
| Ground batteries | ground batteries | direct |
| Armies + Marines | armies | one ground unit type; marines = loaded armies |
| Spaceports / shipyards / drydocks (Neoria) | stardock (docked items on the planet record) | no repair facility entity |
| Prestige / morale | none | |
| Diplomacy: Enemy / Hostile / Neutral / NAP, mission tiers, grace period | one-way "declared enemy" flag per empire pair | |
| Reserve / mothballed fleets | none | |
| Guard / Hold / Patrol commands | Guard Starbase (4), Guard/Blockade World (5), Hold (0), Patrol (3) | EC's guards *do* fight, see §2 |
| Homeworld | each empire's home world (setup places them; ADR 0dt7sw73) | mappable |
| Restricted jump lanes | none (2.5D free movement, two depth layers) | layers give a natural space/orbit split |

So of the ec4x machinery, everything that touches fighters, raiders/detection, shields, planet-breakers,
morale, drydocks, marines-vs-armies, reserve/mothball, and the diplomatic ladder has no EC entity to
stand on. Everything about crippled state, hit application, ROE ratios, task forces, pursuit, theaters,
starbase bonuses, batteries, armies, invasion/blitz shape, and homeworld defence maps onto EC's six
hulls + SB + batteries + armies with no new entity.

## 2. Comparison table

### 2.1 Theaters (spec 7.1)

| # | ec4x rule | ec4x text | NC status | NC detail | Fits EC entities? |
| --- | --- | --- | --- | --- | --- |
| T1 | Three sequential theaters, no skipping | "Your fleets cannot bypass defenses." Space → Orbital → Planetary | DROPPED | No theaters. A battle is one two-sided encounter wherever classic's loop pairs two units (deep space or system layer). "Orbital supremacy" before an assault is an AS comparison among that turn's assault entrants (`assault.rs select_orbital_supremacy_empire`), not a fight; the owner's starbase is counted only when no entrant has combat AS (see §6.1). | Yes: layer 0 = space, layer 1 = orbit is already in the data |
| T2 | Space theater: only *mobile* fleets fight; Hold/Guard/Reserve defend in orbit only | "Guard commands mean enemy fleets defend in orbital combat only" | CONTRADICTED | Classic pairs units by sector + layer, not by order. A Hold fleet in orbit is attacked ("a holding target that flees stays put"). Guard Starbase fleets do not *close* but attack identified targets on ROE (mode 3) and are fought together with their base. Guard/Blockade fleets close hard (F = 5) and attack arriving assault/colonize fleets. | Yes (orders 0/4/5 exist) |
| T3 | Orbital theater: guard fleets + reserve + starbases + unassigned ships fight as one | "fight as a unified defensive position" | SIMPLIFIED | A side is the target plus one partner: the guarded base (if the target is a Guard Starbase fleet at speed 0) or the base's guard fleet (if the target is the base). Other fleets of the same empire at the world never join. | Yes |
| T4 | Planetary theater after orbital supremacy: Bombard / Invade / Blitz | | SIMPLIFIED | One assault class per planet per turn, priority Blitz > Invade > Bombard; only the orbit holder's fleets on that class act; a planet changes owner at most once per turn. | Yes |

### 2.2 Ship status (spec 7.2.1)

| # | ec4x rule | ec4x text | NC status | NC detail | Fits? |
| --- | --- | --- | --- | --- | --- |
| S1 | Nominal → Crippled → Destroyed; crippled = 50 % AS and 50 % DS | "Attack Strength (AS) reduced to 50 % / Defense Strength (DS) reduced to 50 %" | SIMPLIFIED | In-battle only (`FleetCombatState.crippled`). `crippled_as = AS / 2`. DS is not halved; instead crippling costs 1× DS and finishing a crippled ship costs another 1× DS (execution fire kills a nominal ship for 2× DS). Only destroyed hulls are written back; manual: "surviving crippled ships are repaired automatically after battle". Bombard AS of crippled ships is halved (`bombard_attack_as`). | Yes, but FLEETS.DAT has no crippled field |
| S2 | Crippled persists: 50 % maintenance, cannot use restricted lanes, needs drydock (1 turn, 25 % build cost) | "Requires drydock repairs (1 turn, 25% of build cost)" | DROPPED | Nothing persists past the battle. | No drydock, lanes, or maintenance in EC; an owned world's orbit could stand in for the drydock |
| S3 | Fighters skip Crippled | | n/a | No fighters. Nearest analog: a DD (DS 5) dies to 10 execution hits. | — |

### 2.3 Hit application (spec 7.2.2)

| # | ec4x rule | ec4x text | NC status | NC detail | Fits? |
| --- | --- | --- | --- | --- | --- |
| H1 | Rule 1: cripple all before destroying any; excess hits lost | "All full-strength ships must be crippled before any crippled ships can be destroyed." | SAME for suppression fire (CA + BB hits) | `apply_hits_to_fleet`: cripple nominal ships in order DD, CA, BB, SB; only when every class in the target set has no nominal ship left do leftover hits destroy crippled ones; remainder lost. NC-ONLY: *execution fire* (DD, SB, all battery fire) bypasses Rule 1: finishes crippled at 1× DS, then kills nominal at 2× DS. | Yes |
| H2 | Rule 2: natural 9 = critical; may destroy with nominal ships present; "Explicitly targets the highest-value … asset"; at least one cripple/destroy; AAR event | | SIMPLIFIED, and inverted | Natural 9 (with AS > 0) = one extra loss (`apply_critical_hit_to_fleet`): the first *crippled* unit in order DD, SC, TT, ET, CA, BB, SB, else the first nominal unit in that order. So it hits the *cheapest* unit, and it pierces the auxiliary screen (a scout or transport dies before a cruiser). No report event. | Yes; targeting is the opposite of ec4x's |
| H3 | Rule 3: cascading overkill — hits > 1.5× enemy total DS shatter cripple-first protection | "Excess hits cascade directly into destroying the crippled ships in the same round." | DROPPED as a rule | Partly covered by execution fire (DD/SB-heavy volleys kill outright) and by suppression leftovers destroying crippled ships once every class is crippled. | Yes, trivial (total DS is known) |
| H4 | Target choice (manual 7.3.4): weighted random by DS, crippled ×2 weight; spec: none stated | | different | Deterministic lowest-DS first: DD 5 → CA 30 → BB 100 → SB 120; auxiliaries SC/TT/ET only when no combat line. | Yes |

### 2.4 Rules of engagement (spec 7.2.3)

| # | ec4x rule | ec4x text | NC status | NC detail | Fits? |
| --- | --- | --- | --- | --- | --- |
| R1 | 0-10 scale on AS ratio: 1 defenceless, 2 4:1, 3 3:1, 4 2:1, 5 3:2, 6 1:1, 7 2:3, 8 1:2, 9 1:3, 10 always | | SAME table | `exchange.rs rule_threshold_satisfied` is this table exactly. But NC applies it on two scales: classic's pre-battle strength `S = 21 BB + 7 CA + 2 DD (+30 guard-at-base)` for the *attack* decision, and battle AS `10/30/90 (+100 SB)` for *withdrawal*. Proportions almost agree (BB:CA:DD 10.5:3.5:1 vs 9:3:1); only the starbase differs (worth 1.4 BB in classic's test, 1.1 BB in NC's battle). | Yes |
| R2 | Retreat check every round, from round 1 | "6. Check Retreat (Per Fleet)" | SIMPLIFIED | NC-ONLY 3-round commitment: checks only from round 4, once before the volley (declared withdrawal) and once after it. | Yes |
| R3 | Homeworld defenders never retreat | "Fleets defending their homeworld NEVER retreat regardless of ROE or losses." | DROPPED | Analogues: a side that owns the world *and* has a live starbase never withdraws (`defending_starbase_holds_planet`); a guard or incumbent side ignores its first failed check (`free_hold_used`). | Yes (home world exists) |
| R4 | ROE never cancels an explicit Bombard/Invade/Attack | "your fleet executes regardless of ROE. ROE only matters for automated retreat decisions" | CONTRADICTED | Classic: an assault or colonize fleet whose ROE forbids fighting a guard (rule H3) or the protecting starbase aborts to Seek Home; first identification commits any ROE > 0, later re-checks need the strength test. Inside NC's battle ROE applies to assault fleets like any other. Bombardment runs no ROE check at all (ec4x: "Check Retreat (Attacker Only)"). | Yes |
| R5 | Per-fleet ROE: "Each fleet checks its own ROE independently and can retreat separately" | | SIMPLIFIED | Per side: ROE = max over the side's fleets that have combat ships; the whole side withdraws together (`apply_roe_retreat_to_task_force`). Classic: side 2 uses the target's ROE, forced to 10 while a base is alive on that side. | Yes |
| R6 | Manual variant: morale shifts effective ROE ±2 | | DROPPED | No prestige. | No |

### 2.5 Task force formation (spec 7.2.4)

| # | ec4x rule | ec4x text | NC status | NC detail | Fits? |
| --- | --- | --- | --- | --- | --- |
| F1 | All ships of all participating fleets + starbases (orbital) + colony fighters + unassigned ships form one task force per house | | CONTRADICTED | Classic (encounter spec §5.3, probe d4): "side A: the observer fleet only; side B: the target fleet, plus guarded_base_of(T) … No other fleets in the sector join (not allies, not other fleets of either empire)." `fight_encounter` always builds exactly two `TaskForce`s from `BattleSide{fleets,bases}`. | Yes |
| F2 | Screened units: mothballed ships, auxiliaries, facilities stay behind the line; destroyed if defenders eliminated | | SAME for SC/TT/ET | Auxiliaries take hits only when the combat line is gone (then they are shot at DS cost until the 64-round cap). Stardock contents are hit only by bombardment. No mothball/facility entities. | Yes |
| F3 | Several friendly fleets: fight together, retreat separately | | SIMPLIFIED | See R5 (side-level). Only arises when a guard fleet + base form side B. | Yes |
| F4 | Several houses: one task force per house, all hostile pairs engage | | CONTRADICTED | Two sides only. `fleet_battle.rs` carries multi-side plumbing (`engaged_opponents`, `hostile_target_priority`, ROE-ascending withdrawal evaluation) that is never exercised beyond two. | Yes |

### 2.6 Detection and intelligence (spec 7.3)

| # | ec4x rule | ec4x text | NC status | NC detail | Fits? |
| --- | --- | --- | --- | --- | --- |
| D1 | Raider detection roll-off: 1d10 + CLK vs 1d10 + ELI (+2 starbase) | | DROPPED | Replaced by classic sighting: per tick `p% = 10·awareness + vis·F`, awareness (0-3) shared empire-wide and reset when the target changes sector/layer; then an intercept roll (faster attacker always catches). This decides *whether* a battle happens, not who fires first. | No raiders/CLK/ELI |
| D2 | Ambush +4 / Surprise +3 DRM, winner applies hits first, first round only | | DROPPED | Manual: "There is no 'first strike' advantage." (Classic's own surprise branch is dead code.) | No |
| D3 | Starbase: +2 detection; +1 sensor-coordination DRM all rounds; lost if crippled | | SIMPLIFIED | Classic starbases scan hardest of all units (F = 25 in system, 30 in deep space) and share what they identify; NC gives +1 CRT column to any side with a starbase, every round, crippled or not. | Yes |
| D4 | Fresh detection check at orbital combat | | n/a | | — |

### 2.7 Combat round sequence and CER (spec 7.4.1)

| # | ec4x step | ec4x text | NC status | NC detail | Fits? |
| --- | --- | --- | --- | --- | --- |
| C1 | 1. Sum AS both sides | | SAME | 10× scale: DD 10, CA 30, BB 90, SB 100; crippled half. | Yes |
| C2 | 2. Die-roll modifiers | | replaced | Column shifts instead: mixed DD/CA/BB +1, starbase on the firing side +1, ground defender +1. | Yes |
| C3 | 3. 1d10 + DRM on CRT: 0-2 → 0.25×, 3-5 → 0.5×, 6+ → 1.0×, natural 9 = critical. Ground CRT 0.5/1.0/1.5/2.0 | | SIMPLIFIED (different shape) | d10 × force-ratio column (Disadvantaged <0.5, Pressed, Even, Advantaged, Overwhelming ≥3.0), multipliers 0.00-2.50 (`CRT_MULTIPLIER_X4`). Odds enter twice in NC (AS totals *and* column); in ec4x only through AS totals and the roll (max 1.0×). Ground combat uses the same table with defender +1 column, not a separate CRT. | Yes |
| C4 | 4. Hits = AS × CER, round up | | SAME | `div_ceil(4)` on the ×4 table. | Yes |
| C5 | 5. Apply hits (simultaneous unless ambush) | | SAME | Always simultaneous; hits split into suppression/execution pools by the firing side's AS share. | Yes |
| C6 | 6. Retreat check per fleet; **pursuit volley** at 0.5× CER by the victor; retreater moves to nearest friendly system; auxiliaries lose the same proportion as escorts; no escorts → all auxiliaries lost | "the victorious (non-retreating) task force gets one final, free attack … resolved at 0.5× CER" | SIMPLIFIED | Side-level withdrawal from round 4. The *withdrawal exchange* is mutual at the fixed Pressed column (the withdrawing side fires back) and replaces the normal volley between that pair. Retreat = Seek Home to the nearest owned planet by Manhattan distance, speed = max, mission target cleared, **ROE set to 0** (`retreat.rs apply_retreat_order`). Proportional auxiliary losses DROPPED (auxiliaries die only when the line is gone, or to a critical). Loaded transports are lost first and take their army. | Yes |
| C7 | 7. Winner = higher surviving AS; tie → defender | | SIMPLIFIED | Winner = the only side still engaged with combat AS and not withdrawn. If both stand at the 64-round cap: IncumbentDefender > GuardingDefender > Attacker > Neutral, then higher AS, then lower empire id. Loser is sent Seek Home and its assault/colonize missions abort; a side with a live starbase at its own world always holds. | Yes |
| C8 | Max 20 rounds; instant-stalemate detection → +2 DRM desperation round → forced mutual withdrawal | | SIMPLIFIED | 64-round guardrail, no desperation; survivors keep ships, post-battle rules apply. Classic has no cap at all. | Yes |
| C9 | Mutual withdrawal possible | | SAME | Both withdrawing → no winner, both Seek Home. | Yes |

### 2.8 Die-roll modifiers (spec 7.4.2-7.4.4)

| # | ec4x modifier | NC status | NC detail | Fits? |
| --- | --- | --- | --- | --- |
| M1 | Fighter superiority +1/+2 (recomputed each round) | DROPPED | | No |
| M2 | Starbase sensors +1 (defender, if undamaged) | SIMPLIFIED | +1 column for the side with a starbase, undamaged or not. | Yes |
| M3 | Morale ±1/±2 from prestige, first round | DROPPED | (`ec4x_assessment.md` §2.4 calls this a death spiral anyway.) | No |
| M4 | ELI advantage +1 | DROPPED | | No |
| M5 | Homeworld +1 (defender, space/orbital/invasion/blitz) | DROPPED | | Yes |
| M6 | Planet-Breaker +4, bypasses shields | DROPPED | | No |
| M7 | Prepared defences +2 (invasion); Landing under fire +3 (blitz) | SIMPLIFIED | Defender +1 column in both ground battles; blitz's extra cost is the landing fire on transports instead. | Yes |
| M8 | (none in 07-combat.md; `ec4x_assessment.md` §2.2 mentions a +1 mixed-fleet DRM) | NC-ONLY | Mixed DD/CA/BB side: +1 column. | Yes |

### 2.9 Space combat participants, resolution, victory (spec 7.5)

| # | ec4x rule | ec4x text | NC status | NC detail | Fits? |
| --- | --- | --- | --- | --- | --- |
| P1 | Engagement by diplomatic status × mission tier × travel/destination, with a one-turn grace period | "Tier 1 … immediate combat … Tier 2 … Neutral → Hostile, no combat … Turn X+1: Hostile → Combat" | CONTRADICTED | Classic triggers only: (E) observer's empire declared the owner an enemy (one-way); (N2) any alien fleet in one of my solar systems; (H1) my blockade vs a fleet arriving to bombard/invade/blitz, or to colonize an unowned world; (H2) colonize race; (H3) my assault/colonize fleet vs a guard blocking it. No Hostile state, no tiers, no grace period, no travel-vs-destination rule beyond the layer. | EC has only enemy declarations |
| P2 | Who does not fight: Hold, Guard, Reserve, Mothballed, Starbases, Scouts | | CONTRADICTED | Hold fleets fight when attacked; guards fight (see T2); classic Scout orders (10/11) identify and *decide*, so a ROE > 0 scout fleet attacks; starbases never start a fight but join as partner. Non-combat fleets are forced to ROE 0. | Yes |
| P3 | Auxiliaries screened; "An unescorted auxiliary fleet would be immediately destroyed" | | SIMPLIFIED | Auxiliaries are shot at DS cost (10/10/20) once no combat line exists; an unescorted TT/ET/SC fleet cannot withdraw (`our_as == 0` skips the ROE path) and is ground down until gone or the 64-round cap. | Yes |
| P4 | Victory: attacker / defender / mutual withdrawal; loser's mission fails | | SAME in spirit | Field holder = `dominant_empire`; others Seek Home, assault missions abort (`MissionOutcome::Aborted`). | Yes |

### 2.10 Orbital combat and starbase bonuses (spec 7.6)

| # | ec4x rule | ec4x text | NC status | NC detail | Fits? |
| --- | --- | --- | --- | --- | --- |
| O1 | Orbital participants (guards, reserves, starbases, unassigned, colony fighters); screened mothballed/aux/facilities | | see T3, F1 | | |
| O2 | Starbases fight directly, cumulative AS/DS, can be crippled to 50 %, cannot retreat | "Cannot retreat—fight to destruction or victory" | SIMPLIFIED | SB AS 100 / DS 120, execution fire. The *whole* side with a live SB at its own world never withdraws (ec4x: fleets beside the base still may). SB cripple is in-battle only; destroyed bases are unlinked (`remove_destroyed_starbases`). | Yes |
| O3 | Screened units destroyed when defenders eliminated; spaceports survive, destroyed only by bombardment/landing | | SIMPLIFIED | Auxiliaries are then shot down by attrition (see P3). Stardock contents (EC's "spaceport") are touched only by bombardment, matching ec4x's spaceport rule. | Yes |
| O4 | Starbase bonuses: +2 detect, +1 DRM (max one), direct AS/DS; all lost if crippled | | SIMPLIFIED | +1 column while any SB unit remains, plus classic's starbase sighting factor. | Yes |
| O5 | Orbital victory = supremacy → planetary phase | | SIMPLIFIED | Assault-time supremacy is an AS comparison among assault entrants (tie → owner if present, else Blocked); the owner's starbase is counted only when no entrant has combat AS (§6.1). | Yes |

### 2.11 Bombardment (spec 7.7)

| # | ec4x rule | ec4x text | NC status | NC detail | Fits? |
| --- | --- | --- | --- | --- | --- |
| B1 | Requirements: orbital supremacy, AS > 0, Bombard command | | SAME | Order validation `MissingCombatShips`; supremacy comparison. | Yes |
| B2 | Participants: all combat ships; Planet-Breakers +4 and shield bypass | | SIMPLIFIED | Bombard AS = DD 0.5×, CA 1.0×, BB 1.5× of fleet AS (5/30/135), crippled halved; no PB. | Yes |
| B3 | Resolution: fleet AS vs battery AS, space CRT (max 1.0×), shields cut attacker hits, hits → batteries, then spaceports → ground forces → 50/50 infra/population; attacker ROE check; 3 rounds per turn | "Bombardment is limited to **3 rounds per turn**" | SIMPLIFIED | 3 rounds per turn. Rounds 1-2 suppression: attacker hits go **stardock first, then batteries**; armies, goods, factories shielded. Round 3 breakthrough only if batteries were 0 at round start: stardock → batteries → armies → goods → factories (goods/factories at hits ÷ 10, then 100 goods + 1 factory per point). Batteries fire back every round as execution fire (90 AS each, 20 DS). Ratio-column CRT with mixed +1. No ROE check. Critical vs the planet = +1 hit (no effect at 20 per battery). Order differs from ec4x (batteries before spaceports). | Yes |
| B4 | Planetary shields: always on, 25-50 % reduction, never destroyed by bombardment, destroyed when marines land | | DROPPED | | No |
| B5 | Batteries have AS/DS, cripple then destroy, all must be destroyed before standard invasion | | SIMPLIFIED | Battery AS 90 / DS 20; no cripple state (gone at 20 hits). Invasion clears batteries with its own single suppression exchange (I1). | Yes |
| B6 | Infrastructure damage permanent until repaired at 25 % cost; population loss | | SIMPLIFIED | Factories (present production points) and stored goods destroyed; no population; recovery through normal growth. | Yes (EC has no population stat) |
| B7 | Engine doc: >50 % infrastructure damage clears colony queues | | n/a | Stardock items are destroyed directly by bombardment hits. | — |

### 2.12 Standard invasion (spec 7.8.1)

| # | ec4x rule | ec4x text | NC status | NC detail | Fits? |
| --- | --- | --- | --- | --- | --- |
| I1 | Requirements: supremacy, **all batteries already destroyed**, loaded transports, Invade command | "ALL ground batteries destroyed (mandatory—batteries fire on landing transports)" | SIMPLIFIED | Supremacy; combat ships *and* loaded TT required (validation). Batteries need not be pre-cleared: one suppression exchange (bombard AS vs batteries × 90, batteries return execution fire). Any surviving battery → `Aborted`, armies stay aboard. (Manual variant 7.6.1 of ec4x also has this one-round form.) | Yes |
| I2 | Marines land → shields and spaceports destroyed | | n/a | | — |
| I3 | Marine AS vs ground AS; defender +2 (+1 homeworld); ground CRT 0.5-2.0; simultaneous; cripple rules; rounds until one side gone; no retreat | | SIMPLIFIED | NC-ONLY softening step first: one ground-CRT exchange of bombard AS vs armies × 10, armies only, capped at half the starting garrison. Then armies vs armies (one army per hit, +1 on a critical), defender +1 column, simultaneous, no cripple state, no retreat, 64-round cap, no-draw rule (both annihilated → larger entering force keeps one army; ties defender). | Yes |
| I4 | Attackers win: capture, **50 % of remaining IU destroyed**, survivors garrison. Defenders win: all marines destroyed | | SIMPLIFIED | Capture → survivors garrison, surviving batteries transfer, goods/factories untouched (EC conquered worlds convert over ~2 turns, economy chapter). Lose → landed armies gone. Either way the fleet holds in orbit. | Yes |

### 2.13 Blitz (spec 7.8.2)

| # | ec4x rule | ec4x text | NC status | NC detail | Fits? |
| --- | --- | --- | --- | --- | --- |
| Z1 | Requirements: supremacy, loaded transports, Blitz command; no battery requirement | | SAME + NC-ONLY | A loaded-TT-only fleet may blitz with no escorts if no enemy combat force or defending starbase holds orbit. | Yes |
| Z2 | Phase 1: one bombardment round; batteries fire at **all** ships including transports; shields apply; no infrastructure targeted | | SIMPLIFIED | Cover exchange: escorts' bombard AS vs batteries (ground CRT), paying 20 per battery, nothing else damaged. Then landing fire: surviving batteries × 90 vs escorts (space CRT), focus fire on TT first (20 hits each, army lost), then DD/CA/BB; critical +1 ship. Harsher than ec4x (transports first, not random). | Yes |
| Z3 | Phase 2: land at once; shields/spaceports/batteries seized intact; batteries silent during ground combat | | SAME | Surviving batteries transfer on capture; no fire during the ground battle. | Yes |
| Z4 | Phase 3: ground combat, defender +3 Landing Under Fire | | SIMPLIFIED | Defender +1 column, same as invasion. | Yes |
| Z5 | Win: 0 % IU destroyed (vs 50 % for invasion) | | DROPPED distinction | NC invasion also preserves goods and factories, so blitz vs invade differs only by the landing fire and the skipped suppression/softening. | Yes |

### 2.14 Multi-house combat (spec 7.9)

| # | ec4x rule | ec4x text | NC status | NC detail | Fits? |
| --- | --- | --- | --- | --- | --- |
| X1 | Hostility from the diplomatic ladder and mission tiers | | CONTRADICTED | See P1. | EC has only enemy declarations |
| X2 | Identify hostile pairs, allocate AS proportionally, resolve each pair independently | "House H takes casualties from BOTH battles" | CONTRADICTED | One observer attacks one target per tick; "A second target is fought only in a later tick." A fleet can fight several times a year, losing each attacking tick's movement. | Yes |
| X3 | Retreat priority: weakest first, ties by house id, re-check after each, may cancel | | SIMPLIFIED | Post-round withdrawals evaluated in ascending (ROE, empire) order; each withdrawal is marked before the next check so later evaluators see reduced enemy AS; no cancel. Moot with two sides. | Yes |

### 2.15 NC-only rules with no ec4x counterpart

- Suppression vs execution fire split by hull (CA/BB disperse; DD/SB/batteries focus-fire at 2× DS).
- 10× internal combat scale.
- 3-round commitment before any ROE withdrawal.
- Mutual withdrawal exchange at a fixed Pressed column (replaces the pursuit volley).
- Retreat sets the fleet's ROE to 0.
- Mixed DD/CA/BB column shift.
- One assault class per planet per turn (Blitz > Invade > Bombard); planet changes owner at most once per turn.
- TT-only blitz exception.
- Invasion softening capped at half the garrison; ground battle no-draw rule.
- Stardock contents are hit before batteries.
- Non-combat fleets forced to ROE 0 (classic dispatcher rule).
- Classic's sighting/awareness/intercept loop decides contact; battles happen mid-year at a tick and cost the attacker that tick's movement.

## 3. What NC dropped that it should restore

Each item: ec4x rule, what NC has, what restoring it means on EC entities, and what it collides with.

### 3.1 Retreat with pursuit volley and move to the nearest friendly system

- ec4x: per fleet, every round: "If below threshold: Fleet retreats … the victorious (non-retreating)
  task force gets one final, free attack against them … at 0.5× CER … Retreating fleet then moves to
  the nearest friendly system." Auxiliaries lose the escort loss ratio; no escorts → all auxiliaries lost.
- NC has: the destination (Seek Home to nearest owned planet) and a *mutual* withdrawal exchange at the
  Pressed column from round 4, plus a 3-round commitment. Missing: the one-sided free volley, the
  auxiliary attrition on retreat, per-fleet retreat, and ROE from round 1.
- Restore as: drop the 3-round commitment (ec4x needs none because the first-round detection advantage
  and cripple-first protection already guarantee contact is costly); make the withdrawal exchange
  one-sided (victor fires at 0.5× = NC's Pressed column is already roughly that; just stop the
  withdrawer's reply); apply the escort-loss ratio to SC/TT/ET on withdrawal (loaded TT first, army
  lost with it; the code in `distribute_fleet_losses` already orders losses that way); stop zeroing ROE
  on retreat (`apply_retreat_order`), which classic does only for a *scatter* and ec4x never does.
- Collides with: nothing in the classic encounter loop (classic's battle is superseded by ADR b5u6lekb).
  One classic fact worth keeping: a slower fleet cannot escape a faster pursuer (intercept rule, and
  classic's in-battle range/speed retreat). Option: allow withdrawal only if the withdrawer's max speed
  ≥ the pursuer's, else it fights on; the data (`+0x09` max speed) is already in the record.

### 3.2 Crippled persistence and repair

- ec4x: crippled state persists; 50 % AS/DS, 50 % maintenance, no restricted lanes, drydock repair in
  1 turn for 25 % of build cost; a crippled starbase loses its sensor bonuses.
- NC has: in-battle cripple only; "surviving crippled ships are repaired automatically after battle";
  a crippled starbase keeps its +1 column.
- Restore as: carry per-class crippled counts on the fleet (and a crippled flag on the base) in the typed
  game state t039 is introducing ("classic byte records only at the .DAT import/export edge"); on export
  to FLEETS.DAT crippled ships are written as whole hulls (classic cannot represent them, so the compat
  edge loses the flag, which is acceptable and should be stated in the manual). Repair: EC has no drydock,
  so repair at any owned world's orbit at the start of maintenance, paid from that planet's treasury at
  25 % of build cost (or free after one full year in orbit, if a cost rule is unwanted). Crippled ships
  should also halve bombard AS (already done in-battle) and lose the starbase column bonus.
- Collides with: the manual's current "repaired automatically after battle" sentence; the Appendix B unit
  table (needs a crippled column or the 50 % note); the encounter loop's strength cache `S = 21BB+7CA+2DD`
  used for sighting odds and ROE tests — a persisted cripple should halve the ship's contribution there
  too, or the attack decision and the battle will disagree about odds.

### 3.3 A single AS-ratio ROE

- ec4x: one ROE table, one quantity (total AS vs enemy total AS), used only for retreat.
- NC has: the same table, but applied on two scales (classic `S` for the attack/avoid decision, battle AS
  for withdrawal) and two meanings (classic's "attack at first identification regardless of ROE > 0;
  later re-checks need the odds"; ec4x's "ROE only for retreat").
- Restore as: keep classic's *decision* rule (it is the oracle and ADR gtkmxw3l accepts it), but compute
  its strength from the same unit table the battle uses (DD 10 / CA 30 / BB 90 / SB 100, crippled half)
  so the odds the fleet sees before and inside the fight are the same number. The proportions already
  match within rounding except the starbase (classic +30 ≈ 1.4 BB; NC SB 100 ≈ 1.1 BB), so classic's
  probe-confirmed odds barely move; the ADR says Rust matches classic's *odds*, and this changes them only
  where a base is involved. If that is too far from the oracle, at least document the two scales side by
  side in Appendix B.
- Collides with: the classic strength formula `24a3:02b4` that NC matches today, and the "ROE does not
  affect explicit commands" line, which cannot be restored: classic aborts an assault whose ROE forbids
  the guard/base fight (rules H3 and the starbase decision), and that is probe-confirmed.

### 3.4 Invasion suppression rounds

- ec4x: invasion is only possible once *all* batteries are destroyed by prior bombardment turns; the
  landing itself has no orbital exchange. (The ec4x manual's older form runs one bombardment round and
  fails if batteries remain, which is what NC does.)
- NC has: one suppression exchange inside the Invade turn; with batteries at 20 hits each, a modest fleet
  rarely clears a fortified world in one exchange, so the invasion aborts and the armies stay aboard.
- Restore as: run the invasion's orbital stage as the same three-round sequence bombardment uses
  (rounds 1-2 suppression, round 3 breakthrough only if batteries are gone), then soften and land in the
  same turn if the batteries were cleared. This matches the 1992 manual's three stages ("Destroy ground
  batteries. Soften resistance with orbital fire. Land armies.") and reuses `assault.rs` bombardment code;
  or, ec4x-style, require batteries = 0 at the start of the Invade turn and leave clearing to Bombard.
- Collides with: ADR b5u6lekb's "invasion and blitz resolve, win or lose, after which the fleet holds";
  with a hard prerequisite an Invade fleet facing batteries must either auto-bombard or hold and report,
  not retry silently.

### 3.5 Task forces

- ec4x: "All ships from participating fleets" of a house fight as one task force; each fleet keeps its own
  ROE for retreat; starbases and unassigned ships join in orbit.
- NC has: the `TaskForce` type per side, but sides are classic's (one attacker fleet; target + one partner).
- Restore as: see §4, option A. Minimal form: when classic's loop produces `battle(O, T)`, side A = O plus
  every fleet of O's empire in the same sector and layer whose ROE > 0 (Guard Starbase fleets only if
  their base is here); side B = T plus every unit of T's empire there (fleets, the starbase, its guard).
  Raise awareness to 3 for every joined unit both ways so later observers this tick skip the pair. Each
  fleet keeps its own ROE for withdrawal; the side's attack decision stays classic's (observer's ROE).
- Collides with: ADR gtkmxw3l and encounter spec §5.3 (probe d4: no other fleets join). Needs a new ADR
  superseding that one point; classic's sides rule is "who fights", which the ADR explicitly covers.

### 3.6 Theaters

- ec4x: Space → Orbital → Planetary, each won before the next, with distinct participant sets and bonuses.
- NC has: encounter battles wherever classic pairs units, then an AS comparison for "orbital supremacy",
  then the assault.
- Restore as: EC's depth layers give the first split for free: a battle in layer 0 is "space"; a battle
  in layer 1 at a world owned by one side is "orbital" (starbase +1 column, defenders never withdraw on
  their first failed check, screened units exposed when the line falls). The missing piece is the gate:
  at the strike tick, before `select_orbital_supremacy_empire`, fight an orbital battle between the
  assault fleets of the strongest attacker and *all* defenders at the world (guard fleets, holding fleets,
  the starbase), and let only the field holder assault. That replaces today's silent AS comparison (which
  classic does not have either; classic's gate is a real battle against the base, battle spec §2.2) and
  closes the starbase gap in §6.1.
- Collides with: the tick model (the orbital fight happens at the strike tick, not in a conflict phase);
  classic's "one attack per observer per tick", which the strike-time battle would sit outside of, as
  classic's own assault battles do.

## 4. Where the classic encounter loop and ec4x combat conflict

NC's 2.5D movement is taken as given and as a design asset here. From
`docs/dev/ec-movement-spec.md` "Classic Movement Semantics": every fleet and starbase has a depth
`Z` in 0..1 (deep space 0, orbit 1; layer = `Z >= 0.5`); a fleet climbs out of orbit, flies straight
across the map to the target sector's centre, then descends; units meet only in the same sector *and*
layer; Bombard/Invade/Blitz strike on the tick `Z` reaches 1.0; a starbase whose world is lost lifts to
deep space (speed 1) and its guard rides along. Awareness of a unit is reset for everyone when it
changes sector or layer, and a fleet at its assault target in the system layer is ×5 (in system) × ×20
(assault order) more visible than in deep space.

### 4.1 The conflicts

1. **Contact is probabilistic and serial in classic; deterministic and simultaneous in ec4x.** Classic:
   each tick, each unit in a yearly random order rolls to sight each alien unit in its sector + layer
   (`p = 10·awareness + vis·F`), identifies, decides, rolls to intercept, and fights at most one target;
   the attacker loses that tick's movement. ec4x: in the Conflict phase every hostile pair in a system
   fights, once. Consequence: in classic co-located enemies can pass a whole year without a battle (a Hold
   fleet sighting a lone CA in deep space: ~1 % per tick), and one fleet can be attacked by several
   observers in one tick or over the year.
2. **Sides.** Classic: exactly two, "the observer fleet only" vs "the target fleet, plus guarded_base_of(T)".
   ec4x: all friendly units in the system per house, several houses at once.
3. **Triggers.** Classic: one-way enemy declaration; any alien in my solar system; blockade vs assault or
   colonizer; colonize race; assault vs guard. ec4x: Enemy/Hostile/Neutral/NAP × mission tiers × grace
   period. EC has no Hostile state or grace timer to put the ladder on.
4. **Guards.** ec4x: Guard fleets never fight in the space theater. Classic: Guard Starbase fleets do not
   close but attack identified targets on ROE and defend with their base; Guard/Blockade fleets close
   aggressively and attack arrivals. (The depth layer makes classic's guard "orbital only" anyway: a guard
   at depth 1 cannot meet a fleet passing at depth 0.)
5. **ROE semantics.** ec4x: retreat only; explicit commands ignore ROE. Classic: ROE decides *attacking*
   (first identification commits any ROE > 0; later checks need the odds) and makes an assault fleet
   abort to Seek Home when the odds against a guard or base fail it; the in-battle retreat is a separate
   per-round `roe_result` with retreat/scatter and a range/speed model. NC keeps classic's first half and
   its own second half.
6. **Timing of theaters.** ec4x runs the three theaters back to back in one phase. Under the tick loop the
   "space" and "orbital" fights happen on whatever tick a sighting succeeds, possibly ticks before the
   strike, possibly never (if the base is never sighted: §6.1), and the assault fires on the strike tick.
   §4.2 turns this from a conflict into the design.
7. **Starbases.** Compatible in spirit (never start a fight; fight as the defender's partner; sensor
   edge). Difference: classic's base joins only when its guard fleet is the target or the base is; ec4x
   adds the base to every orbital defence.
8. **Speed and escape.** Classic encodes speed twice (intercept roll; in-battle retreat succeeds only when
   out of range and at least as fast). ec4x/NC: any fleet may withdraw at the cost of a volley. So a fleet
   that was caught *because* it was slower can still walk away in NC round 4.

### 4.2 The depth axis as the given: ec4x's three theaters on 2.5D

| ec4x theater | 2.5D location | Who is there | How you get in / out |
| --- | --- | --- | --- |
| Space combat | layer 0 of the target sector (`Z < 0.5`): fleets crossing, patrols (`TZ = 0`), fleets climbing out, a lifted starbase and its guard | mobile fleets, scouts, patrols; a base only after its world was lost | fly in across the map; leave by flying on or descending |
| Orbital combat | layer 1 (`Z >= 0.5`) at an owned world: the descent from 0.5 to 1.0 and orbit itself | starbase, Guard Starbase fleets (ride at the base's depth), Guard/Blockade fleets (`Z = 1`), Hold fleets in orbit, bombarding fleets | descend from the sector centre: `26 / speed` ticks from 0.5 to 1.0 (BB 4 → 6.5 ticks, DD 6 → 4.3); leave by climbing |
| Planetary combat | the strike tick: `Z = 1.0` on Bombard / Invade / Blitz, then each year in orbit for Bombard | assault fleets vs batteries, armies, stardock | arrive at `Z = 1.0`; after Invade/Blitz the fleet holds in orbit |

**What the depth axis makes natural**

- *Retreat = climb out, and the pursuit volley comes for free.* A fleet that fails its ROE gets Seek Home
  (`TZ = 1` at the nearest owned world); the step rule climbs it to `Z = 0` first, so it spends
  `26 / speed` ticks still in layer 1 under the victor's sensors at awareness 3 (`p = 30 % + vis·F`,
  near-certain). Whether the victor gets the parting shots is then classic's intercept rule: a faster
  pursuer always catches it; an equal or slower one needs a 1 % roll or the loser's own ROE to accept.
  That is ec4x's "victorious task force gets one final, free attack" *and* classic's "a retreating side
  escapes only if it is at least as fast", with no synthetic volley. Crossing `Z = 0.5` resets awareness
  and breaks the pairing: contact is lost, as ec4x's "retreating fleet then moves to the nearest friendly
  system" intends.
- *Theater gating = layer gating.* Nothing reaches the planet without spending ticks in layer 1, and an
  assault fleet there at its target is ×100 as visible as in deep space (×5 system × ×20 assault order).
  Ten BB descending: `vis ≈ 57`, so a starbase (F 25) or blockade (F 5) sights it on the first tick with
  certainty and even a Hold fleet (F 1) at 57 %/tick. So "must win orbital before planetary" is enforced
  for assault orders by the sensors themselves, and the orbital fight happens *during the descent*, not in
  a separate phase. The defenders' advantage (ec4x "fight from prepared positions") is positional: they
  are already at depth, the attacker is still descending.
- *A fresh detection check at orbital combat* (ec4x 7.6.2, "Raider fleets … have a chance to re-cloak")
  is the awareness reset on the layer change from 0 to 1: a fleet that fought in deep space must be
  re-sighted in the system. No raider mechanic needed.
- *Blockade = being in layer 1.* Guard/Blockade arrives at `Z = 1`; at a foreign world that puts the fleet
  in the owner's solar system, where rule N2 lets the owner's orbit fleets attack it; at home it attacks
  assault and colonize arrivals (H1). ec4x's "Blockade is a Tier 1 colony attack" needs no diplomatic
  state: position is the declaration.
- *Orbital defeat does not wipe the screened units; the base lifts.* Where ec4x destroys mothballed ships
  and facilities when the orbital defenders fall, EC's base survives a lost world by climbing to deep
  space with its guard. The orbital garrison becomes a layer-0 force the next tick, so a captured world
  can be re-contested from above. Stardock contents, the only "facility", are hit by bombardment only
  (ec4x's spaceport rule).
- *Bombardment persistence is a siege that must be held.* A bombarding fleet sits at `Z = 1` with
  visibility ×20; any relief fleet descending into layer 1 sights it at once and rule N2 lets it attack.
  The bombarder must keep winning in orbit each year to keep firing, which is ec4x's "check ROE between
  turns and may withdraw" without a separate rule.
- *Speed asymmetry is real.* Hold fleets at speed 0 cannot run (`p = 101` intercept); a slow BB line
  cannot shake a DD screen. ec4x has no speed model; 2.5D plus the intercept rule supplies one.

**What the depth axis makes awkward**

- *Task forces when friendly fleets are spread across ticks and layers.* ec4x assumes a discrete system
  where "all ships from participating fleets" are present at once. In 2.5D two friendly fleets heading for
  the same world are at different depths on any given tick (one at `Z = 0.7`, one at `0.3`, one still a
  sector out), so a task force can only be "every friendly unit in this sector and layer at this tick".
  A fleet one tick behind, or one layer up, is a bystander, and a guard whose base has lifted is in the
  wrong layer. Players will see a battle their second fleet "should" have joined. The honest rule is the
  tick-and-layer one; the manual must say it.
- *Sighting as the gate between theaters.* The orbital gate is a sighting roll, not a check. It is
  watertight for assault orders (×20) but leaks for others: a lone DD on Hold descending unseen past a
  starbase has `vis ≈ 0.6` (speed 6), `p ≈ 15 %/tick`, so over a 4-5 tick descent it reaches orbit unchallenged
  about half the time; a Colonize fleet (×2) and a View fleet (×1.2) likewise. ec4x's "no theater
  skipping" therefore holds only for Bombard/Invade/Blitz, and only probabilistically for the space
  theater (a 10-BB fleet crossing a patrolled sector is sighted by a Patrol with ~6 %/tick over ~13
  ticks: roughly even odds). That is a feature for scouts and colonists and a leak for everything else,
  and the strike tick needs a deterministic backstop (§4.3 step 5).
- *A battle is a tick, not a phase.* One ec4x battle is many rounds in one phase; in 2.5D the same two
  forces can fight again on the next tick if the intercept gate opens, so attrition stretches over ticks
  and the attacker (the observer) loses a tick's movement each time. With joined task forces every fleet
  on the attacking side should lose that tick's movement, not only the observer, or fleets will leapfrog
  their own battle.
- *Who is "defender" in layer 1.* ec4x's orbital bonuses belong to the colony owner. In 2.5D the owner's
  fleets, a neutral blockader, and two rival assault fleets can all sit in layer 1; the bonus must follow
  world ownership, not presence (NC's `BattleRole::IncumbentDefender` already does this).
- *Starbase in deep space.* A lifted base is a layer-0 unit fighting as a partner with no world beneath
  it; it should keep its AS/DS but lose the "fortified" column shift, which ec4x never has to decide.

### 4.3 A concrete NC combat sequence on 2.5D (proposal)

Per tick, unchanged: step → encounter → dispatch, units in the yearly random order. Changes are confined
to what happens once classic has decided that `battle(O, T)` starts, plus one deterministic gate at the
strike tick.

1. **Sides at the tick.** Side A = `O` plus every fleet of `O`'s empire in the same sector and layer with
   ROE > 0 (a Guard Starbase fleet only if its base is in this layer). Side B = `T` plus every fleet and
   starbase of `T`'s empire in the sector and layer. Raise awareness to 3 both ways for every joined unit.
   Every fleet on side A loses this tick's movement (classic charges the observer; the join extends it).
   Each fleet keeps its own ROE. (Keeping classic's two sides instead is option B in §4.4; the rest of the
   sequence is the same.)
2. **Theater by layer and ownership.** Layer 0, or layer 1 at a world neither side owns: *space* rules.
   Layer 1 at a world one side owns: *orbital* rules for that side: starbase +1 column while an undamaged
   base is present; the owner's fleets ignore their first failed ROE check; a side with a live base never
   withdraws; a lifted base in layer 0 fights at full AS/DS without the column shift.
3. **Rounds.** Simultaneous ec4x rounds: sum AS (crippled ½), ratio column + shifts, d10, hits = ⌈AS × m⌉;
   cripple-first with execution fire as now; natural 9 = one extra loss on the *highest*-DS unit still
   in the line (fix §6.2); cascading overkill when a side's hits exceed 1.5× the enemy's total DS. ROE is
   checked per fleet after every round from round 1 (drop the 3-round commitment: the first round is
   already compulsory, and the climb below replaces the withdrawal exchange). A battle ends when a side
   has no combat line, every remaining fleet of one side has withdrawn, or 64 rounds.
4. **Withdrawal = climb.** A fleet that fails ROE is set Seek Home (nearest owned world, `TZ = 1`, speed
   = max, mission target cleared, **ROE unchanged**) and leaves the battle; auxiliaries aboard the
   withdrawing fleet take the escort loss ratio (loaded TT first, army lost with them). No withdrawal
   exchange. Next tick the fleet is still in the layer; the victor re-sights it at awareness 3 and the
   intercept rule decides pursuit: faster victor → another tick battle under space/orbital rules; equal
   or slower → the loser gets away unless its own ROE accepts the odds. A fleet at speed 0 is always
   caught on the first pursuit tick. Crossing `Z = 0.5` resets awareness and ends the chase.
5. **Strike tick (deterministic orbital gate).** When an assault fleet reaches `Z = 1.0`, before
   `select_orbital_supremacy_empire`: if the world's owner has a starbase or any fleet in layer 1 that this
   attacker has not already beaten this tick, fight the orbital battle now, sides as in step 1 (all of the
   attacker's assault fleets at `Z = 1.0` vs all owner units in layer 1). Only the field holder assaults;
   a beaten or ROE-forbidden attacker aborts to Seek Home (classic H3 / starbase rule). This replaces the
   silent AS comparison, closes the unsighted-base gap (§6.1), and is where classic's own base gate sits
   (battle spec §2.2). Rival attackers of different empires at `Z = 1.0` on the same tick keep today's
   comparison (strongest, tie → owner) to decide who strikes; declared enemies among them will already
   have fought on earlier ticks.
6. **Assault.** Bombard: three rounds as now, but run the Invade orbital stage as the same three-round
   sequence (§3.4) and land only if batteries reach zero; Blitz as now (cover, landing fire on TT first,
   ground battle). Ground battle: defender +1 column (+1 more at the home world, ec4x's homeworld DRM).
   Bombard keeps its order; Invade/Blitz resolve and the fleet holds in orbit (ADR b5u6lekb).
7. **After capture.** The old owner's base lifts with its guard (classic); the new owner's fleets in
   layer 1 become the incumbent defenders for the rest of the year; the world changes hands at most once
   per year.
8. **Persisted damage.** Crippled counts per class survive the battle in the typed state (t039); a
   crippled starbase gives no column shift; crippled hulls repair at an owned world's orbit at the start
   of maintenance for 25 % of build cost from that world's treasury; the .DAT export writes them as whole
   hulls and the manual says so. The encounter loop's strength `S` halves a crippled ship's contribution
   so the attack decision and the battle agree about odds.

What this keeps from classic untouched: the tick loop, sighting odds, awareness, the decision rules
(enemy, my system, blockade, colonize race, assault vs guard, base), the intercept roll, the strike tick,
the starbase lift, mission aborts, and the two-side rule if option B is chosen. What it takes from ec4x:
per-fleet ROE every round, cripple persistence, criticals on high-value targets, cascading overkill,
pursuit of a retreating force, orbital bonuses tied to the owner, the homeworld edge, and a real orbital
fight before any assault. What stays NC: split fire, the 10× scale, the ratio-column CRT, the three-round
bombardment, invasion softening, blitz landing fire, and one assault class per world per year.

### 4.4 Reconciliation options

**Option A — classic decides contact, then task forces and theaters run on the depth axis** (§4.3 as
written, with joined sides). Cost: a new ADR superseding the "exactly two sides" clause of gtkmxw3l; the
encounter-spec probe for sides (d4) no longer applies; multi-house battles stay two-sided per tick (a
third empire's fleet present is a bystander unless its own roll pairs it). Keeps every classic sighting and
intercept odd intact.

**Option B — keep classic sides; put the theaters only inside the strike-tick gate.** §4.3 with step 1
reduced to classic's two sides for ordinary encounters, and step 5 (all assault fleets vs all layer-1
defenders) as the only joined battle. Smallest deviation from the oracle: classic's assault also fights a
real battle at the strike, and classic has no supremacy comparison to preserve. Does not restore task
forces for fleet encounters.

**Option C — ec4x's conflict-phase model (fleets co-located at year end fight).** Already rejected by ADR
gtkmxw3l ("would engage every allowed encounter in the year it arises and miss fleets caught mid-year");
it would also throw away the depth axis, which does most of ec4x's theater work for free.

**Option D — accept two sides, document ec4x 7.2.4/7.9 as out of scope.** Keep NC as is and restore only
the within-battle items (§3.1-3.3). Cheapest; leaves the §6.1 starbase gap and the silent supremacy
comparison in place.

Recommendation implied by the data: B now (it fixes the gate and costs no ADR), A when the manual's
"Who Fights Whom" is next rewritten; both change only what happens *after* classic has decided that a
fight starts, and both read the depth axis as the theater model rather than fighting it.

## 5. ec4x internal divergences worth knowing before adopting its text

- Spec 7.4.1 has the pursuit volley; manual 7.3.6 says "Victorious fleet does NOT automatically pursue".
  The spec is the 2026 rule; the manual's §7 and `architecture/combat-engine.md` describe the older
  squadron model (phases Ambush → Fighters → Capitals, CER with a 0.75 step, morale shifting ROE, weighted
  random targeting). `engine/combat.md` says "Morale (prestige-based) modifies effective ROE (-2 to +2)"
  while spec 7.4.4 says "Morale does NOT modify Rules of Engagement". Take the spec.
- Spec 7.7.3 bombardment uses the space CRT (max 1.0×); the manual's bombardment table is the same shape.
  Ground CRT (0.5-2.0×) is for invasion/blitz only.
- `ec4x_assessment.md` §2.4 flags the morale DRM as a positive-feedback death spiral; §2.2 flags flat
  hull-count fleet caps favouring doomstacks (NC's 3,000-ship fleets have the same exposure; cascading
  overkill and execution fire are the only brakes).

## 6. Code observations to verify (not requested, found while reading)

1. **Starbase gate at the strike.** Classic runs `decide(fleet, base, mode 2)` in the assault arrival test
   regardless of sighting (battle spec §2.2). NC's only gate is the encounter loop's `decide_vs_base`
   (`encounter/rules.rs:322`), which needs a successful sighting roll first, and
   `assault.rs select_orbital_supremacy_empire` counts the owner's starbase only when no entrant has
   combat AS. An Invade fleet that never rolled a sighting of the base appears to assault straight past it.
2. **Critical hits pierce the screen.** `state.rs fleet_target_order()` = DD, SC, TT, ET, CA, BB, SB, so a
   natural 9 kills a scout or transport before a cruiser while the combat line still stands, contrary to
   the "Auxiliaries … are screened until the combat line collapses" rule.
3. **Every retreat zeroes ROE.** `retreat.rs apply_retreat_order` sets ROE 0; classic does this only for a
   scatter, ec4x never. A fleet that withdrew at ROE 6 arrives home at ROE 0 and will not defend it.
4. **Stale retry comment.** `assault.rs process_planetary_assaults` doc comment still describes the yearly
   retry ("so it strikes again next year"; "Invade: Aborted … is the failed attack"), and the manual's
   "Hostile (Delayed-Resolution) Missions" admonition says "attacks again each year". ADR b5u6lekb and open
   task t036 remove that.
5. **Critical vs planet is a no-op.** `scalar_hits_with_critical` adds 1 hit on a 20-per-battery scale.
   ec4x says criticals do not apply to ground targets; same outcome, by accident. Either drop it or make it
   one extra unit as in fleet combat.
6. **Starbase column bonus ignores cripple.** `has_starbase_column_bonus` tests `counts[IDX_SB] > 0`; a
   crippled base still gives +1 (ec4x removes sensor bonuses when crippled).
