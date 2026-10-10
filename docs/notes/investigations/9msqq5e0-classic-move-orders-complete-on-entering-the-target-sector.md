---
id: 9msqq5e0
date: 2026-10-10
kind: investigation
status: investigating
tags: [investigation, movement, oracle]
---
# Classic move orders complete on entering the target sector

## Question

When does classic ECMAINT complete a move order, and what ETA does classic
ECGAME show? The March 2026
[movement audit](../../dev/movement-oracle-audit.md) and
[transit audit](../../dev/transit-scratch-oracle-audit.md) reported classic
keeping a fleet on its move order for a year after it first showed in the
target sector, so Rust waits for the exact path endpoint and t012 made the
ETA count that extra year on diagonals.

## Evidence

Probes on 2026-10-10 ran `MoveOnly` fleets from (10,10) through
`tools/ncmaint_movement_audit.py` (`run_probe`) and classic ECMAINT under
`nix shell nixpkgs#dosbox-x`, and drove classic ECGAME headless under Xvfb
with xdotool and scrot.

1. With the audit's setup, classic finished 10 of 12 moves a year after the
   fleet entered the target sector, at speeds 1 to 5 and on a horizontal move.
2. ECGAME's ETA calculator for the same fleet reported the year the fleet
   first enters the target sector: speed 3 to (13,13) 2 years, to (16,16) 3;
   speed 5 to (15,10) 2, (12,10) 1, (13,13) 1, (16,16) 2.
3. A move order entered in ECGAME (speed 3 to (15,10)) completed in classic
   ECMAINT in year 2, while the same order written by `nc-cli fleet-order`
   completed in year 3. The two FLEETS.DAT records differed only at `0x22`,
   the first mission aux byte (join host fleet id): ECGAME wrote `00`, the
   audit's setup kept the fixture's `01`.
4. With `0x22..0x23` cleared, classic completed all 15 probed moves in the
   year the fleet first shows in the target sector:

| speed | offset | classic | Rust now |
| ---: | --- | ---: | ---: |
| 1 | (3,3) | 5 | 6 |
| 1 | (2,2) | 4 | 4 |
| 1 | (4,4) | 6 | 7 |
| 1 | (3,1) | 4 | 5 |
| 1 | (3,0) | 4 | 4 |
| 2 | (3,3) | 3 | 3 |
| 2 | (6,6) | 5 | 5 |
| 2 | (4,2) | 3 | 3 |
| 2 | (4,0) | 3 | 3 |
| 3 | (3,3) | 2 | 2 |
| 3 | (6,6) | 3 | 4 |
| 3 | (6,0) | 3 | 3 |
| 4 | (6,6) | 3 | 3 |
| 5 | (6,6) | 2 | 2 |
| 6 | (6,6) | 2 | 2 |

Visible positions matched Rust in every year of every case.

## Finding and resolution

Classic completes a move order in the year the fleet's rounded position
first equals the target sector, and ECGAME's ETA reports that year. The
one-year delay in the March audits was a probe artifact: a nonzero join-host
byte on a move order delays classic completion by a year. Rust's
exact-path-endpoint rule, and t012's ETA built on it, follow that artifact
and finish slow or long diagonal moves a year late.

The same stale byte can reach real exports: `set_fleet_order` in
`rust/nc-data/src/directory/mutators_fleet.rs`, used by turn submissions and
nc-game, keeps a fleet's previous mission aux bytes unless new ones are
given, while ECGAME clears them for a move.

Rust's movement carry also round-trips through a lossy encoding that rounds
up (`encode_movement_fraction` / `decode_movement_fraction` in
`rust/nc-engine/src/navigation/eta.rs`); positions still matched in these
probes, so whether it matters on long trips is untested.

## Validation and remaining work

t031 in [todo.md](../../../todo.md): complete orders and compute ETA on
entering the target sector, clear mission aux bytes for orders that do not use
them, rerun the movement, transit, and hostile-arrival audits with
ECGAME-shaped orders, probe a long trip for carry drift, and correct the
audit docs, the movement spec, and the player manual. The hostile
arrive-then-strike delay is independent of this and stays.

## Every-mission sweep and decompiled movement (2026-10-10)

