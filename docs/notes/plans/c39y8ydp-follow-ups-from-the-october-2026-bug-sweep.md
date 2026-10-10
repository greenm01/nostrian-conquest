---
id: c39y8ydp
date: 2026-10-09
kind: plan
tags: [plan]
---
# Follow-ups from the October 2026 bug sweep

## Scope and exit

Work found but not done during the
[October 2026 bug sweep](../milestones/ym5u2c1x-engine-and-door-bug-sweep-of-october-2026.md).
Each task below exits with a failing-first regression test and green
`cargo test` from `rust/`, unless it says otherwise. Task state and order live
in [todo.md](../../../todo.md).

## Task details

### t006

Confirm the holding-formation rule from the
[engagement decision](../decisions/mnb1phgg-engagement-follows-the-1992-player-guide.md).
A lone starbase, and guards or owner fleets whose ROE declines a fight, no
longer attack enemies merely passing through the sector. Decide whether a
starbase should engage passing enemies on its own; if so, record the rule in
the manual and add a test. Exit: a recorded decision.

Result (2026-10-09): confirmed as built, on the 1992 guide's starbase and guard
text. Recorded in the engagement decision; no code change.

### t007

Blitz landing fire still costs one transport per surviving battery, on the
old scale; move it to the 10x scale used by the
[hit-cost decision](../decisions/lzvpk22s-planetary-hits-pay-defense-strength.md).
Ground combat (`resolve_ground_battle_to_destruction` in `assault.rs`) still
returns an error after 64 rounds; end it like the fleet battle cap instead.

Result (2026-10-09): landing fire is now one CRT exchange of surviving battery
AS against the escorts' bombardment AS, applied as focus fire on transports
first at 20 hits each. Ground combat at the cap goes to the larger remaining
force, ties to the defender. Regression tests: `landing_fire_pays_transport_
defense_strength_on_the_ten_times_scale` and the inline
`ground_battle_cap_never_ends_in_a_draw`. Two blitz tests in
`nc-data/tests/maint_combat.rs` were updated to the new landing losses.
Workspace: 2625 passed, 0 failed.

### t009

`build_initialized_baseline` and `build_seeded_initialized_game` start every
homeworld treasury at 0, so pre-joined test and lab games cannot build on turn
1 under the new budget cap. Real games are unaffected: joining sets the
first-turn treasury. Seed the baselines the same way the join path does.

### t010

`maint/economics/ai.rs` near line 148 still squares sector distances in i16,
which overflows past a distance of about 127. Compute in i32, as the
targeting and retarget code now does.

Result (2026-10-09): the targeting code's widened `sector_distance_sq` moved to
`navigation/geometry.rs` as the one shared helper; autopilot recall and the
retarget fallback both use it. Regression test:
`autopilot_recall_picks_the_nearest_world_across_the_whole_map` in
`nc-data/tests/autopilot_economy.rs`. Workspace with t011: 2626 passed,
1 failed; the failure, `piped_dropfile_launch_stays_interactive_without_tty_stdout`,
is a timing test that failed once under full-suite load and passes alone.

### t011

nc-data's `campaign_contenders` and `empire_campaign_state` in
`directory/economy.rs` still count an empty rogue empire as a contender, so
display and outlook code can disagree with the engine. Reuse the engine's
contesting-empire test.

Result (2026-10-09): the engine's contender rule moved to nc-data as
`is_campaign_contender` (with `is_contesting_empire`) in `player_lifecycle.rs`.
The engine's sole-contender check and nc-data's `campaign_contenders` both call
it; nc-data passes no lifecycle states, which count as a fresh empire's.
`empire_campaign_state` still labels the empire `Rogue` for display. As a side
effect, nc-data now also follows the engine on ETACs: an empire whose only
recovery force is ETACs stops contending once no unowned world is left.
Regression test: `empty_rogue_empire_is_not_a_campaign_contender` in
`nc-data/tests/campaign.rs`. Workspace with t010: 2626 passed, 1 failed (the
timing test noted under t010).

### t012

Fleet ETA counts the year a fleet becomes visible in the target sector, while
the order completes at the exact path endpoint, so diagonal moves can differ
by one year. The current regression covers axis-aligned moves only.

### t013

Salvage-success reports now say "From your fleet" without a number, because
the salvaged fleet is gone by report time; carry the fleet number in the
salvage event. `rendezvous_merged_fleet_indices` in `entries/missions.rs`
should use only survivor-side merge events.

### t014

Empire names, handles, and `set_status_or_name_summary_raw` are not validated
at entry; the RESULTS boundary now shows them as `?`. Use the shared
`is_classic_text_char` check at entry, and in nc-game's input filter so the
rule lives in one place. Replace the em dashes in nc-helm's
`dashboard/app/planet_build.rs` and in the debug message in
`maint/movement/salvage.rs`.

### t018

DOOR.SYS reads the alias from line 10, the real name. The 52-line standard
puts the handle on line 36, and every other dropfile format we parse uses the
handle. Read line 36 when present and non-empty, else line 10. None of the
documented BBS setups use DOOR.SYS, so few seats should be orphaned; binding
could try both lines. Needs a decision before implementation.

### t019

On a Windows console, `stdin_ready` waits on `WaitForSingleObject`, which wakes
on key-up, mouse, focus, and resize events; `ReadFile` then blocks. That
defeats the Esc timeout and the new session deadlines. Loop with
`PeekConsoleInputW` and `ReadConsoleInputW`, dropping non-key-down records.
Affects only `nc-game --dropfile` from a Windows console; needs a Windows
host to verify.

### t020

Door edge cases: an unterminated telnet `IAC SB` swallows input with no length
cap; a carrier drop during a render still exits with code 1; the session clock
starts at door launch, with no warning before time runs out.

### t021

The engagement rule detects fleets entering a blockaded world but never models
fleets leaving one, which the 1992 guide also names.

### t022

The campaign-outlook broadcast says "none currently appear capable of
challenging our claim", written from the contender's point of view, but goes
to every empire. The scouting report keeps two-space-indented "Owned by" rows
where nc-reports.md shows aligned labels.

## Connections

The RESULTS.DAT chain-id question is its own
[investigation](../investigations/e752ccc3-classic-results-dat-chain-ids-across-several-players.md),
tracked as t008. The fuzzer has its own
[plan](lski78er-engine-fuzzer-with-shared-policies-and-invariants.md).
