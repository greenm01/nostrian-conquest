# EC Encounter Spec

The classic ECMAINT per-tick encounter loop: which units can meet, how they
sight, identify, and intercept each other, and when they fight. Rust matches
this rule and its odds (see the
[engagement ADR](../notes/decisions/gtkmxw3l-engagement-follows-the-classic-ecmaint-encounter-rule.md)).
It was read from the unpacked ECMAINT (seg:off maps to file offset
`seg*16+off-0x8140` in `tools/unlzexe/ecmaint_640k.bin`) and checked with
classic probes. "[P]" marks a rule confirmed by a probe, "[C]" one read from
code only. Movement and depth are in [ec-movement-spec.md](ec-movement-spec.md).
The battle that follows an attack is in [ec-battle-spec.md](ec-battle-spec.md);
[ec-combat-reference.md](ec-combat-reference.md) summarises the whole system.

Notation. Fleet record bytes are FLEETS.DAT offsets: `+0x09` max speed,
`+0x0a` current speed, `+0x0b/+0x0c` sector x,y, `+0x19` Z (real48),
`+0x1f` order, `+0x20/+0x21` target x,y, `+0x22` TZ, `+0x23` mission id (u16),
`+0x25` ROE, `+0x26` BB, `+0x28` CA, `+0x2a` DD, `+0x2c` TT empty,
`+0x2e` TT loaded, `+0x30` ETAC, `+0x32` scouts, `+0x34` strength cache.
Starbase record (BASES.DAT, 35 bytes): `+0x02` base id (u16), `+0x04` owner,
`+0x0b/+0x0c` x,y, `+0x19` Z, `+0x1f` moving flag, `+0x20/+0x21` target,
`+0x22` TZ; the full layout is in [ec-starbase-spec.md](ec-starbase-spec.md). Orders: 0 Hold, 1 Move, 2 Seek Home, 3 Patrol, 4 Guard Starbase,
5 Guard/Blockade, 6 Bombard, 7 Invade, 8 Blitz, 9 View, 10 Scout Sector,
11 Scout System, 12 Colonize, 13 Join, 14 Rendezvous, 15 Salvage.
`layer(Z) = 1 if Z >= 0.5 else 0` (Turbo Pascal `Round(Z) == 1` for Z in [0,1]).
`at_target(u) = u.x == u.tx && u.y == u.ty && layer(u.Z) == u.TZ`
(TZ is clamped to 1 by the step).
`Roll(p) = Random_real() * 100 <= p` (`24a3:0000`; Random_real in [0,1)).

---------------------------------------------------------------------------

## 1. Run, year and tick loop

### Unit table (built once per ECMAINT run) [C]
`2895:56d4` (loader) walks each player's fleet chain (player `+0x40` first
fleet, fleet `+0x03` next) and starbase chain (player `+0x44`, base `+0x05`
next) and appends one 12-byte entry per unit. Planets are not units; IPBMs are units of kind 3 that the encounter scan skips.

| Entry byte | Meaning |
| --- | --- |
| +0 | owner empire |
| +1, +2 | sector x, y (Trunc of position) |
| +3 | active (1); cleared when the fleet/base is deleted (destroyed, merged) |
| +4 | kind: 1 fleet, 2 starbase |
| +5 | layer (Z >= 0.5) |
| +6 | record number in FLEETS.DAT / BASES.DAT |
| +8 | sort key (u16) |
| +10 | record word +0 |

Entries for fleets created during the orders phase (`24a3:17ad`, `24a3:1c79`,
e.g. commissioned ships, splits) are appended with layer 1, kind 1, active,
and a zeroed awareness record. All unit creation happens before the year loop.
The entry's x, y, layer are rewritten by that unit's own step each tick
(`0824:0b23..0b4f`).

### Awareness table (per run) [C][P]
A temp file of 25-byte records, one per unit, byte `[empire-1]` = how much
that empire knows about that unit: 0 unknown, 1 contact, 2 identified,
3 fought. Created zeroed at run start (`2895:7f20`), erased at run end
(`0824:1724`). It persists across ticks and across years of a multi-year run.

- `get(empire, unit)` `1b05:0642`; `raise(empire, unit, level)` `1b05:0688`
  only increases (max).
