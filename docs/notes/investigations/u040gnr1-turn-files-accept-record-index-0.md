---
id: u040gnr1
date: 2026-10-10
kind: investigation
status: investigating
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

Open. The parser is the responsible boundary: every 1-based record index in
a turn file should be rejected at parse time when it is 0, matching
`turn.player`. The fuzzer's adversarial policy leaves record 0 out of its
edge values until this is fixed (`without_record_zero` in
`rust/nc-fuzz/src/policy/adversarial.rs`); restore it with the fix.

## Validation and remaining work

A fix needs a parser regression test in `rust/nc-data/tests/turn_submission.rs`
for each record-bearing node, then the record-0 edge value back in the
adversarial policy.

## Connections

- [Engine fuzzer plan](../plans/lski78er-engine-fuzzer-with-shared-policies-and-invariants.md#t015)
  found this.
