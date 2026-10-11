# ASCII ship notation

**Status:** approved by the owner on 2026-10-11; implemented in
[t093 and t097](../notes/plans/tafp4e55-compact-crippled-ship-notation-and-condition-aware-detach-and-transfer.md).
This is the common notation for every compact ship
composition, including healthy fleets: the door and helm fleet screens, the
planet stardock and build-queue lines (a scouted planet's `Docked` line
included), the reports and the CLI's output. The plan's Implementation
section lists the surfaces. Prose keeps full ship names (casualty lists that
name transports in words), and numeric columns keep their class headings,
in the same class order.

## Format

Write the ship code, total count, then an optional crippled count:

```text
CODEtotal!crippled
```

There are no spaces inside a token. Separate tokens with one space.
Counts are decimal integers without padding or leading zeroes. Always show
the total, including for a single ship. The crippled count is included in
the total, never added to it. Omit `!crippled` when it is zero. Omit ship
classes with no hulls from a compact composition; use the existing empty
fleet label if nothing remains.

| Token | Meaning |
| --- | --- |
| `SC1` | One scout. |
| `BB2!1` | Two battleships total; one is crippled. |
| `CA12!3` | Twelve cruisers total; three are crippled. |
| `DD8` | Eight destroyers, none crippled. |
| `TT*4!1` | Four loaded transports total; one is crippled. |
| `TT5` | Five empty transports. |
| `ETAC2` | Two reusable colonization ships. |
| `SB1!1` | One starbase, crippled. |

Use `DD`, `CA`, `BB`, `SC`, `TT`, `TT*`, `ETAC` and `SB` consistently.
Starbases use the same syntax where a base composition is shown; this does
not make them transferable ships. Empty and loaded transports are separate
categories: `TT*4!1 TT5` means nine transports total, not five plus four
additional armies counted as ships. Keep cargo and condition distinct:
`*` means loaded, `!` introduces the crippled count.

Example fleet:

```text
BB2!1 CA12!3 DD8 TT*4!1 TT5 ETAC2
```

Short legend:

```text
!n = crippled ships included in total; TT* = loaded transports
```

A lone `!` beside a fleet label means that fleet contains at least one
crippled ship. It does not mean the entire fleet is crippled. This marker
must remain visible when the composition text has to be shortened.

## Where to use it

Use this grammar across compact composition strings in fleet lists,
details, ship/fleet reports, manifests and operation previews or summaries.
Do not alternate between `12CA`, `CA 12`, `CA12 (3!)` and `CA12!3` for the
same kind of composition. Reuse one formatter across those surfaces.

Full ship names remain appropriate in prose, explanations and menu labels.
Dedicated numeric columns retain their class headings and numeric cells,
in the class order below (the door's commission picker reads
`SC BB CA DD TT ET SB`); do not insert a ship code into every numeric cell
or widen those tables.
Every composition lists its classes in one order, the fleet screens' order:
`SC BB CA DD TT* TT ETAC SB`. One formatter writes it
(`nc-data/src/ship_notation.rs`). A planet's stardock and build-queue lines
follow the starbases with the two non-ship build items, `AR` for armies and
`GB` for ground batteries (`BB5 CA10 DD4 TT6 ETAC3 SB2 AR8 GB7`). New and
docked transports are empty, so they read `TT`. Keep BBS screen widths,
columns, prompt rows and hotkeys. Do not add recovery-time columns or
countdowns.

Show only information that the player is entitled to know. Omitted damage
on incomplete enemy intelligence is not a guarantee that the ships are
healthy. Never invent per-category damage counts or expose hidden state to
complete a token. In particular, loaded/empty transport damage must agree
with the underlying state and with ship selection, not an arbitrary display
split of a shared transport total.

The player manual must explain the grammar beside fleet composition and
recovery, with the examples and short legend above. Existing door help
should repeat the short legend where useful, without adding a new screen.
Update the manual and help alongside the implementation so they describe
available behavior.

## Quantity input is different from composition output

In the existing Detach and Transfer quantity prompt for a selected class:

- `3` selects three ships, healthy first and then crippled if needed.
- `3!` selects exactly three crippled ships. Reject it if unavailable;
  never substitute healthy hulls.

`3!` is an input selector, not a composition token. A confirmation such as
`CA3!1` means three ships will move, including one crippled. `CA3!3` means
all three selected cruisers are crippled. A short prompt hint can say
`Amount (3! = crippled only)`.

Among crippled ships, select the longest remaining recovery first and
preserve their recovery state. Existing cargo, ownership, co-location and
mission rules still apply. See [the implementation plan](../notes/plans/tafp4e55-compact-crippled-ship-notation-and-condition-aware-detach-and-transfer.md)
for selection, validation and fixed-width rendering acceptance checks.
