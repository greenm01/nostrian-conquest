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

Result (2026-10-09): the join path's treasury step is now
`CoreGameData::seed_first_turn_treasury`, one year's tax revenue from the
homeworld. `join_player` and `build_initialized_baseline` both call it, so
`build_seeded_initialized_game` inherits it; a pre-joined homeworld starts
with 50 points, as a joined one does. Regression test:
`pre_joined_baselines_start_with_the_join_path_treasury` in
`nc-data/tests/builder.rs`. No existing test relied on the zero treasury, so
none changed. Workspace: 2631 passed, 1 failed (the timing test noted under
t010, which passes alone; load average was about 50 during the run).

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

Result (2026-10-09): the fleet ETA now replays the stepper exactly. It counts
years until the exact path endpoint, stores the exact position at its 1/256
precision between years (new `round_trip_exact_position` in
`fleet_motion_state.rs`), and treats a fleet that shows in the target sector
with an unfinished exact path as still under way, not arrived. The stepper and
the classic fraction byte are unchanged, and the starbase travel-time display
still stops at the first rounded target sector. The stepper's
`exact_position_reached_target` moved to `navigation/geometry.rs` so both use
it. `fleet_eta_matches_actual_arrival_turn_for_every_speed` now covers five
diagonal offsets and checks the estimate before every year, not just the first.
A sweep of every offset up to (17,17) at every speed, run while developing,
found no mismatch. Two expectations in `move_only_traces_match_current_classic_oracle_cases`
(`nc-data/tests/movement_trace.rs`) changed: speed-3 (10,10) to (16,16) now
predicts 4 years, not 3, and speed-1 (10,10) to (13,13) predicts 6, not 5. In
both, the fleet shows in the target sector a year before its order completes.
The traced positions are unchanged. The ETA paragraph in
[ec-movement-spec.md](../../dev/ec-movement-spec.md) now says this.
Workspace: 2628 passed, 1 failed (the timing test noted under t010).

### t013

Salvage-success reports now say "From your fleet" without a number, because
the salvaged fleet is gone by report time; carry the fleet number in the
salvage event. `rendezvous_merged_fleet_indices` in `entries/missions.rs`
should use only survivor-side merge events.

Result (2026-10-09): `SalvageResolvedEvent::Succeeded` now carries
`fleet_number`, read before the fleet is removed, and the salvage report uses
it. The rendezvous arrival filter keeps only survivor-side merge events, whose
indices follow compaction; an absorbed fleet's stale index no longer hides
another fleet's "waiting" arrival report. Regression tests in
`nc-engine/tests/mission_report_fleet_numbers.rs`:
`salvage_success_report_names_the_salvaged_fleet` and
`absorbed_side_merge_index_does_not_hide_another_fleets_rendezvous_arrival`.
Workspace: 2628 passed, 1 failed (the timing test noted under t010, which
passes alone).

Reversed by t031 (2026-10-10): the classic delay this matched was a probe
artifact, and classic completes a move on entering the target sector; see
[the investigation](../investigations/9msqq5e0-classic-move-orders-complete-on-entering-the-target-sector.md).

### t014

Empire names, handles, and `set_status_or_name_summary_raw` are not validated
at entry; the RESULTS boundary now shows them as `?`. Use the shared
`is_classic_text_char` check at entry, and in nc-game's input filter so the
rule lives in one place. Replace the em dashes in nc-helm's
`dashboard/app/planet_build.rs` and in the debug message in
`maint/movement/salvage.rs`.

Result (2026-10-09): `join_player` now rejects an empire name with any
character outside `is_classic_text_char`, with the new
`GameStateMutationError::InvalidEmpireName`, as renames already do for planet
names. The raw record setters that other entry points use
(`set_controlled_empire_name_raw`, `set_assigned_player_handle_raw`,
`set_status_or_name_summary_raw`) store `?` for any other character through
the new `nc_data::support::classic_text`, so BBS handles and sysop or harness
names never reach a record as raw UTF-8 or get cut mid-character. nc-game's
name input filter calls `is_classic_text_char` instead of its own copy of the
rule. The em dashes in nc-helm's two "Stardock is full" strings, the salvage
debug message, and the contact-report quote in
[approach.md](../../dev/approach.md) are now `--`. Regression tests in
`nc-data/tests/mutators.rs`: `join_player_rejects_non_printable_ascii_empire_names`
and `classic_text_setters_store_question_marks_for_other_characters`. The
string changes have no test. Workspace: 2631 passed, 0 failed.

