---
id: ghgq0b1j
date: 2026-10-11
kind: plan
tags: [plan, door]
---
# Keep fleet actions aligned with the highlighted fleet

## t105

The owner reported a fleet-list ID differing from the detach donor. Two
regressions reproduce selection mismatches: descending prefix matching can
highlight Fleet 14 after typing 1 while detach resolves Fleet 1; navigating
with arrows leaves the typed number active and overrides the new highlight.

Prefer an exact numeric ID when highlighting, while retaining prefix input
for longer IDs. Navigation clears that input, so subsequent actions target
the highlighted fleet. Keep the record-index versus fleet-number distinction
unchanged: it is not the cause of these failures.

Validation: both identity regressions failed before the fix. All 672
nc-game tests passed after it, including typed-ID, leading-zero, sort,
filter, detach and transfer tests.
