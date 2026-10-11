---
id: p01cg7ll
date: 2026-10-10
kind: adr
status: accepted
tags: [adr, combat, recovery]
---
# RT controls automatic ship recovery

## Decision

The owner approved **RT — Recovery Turns** as a tunable ship-class stat.
RT is the number of full game turns (years) a surviving crippled ship needs
to recover automatically to nominal, full strength.

Until recovery completes, the ship retains its crippled combat strength.
Recovery continues while moving. It requires no payment, repair order,
shipyard, owned planet, or local production capacity. Destroyed ships do
not recover. Splitting, transferring, or merging fleets preserves remaining
recovery time. Those operations must never reset or accelerate recovery.

The maintenance year in which a ship is crippled does not count as a full
recovery turn. Count subsequent complete maintenance years. For RT 2, a
ship crippled during year N remains crippled through N+1 and recovers after
N+2 completes. Encounter count does not advance recovery.

The owner also approved automatic starbase-assisted recovery. Ordinary
recovery still works away from bases. The proposed bonus is one extra
recovery turn after a full year stationary at an owned starbase, with no
recovery during the damage year. The exact eligibility and bonus accounting
need to be finalized before implementation; arrival or merging at year-end
must not create a full year's eligibility.

## Player display and BBS compatibility

The owner requires minimal changes to existing tables, reports, and menus.
Preserve BBS door widths, row heights, navigation, and key bindings. Do not
add a repair menu, widen fleet tables, or expand every fleet into per-class
recovery rows.

Use a compact aggregate such as “4 damaged” in the fleet summary only where
it fits the existing layout, and crippled counts by class in the existing
fleet detail view. Players see nominal or crippled, not remaining recovery
time; there is no RT column in any table. Reports
normally summarize “4 ships recovered”, adding starbase assistance only
when relevant. Avoid routine per-class recovery chatter.

Recovery remains visible and understandable without exposing internal
counters. Verify the changes in the existing BBS door layout, including
mixed fleets and large counts, before shipping. If space is insufficient,
use the existing detail view rather than changing the table geometry.

## Scope

The automatic recovery concept and the RT name are approved. The owner set
the values on 2026-10-10 after Claude and Codex's joint recommendation:
**RT 2 for every class, starbases included.** A ship damaged this year
fights damaged next year and the year after, and returns to full strength
after that; a full qualifying year at a friendly starbase speeds that up.
Because every class has the same value, RT is one game-wide rule, not a
per-class stat, and no ship table shows it (owner, 2026-10-10). The only
state a player sees for a ship or fleet is nominal or crippled; the manual
states the recovery rule once, and reports say when ships recover. The
engine still tracks each crippled hull's remaining time. If a later
playtest wants classes to differ (DD 1 was a candidate; BB 3 is not used),
RT can become a per-class value then. RT 2 is the shortest value that
lets the starbase bonus matter; with RT 1 every eligible hull recovers in
the same pass anyway. Codex's campaign controls (t068, in
[4lasivnr](../investigations/4lasivnr-ground-combat-and-recovery-turns-campaign-calibration.md)):
fleets that fight every second year lose 184 build points under RT 2
against 105 under RT 1, and are always damaged at the next fight; under
DD 1 / CA 2 / BB 3 a battleship fleet fighting every third year is wiped
out 38.8% of the time against 13.8% under RT 2.

The owner rejected paid repairs and shipyard management. The earlier
25%-cost and installment proposals are historical alternatives, not future
implementation work. The owner subsequently chose automatic recovery;
there is no accepted policy that all damage must end with the battle.

[Decision 1](ld2f252j-flat-combat-effectiveness-with-battle-local-carry.md)
still applies: crippled finishing costs full DS, and unused hit banks end
with the encounter. Recovery tracks completed crippling, not unused hits.
Withdrawal and pursuit remain a separate owner decision.

## Timing in the turn cycle

The owner agreed this timing on 2026-10-10 after review by Claude and
Codex. A turn is one maintenance pass plus the order-writing time between
passes. A pass culls empty fleets, merges co-located fleets, recalls
autopilot fleets, and runs the planet pass (builds, then economy); then the
52 ticks of movement, encounters, battles, and strikes; then post-year
merges and colonization, the AI, player statistics, lifecycle, and reports.

1. **No recovery inside a pass.** A ship crippled at tick 10 is still
   crippled at tick 30 and in every later battle until it recovers. The
   engine's restoration of crippled ships after each battle goes.
2. **Recovery runs once per pass,** after the 52 ticks, the post-year
   merges, and colonization, and before the AI and reports. No battle
   happens between the end of one pass and the start of the next, so for
   combat this is the same moment as the next pass's start; running it here
   lets reports and the AI show the state orders will be written against.
