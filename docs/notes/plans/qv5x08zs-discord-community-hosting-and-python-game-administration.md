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

**Exit:** two sessions on one campaign interleave orders, and every order
of both is saved with no refusal (store and `nc-game` regression tests); a
session that loaded before maintenance is still refused.

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
