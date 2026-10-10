# EC Battle Spec

Classic ECMAINT's battles: planetary assault (bombard, invade, blitz), fleet
against fleet, and fleet against starbase. Read from the unpacked ECMAINT
(`tools/unlzexe/ecmaint_640k.bin`, seg:off -> file offset `seg*16+off-0x8140`,
DS = `0x3529`) and checked against classic probes. Rust ports this rule and
its odds (task t036), except the classic bugs listed in section 8.

- **[P]**: confirmed by classic probes (statistically, section 6).
- **[C]**: read from code only.

The encounter loop, sighting, `decide()`, and intercept are in
`docs/dev/ec-encounter-spec.md`. This spec starts where those end.

Notation:
- Fleet record bytes are FLEETS.DAT offsets (`+0x1f` order, `+0x25` ROE,
  `+0x26` BB, `+0x28` CA, `+0x2a` DD, `+0x2c` empty TT, `+0x2e` loaded TT,
  `+0x30` ETAC, `+0x32` scouts, `+0x09` max speed, `+0x0a` speed).
- Planet record (PLANETS.DAT, 97 bytes), as ECMAINT uses it:

| Offset | Type | Meaning |
| --- | --- | --- |
| `+0x02` | u8 | potential production |
| `+0x03..+0x08` | real48 (exponent byte at `+0x03`) | present production ("factories"), called **P** below |
| `+0x09..+0x0e` | real48 (exponent byte at `+0x09`) | stored goods (production points), called **G** below |
| `+0x24..+0x37` | 20 bytes | build queue, zeroed on capture |
| `+0x38..+0x57` | 8 × u32 | stardock counts, in this slot order: DD, CA, BB, scout, TT, ETAC, starbase, IPBM |
| `+0x58` | u16 | armies |
| `+0x5a` | u16 | ground batteries |
| `+0x5c` | u8 | status, set to 0 on capture |
| `+0x5d` | u8 | owner |
| `+0x60` | u8 | Scorch-Earth order, set in ECGAME's planet menu |

The dock slot order comes from the bombard report (`110a:0438`). P and G come
from the report's "% of the factories" and "% of the stored goods" lines [P].
The preserved fixtures agree: the homeworld seeds read P = 100 and G = 35, and
ECMAINT's builds land in the typed dock slots. Rust's `PlanetRecord` uses this
layout.

- `Random(n)` is Turbo Pascal `(RandSeed>>16) mod n`, and `Random(0) = 0`.
- `Roll(p)` is `Random_real*100 <= p` (`24a3:0000`).
- Only the distributions need to match. The exact draw order is not portable.

---------------------------------------------------------------------------

## 1. Combat units

### 1.1 Unit stats [C][P]
Each ship, battery, and starbase is one unit record of 10 bytes. There is no
damage state outside a battle. A unit has hit points (HP) only while the battle
runs. A surviving ship goes back into the fleet intact, so there are no
crippled ships, no stored damage, and no repair step. The per-type table is the
same in both battle kinds. The planetary builder is `110a:3ddb`/`4672`, and the
fleet battle builder is `1b05:675e`/`7b74`.

| Type | Unit | Range | HP | Max damage | Speed | Armor | Hit % | Value | Strength |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | Destroyer | 10 | 3 | 4 | 6 | 0 | 55 | 1 | 2 |
| 2 | Cruiser | 15 | 9 | 12 | 5 | 1 | 50 | 3 | 7 |
| 3 | Battleship | 20 | 30 | 36 | 4 | 2 | 45 | 9 | 21 |
| 4 | Scout | 0 | 3 | 0 | 6 | 0 | 55 | 0 | 0 |
| 5 | Empty troop transport | 0 | 5 | 0 | 5 | 1 | 40 | 0 | 0 |
| 6 | Loaded troop transport | 0 | 5 | 0 | 5 | 1 | 40 | 0 | 0 |
| 7 | ETAC | 0 | 18 | 0 | 3 | 1 | 40 | 0 | 0 |
| 8 | Starbase | 25 | 60 | 36 | 1 (0 in the unit) | 3 | 60 | 15 | 30 |
| 11 | Ground battery | 20 | 18 | 36 | 0 | 1 | 65 | 8 | 17 |

- *Value* is the word table at DS:0x352 (`[20,1,3,9,0,0,0,0,15,0,0,8]`). It
  is used only by the engagement count (§1.3).
- *Strength* is the ROE strength: `21 BB + 7 CA + 2 DD`, plus 30 for a
  starbase. In the planetary battle each battery counts 17.
- The starbase unit's own speed byte is 0, so it can never escape. The
  per-side speed recompute counts a starbase as speed 1.
- The army is not a unit. Ground combat is a separate count-based loop (§2.8).

### 1.2 Unit order inside a side [C]
Units are built in this order: BB, CA, DD, scouts, empty TT, loaded TT, ETAC.
Targets are stored as indexes into this array, and the engagement count takes
a prefix of it, so Rust must keep the order. "Combat count" `c` is the number
of BB+CA+DD units. They are always the first `c` entries. A starbase goes last
on its side, after any guard fleet's ships, and does not count as a combat
ship.

### 1.3 Engagement count `engaged(s)` [C][P]
`110a:483a` is used for planets, and `1b05:768c` is used for fleet battles.
Only the first `n` units of side `s` fire in a round:
```
A = sum(value) of side s,  B = sum(value) of the other side
if A == 0: n = 0
elif B == 0 or B >= A: n = all units of s
else:
    y = (A/B) ** (1/e)          // planetary e = 4.0, fleet e = 3.3
    if y > m: y = (y + m)/2     // planetary m = 1.5, fleet m = 1.3
    (fleet only) y = min(y, 2.0)
    k = Trunc(B*y + 0.5)
    n = 0; while n < count(s) and k > 0: n += 1; k -= value(unit n)
units n+1..: target := 0  (starbase: target := 1 if the enemy has units)
```
So the larger side commits about the enemy's value, scaled up by
`(A/B)^(1/e)`, starting with its heaviest ships. Value-0 ships (scouts,
transports, ETACs) count toward `n` but never fire. The count is recomputed
every round. [P: before this was fixed, bombard battle losses were off by
z > 10. They match after the fix (section 6).]