- It is per empire, not per fleet: once any fleet or starbase of an empire
  identifies a unit, every unit of that empire treats it as identified. [P]
  (`shared.py`: two patrols of one empire never both report first contact
  with the same target, 0 of 35 runs; independent awareness would give ~8.)
- Reset: at the end of a unit's step, if `Trunc(X)`, `Trunc(Y)` or
  `layer(Z)` changed, that unit's whole record is zeroed (every empire
  forgets it) (`0824:0e0c` -> `0824:0f92`). [P] (`reset.py`: a fleet crossing
  three patrolled sectors is re-contacted in each one.) A guard-starbase
  escort carried along with a moving starbase is reset too.
- Nothing else lowers awareness.

### Year driver `0824:1004` [C]
Called once per simulated year (ECMAINT `/R` normally runs one year).
Before it, per year: `0824:26a5` (CONQUEST previous-year snapshot),
`24a3:3cc6` (rogue and autopilot orders), `1852:0000` (the planet pass:
status step, builds, economy); see
[ec-turn-cycle-spec.md](ec-turn-cycle-spec.md) section 4.0.

```
for each unit i: key[i] = Random(0xFFDC)          // TP Random(n) = (Seed>>16) mod n
bubble-sort units by key, descending (equal keys keep order)   // once per year
for tick in 1..=52:
    stardate = tick                               // reports show "Stardate: tick/year"
    for i in sorted order:
        if !entry[i].active: continue
        step(i)          // 0824:01c0: movement; fleets update their in-memory record,
                         // entry x/y/layer updated, awareness reset on sector/layer change
        encounter(i)     // 18ea:13cf (section 2-5)
        reset report buffer
        dispatch(i)      // 0b5d:5581: arrival tests and order actions; saves the fleet record
        reset report buffer
```
The order is fixed for the whole year. Starbases are units in this loop too
(they step, scan and dispatch). A fleet's step result is only in memory until
its dispatch saves it (relevant to section 5).

---------------------------------------------------------------------------

## 2. Encounter handler `18ea:13cf` for observer unit i

### Observer setup
Fleet observer (record O):
```
closing = (O.ROE != 0)
F = 1.0                                       // sensor factor
if O.speed == 0 || at_target(O):
    match O.order:
        3 (Patrol)          => F = 10, closing = true
        10, 11 (Scout)      => F = 5,  closing = true
        5 (Guard/Blockade)  => F = 5,  closing = true
        4 (Guard Starbase)  => closing = false
        _                   => (F = 1, closing unchanged)
```
Starbase observer (record S): it scans only if `S.x==S.tx && S.y==S.ty &&
S.Z == float(S.TZ)`; then `F = 25` if `layer(S.Z)==1` else `F = 30`
(`closing` = false). A starbase not at its target does nothing this tick. [P]
Starbases move ([ec-starbase-spec.md](ec-starbase-spec.md)); a base's step
updates its entry and its escort's, as a fleet's does.

### Pairing loop
```
for j in 1..=unit_count (table order, not sort order):
    stop if the observer attacked earlier in this loop (one attack per observer per tick)
    skip unless entry[j].owner != entry[i].owner
             && entry[j].x == entry[i].x && entry[j].y == entry[i].y
             && entry[j].layer == entry[i].layer && entry[j].active
    a = get(owner(i), j)                       // observer empire's awareness of j
    newa = a
    ... branch by (observer kind, target kind), below ...
    raise(owner(i), j, newa)                   // always, at the end of the pair
```
Each pair gets at most one sighting roll per tick from each side (each unit
rolls when it is the observer). Starbases never start a fight [P t034].

### Fleet observer, fleet target `18ea:16f7`
```
if !closing && a >= 3: skip pair
p = 10*a + vis_fleet(T) * F
if !Roll(p): (raise to newa = a) next
if O.order in {10, 11}:                        // scouts
    if a == 0: contact report; newa = 2 ... identify_and_decide(mode 2)
    elif a == 1: newa = 2; identify_and_decide(mode 2)
    else: nothing (scouts never re-decide against fleets)
else:
    newa = max(a, 1)
    if a == 0: contact report (581); attack = closing
               if closing: identify_and_decide(mode 2)    // same tick
    elif a == 1: if closing: identify_and_decide(mode 2)
    else (a in {2,3}): attack = decide(O, T, mode 3)       // silent, see below
if attack: battle(O, T)  -> section 5
```
`identify_and_decide(mode 2)` (`18ea:0831`): `newa = 2`, sends the
"located and identified" report with the decision text, `attack = decide(O,T,2)`.
Contact report (`18ea:0581`): "Sensor contact shows an alien fleet ...";
if closing, "Closing to check it out..."; otherwise "We are designating this
sighting as FS-n ..." and `newa = 1`.

