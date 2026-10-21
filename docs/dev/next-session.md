# Next Session

Historical handoffs are in
[archive/next-session-archive.md](archive/next-session-archive.md).

## Codex takeover (2026-10-11)

Claude and her subagents have stood down. The inherited work is merged
and its worktrees removed. Final merged validation: 3212 Rust tests pass,
0 fail, 11 ignored; four Python evidence tests pass. The inherited work:

- t101: CLI maintenance uses the shared campaign runner and retains
  starbase, scorch and recovery events. Three failing-first regressions
  cover the omitted reports. The merged workspace passed 3198 tests.
- t099: classic AI merge eligibility, planetless civil-disorder conversion
  and Hold depth. The owner rejected classic's stale missing-base merge
  flag. Final review survives rogue conversion. Fleet-chain removal and
  intentional rogue disbanding have regressions. The merged workspace
  passed 3212 tests, 0 failures, 11 ignored. Both manual PDFs were rebuilt.
- t090: the 7020-game price matrix and tooling are in
  [tools/price-campaigns](../../tools/price-campaigns/README.md), with results
  in [n7fzd2vk](../notes/investigations/n7fzd2vk-unit-prices-on-generated-maps-with-hostile-openings.md).
  Keep prices and the opening unchanged. This is scripted-policy evidence,
  not human win odds. Most campaigns did not finish; the attacker protects
  home and favors colonies. The matrix predates t099, which is explicit in
  its provenance. Four Python integrity tests pass; a separate current-code
  release-build smoke run passed.

The initial unfinished files, raw price TSVs and validation logs are saved
outside tmpfs under the development host's local-state directory:
`~/.local/state/nostrian-conquest/handoffs/20261011T175210Z/`.
All 90 TSV hashes match committed provenance. Experimental price worktrees
and their cargo pools were removed after verifying their source backup.
Do not restart the old `run_matrix.sh`: it deletes its output directory.

## Current work: learned AI opponent (Mac mini)

The owner approved the [AI opponent plan](../notes/plans/qkgqypzw-learned-ai-opponent-harness-hierarchical-agent-and-fleet-level-bootstrap-on-the-mac-mini.md)
(t150–t158, `+parallel +ai`). Background and measurements are in
[lenngda1](../notes/investigations/lenngda1-training-an-ai-opponent-for-nostrian-conquest.md).
Done through t175, t157, t158 and the work toward t165 and t167. The
details are in the plans' result sections.

- **The agent.** It is the fleet-level agent (t157), with the strike
  family handed to the network and convoys (transports without warships)
  left to the executor's ferry, on t175's strategist with the allocation
  goal. t191 retrained it to join fleets and promoted t191-orders snapshot
  200 on 2026-10-20: greedy against three executors on 1,024 maps it wins
  0.589 at 60 years and 0.786 at 100 (+0.083 / +0.051 paired against the
  t206 agent), with about 49 / 299 joins and 0.4 / 0.6 unloads a game. The
  previous agent is `fleet-final-pre-t191`. Models are backed up with
  restic to archvps and crunch (`~/.local/bin/nc-ai-backup`).
  - Checkpoints: `~/.local/state/nostrian-conquest/ai/fleet-final.pt` and
    `fleet-final.onnx`.
  - The doctrine work after t164 kept only the blockader (t171). The
    seat actions gained Guard a Starbase, Detach and deep-space waypoints.
- **Real games (t165).** `nc-ai-onnx` runs the exported agent in Rust at
  2 ms a turn and matches Python exactly. Sysops mark AI seats with
  `nc-sysop settings ai-seat` (difficulty and personality). `nc-sysop maint`
  and `nc-host` maintenance play them with `NC_AI_MODEL`.
- **Personalities (t167).** Land grab, rusher and turtle are profiles in
  `nc-ai/personalities/*.kdl`.

- **Combat rules (t206, 2026-10-18).** Heavies cover the destroyers,
  execution finishes cripples at half price, a heavy line adds +1
  ([tdkitij6](../notes/decisions/tdkitij6-heavies-cover-the-destroyers-which-finish-cripples-at-half-price.md)).
  Mixes now beat pure cruisers 70/30 at equal cost. The combat model's
  `engine-t206` preset matches the engine.
- **Mixed hulls adopted, agent retrained (t189/t191, 2026-10-19).**
  Retuned under t206, mixed executors match cruiser ones, so the land-grab
  style and rusher build cruisers and destroyers (the turtle stays
  cruisers). The executor builds for the network seat, so the agent was
  retrained on mixed fleets and promoted: 0.725 at 100 years against the
  old agent's 0.192 with mixed builds. Main's AI may ship again; the live
  stack carries beta.4. See the AI plan's t189 and t191 sections.
- **Next (t207, 2026-10-20).** Opening the mission head by RL failed
  (rare missions never win the greedy argmax). The owner chose executor
  rules first. Blitz is adopted: strikes blitz worlds whose batteries are
  known gone, in executor seats and, by converting the network's Invade,
  in agent seats (neutral in both). A Salvage rule (scrap spent ETACs)
  lost slightly at 100 years and was dropped. The RL variant waits. See the
  AI plan's t207 section.