---------------------------------------------------------------------------

## 2. Planetary assault (Bombard 6, Invade 7, Blitz 8)

### 2.1 Arrival test and aborts, `0b5d:127f` / `1670` / `1a9f` [C][P]
The assault runs from the order dispatcher (`0b5d:5581`), right after the
fleet's step, on the first tick that passes this test:
1. **Own world.** The world at the target belongs to the fleet's owner. The
   ownership check runs anywhere along the route. The fleet reports "Since we
   now own the world in System(x,y), we are aborting our mission to
   <bombard|invade|blitz> it." If its strength (`1b05:0000`) is above 0 and
   ROE > 0, it takes order 5 (Guard/Blockade), "taking the initiative to guard
   this new world". Otherwise it takes order 1 (Move). In both cases TZ = 1,
   and speed is refilled from max speed if it was 0.
2. **Unowned world.** The fleet is in the target sector, `Round(Z) >= 1`, and
   the world is unowned. It reports "...have found an unowned world. Since
   there is nothing to destroy / Since the world needs terraforming, we are
   aborting ..." and takes order 0.
3. **Arrived.** The fleet is in the target sector, `Round(Z) >= 1`, and
   `Z >= 1.0` (or `Z == TZ`). The assault action runs:
   bombard `0b5d:149e` -> `110a:7026`, invade `188f` -> `72ed`,
   blitz `1cbe` -> `7188`.

Each fleet assaults alone, in its own dispatch slot. Other fleets of the same
empire at that world assault separately when their turn in the tick order
comes, against whatever the earlier assaults left. The world owner's guard
fleets and starbases are not part of the planetary battle. Starbases are
handled by the gate (§2.2), and guard fleets by the encounter loop.

The dispatcher also sets `ROE = 0` for any fleet without BB, CA, or DD, every
tick (`0b5d:55bf`) [C].

### 2.2 Starbase gate `110a:67bd` [C][P]
Before any assault, check each active starbase unit owned by the planet's
owner, in layer 1, in the fleet's sector:
- The fleet reports "There is an alien starbase orbiting the target world. "
  and runs `decide(fleet, base, mode 2)` (encounter spec, starbase target).
  That decision either attacks the base or aborts the fleet to Seek Home.
- If it attacks, the fleet fights `battle(fleet, base, 0)` (section 3). After
  that battle the fleet record is **reloaded from disk**: it keeps the battle
  losses, but this tick's movement is lost.

The assault goes ahead only if the last decision was "attack", the fleet is
still active, and its order is still 6, 7, or 8. So a fleet that beats the
base assaults the world in the same tick [P: S1/S3 runs show the base battle,
then the bombardment, at tick 1].

### 2.3 Setup `110a:4672` [C]
- Copy the planet into two buffers: "before" (for reports) and "work".
- Build the fleet units (§1.2), and one battery unit per ground battery
  (`+0x5a`).
- Record the initial battery count `B0` and the fleet's ROE.
- Zero the kill tallies.
- `raise(planet owner, fleet unit, 3)`: the planet owner's empire is now
  "fought"-aware of the fleet.
- Mission weights `w` (`110a:3d87`), used by the bombardment passes (§2.6),
  in the order armies, factories, goods, dock:
  - bombard (10, 20, 30, 40)
  - invade (20, 25, 25, 30)
  - blitz (0, 0, 0, 100), unused

### 2.4 Orbital battle `110a:4ba4(rounds = 10)` [C][P]
This battle runs only when the world has batteries (bombard and invade; a
blitz never runs it).
```
clear all unit targets
round = 1; dist = 25; D = 0; retreat = false; done = false
while !done and round <= 10:
    S = 21 BB + 7 CA + 2 DD over fleet units;  E = 17 * batteries
    spd = min(10, speed of every fleet unit)       // DD 6, CA 5, BB 4, SC 6, TT 5, ET 3
    if dist < 20 and !retreat and !ROE_ok(fleet ROE, S, E): retreat = true
    for side in (fleet, planet):                   // both fire before any removal
        focus = 0
        n = min(engaged(side), units(side))
        for u in first n units of side:
            if u.dmg == 0 or u.range < dist: continue
            if u.target == 0:
                if focus != 0 and Roll(30) and enemy[focus].HP > 0:
                    u.target = focus
                    if u.dmg >= enemy[focus].HP: focus = 0
                else:
                    u.target = Random(c)+1 if side == planet     // batteries only shoot combat ships
                               else Random(batteries)+1
                    focus = u.target
            if Roll(u.hit%) and u.target != 0:
                d = max(0, Random(u.dmg) + 1 - enemy[u.target].armor)
                if side == fleet: D += d                  // overkill included
                enemy[u.target].HP = max(0, HP - d)
    remove dead units, fleet first, then batteries (see below)
    if retreat and dist >= 25: done = true
    if !done:
        if retreat: dist += spd                  // dist < 25 here
        elif dist > 5: dist -= spd
    round += 1
batteries_left = remaining battery units
```

Rules:
- `ROE_ok` is the ROE table in the encounter spec, §4: `test(ROE, S, E)`.
  If `S == 0` or `E == 0` it is `S != 0`. The check only starts once the
  fleet is inside 20 (after closing a round or two), and a retreat is
  permanent. The planet never retreats.
- **Removal.** Walk each side by index. A unit with `HP < 1` counts as killed
  (by type). Enemy targets pointing at it are cleared, and targets above its
  index shift down by 1. The array compacts.
  - If the side becomes empty, the battle is done, and if it is the fleet,
    its unit-table entry is marked inactive (`110a:5169`, see bug B1).
  - Otherwise, if a fleet unit with `index <= c` died, then `c -= 1`, and if
    `c == 0` the battle is done: the fleet has no warships left. Its
    non-combat ships survive, because batteries only target combat ships.
    [P: I3 run 2 ended with 0 BB and 15 loaded TT intact.]
- With batteries in range 20 and ship ranges of 20/15/10, a BB fleet (speed 4)
  takes battery fire from round 3, and a DD fleet (speed 6) from round 2.
  Destroyers fire from 7 or closer. Distance stops closing at 5 or below.