Consequences [P]: a fleet with ROE 0 that is not Patrol/Scout/Guard-at-post
(and any Guard Starbase fleet at its post) only ever reaches contact (1) by
its own sensors and never attacks on its own, unless its empire already
identified the target (shared awareness), in which case it runs mode-3
decisions. Mode-3 decision reports are discarded (not flushed), but their
state changes (attack, abort to Seek Home) happen. [P d2 runs r5/r6/r8:
silent Seek Home]

Quirk [C]: for scouts the handler sets `closing = false` after the first new
contact, for the rest of that observer's loop this tick.

### Fleet observer, starbase target `18ea:1d10`
```
if !closing && a != 0: skip pair
p = 10*a + vis_base * F
if !Roll(p): next
attack = closing
if O.order in {10, 11}:
    newa = max(a, 2)
    if a in {0,1}: identify base, attack = decide(O, base, 2)
    else: attack = decide(O, base, 3)
else:
    newa = max(a, 1)
    if a == 0: base contact report; if attack: identify base + decide(mode 2)
    elif a == 1: if attack: identify base + decide(mode 2)
    else: attack = decide(O, base, 3)
if attack:
    g = guard_fleet_of(base)        // 1b05:21de, below
    if g: raise(owner(O), g, 3)
    battle(O, base)
```

### Starbase observer, fleet target `18ea:1900`
```
if a >= 2: skip
if Roll(10*a + vis_fleet(T) * F): newa = 2; report to base owner
```
The report ("We have located and identified an alien fleet ...") ends with
either "The aliens appear to be headed at our planet X! We are alerting all
fleets in the area." (target order 6/7/8 with TZ==1 at its own target sector,
or target in layer 1, and the world there belongs to the base owner),
"We are alerting all fleets in the area." (base owner declared the target's
owner an enemy), or "Continuing with scanning assignment...". The alert is
text only; the only state change is awareness 2 for the base owner's empire,
which every fleet of that empire then shares. [C][P]

### Starbase observer, starbase target
Same as above with `vis_base`; awareness 2 and a report only. The report
(`18ea:1fea`) reads "We have located and identified an alien starbase in
Sector(x,y). It is Starbase N of <empire>." ("System" for layer 1), and ends
with "We are alerting all fleets in the area." when the base owner has
declared the target's owner an enemy, else "Continuing with scanning
assignment...". After a fleet's earlier contact (awareness 1) it opens with
"We have identified previously reported starbase sighting BS-N as " instead.
There is no "headed at our planet" form for a base. [C]

---------------------------------------------------------------------------

## 3. Sighting formulas

### vis_fleet(T) `18ea:007a` [C][P]
```
A = 45*(BB+ETAC) + 30*(CA+TT_empty+TT_loaded) + 15*DD      // each product u16 (wraps)
v = sqrt(A)
if layer(T.Z) == 1: v *= 5
v += sqrt(3*Scouts)
if at_target(T): v *= order_vis[T.order]                    // else ×1
v *= 1 + T.speed/10                                          // current speed +0x0a
v /= 52
```
`order_vis`: 0 Hold 1, 1 Move 1, 2 Seek Home 1, 3 Patrol 2, 4 Guard SB 1,
5 Guard/Blockade 1.1, 6 Bombard 20, 7 Invade 20, 8 Blitz 20, 9 View 1.2,
10 Scout Sector 1.5, 11 Scout System 2, 12 Colonize 2, 13-15 1.
(The real constants are 1.1 and 1.2 as real48: `81 cd cc cc cc 0c` and
`81 9a 99 99 99 19`.)

A Hold fleet's dispatch zeroes its speed (`+0x0a`), so the speed term is 1
from its second tick on; its position does not change. [P t044: speed 3 at
Hold, deep space and in a system, `+0x0a` 0 after the year]

