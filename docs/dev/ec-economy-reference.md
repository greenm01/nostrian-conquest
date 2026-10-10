# Classic Esterian Conquest: Economy Reference

This document records the economy of the 1992 DOS game, decoded from the
v1.5 binaries and checked against them. It is a reference. Nostrian Conquest
keeps its own economy, defined in [economics.md](economics.md); the last
section compares the two.

Sources: ECMAINT's yearly planet pass (`1852:0000`), its driver and reports,
ECGAME's tax, build, commission, review, profile, and rankings screens, and
ECUTIL's seeding. Each rule carries a mark:

- `[P]` probe-confirmed: ECMAINT ran under DOSBox-X on a patched fixture and
  the output matched the rule exactly.
- `[C]` code-only: read from the disassembly, not yet run.

Address conventions:

- ECMAINT: `seg:off` in the live image (`tools/unlzexe/ecmaint_640k.bin`).
  The dump offset used by [ec-file-formats.md](ec-file-formats.md) is
  `seg * 16 + off - 0x8140`, so `1852:0457` is dump `0x10837`.
- ECGAME: file offsets in `tools/unlzexe/ecgame_640k.bin`, as in
  ec-file-formats.md. A far call `seg:off` lands at file `seg * 16 + off`.
- ECUTIL: physical addresses, as in ec-file-formats.md.
- Reals are Turbo Pascal 6-byte reals. The runtime in ECMAINT segment `3374`
  is: add `112b`, sub `1131`, sqr `1137`, mul `113d`, div `1143`, cmp `114d`,
  float `1151`, trunc `1155`, round `115d`, sqrt `1204`, sin `1276`,
  arctan `144d`, `Random(n)` `15ea`, `Random` `1600`. `Random(n)` returns
  0..n-1.

## Summary

Every world has a potential (a byte, 10..150, fixed at setup), a present
production P (a real), and stored goods G (a real). Goods live on the world.
There is no empire treasury. The empire sets one tax rate, 0..100.

Once a year, before any fleet moves, ECMAINT visits every owned world in
record order and does this:

1. Steps the world's status (0 just captured, 1 recently captured, 2 normal)
   and picks a scale from the old value: 0, 0.5, or 1.0.
2. If the owner has a starbase and one is at rest in orbit here, rolls a tax
   cut of `Random(10)` points (0..9).
3. Moves the build queue into the stardock and onto the surface, and charges
   the full cost against G. Nothing limits this; G can go negative.
4. If the world is under Scorch-Earth, cuts G to `min(G/10, sqrt(G))` and P to
   30..49% of itself, and stops.
5. Grows P. Production is a point on a quarter circle: `P = potential *
   sin(angle)`. The angle advances by 7.5 degrees a year, scaled by status
   and by a tax factor `1 - (tax - cut + 5 - Random(11)) / 70`. At 0% tax a
   world goes from nothing to full in 12 years; at 50% in 42; at 70% it
   stands still on average; above that it shrinks. P is then rounded to
   0.001.
6. If tax > 70, cuts P again by `(tax - 70) * (0.5..1.5)` percent.
7. Raises P to at least 1.0.
8. Adds income `P * tax / 100 * scale` to G and caps G at 65535.

The "65%" advice in the manuals is exact: 65% is the highest rate at which
the growth term can never be negative. At 66% one roll in eleven shrinks the
world; at 70% five do. A starbase's cut of 0..9 points is what lets a base
world "endure 67% to 70%, but there is no guarantee".

In ECGAME the player can spend, per world and per turn, `Trunc(min(G, P))`
points, or `Trunc(min(G, 5 * P))` when one of his starbases orbits the world.
Ships cost 5/15/45 (DD/CA/BB), 15 (scout), 5 (transport), 20 (ETAC), 50
(starbase); armies 2, batteries 20. Orders complete in full at the next
maintenance. Ships and bases wait in the stardock until commissioned; armies
and batteries go straight to the surface. An empire may have at most 500
fleets and bases.

Rankings are not a score. ECMAINT's `RANKINGS.TXT` and ECGAME's rankings
screen sort empires by production (the truncated sum of P over their worlds)
or by planet count, and show last year's values in parentheses.

## The yearly planet pass (`1852:0000`)

### Order of operations