- Afterwards, apply collateral damage (§2.5) with `D`.
- The battle's round counter (`rounds fought + 1`) seeds the bombardment pass
  counter in §2.6.

### 2.5 Collateral damage `110a:5d44(D)` [C][P]
This runs after every orbital battle (§2.4), and once more after every
landing (§2.7), with that phase's damage `D`.
```
k = max(1, sqrt(D/4))
if B0 > 0: k = min(k, 90 * (B0 - B1) / B0)        // B1 = batteries now; no battery killed -> k = 0
had_armies = armies > 0
P *= clamp((100 - k)/100, 0, 1)
G *= clamp((100 - 1.5k)/100, 0, 1)
target = Trunc(dock_value * clamp((100 - 2k)/100, 0, 1))
while dock_value > target: remove one random dock item (weighted by count, byte-truncated pick, bug B5)
lost = armies - Round(armies * clamp((100 - 2k)/100, 0, 1))
lost = clamp(lost, 0, (D - (B0 - B1)*18) div 6)    // longint div
armies -= lost; if armies < 1 and had_armies: armies = 1
write the work planet back
```
`dock_value = 3 DD + 18 CA + 30 BB + 3 SC + 5 TT + 18 ETAC + 60 SB + 2 IPBM`.
The code adds `9 × CA` twice, which may be a typo (B6).
- If the world had batteries but none were destroyed, nothing happens.
- With no batteries (`B0 = 0`), `k` is at least 1, so even an unopposed
  landing costs 1% of factories, 1.5% of goods, and 2% of dock value.
  [P: L2 and L4 goods fell 35 -> 34.475.]

### 2.6 Bombardment passes `110a:6367(R)` [C][P]
R is 20 for bombard (after §2.4) and 12 for invade softening (§2.7). Only
combat ships (the first `c` fleet units) take part.
```
thresholds = (w1, w1+w2, w1+w2+w3, 100)
dockdmg = 0
done = nothing_left()                              // 625a
pass = round counter from §2.4 (0 if no battle ran)
if retreat: pass = R; if Roll(5): pass = R - 1     // a retreating fleet: 0 passes, 5% chance of 1
while pass < R and !done:
    pass += 1
    for each combat ship u (index 1..c):
        repeat: r = Random(100)+1; cat = first i with threshold[i] >= r
        until category_ok(cat) or done
        if Roll(u.hit%) and !done:
            d = max(0, Random(u.dmg) + 1 - 2)
            armies:    if d >= 2 and armies > 0: armies -= 1
            factories: P = max(0, P - 0.1 d)
            goods:     G = max(0, G - 0.5 d)
            dock:      dockdmg = min(dockdmg + 2 d, dock_value)
            done = nothing_left()
    if invade and armies*4 <= L0 and L0 - armies > 4: done = true    // L0 = fleet's initial loaded TT count (B12)
remove random dock items until dock_value <= dock_value_before - dockdmg
P = max(P, 1.0)
```
`category_ok`:
- armies: `armies > 1`, or `armies == 1` and `P / potential < 0.03`. The last
  army can only be bombed once factories are nearly gone.
- factories: `P > 0`.
- goods: `G > 0`.
- dock: `dock_value > dockdmg`.

`nothing_left()` means no category is ok.

Pass counts:
- A bombard with no batteries gets 20 passes.
- After a full 10-round battle it gets 9.
- After a battle that cleared the batteries in round r it gets `20 - (r+1)`.
- A fleet that retreated gets 0 passes (5% chance of 1).

[P: 10 CA against 0 batteries took the dock to 0 and goods to 0, left 1.18
armies on average, and cut factories to 79%. The model gives 1.10 and 78.7%.]

### 2.7 Invade `110a:72ed` and Blitz `110a:7188` [C][P]
- **Invade:**
  1. Run §2.4 if the world has batteries.
  2. If batteries remain, there is no landing: `L = 0`, and the invasion has
     failed.
  3. Otherwise run softening (§2.6 with R = 12), then the landing
     (`53bb`, which lands at once with no batteries), then ground combat
     (§2.8).
- **Blitz:** no orbital battle and no softening. Run the landing `53bb`
  against the live batteries, then ground combat if anyone landed.

**Landing `110a:53bb`** (the blitz approach) [C][P]
```
clear targets; transports = fleet units of type 5/6 (count, first index, last index)
dist = 25; landed = 0; approaching = true; D = 0
done = !(initial loaded TT > loaded TT killed so far)
while !done:
    fleet fires: n = min(engaged(fleet), units); skipped once landed > 0
    batteries fire: all batteries (no engagement count)
       battery target pick (when its target is 0): focus rule as in §2.4, else
          if approaching and Roll(40): Random(transport count) + first transport
          else Random(last transport index) + 1         // never an ETAC
       fleet target pick: Random(batteries)+1
       damage d = Random(dmg)+1-armor, NOT clamped at 0 here (bug B3)
       units fire only if range >= dist
    removal as in §2.4, with transport index bookkeeping (B4); a dead loaded TT kills its army
    if fleet or batteries all gone: done
    if done and (!approaching or no transports): next round (loop ends)
    if approaching:
        if dist > 0 and !done:
            if a loaded TT survives: dist = max(0, dist - 5) else approaching = false
        else:   // LAND
            landed = initial loaded TT - loaded TT killed; approaching = false
            every surviving loaded TT becomes an empty TT
    else:
        dist += 5; if dist >= 25: done
apply collateral (§2.5) with D; return landed
```
Timeline:
1. Batteries fire at distances 20, 15, 10, 5, and 0 on the way in, with 40% of
   new targets aimed at transports.
2. The armies land at the end of the round at distance 0.
3. Batteries keep firing at the withdrawing fleet at 0, 5, 10, 15, and 20.
   The fleet no longer fires back.

With no batteries (or once all batteries are destroyed) the transports land at
once.

### 2.8 Ground combat `110a:5260(L)` [C][P]
```
pD = 40 + Random(10)        // defenders' hit %, rolled once per battle
pA = 30                     // attackers' hit %
loop while L > 0 and Dn > 0:          // Dn = defending armies
    repeat Dn times (count fixed at loop start): if L > 0 and Roll(pD): L -= 1
    repeat L times (after the defenders' volley):  if Dn > 0 and Roll(pA): Dn -= 1
armies = Dn; return L (surviving attackers)
```
The defenders shoot first and hit harder. Attacker win odds from the model:

