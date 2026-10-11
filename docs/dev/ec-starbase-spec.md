# EC Starbase Spec

The classic BASES.DAT record, the starbase chain, and how ECMAINT moves
starbases and the fleets that guard them. It was read from the unpacked
ECMAINT (seg:off maps to file offset `seg*16+off-0x8140` in
`tools/unlzexe/ecmaint_640k.bin`) and the unpacked ECGAME
(`tools/unlzexe/ECGAMEU.EXE`, offsets into the load image), and checked
with classic probes. "[P]" marks a rule confirmed by a probe, "[C]" one read
from code only. Fleet movement, which starbases share, is in
[ec-movement-spec.md](ec-movement-spec.md); sighting and combat are in
[ec-encounter-spec.md](ec-encounter-spec.md).

Rust reads and writes every field below
([starbase.rs](../../rust/nc-data/src/model/starbase.rs),
[base.rs](../../rust/nc-compat/src/classic/base.rs)) and moves starbases by
the rules below
([starbase.rs](../../rust/nc-engine/src/maint/movement/starbase.rs),
[guard.rs](../../rust/nc-engine/src/maint/movement/guard.rs)). Where
classic misbehaves, [Rust Differences](#rust-differences) says what Rust
does instead.

## Record Layout

A record is 35 bytes. It mirrors the front of a FLEETS.DAT record: chain
words, position, and target sit at the same offsets, with the fleet's speed
bytes replaced by a previous-base link.

| Bytes | Field | Encoding | Evidence |
| --- | --- | --- | --- |
| `0x00..0x01` | escort: local slot word (FLEETS `0x00`) of the Guard Starbase fleet that rides with the base; 0 for none | u16 | [C][P] |
| `0x02..0x03` | starbase id ("Starbase 1"), named by a guarding fleet's `0x23` | u16 | [C][P] |
| `0x04` | owner empire | u8 | [C][P] |
| `0x05..0x06` | next base in the owner's chain (record number; 0 ends it) | u16 | [C][P] |
| `0x07..0x08` | this record's own number (1-based) | u16 | [C][P] |
| `0x09..0x0a` | previous base in the owner's chain (0 at the head) | u16 | [C] |
| `0x0b`, `0x0c` | sector `x`, `y` | u8 | [C][P] |
| `0x0d..0x12` | in-sector `x` fraction `fx` | 6-byte real | [C][P] |
| `0x13..0x18` | in-sector `y` fraction `fy` | 6-byte real | [C][P] |
| `0x19..0x1e` | depth `Z`: 0 deep space, 1 orbit | 6-byte real | [C][P] |
| `0x1f` | moving flag: 1 while under way, 0 once arrived or halted | u8 | [C][P] |
| `0x20`, `0x21` | target sector `tx`, `ty` | u8 | [C][P] |
| `0x22` | target depth `TZ`: 1 orbit, 0 deep space | u8 | [C][P] |

The shipped game's one base reads
`01 00 01 00 01 00 00 01 00 00 00 10 0d` then `fx = fy = 0.5`, `Z = 1.0`,
`00 10 0d 01`: escort fleet 1, Starbase 1, owner 1, no neighbours, record 1,
at rest in orbit at (16,13).

Evidence by field:

- Escort. The guard arrival action (`0b5d:10a7`) writes the arriving fleet's
  `0x00` word here when no escort is present. The escort carry
  (`0824:04c8`) matches the word against each fleet unit's record word 0.
  ECMAINT's robot guard assignment (`2000:72d7`) picks a base only when the
  word is 0, and a new base starts at 0 (`2000:6674`). Nothing in ECMAINT
  clears it when the escort leaves. In the shipped game it is 1, and fleet 1
  of empire 1 guards Starbase 1. [C][P]
- Id. The guard lookup (`1b05:2133`) scans the owner's base units and
  matches this word against FLEETS `0x23`; the unit loader (`2895:5cdb`)
  copies it to the unit entry's `+10`. A Guard Starbase fleet with
  `0x23 = 1` resolves against the shipped base. [C][P]
- Owner. The unit loader (`2895:5c78`) fails the integrity check unless the
  byte equals the player whose chain reached the record. That is why the
  archived probe that set `0x04 = 2` on a second base in player 1's chain
  failed: `0x04` is the owner, not the id. The tick dispatcher compares it
  with the planet owner (`0b5d:58d0`) and sends its reports to that empire.
  [C][P]
- Links and record number. The loader starts at PLAYER `0x44`, reads record
  `link - 1`, follows `0x05`, and takes each later unit's record number from
  `0x07` (`2895:5d72..5ebc`). A one-base file with `0x05 = 0x0001` links the
  base to itself and ECMAINT aborts. [C][P]
- Position, depth, target. The starbase step loads these
  (`0824:0303..03c3`) and writes them back (`0824:0bde..0c42`). The
  lift probe below ends with `Z = 1/52`, `TZ = 0`. [C][P]
- Moving flag. The tick dispatcher sets and clears it (`0b5d:5806`,
  `0b5d:5853`); ECGAME sets it on Move and clears it on Halt. In the move
  probe it stays 1 for two years and drops to 0 on arrival. The sighting
  code reads it but multiplies by 1 either way (`18ea:0028`). [C][P]

### PLAYER Words

The starbase chain hangs off PLAYER.DAT: `0x44..0x45` is the record number of
the owner's first base and `0x46..0x47` of the last, 0 for none. ECMAINT
normalises `0x46` from 0 to 1 for the one-base fixture. [C][P] Rust names
them `first_base_record_raw` and `last_base_record_raw`, walks the chain from
`0x44` ([base_chain.rs](../../rust/nc-data/src/directory/base_chain.rs)),
rejects a chain that runs past the file, loops, or reaches another empire's
record, and sets `0x46` to the tail after maintenance. A destroyed base is
unlinked from its chain before its record is zeroed.

### Chain And Creation

Each empire's bases form a doubly linked list sorted by id. A new base
(`2000:6493`) takes the lowest id not in use: walking from the head with a
counter starting at 1, it inserts before the first base whose id exceeds the
counter, or appends at the tail. It fixes the neighbours' `0x05` and `0x09`
and PLAYER `0x44` or `0x46` as needed. [C] Rust's commission does the same.

The new record is set to escort 0, `tx, ty = x, y`, `TZ = 1`, moving flag 0,
`fx = fy = 0.5`; the caller supplies the depth. [C]

## Movement

A starbase is a kind-2 unit in the per-tick loop, stepped in the same order
as fleets: step, encounter, then the tick dispatcher (`0824:1230..124b`).
The unit entry's `x`, `y`, and layer are rewritten by the base's own step
(`0824:0b23..0b4f`). [C]

### Step (`0824:02d8..0f8c`)

1. The base is under way when its position differs from its target:
   `x + fx != tx + 0.5`, `y + fy != ty + 0.5`, or `Z != TZ` (with `TZ`
   clamped to 1). The moving flag is not consulted. [C]
2. An under-way base moves exactly like a fleet of speed 1: a budget of
   `1/52` a tick, spent climbing (if outside the target sector or deeper
   than `TZ`), then flying straight toward the target sector center, then
   descending toward `TZ`. The code is the fleet step itself
   (`0824:069e..0ae8`), so the 6-byte real rounding matches. A base at rest
   in orbit takes a full year to climb out and then covers one sector a year.
   [C][P]
3. Before stepping, the base looks for its escort: the first active fleet
   unit of the same owner whose record word 0 equals the base's escort word,
   or that shares the base's sector and layer, and whose record has order 4,
   `0x23` equal to the base id, and speed 0 (`0824:042a..05ab`). [C]
4. After stepping, if there is an escort, it takes the base's `x`, `y`,
   `fx`, `fy`, and `Z`, its target becomes the base's sector with
   `TZ = Round(Z)`, and its unit entry follows (`0824:0c46..0d33`). [C][P]
5. If the step changed the base's sector or layer, every active fleet of the
   owner with order 4 and `0x23` equal to the base id gets the base's sector
   as its target and `Round(base Z)` as its `TZ` (`0824:0e2a..0f7f`). Such a
   fleet that is not the escort but now shares the base's sector and layer
   is merged into the escort (`2895:4a6b`) when there is one. [C][P]

### Tick Dispatcher (`0b5d:5793..5abd`)

After the step and the encounter pass, each tick:

1. At target: `x == tx`, `y == ty`, and either `Z == TZ` exactly or
   `Round(Z) == TZ` on tick 52. A base at target with the moving flag set
   clears it and reports "We have arrived at our destination and are
   beginning our assignment of Protection & Enhancement." (`TZ = 1`) or
   "...Deep-Space Surveillance." (`TZ = 0`). A base not at target sets the
   flag. [C][P]
2. A base needs a friendly world:
   - moving with `TZ = 1`: the world at the target sector must be its
     owner's;
   - not moving and `Round(Z) == 1`: the world at its own sector must be its
     owner's;
   - otherwise there is nothing to check.
3. When the check fails, the base reports "Since we have lost control of
   planet "..." in System(tx,ty), " and then:
   - if it is moving and not already in orbit at that world (same sector and
     `Round(Z) == 1`), it holds: `TZ = Round(Z)`, `Z = TZ` (snapped), moving
     flag 0, `tx, ty = x, y`, "we are holding our current position and
     awaiting new orders.";
   - otherwise it lifts: `TZ = 0`, moving flag 1, target unchanged, "we are
     moving the base out of the solar system. Our new destination is
     Sector(x,y)."; on tick 52 `Z` is also set to 0. [C][P]

A base that lifts on tick 1 climbs for the remaining 51 ticks, ends the year
at `Z = 1/52`, and passes the tick-52 arrival rule with `TZ = 0`, so the flag
is 0 again at year end. [P]

### Guards

A Guard Starbase fleet finds its base by owner and id (`1b05:2133`). If no
base matches, ECMAINT logs "Fleet assigned to an unknown starbase." and the
fleet holds. If the base was destroyed, the fleet reports it and holds
(`0b5d:0c17`). [C]

The fleet arrives when it shares the base's sector and depth layer and its
speed is above 0 (`0b5d:0b3e`). On arrival (`0b5d:0e47`) its speed becomes 0
and it reports "We have arrived at Starbase N". If another fleet of the
owner already guards the base there (same sector and layer, order 4, same
id, speed 0), the newcomer merges into it, or holds when the merge would pass
3000 ships. Otherwise it becomes the escort: the base's escort word takes the
fleet's `0x00` word, and the fleet's `Z` takes the base's `Z` (`0b5d:10a7`).
[C]

## ECGAME Starbase Command

Move/Halt (image offset `0x1a37c`) offers Move only for a base at its
target (`Round(Z) == TZ`, `x == tx`, `y == ty`), first clearing its moving
flag, and Halt only for a base under way. [C]

- Move first checks a base fully in orbit (`Z >= 1.0`). If the world beneath
  it is the player's and its builds rely on the base (the dearest queued unit,
  or the whole queue, costs more than the world's production) and no other
  base rests in orbit there, ECGAME warns that moving the base cancels all of
  the world's builds and asks whether to continue. "Yes" clears the whole
  build queue with no refund. [C]
- Move asks for the destination. When the player owns the world there,
  ECGAME asks whether to move the base to it; "yes" gives `TZ = 1` and "no"
  aborts. Any other destination gives `TZ = 0`. A destination equal to the
  base's own sector and layer aborts. After the ETA and confirmation,
  ECGAME cancels any planet build that relies on the base, then writes
  moving flag 1, `tx, ty`, and `TZ`. [C]
- Halt never touches builds. It writes moving flag 0, `tx, ty = x, y`, and `TZ = Round(Z)`. `Z` is
  left alone; the base finishes its climb or descent to that layer. [C]

Rust's Starbase Command writes the same fields: Move sets `TZ = 1` exactly
when the player owns a world at the destination, and it cancels the builds
that rely on the base as ECGAME's "yes" does. The Rust interface does not
show ECGAME's warning first.

## Rust Differences

Rust follows the rules above except in four places. The first three fix
classic guard bugs, by decision of 2026-10-10
([t040](../notes/plans/q0a0mhm2-classic-follow-ups-from-the-october-2026-oracle-work.md#t040)):

- A guard that becomes its base's escort takes the base's layer as its
  `0x22` as well as the base's `Z`, so it reports its arrival once. Classic
  keeps the `0x22 = 1` that ECGAME writes for the order; when the base is in
  deep space, the guard gets its speed back and arrives again every tick,
  reporting each time.
- A guard at rest in orbit in the sector of a base in deep space takes
  `0x22 = 0`, climbs out, and arrives at the base as a guard from elsewhere
  would, becoming the escort or merging into it. In classic the dispatcher's
  prelude counts the guard as arrived in orbit, so it never moves.
- When a base lifts on tick 52 and drops to `Z = 0`, its escort follows it,
  as it does after every step: the escort takes the base's position and
  `Z`, and `TZ = 0`. Classic leaves the escort at its old depth.
- Classic reads a garbage owner when the world it checks is missing (a
  worldless target sector or orbit). Rust treats a sector with no world as
  nobody's, so the check fails.

## Probes

Scratch probes start from `fixtures/ecmaint-starbase-pre` (empire 1's
Starbase 1 in orbit at (16,13), escorted by fleet 1).

| Probe | Change | Result |
| --- | --- | --- |
| move | base `0x1f = 1`, target (18,13), `TZ = 0` | year 1: Z falls from 1 to about 0 at (16,13); year 2: (17,13), `fx` 0.4999999; year 3: (18,13), flag 0. Fleet 1 follows every year: same position and `Z`, target = base sector, `TZ = 0`. |
| lift | planet at (16,13) given to empire 2 | year 1: `Z = 1/52`, `TZ = 0`, flag 0, target unchanged; fleet 1 carried to the same `Z` with `TZ = 0`. PLAYER `0x46` normalised to 1. |

The pre and post files of both probes are preserved under
`fixtures/ecmaint-starbase-move-*` and `fixtures/ecmaint-starbase-lift-*`.