Decision (2026-10-10): the user confirmed the split. Typed empire names are
rejected so the player can retype; BBS handles and other names with no one to
ask store `?`.

### t018

DOOR.SYS reads the alias from line 10, the real name. The 52-line standard
puts the handle on line 36, and every other dropfile format we parse uses the
handle. Read line 36 when present and non-empty, else line 10. None of the
documented BBS setups use DOOR.SYS, so few seats should be orphaned; binding
could try both lines. Needs a decision before implementation.

Decision (2026-10-09): approved. Read line 36 when present and non-empty, else
line 10; seat binding tries the handle first, then the real name, so existing
seats keep working.

Result (2026-10-09): DOOR.SYS now reads the handle from line 36, and the
real name from line 10 when line 36 is missing or blank. When the handle
binds no seat, binding retries with the real name (in nc-game's
`resolve_launch_player_binding`), so seats reserved or joined under the real
name keep working; the alias that bound becomes the caller alias. A new
caller joins under the handle. Tests: three DOOR.SYS parser cases in
`nc-game/tests/dropfile.rs` (52 lines with a handle, blank line 36, short
file) and four binding cases in `cli.rs` (stored and reserved real-name
seats, handle first, new caller). The sysop manual's drop file section says
so.

### t019

On a Windows console, `stdin_ready` waits on `WaitForSingleObject`, which wakes
on key-up, mouse, focus, and resize events; `ReadFile` then blocks. That
defeats the Esc timeout and the new session deadlines. Loop with
`PeekConsoleInputW` and `ReadConsoleInputW`, dropping non-key-down records.
Affects only `nc-game --dropfile` from a Windows console; needs a Windows
host to verify.

Decision (2026-10-09): approved. The user verifies on real Windows after a
`git pull`, building natively; the VM approach was dropped.

Result (2026-10-09): the console branch of `stdin_ready` now peeks the input
queue with `PeekConsoleInputW`, discards records that are not key presses
carrying a character with `ReadConsoleInputW`, and reports ready only when a
usable key is first; it waits on the handle only while the queue is empty.
Pipe and socket waits are unchanged. Unit tests cover the record filter and
the `INPUT_RECORD` layout; the console calls cannot run on Linux. The
`x86_64-pc-windows-gnu` release build of `nc-door` and `nc-game` is clean.
Windows runtime verification is pending the user's check below; there is no
VM kit.