| Defenders | 1:1 | 1.5:1 | 2:1 | 3:1 | 4:1 |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 5 | 9% | 42% (7) | 90% | 99.9% | 100% |
| 10 | 3% | 54% | 96.5% | 100% | 100% |
| 20 | 0.5% | 57% | 99.4% | 100% | 100% |

[P: L2, 10 attackers against 10 defenders, captured 1/60 times (model 3.7%).
L4, 14 against 8, captured 78% of the time (model 81%). L5, 20 against 10,
always captured (model 97%).]

### 2.9 Capture `110a:6a9f` (when L > 0) [C][P]
1. The old owner's planet count `player[+0x50]` goes down by 1 and the new
   owner's goes up by 1.
2. Status `+0x5c` = 0, and owner = the fleet's owner.
3. The build queue `+0x24..+0x37` is zeroed.
4. Armies = L (the surviving attackers).
5. Damage, by mission:
   - **Invade:**
     - `G *= Random(400)/1000 + 0.1` if G > 1.0, else G = 1.0.
     - `P *= Random(200)/1000 + 0.2`.
     - Batteries = 0.
     - Each dock slot becomes `Round(r * slot * (Random(800)/1000 + 0.2))`,
       with one `r = Random_real` drawn for all slots.
   - **Blitz:** let `f = min(1, sqrt(L / armies_before_assault))`.
     - `P *= f * (Random(150)/1000 + 0.85)`.
     - `G *= f * (Random(450)/1000 + 0.55)`.
     - Batteries = `Round(batteries_left * f * (Random(650)/1000 + 0.35))`.
     - Each dock slot becomes
       `Round(slot * f * (Random(800)/1000 + 0.2))`.
6. **Scorch-Earth** (`+0x60 != 0`): `P *= 0.5`, `G *= 0.5`, the dock is
   emptied, and the flag is cleared.
7. `P = max(P, 1.0)`.

So an invasion keeps only 20-40% of factories (after softening has also hit
them). A blitz keeps 85-100% scaled by `f`. [P: I1 factories 27.9 (model
27.0). L3 87.6 (model 87.8). L4 87.3 (model 86.1).]

If L = 0 the world keeps its owner, with the armies and batteries the fight
left.

### 2.10 After the assault: the fleet [C][P]
`110a:69b4`, and also `7026` for bombard:
- **The fleet survives** (at least one unit). Its ship counts are rebuilt from
  the surviving units: transports that landed come back as empty TT, and
  transports that did not land stay loaded. The order becomes **0 (Hold)**
  for every mission and every outcome: success, failure, retreat, or no
  damage. The dispatcher then saves the record. The fleet is still in orbit
  at Z = 1.0, and the next tick sets its speed to 0. [P: every surviving
  fleet in pl runs had order 0.]
  - So classic Bombard is a **single bombing run**: one battle, then the
    passes, then Hold. It does not persist.
- **The fleet is destroyed**: `entry.active = 0`, then `2895:3d0c` is called
  to delete the fleet. But that delete routine returns at once when the entry
  is already inactive. So the fleet is **never deleted and never saved**, and
  FLEETS.DAT keeps the record as of the previous tick's save, with all ships
  and the order intact. Next maintenance the "destroyed" fleet attacks again.
  This is the "failed attack -> tick not saved -> retry next year" behaviour
  (bug B1).
  - It is the **only** cause. It applies only when the whole fleet is
    destroyed in the orbital battle or the landing.
  - [P: every "lost all contact" run (B1, B3, B5, B6, L1) left the fleet in
    FLEETS.DAT with order 6/8, its original ships, and Z one tick short. After
    a starbase battle, the record is resurrected in its post-base-battle state
    (S1: 2 BB with order 6).]

### 2.11 After the assault: the world and empires [C]
- Write the work planet back (`7026`/`7188`/`72ed`). Reports follow (§4).
- **Rogues** (`110a:6f13`): if the planet's owner slot is unclaimed (player
  `+0` = 0) and that empire's total present production is under half its total
  potential (or its potential is 0), the slot becomes the AI empire "Rogues"
  (`+0 = 0xff`). Its diplomacy declares only the attacker's empire an enemy.
- Per-empire planet status table `[0x3562 + planet*25 + k]`: for each empire
  whose entry is neither 0 nor 1, set it to 6 after a bombard, 4 after an
  invade, or 7 after a blitz. These codes were not checked further.
- DATABASE.DAT (`2895:2ddc(planet, empire, mode)`):
  - After every assault the attacker gets a mode-2 update: owner, year, name,
    potential, `Trunc(P+0.5)`, `Trunc(G+0.5)`, armies, and batteries.
  - If the owner changed, the old owner gets a mode-2 update too.
  - After a fleet battle, each participant in layer 1 at a world it does not
    own gets a mode-1 update: owner and year. If the owner changed, the
    detailed fields reset to unknown (`0xff`/`0xffff`).

---------------------------------------------------------------------------

## 3. Fleet battle `1b05:7b74(attacker, target, surprise = 0)` [C][P]

### 3.1 Sides and setup
- **Side 1** is the attacking fleet, reloaded from disk.
- **Side 2** is the target fleet. If the target has order 4, speed 0, and its
  guarded base (`2133`) is active in the same sector and layer, that base
  joins as a starbase unit (last), and side 2's strength gets +30.
- If the target is a **starbase**, side 2 is `guard_fleet_of(base)`'s ships
  (if any) plus the base.
- Strength: `S1 = 24a3:02b4(attacker)` (`21BB+7CA+2DD`, +30 if guarding a
  base at speed 0). `S2 = 21BB+7CA+2DD` (+30 base).
- Speeds start as each record's max speed (`+0x09`). Side 2 with no guard
  fleet has an **uninitialised** speed byte at this point (B8).
- ROE: side 1 uses its own ROE. Side 2 uses the target's ROE, but **ROE 10
  while a base is on side 2 and alive**.
