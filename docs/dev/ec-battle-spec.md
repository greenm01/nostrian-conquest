# EC Battle Spec

Classic ECMAINT's battles: planetary assault (bombard, invade, blitz), fleet
against fleet, and fleet against starbase. Read from the unpacked ECMAINT
(`tools/unlzexe/ecmaint_640k.bin`, seg:off -> file offset `seg*16+off-0x8140`,
DS = `0x3529`) and checked against classic probes. This is a reference to
classic's behaviour only. Rust does not port it; NC combat is defined in
[nc-combat-spec.md](nc-combat-spec.md).

- **[P]**: confirmed by classic probes (statistically, section 6).
- **[C]**: read from code only.

The encounter loop, sighting, `decide()`, and intercept are in
[ec-encounter-spec.md](ec-encounter-spec.md). This spec starts where those end.
[ec-combat-reference.md](ec-combat-reference.md) summarises the whole system
and lists how NC combat differs.

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
ECMAINT's builds land in the typed dock slots. The PLANETS.DAT codec in
`nc-compat/src/classic/planet.rs` reads and writes this layout.

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
a prefix of it, so the order matters. "Combat count" `c` is the number
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
  - Otherwise, if the dead unit was a fleet unit with `index <= c`, then
    `c -= 1`. Then, after any death on either side, if `c == 0` the battle
    is done: the fleet has no warships left. Its non-combat ships survive,
    because batteries only target combat ships. [P: I3 run 2 ended with 0 BB
    and 15 loaded TT intact.]
  - A fleet with no warships starts at `c = 0`. `Random(0)` is 0, so every
    battery shoots unit 1, and the battle ends as soon as it dies: the fleet
    loses exactly one ship, and its retreat (ROE 0) brings no passes. [P: NW1,
    5 empty TT against 4 batteries, and NW3, 10 loaded TT invading against 2
    batteries: one transport lost in every one of 20 runs, no damage to the
    world, and the invasion "defeated". NW2, 5 TT against no batteries: no
    losses and no damage.]
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
The code adds `9 × CA` twice, which may be a typo (B6). Slot 8 (IPBMs) is an
ordinary dock item here, in §2.6, and in §2.9 [P: IP1, 10 CA bombarding a
world with 50 IPBMs in dock and nothing else: all 50 destroyed, and both
reports read "all of the ship(s) in stardock including 50 IPBMs". IP2, a
blitz capture scaled the 50 to 14-45].
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
    if invade and armies*4 <= L0 and L0 - armies > 4: done = true    // L0 = fleet's initial loaded TT count (B10)
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
       if side == fleet: D += d                   // as in §2.4
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
1. Batteries fire at distances 20, 15, 10, 5, and 0 on the way in. The 40%
   is one branch of the fresh-target pick, not the share of shots that reach
   transports: the other 60% picks uniformly from unit 1 through the last
   transport, which includes the transports. With T transports among those N
   units a fresh pick lands on a transport with probability
   `0.4 + 0.6 × T/N`: 88% for 20 transports behind 5 warships, and 100% with
   no warships at all. Kept targets, the focus rule, deaths, and B4 shift
   later picks.
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
   - **Blitz:** let `f = min(1, sqrt(L / D0))`, where `D0` is the world's
     armies when the assault began: the count in the planet snapshot that
     setup `4672` copies to `ds:0x4b6` before any fire (`[0x50e]`, its
     `+0x58`), which the owner's report prints as "Our defensive force
     consisted of". It is not the attackers' starting count, and it is
     taken before the landing's collateral damage. The division is
     `[0x626] / [0x50e]` at `110a:6c87`, then `Sqrt`, then the cap at 1.0;
     with `D0 = 0`, `f = 1`. Ten survivors against ten starting defenders
     give `f = 1` however many armies landed.
     - `P *= f * (Random(150)/1000 + 0.85)`.
     - `G *= f * (Random(450)/1000 + 0.55)`.
     - Batteries = `Round(batteries_left * f * (Random(650)/1000 + 0.35))`.
     - Each dock slot becomes
       `Round(slot * f * (Random(800)/1000 + 0.2))`.