The pass loops over PLANETS records 1..n (`DS:1714`) and skips worlds with
owner 0. For each owned world, in this order:

| Step | Addresses | Rule |
|---|---|---|
| Status | `1852:0056..00a3` | Step status, pick scale. |
| Starbase cut | `1852:011b..0206` | Roll the tax cut if a base is at rest in orbit. |
| Builds | `1852:0206..0445` | Queue to dock and surface; charge G. |
| Scorch | `1852:0449`, `0831..094a` | If `0x60` is set, shrink G and P, skip growth and income. |
| Growth | `1852:0457..063e` | Angle model. |
| Rounding | `1852:063e..0682` | `P = Round(P * 1000) / 1000`. |
| High-tax penalty | `1852:0682..074f` | If tax > 70, shrink P. |
| Floor | `1852:074f..0783` | `P = max(P, 1.0)`. |
| Income | `1852:0783..07f5` | `G += P * tax / 100 * scale`. |
| Cap | `1852:07f9..082e` | `G = min(G, 65535)`. |
| Clear queue | `1852:094a..0969` | Zero all ten queue slots. |

The pass runs for every owned world whatever the owner's mode. An empire in
civil disorder (PLAYER `0x00` = 0) grows and collects like any other; it just
keeps whatever tax rate it has. `[P]` mode 0 and mode 1 owners gave the same
results.

### Planet status (`0x5C`)

`[P]` The status byte counts years under the current owner, capped at 2. The
pass reads it, steps it, and keeps a scale from the old value:

| Status at entry | New status | Scale |
|---|---|---|
| 0 | 1 | 0.0 |
| 1 | 2 | 0.5 |
| 2 | 2 | 1.0 |

The scale multiplies both the growth step and the income. A just-captured
world neither grows nor pays; a recently captured one does half of each. The
high-tax penalty, the floor, and the build charges ignore the scale.

Capture sets status 0 (ECMAINT dump `0xFA41`). Capture happens inside the
52-tick year, after that year's pass, so a captured world pays nothing the
next year, half the year after, and full from the third year. Colonizing
sets status 2 (dump `0x7858`), so a colony pays in full from its first pass.

Probe: status 0, P 50, tax 50 left P and G unchanged and set status 1.
Status 1 gave half the growth step and half the income (P 51.210, G +12.8025
= 0.5 x 51.210 x 0.5) and set status 2.

### Starbase tax cut

`[C][P]` At `1852:012f` the pass reads the owner's base count (PLAYER
`0x44`). If it is zero, the cut is 0. Otherwise it scans the unit table for
an entry of kind 2 (a base) in the world's sector at depth layer 1
(`unit+5 != 0`), loads the base record (`2895:374a`), and requires its moving
flag (BASES `0x1f`) to be 0. The first such base stops the scan and sets
`cut = Random(10)`, a real in 0..9. The scan does not check the base's owner.

The cut lowers the tax rate in the growth term only. It does not change
income. One roll in ten gives no cut at all.

The t041 probes fixed the rule: a base at rest in orbit counts; a base in
deep space, a base under way, and a base at depth 0.4999 do not; another
empire's base at rest counts as long as the owner has some base somewhere.
At tax 70 the growth outcomes without a base covered `Random(11) - 5` =
-5..+5; with a base they reached +10 or more, which needs the cut.

The cut is stored in a stack slot (`[bp-0x54]`) that is written only when a
base is found (`1852:01e9`) or when the owner has no bases (`1852:01f7`).
See "Classic bugs" for the leak this causes.

### Build completion

`[P]` Queue slots 1..8 (PLANETS `0x24 + 2(k-1)`, kinds 1 destroyer,
2 cruiser, 3 battleship, 4 scout, 5 transport, 6 ETAC, 7 starbase,
8 reserved) move into stardock slots 1..8 (`0x38 + 4(k-1)`) at
`1852:0206..0325`. Each dock slot saturates at 65535. Slot 9 adds to armies
(`0x58`) and slot 10 to ground batteries (`0x5A`), both saturating at 65535
(`1852:0328..0445`).

The costs are a table of reals built on the stack at `1852:00a3..011a`:

| Kind | Unit | Cost |
|---|---|---|
| 1 | Destroyer | 5 |
| 2 | Cruiser | 15 |
| 3 | Battleship | 45 |
| 4 | Scout | 15 |
| 5 | Troop transport | 5 |
| 6 | ETAC | 20 |
| 7 | Starbase | 50 |
| 8 | (reserved) | 100 |
| 9 | Army | 2 (`1852:0376`) |
| 10 | Ground battery | 20 (`1852:0406`) |

For every slot, `G -= queue[k] * cost[k]`, then `queue[k] = 0`. Nothing checks
G first. The charge happens before the year's income, so this year's income
never pays for this year's orders.

Probes: the build fixture pair (`fixtures/ecmaint-build-pre/post`) queued
3 destroyers and 1 ETAC with G 35 and ended with dock [3,0,0,0,0,1,0,0] and
G 0. A queue of 2 DD, 1 CA, 1 scout, 2 TT, 1 ETAC, 5 armies, 1 battery on a
world with P 100, G 200 at 50% tax ended with dock [2,1,0,1,2,1,0,0], 15
armies, and G 150 = 200 - 100 + 50. Three battleships queued against G 35
completed anyway and left G at -73.003 (-100 plus the year's income).

### Scorch-Earth (`0x60`)

`[P]` If the flag is set, after the builds (`1852:0831..094a`):

- `G = min(G / 10, sqrt(G))` (`1852:0831..08c2`). Below G = 100 that is a
  tenth; above, the square root.
- `P = P * (0.30 + Random(20) / 100)`, a factor of 0.30..0.49
  (`1852:08c6..0912`).
- `P = max(P, 1.0)`.
- No growth, no income. The flag stays set, so the world keeps shrinking each
  year until the order is rescinded or the world falls.

The file-formats scorch probe saw G 35 -> 3.5 and 66 -> 6.6 and P 100 ->
39..47.

### Growth

`[P]` Growth is an angle on a quarter circle (`1852:0457..063e`):

```text
ratio = P / potential                                   (1852:047e)
angle = arctan(ratio / sqrt(1 - sqr(ratio)))            = arcsin(ratio); 1852:0495..04ac, 059a..059f
                                                         (sqrt(...) is replaced by 1e-5 when it is 0; 1852:04c9..04da)
roll  = 5 - Random(11)                                  -5..+5; 1852:04df..04fd
eff   = tax - cut + 5 - Random(11)                      1852:0505..0537
step  = (pi / 24) * scale * (1 - eff * 1.42857 / 100)   1852:053c..0580
angle = clamp(angle + step, 0, pi / 2)                   1852:05a7..0604
P     = potential * sin(angle)                           1852:060d..063a
```

The constants are literal: `1.42857` (not 1/0.7 exactly), `pi/24` (7.5
degrees), `pi/2`, and `0.00001`. So the tax factor is `1 - eff / 70.0001`.

What this means:

- The step is a fixed share of the quarter circle, 7.5 degrees at 0% tax
  (12 years from zero to full), 3.75 at 35% (24 years), 2.14 at 50% (42
  years), 0.54 at 65% (168 years), 0 at 70%. Each point of tax is worth
  1/70 of the step.
- Because P follows a sine, a young world gains many points a year and a
  nearly full one almost none. From P 1 at 0% tax: 14, 27, 39, 62 (year 5),
  97 (year 10). At 50%: 4.7, 8.5, 12, 20 (year 5), 38 (year 10), 69 (year
  20).
- The roll adds -5..+5 points of noise to the tax rate. At 65% the factor is
  `(R) / 70 >= 0` for every roll: the world never shrinks. At 66% roll 0
  shrinks it; at 70% rolls 0..4 do; at 75% all but roll 10; from 76% every
  roll.
- The starbase cut of 0..9 points shifts that boundary up by the roll: at
  70% a base world shrinks only when `cut + R < 5`.
- `clamp` keeps a full world at its potential and a shrinking one at or
  above 0 (the floor later makes it 1.0).
- Scale 0 (just captured) freezes the angle; scale 0.5 halves the step.

Probes (fixture `ecmaint-econ-pre`, world 15, potential 100, P 50, G 35,
status 2; distinct outcomes listed with the roll `R = Random(11)` that
reproduces them to 0.001):

| Tax | P after | Roll | G after |
|---|---|---|---|
| 0 | 61.468, 60.876 | 9, 5 | 35.000 |
| 35 | 55.868, 56.177 | 7, 9 | 54.554, 54.662 |
| 50 | 53.835, 53.520, 52.409 | 9, 7, 0 | 61.918, 61.760, 61.204 |
| 65 | 51.129, 50.808 | 7, 5 | 68.234, 68.025 |
| 70 | 50.324 | 7 | 70.227 |

Also: P 1.0 at 50% became 5.672 (roll 10) and at 0% became 14.969 (roll 10);
P 10 with potential 30 at 50% became 11.207 (roll 8); P 100 stayed 100. In
every run `G_after - G_before = tax * P_after / 100 * scale` to the last bit
of the real.

### High-tax penalty

`[P]` After growth, if `tax > 70` (`1852:06ab`, compared as reals):

```text
P = P * (1 - (tax - 70) * (Random + 0.5) * 1.0 / 100)     1852:06b5..074b
```

`Random` is the real generator in [0, 1), so the factor is `(tax - 70) *
0.5..1.5` percent: at 80% the world keeps 85..95% of P; at 100% it keeps
55..85%. The `* 1.0` is a dead multiply left in the code. The penalty applies
whatever the status and whether or not a base is in orbit.

Probes at P 50: tax 80 gave 43.241 and 41.547; tax 100 gave 34.512, 31.589,
and 26.974. All lie inside the band for some roll.

### Rounding, floor, income, cap

`[P]`

- `P = Round(P * 1000) / 1000` (`1852:063e..067e`).
- `if P < 1.0 then P = 1.0` (`1852:074f..0783`). Colonizing also writes 1.0.
- `G = G + tax * P / 100 * scale` (`1852:0783..07f5`). P is the value after
  growth and penalty. G is a real, so fractions accumulate: a colony at P 1
  and 50% tax banks 0.5 a year.
- `if G > 65535 then G = 65535` (`1852:07f9..082e`). Probe: G 65500 at 100%
  tax on P 100 ended at exactly 65535.000.

## Around the pass

### Year order

`[C]` The driver (`0824:29bf..29ec`) runs each year as:

1. Copy every empire's production (PLAYER `0x52`) and planet count (`0x50`)
   into CONQUEST.DAT's previous-year arrays (`0824:26a5`).
2. Run the rogue and autopilot pass (`24a3:3cc6`): it sets tax and queues
   builds for those empires.
3. Run the planet pass (`1852:0000`, called at `0824:29d1`).
4. Run the 52-tick fleet year (`0824:1004`).
5. Increment the year (`DS:190C`).

So builds ordered last turn complete before any fleet moves, but a newly
built ship sits in the dock through the whole year until the player
commissions it in ECGAME. Capture and colonization happen during step 4,
after the pass.

### Recounts and previous-year arrays

`[P]` After the last year the driver recounts every empire (`0824:1271`,
called at `0824:2a37`): `PLAYER 0x50 = count of PLANETS with owner = i` and
`PLAYER 0x52 = Trunc(sum of P over those worlds)` (`2883:0000`). The probe
with P 50 -> 61.468 ended with production 61. During the run, capture
adjusts `0x50` directly (dump `0x78C8`, `0xFA2A`, `0xFA3D`).

The previous-year arrays are filled at the start of each year from the
values the last recount left, so within one multi-year run they do not move.

### Rankings

`[P]` With the `/R` switch (ECREADME.DOC: "run ECMAINT and generate rankings
file"; `DS:16A1`), the driver writes `RANKINGS.TXT` after the recount
(`0aca:07b5`, called at `0824:2a49`). The file has a creation date, the
stardate, and three tables: "by ID", "by Current Production", "by Planets
Owned", each with Empire Name, ID, Planets Owned, Current Production, and
Status (Alive or *Dead*), with last year's planets and production in
parentheses.

`[C]` The table builder (`0aca:01ab..`) reads PLAYER `0x52` and `0x50` and
the previous-year arrays for empires 1..player_count. The sort
(`0aca:02ca..037a`) is a bubble sort, descending, on the production word or
the planet-count byte; ties keep the lower ID first. Empires that have not
joined are left out (the probe fixture, with no joined empire, produced three
empty tables).

ECGAME's "O>ther empires (rankings)" screen (file `0x36D13` region) shows the
same columns, sortable by ID, production, or planets, "Previous Year's
Information is Parenthesized". The profile screen prints the empire's rank
by planets and by production. There is no composite score anywhere.

### The rogue and autopilot tax

`[C]` The AI pass sets an empire's tax at `24a3:0fb5..1037` (called from
`24a3:2a30` and `24a3:2a61`):

```text
if sum(potential) > 0 then tax = Round(65 * sum(P) / sum(potential))
else tax = 0
```

`sum(P)` is `2883:0000` (reals), `sum(potential)` is `2883:0098` (bytes),
both over the empire's worlds. A fully developed AI empire taxes at 65%; an
empire whose worlds average half their potential taxes at about 33%. The AI's
build orders go straight into the queue (dump `0x1DB6E..0x1DDAB`); see
ec-file-formats.md.

## ECGAME

### Tax rate

`[C]` Planet Command `T` (file `0x10FAB..0x10FFB`) prints "Warning: Taxes in
excess of 65% may actually REDUCE your planets' productivities!" and reads an
integer 0..100 with the current rate as the default, then stores it in
PLAYER `0x51`. A joining player starts at 50 (file `0x2E973`); ECUTIL seeds
0.

### Build screen

`[C]` Planet Command `B` (file `0x1258B`) first tests for a starbase in orbit
(file `0x12616..0x126D4`; economics.md cites this as image offset `0xa3fe`):
one of the player's own bases (ECGAME loads only his chain) whose position
is the world's sector, whose depth is at least 0.5, whose target is the
world's sector, and whose target layer is at least 0.5. The moving flag is
not checked, so a base descending onto the world already counts, unlike in
ECMAINT's growth test.

The screen then says either "There is a starbase orbiting planet X. Standard
building restrictions do not apply." or "There are no starbases orbiting
planet X. Standard building restrictions apply." and computes the budget
(file `0x126EB..0x12843`):

```text
budget = Trunc(min(G, 5 * P))     with a starbase
budget = Trunc(min(G, 1 * P))     without
```

"You have spent S out of B points. You have B - S left to spend." where
`S = sum over kinds of queue[k] * cost[k]` (file `0x128AA..0x128E7`, cost
table at file `0x47DA6 + 2k`, the same costs as ECMAINT's, 100 for kind 8).

`S` (Specify builds, file `0x112F5`) refuses to start when `B - S < 2`
("cannot afford to build anything else"). Per kind it offers at most:

- ships (kinds 1..7): `Trunc((B - S) / cost)` (file `0x11643..0x116B1`);
- armies: `min(Trunc((B - S) / 2), Trunc(10 * P) - armies - queued armies)`
  (file `0x114C4..0x11562`);
- batteries: `min(Trunc((B - S) / 20), Trunc(P) - batteries - queued
  batteries)` (file `0x11599..0x11635`).

So a world can hold at most 10 P armies and P batteries by ECGAME's rules.
ECMAINT does not enforce those caps. The quantity prompt (file `0x11D37`)
accepts 0..max and adds to the queue slot (file `0x11D5F..0x11D76`). Kind 8
is refused with "There is no unit #8 in Esterian Conquest" (file `0x11CB5`).
`A` clears the queue.

The review screen for a world first scales P by status (file `0x2864A`): 0
when just captured, P/2 when recently captured, P otherwise. It prints
"Efficiency" as `100 * scaled P / potential` to one decimal (file
`0x29230`) and "Expected revenue (tax% tax rate)" as `Trunc(tax * scaled P /
100)` points (file `0x293BB`). ECMAINT's pass applies the same scale to the
real income, without truncation.

### Commissioning

`[C]` Ships and starbases stay in the stardock until Planet Command `C`
(commission), `M` (mass commission), or the auto-commission makes a fleet or a
base of them (file `0x132AB`, `0x1339D`, scouts `0x143A4`, starbases
`0x13948`). A new fleet takes the next fleet number and ROE 6 if it has
warships, else 0 (t042). The dock contents can be destroyed by bombardment
before they are commissioned. Each commission checks `fleets + bases < 500`
(file `0x13C87`, `0x13EB5`, `0x13F29`, auto-commission `0x1428B`): "You
already have 500 fleets and starbases in play and cannot commission any
more." The profile prints that limit as "Maximum # of Fleets & Bases".

### Profile and status displays

`[C]` The profile (General Command `P`, file `0xAD79`) prints, per empire:

- Number of Planets: PLAYER `0x50`, and its rank.
- Present Production: PLAYER `0x52`, and its rank.
- Potential Production: the sum of potential over the empire's worlds (file
  `0xB20A..0xB22B`).
- Tax Rate of Empire: PLAYER `0x51`.
- Total Available Points: the sum of `Trunc(G)` over the empire's worlds
  (file `0xB2FB..0xB325`).
- Efficiency of Empire: `100 * production / potential`, to one decimal (file
  `0xB3D1..0xB453`).
- Current and Maximum # of Fleets & Bases: fleets plus bases, and 500.

## Setup, colonies, captures

`[C]` ECUTIL F1 seeds (ec-file-formats.md, PLANETS.DAT and PLAYER.DAT):

- Each homeworld: potential 100, P 100.0, G 35.0, status 2, 10 armies, 4
  batteries, name "Not Named Yet". `[P]` Every ECUTIL fixture reads P 100 and
  G 35.
- Other worlds by generator class: potential `Random(41) + 50`,
  `Random(51) + 30`, or `Random(141) + 10` (10..150 overall), P 0, G 0,
  status 0, owner 0, name "Unowned".
- Each empire: tax 0, production 100, planet count 1, no bases. CONQUEST.DAT's
  previous-year arrays read 100 and 1.

`[C]` Colonizing (ECMAINT dump `0x781B..0x7873`) sets owner, P 1.0, status 2,
1 army, and the name "Not Named Yet". G stays 0. The colony's first pass is
the next year, at full scale, from P 1.0.

`[C]` Capture (dump `0xFA41..0xFE38`) sets status 0 and clears the queue. If
the world was under Scorch-Earth it halves P and G, empties the dock, and
clears the flag. A halved P can fall below 1.0 until the next pass floors
it.

## Probe log

All runs: `original/v1.5/ECMAINT.EXE` under DOSBox-X through
`tools/ncmaint_oracle.py`, on copies of `fixtures/ecmaint-econ-pre/v1.5`
(world 15 at (16,13), potential 100, owner empire 1) or
`fixtures/ecmaint-starbase-pre/v1.5` (same, plus empire 1's base at rest in
orbit). Patched bytes: PLANETS `0x03` (P), `0x09` (G), `0x5C` (status),
`0x60`, `0x24..` (queue), `0x5D` (owner); PLAYER `0x51` (tax), `0x00`
(mode), `0x44` (base count); BASES `0x19` (Z), `0x22` (target layer).
Runs started in parallel share the clock seed, so repeated outcomes within a
batch are expected; distinct outcomes are what count.

| Probe | Runs | Result |
|---|---|---|
| Tax 0, 35, 50, 65, 70 at P 50 | 25 | Every P matched one of the 11 rolls; income exact. |
| Tax 80, 100 at P 50 | 10 | P inside the penalty band; income exact on the shrunk P. |
| Status 0 and 1 | 6 | Scale 0 and 0.5 on both growth and income; status stepped. |
| Owner mode 1 (joined human) | 3 | Same pass as mode 0. |
| G 65500, tax 100, P 100 | 2 | G exactly 65535. |
| 3 battleships queued, G 35 | 2 | Dock +3, G -73.003. |
| Mixed queue, P 100, G 200 | 2 | Every kind completed; charge 100; income 50. |
| P 1.0 at tax 50 and 0 | 6 | 5.672 and 14.969; G 2.836 and 0. |
| P 10, potential 30 | 3 | 11.207 (roll 8). |
| P 100 | 3 | Unchanged; income 50. |
| Base at rest in orbit, tax 50 | 6 | Outcomes need `cut + roll` = 9, 10. |
| First owned world with the owner's only base in deep space | 18 | Outcomes fit a cut of 0. |
| `/R` run | 1 | `RANKINGS.TXT` written; empty tables with no joined empire. |

The t041 batches (October 2026) fixed the base-in-orbit rule and showed the
leak at tax 70; the file-formats scorch probe fixed the Scorch-Earth
numbers.

## Classic bugs

1. **Starbase cut leak.** `[P]` The cut lives in a stack slot that is set
   when a base is found and reset to 0 only when the owner has no bases at
   all. A world of a base-owning empire with no base in orbit keeps the cut
   rolled for the last world that had one, even another empire's. The first
   such world in record order reads whatever the stack held; in 18 runs it
   behaved as 0, which depends on the calls before the pass and is not
   guaranteed.
2. **Any empire's base grants the cut.** `[P]` The orbit scan checks the
   base's position, depth, and moving flag, not its owner. The gate is only
   that the world's owner has at least one base somewhere.
3. **Goods go negative.** `[P]` ECMAINT charges the whole queue. ECGAME keeps
   orders within the budget, but goods can fall after the order (bombardment
   destroys stored goods), and the AI writes the queue directly. The world
   then owes points and pays them back from income.
4. **Kind 8.** `[C]` ECMAINT prices slot 8 at 100 and moves it into dock slot
   8, which the bombard report calls IPBMs. ECGAME refuses the slot.
5. **Surface and dock caps.** `[C]` ECMAINT saturates armies, batteries, and
   each dock slot at 65535 while ECGAME caps armies at 10 P and batteries at
   P. The two programs disagree on the limit.
6. **Orbit test mismatch.** `[C]` ECGAME's 5x test ignores the moving flag;
   ECMAINT's growth test requires it. A base ordered down onto the world
   gives 5x build capacity before it gives any growth help.
7. **Stale previous-year values within a run.** `[C]` The arrays are filled
   from PLAYER `0x52`/`0x50`, which are recomputed only after the last year
   of a run.
8. **Dead multiply.** `[C]` The high-tax penalty multiplies by a literal 1.0.
   Harmless.

## How NC differs

NC's economy is its own design, in [economics.md](economics.md) and the
player manual's Economy chapter and Appendix A. This section compares; it is
not a list of changes to make.

| Topic | Classic | NC |
|---|---|---|
| Growth | Angle on a quarter circle, 7.5 degrees a year times `1 - (tax +- 5)/70`; big gains early, almost none near potential; no minimum; can be negative from 66% | `ceil(gap * (100 - tax) / 400)`, at least 1, never negative below the penalty |
| 65% threshold | 65% is the highest rate that never shrinks a world; the random roll of +-5 makes 66..75% a gamble; the direct penalty starts above 70% at `(tax - 70) * 0.5..1.5` percent of P | A deterministic penalty `ceil(P * (tax - 65) / 500)` above 65% |
| Starbase growth help | A random cut of 0..9 tax points on the growth term, with the leak bug; any base at rest counts if the owner has a base | +50% of base growth at tax <= 50, tapering to 0 at 65; the owner's base at rest, by the same orbit test |
| Starbase build help | 5x spend limit in ECGAME; base in orbit with the world as target, moving flag ignored | 5x build capacity; same ECGAME test |
| Revenue | `P * tax / 100 * status scale`, a real; fractions accumulate | `floor(P * tax / 100)`, an integer |
| Status after capture | Scale 0, 0.5, 1.0 on both growth and income for three passes | "Approximately two turns"; same stepping idea |
| New colony | P 1.0, status 2, full growth and income from the next pass | P 0, no treasury; growth from the next turn |
| Treasury | Per-world goods, capped at 65535, can go negative | Per-world treasury; budget `min(treasury, capacity)`; never negative |
| Builds | Everything queued completes; no partial progress; dock slots saturate at 65535; ECGAME limits the order, not ECMAINT | Spending limited to the budget; partial progress carries over; blocked builds stay queued; armies and batteries cap at 255 |
| Unit caps | ECGAME: armies <= 10 P, batteries <= P per world | 255 each |
| Scorch-Earth | Yearly: G to `min(G/10, sqrt(G))`, P to 30..49%; no growth or income | Not covered in economics.md |
| Year order | Pass before the 52 ticks; builds complete before movement | After the tick loop (t048 covers the mismatch) |
| Disorder empires | Economy runs; they keep their last tax | Frozen |
| AI tax | `Round(65 * sum P / sum potential)` | Not stated |
| Rankings | By production or planet count, previous year in parentheses; no score | Not covered in economics.md |
| Homeworld seed | ECUTIL writes P 100.0, G 35.0 | Starts at production 100; economics.md notes older seeds read 50 |

Rust reads the classic fields it needs for import and export; see
ec-file-formats.md for the byte layout. Nothing here changes those layouts.