- Awareness is raised to 3 both ways (and for the guard or base partner).
- Separation distance: `dist = 5 + Random(20)`. The encounter spec's "5 +
  Random(20) rounds" is this distance, not a round count. The surprise branch
  (`5 + Random(5)`, side 2 holds fire in round 1, 50% roll) is dead code:
  every caller passes 0.
- `init1` and `init2` are the starting unit counts.

### 3.2 Round loop (no round cap) [C][P]
```
round = 1; flags: retreat[1..2], scatter[1..2]; flee_start
loop until done:
    for s in (1, 2):                       // both fire, then removal
        t = 3 - s; focus = 0
        n = engaged(s)                     // §1.3, fleet constants
        for i, u in first n units of s:
            if u.dmg == 0: continue
            if u.target == 0 or enemy[u.target].HP < 1:      // retarget dead targets too
                if focus and Roll(30) and enemy[focus].HP > 0:
                    u.target = focus; if u.dmg >= enemy[focus].HP: focus = 0
                else:
                    if i <= c[t]: u.target = Random(c[t]) + 1               // a combat ship
                    elif retreat[t] or scatter[t] or c[t] == 0: u.target = Random(count[t]) + 1
                    else: u.target = Random(c[t]) + 1
                    focus = u.target
            if in_range(s, u) and u.target and Roll(u.hit%):
                d = max(0, Random(u.dmg) + 1 - enemy.armor); enemy.HP = max(0, HP - d)
    removal (as §2.4; c[s] -= 1 when a dead unit's index <= c[s]; side empty -> done;
             a dead starbase sets base_killed)
    if (scatter[1] or scatter[2]) and !done:
        escape step for the scattering side X against Y (below); skip the ROE checks
    else:
        per side: S = Σ strength, spd = min(10, unit speeds incl. non-combat, SB = 1),
                  maxrange = max range (10/15/20/25)
        r2 = roe_result(side 2); 0: clear both side-2 flags; 1: retreat[2]; 2: scatter[2], flee_start = round
        if base on side 2 and alive: retreat[2] = false
        if init2 == 1 or (init2 == 2 and base): scatter[2] = false
        if !scatter[2]:
            r1 = roe_result(side 1); same mapping into side-1 flags
    if init1 == 1: scatter[1] = false
    dist += spd1 if (retreat[1] or scatter[1]) else -spd1
    dist += spd2 if (retreat[2] or scatter[2]) else -spd2
    if (retreat[1] or scatter[1]) and maxrange2 < dist and spd1 >= spd2: done
    if (retreat[2] or scatter[2]) and maxrange1 < dist and spd2 >= spd1: done
    round += 1
    dist = max(dist, 0)
    if (c[1] == 0 and c[2] == 0) or dist > 30: done
    if !done and any flag set and spd1 == spd2:
        done = true unless some unit with a target is in range
```
- `in_range(s, u)` (`7548`): `u.range >= d`, where `d = dist`. If the enemy
  side is scattering, `d = dist - (u.spd - target.spd) * flee_start`. Classic
  multiplies by the scatter **start round**, not the elapsed rounds (B7).
- `roe_result(side)` (`6cfe`): 0 if `test(ROE, S_own, S_enemy)` passes.
  Otherwise 2 (**scatter**) if `S_own == 0`, or if `S_enemy/S_own > 4` and the
  enemy is at least as fast. Otherwise 1 (**retreat**). The ROE is checked
  every round with nobody scattering, so a retreat can be cancelled if the
  odds improve. Once a side scatters, no further ROE checks are made.
- **Retreat (1):** the side backs off at its speed while the enemy closes at
  its own. With equal speeds the distance holds. The fight continues while
  anything is in range, and ends when the distance exceeds the pursuer's
  maximum range and the retreating side is at least as fast.
- **Scatter (2), escape step:** let `el = round - flee_start`. For each unit
  `u` of the scattering side X:
  - It is a candidate if `el * u.spd >= 30 - dist`.
  - It escapes outright if `el * (u.spd - spdY) >= 30 - dist`.
  - Otherwise it escapes unless some enemy unit targeting it has
    `range >= el*(u.spd - p.spd)`, or `range >= dist`, or `p.spd > u.spd`.
  - Escaped units leave the battle, are not counted as killed, and go back
    into the fleet. Warships that escape lower `c`.
- The battle ends only on one of these:
  - a side is wiped out;
  - neither side has warships;
  - distance > 30;
  - a retreating side gets out of range;
  - equal speeds with nothing in range.

  There is no 64-round cap and no 3-round commitment. ROE 10 against ROE 10
  fights until one side's warships are gone. [P: F1, 10 CA against 10 CA,
  lost 8.2 each.]

### 3.3 After the battle `7b74` tail [C][P]
1. Each side's ships = survivors + escaped.
2. A surviving fleet's max speed is recomputed (`24a3:036b`).
3. The attacker record is saved, or deleted if nothing is left.
4. The target fleet is saved if `total2 > 1`, or `total2 == 1` with no base or
   with the base killed. Otherwise it is deleted. Here the delete works,
   because the entry is still active.
5. A killed base is deleted (`2895:3ebd`).
6. Each participant in layer 1 at a world it does not own gets the DATABASE
   mode-1 update.
