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

### t011

nc-data's `campaign_contenders` and `empire_campaign_state` in
`directory/economy.rs` still count an empty rogue empire as a contender, so
display and outlook code can disagree with the engine. Reuse the engine's
contesting-empire test.

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

### t027

`nc-game/tests/basic.rs::piped_dropfile_launch_stays_interactive_without_tty_stdout`
relies on fixed 150-550 ms sleeps and failed several times while the machine's
load average was 65-70 from parallel builds; it passes when run alone. Replace
the sleeps with waits on observable progress, such as reading stdout until the
expected prompt appears, with a generous timeout.

### t028

Found in t018. When a DOOR.SYS handle binds no seat, binding retries with the
real name from line 10. A caller whose real name equals another player's
stored handle would therefore bind to that player's seat. Binding before t018
had the same exposure, because it always used the real name. Decide whether
to restrict the retry, for example to seats first bound from a DOOR.SYS real
name, which would need the seat to record how it was bound.

## Connections

The RESULTS.DAT chain-id question is its own
[investigation](../investigations/e752ccc3-classic-results-dat-chain-ids-across-several-players.md),
tracked as t008. The fuzzer has its own
[plan](lski78er-engine-fuzzer-with-shared-policies-and-invariants.md).
