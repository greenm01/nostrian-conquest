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

## Connections

- [Bug-sweep follow-ups, t012](../plans/c39y8ydp-follow-ups-from-the-october-2026-bug-sweep.md#t012):
  the ETA change this reverses.
- [Hostile arrival audit](../../dev/hostile-arrival-oracle-audit.md): the
  strike-next-year rule, unaffected.