7. Order changes:
   - **Scatter** (`1b05:2562`): if the owner still has a planet and the order
     is not 2 or 15, the fleet goes Seek Home to the nearest friendly system
     (`24a3:0876`) with TZ 1, **ROE 0**, order 2, and speed = max. The report
     reads "After reassembling our fleet, we've decided to abort our mission
     ... and to seek the safety of a friendly controlled solar system at
     maximum speed. Until ordered otherwise, we are going to avoid contact
     with alien crafts." [P: F6 targets ended at their home world (15,13)
     after Seek Home.]
   - **Retreat** (`1b05:2e1b`): this applies only to a fleet at its target
     sector with `Round(Z) == 1` and an order other than 2 or 15.
     - An assault or colonize fleet (order 6/7/8/12) aborts to Seek Home
       ("Since we cannot get past them/the alien starbase, we cannot <x> the
       target world ...") when the enemy is a guard (order 4/5 on station, or
       speed 0), or a live base fought.
     - A guard (order 5) aborts ("The aliens seem to be preparing to <x> our
       planet ... Since we cannot stop them ...") when the enemy is assaulting
       its own world (order 6/7/8 on target, no base involved).
     - Any other fleet that retreats keeps its order and position. [P: F2/F3
       holding targets stayed with order 0.]
8. The return value is 1 (the battle happened), and the caller reloads the
   attacker record from disk. That is the encounter spec's lost tick movement.
   Stored diplomacy never changes.

---------------------------------------------------------------------------

## 4. Reports [C][P]
All go to RESULTS.DAT under the stardate `tick/year`.

**Planetary assault (attacker):**
- Bombard: "Bombardment mission report: We have just concluded a bombing run
  against planet "N", currently owned by E. The target world was defended by
  <batteries/armies | Unfortunately for them, ... had no planetary defenses>."
  Then, if it retreated, "After assessing the target world's defenses and
  checking our ROE, we broke off our attack." Then "We managed to destroy <list>
  / We regret that we were unable to inflict any noticeable damage or
  casualties. Our force initially contained ... In the raid, we lost ... / We
  suffered no losses. We are holding our position and are awaiting new
  orders."
- Invade or blitz success: "We have successfully <invaded|blitzed> and taken
  planet ... After the smoke cleared, we had N armies enforcing control on the
  planet." Failure: "Our <invasion attempt|blitz attempt> was defeated. We
  failed to take planet ..."
- Army losses read "N armies, M of which were killed inside their transports"
  or "all of which were killed while still on board".
- If the fleet was destroyed: "We have lost all contact with the N Fleet
  shortly after it began its <bombing run|invasion|blitz attempt> ... with all
  hands presumed lost." It lists confirmed damage. If a capture still happened,
  the text is "managed to drop off the invaders just prior to their
  destruction by ground fire."

**Defender (planet owner):**
- "We have been bombarded by / We have just repelled an invasion attempt / a
  blitz attack by the N Fleet of empire E."
- On capture: "We have been <invaded|blitzed> and conquered by ..." [sic,
  "conquored"], with "When we realized that the battle was lost, we tried to
  destroy anything we thought the aliens could use. We estimate that the
  production of the world is down to Round(P) point(s) out of a possible
  potential."
- Then losses: "In the attack, we lost <k ground batteries, armies, X% of the
  factories, Y% of the stored goods (production points), all/some of the
  ship(s) in stardock including ...>". Factory % is `Round((1-P'/P)·100)`
  capped at 99, shown when the starting P > 1. Goods % is shown when the
  starting G > 0.1.

**Fleet battle:**
- Attacker: "We successfully intercepted ...". Target: "We were attacked by
  ... / The N Fleet and ourselves were attacked by ...".
- Outcome sentences:
  - "The aliens fled before us."
  - "To avoid the total destruction of our fleet, we were forced to scatter."
  - "After reviewing our ROE and the combat situation, we fled"
  - "Though part of the alien fleet scattered ..."
  - "The aliens were completely destroyed."
  - "both sides destroyed each other"
- Then "We lost ... / Luckily, we suffered no losses." A wiped-out side gets
  "We lost all contact with ... burnt flight recorder ...".

---------------------------------------------------------------------------

## 5. Order and record effects, summary
| Event | Fleet record afterwards |
| --- | --- |
| Assault, fleet survives (any result) | order 0 Hold, counts = survivors (landed TT empty), Z 1.0 |
| Assault, fleet destroyed | classic: unchanged pre-tick record (bug B1); Rust: delete |
| Arrival at own world | order 5 (strength > 0 and ROE > 0) or 1, TZ 1 |
| Arrival at unowned world | order 0 |
| Fleet battle, scattered | Seek Home, ROE 0 (if the owner has a world) |
| Fleet battle, retreated | Seek Home only in the 2e1b cases; else unchanged |
| Fleet battle, wiped out | deleted |
| Starbase killed | deleted from BASES.DAT |

---------------------------------------------------------------------------

## 6. Odds check against classic (probes)
Method:
- Each case patches an empire-2 colony at (15,13) (planet record 14), or a
  starbase scenario, and sets the DOSBox clock to a random time before each
  run so classic's random seed varies.
- The comparison model is the decode above written in Python, with the
  classic bugs B1-B3 and B5 included so it can be checked against classic.
  The probe scripts and raw data were kept in the session scratchpad only.
- 1,500+ classic runs in total. z = (probe − model)/SE.

### 6.1 Planetary (selected; 60 to 100 classic runs each, 4,000 model runs)
| Case | Metric | Classic | Model |
| --- | --- | ---: | ---: |
| B1 3 BB bombard vs 4 bat, 10 arm | fleet destroyed | 0.733 | 0.736 |
| | batteries left | 1.58 | 1.64 |
| | goods left (of 35) | 19.8 | 19.9 |
| B2 10 CA bombard vs 0 bat | armies left | 1.18 | 1.10 |
| | factories | 79.0 | 78.7 |
| B4 20 DD bombard vs 4 bat | DD left | 7.03 | 7.35 |
| | batteries left | 1.55 | 1.45 |
| B5 2 CA ROE 10 vs 4 bat | destroyed / armies left | 1.00 / 9.40 | 1.00 / 9.41 (ghost passes, B2) |
| I1 3 BB + 20 TL invade vs 0 bat, 10 arm | captured / armies | 1.00 / 16.7 | 1.00 / 16.8 |
| I2 2 CA + 12 TL vs 0 bat, 10 arm | captured | 0.53 | 0.63 (z −1.6) |
| I3 4 BB + 15 TL vs 4 bat | captured | 0.47 | 0.39 |
| L2 10 TL blitz vs 0 bat, 10 arm | captured | 0.017 | 0.037 |
| L3 2 BB + 30 TL blitz vs 4 bat | TT left / armies | 17.5 / 15.6 | 18.0 / 16.0 |
| L6 20 TL blitz vs 2 bat, 5 arm | captured | 0.967 | 0.997 (z −4.5, see §8) |
| L7 20 TL blitz vs 1 bat, 5 arm | armies after | 14.2 | 14.5 |

Of 89 metric comparisons, 88 are within 2σ (the exception is L6's capture
rate, see §9). Before the engagement-count fix (A/B, not B/A) the bombard
battles were off by z = 10.9. Pricing the cruiser dock slot at 9 instead of 18
(B6) moves the dock-value comparisons to z = 1.9-2.8, so classic really does
use 18.

### 6.2 Fleet battles (≈100 classic battles each; first battle of the year only)
| Case | Classic loss (A / T) | Model loss (A / T) |
| --- | --- | --- |
| F1 10 CA vs 10 CA, both ROE 10 | 8.18 / 8.23 CA | 8.17 / 8.32 |
| F2 10 CA vs 10 CA, T ROE 0 | 4.74 / 5.01 | 4.40 / 4.40 |
| F3 5 BB vs 20 DD, T ROE 0 | 0 BB / 4.48 DD | 0 / 4.26 |
| F4 3 BB + 6 DD vs 4 CA + 4 DD (ROE 10) | 0.72 BB, 4.46 DD / 3.97 CA, 4.00 DD | 0.71, 4.50 / 3.94, 3.97 |
| F5 6 CA ROE 6 vs 3 BB ROE 10 | 3.25 CA / 0.59 BB | 3.41 / 0.45 |
| F6 8 CA vs 4 CA + 6 TT, T ROE 0 (n = 249) | 1.38 CA / 1.87 CA, 1.04 TT | 1.61 / 2.11, 1.23 (z ≈ −2) |
| S1 4 BB vs starbase + guard (1 CA, 1 ETAC) | 1.69 BB lost, base killed 94% | 1.62, 92% |
| S2 8 CA vs starbase + guard | 5.15 CA, base killed 72% | 4.93, 79% |
| S3 2 BB vs starbase + guard | 1.69 BB, base killed 24% | 1.66, 31% |

Blitz landing model (10 or 20 loaded TT, no escort; mean armies landed / share
of runs with none landed):

| Batteries | 0 | 1 | 2 | 4 | 8 |
| --- | --- | --- | --- | --- | --- |
| 10 TL | 10 / 0 | 7.2 / 0 | 4.6 / 0 | 0.96 / 48% | 0 / 100% |
| 20 TL | 20 / 0 | 17.2 / 0 | 14.5 / 0 | 9.5 / 0 | 1.9 / 28% |

---------------------------------------------------------------------------

## 7. Rust vs classic: port checklist
Rust's current model is `nc-combat-spec.md`, the manual's Combat chapter, and
Appendix B.
- [ ] **Unit model.** Rust uses AS/DS on a 10x scale, nominal -> crippled ->
  destroyed, a CRT d10 table, combined-arms and starbase column shifts, and a
  critical 9. Classic gives each unit HP, max damage, armor, hit %, range, and
  speed (§1.1), with per-shot `Random(dmg)+1-armor` against HP, nothing
  persisting past the battle, and no column shifts or criticals.
- [ ] **Suppression/execution fire.** Rust splits suppression fire (CA/BB)
  from execution fire (DD/SB/batteries). Classic has none. Every shooter picks
  one target (random, or the shared focus target 30% of the time) and keeps it
  until it dies.
- [ ] **Targeting.** In fleet battles, classic shooters target combat ships
  unless the enemy is retreating or scattering or has none left; shooter
  index above the enemy's combat count matters (§3.2). Planetary batteries
  only ever target combat ships, so they never hit transports, scouts, or
  ETACs, except during a blitz landing.
- [ ] **Engagement count.** Classic's outnumbering side commits only part of
  its fleet (§1.3). Rust commits everything.
- [ ] **Rounds.**
  - Rust: 3-round commitment, ROE re-checks from round 4, a withdrawal
    exchange at the Pressed column, and a 64-round cap.
  - Classic fleet battle: a separation distance of `5+Random(20)` that closes
    by each side's speed, weapon ranges 10/15/20/25, ROE checked after every
    round from round 1 (retreat or scatter), escape by speed and range, and no
    cap.
  - Classic planetary battle: 10 rounds from distance 25, with a ROE check
    once inside 20.
- [ ] **Pre-combat sensor check / clean retreat.** Classic has none. A fleet
  committed by `decide()` always fights at least one round; it retreats or
  scatters only afterwards.
- [ ] **Hold formation.** Classic has no anchored hold-formation rule. Guards
  (order 4 at a base) fight at ROE 10 while the base lives. A starbase never
  retreats.
- [ ] **Retreat outcomes.**
  - Rust: retreat aborts the mission.
  - Classic scatter: Seek Home and ROE 0.
  - Classic retreat: Seek Home only for assault, colonize, or guard fleets in
    the 2e1b situations; any other fleet keeps its orders.
- [ ] **Participants.** Classic fights exactly attacker vs target (+ base, or
  guard + base), never task forces per empire (already noted in the encounter
  spec).
- [ ] **Bombard.**
  - Rust: three rounds per turn (stardock, then batteries, then a
    breakthrough), hit costs (battery 20, army 10, ...), "100 goods and 1
    factory per point", and Bombard persists each turn.
  - Classic: one orbital battle (10 rounds, batteries only, §2.4), then
    collateral (§2.5), then up to 20 bombardment passes of random category
    picks weighted 10/20/30/40 (armies, factories, goods, dock), with the
    stardock destroyed by value. Then the order goes to **Hold**.
- [ ] **Invade.**
  - Rust: one suppression exchange, softening that hits armies only and kills
    at most half, the defender +1 column, and no draws.
  - Classic: the full 10-round battle. Landing only if every battery is dead.
    Softening is 12 passes weighted 20/25/25/30, which damages factories,
    goods, and the dock as well as armies, and stops early at a 4:1 loaded-TT
    lead. Ground combat is count-based, with defenders hitting at 40-49%
    before attackers hit at 30%. On capture the invasion wrecks the world
    (factories ×0.2-0.4, goods ×0.1-0.5, batteries 0, dock scaled).
- [ ] **Blitz.**
  - Rust: one CRT exchange of battery AS against escort AS, and the starbase
    or enemy combat control blocks the landing.
  - Classic: a descent from 25 to 0 in steps of 5 under all-battery fire (40%
    aimed at transports), the escorts firing until the landing, battery fire
    on the withdrawal, and the same ground combat. A blitz has no orbital
    battle and no softening, and nothing blocks it except a starbase (the
    gate). Captured worlds keep most industry, scaled by
    `sqrt(survivors/defenders)`, and keep a random share of their batteries.
- [ ] **Blitz eligibility.** The Rust manual's "2:1 advantage recommended" is
  roughly right: at 2:1, 90-99% win (§2.8).
- [ ] **Simultaneous arrival.** Rust has a Blitz > Invade > Bombard priority
  and orbital supremacy per planet. Classic has neither: each fleet assaults
  in its own dispatch slot, in tick order. A world can change hands more than
  once in a tick. Other empires' fleets in orbit don't block it; only the
  planet owner's starbase does (via the gate). The owner's guard fleets fight
  through the encounter loop.
- [ ] **Failed attack and retry.** Rust's rule ("after a failed attack it keeps
  its order and speed and attacks again each year", from
  `ec-movement-spec.md` and the manual) copies bug B1. Without the bug: a
  surviving assault fleet goes to Hold, and a destroyed one is deleted. Fix
  the movement spec, the manual §Missions (Hostile missions), and the
  mission table.