The finding above holds for orders whose target is a sector and is wrong for
the other twelve. An ECGAME capture of all 16 missions showed byte `0x22` is
not a join host: ECGAME writes `00` for Move, Patrol, Scout Sector, and
Rendezvous and `01` for every other order, and `0x23` holds the guarded
starbase id or the join host fleet id. The "stale join-host byte" that delayed
the March moves was this flag, turning the move into a world-target trip.

A classic ECMAINT matrix (`tools/ncmaint_mission_sweep.py`, ~120 cases over
missions 2, 4-9, 11-13, 15, speeds 1-5, diagonal and straight offsets) and a
decompile of the unpacked ECMAINT (`tools/unlzexe/ecmaint_640k.bin`, Ghidra
12.1.2 plus a capstone pass) give this model, which reproduces every traced
position, tuple c value, and resolution year:

- **Continuous 3-D movement.** A fleet has a position `X = x + fx`,
  `Y = y + fy` (`0x0b/0x0d..0x12`, `0x0c/0x13..0x18`, fractions stored as
  Turbo Pascal 6-byte reals, 0.5 = cell center) and a depth
  `Z` = tuple c (`0x19..0x1e`, a 6-byte real). The target is
  `(tx + 0.5, ty + 0.5, TZ)` with `TZ = min(0x22, 1)`.
- **Ticks.** The year driver (`0824:1004`) sorts fleets by `Random(0xffdc)`
  and runs 52 ticks. Each tick each active fleet gets a budget of
  `speed / 52` (step `0824:01c0`), then the encounter handler (`18ea:13cf`)
  and order dispatcher (`0b5d:5581`) run, then the record is saved.
- **Step.** If the fleet is outside the target cell or deeper than `TZ`, it
  first climbs: `Z` drops by the budget, floored at 0. Remaining budget moves
  it straight toward the cell center; once at the center, leftover budget
  dives `Z` toward `TZ`. A fleet at rest sits at `Z = 1`, so speed `v` covers
  `v - 1` sectors in its first year and `v` per year after. The speed x 8/9
  carry model was wrong; it agrees on short trips and drifts on long ones.
- **Arrival tests** (per order, in the dispatcher):
  - Base test, used by Move, Patrol, Guard/Blockade, View, Scout, Join,
    Rendezvous, Salvage: in the target sector and `Z == float(TZ)`. With
    `TZ = 0` that fires on sector entry; with `TZ = 1` it needs the center
    and then a full dive.
  - Seek Home (2) and Colonize (12): the base test, or `Round(Z) == 1` at the
    end of tick 52, so they resolve at year end once half dived.
  - Guard Starbase (4): the starbase is in the same sector and depth layer
    (`Z >= 0.5`) and speed > 0.
  - Bombard, Invade, Blitz (6-8): ownership checks, then in the target sector
    with `Round(Z) >= 1` and `Z >= 1.0` (or `Z == TZ`). The action runs a
    random 10-round planetary battle (`110a:4ba4`). If the attack fails, that
    tick's record is not saved, so the file keeps `Z` one tick short of 1.0
    (0.988129, 0.957026, 25/26, ...) and the fleet retries next year. The
    "speed-1 blitz never lands" result was a run of failed battles, not a
    speed rule.
- The dispatcher refills `speed` from `0x09` when a fleet that has not
  arrived has speed 0.
- `1 - epsilon` depths (`80 b9 ff ff ff 7f`, `80 ff ..`) are 6-byte-real
  rounding during the climb; the arrival test is an exact `Z == 1.0`, so such
  a fleet settles a year later.

Scratch artifacts (disassembly, trace dumps, patch experiments) were kept
in the session scratchpad only. Decisions taken with the user: Rust steps
the same model a year at a time (no tick loop), stores `fx`, `fy`, and `Z`
in classic's encoding, and hostile missions strike in the year `Z` reaches
1.0 and retry each year while their battle fails. Open: the battle's win
probability, which line skips the save, and the View World resolve action
(it sets `Z = 0`).

## Connections

- [Bug-sweep follow-ups, t012](../plans/c39y8ydp-follow-ups-from-the-october-2026-bug-sweep.md#t012):
  the ETA change this reverses.
- [Hostile arrival audit](../../dev/hostile-arrival-oracle-audit.md): the
  strike-next-year rule, unaffected.