6. **Scorch-Earth** (`+0x60 != 0`): `P *= 0.5`, `G *= 0.5`, the dock is
   emptied, and the flag is cleared. This runs after the mission damage
   above, and only on a capture: a failed assault leaves the flag set and
   the dock damaged as usual. The yearly economy pass runs before the tick
   loop, so a flagged world is already scorched (P × 0.30-0.49, G × 0.1)
   when the assault comes. [P: SC1, 3 BB + 20 TL invading a flagged world
   with 17 dock items: P 1.7-7.2, G 0.5 in every run (softening took G to
   0, the capture floor made it 1.0, the scorch halved it), dock empty,
   flag 0. SC2, a blitz capture: P 13-21, G 1.2-1.7, dock empty, flag 0.
   SC3 and SC4, a failed invasion and a bombard: flag kept, dock damaged
   normally.]
7. `P = max(P, 1.0)`.

The owner's capture report always carries "When we realized that the battle
was lost, we tried to destroy anything we thought the aliens could use"
(`110a:3cb5`, gated on the capture), whether or not Scorch-Earth was set.

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

### 2.11 After the assault: the world and empires [C][P]
- Write the work planet back (`7026`/`7188`/`72ed`). Reports follow (§4).
- **Rogues** (`110a:6f13`) [P]: after the writeback, take the world's current
  owner. If that empire slot is in civil disorder (player `+0x00 = 0`), sum
  present production P and potential over all its worlds (`2883:0000`,
  `2883:0098`). If the potential sum is 0, or `P / potential < 0.75`, the
  slot becomes the AI empire: `+0x00 = 0xff`, name "Rogues", handle blanked,
  and its diplomacy marks only the attacker's empire an enemy (every other
  slot is set to 0). The check runs after every assault, even one that did
  no damage, and it reads the owner after the capture step: a captured world
  belongs to the attacker by then, so a capture never converts the old
  owner. This is the one way a fight changes stored diplomacy; fleet
  battles never do (§3.3). [P: 1 CA bombarding a civil-disorder world whose
  P stood at 60%, 68%, and 41% of potential after the yearly growth and the
  raid: converted,
  diplomacy `[1,0,0,0]`. At 98%: not converted. A blitz capture of a 40%
  world: not converted. The threshold is 0.75 (`80 00 00 00 00 40` as real48),
  not 0.5.]
- **Planet status table** `[0x3562 + 25*planet + empire]` [C]: a per-run
  table, zeroed at start-up (`320d:0085`), that tells later intelligence
  reports what happened to a world since an empire last looked at it. The
  dispatcher writes 1 when a View fleet, or a colonizer that finds aliens,
  looks at a world; 2 when a Scout System fleet scans it; and 3, for every
  empire with a nonzero entry, when a colony is founded. After an assault,
  every empire whose entry is neither 0 nor 1 gets 4 (invade), 6 (bombard),
  or 7 (blitz). A later Scout System arrival then reports "We have an update
  concerning the world in System(x,y). The world has been attacked by an
  invasion fleet / The world has been attacked by a blitzing fleet / The
  world has been bombarded. We have compiled the following new data
  concerning this world:" and the full scan; a second scan in the same run
  (entry already 2) reports nothing. A later View reports "has been under an
  invasion attack since it was last viewed or scouted. Its present owner is
  E." for 4, tests 8 for a blitz, which nothing writes, and has no text for 6
  (bug B11). Code 5, "has been hit by an IPBM", is never written.
**DATABASE.DAT** (`2895:2ddc(planet, empire, mode)`) [C][P] is the
per-empire world intelligence file: one 100-byte record per (empire, world),
at record `(empire - 1) * planet_count + planet - 1`.

| Offset | Type | Field |
| --- | --- | --- |
| `+0x00` | String[20] | world name ("UNKNOWN" until seen) |
| `+0x15` | u8 | owner (0xff unknown) |
| `+0x16` | u16 | year of the last update |
| `+0x18` | u16 | year of the last detailed scan (0 none) |
| `+0x1a` | u16 | not written by ECMAINT |
| `+0x1c` | u8 | potential production (0xff unknown) |
| `+0x1d` | u8 | present production, `Trunc(P + 0.5)` (0xff unknown) |
| `+0x1e` | u16 | stored goods, `Trunc(G + 0.5)` (0xffff unknown) |
| `+0x20` | u8 | previous owner (0xff none) |
| `+0x21` | u16 | year captured (0 none) |
| `+0x23` | u16 | armies (0xffff unknown) |
| `+0x25` | u16 | ground batteries (0xffff unknown) |
| `+0x27` | u16 | year of the last detailed scan, again |
| `+0x29..+0x44` | 14 × u16 | zeroed by a detailed scan; ECMAINT never reads them |
| `+0x45..+0x63` | 31 bytes | not written by ECMAINT |