- [ ] **Diplomacy.** Rust escalates diplomacy to Enemy. Classic never changes
  stored diplomacy (encounter spec).
- [ ] **Damaged ships.** Rust auto-repairs crippled ships. Classic has no
  damage state, so there is nothing to repair.
- [ ] **Scorch-Earth.** Planet `+0x60`, set from ECGAME: on capture P and G
  are halved and the dock is emptied. Check that Rust has it.
- [x] **Data layout.** Rust reads present production and stored goods as the
  reals at `+0x03` and `+0x09`, and the stardock as eight typed u32 counts.
- [ ] **Rogues.** An unclaimed empire slot attacked while weak becomes the
  "Rogues" AI (§2.11). Check whether Rust models it.
- [ ] **Docs.** Correct the encounter spec: `5+Random(20)` is the starting
  separation distance, not a round count.

---------------------------------------------------------------------------

## 8. Classic bugs (Rust should not copy them)

Rust fixes all ten, B6 and B10 included (user decision, 2026-10-10; see the
[battle bugs ADR](../notes/decisions/sb0sfjgt-rust-fixes-classic-battle-bugs.md)).
- **B1. A destroyed assault fleet is resurrected.** [P]
  - Where: `110a:5169` (orbital battle), `5a9e` (landing), and `6a87` mark the
    unit entry inactive before calling `2895:3d0c(delete)`, which returns at
    once for an inactive entry.
  - Effect: FLEETS.DAT keeps the previous tick's record. The fleet reappears
    next run with all its ships and its assault order, and attacks again. The
    "lost all contact" reports go out every year.
  - Rust: delete the fleet.
