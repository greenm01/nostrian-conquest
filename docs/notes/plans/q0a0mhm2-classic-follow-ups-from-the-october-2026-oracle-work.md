---
id: q0a0mhm2
date: 2026-10-10
kind: plan
tags: [plan, oracle]
---
# Classic follow-ups from the October 2026 oracle work

The t031-t038 agents found classic behaviours and Rust gaps outside their
own tasks. Each section below is one task in [todo.md](../../../todo.md).
Policy: Rust matches the classic binaries exactly, except clear classic bugs
the user approves fixing.

## t040

Decide whether three classic starbase-guard quirks are bugs. t037b reproduces
all three today.

1. ECGAME writes `0x22 = 1` for every Guard Starbase order. A guard whose
   target depth differs from its base's layer re-arrives every tick in
   classic and sends an arrival report each time. Rust arrives once a year;
   the year-end state is the same.
2. A guard already in orbit in the sector of a deep-space base, with
   `0x22 = 1`, never arrives in classic.
3. A base that lifts on tick 52 gets depth 0, but its escort keeps its own
   depth.

Source: the t037b report; rules in [ec-starbase-spec.md](../../dev/ec-starbase-spec.md).

Decision (user, 2026-10-10): all three are classic bugs. Rust reports an
arrival once; a guard in orbit next to a deep-space base climbs to the base's
depth and joins it; an escort follows its base's depth when the base lifts on
tick 52. Document each in ec-starbase-spec.md as a deliberate difference.

## t041

Rust's economy counts a planet as having a starbase when a base is anywhere
in its sector, even in deep space. Probe classic ECMAINT to see whether its
starbase production bonus requires the base to be in orbit, and match it.

## t042

Small order fixes found by t038:

- ECGAME's composition recompute (`0x18333`) also sets ROE to 0 for a fleet
  with no warships. Rust's recompute does not.
- Rust's detach does not update the PLAYER.DAT fleet chain tail.

Result (ECGAME 1.51 disassembly and runs; addresses are image offsets in
`ecgame_640k.bin`):

- `0x18333` runs only on the donor, from `0x18049`, after a detach
  (`0xf497`), a detach-and-join (`0xda27`), or a transfer (`0xdbb2`). A
  donor left without a BB, CA, or DD gets ROE 0; any other ROE stays, even 0.
  The transfer host, load, unload, and ECMAINT's merge (`2895:4a6b`) never
  write `0x25`; ECMAINT's only ROE write is the retreat at `1b05:2b72`. A new
  detached or commissioned fleet takes ROE 6 if it has warships, else 0, then
  the ROE prompt (0-10, default 6).
- Detach and commission share `0xeac1`. The new fleet's id is the
  FLEETS.DAT record count plus one; its previous link names the last fleet in
  the chain, whose next link names it. `0x15d89` then sorts the chain by
  fleet number, moving record contents and leaving the next, id, and previous
  words in place, so a fleet number filled into a gap takes an earlier
  record. Nothing writes the PLAYER.DAT tail (`0x42`) except fleet deletion
  (`0x194be`), so after a detach it still names the old last fleet, and
  ECMAINT leaves it there. Rust matches this; whether the stale tail is a
  classic bug is open.

Rust now matches the ROE rules, the chain links, the sort, and the tail. It
still gives a new fleet the lowest unused id, which is the record count plus
one while ids are dense. Tests: `nc-data/tests/fleet_composition.rs`.

## t043

Decide how nc-game and nc-helm handle ECGAME's "seek the next closest
system?" prompt. When a fleet is already in the nearest owned system, ECGAME
asks, defaulting to yes. Rust's clients take the yes without asking; a turn
file can still name either target.

Decision (user, 2026-10-10): match classic. When the fleet is already in
the nearest owned system, nc-game and nc-helm ask "seek the next closest
system?", defaulting to yes.

## t044

Report gaps between classic and Rust:

- Classic reports a starbase sighting and an "unable to intercept" outcome;
  Rust has no report types for them (t035).