Both modes copy the name, set the owner, the year, and the potential, and
store the capture year when the world's status byte is 0 (captured this
year). When the owner changed, both store the old owner in `+0x20`. A
**mode-2** update (a detailed scan) also fills production, goods, armies,
batteries, and `+0x18`/`+0x27`, and zeroes `+0x29..+0x44`. A **mode-1**
update (owner and year) resets production, goods, armies, batteries, and
`+0x18` to unknown when the owner changed. The year is the game year being
simulated. Callers:
- after every assault: mode 2 for the attacker, and for the old owner when
  the world changed hands (`0b5d:14da`, `18e4`, `1905`, `1d13`, `1d34`);
- after a fleet battle: mode 1 for each side in layer 1 at a world its
  empire does not own (`1b05:985a`, `98fb`);
- a View arrival: mode 1 (`0b5d:2640`); a Scout System arrival: mode 2,
  unless the status table already holds 2 (`33d4`); a colonizer that finds
  an alien world: mode 1 (`4133`);
- the end of the run (`0824:252a`, `25a3`, "Updating intelligence
  database..."): mode 2 for every owned world and its owner, then mode 1
  for every active unit in layer 1 (fleets, starbases, and IPBMs alike)
  whose empire does not own the world beneath it.

[P: the attacker's record of a civil-disorder world went from "UNKNOWN" and
0xff everywhere to the world's name, owner 3, the year, potential 100,
production 76, goods 10, armies 5, batteries 0, previous owner 0xff, after
one bombard; the owner's own record took the same values at the end of the
run.]

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
  fleet has an **uninitialised** speed byte at this point (B8). The
  recompute at the end of round 1 overwrites it, so it only changes the
  first distance step (§3.4).
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
  [P: X1, 15 DD (speed 6) against 1 CA + 2 TT at ROE 0, 104 battles: the
  attacker lost 2.48 DD; the model gives 2.46 with B7 and 1.74 without it.]
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
  - A unit that an enemy is shooting at never escapes, because the pursuer's
    range always exceeds a negative gap. Shooters keep a target while it
    lives, and before the scatter flag is set they all pick warships, so the
    transports behind a warship are untouched until it dies. [P: X1: in 53
    of 104 battles both transports escaped untouched and the cruiser died;
    in 48 all three died; the model gives 49% and 46%. X2, 4 DD against 8 TT:
    3.8 transports lost (model 3.8) and the rest escaped. Every scattered
    fleet went Seek Home with ROE 0, and most were caught again while they
    climbed out.]
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
3. The attacker record is saved, or deleted if nothing is left
   (`1b05:9927`).
4. The target fleet is saved if `total2 > 1`, or `total2 == 1` with no base or
   with the base killed. Otherwise it is deleted (`1b05:9966`). Here the
   delete works, because the entry is still active. Each delete lets go of
   the fleet's starbase if the fleet is on Guard Starbase (`2895:3d46`,
   [ec-starbase-spec.md](ec-starbase-spec.md)).
5. A killed base is deleted (`2895:3ebd`).
6. Each side in layer 1 at a world its empire does not own gets the
   DATABASE.DAT owner-and-year update (§2.11, mode 1).
7. Order changes:
   - **Scatter** (`1b05:2562`): if the owner still has a planet and the order
     is not 2 or 15, the fleet first lets go of its starbase (`2895:30f4`,
     [ec-starbase-spec.md](ec-starbase-spec.md); it does so even when the pick
     below finds no world, and the fleet then keeps its order), then goes
     Seek Home to the nearest friendly system
     (`24a3:0876`) with TZ 1, **ROE 0**, order 2, and speed = max. The report
     reads "After reassembling our fleet, we've decided to abort our mission
     ... and to seek the safety of a friendly controlled solar system at
     maximum speed. Until ordered otherwise, we are going to avoid contact
     with alien crafts." [P: F6 targets ended at their home world (15,13)
     after Seek Home.]
   - **Retreat** (`1b05:2e1b`): this applies only to a fleet at its target
     sector with `Round(Z) == 1` and an order other than 2 or 15. The fleets
     it re-orders are on orders 5 to 8 and 12, so its escort release
     (`1b05:3151`) finds nothing.
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
   Stored diplomacy never changes; the one post-fight rewrite, the Rogues
   conversion, follows assaults only (§2.11).

