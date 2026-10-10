---
id: u040gnr1
date: 2026-10-10
kind: investigation
status: resolved
tags: [investigation, fuzz, turns]
---
# Turn files accept record index 0

## Question

Does the turn submission path reject a planet, fleet, host, or planet-target
record index of 0, as it rejects `turn player=0`?

## Evidence

Found by the nc-fuzz adversarial policy while building t015. The minimal
turn file is:

```kdl
turn player=1 year=3000
fleet record=0 {
  roe value=3
}
```

- `nc-data/src/turns/parser.rs` reads `record`, `host`, `to`, and `planet`
  with `prop_usize`, which accepts 0. Only `turn.player` is checked for
  1-based.
- `TurnSubmission::apply_to` then indexes `records.get(index - 1)` in the
  mutators, for example `set_fleet_rules_of_engagement` in
  `nc-data/src/directory/mutators_fleet.rs`. In a debug build this panics with
  "attempt to subtract with overflow" (observed in the fuzzer). In a release
  build without overflow checks the index wraps, `get` returns `None`, and the
  submission is rejected as a missing record (inferred, not run).
- The same parse-then-apply path backs `nc-game submit-turn`, the hosted turn
  queue, and nc-harness turn files.

## Finding and resolution

Fixed by t029 (2026-10-10) at the parser, the responsible boundary. A fleet
or planet `record`, `join host`, `transfer to`, or `load_armies` /
`unload_armies` `planet` of 0 is now a parse error ("<node>.<property> must
be 1-based"), matching `turn.player` and `commission.slot`, so no 0 index
reaches `apply_to`.

## Validation and remaining work

`turn_submission_rejects_record_index_zero_at_parse_time` in
`rust/nc-data/tests/turn_submission.rs` covers each record-bearing property.
The adversarial fuzz policy offers record 0 as an edge value again.

## Connections

- [Engine fuzzer plan](../plans/lski78er-engine-fuzzer-with-shared-policies-and-invariants.md#t015)
  found this.