### vis_base `18ea:0000` [C][P]
`90 / 52` (= 1.7308) for every starbase (the `+0x1f` test multiplies by 1).

### Per-tick probability
`p% = 10*awareness + vis * F`, success iff `Random*100 <= p`. Starting from
awareness 0 the first-contact tick is geometric with `q = p/100`.

Probe confirmation (MLE of q from first-contact ticks, n = 240 runs each):

| Pair (observer sees target) | Expected q | Measured |
| --- | --- | --- |
| Patrol sees 1 CA hold, layer 0 | 1.053% | 0.981 ± 0.10 |
| Hold sees Patrol 1 DD (×2 vis) | 0.149% | 0.098 ± 0.03 |
| Guard (F 5) sees 1 CA, layer 1 (×5) | 2.633% | 2.696 ± 0.20 |
| Hold sees Guard 1 BB layer 1 (×5 ×1.1) | 0.710% | 0.625 ± 0.08 |
| Patrol sees 1 DD + 2 SC hold | 1.216% | 1.240 ± 0.12 |
| Hold sees Patrol 2 CA + 3 SC | 0.413% | 0.476 ± 0.07 |
| Hold sees 1 ETAC + 1 TT, layer 1 | 0.833% | 0.721 ± 0.08 |
| Hold sees 1 CA, layer 1 | 0.527% | 0.608, 0.524, 0.562 |
| Patrol sees Patrol 1 CA (both ways) | 2.107% | 2.021, 1.990 |
| Patrol sees moving 1 CA, speed 3 | 1.369% | ~1.50 (17-tick window) |
| Patrol sees 4 scouts hold | 0.666% | 0.595 ± 0.07 |
| Hold sees Patrol 3 DD | 0.258% | 0.370 ± 0.06 |
| Starbase layer 1 (F 25) sees 1 CA | 13.17% | 13.14 ± 1.5 |
| Starbase layer 0 (F 30) sees 1 CA | 3.160% | 3.21 ± 0.40 |
| Hold sees starbase | 1.731% | 1.63 ± 0.14, 1.99 ± 0.27 |

The `10*awareness` term [P]: after a battle (awareness 3 both ways) a
moving observer (F 1) re-engaged a 50-ETAC fleet (vis 0.91) with mean gap
3.23 ticks over 22 gaps; `p = 30.9%` predicts 3.2 (`mvlost.py`).

---------------------------------------------------------------------------

## 4. Decision `decide(O, T, mode)` `1b05:1277`

`mode` 2 = first identification, 3 = later re-check. Define
```
willing(O, T) = test(O, S(T)) || (mode < 3 && O.ROE > 0)
engage        = willing && intercept(O, T)      // attack iff engage
```
So at first identification any ROE > 0 commits regardless of odds; later
re-checks need the strength test. `planet` = the world in O's sector (only
looked up when O is in layer 1; see the deep-space note).

### Strength `S(F)` `24a3:02b4` [C]
`S = 21*BB + 7*CA + 2*DD` (u16 arithmetic), `+30` if order 4 and speed 0.
Transports, ETACs, scouts count 0. ECMAINT writes `S` to `+0x34`.

### ROE test `test(F, E)` `1b05:0706` (own strength `S(F)`) [C]
| ROE | true when |
| --- | --- |
| 0 | never |
| 1 | E == 0 |
| 2 | E == 0 or S/E >= 4 |
| 3 | E == 0 or S/E >= 3 |
| 4 | E == 0 or S/E >= 2 |
| 5 | E == 0 or S/E >= 1.5 |
| 6 | E == 0 or S/E >= 1 |
| 7 | S > 0 and E/S <= 1.5 |
| 8 | S > 0 and E/S <= 2 |
| 9 | S > 0 and E/S <= 3 |
| 10 | always |
| >10 | undefined in classic (uninitialised); treat as 10 or reject |

`test_vs_fleet(F, T) = test(F, S(T))` (`09c1`);
`test_vs_base(F, B) = test(F, 30 + S(guard_fleet_of(B)) if any else 30)` (`0a0e`).

### Fleet target, in this order

**H. Hostile-mission block** — only if all hold: `layer(O.Z)==1`,
`O.order in {5,6,7,8,12}`, `O.x==O.tx && O.y==O.ty`, `at_target(T)`,
`T.order in {4,5,6,7,8,12}`.

