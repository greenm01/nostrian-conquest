---
id: jladc1qu
date: 2026-10-10
kind: investigation
status: investigating
tags: [investigation, economy, storage, oracle]
---
# Build queue translation between Rust slots and classic counts

## Question

How should the PLANETS.DAT codec carry pending builds between Rust's slots
and classic's counts by kind (t056) without charging twice or handing out
free units? Rust keeps its own build economy (decision of 2026-10-10).

## What each side stores

**Classic** ([ec-file-formats.md](../../dev/ec-file-formats.md) PLANETS
0x24; [ec-economy-reference.md](../../dev/ec-economy-reference.md) "Build
completion", "Build screen"). 0x24 is `u16[10]`, units ordered by kind:
1 DD, 2 CA, 3 BB, 4 scout, 5 TT, 6 ETAC, 7 starbase, 8 reserved, 9 army,
10 battery. Orders are not paid when made: ECGAME adds to the slot
(0x11D5F) and never writes G, capping the order at `Trunc(min(G, P or 5P))`
less `sum(queue * cost)`. ECMAINT's pass, before income, docks slots 1..8,
lands 9 and 10, charges `G -= queue[k] * cost[k]` unchecked, and zeroes the
queue (`1852:0206..0445`). All of it completes in one pass; G can go
negative ([P] build fixture: 3 DD + 1 ETAC, G 35 to 0). The brief's premise
that classic counts are prepaid is wrong.

**Rust** (`BuildSlot` in `nc-data/src/model/planet.rs`;
`process_build_completion` in `nc-engine/src/maint/economics/build.rs`;
`append_planet_build_order` in `nc-data/src/directory/mutators_planets.rs`).
Ten slots of `points_owed: u8` and a kind whose `to_raw` codes differ from
classic's (7 battery, 8 army, 9 starbase). Orders are unpaid too: `qty *
cost` points fill slots of the kind, then empty ones, up to 255 rounded to
the cost for ships, one starbase, or 255 points for armies and batteries.
New orders are capped at `min(round G, capacity)` less points queued, as in
ECGAME. The pass, before income, pays slots in order up to that budget; a
slot delivers `ceil(old/c) - ceil(new/c)` units, so a slot owing `p` with
`p mod c != 0` holds one part-paid unit, `a = ceil(p/c) * c - p` paid. A
slot that would pass the 255 army or battery cap is skipped unpaid.

So both hold unpaid orders sized to the same budget and pay them at the next
pass. Rust leaves a remainder only when that budget shrinks below the queue
(the 5x base left, bombardment cut G or P, the cap blocks, an import exceeds
Rust's budget). Neither side has a paid-but-unbuilt unit.

## Cases

1. Whole units owed: `p/c` units out, `n * c` points in. Exact.
2. A part-paid unit: no classic form. This is the real choice.
3. Armies and batteries (classic 9, 10; Rust 8, 7): same arithmetic; an
   imported order past Rust's 255 cap waits unpaid (ECMAINT caps at 65535).
4. Starbases (classic 7, Rust 9): one per Rust slot, but ECGAME allows up to
   15 a year (P 150 with a base), more than ten slots hold.
5. Slot 8: only a file edit fills it; ECMAINT charges 100 and docks an IPBM.
   Rust has no kind for it. Rust `Unknown` kinds have no classic form.
6. Several slots of one kind: export sums units per slot, `sum ceil(p_i/c)`,
   which is what Rust would deliver. Import fills slots in kind order, so
   Rust's slot order (payment priority) does not survive a round trip.
   Rust bug: army and battery slots hold 255 points, so 128 armies split
   255 + 1 and deliver 129 (capacities 254 and 240 fix it).
7. Rust's `round(P) * 5` can sit 2 points under ECGAME's `Trunc(5P)`, so an
   edge order from classic may part-pay in Rust.
8. Negative classic G imports as is; `stored_production_points` clamps it to
   0 at Rust's first pass, forgiving the debt.

## Candidate rules

Export, per slot: `u = ceil(p/c)` units, `a = u * c - p` already paid.

- **E1, count and credit.** Write `sum u` per kind; add `sum a` to G. ECMAINT
  charges `sum p + sum a`, so G and the dock end where Rust's pass would put
  them whenever Rust's budget covers the queue. The player sees G up by `a`,
  and ECGAME offers up to `a` less this turn when P limits the budget. A
  round trip with no pass keeps value but needs `a` more capacity in Rust.
- **E2, count, no credit.** The player pays the paid part twice (at most 44
  for a battleship, 49 for a starbase); the oracle sees G short by `a`.
- **E3, drop and refund.** Write `floor(p/c)`, add `a` to G. Value balances,
  but the unit is lost and reordered at full price a year late.
- **E4, refuse.** Exact, but a sysop cannot clear the slot: Rust's abort
  discards the paid part, and waiting can take years. Refusing on any
  pending build blocks every live game.

Import:

- **I1, owed points.** Count `n` becomes `n * c` owed; G unchanged. Exact,
  since neither side has charged, and Rust's next pass completes it except
  in cases 3, 7, or a changed budget. Refuse, naming the planet, for slot 8
  or a queue needing more than ten slots.
- **I2, prepaid slots.** A new `BuildSlot` field (`prepaid_points`) the pass
  completes regardless of budget, with G debited at import. Copies classic
  for imported orders, but buys nothing I1 lacks: counts are unpaid.
- **I3, widen the model.** `Vec<BuildSlot>` or per-kind `u32` points, so any
  queue fits. Ten `u8` slots only existed to fit the 20 classic bytes. It
  changes a player rule ("10-slot build queue", "Build queue is full").

Rejected: keeping `a` in spare bytes 0x5E/0x5F, which the 2026-10-10
decision zeroes and a classic pass would leave stale.

## Recommendation

E1 with I1. E1 alone keeps value and units equal to Rust's next pass, so
`db-export`, ECMAINT, `db-import` matches Rust whenever Rust's budget covers
the queue, and a sysop can explain the credit as the paid part of an
unfinished unit. I1 needs no model change. Refuse import rather than drop.

- The codec (`nc-data/src/classic/planet.rs`) maps kinds by name, credits
  `stored_goods` on encode, and decodes fallibly.
- `planet-build 15 0x03 0x01` (3 points of destroyer) encodes as classic
  3 DD + 1 ETAC by coincidence; under E1 it is one destroyer and a 2-point
  credit. It, `planet-build-init`/`-batch-init`, the `planet-build` scenario,
  `GameStateBuilder::with_planet_build`, and the slot-byte tests in
  `classic_codec.rs` and `storage_core.rs` move to orders by kind.
- `ecmaint-build-pre` is the only fixture with a queue; I1 round-trips it,
  and Rust's pass should then match `ecmaint-build-post`. No `ncmaint_*`
  audit queues builds.
- Rust queues AI builds at the end of a turn and ECMAINT's AI queues again
  before its pass (`0x1DB6E..0x1DDAB`); check before trusting AI runs.

## Questions for the owner

1. E1 raises G in the export. Accept, or prefer E2's small loss?
2. A classic queue needing more than ten slots: refuse, or widen (I3)?
3. Slot 8 counts from an edited file: refuse, or drop with a warning?
4. Negative classic G: keep Rust's clamp, or carry the debt?
5. File the army and battery slot-capacity bug as its own task?

## Connections

t056 in [todo.md](../../../todo.md);
[plan section](../plans/7h5r6ne8-typed-game-state-in-rust-and-sqlite.md#build-queue-at-the-classic-edge-t056);
Rust's rules in [economics.md](../../dev/economics.md).