- In classic a Hold fleet's dispatch probably zeroes its speed; Rust leaves
  it, which changes the speed term in its visibility (t035).
- Classic keeps old RESULTS.DAT reports until the player reads them; Rust
  clears them each year (t031b). Decide which the runtime store should do.

Decision (user, 2026-10-10): match classic on all three. Add the
starbase-sighting and unable-to-intercept reports; zero a Hold fleet's speed
once a probe confirms classic does; keep reports until the player reads
them. Classic's purge-after setting never purged anything, so Rust adds no
cap for now.

Done (2026-10-10). The probe confirmed the speed rule: a Hold fleet at speed
3, in deep space or in a system, has `+0x0a` 0 after a year and has not
moved; Rust's dispatcher now zeroes it. The starbase sighting carries what
the base makes of the fleet (`StarbaseAlert`: headed at our planet, enemy
alert, or scanning), and a failed intercept is a `NoEngagement` with
`InterceptFailed`, both worded as classic. Maintenance carries unread report
rows into the new snapshot ahead of the year's own
(`carry_unread_report_blocks`), so the inbox, the logon review, and the
RESULTS.DAT export keep them until the player deletes them.

## t045

Finish a full reference to classic's combat, for the record; Rust keeps its
own combat system. The decode in [ec-battle-spec.md](../../dev/ec-battle-spec.md)
and [ec-encounter-spec.md](../../dev/ec-encounter-spec.md) covers units,
planetary assaults, fleet battles, encounters, and reports, checked against
1,500+ classic runs. To finish it:

- Probe the paths read only from code: the Rogues conversion, Scorch-Earth,
  IPBMs in the stardock, the scatter escape and bug B7, and an assault by a
  fleet with no warships.
- Explain the two statistical gaps: the blitz case with heavier transport
  losses, and the fleet case about 2 sigma low on losses.
- Decode starbase-only fights and IPBM attacks, starting from the IPBM
  record decoded for t039.
- Decode the DATABASE.DAT and planet-status updates after a battle.
- Merge the two specs into one classic combat reference with a short plain
  summary first; keep the Rust comparison in its own section.

## t046

A Join order stores its host as the host's fleet number within the empire
(FLEETS.DAT `0x00`), not its record id. ECMAINT's merge (`2895:4a6b`)
compares `0x23` with word `0x00`, and ECGAME's renumbering (`0x15b35`)
rewrites `0x23` when fleet numbers change. Rust's `set_join_fleet_order`,
the maintenance id remap, and the join tests treat `0x23` as the record id.
The two agree only while id equals fleet number, which holds for empire 1 in
the fixtures and fails for everyone else. Found by t042.

Evidence. ECMAINT's Join host search (`0824:79d6`) compares the joiner's
`0x23` word with the fleet index entry's number word, and the merge
(`2895:4a6b`) points the absorbed fleet's joiners at the host's `0x00`
word. ECGAME's renumbering (`0x15ad5`..`0x15b3f`) rewrites `0x23` on every
Join order that names the old number. A classic probe on
`fixtures/ecmaint-post` (empire 2's fleets are records 5..8 with numbers
1..4) ordered record 6 to join with `0x23 = 1`: ECMAINT merged it into
record 5 and compacted FLEETS.DAT. With `0x23 = 5` (the record id) ECMAINT
left every file unchanged and the Join order in place.

Rust now stores and matches the host's fleet number: `set_join_fleet_order`
writes it, validation and the maintenance merge, retarget, and removal remap
look hosts up by (owner, number), and `set_fleet_local_slot` renumbers the
empire's Join orders as ECGAME does. The turn file's `join host=` keeps
naming the host by record index, like every other fleet reference in it.

## t047

Small classic-match items from the t039 decodes and t042:

- Rust writes maintenance-day codes (`MAINTENANCE_DAY_ENABLED_CODES` in
  `rust/nc-data/src/lib.rs`) that came from an ECUTIL bug; classic reads
  the seven bytes as booleans. Write 1 for an enabled day.