- H1, `O.order == 5` (guard/blockade, any world, own or foreign): if
  `T.order in {6,7,8}` or (`T.order == 12` and planet unowned): handled.
  If willing: "The alien fleet seems to be preparing to <invade|bombard|blitz|colonize>
  our planet X / the world in this solar system." then intercept ->
  "Accelerating to intercept alien fleet..." / "However, we are unable to
  intercept alien fleet."; else "In accordance to our ROE, we are avoiding
  this alien fleet...". Other target orders: not handled (fall through).
- H2, `O.order == 12 && T.order == 12` and planet unowned (colonize race):
  handled; willing -> "trying to beat us to the unowned world" + intercept;
  else avoid. [P d3]
- H3, `O.order in {6,7,8,12}` and `T.order in {4,5}` and `layer(T.Z)==1`:
  handled. Willing -> "The alien fleet is blocking our path to the target
  world." + intercept ("Accelerating to engage alien fleet..." or "Though
  they seem to be guarding the world in this system, the aliens are fleeing
  before us."). Not willing -> abort: "blocking our path. To follow our ROE,
  we must evade the alien fleet. We are therefore aborting our mission of
  <bombarding|invading|blitzing|colonizing> the world in System(x,y)" and
  `abort_to_seek_home(O)`. [P d2] Applies to neutral guards too.

