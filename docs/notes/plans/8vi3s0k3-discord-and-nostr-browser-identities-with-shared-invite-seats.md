---
id: 8vi3s0k3
date: 2026-10-12
kind: plan
tags: [plan, milestone]
---
# Discord and Nostr browser identities with shared invite seats

### t126

Implement the owner-approved shared Discord/Nostr browser login plan. Each seat
is initially claimed by one verified identity; both providers share campaigns and existing
single-use invite codes. NIP-07 signs a NIP-98 login request with a browser-bound,
single-use server challenge. No relay is required. The owner subsequently requested operator-only linking
and unlinking: one Discord identity and one Nostr identity may share a seat,
but invite redemption never links accounts. Replacement removes all bindings. Names are
labels only; Nostr players may supply a handle or use a key-derived default.

Generalize the host's identity model, sessions, exports and seat replacement;
add browser code redemption; preserve existing Discord behavior. Convert the
current pilot databases with an explicit offline tool, not runtime migrations.
Expire old browser sessions. Preserve all seats, unused codes and game state.

Exit: mixed-provider seat and authorization regressions, signed-login rejection
and replay tests, conversion/CLI tests, full Rust suite and real-door smoke.
Actual nos2x and Discord login acceptance requires an attended browser and is
reported separately. Production deployment is separate. Beta 1 must remain
recruiting at year 3000 with no deadline; no live data is touched by development.

## Evidence

The initial mixed-provider regression failed against the original host because
`account` was rejected in favor of `discord_id`. It passes with the shared
identity implementation. Claims are single-use across providers; operator linking
is a distinct local action. Tests cover both provider orderings, simultaneous
claims, replacement, unlinking and unchanged recruiting/year/deadline state.

Signed NIP-98 login tests verify event signatures/IDs, request URL/method/body,
browser binding, stale/future timestamps, expiry and simultaneous replay on two
runtime threads. Names remain labels. A legacy session database is rejected until
explicitly converted; fresh and converted schemas restart normally.

The final full workspace run passed **3,272 tests, zero failures, 11 ignored**,
including the test split and bootstrap/replay additions.
All 13 Python CLI/conversion tests pass. The built Python zipapp generates the
expected operator account request in dry-run mode. The real-host conversion smoke
preserves the runtime database bytes, existing Discord ownership and unused code
claims, and leaves the game recruiting at year 3000 without a deadline.

Debug and release real-door smokes pass with Discord and Nostr players in one
campaign. They cover private exports, globally consumed codes, linked-seat
exclusion, active unlink/logout revocation and child/lease cleanup. These smokes
use synthetic sessions; the cryptographic login path is tested separately.

A headless Chromium check with a mock signer/server exercised the actual browser
JavaScript: absent extension, rejected signing, NIP-98 URL/method/payload creation,
identity display, private code submission and logout. It does not establish
compatibility with the real nos2x popup. The player PDF was rebuilt and its
connection/command pages visually checked. Changed documentation links resolve;
source whitespace checks pass (the generated PDF's native CRLF xref records are
excluded from text whitespace checking).

Release executables were built from base `ae5ca8a0` plus the t126 working-tree
changes through cargo-slot and remain in its managed
`nostrian-conquest/2/release` pool; no binaries were copied into evidence. SHA-256:

- `nc-host`: `bc8fb16fa0afeec9fa665aa7a947ecd7ed0ddc272a318d592be94f7c77a7be84`
- `nc-bridge`: `2baab145824960239334d626edfd9afcc49613aaa518c39689a85ef75262c5db`
- `nc-discord`: `e53b9aa408c72ac8adcd74688ca90bfe3dd0ba3f34109a5bc6b379f3fbb2679f`
- `nc-door`: `3ce6afd1b483cbc04dbe40daf77c7df5d538b947f96bd15e31c8322220d518e2`

No production service, campaign or announcement changed. Deployment needs the
explicit offline converter and all matching components. An attended real nos2x
and Discord acceptance check remains hosting acceptance work; these automated
checks do not substitute for it.

## Connections

- [Community hosting](../../dev/community-hosting.md)
- [Hosting plan](qv5x08zs-discord-community-hosting-and-python-game-administration.md)

### t127

The owner requested a Nostr login button beside Discord on the public game
listing. Group both sign-in controls in a wrapping row, with the optional Nostr
player name below. Deploy the matching t126 backend and offline account
conversion so the public button works. Preserve Beta 1's recruitment hold,
existing seats and invite states. Back up before conversion, verify release
hashes/signature, public login routes and unchanged campaign state afterward.
An attended real-extension sign-in remains separate acceptance evidence.

The updated page passes the existing headless Chromium signer/claim/logout
smoke. The rollout preflight found no live door processes and confirmed Beta 1
was recruiting at year 3000 with no deadline.

## t127 deployment evidence