### 3.4 Starbase fights [C][P]
A starbase is attacked only through the encounter: by a declared enemy, or
by an assault fleet whose target world it protects (encounter spec §4, and
the gate in §2.2). It never attacks on its own. In the battle it is one unit
on side 2, last in order, with range 25, 60 HP, max damage 36, armor 3, and
a 60% hit chance. It fires like any unit, but only when it falls inside the
engagement prefix (§1.3): a base behind a large guard fleet may not fire at
all. A lone base always fires, because `k >= 1` for any enemy value. Side 2
fights at ROE 10 while the base lives, never retreats, and never scatters
when the base is alone (`init2 == 1`) or has a one-ship guard
(`init2 == 2`). The base cannot move, so a fight with a lone base ends only
when the base or the attacker is destroyed, or when the attacker retreats or
scatters out of range.
- The lone base's side-2 speed starts uninitialised (B8). The recompute at
  the end of round 1 sets it to 1, and the first distance step can only
  close the range faster, so B8 has no visible effect: the model gives the
  same result for any starting value from 0 to 100.
- A killed base is deleted from BASES.DAT (`2895:3ebd`); a guard's escort
  word is left stale ([ec-starbase-spec.md](ec-starbase-spec.md)).
- An assault fleet that kills the base assaults the world in the same tick
  (§2.2). [P: S4, 60 runs: the base died every time and the surviving
  battleships bombarded the 4-battery world at once.]

[P: S4, 4 BB against a lone base: 0.95 BB lost, base killed 100% (model
0.84, 99%). S6, 8 CA: 3.13 CA, 93% (3.22, 97%). S7, 2 BB: 1.05 BB, 67%
(1.16, 64%). S8, 1 DD: the destroyer always dies without scoring. With a
guard, S1-S3 in §6.2.]

### 3.5 IPBMs [C][P]
IPBM.DAT missiles never fight. The unit table loads them as kind 3, the
encounter skips kind 3 as observer and as target, no battle targets them,
and their arrival does nothing ([ec-file-formats.md](ec-file-formats.md)).
The only IPBM content in combat is the stardock: slot 8 is a dock item worth
2 in the dock value (§2.5), destroyed by bombardment with the rest of the
dock and scaled on capture like every other slot [P: IP1, IP2]. The View
report's "has been hit by an IPBM" line (status 5, §2.11) is never reached.

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
- Against a starbase: "We successfully intercepted Starbase N of E. We had 4
  battleships. The aliens were completely destroyed. We lost 2 battleships."
  The base's owner gets "We lost all contact with Starbase N shortly after it
  was attacked by the F Fleet of E ...", or "We were attacked by ... The
  aliens were completely destroyed. Luckily, we suffered no losses." when the
  base holds.
- A scattered fleet adds "After reassembling our fleet, we've decided to seek
  the safety of a friendly controlled solar system at maximum speed. Until
  ordered otherwise, we are going to avoid contact with alien crafts. Our new
  destination is planet "N", located in System(x,y)."

---------------------------------------------------------------------------

## 5. Order and record effects, summary
| Event | Fleet record afterwards |
| --- | --- |
| Assault, fleet survives (any result) | order 0 Hold, counts = survivors (landed TT empty), Z 1.0 |
| Assault, fleet destroyed | unchanged pre-tick record (bug B1) |
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
| L6 re-run (n = 200) and pooled (n = 259) | captured / armies killed in transports | 0.995 / 5.40; pooled 0.988 / 5.56 | 0.997 / 5.47 (see §9) |

Of 89 metric comparisons, 88 are within 2σ (the exception was L6's capture
rate; a larger sample closed it, see §9). Before the engagement-count fix
(A/B, not B/A) the bombard battles were off by z = 10.9. Pricing the cruiser dock slot at 9 instead of 18
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
| F6 re-run (n = 249) and pooled (n = 498) | 1.88 / 2.26, 1.35; pooled 1.63 / 2.06, 1.19 | 1.57 / 2.07, 1.23 (see §9) |
| X1 15 DD (speed 6) vs 1 CA + 2 TT, T ROE 0 (n = 104) | 2.48 DD / 1 CA, 0.95 TT | 2.46 / 1, 0.97 (1.74 DD without B7) |
| X2 4 DD vs 8 TT, T ROE 0 (n = 30) | 0 / 3.83 TT | 0 / 3.82 |
| S4 4 BB vs lone starbase (n = 60) | 0.95 BB, base killed 100% | 0.84, 99% |
| S6 8 CA vs lone starbase (n = 60) | 3.13 CA, base killed 93% | 3.22, 97% |
| S7 2 BB vs lone starbase (n = 100) | 1.05 BB, base killed 67% | 1.16, 64% |
| S8 1 DD vs lone starbase (n = 20) | 1 DD, base killed 0% | 1, 0% |

