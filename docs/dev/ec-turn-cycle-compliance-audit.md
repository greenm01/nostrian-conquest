# EC Turn-Cycle Compliance Audit

This document tracks current Rust conformance to
[ECMAINT Canonical Turn Cycle](ec-turn-cycle-spec.md).

Use it as the claim-by-claim Rust maintenance status snapshot. Keep
[ec-turn-cycle-spec.md](ec-turn-cycle-spec.md) normative and historical, and
keep [rust-turn-cycle-implementation.md](rust-turn-cycle-implementation.md)
Rust-native and architecture-facing. Do not duplicate the full matrix in those
docs.

## Status Vocabulary

- `Compliant`: implemented closely enough to satisfy the current spec claim
- `Compliant (by design)`: intentionally simplified in Rust and already
  acknowledged by the companion implementation docs
- `Partial`: some supporting behavior exists, but the full spec-shaped phase
  or guarantee is not yet present
- `Not implemented`: explicit gap
- `Not applicable`: spec claim is about classic file/CLI behavior outside the
  Rust engine core

## Summary

Headline status:

- `15` fully compliant
- `3` compliant by explicit design simplification
- `3` partial/gap items
- `2` not-implemented items
- `2` not-applicable CLI concerns

| Spec claim | Status | Notes |
| --- | --- | --- |
| 1. Schedule / token gate | Compliant | `gate.rs` complete |
| 2. Crash recovery (`Move.Tok`) | Compliant | `recovery.rs` complete |
| 3. Cross-file integrity validation | Partial | Structural checks now run at the engine entrypoint through `validate_maintenance_state()`, but the current rule set is still narrower than the full recovered classic validation surface |
| 4a. Annual movement (pre-loop) | Compliant | Single pass per turn |
| 4b. 52-week loop = event scheduling | Compliant (by design) | Spec-acknowledged simplified model |
| 4c. Intra-year weekly scheduler | Compliant | Timing codes + canonicalization |
| 4d. Single timing stream | Compliant | All events through `canonicalize_events` |
| 4e/f/g. `024d` planet mutation pass | Partial | Outcome-equivalent; binary internal staging not replicated |
| 4h. Ready hostile world-resolution | Compliant | Pre-snapshotted `bombard_ready` etc. |
| 4i. 52-pass fleet loop | Compliant (by design) | Single-pass Rust structure; spec-acknowledged |
| 4j. Inline combat report emission | Partial | Battle events are produced during simulation and ordered correctly later, but the current Rust engine does not perform inline combat-file emission |
| 4k. Pre-loop fleet setup | Compliant | `process_fleet_merging` before movement |
| 4l. PRNG visit order | Compliant (by design) | Deterministic slot order; spec-acknowledged |
| 4m. Combat triggered by first co-located hostile | Compliant | `process_fleet_battles` |
| 4n. Fleet reassignment between empires | Not implemented | A preserved `ecmaint-fleet-battle` pre/post probe confirms the current Rust engine does not reproduce the documented cross-empire fleet-owner reassignment |
| 4o. File write ordering | Not applicable | CLI / projection concern |
| 4p. Position-first, mission-next-year | Compliant | Pre-snapshotted `bombard_ready` etc. |
| 4q. Colonization atomic on arrival | Compliant | All fields set in one pass |
| 4r. Planet pass gates on ownership, status and Scorch Earth | Divergent | `planets.rs` skips the economy of a civil-disorder empire's worlds; classic's `1852:0000` reads only PLAYER `+0x44` and `+0x51` |
| 4s. Planet pass before the ticks | Compliant | `mod.rs` completes builds and settles the economy before `encounter::run_year`; the autopilot/rogue orders pass runs at the end of the turn, where classic runs it at the start of the next |
| 4t. Builds first in the planet pass | Compliant | `process_build_completion` before `process_planet_economics`, both before the ticks |
| 5. Canonicalization and sort | Compliant | `canonicalize_events` |
| 6. Late `1..52` report emission loop | Not implemented | Weekly report-emission parity gap; report builders exist, but the recovered weekly emission loop is not yet modeled directly |
| 6-table. Timing constants | Compliant | `timing.rs` exact match |
| 7. Final flush / cleanup | Not applicable | CLI concern |

## Priority Follow-Up

1. Phase 6 reporting:
   implement a structured weekly report-emission pass for `RESULTS.DAT` and
   `MESSAGES.DAT` parity with the oracle.
2. Phase 3 validation shape:
   either promote current preflight structural checks into a more explicit
   maintenance validation phase, or tighten the docs to explain why the
   current split is sufficient.
3. Phase 4n implementation or explicit deferral:
   either implement the preserved fleet-owner reassignment seen in the
   `ecmaint-fleet-battle` fixture pair, or keep it documented as an unsupported
   oracle divergence at the compatibility boundary.
4. `024d` internal staging:
   keep this as a low-urgency refinement item only; current Rust behavior is
   already outcome-equivalent enough for the present milestone.

## Notes

- This audit is a status document, not the normative turn-cycle spec.
- The current gaps are not game-state correctness blockers for the current
  milestone.
- The clearest explicit implementation gaps are late report-emission parity and
  preserved cross-empire fleet-owner reassignment.
- Classic `.DAT` outputs are compatibility/oracle projections at the edge of
  the Rust engine, not the primary narrative for the maintenance model here.
