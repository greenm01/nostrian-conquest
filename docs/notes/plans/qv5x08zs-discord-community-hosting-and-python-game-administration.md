---
id: qv5x08zs
date: 2026-10-11
kind: plan
tags: [plan, milestone]
---
# Discord community hosting and Python game administration

## Scope and exit

The owner approved Discord discovery, browser BBS play, multiple campaigns,
and Python administration over SSH. No dashboard or embedded Activity is
required. Preserve existing BBS/Nostr services until a private pilot passes.
Python coordinates supported Rust operations; Rust owns state and authorization.
Personal deployment details and secrets remain outside public notes.

Use the owner's existing Nostrian Conquest Discord community:
<https://discord.gg/eZMRpdQTu>. The invitation is a community entry point,
not bot authorization or a browser authentication credential.

### t117

Prevent concurrent door saves from overwriting other players in the same year.
Use monotonically increasing snapshot revisions and transactional optimistic
checks. A stale action must fail clearly and reload safely. Test two players,
maintenance races and repeated same-year saves.

### t118

Add transport-independent campaign operations to nc-host and the standard-library
Python nc-admin client. Lifecycle: draft, recruiting, running, paused, archived.
Expose JSON, previews, expected-year maintenance, request IDs, history, backup,
restore and independent schedules. Games start unscheduled. Persist completed
maintenance with state; announcement failures cannot repeat turns.

### t119

Authenticate browser users with Discord, bind verified IDs to game seats, serve
a game selector and the existing 80x25 terminal. Enforce campaign/seat isolation,
one terminal per seat, child cleanup and exact authorized private exports.
Do not trust identity query parameters. Preserve ordinary BBS launch semantics.

### t120

Provide Discord game discovery, join/play links, real status and successful-turn
announcements across games. Recruiting seats are self-service; replacements
require an operator. Use game/account authorization for every private download.

### t204

Let players of one campaign play at the same time without refusing each
other's saves.

Every session change saves the whole runtime state as a new snapshot,
expecting the snapshot it loaded (`expecting_snapshot`,
`CampaignStoreError::ConcurrentSnapshot`). When another player saved in
between, the save is refused and the session reloads; the player redoes the
action ("Game updated; your last change was not saved. Please try again.").
Nothing saved is lost, but two active players keep costing each other
redos, although each changes only their own empire. Classic EC ran one
player at a time; a hosted game must not (owner, 2026-10-17). A playtester
reported the refusal on Discord and fell back to one player at a time.

Approach (owner, 2026-10-17): save only the player's own records. A session
save writes only what its player may change (their empire, worlds, fleets, stardocks, diplomacy, outgoing
mail and preferences), applied onto the latest snapshot inside the same
`BEGIN IMMEDIATE` transaction, instead of the whole state. List every
session action that touches another player's data before choosing; any
that does keeps the refusal. Maintenance's stale-year refusal is unchanged.

Records are stored by array position, and new fleets take the lowest free
global fleet ID, so copying one player's rows over the latest snapshot
would make two concurrent commissions collide. The design (owner,
2026-10-17: best practice, phase 1 then phase 2 straight after):

1. **Phase 1, transactional changes.** WAL journal and a busy timeout on
   every connection. `CampaignStore::change` opens one `BEGIN IMMEDIATE`
   transaction, loads the latest state inside it, applies the player's
   change through the existing `nc-data` mutators, writes and commits;
   the session then takes the committed state. Every session save in
   `nc-game`, every local-store save in `nc-helm` (which saves today with
   no snapshot check, last writer wins) and `submit-turn` (the same) go
   through it. Mutators that rely on the UI for ownership (build queue,
   scorch, fleet orders, homeworld rename) check it themselves. First-time
   join chooses its seat inside the transaction.
2. **Phase 2, row-level writes (scope corrected with the owner,
   2026-10-17).** The store keeps one snapshot per year, so the latest
   year's rows are already the live state and older years the history;
   only the rewrite of the whole current year on every change is waste. A
   change writes only the records it altered, found by comparing the
   records before and after the mutator inside the transaction, into the
   current year's rows. Phase 1 serialises every change on the latest
   state, so positional record keys cannot collide within a year and no
   engine ID refactor is needed; that refactor would also put classic
   compliance at risk. A revision counter in `campaign_metadata` replaces
   the snapshot ID as the "campaign changed" signal (maintenance's stale
   check reads it). No schema change: existing campaigns keep loading.

