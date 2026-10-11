---
id: atdsvb5y
date: 2026-10-10
kind: adr
status: accepted
tags: [adr, combat, surprise, encounter, storage]
---
# Surprise uses local sighting memory

## Context

Decision 6 of the [combat review](../investigations/ebrogywq-nc-combat-design-review-against-its-sources.md)
proposed surprise as a first-exchange bonus. The engine's encounter
awareness cannot carry it: `encounter::run_year` resets every awareness
byte at the start of each year, so a standoff (a blockade guard sitting on
a garrison, two guards in one system) would hand a fresh surprise to
whoever sights first every year. A cheaper rule, no surprise between units
both present when the pass began, removes that artifact but also denies
surprise to forces that stayed hidden: one orbital blockade guard leaves
10 Hold CA unseen after a full year's 52 scans with probability 1.1%, and
one Hold DD with probability 37.6%; after four scans (a late arrival) the
rates are 70.6% and 92.8%.

## Decision

The owner approved persistent sighting memory on 2026-10-10. This record
covers surprise only; decision 6's participation items (admission,
withdrawal, loss allocation) and the first-identification ROE bypass are
separate, still pending.

1. **Memory.** Each viewing empire keeps, per sector and depth layer, a set
   of opponent empires it has sighted there. An actual sighting or a combat
   in that sector and layer adds the opponent. The set is shared by all of
   the viewer's fleets and starbases there.
2. **Lifetime.** The set persists across year boundaries while any of the
   viewer's fleets or starbases remains in that sector and layer, and is
   cleared when that local presence ends. An opponent leaving and returning
   does not clear it. It is local readiness, not empire-wide knowledge.
3. **Surprise.** Eligibility is resolved before combat raises awareness:
   the attacking side gets +1 on its first ordinary exchange when the
   defending empire's memory there does not include the attacker's empire.
   The +1 counts within the existing +2 modifier cap. Natural-9 criticals
   are unchanged, and pursuit exchanges never refresh surprise. Detection
   and interception rolls do not change. Only the attacking side can be
   surprising: a hidden defender joining the battle earns no counter-bonus,
   because surprise means catching the defending side unprepared.
4. **Storage and export.** The memory is typed state with its own SQLite
   storage, saved and loaded deterministically. Classic export cannot
   represent it: export omits it, says so, and never alters the store. A
   fresh classic import starts with no memory. No new menu or player
   setting.

## Alternatives

- Transient per-year awareness: a recurring standoff gets a fresh surprise
  every year.
- No surprise between units present at the start of the pass: no storage,
  but it removes surprise from hidden raiders, deep-space forces, and late
  arrivals (the percentages above), and makes two similar approaches differ
  only because one crosses a year boundary.
- An empire-level snapshot: a hidden scout present at year start would deny
  surprise to its own empire's later battle fleet; a per-unit snapshot would
  let a fresh reinforcement qualify though the enemy already knows its
  empire is there. Opponent-specific memory avoids both.

## Consequences

Reconnaissance and concealment matter: staying unseen keeps the chance of
a surprise first exchange, and watching a system keeps a defender ready.
The cost is a new typed store of opponent masks, updated on sightings,
combat, movement, elimination, and ownership changes, with save/load and
boundary tests. The percentages above are conditional eligibility rates,
not campaign ambush rates.

Implementation is pending, after t039: the model and SQLite table, the
encounter-loop updates, the first-exchange bonus, the classic export
disclosure, the combat spec, and the manual's combat chapter.

## Acceptance and connections

Accepted by the owner on 2026-10-10 after Codex's comparison and Claude's
review. Evidence:
[m1h272uh](../investigations/m1h272uh-joined-participation-surprise-and-allocation-fairness.md)
and `tools/combat-model/surprise_memory_review.py`. Related:
[decision 2](kiswrzos-withdrawal-pursuit-and-one-strength-yardstick.md)
(the modifier cap and pursuit), [the engagement ADR](gtkmxw3l-engagement-follows-the-classic-ecmaint-encounter-rule.md)
(classic sighting and awareness, unchanged).