- A new fleet from detach or commission starts at ROE 6 if it has warships,
  else 0, and ECGAME then prompts. nc-game passes the donor's ROE without
  asking. Use classic's default.
- Classic never updates the PLAYER.DAT fleet chain tail on detach either;
  Rust matches it. No change; recorded here so it is not reopened.
- "Purge after turns" (SETUP 518) does nothing in classic v1.5. Rust's
  setting is dead too; say so in the sysop manual or remove the setting.

## t048

Classic runs the planet economy and the build queue (`1852:0000`, called at
`0824:29d1`) before the 52-tick year driver (`0824:1004`). Rust runs
economy and builds after the tick loop, so ships commissioned this year
appear after movement instead of before it. `docs/dev/economics.md`
"Turn-Order Placement" states the Rust order as if it were classic's. Match
classic's order and correct the doc. Found by t041.

## t049

The nc-fuzz `engine_invariants_hold_under_hegel` test failed on two agents'
branches ("event names the wrong fleet" on a BombardWorld mission event)
and passes on main, three runs in a row. Find out whether it is seed- or
load-dependent and make it deterministic.

## t050

Document classic's economy in full, as a reference; Rust keeps its own
economy (user, 2026-10-10). Decode from ECMAINT `1852:0000` and ECGAME:
tax revenue, growth toward potential (`1852:0457..0560`:
`ln(1 - sqrt(present/potential))` with a `5 - Random(11)` term), the
starbase effect (a tax cut of `Random(10)` points at `1852:0526`, with the
bug that leaks it to later planets), the high-tax penalty, the planet status
scaling (0x5C: 0, 0.5, 1.0), newly colonized and captured worlds,
Scorch-Earth, build costs and the build queue, stardock and commissioning,
treasury, and the rankings. Confirm the formulas with classic probes and
write `docs/dev/ec-economy-reference.md`, with a "How NC differs" section
citing `docs/dev/economics.md`. The economy review itself (t051) waits for
the combat work.

## t051

Review NC's economy (`docs/dev/economics.md`, the manual's Economy chapter
and Appendix A) against classic's decoded economy (t050), ec4x's economy,
and the game-balance findings from the combat review; decide what to keep
and what to change. Starts after the combat mechanics work is settled.

Design constraints (user, 2026-10-10): the economy stays simple for the
player, with the tax rate as the one control; useful mechanisms may run
under the hood. The user designed ec4x's economy. No investment or research
budgets and no new economic orders as a first step. Codex's analysis of
classic EC against VBAM and ec4x on that basis is an input to this task,
kept in the [economy investigation](../investigations/tkce84f0-a-simple-nc-economy-informed-by-classic-ec-vbam-and-ec4x.md),
which links the model, campaign results, recommendations, and validation limits.

## t059

A mission abort in the encounter handler (`abort_mission` in
`nc-engine/src/maint/encounter/handler.rs`) still sends the fleet home to
the nearest owned world by Manhattan distance (`nearest_owned_planet`),
with a climb-out or hold fallback. Battle retreats and t054's orbital-gate
abort use `retreat_fleet`, which picks by classic's travel distance
(`mutators_nearest_world.rs`, t034/t053). Check which picker ECMAINT uses
for each abort, then make Rust match: one picker unless classic itself
differs. Found by the t054 agent.

Result (ECMAINT disassembly, seg:off in the unpacked image, and two classic
probes): every abort goes through one routine, `24a3:0876`, with flag 1.
It is ECGAME's Seek Home picker: the owned worlds in PLANETS.DAT order, less
the world in the fleet's sector when the fleet is in that system, by the
travel distance `24a3:04f2`, first on a tie. Its callers with flag 1 are
the fleet and base decision aborts (`1b05:1877`, `1b05:1f76`; the starbase
gate `110a:67bd` runs the base decision), the battle's scatter and retreat
(`1b05:25c2`, `1b05:2f41`, `1b05:309f`), Seek Home whose world was lost
(`0b5d:033d`), an aborted Salvage (`0b5d:4b2a`), and a defecting fleet
(`24a3:380b`); rogue and autopilot orders use flag 2 (`24a3:26c2`,
`24a3:29c5`), distance over present production. With nowhere to go, the
fleet decision climbs out, the base decision stops, the battle leaves the
order alone, and the lost Seek Home holds. The probes: a bombard fleet
turned back by a starbase at (16,13) went to (10,6), nearer as it flies,
not (18,3), nearer by the Manhattan count; a Seek Home fleet in orbit of
its own (16,13) whose target was lost went to (12,12), not (16,13).