Blitz landing model (10 or 20 loaded TT, no escort; mean armies landed / share
of runs with none landed):

| Batteries | 0 | 1 | 2 | 4 | 8 |
| --- | --- | --- | --- | --- | --- |
| 10 TL | 10 / 0 | 7.2 / 0 | 4.6 / 0 | 0.96 / 48% | 0 / 100% |
| 20 TL | 20 / 0 | 17.2 / 0 | 14.5 / 0 | 9.5 / 0 | 1.9 / 28% |

### 6.3 Deterministic probes (t045; 10 runs each unless noted)
| Case | Result |
| --- | --- |
| R50, R60, R30: 1 CA bombards a civil-disorder world with P 50, 60, 30 (2 runs each) | Rogues conversion at P/potential 0.60, 0.68, 0.41; diplomacy enemy to the attacker only |
| R100: the same at P 100 | not converted (0.98) |
| R30_blitz: 20 TL blitz the P 30 world | captured; the old owner not converted |
| R30_scorch: 1 CA bombards the P 30 world flagged Scorch-Earth | converted; flag kept |
| SC1: 3 BB + 20 TL invade a Scorch-Earth world holding 17 dock items | captured; P 1.7-7.2, G 0.5, dock empty, flag cleared |
| SC2: 20 TL blitz the same world | captured; P 13-21, G 1.2-1.7, dock empty, flag cleared |
| SC3: 10 TL invade the same world (no warships) | failed; flag kept, dock lost 1-2 items to collateral |
| SC4: 3 CA bombard the same world | flag kept; dock wrecked as usual |
| IP1: 10 CA bombard a world with 50 IPBMs in dock | all 50 destroyed and named in both reports |
| IP2: 20 TL blitz the same world | captured 9 of 10; slot 8 scaled to 14-45 |
| NW1: 5 empty TT bombard against 4 batteries | 1 TT lost every run, no damage, Hold |
| NW2: 5 empty TT bombard against 0 batteries | no losses, no damage, Hold |
| NW3: 10 TL invade against 2 batteries | 1 TL lost every run, "invasion attempt was defeated", Hold |

---------------------------------------------------------------------------

## 7. NC combat
Rust does not port this battle ([NC combat stays](../notes/decisions/b5u6lekb-rust-keeps-its-own-combat-system.md)).
The differences between NC combat and classic are listed in
[ec-combat-reference.md](ec-combat-reference.md), "How NC differs".

---------------------------------------------------------------------------

## 8. Classic bugs

These are classic's defects, recorded for reference. NC combat does not
inherit them, because it does not port the battle (§7).
- **B1. A destroyed assault fleet is resurrected.** [P]
  - Where: `110a:5169` (orbital battle), `5a9e` (landing), and `6a87` mark the
    unit entry inactive before calling `2895:3d0c(delete)`, which returns at
    once for an inactive entry.
  - Effect: FLEETS.DAT keeps the previous tick's record. The fleet reappears
    next run with all its ships and its assault order, and attacks again. The
    "lost all contact" reports go out every year.
- **B2. Ghost bombardment.** [P: B5/B6, every fleet destroyed, yet armies,
  goods, and dock still fell]
  - Where: when the fleet's last unit dies in §2.4, the combat count `c` is not
    decremented (that branch is skipped), so `6367` still loops over one stale
    dead unit.
  - Effect: a destroyed bombard fleet keeps bombing for all the remaining
    passes.
