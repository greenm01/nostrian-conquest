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

Chain pointers use 1-based record indexes:

- header `ChainId` = previous header index + 1, or `0` for the first report
- header `NextChainId` = next header index + 1, or `0` for the last report
- continuation and EOT records must have `NextChainId = 0`
- all records in one logical report share the same `ChainId`

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
is correct.

## Export Boundary

Use this doc for:

- `RESULTS.DAT` generation
- record counting
- chain pointer validation
- wrap-width behavior at the classic boundary

Do not use it as the player-facing wording authority for Rust reports.