The owner signed off t165 and t167 on 2026-10-18 without recorded
playtests. t166 now needs real human games, played with the players'
consent.

Plans:
[doctrine follow-ups](../notes/plans/kvqg7wv5-executor-doctrine-follow-ups-from-strategy-research.md),
[AI opponent](../notes/plans/qkgqypzw-learned-ai-opponent-harness-hierarchical-agent-and-fleet-level-bootstrap-on-the-mac-mini.md).

- Run one PPO experiment at a time on the mini: two at once swapped it
  heavily.
- `zk` and `todo.sh` come from Homebrew; `~/src/todo.txt-cli/todo.sh`
  links to the Homebrew copy.

**Where the work runs:**
- Training runs on the Mac mini (M4 Pro, 12 cores, 16-core GPU, 24 GB),
  repo `~/dev/nostrian-conquest`, Rust 1.95 stable.
- bean-void (`ssh bean-void`) is spare CPU.
- Crunch hosts another project's latency measurements, and AI work never
  runs there (it only holds the restic model backup).

## Current work: Discord community hosting

**Owner launch hold:** Beta 1 must stay recruiting while the owner finds players.
Do not start, resume or manually advance it during setup, deployment or testing.
Wait for the owner's explicit launch action or instruction. Filling its seats
and opening Play do not start it. The current launch control is CLI-only;
there is no Launch button yet. See the [returning-host instructions](../../tools/nc-admin/README.md#returning-to-a-recruiting-game).

The owner approved the [hosting plan](../notes/plans/qv5x08zs-discord-community-hosting-and-python-game-administration.md)
(t117–t121): Python administration, Discord discovery and browser BBS play.
The first implementation milestone is ready for a private pilot. Read that plan and
[community-hosting.md](community-hosting.md) before continuing. The isolated browser
and bot services are running; existing BBS/Nostr services remain in place.
HTTPS, the OAuth redirect and the initial seven guild commands pass live checks.
The owner tested login, empire setup, saved orders and reports after two manual
turns in a reserved test game. Turn and daily-backup timers are enabled. The browser test game is now archived at the owner's request, with a verified backup retained. Announcements remain disabled. The new four-player invite-only
lobby stays recruiting until the owner explicitly starts it; its configured
schedule is midnight America/New_York, adjusted for daylight saving.
The workspace gate passed 3,245 tests (zero failures, 11 ignored), and final
hosting controls and six Python tests pass. t117 and t118 are complete.
Debug and release real-door smokes pass. Discord credentials were entered
privately on the deployment host, and the existing bot installation was verified.
Multi-player acceptance and automated off-server copies remain outstanding.
Current off-server copies have passed verification and isolated paused restores.
Do not mistake synthetic session tests for live login acceptance.

t122's map ZIP and `/help`, `/about`, `/manuals` are deployed in signed release
`bcf9be05`. All ten guild commands registered; the live ZIP contains the five
expected files, and both nested workbook archives and their XML validate.
All 97 affected-crate tests pass. The [welcome post](discord-welcome.md) is posted
and pinned in the approved read-only rocket-marked start-here channel. t122 is
complete. t123 replaces the browser launcher's `web1` placeholder with a verified
Discord display name and separates player and empire in the greeting. Its
784 tests and debug/release real-door smokes pass. Signed release `499b81c6`
is deployed; health and profile storage checks pass. Existing
sessions must sign in again to populate their verified profile.

t124 is deployed in signed release `263a4d34`: private eight-character seat
codes, owner-only `/invite`, `/join` with a code, and Python create/list/revoke
commands. The operator wrapper is installed; see the linked command guide.
All 109 affected Rust tests, eight Python tests and the release real-door smoke
pass. Eleven guild commands registered. Both the start-here post and fresh
lobby instructions are pinned; the old channel retains its history in an
archive. No invite codes were pre-generated. The owner can issue them through
the bot or CLI. Its sole claimed seat was assigned to the owner before play;
returning to that seat requires no new code. A live no-code claim for an unbound
account was rejected, with the roster unchanged. A successful code claim reserves a seat; it never starts a game.

## NIP-07 browser login update (t126)

The shared Discord/Nostr identity implementation is deployed. See
[the implementation note](../notes/plans/8vi3s0k3-discord-and-nostr-browser-identities-with-shared-invite-seats.md)
for validation: 3,272 Rust tests and 13 Python tests pass; debug/release door
and conversion smokes pass. Invite codes are consumed once across both providers. The owner
also requested unadvertised operator-only link/unlink controls: a sysop can grant
both identities the same seat; a used invite can never do this. The player UI
and bot expose no linking operation. One terminal per seat still applies.

Deployment used the explicit [offline pilot conversion](../../tools/nc-admin/README.md#offline-pilot-identity-conversion)
and matching host/bridge/bot/Python/static versions. Existing seats, codes and
profiles were preserved; old browser sessions expired. Source tests and synthetic
browser/door smokes do not replace an attended nos2x and Discord login check.
t127 puts both sign-in buttons together on the public game listing. Beta 1's
launch hold remains in force: recruiting, year 3000, no deadline. No seat or
invite was claimed during rollout. Attended login acceptance remains part of
the hosting acceptance work.

t128 is deployed in signed release `f7e017a0`. The game list hides login choices
once signed in, shows Join only for an available unclaimed seat, and shows Play
for a member. Reconnect appears only for an active terminal; paused/unavailable
games explain their state. Workspace tests (3,273 passed), 13 Python tests,
Chromium state checks and release real-door checks pass. This release changes no
database schema and preserves existing sessions. Deployment verified all game
database contents unchanged. Beta 1 is still recruiting with no deadline.

t129 is deployed in signed release `25c3f5f3`: public `/howto` and `/contact`
pages, the owner's Discord/Nostr contact details, and shared warm gray/sage
styling for the browser listing. Desktop/mobile navigation, clipboard controls
and login-state checks pass; 3,274 Rust tests and 13 Python tests pass. The live
Playwright checks pass too. Deployment preserved sessions and all campaign data.

t130 is deployed in signed release `4115f43c`: external website links open new
tabs; local navigation and login stay in the same tab. Game data and sessions
were preserved.

t140 is deployed in signed release `4d0fc4be`: Matrix sign-in through the new
`nc-matrix` service and `@nostrian-conquest-bot:matrix.org`, plus Community
Matrix links and the owner's Matrix contact. The owner signed in with a real
Matrix client. t142 (signed release `5d6407dc`) makes **Sign in with Matrix**
one-click OAuth for matrix.org accounts, requesting only `openid`; the bot code
form remains for other homeservers. Details and evidence are in the
[Matrix plan](../notes/plans/mmoyy1bp-matrix-browser-login-and-community-room-bot.md);
The Matrix bot answers `!games`, `!help` and `!login` and posts announcements in the community room (t141, done 2026-10-20; not yet deployed).

On 2026-10-14 the bot moved to `#nostrian-play:matrix.org`, the *play* room
inside the `#nostrian:matrix.org` space, and its display name became
`ncbot`.

Signed release `19fb1f5d` (v1.0.0-beta.4) went live on 2026-10-19 after an
`nc-admin` backup of Beta 1; the owner accepted its door rendering and map
downloads there. Beta 1 stays recruiting at year 3000 with one seat claimed;
`ea1a6b3b` is kept for rollback. The Nostr gate stays disabled. Community
Beta 1 is the only hosted game. Earlier release history is in the archive.

## Release context

Beta.4 is published (t210, 2026-10-19): four `nc-sysop` archives built from
`4abc2140` and a signed `SHA256SUMS.txt` replaced beta.3 on
`release-artifacts`; the website's three manual PDFs match. It adds door
renders that send only screen changes (t208) and nc-sysop `map-export`,
`economy-report` and `check` (t209). The save format is unchanged from
beta.3. See the [release plan](../notes/plans/02cjqug4-release-v1-0-0-beta-4.md)
for hashes and checks.

The x86 Defender block was cleared by building the standard library from
source, as the Win7+ target already did. Both x86 release targets now use that
method. No exclusion or allow rule was added. All three Windows candidates
passed explicit Defender scans and runtime smoke tests on Windows 11; Windows
7 itself was not tested. Linux passed the same runtime smoke tests. The
workspace passed 3,233 tests, zero failures and 11 ignored; five packaging
tests passed.

On 2026-10-18 the owner closed t119, t120, t143, t144 and t205 as built and
tested; their attended checks (sign-in on all three providers, two browser
terminals through a turn, live announcements once `NC_DISCORD_CHANNEL_ID` is
set) were t121's pilot exit. t121 closed on 2026-10-20: the automated pilot
(`tools/community-pilot/pilot.py`, 47 checks on archvps), the owner's
Discord and matrix.org sign-in and play, and nightly off-server copies to
crunch. Live announcements moved to t211. Run `zk queue` before starting new work. The current priority is the approved community-hosting work. Windows builds use a native MSVC host;
sync by git bundle rather than remote `git pull`, which has hung before.
Check its tree before updating it.

All commits must be signed. A cold GPG/SSH agent cache can block signing or
pushing; ask the owner to unlock it instead of committing unsigned. Fetch
before merging: the owner's other agent may push nc-bridge, nc-herald or
nc-dropfile work. Use cargo-slot from `rust/`; never invent a target dir.
Task IDs are allocated above all IDs in todo and monthly done files.

The broader invalid-order abort differences found by t099 remain outside
its scope; see [q0a0mhm2](../notes/plans/q0a0mhm2-classic-follow-ups-from-the-october-2026-oracle-work.md#t099).

The approved Tokyo Night Storm palette (t131) is live in signed static release
`4716b19a` across the listing, how-to and contact pages. Live desktop/mobile
checks pass. Services were not restarted; campaign data was not modified.
The title accent follow-up (t132), signed static release `b59a7a24`, is live:
warm yellow site name, lavender page titles and blue section/game headings.
Desktop/mobile checks pass locally and live.