Results (phase 1, 2026-10-17): every player change in `nc-game`,
`nc-helm` (local store), `submit-turn` and `nc-host`'s seat operations
runs through `CampaignStore::change`; WAL journal; saves wait up to 30 s
for the write lock, since SQLite's wait is not fair. A change costs 1.4 ms
on a 4-player campaign and 9.3 ms on a 25-player one (release build). Not
done: ownership checks inside the four mutators that rely on the UI (build
queue, scorch, fleet order, homeworld rename), since ownership changes only
in maintenance; the whole-state save methods stay public for maintenance,
import and game creation. Found on the way: a seeded game leaves
`fleets_first` at 0, so its starting fleets are on no chain.

Results (phase 2, 2026-10-17): a change rewrites only the changed records'
rows in the current year's snapshot, including a player's diplomacy rows
only with its diplomacy, and the derived intel, owned-since, activity and
lifecycle rows only where a full save would change them. A tax change
writes 3 rows instead of 567; a change costs 0.8 ms on a 4-player campaign
and 4.9 ms on a 25-player one (release build), most of it loading the year
the mutators read. The `revision` counter in `campaign_metadata` replaces
the snapshot ID as the change signal (`expecting_revision`,
`ConcurrentChange`), for maintenance and the hosted map export. Tests:
`nc-data/tests/row_changes.rs` (the same random changes through row-level
writes and through rewriting the year leave identical tables; write counts;
maintenance refused when a change commits during its run) and
`nc-data/tests/concurrent_changes.rs`.

**Exit:** two sessions on one campaign interleave orders, and every order
of both is saved with no refusal (store and `nc-game` regression tests); a
session that loaded before maintenance is still refused. A stress test of
several processes giving random orders to one campaign loses no order and
leaves fleet and base chains valid, and maintenance then runs. Phase 2: an
action writes only the rows it changes; the same changes through the old
full rewrite and the row-level path leave identical tables; maintenance
refuses a save when a change committed during its run; the classic oracle
tests stay green.

### t205

Make every database behind the community stack and BBS doors production
ready for concurrent players, admin and bots.

t204 hardened `ncgame.db`. A survey (2026-10-17) found the rest unhardened:
`hosted.db`, `roster.db` and `.web-sessions.db` open plain connections (no
WAL, 5 s busy wait, no foreign keys), rerun migrations on every open and
ignore their errors, and write multi-statement changes in DEFERRED
transactions. Seat claims ignore a lost race, the turn queue's supersede and
enqueue are separate statements, `update_settings` rewrites a whole row
read earlier, and roster handles have no UNIQUE constraint. A crashed admin
maintenance run leaves the write block, status and `web_runs` set with no
recovery; `.admin.lock` is held through the whole run, so terminal sign-in
waits on a turn; `nc-sysop maint` does not block writes, so steady play can
keep refusing its save. Backups copy the two databases one after the other,
and a restore leaves a `running` `web_runs` row. The tests use threads, not
processes, and have no crash or soak run. `nc-host serve` is
development-only and is documented, not hardened.

Approach (owner-approved plan, 2026-10-17):

1. One connection policy for every SQLite file (`nc-data/src/sqlite.rs`):
   foreign keys; writers add WAL, a 30 s busy timeout and
   `synchronous=NORMAL`. Migrations run once in `BEGIN IMMEDIATE`, keyed by
   a stored schema version, with errors surfaced.
2. Every read-modify-write in `hosted.db` in one IMMEDIATE transaction;
   seat claims report a lost race; targeted settings updates; a UNIQUE
   roster handle.
3. One maintenance routine for every driver (block writes, run, save with a
   receipt, unblock), recovery of a crashed run, and `.admin.lock` held only
   around `hosted.db` bookkeeping.
4. Backups of both databases under one lock, copies out of WAL mode,
   restores that clear stale runs.
5. Multi-process stress, `kill -9` crash, maintenance-mid-play,
   `hosted.db` concurrency and broadened equivalence tests, and a 60-minute
   soak on bean-void.
6. Integrity checks at open and in the daily backup, logged lock waits and
   failed saves, disk-full handling, whole-state saves renamed for
   maintenance, import and game creation only, and the runbook.