Deployed signed release `8709ca82-ui` on 2026-10-12. Host, door and Discord
executables retain the verified t126 hashes above. The bridge was rebuilt from
`8709ca82` because the game-list HTML is embedded at compile time; its SHA-256
is `0f0e09dbe9f8653b8b7b4a36ce1045521f836af2e4100519437c56adcdbd6b0e`.
The matching Python client, static assets and converter are included in the
signed release manifest. The rebuilt bridge passed the real-door smoke again.

The offline conversion backed up five databases. Backup hashes and SQLite
integrity passed; every preserved table row was compared before/after, accounting
for provider prefixes and intentionally expired browser sessions/OAuth state.
Campaign inspection matched exactly after normalizing seat identities. A private
off-server archive of the conversion backup also passed all five database hashes.
Beta 1 remains recruiting at year 3000, three seats open, no deadline. The archived
test game remains archived. No invite or seat was claimed and no turn advanced.
Community browser/bot and turn/backup timers are active; announcements remain off.

The public HTML matches the committed page. Headless Chromium shows adjacent
Discord/Nostr buttons at desktop width, wrapping without overflow at phone width,
and loads the recruiting game card. Existing mock-signer checks cover missing
extensions, signing rejection, payload, identity, invite claim and logout. A live
HTTP smoke with a disposable software signer passed signed NIP-98 login, identity,
Secure session, unbound-seat rejection, replay rejection and logout. Discord's
OAuth redirect passes. This does not replace attended nos2x/Discord acceptance.

### t128

The owner approved state-aware browser controls. Show sign-in choices only to
signed-out visitors; signed-in players see their identity and one sign-out button.
Game cards show Join and an invite field only to signed-in nonmembers while
recruitment has room. Members see Play and downloads, or an explanation when play
is unavailable. Reconnect replaces Play only while the player's seat has a live
terminal. Access lookups describe only the caller's membership and never grant
an unbound account terminal/export authorization.

Exit: browser state-transition checks, membership/isolation and real-terminal
connection checks, workspace tests, and verified deployment without changing
Beta 1's seats, invites, year or recruitment state.

The workspace passed 3,273 tests (zero failures, 11 ignored); all 13 Python
unit tests pass. The Chromium listing smoke covers both providers, signed-out
and signed-in controls, joining, existing seats, active-terminal reconnect,
paused/maintenance/full/closed games, lookup failure and logout. The release
real-door smoke verifies connected status for a linked identity and null-seat
status after unlink, alongside existing authorization and revocation checks.

Signed release `f7e017a0` is deployed. Release signature and every artifact hash
were verified on the server before switching. Pre/post-switch game inspection
and logical hashes of all campaign databases match; SQLite integrity passes.
The browser/bot and turn/backup timers are active. Beta 1's Nostr seat and three
remaining invites are untouched; it stays recruiting at year 3000 with no
deadline. No database conversion or session expiry was needed.

A Playwright check against the live HTTPS site used a disposable software Nostr
signer: signed-out listing, signed login, hidden login choices, one visible
Sign out button, Join-only unbound game card, and logout reset all passed.
No game seat was claimed. This verifies the deployed browser/backend flow,
not an attended nos2x extension popup.

### t129

Add public How to play and Contact sysop pages beside Community Discord. The
owner requested brief, clear prose and a softer appearance. Use a shared warm
gray/sage stylesheet for the listing and both pages, readable type and spacing,
and wrapping layouts. Explain Discord/Nostr sign-in, nos2x/Alby setup links,
single-use invitations and returning with the same identity. Keep operator-only
account linking out of player documentation.

The owner supplied Discord username `niltempus`, user ID `1084285950512857108`,
and the public Nostr key shown on the contact page. Offer a Discord profile link,
community fallback, Nostr app link and copy controls. No messages are sent.
Verify desktop/mobile pages, navigation, copy success/failure, login controls and
public routing before deploying. Preserve all live sessions and campaign data.

The full workspace passed 3,274 tests (zero failures, 11 ignored), and 13 Python
unit tests pass. Existing Chromium sign-in/seat-control checks pass with the
shared stylesheet. Playwright checks at 1280px and 390px verified navigation,
no horizontal overflow, exact contact details, clipboard success and failure,
and signed-out controls. Screenshots were inspected. The supplied npub passes
Bech32 checksum and 32-byte length validation. Extension links were checked
against the nos2x project and official Alby documentation.

Signed release `25c3f5f3` is deployed. Its signature and all artifact hashes
passed verification on the host. Both pages, shared CSS and copy script are
served publicly. The desktop/mobile Playwright checks also pass against live
HTTPS, including contact details, copy controls and navigation. Campaign
inspection and logical hashes of every campaign database matched across the
switch. Sessions were preserved; all community services/timers are active.
No seat, invite or game state changed, and no messages were sent.
