# ECMAINT Transit Scratch Audit

Focused follow-up on the remaining in-transit movement-state mismatch between
Rust and classic `ECMAINT`.

Status: archived implementation note. This material explains why Rust keeps an
internal exact-position seam, but it is not an active requirement to reproduce
every hidden classic movement byte.

Primary source for these controlled probes:

- `tools/ncmaint_transit_scratch_audit.py`

## What This Audit Establishes

- In the controlled transit turns checked so far, classic leaves
  `0x19..0x1e = 00/00/00/00/00/00`.
- That zeroed transit window is not limited to one mission family. It appears
  in the current controlled probes for:
  - `MoveOnly`
  - `PatrolSector`
  - `GuardBlockadeWorld`
  - `GuardStarbase`
- Rust's current `0x1a..0x1e` exact-position encoding is therefore a pragmatic
  internal movement/ETA seam, not a recovered classic byte model.
- Classic completes a one-shot move in the year the fleet's rounded position
  first enters the target sector, with the order written as ECGAME writes it
  (mission aux bytes `0x22..0x23 = 00 00`). A nonzero `0x22` delays completion
  by a year. See [movement-oracle-audit.md](movement-oracle-audit.md).

## Standing-Mission Transit Byte Result

The controlled standing-mission transit probes support the same byte-level
conclusion:

- `PatrolSector`, `GuardBlockadeWorld`, and `GuardStarbase` use the expected
  in-transit `0x0d..0x12` motion window
- classic leaves `0x19..0x1e` zero during those transit turns
- Rust stores its internal exact position there instead

That mismatch is acceptable as an internal seam, documented as Rust-owned
state rather than recovered classic semantics.

## Current Rust Consequence

- keep the Rust exact-position seam because it gives correct geometry and ETA
  in the controlled cases
- do not describe `0x1a..0x1e` as classic exact-position bytes
- complete one-shot movement on the first rounded target-sector hit

This explains why Rust carries an internal movement seam; it is not a mandate
to chase every hidden classic transit byte.