**Exit:** the workspace is green; the stress, crash, maintenance and
`hosted.db` tests pass ten times running; the soak loses no change and
reports latency; a backup taken during writes restores into a playable game;
two browser terminals keep their orders through a turn.

Results (2026-10-17):

- `nc_data::sqlite` opens every hosted SQLite file: foreign keys, WAL for
  writers, `synchronous=NORMAL`, and a 30 s wait that retries the lock every
  millisecond (SQLite's own backoff let one of four nonstop writers wait 6 s;
  the retry cut the longest wait to 0.36 s at the same throughput). Schema
  upgrades of the campaign store, `hosted.db` and `roster.db` run in one
  `BEGIN IMMEDIATE`; hosted transactions use `write_transaction`.
- Maintenance: `nc-sysop maint` and `nc-cli maint` run under
  `CampaignStore::begin_maintenance` (an OS lock, players paused, a crashed
  run's block taken over by the next); the admin `due` check replays a turn a
  dead process left in maintenance; restore marks such a run failed; seat
  authorization does not wait for the admin lock.
- Backups hold `hosted.db`'s write lock across both copies and leave each
  copy out of WAL mode.
- Operations: a door checks the file with `quick_check` at open; `nc-game`
  logs saves that waited over 1 s and failed saves; whole-year saves are
  `save_year*`.
- Tests: `nc-data/tests/process_stress.rs` (writer processes killed with
  SIGKILL over three rounds lose no logged change and leave no partial one;
  maintenance amid four processes), `row_changes.rs` (sixteen change kinds
  over three years with maintenance, and four concurrent sessions replayed
  in commit order, match the year rewrite), `maintenance_run.rs`,
  `hosted_connections.rs`, `storage_failures.rs` (an unwritable file fails
  the change and changes nothing; a damaged file is reported), the Matrix
  request race, and `nc-host` tests for crashed-turn recovery, sign-in during
  a turn, and backups taken during writes. The stress, crash, maintenance and
  hosted tests passed ten runs in a row; the workspace passes (3381 tests, no
  warnings on Linux).
- Soak (bean-void, 60 min, `nc-data/examples/soak.rs`): eight writer
  processes changing a 25-empire campaign nonstop, a reader, maintenance
  every 3 min (20 years, 0.2-1.6 s each) and a backup every minute (60, all
  intact). About 107,000 changes, no writer failure, intact file and chains;
  WAL at most 7 MB; database 97 MB after 20 years of history. Reader loads p50
  39 ms, p99 60 ms. Changes p50 160 ms, p99 1.2 s, max 4.0 s: with eight
  writers back to back, each change (about 20 ms on the late-game campaign)
  queues behind the others; capacity is about 30 changes a second.
- Not hardened (development only): `nc-host serve`'s worker, whose turn queue,
  settings rewrite and roster handles race and which commits accepted turns
  before the campaign save. Not done: the attended check of two browser
  terminals keeping their orders through a turn.

### t121

Package services, back up and restore campaigns, and exercise a private two-game
pilot. Verify concurrency, login, reconnect, schedules, failure recovery and
80x25 rendering. Public launch needs configured Discord credentials and attended
acceptance. Keep 30 daily backups and pre-upgrade snapshots, plus off-server
copies. Document operator commands and the server-independent installation.

## Implementation evidence (2026-10-11)

The working tree contains the Rust admin operations, Python client, managed
browser router, multi-game Discord commands and service examples. No live
services, campaigns or Discord messages have been changed by this work.

Same-year saves now compare a monotonic snapshot revision in the write
transaction. The original two-player regression failed before the fix. The
new controls cover simultaneous writers, seat-claim races, paused-order rollback,
private exports, OAuth state rejection, duplicate terminal connections and
logout. Turn receipts commit with the runtime snapshot; a control simulates a
crash before metadata finalization and recovers without rerunning the engine.

The affected-crate run passed 1,727 tests. The subsequent full workspace gate
passed 3,245 tests, with zero failures and 11 ignored. Focused follow-up tests
cover the final backup and recovery changes: nine host controls and three
browser authorization controls pass. Six Python CLI tests pass. t117 and t118
are complete; t119–t121 await live configuration and attended acceptance. Debug and release loopback
smokes used the real door binary and two disposable games: terminal output,
wrong-game export rejection, duplicate-seat rejection, logout revocation and
lease cleanup passed. Its synthetic session is test-only; it does not validate
Discord OAuth or visual browser rendering.

See [hosting operations](../../dev/community-hosting.md),
[Python operator commands](../../../tools/nc-admin/README.md) and the integration
tests in `rust/nc-host/tests/web_admin.rs` and
`rust/nc-bridge/tests/community.rs`. The owner has configured the Discord application, credentials and bot installation.
Attended OAuth/browser/Discord acceptance, server installation and a verified
off-server backup destination are still
required before public launch.

## Map downloads and bot onboarding

### t122

The owner requested one map ZIP, bot help and about commands, player-manual
downloads, and a short pinned welcome post. The ZIP includes TXT, grid CSV,
known-world-details CSV, ODS and XLSX. Both workbooks use the same authorized
snapshot and have Starmap and Known Worlds sheets. Map ZIP is the default for
Discord and a download link on the website. Existing individual formats remain.
The shared host exporter owns workbook generation; the Discord-only Excel
converter has been removed. User-supplied names are spreadsheet strings, not
formulas. Tests cover names containing commas and XML characters, unauthorized
accounts, wrong-game access, binary HTTP downloads and archive contents.
LibreOffice opened both workbook formats and converted them to CSV successfully.
All 97 tests in nc-host, nc-bridge and nc-discord pass, including the new
archive, authorization, binary response and command-registration controls.

`/help`, `/about` and `/manuals` supplement `/guide`. Manual links returned 200.
The [welcome post](../../dev/discord-welcome.md) is under Discord's 2,000-character
limit. The owner approved the read-only rocket-marked start-here channel.
The channel and pinned welcome post are live; pin state and member permissions
were verified through Discord. All ten commands are registered.

### t123

Use the verified Discord display name as the player handle, with username as
fallback. The Discord ID remains the permanent seat key. Sync the handle when
opening the browser terminal; do not change the empire name or move a seat
when a display name changes or duplicates another name. Strip control characters
and use the existing 25-character game handle rules. Existing sessions without
a verified profile must sign in again. Clarify the startup greeting by naming
the player and empire separately. Test account isolation, renames, terminal
injection, saved empire preservation and real-door startup.

The implementation passes 784 tests across nc-host, nc-bridge and nc-game.
The real-door socket control verifies the saved display handle, terminal output,
wrong-game rejection, duplicate sessions, logout and cleanup. The startup
regression shows player and empire names on separate lines. Signed release
`499b81c6` is deployed after a backup and a quiet-session check. Debug and
release door controls pass; live health and profile storage checks pass.
Existing profiles refresh on the next sign-in.

## Connections

- [Hosting review](../investigations/dp6dayf5-review-web-and-discord-hosting-readiness.md)
- [Current architecture](../../dev/rust-architecture.md)
- [Work tracking](../../dev/work-tracking.md)

### t124

The owner approved eight-character, case-insensitive, single-use seat codes.
Omit visually confusing `0 O 1 I`; store hashes and show each new code once.
Python `nc-admin invites create/list/revoke` manages reservations. Discord
`/invite` verifies the server owner and replies privately; `/join` accepts a
code and binds the seat to the verified Discord ID. Retrying a successful claim
returns the existing seat. Revoke does not evict players. Account and game rate
limits bound failed guesses. Website joins cannot bypass the restriction.

Configure daily midnight using `America/New_York`, including daylight saving.
Recruitment stays open until an explicit operator start, even when full.
Manual-turn games have no automatic deadline. Early operator-run turns do not
push a future scheduled deadline back. Update the pinned welcome post with the
code claim flow. Replace the unused old lobby with a fresh four-player game;
archive its old channel and retain its data privately.

Validation: invite lifecycle, concurrent claims, wrong-game claims, rate limits,
secret-free listing and history, browser bypass rejection, manual scheduling,
owner-controlled start, and both daylight saving transitions. All 109 affected Rust tests and eight Python CLI tests pass. The release real-door
smoke passes. Deployment preflight found and fixed a Python parser bug that
dropped options placed before a subcommand; a regression covers all positions. Signed release `263a4d34` is deployed; its manifest signature and every staged
file hash verified. All eleven guild commands registered, including `/invite`
and `/join`'s optional code. The fresh four-player lobby is invite-only and
recruiting, with its owner seat reserved and no deadline. The prior channel is
archived with history retained. The pinned start-here post now explains codes,
joining and the matching browser identity; the new lobby has its own pinned
instructions. The legacy game data remains intact and unscheduled.

Turn and daily-backup timers are enabled. A live due check performed no turns:
the lobby is recruiting and the separate test game is manual. Health and
operator doctor checks pass. Both games have verified off-server copies and
were restored into an isolated local root, paused, at their expected years.
Off-server copying is verified but not yet automated. Two-account attended
acceptance and general launch readiness remain t119–t121 work; command
registration and synthetic claim tests do not substitute for a friend's live
Discord claim.

The owner reaffirmed the launch hold while finding players: keep Beta 1
recruiting indefinitely until the owner explicitly launches it. Do not start,
resume or advance it during deployment or acceptance checks. A read-only live
check confirmed year 3000, recruiting, and no deadline. The
[returning-host instructions](../../../tools/nc-admin/README.md#returning-to-a-recruiting-game)
cover invite creation, claiming, progress checks and the separate launch step.
The implemented launch control is CLI-only; no Launch button is present yet.

At the owner's request, browser-test was backed up, verified, paused and archived;
it no longer appears in the public catalog. No data was deleted. A live no-code
claim for an unbound account against Beta 1 was rejected with "an invite code is
required"; its roster stayed unchanged. Beta 1 remains recruiting at year 3000,
with one owner-reserved seat, three unclaimed seats and no deadline. The owner
could enter without a code because that Discord ID already held the reserved
seat. The open-seat count is capacity, not permission to join without an invite.

The landing page now directs new players to the verified community Discord
invite, with a separate browser link for returning players. The owner wanted
the terminal style kept but the repeated instructions and technical details
removed. The relay address, Nostr profile link and decentralized-play claim are
removed from the landing page because that path is not ready to advertise.
Desktop and narrow-screen browser previews were checked. This changes discovery
only; it does not authorize starting Beta 1.

The player manual now recommends Discord discovery and browser play through
xterm.js, while retaining traditional BBS and localhost instructions. It covers
private code claims, matching Discord identity, the host-controlled start, map
ZIP downloads and reports. Nostr GameServer is explicitly roadmap-only and not
yet joinable. The PDF was rebuilt and its connection pages checked for content
and layout; the command list stays together on its own page.

The website's Downloads entry is now Manuals: Quick Start, Player Manual and
Sysop Manual, in that order. Release packages are linked through GitHub Releases
instead of duplicated platform rows. The landing and screenshot navigation use
`manuals.html`; `downloads.html` forwards old bookmarks to the new page. The
narrow-screen layout and all three PDF links were checked before publication.

### t125

Rewrite the README around Play, Manuals, Host a game, Build and contribute,
Roadmap and License. Lead with Discord/browser play while retaining BBS and
local options. Keep one paragraph of setting, a concise executable table and
one complete local development workflow. Move specialist commands into a linked
developer guide without losing them. Remove stale claims about Nostr being the
current network route. Verify local setup commands in a disposable game and
check documentation links before publishing.

The README now follows that structure. Specialist workflows live in
[Developer commands](../../dev/developer-commands.md), including explicit
engine/model preset selection for combat comparisons. All relative links in
both documents resolve. Through cargo-slot, the documented new-game command
created a disposable four-player campaign and maintenance advanced it from
3000 to 3001. The nc-game binary built and its help confirmed the player and
scripted-turn options; this was not an interactive terminal test. The sweep
CLI help confirmed the comparison flags. No live campaign was changed.

### t201

`scripts/install_vps.sh`, [scripts.md](../../dev/scripts.md) and the
[Nostr gate guide](../../nostr/ec-gate.md) still install or describe
`nc-sysop nostr serve` and an `nc-maint-all` timer. Both commands were removed
from `nc-sysop` in `cf664453` (2026-04-11). The Nostr daemon is now
`nc-host serve`, and community games are scheduled by
`nc-admin maintenance due`. On 2026-10-16 the archvps gate units
(`nc-nostr.service`, `nc-maint-all.timer`) were disabled after a redeploy showed
the gap.

Owner decision (2026-10-16): the Nostr host and server are still in
development and are not ready for production or public promotion. The
browser (xterm.js) community stack and traditional BBS doors are the only
supported ways to play. Exit: when the Nostr host work resumes, update the
installer and guides to `nc-host`, and keep them marked development-only
until the owner approves a public Nostr path.
