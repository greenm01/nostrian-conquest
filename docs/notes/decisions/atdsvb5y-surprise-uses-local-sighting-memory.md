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
withdrawal, loss allocation) and the first-identification ROE bypass are in
[gzly58c5](gzly58c5-fleets-of-one-empire-fight-together-and-first-contact-obeys-roe.md).

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

The implementation is done (see Implementation).

## Implementation (t079)

**State.** `SightingMemory` (nc-data model) sits on `CoreGameData` as
`sightings`: for each (viewer empire, sector, layer) one `u32` mask of the
opponents it has sighted, bit `empire - 1`. The store keeps it in its own
table, `snapshot_sightings` (viewer, sector_x, sector_y, layer, opponents),
and `RUNTIME_SCHEMA_VERSION` is 16, with no migration. The state hash feeds
the sets in only when any exist, after the conquest record, so the pinned
fixture seeds stand. A classic import and `load_classic_core_files` start
empty; the codecs never see it.

**Writes.** In the encounter handler, `note_sighting` runs after every
passed sighting roll, whoever the observer is (a fleet scanning a fleet, a
fleet scanning a base, a base scanning anything), before the contact report
or the decision. A fleet whose ROE then declines the fight, or whose
trigger admission cancels, has still sighted. `note_combat` puts each of two
fighting empires in the other's set, only where it has a unit in play, in
`battle` (so at the orbital gate too) and when a strike's world belongs to
another empire (a planetary assault is combat; the ADR left it open).

**Surprise.** `battle` reads the defending empire's set for the attacker
before `note_combat` and passes the result down `fight` and
`fight_encounter` to `fire_exchange`. Round 1 of the battle gives the
attacking empire's side +1 through `fleet_die_modifier(state, surprise)`,
which adds it to the mixed and starbase modifiers under the same cap of 2;
`pursue` passes none. Criticals are `roll == 9` as before, and no detection,
awareness, ROE, or intercept code reads the memory. `FleetBattleEvent` gains
`surprise`; the battle report (and the fleet-lost variant) adds "We attacked
by surprise." for the attacker and "The enemy attacked by surprise." for the
defender.

**Lifetime.** Every point where the unit table moves a unit out of its
sector or layer, or takes it out of play, records the empire's place left in
`Year::departed`: `follow_fleet`, `follow_base_step` (the base, its escort,
the guards merged into it), `strike`, a fleet's removal at dispatch, and the
recompute after each battle. `settle_sightings` runs at the end of every
unit's turn and forgets each such place that the empire has no unit in play
at, so a gap inside a year clears the memory even if a second unit arrives
later in that year. Everything outside the ticks is settled against the
fleet and starbase records by `prune_sightings`, at the start of
`run_year` and at the end of the maintenance turn after colonization and
defections. Nothing in the engine changes a unit's owner, so there is no
ownership hook.

**Export.** `db-export` prints "Sighting memory at N place(s) was not
written: classic files have no sighting memory, so surprise starts fresh
from the export (the campaign store keeps it)." when the store holds any,
beside t066's crippled-hull notice, and the export never writes to the
store.

**Tests.** `nc-data/tests/sighting_memory.rs` (set arithmetic, per-snapshot
store round trip, export omission and fresh import) and
`campaign_seed.rs` (the seed sees the memory, and no memory leaves the
pinned seed); `nc-cli/tests/storage.rs` (the notice); scripted dice in
`fleet_battle_tests.rs` (+1 in the attacker's first exchange only, the
defender gets nothing and the empire numbers do not matter, the cap, the
natural 9, no bonus in a pursuit exchange); whole years over seeds in
`tests/encounter/memory.rs` (a sighting is remembered and kept across years;
a combat is remembered by the survivors; surprise exactly when the defender
had not sighted, with a watching starbase's share checked against the Python
control's formula; a declined sighting still counts; memory from an earlier
year removes the surprise and unrelated memory changes nothing; memory lasts
while the empire stays, whatever the opponent does; it ends when the last
unit leaves or dies; a gap inside a year clears it); and a report test.
Against `surprise_memory_review.py`'s formula for a starbase scanning a
blockading stack in deep space (factor 30, five cruisers at 1.1 times),
3,959 attacks over 4,000 seeds had 1,841 follow the base's own sighting,
where the formula puts the count between 1,743 and 1,916 (the bounds are a
tie at the attack's own tick resolved each way).

**Changed expectations.** The store's schema tests expect version 16 (and
refuse 15), and its table list gains `snapshot_sightings`. Nothing else
moved: no existing battle test depended on the absence of a surprise.

## Acceptance and connections

Accepted by the owner on 2026-10-10 after Codex's comparison and Claude's
review. Evidence:
[m1h272uh](../investigations/m1h272uh-joined-participation-surprise-and-allocation-fairness.md)
and `tools/combat-model/surprise_memory_review.py`. Related:
[decision 2](kiswrzos-withdrawal-pursuit-and-one-strength-yardstick.md)
(the modifier cap and pursuit), [the engagement ADR](gtkmxw3l-engagement-follows-the-classic-ecmaint-encounter-rule.md)
(classic sighting and awareness, unchanged).