**E. Declared enemy** (if not handled): the observer's own player record
`+0x53+T.owner != 0` (O's empire declared T's owner an enemy; the other
direction does not count). Willing -> intercept ("Accelerating to engage the
enemy..." / "We are unable to intercept the enemy."); else "In accordance to
our ROE, we are avoiding this enemy fleet...". No location condition beyond
same sector and layer. [P bd3, bd5, d4, icpt]

**N. Neutral** (not handled, not enemy):
- N1: `T.order in {6,7,8}`, T at its target in layer 1, planet owned by
  O.owner: "preparing to <mission> our planet" + willing/intercept/avoid.
- N2: `layer(O.Z)==1` and planet owned by O.owner: "Even though the alien
  fleet belongs to a neutral empire, it is within one of our star systems."
  + willing/intercept ("Accelerating to intercept the aliens..." /
  "However, we are unable to intercept the aliens.") / avoid. [P t034]
- else "Ignoring alien fleet...". [P bd1, bd2, bd4]

N1 is a subset of N2 in practice (same outcome, different text).

**abort_to_seek_home(O)**: the destination is the nearest friendly world
(`24a3:0876`, called with flag 1). If found: order 2, target = that world,
TZ 1, speed = max speed, mission id untouched; text "Until ordered
otherwise, we will be seeking the safety of a friendly controlled solar
system. Our new destination is ...". If none: order 1 (Move) to O's own
sector with TZ 0 (climb out), speed = max ("and are leaving the solar
system"). The record is saved immediately.

**Nearest friendly world** `24a3:0876` [C][P]. It lists the worlds O's
empire owns in PLANETS.DAT order, leaving out the world in O's sector when
O is in that system (`Z >= 0.5`), and takes the one with the shortest
travel distance (`24a3:04f2`, to the world's centre at depth 1, the measure
ECGAME's Seek Home uses), keeping the first on a tie. It finds nothing when
the empire owns no other world. Every abort in ECMAINT asks it this way,
with flag 1: both decision aborts above (`1b05:1877` fleet, `1b05:1f76`
base, which the starbase gate `110a:67bd` also runs), the battle's scatter and
retreat (`1b05:25c2`, `1b05:2f41`, `1b05:309f`), Seek Home whose world was
lost (`0b5d:033d`), an aborted Salvage (`0b5d:4b2a`), and a defecting
fleet (`24a3:380b`). Only rogue and autopilot orders call it with flag 2,
which divides each distance by the world's present production. [P t059:
a bombard fleet in orbit of (16,13) that would not fight the starbase
there went to (10,6), 9.22 sectors across the map and 13 by the Manhattan
count, not to (18,3), 10.20 and 12; a Seek Home fleet in orbit of
its own world (16,13) whose target was lost went to its other world
(12,12), not to (16,13).] Rust uses this picker for both decision aborts,
the orbital gate's abort, and a fleet that lost the ships its mission
needs.

### Starbase target `1b05:1d11`
- If `layer(O.Z)==1`, `O.order in {6,7,8}`, `O.x==O.tx && O.y==O.ty`,
  `O.TZ == 1` (any owner's base, diplomacy ignored): "The alien starbase is
  protecting the target world. This base must be eliminated before we can
  ..." then `test_vs_base(O, B)` -> attack ("Accelerating to attack alien
  base..."), else "since our ROE forbids us to attack a starbase, we must
  abort our mission ..." and abort to Seek Home as above, except with no
  friendly system the fleet gets order 0, speed 0. No intercept roll.
- Else if O's empire declared the base owner an enemy: `test(O, 30)` (guard
  fleet not counted here) -> "Accelerating to attack enemy base..." / "Avoiding
  enemy base." No intercept roll. [P t034 s_e21]
- Else "Ignoring alien base." [P t034 s_neutral]

### Intercept `intercept(O, T)` `1b05:03b1` [C][P]
```
if O.max_speed (+0x09) > T.max_speed: return true
aw = get(T.owner, O)                    // target empire's awareness of the observer
if Roll(100 - 33*aw):
    if T.speed (+0x0a) == 0: return true                   // p = 101
    u = Random_real()                                      // drawn first
    return Roll(100 * sin(pi*u) * O.max_speed / T.speed)
else:
    return test(T, S(O))                // the target chooses to fight anyway
```
Probe (`icpt.py`, aw = 0, target speed 1, target max 6, n ≈ 86 each):
observer max 1: 0.651 (expected 0.637); max 2: 0.852 (0.837); max 6: 0.976
(0.947); max 7: 1.000 (1.000). The aw term explains why fights rarely
repeat against a fleet that has already fought (aw 3 -> 1%). [P bd5, d4]

### Guard pairing helpers [C]
- `guard_fleet_of(B)` (`1b05:21de`): first active fleet entry with B's owner,
  B's x,y, layer == `layer(B.Z)`, order 4 and `+0x23 == B.id`.
- `guarded_base_of(F)` (`1b05:2133`): the base of F's owner with
  `id == F.+0x23`.

### Worldless sectors: a deliberate difference [C]
The planet lookup (`31f9:0000`) returns 0 when the sector has no world, and
classic then reads owner byte `+0x5d` through the slot-0 pointer, which is
garbage. This matters only for a layer-1 observer in a worldless sector (the
H1, H2, N1, and N2 tests). It is a classic bug, so Rust does not copy it: a
sector with no world belongs to no one and is nobody's system.

---------------------------------------------------------------------------

## 5. When an attack starts

1. Handler (fleet vs fleet): `battle = 1b05:7b74(O, T, 0)`. Fleet vs base:
   first `raise(owner(O), guard_fleet_of(B), 3)`, then the same battle call.
2. Battle setup raises awareness both ways to 3 (`raise(T.owner, O, 3)`,
   `raise(O.owner, T, 3)`), and to 3 for the partner that joins.
3. Participants are exactly two sides [C][P d4]:
   - side A: the observer fleet only;
   - side B: the target fleet, plus `guarded_base_of(T)` (+30 strength) if
     T has order 4, speed 0, and that base is active in the same sector and
     layer; or, for a base target, the base plus `guard_fleet_of(B)`.
   No other fleets in the sector join (not allies, not other fleets of
   either empire). A second target is fought only in a later tick.
4. The battle starts at a separation distance of `5 + Random(20)` (the
   `Random(5)+5` surprise branch needs a flag that the encounter path always
   passes as 0). The battle itself, with its retreat, scatter, and loss
   rules, is decoded in [ec-battle-spec.md](ec-battle-spec.md) §3.
5. After the battle [C][P]: survivors' max speed recomputed (`24a3:036b`);
   destroyed fleets/bases deleted (entry inactive); each participant in layer
   1 at a world its empire does not own gets a DATABASE.DAT owner-and-year
   update (`2895:2ddc` mode 1, battle spec §2.11). An attacker with
   order 6/7/8/12 that is driven off aborts to Seek Home ("Since we cannot
   get past them ... aborting our mission"); a holding target that flees
   stays put with its order [P d2, d4].
6. The handler then reloads the observer record from disk and stops the
   observer's pairing loop for this tick. Because the battle read the
   observer from disk, **the observer's movement for that tick is lost**: its
   saved position is its start-of-tick position. [P `mvlost.py`: the final X
   falls short by exactly `attacks/52` for a speed-1 mover, 16 runs, 0 to 5
   attacks each.] The unit table keeps the stepped sector/layer until the
   next step.
7. Stored diplomacy never changes [P t034].
8. Report: both sides get battle reports ("We successfully intercepted ...",
   "We were attacked by ...", "We lost all contact with ...").

Rust follows steps 1-3 and 5-7 but resolves the battle itself with NC combat
(see [nc-combat-spec.md](nc-combat-spec.md) and
[NC combat stays](../notes/decisions/b5u6lekb-rust-keeps-its-own-combat-system.md));
classic's battle is decoded in [ec-battle-spec.md](ec-battle-spec.md) for
reference.

---------------------------------------------------------------------------

## 6. Blockade departure (t021) [C][P]

Classic has no departure strike and nothing runs before movement. A fleet
leaving a world is an ordinary unit in the tick loop: it shares the
blockader's sector and layer while `Z >= 0.5`, about `26/speed` ticks of
climbing (9 ticks at speed 3), and the normal rules apply:

- Neutral blockader (the fleet's empire not declared an enemy by the
  blockader's empire): the blockader identifies it and logs "Ignoring alien
  fleet..." and never attacks. [P bd1 0/10, bd2 0/10 fights; 7 of 20 runs
  logged the identification]
- Holding fleets of the world's owner (including just-commissioned ships,
  which are ordinary order-0 fleets in layer 1 from tick 1) are also
  ignored by a neutral blockader. [P bd4 0/10]
- Enemy (the blockader's empire declared the leaver's owner an enemy): the
  normal enemy rule applies while they share the layer: sighting roll, then
  intercept. [P bd3: 1 fight in 10 departures at speed 3; bd5 holding
  enemy: fights in 8/10]
- The leaving fleet may attack the blockader itself under N2 if it is in its
  own empire's system and has ROE > 0 (it needs its own sighting first).
- A blockader does attack third-party bombard/invade/blitz arrivals, and
  colonizers of an unowned world (H1), at a foreign world too.

So departures are fought only by blockaders whose empire declared the leaver
an enemy, by chance, during the climb. Rust has no departure strike either.

---------------------------------------------------------------------------

## 7. Turbo Pascal RNG [C]

- `RandSeed` (DS:0x3A6, u32): `RandSeed = RandSeed * 0x08088405 + 1`
  (`3374:1639`).
- `Random(n)` = `(RandSeed >> 16) mod n` (`3374:15ea`; TP6-style modulo, not
  multiply).
- `Random` real (`3374:1600`) = `RandSeed / 2^32`.
- `Randomize` (`3374:1671`, INT 21h/2Ch time of day) runs at startup
  (`0824:27d9`), so the seed is the DOS clock. Exact sequence reproduction
  would need the clock time and the exact order of every Random call in
  ECMAINT; only distributional fidelity is practical. (Probes vary the seed
  by setting the DOSBox clock with `time HH:MM:SS` before ECMAINT; runs
  started in the same second otherwise repeat exactly.)

Random draws per tick in the encounter: one per sighting roll; per
intercept up to three (aw roll, sin draw, speed roll); battles draw more.

---------------------------------------------------------------------------

## 8. Not settled

- The nearest-friendly picker (`24a3:0876`, `09b7..09ef`) has a side
  effect: when the empire has two or more candidate worlds, it raises the
  present production (PLANETS `+0x03`) of any candidate below 1.0 to 1.0,
  in every mode, on the in-memory planet table that ECMAINT writes to
  PLANETS.DAT at the end of the run (`0824:1398`). It reads as a guard for
  flag 2's division that leaks into every call. It shows only for a world
  whose production fell below 1.0 during the year's ticks, since the planet
  pass raises it to 1.0 first. Rust does not copy it; the owner decides
  whether it should (t059).
- AI empires (players marked 0xFF) were not probed.
- ROE > 10 and u16 overflow in vis/strength (more than ~1450 ships of a
  class) are undefined or wrap in classic.
- The unit table owner byte uses the record owner when global `[0x16a4]` is
  set, otherwise the player slot; they are equal in normal games.
