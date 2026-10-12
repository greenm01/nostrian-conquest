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
turns in a reserved test game. Turn and daily-backup timers are enabled. The test game now uses manual turns
with no deadline. Announcements remain disabled. The new four-player invite-only
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
the bot or CLI. A successful code claim reserves a seat; it never starts a game.

## Release context

Beta.3 is published. t024 is complete: all four BBS/sysop packages, their
signed checksum manifest, and the matching download page are live. The
quick-start PDF appears above the full manuals. All seven archive/manual links
returned HTTP 200 with matching hashes; downloaded checksums and the GPG
signature verified. See the [release plan](../notes/plans/fv6fg33o-release-v1-0-0-beta-3.md)
for build commit `1b993652`, hashes, compiler versions and acceptance limits.

The x86 Defender block was cleared by building the standard library from
source, as the Win7+ target already did. Both x86 release targets now use that
method. No exclusion or allow rule was added. All three Windows candidates
passed explicit Defender scans and runtime smoke tests on Windows 11; Windows
7 itself was not tested. Linux passed the same runtime smoke tests. The
workspace passed 3,233 tests, zero failures and 11 ignored; five packaging
tests passed.

Run `zk queue` before starting new work. The current priority is the approved community-hosting work. Windows builds use a native MSVC host;
sync by git bundle rather than remote `git pull`, which has hung before.
Check its tree before updating it.

All commits must be signed. A cold GPG/SSH agent cache can block signing or
pushing; ask the owner to unlock it instead of committing unsigned. Fetch
before merging: the owner's other agent may push nc-bridge, nc-herald or
nc-dropfile work. Use cargo-slot from `rust/`; never invent a target dir.
Task IDs are allocated above all IDs in todo and monthly done files.

The broader invalid-order abort differences found by t099 remain outside
its scope; see [q0a0mhm2](../notes/plans/q0a0mhm2-classic-follow-ups-from-the-october-2026-oracle-work.md#t099).
