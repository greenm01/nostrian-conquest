# ECMAINT Movement Audit

Compact movement-fidelity summary for the current controlled oracle work.

Status: solved enough for the current Rust engine. This doc records the
player-facing conclusions and the accepted compatibility seam; it is not a
standing requirement to keep chasing every hidden classic movement byte.

## What Is Settled

- movement is annual and happens before the weekly `1..52` scheduling loop
- the annual distance budget still follows the recovered `speed * 8 / 9`
  pattern
- classic does not simply resample from the rounded visible sector each year
- the controlled horizontal / diagonal / shallow / steep coordinate traces are
  now good enough to support the current Rust direct-stepper geometry
- persistent standing missions now have their own settled follow-up in
  [persistent-mission-oracle-audit.md](persistent-mission-oracle-audit.md)
- delayed hostile arrival behavior now has its own settled follow-up in
  [hostile-arrival-oracle-audit.md](hostile-arrival-oracle-audit.md)
- for Rust policy, unresolved hidden movement bytes are now treated as
  implementation detail unless they prove to change player-facing behavior or
  classic file safety

## Completion Timing

Classic completes a one-shot move in the year the fleet's rounded position
first enters the target sector, and ECGAME's fleet ETA shows that year. For
`MoveOnly`, `speed=3`, `10,10 -> 16,16`, the visible trace is
`10,10 -> 11,11 -> 14,14 -> 16,16` and the order clears to `hold`, `speed=0`
in year 3, with the exact path still short of the target point.

ECGAME writes a move order with mission aux bytes `0x22..0x23 = 00 00`. A
nonzero `0x22` (a join host fleet id, set on every fleet in the ECUTIL
starting game) makes classic finish a move a year later, so probe orders
clear both bytes, as `tools/ncmaint_movement_audit.py` does.

Rust completes orders when the rounded position enters the target sector,
and player orders clear the aux bytes that the order does not use. The full
case table is in the
[investigation](../notes/investigations/9msqq5e0-classic-move-orders-complete-on-entering-the-target-sector.md).

## Accepted Boundary

- preserve the recovered annual movement geometry and completion in the year
  the fleet enters the target sector
- keep hazard-detour policy clearly separated from the classic direct-stepper
  geometry
- do not treat hidden movement scratch bytes as a fidelity target unless they
  are later shown to matter for gameplay or compatibility