Rust now picks with `nearest_friendly_world` (`nearest_owned_world(idx,
true)`) for both decision aborts, the orbital gate (which now aborts
through the base decision's code), the capability-loss abort, and battle
retreats, which until now could pick the world of the system the fleet was
in.
Tests: `nc-engine/tests/encounter/aborts.rs` and
`retreat_from_the_fleets_own_system_seeks_another_world` in
`nc-engine/tests/combat/retreat.rs`. Still different from classic: Seek
Home retargets before the year by squared distance, for every Seek Home
fleet, instead of in the tick its world is lost; a Salvage whose world was
lost flies on, fails on arrival, and holds, where classic seeks home in the
tick it sees the loss; autopilot recall ignores production. Open for the
owner: the picker also raises any candidate world's present production
below 1.0 to 1.0 in memory, which ECMAINT saves (see the encounter spec,
"Not settled").

Owner decisions (2026-10-10): the picker's production bump is a classic bug
and Rust does not copy it (recorded in ec-encounter-spec.md); a fleet beaten
in orbit of its own world retreats to the nearest other owned world, as in
classic.

## t064

Three home-picking paths still differ from classic, found by t059:

- **Seek Home retargeting.** `nc-engine/src/maint/retarget.rs` retargets
  every Seek Home fleet before the year by squared Euclidean distance, even
  when its target world is still owned, overriding the player's choice.
  ECMAINT retargets only in the tick the world is lost, through the flag-1
  picker (`0b5d:033d`).
- **Salvage.** A Salvage whose world was lost flies on, fails on arrival,
  and holds. ECMAINT aborts it to Seek Home in the tick it sees the loss
  (`0b5d:4b2a`).
- **Autopilot recall.** `economics/ai.rs` picks by squared distance; the
  classic AI uses the picker's flag 2, which divides distance by the world's
  present production (`24a3:26c2`, `24a3:29c5`).

Make each match classic, with probes where the disassembly is unclear.

Research (2026-10-10; ECMAINT disassembly and six classic probe groups under
DOSBox-X). The classic rules are now in
[ec-movement-spec.md](../../dev/ec-movement-spec.md) ("Seek Home And Salvage
Check Their World", "Rogue And Autopilot Orders") and, for the picker's flag
2, [ec-encounter-spec.md](../../dev/ec-encounter-spec.md). One correction to
the bullets above and to t059: an aborted Salvage does not switch to Seek
Home; it keeps the Salvage order. Per path, the Rust change and the scenarios
that tell old from new:

- **Seek Home.** Classic tests the owner of the target world every tick in
  the dispatch, after the step (`0b5d:01f9`), and retargets only when the
  world is not the empire's (lost, never owned, or no world in the sector),
  with flag 1 from the post-step position; the order, `TZ`, and speed stay,
  and that tick reports no arrival. With no world it holds: order 0, speed 0,
  target its own sector, `TZ = Round(Z)`, depth unchanged. Rust: delete
  `refresh_seek_home_targets` (`maint/retarget.rs`) and its call in
  `maint/mod.rs`, with `nearest_owned_planet_target_from_list` and the
  `sector_distance_sq` import that only it uses; in `Movement::dispatch`
  (`maint/movement/mod.rs`), for `Order::SeekHome`, before `test_arrival`
  and every tick whether or not the fleet stepped, look up the world at the
  target sector and, when it is missing or not the fleet's empire's, call
  `nearest_friendly_world` (flag 1) and apply the above; return
  `TickEnd::Underway`. Replace the invented retarget and abandon texts in
  `results/entries/misc.rs` with classic's, stamped with the tick. Scenarios
  (move fixture, empire 1 made human): a fleet at (13,13) depth 0 seeking
  owned (16,13) with (12,12) also owned keeps (16,13) (old Rust: (12,12)); a
  fleet at (13,14) seeking empire 2's (18,15) retargets to (12,12) at
  stardate 1 and ends the year over (12,12) at depth 0.67036, Hold by the
  tick-52 rule (0.76393 without the first tick toward the old target); a
  fleet seeking the empty sector (13,13) retargets the same way; with
  (16,13) given to empire 2 and nothing else owned, a fleet at (13,13) depth
  0.75 holds at stardate 1 with depth 0.69231 and `TZ` 1.
- **Salvage.** Classic runs the same check in its test (`0b5d:4a44`, arrival
  `x,y` at target and `Z >= 1.0`): with a friendly world it changes only the
  target, refills a zero speed, and keeps order 15, so the fleet salvages at
  the new world; with none it holds as Seek Home does but also sets
  `Z = Round(Z)`. Rust: the same hook in `Movement::dispatch` for
  `Order::Salvage`; the `PlanetNotOwned` and `NoPlanetAtTarget` failures in
  `movement/salvage.rs` can no longer happen and go, with their report texts;
  `test_salvage_fails_at_foreign_planet_without_removing_fleet`
  (`nc-data/tests/maint_fleet.rs`) must be rewritten. Scenarios: (16,13) to
  empire 2, (12,12) owned: a Salvage fleet (CA, ETAC) at (13,13) seeking
  (16,13) reports the abort at stardate 1 and salvages at (12,12) in tick
  44, 18 points (old Rust: flies to (16,13), fails, holds); with nothing
  owned, Salvage fleets at depth 0.75 (speed 3) and 0.25 (speed 6) hold
  with depth 1.0 and 0.0. A related gap outside the three paths: classic's
  arrival (`0b5d:4d65`) credits 23/8/3/3/10/8 per battleship, cruiser,
  destroyer, transport, ETAC, and scout and lands each loaded transport's
  army; Rust's `fleet_salvage_value` credits half the summed cost with
  armies at 2 and lands none (probe: one of each plus two loaded transports
  gave 61 points and 2 armies, Rust 59 and none; the existing test locks 10
  for a destroyer and a cruiser, classic 11).
- **Autopilot and rogue recall.** Classic's orders pass (`24a3:3cc6`, before
  the planet pass) runs `24a3:25f7` for rogue empires (every fleet) and
  autopilot empires (Hold fleets only), picks with flag 2 (all owned worlds,
  the fleet's own system included, travel distance over present production
  floored at 1.0, earlier world on ties, a lone world unmeasured), then sets
  target and `TZ` 1: in that world's sector at layer 1 with warships,
  Guard/Blockade, speed 0, max speed recomputed, ROE 10 (rogue) or 6 when 0
  (autopilot); without warships the order stays; otherwise Seek Home at max
  speed, a rogue's ROE 0. A Guard/Blockade fleet at speed 0 then takes the
  first unescorted starbase at rest in its system as Guard Starbase and
  writes the base's escort word; a rogue Guard Starbase fleet clears its
  base's escort word first (`2895:30f4`). Rogue starbases get the same pick
  (`24a3:28fe`). Rust: rewrite `autopilot_fleet_orders`
  (`economics/ai.rs`) for both empire kinds (rogue mode 0xFF with worlds;
  autopilot mode 1 with `0x6D` nonzero), drop its "outside an owned world's
  sector" filter, and add a flag-2 variant beside
  `CoreGameData::nearest_owned_world`
  (`nc-data/src/directory/mutators_nearest_world.rs`) that divides
  `travel_distance` by `max(P, 1.0)` in Real48 and writes nothing back. Rogue
  fleet and starbase orders are new behaviour in Rust, not a picker swap;
  the planetless rogue's random patrols (`24a3:0dbc`) stay out of scope.
  Scenarios (empire 1 on autopilot; (16,13) at production 5, (12,11) and
  (12,12) at 2, (2,16) at 0): a Hold fleet at (13,12) seeks (16,13) (old
  Rust: (12,12)); a Hold fleet with a cruiser in orbit of (12,12), ROE 0,
  takes Guard/Blockade with ROE 6 (old Rust: untouched); a Hold fleet at
  (12,12) with `fy = 0`, depth 0, ties (12,11) and (12,12) at 0.75 and seeks
  (12,11); a Hold fleet in (2,16)'s sector at depth 0 seeks (2,16). With an
  unescorted Starbase 1 over (12,12), the cruiser fleet takes Guard
  Starbase, `0x23 = 1`, escort word 1; an ETAC-only fleet in orbit stays on
  Hold; Move and Seek Home fleets are untouched. Rogue empire 2: a Move
  fleet at (8,13) goes home with ROE 0; its Guard/Blockade fleet at home
  keeps order 5 with ROE 10.

For the owner (classic behaviour that reads like a bug):

1. An aborted Salvage reports that it will "seek to safety of a friendly
   controlled world" but keeps the Salvage order, so it scraps itself at
   that world. Every other abort writes Seek Home; this one leaves out the
   order write. Copy it, or set Seek Home as the report says?
2. A Salvage that has nowhere to go snaps its depth to the nearer layer, a
   jump of up to half a layer; Seek Home in the same case leaves the depth.
   Copy the snap, or leave the depth?

A target sector with no world reads a garbage owner and a blank name in
classic (planet slot 0); the probe behaved as "not owned", which is what the
existing worldless-sector ruling gives Rust. The production floor in flag 2
is the bump already ruled on: Rust divides by `max(P, 1.0)` and never writes
it. Probe note for later work: a world whose present production exceeds its
potential makes ECMAINT stop without output, leaving its `.TOK` files.

Owner decisions on t064 (2026-10-10): an aborted Salvage keeps classic's
behaviour, salvaging at the nearest friendly world, and its report says it
will salvage there rather than "seek to safety"; a Salvage fleet with no
friendly world left holds at its current depth like Seek Home, without
classic's snap to the nearer layer (a classic bug).

Result (2026-10-10; t064 and t069): all three paths now match classic, and
the salvage values and army landing with them.

- **Seek Home and Salvage.** `refresh_seek_home_targets` and its helpers are
  gone. `YearMovement::check_home_world` (`maint/movement/home.rs`) runs in
  `Movement::dispatch` every tick, after the step and before the arrival
  test, for both orders. A target world that is missing or not the empire's
  sends the fleet to `nearest_friendly_world` from its stepped position (the
  retarget, stamped with the tick) or, with none, to Hold at speed 0 with
  `TZ = Round(Z)` and the depth untouched. Salvage keeps its order and
  refills a zero speed; its report says it will salvage at the new world
  (owner decision). `SalvageResolvedEvent::Failed` and
  `SalvageFailureReason` are removed, since a Salvage fleet can only arrive
  at its own world. The reports use classic's texts, with `System(x,y)` or
  `Sector(x,y)` in the header by depth (classic: `System` from depth 0.5), so
  the retarget events carry `in_system` and `stardate_week`.
- **Autopilot recall.** `autopilot_fleet_orders` follows classic's
  `24a3:25f7` for autopilot empires, using the new
  `CoreGameData::nearest_productive_world` (flag 2, no production write).
  Rogue empires' fleet and starbase orders are not implemented and stay
  with t070.
- **Salvage values (t069).** 23, 8, 3, 3, 10, 8 per battleship, cruiser,
  destroyer, transport, ETAC, and scout, and loaded transports land their
  armies; the report says so.
- **Probes rerun under DOSBox-X** on the move fixture with empire 1 human
  (scripts were in the session scratchpad). Classic and Rust agree on: a
  Seek Home fleet at (13,13) keeps (16,13) while (12,12) is nearer; the
  deep-space fleet at (13,14), speed 3, seeking empire 2's (18,15) ends over
  (12,12) at depth 0.67036, and 0.75769 when it sought the empty sector
  (13,13) (a fleet starting in orbit at speed 4 gives 0.76393 in both);
  depth 0.69231 held
  with `TZ` 1 and no world left; the Salvage fleet (CA, ETAC) retargeted at
  stardate 1 and salvaged 18 points at (12,12) in tick 44; one of each hull
  plus two loaded transports gave 61 points, 2 armies; the four autopilot
  cases. Rust differs from classic only where the owner ruled: the Salvage
  hold keeps depth 0.692 and 0.135 (classic snaps to 1.0 and 0.0), and the
  retarget report names salvaging. `tools/ncmaint_mission_sweep.py` for
  missions 2 and 15 (speed 3, offsets d3 and s3, four years) matches classic
  year by year, planet bytes included.
- **Other changes.** The nc-fuzz ship-conservation invariant finds a
  salvaged fleet by its number, because its target can change during the
  year. The order traces turn autopilot off for empire 1, since an
  autopilot empire now gives an idle fleet in orbit of its world new orders.
- **Tests.** `nc-data/tests/home_orders.rs` (the discriminating scenarios),
  `nc-data/tests/salvage_value.rs`, the rewritten
  `test_salvage_whose_world_is_not_ours_salvages_at_a_friendly_world` in
  `maint_fleet.rs`, and `seek_home_picks_nearest_world_across_distant_coordinates`
  in `maint_hardening.rs`, rewritten around a lost target.

## t070

Rogue empires got no orders in Rust: t064 left their fleets and starbases
to the player-order code that never ran for them. Classic's orders pass
(`24a3:3cc6`) plays them every year, and the starbase spec wrongly said
nothing in ECMAINT clears a base's escort word.

Research (2026-10-11; ECMAINT disassembly and nine classic probe groups
under DOSBox-X on the move fixture with empire 2 rogue). The rules are in
[ec-movement-spec.md](../../dev/ec-movement-spec.md) ("Rogue And Autopilot
Orders") and [ec-starbase-spec.md](../../dev/ec-starbase-spec.md) ("Escort
Release"). What the disassembly added to t064's reading of `24a3:25f7`:

- A rogue with a saved planet count (PLAYER `0x50`) above 0 runs `25f7`
  then `28fe`; with 0 it runs `24a3:0dbc`, whether or not it owns a world.
- The starbase step at the end of `25f7` tests the order the fleet has by
  then, not the order the pass gave it. Autopilot fleets already on
  Guard/Blockade at rest take an unescorted base, and so do fleets without
  warships. A base counts as at rest when it sits at its target and
  `TZ = Round(Z)`; the moving flag is not read.
- `28fe` measures a rogue base from a scratch record whose in-sector offsets
  are never filled (stack bytes, zero in practice), so a base in deep space
  breaks near-ties differently from its own offsets.
- `0dbc` (no world): armed fleets get Patrol (3), ROE 10, target depth 0,
  maximum speed, and a target drawn as `Random(9 * Trunc(Sqrt(players))) + 1`
  per axis only when the fleet's target is its own sector; unarmed fleets and
  all starbases are deleted; an empire with no fleet is left alone.
- `2895:30f4` (escort release) clears the escort word of every base of the
  empire in the fleet's sector and layer that holds the fleet's number, for
  a fleet on Guard Starbase. Its callers are the orders pass, the planetless
  patrol, every fleet deletion (`2895:3d0c`), the encounter's decision aborts
  and battle retreats (`1b05:185e`, `1f5d`, `25ab`, `3151`), and the planet
  assault wrap-ups (`0b5d:c53a`, `cb7b`, which find nothing to release).

Result (2026-10-11): rogue empires now get classic's orders.

- **Orders pass.** `economics/orders.rs` holds the pass for both kinds of
  empire: `fleet_orders` (`25f7`, shared by rogue and autopilot, with the
  Guard Starbase step now applying to every fleet on Guard/Blockade at rest),
  `starbase_orders` (`28fe`), and `patrol_orders` (`0dbc`). The autopilot code
  moved out of `ai.rs`. Patrol targets come from the campaign seed
  (`RNG_TAG_ORDERS`, year, empire). Fleets the patrol disbands go through
  `apply_fleet_removal_remap`, and `maint/mod.rs` carries the remap and the
  Join host bookkeeping into the turn's references.
- **Shared pieces in nc-data.** `CoreGameData::release_starbase_escort`
  (`30f4`), `nearest_productive_world_for_base` (the flag-2 pick from a base,
  measured from offsets of zero), and `RNG_TAG_ORDERS`.
- **Probes** (classic and Rust agreed on every fleet and base record, apart
  from the patrol's random targets, tax rates, which the economic AI sets,
  the planetless rogue's fate, and ETAC-only Guard/Blockade fleets, which
  Rust's input check had moved first): `core` (Move fleet far out,
  Guard/Blockade at home ROE 3 to 10, ETAC-only Hold in orbit,
  Guard/Blockade over an unowned world with the production-weighted pick), `orbit` (Move and Patrol warships in orbit stop
  on Guard/Blockade ROE 10; deep-space Hold goes home), `bases` and
  `baserest` (unescorted, escorted, about-to-lift, and moving-flag bases;
  Guard Starbase fleet re-ordered and its base released; rogue base picks),
  `basefrac` (the zero-offset tie that tells the scratch record from the
  base's own offsets), `autopilot` (Guard/Blockade fleets at rest take a
  base), `stalecount` and `patrol` (the count gate; ten runs gave patrol
  targets in 1 to 18 on both axes), `nofleets` (an empire with no fleet keeps
  its base).
- **Tests.** New `nc-data/tests/rogue_orders.rs` (18 tests, one per branch
  above: any-order recall, production weighting, ROE 10, warship and
  unarmed orbit cases, base taken, base escorted, base lifting, moving flag,
  escort release in the pass and in `release_starbase_escort`, base picks and
  the zero-offset tie, patrol, seed dependence, redraw, disbanding, no fleet,
  stale count) and one autopilot test in `home_orders.rs`. Ten existing tests
  assumed rogue fleets stay put: the join and rendezvous merge traces now give
  the rogue a saved planet count and expect Seek Home, and nine battle
  tests on `ecmaint-fleet-battle-pre` (whose empire 1 is rogue) make it human.
- **Escort word elsewhere.** Rust clears it only in this pass. The decision
  aborts, battle retreats, fleet removal, and planet-assault wrap-ups are in
  or beside the encounter and combat code under t080 and are untouched.

For the owner:

1. Zero offsets. Classic measures a rogue base's distances from stack bytes,
   which read as offsets of zero while the empire's last fleet sits at a
   sector centre and are arbitrary otherwise (a fleet in flight can give
   enormous distances). A base in orbit of an owned world is unaffected
   (its own world is at distance 0). Rust uses zero always. Keep that, or
   use the base's real offsets?
2. Input check. Rust's sanitizer (`sanitize.rs`) sends a fleet without
   warships on Guard/Blockade home or to Hold before the pass runs, for any
   empire. Classic does not check the orders of rogue or autopilot empires:
   its ETAC-only Guard/Blockade fleets stay and take a base. Exempt the
   empires ECMAINT plays, or keep the check?
3. Escort release. Add the release where Rust re-orders a guard in the
   encounter and combat code, and where a fleet record is removed?
4. Escape episodes. An order the pass gives does not end a fleet's escape
   episode, as a player's order does. Should it?
5. Planetless rogues. Classic keeps a rogue with no world alive, patrolling
   with its warships forever. Rust's campaign rules turn a defeated empire
   into civil disorder at the end of the same turn (`defeat_empire`), so the
   patrol runs for one year.
6. Autopilot, classic fixes made here: the Guard Starbase step applies to
   every fleet on Guard/Blockade at rest, not only Hold fleets the pass
   re-ordered, and a base is at rest by target and `TZ = Round(Z)` with the
   moving flag ignored (t064 required the flag clear and ignored `TZ`).
   The player manual's autopilot paragraph says so; the PDF is not rebuilt.