3. **Counters.** Each fleet and starbase keeps, per class, its crippled
   hulls counted by turns remaining (1 to the class's RT), plus a separate
   count of hulls crippled during the current pass. At the recovery step,
   every turns-remaining count drops by one, hulls that reach zero recover,
   and the hulls crippled this pass enter at RT. RT 2 crippled in pass N is
   crippled through N+1, recovers at the end of N+2, and fights whole from
   N+3. Merges add counts, the current pass's count included.
4. **Starbase assistance** (direction approved, eligibility still to be
   finalized). The extra turn can apply only to hulls that were crippled
   before the pass began, belonged to the fleet when it began, and stayed at
   rest in orbit of the same owned world, with the same starbase and the
   same owner, through all 52 ticks. Hulls crippled this pass, and hulls
   that arrived or merged in during the pass, never receive it; a late
   arrival or merge cannot pass its hulls off as eligible. A starbase does
   not assist its own recovery. The engine decides this from a snapshot
   taken at the start of the pass and kept only for that pass, so it needs
   no new stored state.
5. **Finishing and detaching.** Hit allocation still chooses the class and
   whether it strikes nominal or crippled hulls; within that choice, fire
   that finishes a crippled hull takes the one with the most turns
   remaining first, newly crippled hulls before all others. Detaching
   crippled hulls follows the same order. A surviving crippled hull keeps
   its count; nothing resets it. A fleet's survivors stay closest to
   recovery.
6. **Strength tests see damage.** Every strength path counts a crippled
   hull at its crippled value and keeps its current weights: the ROE
   engagement test and the orbital gate use classic's weights (BB 21, CA 7,
   DD 2, and the starbase constant) with a crippled hull at half;
   bombardment keeps its 5/30/135 weights and its current crippled rounding.
   No path switches to combat AS; replacing the classic weights is the
   separate yardstick question in decision 2.
7. **Other events.** Destroyed hulls leave their counts. A fleet that
   changes owner, by capture or civil-disorder defection, keeps its counts,
   and reports name it by its new identity. ETACs are reusable: colonization
   consumes none, and recovery changes no mission ability. A classic export
   between passes writes whole hulls, says that recovery state is lost, and
   leaves the store unchanged.

## Implementation proposals and remaining choices

Use counts by ship class and remaining recovery turns, rather than per-hull
objects. Merge adds matching counts. Bound the bucket range by the supported
RT values. The counters, including the separate count for hulls crippled
in the current pass, are defined under Timing in the turn cycle.

For detaching crippled hulls, Claude proposes taking the longest remaining
recovery times first, leaving the retained fleet closest to full strength.
Codex agrees this is deterministic and explainable. This is an implementation
proposal, not an extra player choice. Document the default selection between
nominal and crippled ships as well; no operation may lose recovery metadata.

The detail view shows crippled counts by class; routine reports use the
compact aggregate described above. Export behavior for recovery metadata still
needs an explicit compatibility contract: classic has no such field. The
consultation recommends exporting whole surviving hulls without changing
the SQLite state and disclosing that classic loses recovery state; this
compatibility recommendation has not been separately approved by the owner.

The engine, the display, and the manual are done (see Implementation). This
approval does not authorize a crippled speed penalty or partial AS recovery.

## Approved door-interface follow-up (2026-10-11)

The owner broadened this to consistent notation for every compact ship
composition, not only damaged ships. [ASCII ship notation](../../dev/ship-notation.md)
defines the code/count grammar, `TT*` cargo marker, `!n` damage suffix,
examples and manual/help documentation required by t093. Full names in
prose and dedicated numeric columns keep their existing appropriate labels.

The owner approved `CA12!3`: 12 cruisers total, including 3 crippled. Omit
`!n` when no ships are crippled, and use a lone `!` beside a fleet with any
damaged hulls. Keep `TT*` for loaded transports. Use this compact notation
in existing fleet details, report compositions and operation summaries,
without widening BBS tables or showing recovery timers.

Detach and Transfer accept `3!` to move exactly three crippled ships of the
selected class. Reject insufficient crippled counts without substituting
healthy hulls. Plain `3` selects healthy ships first, then crippled if
necessary. This replaces t066's crippled-first default for plain quantities;
within crippled hulls, longest remaining recovery still moves first. All
recovery state and cargo move intact. Preview the actual selection with the
same notation in the existing flow, for example `CA3!1`.

Implemented in [t093](../plans/tafp4e55-compact-crippled-ship-notation-and-condition-aware-detach-and-transfer.md).
The t066 implementation account below describes the earlier behavior;
its display and ordinary-quantity selection are superseded by this
follow-up. RT 2, automatic recovery and starbase assistance are unchanged.

## Implementation (t066)

Done in t066. The model, the engine, the display, and the tests follow the
sections above; what they leave open, and what the code settled, is here.

