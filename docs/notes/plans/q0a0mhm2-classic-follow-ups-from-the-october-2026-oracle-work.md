---
id: q0a0mhm2
date: 2026-10-10
kind: plan
tags: [plan, oracle]
---
# Classic follow-ups from the October 2026 oracle work

The t031-t038 agents found classic behaviours and Rust gaps outside their
own tasks. Each section below is one task in [todo.md](../../../todo.md).
Policy: Rust matches the classic binaries exactly, except clear classic bugs
the user approves fixing.

## t040

Decide whether three classic starbase-guard quirks are bugs. t037b reproduces
all three today.

1. ECGAME writes `0x22 = 1` for every Guard Starbase order. A guard whose
   target depth differs from its base's layer re-arrives every tick in
   classic and sends an arrival report each time. Rust arrives once a year;
   the year-end state is the same.
2. A guard already in orbit in the sector of a deep-space base, with
   `0x22 = 1`, never arrives in classic.
3. A base that lifts on tick 52 gets depth 0, but its escort keeps its own
   depth.

Source: the t037b report; rules in [ec-starbase-spec.md](../../dev/ec-starbase-spec.md).

## t041

Rust's economy counts a planet as having a starbase when a base is anywhere
in its sector, even in deep space. Probe classic ECMAINT to see whether its
starbase production bonus requires the base to be in orbit, and match it.

## t042

Small order fixes found by t038:

- ECGAME's composition recompute (`0x18333`) also sets ROE to 0 for a fleet
  with no warships. Rust's recompute does not.
- Rust's detach does not update the PLAYER.DAT fleet chain tail.

Result (ECGAME 1.51 disassembly and runs; addresses are image offsets in
`ecgame_640k.bin`):

- `0x18333` runs only on the donor, from `0x18049`, after a detach
  (`0xf497`), a detach-and-join (`0xda27`), or a transfer (`0xdbb2`). A
  donor left without a BB, CA, or DD gets ROE 0; any other ROE stays, even 0.
  The transfer host, load, unload, and ECMAINT's merge (`2895:4a6b`) never
  write `0x25`; ECMAINT's only ROE write is the retreat at `1b05:2b72`. A new
  detached or commissioned fleet takes ROE 6 if it has warships, else 0, then
  the ROE prompt (0-10, default 6).
- Detach and commission share `0xeac1`. The new fleet's id is the
  FLEETS.DAT record count plus one; its previous link names the last fleet in
  the chain, whose next link names it. `0x15d89` then sorts the chain by
  fleet number, moving record contents and leaving the next, id, and previous
  words in place, so a fleet number filled into a gap takes an earlier
  record. Nothing writes the PLAYER.DAT tail (`0x42`) except fleet deletion
  (`0x194be`), so after a detach it still names the old last fleet, and
  ECMAINT leaves it there. Rust matches this; whether the stale tail is a
  classic bug is open.

Rust now matches the ROE rules, the chain links, the sort, and the tail. It
still gives a new fleet the lowest unused id, which is the record count plus
one while ids are dense. Tests: `nc-data/tests/fleet_composition.rs`.

## t043

Decide how nc-game and nc-helm handle ECGAME's "seek the next closest
system?" prompt. When a fleet is already in the nearest owned system, ECGAME
asks, defaulting to yes. Rust's clients take the yes without asking; a turn
file can still name either target.

## t044

Report gaps between classic and Rust:

- Classic reports a starbase sighting and an "unable to intercept" outcome;
  Rust has no report types for them (t035).
- In classic a Hold fleet's dispatch probably zeroes its speed; Rust leaves
  it, which changes the speed term in its visibility (t035).
- Classic keeps old RESULTS.DAT reports until the player reads them; Rust
  clears them each year (t031b). Decide which the runtime store should do.