- **B3. Signed/unsigned damage in the blitz landing (`53bb`).** [C]
  - Where: `110a:5772`, `cmp [bp-0x1a], 0; jae`: the clamp of
    `Random(dmg)+1-armor` to 0 compares unsigned, so it never fires, and a
    result of -1 stays 0xffff.
  - Effect: the hit-point update subtracts the zero-extended word in 32-bit
    arithmetic and stores the low word, so the target gains one hit point
    (a fresh BB goes from 30 to 31). Nothing else changes. The damage
    accumulator `D` is untouched: `110a:577d` adds to it only when the
    target side is the planet, so it sums the fleet's fire on the
    batteries, and a fleet shot against a battery (armor 1) is
    `Random(dmg)+1-1 >= 0`, never negative. Collateral damage is unaffected.
  - Reachable only when a battery (damage 36) rolls 1 against a battleship
    (armor 2): one hit in 36 on a BB. The orbital battle `4ba4` clamps with
    a signed compare (`jge` at `110a:4e91`) and has no such bug.
  - `tools/classic-model/sim.py` models it this way: the BB heals by one and
    `D` is unchanged.
- **B4. Transport index bookkeeping in the landing.** [C] A dead scout or
  ETAC shrinks the transport count instead of the start or end index, which
  biases the 40% "aim at transports" pick.
- **B5. Dock removal pick truncated to a byte.** [C]
  - Where: `5d44`'s weighted pick stores `Random(total)+1` in a byte. Once the
    dock holds more than 255 items, the pick is biased.
  - Effect: a pick of 0 decrements slot 1 even when it is empty. The u32
    underflows to 4294967295, and the loop runs on.
  - `6367` uses a word and is fine.
- **B6. Cruiser dock value counted twice** (`3·DD + 9·CA + 9·CA + ...`). [C]
  Possibly a typo. It changes how much of the dock goes per damage point. The
  probes favour 18 (with 9, the dock comparisons drift to z = 1.9-2.8).
- **B7. Range check against a scattering fleet** (`1b05:7548`). [P] It uses
  `(u.spd − target.spd) × flee_start_round` where elapsed rounds were surely
  meant. A pursuer faster than the scatterer gets into range later than it
  should, so the scatterer's warships get more shots (X1: 2.48 DD lost
  against 1.74 without the bug).
- **B8. Uninitialised side-2 speed.** [C] For a lone starbase (no guard
  fleet), the first distance update reads a never-set stack byte. The round-1
  recompute replaces it, and the model's result is the same for any value
  from 0 to 100, so it cannot be seen from outside (§3.4).
- **B9. Possible division by zero in `625a`.** [C] `P / potential` divides by
  zero when potential is 0 and exactly one army is left (Turbo Pascal runtime
  error 200).
- **B10. Invade's 4:1 early stop uses the initial loaded-TT count**, not the
  survivors (`6367`). [C]
- **B11. The View report tests the wrong blitz code.** [C] The View arrival
  (`0b5d:24b2`) compares the planet status (§2.11) with 8 for "has been under
  a blitz attack since it was last viewed or scouted", but a blitz writes 7,
  and a bombard (6) has no View text at all. Only an invasion (4) and a new
  colony (3) reach a viewing fleet.
- (Encounter spec) The garbage owner read for worldless sectors is already
  recorded there.

---------------------------------------------------------------------------

## 9. Unresolved / not checked
- **L6 tail: sampling noise.** The first 60 L6 runs (20 TL against 2
  batteries and 5 armies) showed 8 or more armies killed in transports in 18%
  of runs (model 9%) and 2 failed blitzes. A second sample of 200 runs showed
  10% and 1 failure. Pooled (n = 259): 5.56 killed on average (model 5.47),
  8 or more in 12.0% (9.5%), 9 or more in 3.5% (2.0%), all within 1.7σ, and
  the ground battle's losses by landed count match the model (814 in total
  against 802 expected). Three failures against 1.1 expected is within chance
  (p ≈ 0.09). No mechanism was found and none is needed.
- **F6: sampling noise.** Both sides' low losses came from the share of
  battles that end before contact: a start distance above 15 (9 of 20) ends
  the fight at once when the target retreats at equal speed. The first
  sample had 50.6% such battles, the re-run 39.8%, the model 45.8%. Pooled
  (n = 498): 45.2%, and the losses given contact are 2.97 / 3.77 CA and 2.18
  TT against the model's 2.90 / 3.83 / 2.27.
- **Still not checked:**
  - The status-table report texts (§2.11) were read from code, not probed.
  - DATABASE.DAT bytes `+0x1a` and `+0x29..+0x63` belong to ECGAME; what it
    stores there was not decoded.
  - Which stack byte B8 reads was not traced; it has no visible effect.
  - AI (Rogue) empires' own assaults were not probed.