Windows check, in `cmd.exe` from `rust\` on a checkout of `main`. Build,
make a test campaign, write a local DOOR32.SYS with a 3-minute limit (line 9,
so the t020 warning shows too; `2` gives a plain 2-minute limit), and run:

```bat
cargo build --release -p nc-game --bin nc-door --bin nc-game
cargo build --release -p nc-sysop
mkdir %TEMP%\nc-t019
target\release\nc-sysop.exe new-game %TEMP%\nc-t019\t019 --players 4 --seed 1
(echo 0&echo 0&echo 38400&echo Local&echo 1&echo Test Caller&echo Tester&echo 100&echo 3&echo 1&echo 80&echo 25) > %TEMP%\nc-t019\DOOR32.SYS
target\release\nc-game.exe --dir %TEMP%\nc-t019\t019 --dropfile %TEMP%\nc-t019\DOOR32.SYS
```

Use `nc-game.exe`: `nc-door.exe` is built for the windows subsystem and has
no console. If raw `←[0;37;40m` codes show, the console lacks VT output;
Windows Terminal renders them, and the input check works either way.

1. On the first-time menu press `Q` for the quit prompt, then `Esc` alone.
   The prompt must close at once (within about 100 ms). Before the fix it
   waited for another key.
2. Press `Q`, then a few arrow keys, then `Esc`. `Esc` must still act at
   once. Arrows themselves do nothing in a plain console: raw mode does not
   enable VT input, so they carry no characters.
3. Press `Q`, then resize the window, Alt+Tab away and back, and click in
   it. Turn off QuickEdit Mode in the window's Properties first, or a click
   starts a text selection instead of reaching the program. `Esc` must still
   act at once.
4. Leave the session idle after the clicks and focus changes. About one
   minute after launch the line "Two minutes remain in your time limit for
   this session." appears above the command line. At three minutes "Your
   time limit for this session has expired. Returning to the BBS." shows and
   the program exits; `echo %ERRORLEVEL%` prints `0`. Before the fix, a
   mouse, focus, or resize event left the session stuck past its limit.

Windows result (2026-10-10, the user on bean, PowerShell 7.6, Windows
console): `Esc` closes the quit prompt at once, also after arrow keys and
window events. The check found three more console bugs, fixed and verified
there: the help screen crashed because Rust's console `Stdout` refuses
non-UTF-8 bytes (`466344c7`, raw `WriteFile`); arrow keys never reached the
door (`4bd50a69`, VT input mode); and every frame cleared the screen first,
which flashed (`739158c9`, rows redrawn in place). The session limit works
too: the two-minute warning appeared on time, and at three minutes the
session printed "Your time limit for this session has expired. Returning to
the BBS." and returned to the prompt.

### t020

Door edge cases: an unterminated telnet `IAC SB` swallows input with no length
cap; a carrier drop during a render still exits with code 1; the session clock
starts at door launch, with no warning before time runs out.

Decision (2026-10-09): approved, all three. The time warning fires 2 minutes
before the session limit.

Result (2026-10-09): all three are done. A subnegotiation with no `IAC SE`
within 1024 payload bytes is dropped, and the bytes after it decode as data
again. A render whose write fails with broken pipe, connection reset, or pty
EIO ends the session as a hangup with exit code 0; `is_closed_input_error`
became `is_hangup_error` since it now covers writes too. A session limit
longer than two minutes draws "Two minutes remain in your time limit for
this session." once over the last body row, two minutes before the limit,
saving and restoring the cursor; the next frame redraws the row. The clock
still starts at door launch, which the sysop manual now states with the
warning. Tests: `socket_input_resumes_after_an_unterminated_subnegotiation`,
`socket_input_drops_a_subnegotiation_at_the_length_cap`,
`carrier_drop_during_a_render_exits_cleanly`, and three session-warning
tests in `nc-game/tests/door_terminal.rs` that scale the two-minute lead
down. Workspace: 2640 passed, 0 failed.

### t021

The engagement rule detects fleets entering a blockaded world but never models
fleets leaving one, which the 1992 guide also names.

Decision (2026-10-09): approved. Count both a fleet ordered out of a
blockaded world's orbit and newly built ships launching from its stardock.

Result (2026-10-09): a departure interception now runs before movement. A
fleet that starts the turn at rest at a world with a Guard/Blockade World
fleet on station, and whose order moves it elsewhere, fights the blockading
fleets of every other empire, at war or not. Commissioning happens between
maintenance runs, so a launch is an ordinary fleet at the world by then and
the same rule covers it. Ships built during maintenance stay in stardock
until commissioned. The battle reuses the fleet battle driver with only the
departing and blockading forces. Starbases and other fleets at the world do
not join, survivors move on, and diplomacy escalates as for any fight. Tests
are in `nc-engine/tests/combat/departure.rs`. A fleet holding in the
sector's deep space also counts as departing when it leaves, because fleet
records keep no earlier mission context.

### t022

The campaign-outlook broadcast says "none currently appear capable of
challenging our claim", written from the contender's point of view, but goes
to every empire. The scouting report keeps two-space-indented "Owned by" rows
where nc-reports.md shows aligned labels.

Decision (2026-10-09): approved. The leader sees "our claim"; other empires
see the leader named. Align scouting report rows with nc-reports.md.

Result (2026-10-09): the outlook is now one targeted report per joined
empire. The contender reads "We now stand ... our claim", and the others read
"<empire> now stands ... No other empire, ours included, currently appears
capable of challenging its claim." It was the last broadcast report, so the
unused broadcast routing was removed. Scout Solar System fact rows go through
the shared labeled-row helper, and the example in
[nc-reports.md](../../dev/nc-reports.md) shows the real output.

### t026

Found during t008. In `nc-data/src/storage/report_blocks.rs`, when one empire
deletes its reports, broadcast report rows are deleted for every empire, and
deleting one report by `block_index` also deletes the broadcast row with the
same index. Deletion must only remove the deleting empire's view of a
broadcast report.

Update (2026-10-09): since t022 the engine emits no broadcast rows; the
campaign outlook is now one report per empire. The bug still affects
broadcast rows already stored in existing campaign databases, which the
t008 layout continues to export into every empire's chain.

Result (2026-10-09): every delete path writes the whole in-memory row set back
as a new snapshot, so the SQL `mark_*` updates in `storage/report_blocks.rs`
were overwritten and the real fault was in memory: the bulk delete, the
startup-review delete, the `nc-game` inbox delete, and the `nc-helm` inbox
delete all set `recipient_deleted` on the shared broadcast row. All four now
call `nc_data::delete_report_block_for_viewer` or
`delete_report_blocks_for_viewer`. Deleting a broadcast row for one empire
marks it deleted and adds a copy for each other active human empire, the set
`layout_classic_results` already sends broadcasts to, under the next free
block index. The copy therefore moves after that empire's own reports in its
chain. No schema change: the copies are ordinary per-empire rows, so existing
`ncgame.db` files load unchanged. The unused SQL `mark_*` functions were
removed. Tests: `nc-game/tests/update/broadcast_report_delete.rs` covers bulk,
inbox, and startup-review deletes with two joined empires and the export
layout, and failed on all three before the fix;
`nc-data/tests/report_block_delete.rs` covers the shared helper, including a
targeted row with the broadcast row's block index and a repeat delete. An
empire that joins after the delete does not get the copy.

### t027

`nc-game/tests/basic.rs::piped_dropfile_launch_stays_interactive_without_tty_stdout`
relies on fixed 150-550 ms sleeps and failed several times while the machine's
load average was 65-70 from parallel builds; it passes when run alone. Replace
the sleeps with waits on observable progress, such as reading stdout until the
expected prompt appears, with a generous timeout.

Result (2026-10-09): with 128 busy loops competing for 32 cores, the old test
failed 10 of 10 runs: the `j` keystroke arrived before the first-time menu was
read, so "EMPIRE NAME" never appeared. The test now collects the child's stdout
on a reader thread and sends each key only after the screen it answers is
drawn: `j` after "FIRST TIME COMMAND", Esc after "EMPIRE NAME", and `q` after
the reserved-seat prompt "Continue with reserved setup?". It then closes stdin,
which ends the session, as the old writer thread did. One 30 s deadline covers
the waits and the exit; a missed prompt kills the child and fails with the
stdout seen so far. Under the same load the new test passed 20 of 20 runs, and
its assertions are unchanged. The `collect_stdout` and `wait_for_stdout`
helpers live in `nc-game/tests/basic.rs`. One other test there still sleeps
before a key: `nc_door_rejects_schema_mismatch_with_dismissible_error_screen`
(150 ms, then a 5 s exit timeout). It does not use the helpers, so it was left
as it is; it passed 10 of 10 runs under the same load. The Windows-only
`windows_nonblocking_socket_door_terminal_handles_would_block` sleeps on
purpose to provoke a would-block read.

### t028

Found in t018. When a DOOR.SYS handle binds no seat, binding retries with the
real name from line 10. A caller whose real name equals another player's
stored handle would therefore bind to that player's seat. Binding before t018
had the same exposure, because it always used the real name. Decide whether
to restrict the retry, for example to seats first bound from a DOOR.SYS real
name, which would need the seat to record how it was bound.

Decision (2026-10-10): option (b). Each seat records whether it was bound by
handle or by real name, and the real-name retry matches only real-name
seats. A seat with no record counts as bound by real name. A real-name seat
that matches is rebound to the caller's handle, so every seat moves to
handle binding at its player's next login.

Result (2026-10-10): ncgame.db has a new `seat_bind_sources` table
(`handle` or `real_name` per seat), created with `CREATE TABLE IF NOT EXISTS`
like the other per-seat runtime tables, so beta.3 databases open without a
schema version bump and every existing seat counts as bound by real name;
PLAYER.DAT is untouched. nc-data's `SeatBindSource` and
`CampaignStore::handle_bound_seats` / `set_seat_bind_source` own it. The
shared resolver in nc-session takes `skip_stored_handle_seats`, and nc-game's
`resolve_launch_player_binding` passes the handle-bound seats on the
real-name retry only. When a caller with a handle binds through a stored
handle, `bind_seat_to_caller_handle` stores the handle on the seat if it
differs (normal access only) and records the seat as handle-bound, so seats
converge to handle binding at their player's next login. A join records the
seat by the caller alias's source; a DOOR.SYS with no line 36 handle
(`alias_is_real_name`) records real name, so a later 52-line DOOR.SYS can
still rebind it. Tests: `door_sys_real_name_does_not_bind_a_seat_bound_by_handle`
and `door_sys_seat_bound_under_the_real_name_is_rebound_to_the_handle` in
nc-game's `cli.rs`, the two `unbound_bbs_join_*` cases in
`nc-game/tests/update/startup_join.rs`,
`resolve_launch_binding_skips_listed_stored_handle_seats` in
`nc-session/tests/launch.rs`, and
`sqlite_store_seat_bind_sources_default_to_real_name_on_existing_databases`
in `nc-data/tests/storage.rs`. The sysop manual's drop file section says so.

### t030

Handles are stored in classic form (`?` for characters outside classic text,
cut to 25 characters), but seat lookup and the reservation check compared
the stored handle with the caller's full DOOR.SYS handle. A caller whose
handle had an accented letter or more than 25 characters found no seat and
landed in the join flow again.

Decision (2026-10-10): approved.

Result (2026-10-10): `PlayerRecord::assigned_player_handle_matches` compares
an alias in its stored form, and `nc-session/src/launch.rs` uses it for seat
lookup and the reservation check.
`resolve_launch_binding_finds_handles_stored_in_classic_form` in
`nc-session/tests/launch.rs` covers both cases and fails without the fix.

### t032

Found in t028. When DOOR.SYS has no handle on line 36, the caller's real name
is the primary alias and is still matched against every seat, so a caller
whose real name equals another player's stored handle binds to that seat,
the exposure t028 closed for callers with a handle. Skipping handle-bound
seats there instead would let the caller join under a handle another seat
already holds and lock that player out with the "matches multiple joined
empires" error. Decide whether to refuse such a join, ask the sysop to
reserve the caller, or accept the exposure for BBSes that omit line 36.

Decision (2026-10-10): refuse the join with a clear message and tell the
sysop. The case is rare and the sysop can reserve the seat by hand.

## Connections

The RESULTS.DAT chain-id question is its own
[investigation](../investigations/e752ccc3-classic-results-dat-chain-ids-across-several-players.md),
tracked as t008. The fuzzer has its own
[plan](lski78er-engine-fuzzer-with-shared-policies-and-invariants.md).

### t061

`nc-host maint` (`nc-host/src/commands/maint.rs`), the hosted Nostr path's
maintenance command, lists each turn's pending submissions but never
applies them, discards the events `run_maintenance_turn` returns, and
saves the snapshot with empty report blocks, queued mail, and scorch
orders. The hosted scheduler computes due times, but nothing runs
maintenance on them. The shipped BBS and localhost paths (`nc-sysop maint`)
are not affected. Finish the hosted maintenance path (apply submissions,
compose reports, keep mail and scorch orders) or gate the command until it
is finished. Found by the t039 phase 4 agent on 2026-10-10.
The t067 follow-up confirmed the mail side: hosted maintenance saves queued
mail as empty and `ProcessTurnSubmission` is a stub, so no mail reaches a
hosted game, and maintenance would wipe it if it did.

Result (t061, 2026-10-11): the load, run-turn and save sequence
`nc-sysop maint` ran inline is now `nc_engine::CampaignMaintenance`
(`nc-engine/src/maint/campaign_run.rs`), and `nc-sysop maint`, `nc-host
maint` and the hosted scheduler all run it. Hosted intake validates a
`30522` submission against the live snapshot (seat, turn, typed KDL, a dry
run of the orders) and answers `accepted`, `rejected`, `wrong_turn`,
`not_claimed` or `superseded`; maintenance applies the staged submissions
through `TurnSubmission::apply_to_runtime_state`, the path `nc-game
submit-turn` uses, records the game clock, and sends every claimed seat a
`30520` snapshot or a `30521` delta. `nc-host serve` polls the schedule
once a minute; an unscheduled game is armed one interval out, and a run
moves the next due time one interval past now. Tests:
`nc-host/tests/hosted_maint.rs` (the hosted and local paths produce the
same snapshot, reports and mail for the same orders), `turn_intake.rs` and
`maint_schedule.rs`. The shared `MaintenanceEvents::extend` also carries the
starbase, Scorch-Earth and ship-recovery events that the hand-copied merge
in `nc-sysop` dropped, so those reports now reach local players too.

### t067

Reported by the user on 2026-10-10: after sending a message there seemed to
be no way to review it or cancel it before maintenance. The outbox exists
(Messages, then `D` on the recipient list) but is hard to find, shows only
number, recipient, and subject, and deletes the highlighted message on
`Enter` without asking, although its help calls `Enter` "accept highlighted
item" and its footer shows only `? <Q>`.

Decision (2026-10-10, owner): within the existing screens and table layout,
`Enter` opens the highlighted queued message to read; `D` deletes it after a
"Delete? [Y]/N" prompt that defaults to yes, as the inbox does; the outbox
footer shows `D`; and the send confirmation says that `D` on the recipient
list reviews or cancels queued messages.

The owner then clarified the real problem: after sending, Review Inbox said
"Notice: Inbox is empty.", so the message seemed gone. Added to t067: Review
Inbox lists the player's own messages queued this turn, first and marked
outgoing, with the recipient in the sender's place; `Enter` reads one and
`D` cancels it with the inbox's default-yes prompt. They leave the list when
maintenance delivers them.

Result (2026-10-10): merged. The outbox reads on `Enter` and deletes on `D`
after the inbox's default-yes prompt; Review Inbox lists the player's queued
messages first, marked `OUT`, until maintenance. The agent also fixed an
outbox crash (a 60-character subject pushed the table past 80 columns) and
stale inbox numbers after a cancel. Owner decisions on its findings: the
outbox shows only this turn's queued messages; a recipient sees a message
only after the maintenance that delivers it (shared local and BBS games
delivered at once before); the outbox read view scrolls like the inbox
preview. Those three land as a follow-up commit under t067.

Follow-up (2026-10-10): the outbox lists only this turn's queued messages;
mail reaches its recipient only after the maintenance that delivers it (its
year is before the current one) in the inbox, login review, counts, bulk
delete, nc-helm, and the hosted payload; the outbox read view scrolls with
the inbox preview's keys.

### t076

Found by the t067 follow-up (2026-10-10): a player session left open
while maintenance runs keeps saving to the year it loaded. The store keeps
one snapshot per game year and loads the highest, so anything the session
saves afterwards (mail, orders) lands in the old year's snapshot and is
silently lost. Detect a newer snapshot before saving, refuse the stale
write, and tell the player that maintenance has run and the session must
reload.

Result (2026-10-10): every `save_runtime_state_*` now takes `loaded_year`,
the year the caller loaded. Inside the save transaction, before any write, the
store compares it with the highest stored year and returns
`CampaignStoreError::StaleSnapshot { loaded_year, latest_year }` when a newer
snapshot exists, leaving the store untouched. Maintenance (`nc-cli maint`,
`nc-sysop maint`, `nc-host maint`) passes the year it loaded and writes the
next one, so it still saves; a second maintenance run from the same year is
now refused too. Code that creates state from nothing (init, import, harness
builders) passes the year it writes. An import that is older than the stored
year is refused as well; `nc-cli scenario-init-replayable` did exactly that
(it imported the year-3000 replay context under the year-3001 init files,
where nothing read it) and no longer refreshes the store after copying the
context.

`nc-game` marks the session stale on the first refusal and never saves again.
After the action that tried to save, it shows "Maintenance has run; your last
change was not saved. Log in again." as the main-menu notice, so each
screen's own "try again" text cannot hide it. In a BBS door the loop then
ends through the time-limit exit path (`SessionEnd::Stale`), printing the
same line; a local session stays open. `nc-helm` shows the message in the
status line of whichever prompt saved, since every site already prints the
error's text, and each later save is refused the same way.
`TurnSubmission::submit_to_runtime_state` splits the apply-and-save step out of
`submit-turn` so the stale case can be tested; the CLI fails with the same
message. The line is shorter than the example in the first note so it fits one
80-column notice row.

Result for t089 (2026-10-11): `nc-cli db-import` matches `CampaignStoreError::StaleSnapshot` and refuses with its own operator message, naming the store's year and the directory's year and saying to import into a fresh store or export the store first; the player-worded text is unchanged.

### t077

`nc-helm`'s "current year only" inbox filter keeps items whose year equals
the current year, but delivered mail and maintenance reports carry an
earlier year (`report_year = year - 1`), so the filter hides everything.
Make it show the latest delivered year. The filter is off by default.
Found by the t067 follow-up.

Result (2026-10-11): `nc-helm`'s "current year only" inbox filter now keeps delivered mail and reports of the year before the current one (the latest delivered year), and an outgoing row (the sender's own queued message, not yet delivered) of the current year.

Result for t092 (2026-10-11): `nc-helm`'s inbox lists the player's own queued messages first, newest first, as `OUT` rows (from `runtime_outgoing_inbox_items`) under the all and messages filters; the preview shows recipient, subject and body, `D` cancels one after the inbox's confirmation through the helper the outbox also uses (local queue and hosted draft), and the comms panel counts reports of the latest delivered year.