- **B2. Ghost bombardment.** [P: B5/B6, every fleet destroyed, yet armies,
  goods, and dock still fell]
  - Where: when the fleet's last unit dies in §2.4, the combat count `c` is not
    decremented (that branch is skipped), so `6367` still loops over one stale
    dead unit.
  - Effect: a destroyed bombard fleet keeps bombing for all the remaining
    passes.
  - Rust: no passes once the fleet is destroyed.
- **B3. Signed/unsigned damage in the blitz landing (`53bb`).** [C]
  - Where: `Random(dmg)+1-armor < 0` is compared unsigned, so it is not
    clamped.
  - Effect: the target gains HP, and the damage accumulator gains +65535.
    That inflates the collateral `k` up to the `90 × batteries-destroyed
    fraction` cap.
  - Reachable when a battery rolls 1 against a battleship (armor 2).
  - Rust: clamp at 0, as §2.4 does.
- **B4. Transport index bookkeeping in the landing.** [C] A dead scout or
  ETAC shrinks the transport count instead of the start or end index, which
  biases the 40% "aim at transports" pick.
- **B5. Dock removal pick truncated to a byte.** [C]
  - Where: `5d44`'s weighted pick stores `Random(total)+1` in a byte. Once the
    dock holds more than 255 items, the pick is biased.
  - Effect: a pick of 0 decrements slot 1 even when it is empty. The u32
    underflows to 4294967295, and the loop runs on.
  - `6367` uses a word and is fine. Rust: use the full width.
- **B6. Cruiser dock value counted twice** (`3·DD + 9·CA + 9·CA + ...`). [C]
  Possibly a typo. It changes how much of the dock goes per damage point. The
  probes favour 18 (with 9, the dock comparisons drift to z = 1.9-2.8). Rust counts
  cruisers once.
- **B7. Range check against a scattering fleet** (`1b05:7548`). [C] It uses
  `(u.spd − target.spd) × flee_start_round` where elapsed rounds were surely
  meant.
- **B8. Uninitialised side-2 speed.** [C] For a lone starbase (no guard
  fleet), the first distance update reads a never-set stack byte.
- **B9. Possible division by zero in `625a`.** [C] `P / potential` divides by
  zero when potential is 0 and exactly one army is left (Turbo Pascal runtime
  error 200).
- **B10. Invade's 4:1 early stop uses the initial loaded-TT count**, not the
  survivors (`6367`). [C] Rust uses the
  surviving count.
- (Encounter spec) The garbage owner read for worldless sectors is already
  recorded there.

---------------------------------------------------------------------------

## 9. Unresolved / not checked
- **L6 tail.** In L6 (20 TL against 2 batteries) classic shows a slightly
  heavier tail of transport kills during the descent: 18% of runs had 8 or
  more armies killed in transports, against 9% in the model. That gave 2/60
  failed blitzes. All other blitz metrics match.
- **F6** is within z ≈ 2 low on both sides' losses (n = 249). No mechanism
  found.
- **The rest:**
  - The DATABASE.DAT field mapping (`2895:2ddc`) and the `[0x3562]` status
    codes 4, 6, and 7 were only skimmed.
  - Rogues conversion is not probed.
  - Scorch-Earth is not probed.
  - IPBMs in the dock (slot 8) are not probed.
  - Fleet battles involving scatter escape were not isolated, and B7 is not
    probed.
  - The planetary battle with a fleet of only non-combat ships (`c = 0`,
    `Random(0) = 0`, so batteries target unit 1) is not probed.