**Model and store.** `CrippledHulls` (`nc-data/src/model/crippled.rs`) holds,
for one class, `turns[i]` hulls with `i + 1` turns left and `fresh`, the
hulls crippled during the current pass. `RECOVERY_TURNS` is the one
constant, 2; there is no per-class value, stat, or column. `Fleet::crippled`
holds one `CrippledHulls` per class (destroyer, cruiser, battleship, scout,
transport, ETAC; loaded and empty transports share one), and
`Starbase::crippled` holds the base's one hull. The campaign store keeps
them in two child tables, `snapshot_fleet_crippled` and
`snapshot_base_crippled`, one row per non-empty class and turns-left value
(`turns_left` 0 is this pass's count). No migration: the tables are created
beside the old ones and an old store reads as nothing crippled. The state
hash feeds a class in only when it has crippled hulls, so the pinned
campaign seeds stand. A classic export writes whole hulls, and `db-export`
prints how many crippled hulls it wrote whole and that the recovery state is
lost; the store is unchanged. A classic import starts with nothing crippled.

**Battles.** A battle, bombardment, or strike loads its sides from the
records with their crippled counts and writes the result back
(`combat/damage.rs`). Finishing fire leaves the cohorts in the order of
item 5 of the timing list; the hulls it removes leave the first fleets or bases that hold
that cohort, and the hulls a battle cripples are taken from the first
nominal hulls left. A hull crippled and finished within one battle shows as
a nominal hull destroyed. The old restoration was never an explicit step:
battle state was rebuilt from the counts each time, so persisting the
counts is what removes it.

**Recovery and assistance.** `maint/ship_recovery.rs` runs the recovery step
after colonization and before the AI. The assistance snapshot (`AssistWatch`)
is taken when the 52 ticks begin, after the planet pass and the pre-pass
merges, so hulls merged at the pass start count as the fleet's own. It lists
each fleet that has crippled hulls and rests in orbit (depth 0.5 or more) of
a world its owner holds, with the owner's bases at rest there (the growth
bonus's test). After every unit's turn it strikes off a fleet whose owner,
sector, fractions, or depth changed, whose world changed hands, or whose
snapshot bases are all gone, moving, or no longer at rest. It keeps the
bases that qualified at the start and requires that one of those same bases
stay; losing one of two does not strike a fleet off. The assisted hulls are
the fleet's turns-left counts at the start, narrowed after every unit's turn
to what the fleet still holds, which makes losses take the unassisted hulls
first and keeps merged-in hulls out. Only hulls with two turns left gain
from the extra turn. A starbase's own recovery has no extra turn.

**Detaching.** A player picks ships by class, not by condition, so the
default is: a detachment or transfer takes the donor's crippled hulls of
each class first, this pass's before the rest and the most turns left
first, then nominal hulls. This leaves the fleet that stays closest to full
strength, as proposed above. Merging adds the counts.

**Strength.** `warship_attack_strength` takes the fleet's crippled counts;
`FleetFacts` carries them. The attack decision, the intercept's fallback,
the orbital gate, and the in-battle check count a crippled hull at half its
AS and a crippled base at 50 (`base_attack_strength`); `decide_vs_base`
takes the base's own strength. Bombardment keeps its 5/30/135 weights and
its 2/15/67 crippled values. The +1 die for a side with a starbase does not
depend on the base's condition.

**Display** (superseded by t093: every composition uses the ship notation,
and the legend replaces the `Damaged:` row). The BBS fleet list adds `N damaged` after the composition in the
Ships cell when the whole composition and the note fit, and otherwise draws
exactly the cell it drew before; the review screen adds a `Damaged:` row
(crippled ships by class, `BB 2DD`) in the blank row under Composition, only
when the fleet has damage. The prompt row, the columns, and the widths do not
move, and no RT column exists. The helm's fleet popup gains a `Damaged`
line in the same way. Reports say `N ships recovered to full strength`, with
`, M of them a year early with the help of the starbase` only when the
extra turn brought hulls back early; a base reports `We have recovered to
full strength.` No report names a turns-remaining count.

**Owner changes.** The engine has no path that moves a fleet or a base
between empires. A captured world passes on its docked ships, which carry no
damage state, and a civil-disorder defection removes the fleet outright. The
rule that an owner change keeps the counts therefore holds because nothing
touches them; the defection test checks that the fleets that stay keep
theirs. Any future path that changes a fleet's owner must carry `crippled`
with it.

Detaching (owner, 2026-10-11, superseding an earlier same-day choice of
crippled-first): a plain quantity takes healthy ships first, then crippled
ones if needed; `3!` takes exactly three crippled ships. Within crippled
ships, the most turns remaining go first. See the t093 paragraph below.

## Evidence and acceptance

Approved by the owner on 2026-10-10 following the discussion of automatic
recovery and the name RT. See the [decision-2/3 investigation](../investigations/7y0ypr6b-combat-withdrawal-and-persistent-damage-after-decision-1.md)
for the earlier experiments and their limits. The
[combat spec](../../dev/nc-combat-spec.md) and the player manual state the
rule as implemented.
