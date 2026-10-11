# Classic RESULTS.DAT Compatibility

This doc covers the classic-compatible `RESULTS.DAT` export contract.

It is not the player-facing Rust report style spec. For Rust-engine report
format and wording, use [nc-reports.md](nc-reports.md).

## Purpose

Keep this doc focused on the classic file boundary:

- packed record layout
- chaining semantics
- wrapping and footer rules
- fields that ECGAME relies on when reading `RESULTS.DAT`

## Binary Format

Each logical report is encoded as 84-byte packed Borland Pascal records.

```
Offset  Size  Field
0       1     Kind byte
1       73    Text: String[72] (1 byte length prefix + 72 chars)
74      10    Tail: chain pointers + year
```

Tail layout:

```
Offset  Size  Field
74-75   2     ChainId (u16 LE)
76-77   2     Reserved (zero)
78-79   2     NextChainId (u16 LE)
80-81   2     Reserved (zero)
82-83   2     Year (u16 LE)
```

## Kind Byte

The kind byte is the logical report record count, not a fixed report-family
identifier.

Implementation rule:

- compute it as `text_lines + 1`
- include the `<end of transmission>` record in that count
- write the computed kind to every record in the logical report

## Chain Semantics

`RESULTS.DAT` is one file shared by every empire. Each empire's reports form
their own linked chain of report headers. Chain pointers are 1-based record
indexes into the whole file:

- header `ChainId` = record index of the previous header in the same empire's
  chain, or `0` for that empire's first report
- header `NextChainId` = record index of the next header in the same empire's
  chain, or `0` for its last report
- continuation and EOT records must have `NextChainId = 0`
- all records in one logical report share the same `ChainId`

Each report belongs to exactly one empire's chain. Original ECMAINT appends
reports in event order, so chains of different empires can interleave in the
file. A file whose chains sit in contiguous per-empire blocks is equally valid.

## PLAYER.DAT Pointers

Each `PLAYER.DAT` record points at its empire's chain:

```
Offset     Size  Field
0x38-0x39  2     FirstHeader (u16 LE): record index of the first header, or 0
0x3A-0x3B  2     zero
0x3C-0x3D  2     LastHeader (u16 LE): record index of the last header, or 0
0x3E-0x3F  2     zero
```

An empire with no reports has both pointers zero. `+0x3C` is the last header
of the chain, not a record count or a next-free index.

## Rust Export Rule

The Rust engine stores report rows per viewer. A row with viewer `0` is a
broadcast report. `nc_data::layout_classic_results` is the one place that turns
rows into a classic file:

- write each empire's visible rows together, empires in ascending order
- copy a broadcast row into the chain of every active human empire
- rewrite every record's chain ids to absolute header indexes
- return each empire's first and last header for `PLAYER.DAT`

The `nc-compat` export, the maintenance commands, the scenario harness, and
the player client all set the `PLAYER.DAT` pointers from this layout through
`ClassicResultsLayout::apply_to_players` or `classic_results::set_results_chain`. The
`nc-compat` import reverses it: it walks each empire's chain from `+0x38` and
tags the report with that empire, so a re-export keeps every chain.

Deleting a report only removes it from the deleting empire's view.
`nc_data::delete_report_block_for_viewer` replaces a deleted broadcast row
with a copy for each other active human empire, so their chains keep it, now
after their own reports. The engine no longer writes broadcast rows; they come
from older campaign databases and from imported reports no chain reaches.

Reports an empire has not deleted survive maintenance. Each year's rows are
last year's unread rows, in their chain order, followed by the year's own
(`nc_data::carry_unread_report_blocks`), so an export's chain holds an
empire's older unread reports before its new ones, as classic's does.

## Oracle Evidence

Original ECMAINT runs with two and three joined human empires are preserved
as `fixtures/ecmaint-results-two-human-*` and
`fixtures/ecmaint-results-interleaved-*`. The
[investigation note](../notes/investigations/e752ccc3-classic-results-dat-chain-ids-across-several-players.md)
records the setup and the observed layout.

## Text Contract

The exported text still follows the classic shell:

- first line contains the source clause and `Stardate`
- body wraps at 72 characters
- one logical report ends with exactly one `<end of transmission>` line

Compatibility rules:

- encode text as printable ASCII before wrapping; any other character becomes
  `?`, so one character is always one record byte
- the length byte never exceeds 72; wrapping and padding count bytes
- body lines that already fit in 72 bytes are written unchanged (structured
  label columns keep their padding); only longer lines are re-wrapped
- preserve explicit blank body lines when they are part of the rendered report
- do not let wrapped text overflow into the next logical report
- fleet-origin headers must name a fleet when that identity is known

## String Semantics

The text payload is a Borland Pascal `String[72]`:

- byte `0` is the string length
- bytes `1..=72` are character data
- ECGAME and BP string comparison use the declared length, not trailing bytes

Rust may zero-fill trailing bytes. Original EC sometimes reused old buffer
contents. That difference is acceptable as long as the declared string content
is correct. The import decoder ignores bytes past the declared length in
records whose kind byte is nonzero.

## Export Boundary

Use this doc for:

- `RESULTS.DAT` generation
- record counting
- chain pointer validation
- wrap-width behavior at the classic boundary

Do not use it as the player-facing wording authority for Rust reports.
