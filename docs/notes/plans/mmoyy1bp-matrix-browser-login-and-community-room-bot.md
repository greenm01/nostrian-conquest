---
id: mmoyy1bp
date: 2026-10-13
kind: plan
tags: [plan, milestone]
---
# Matrix browser login and community room bot

## Scope and exit

The owner approved Matrix as a third browser sign-in provider beside Discord
and Nostr. A new `nc-matrix` process runs a dedicated bot account on
matrix.org, as `nc-discord` runs the Discord bot. The community room is
`#nostrian-conquest:matrix.org`; the owner's account is
`@niltempus:matrix.org`. The bot must use its own account, never the owner's.
The owner registers it; its credential lives only in the server env file.

Login (t140) comes first. Room announcements and commands (t141) follow later.

## Task details

### t140

Players request sign-in by messaging the bot directly or by sending `!login`
in the community room. The bot reads only the verified sender, which stays
visible in encrypted rooms. It replies in an unencrypted room it creates with
that user and sends one message holding both a one-click link and a typed code.
The link opens the sign-in in the browser that follows it; the code serves
players whose chat client runs on another device. The bot never contacts
anyone who has not contacted it first, and replies are rate-limited per user.

The code reuses the invite generator's alphabet and length and is stored only
as a hash. It is single use, expires after ten minutes, and is cancelled by a
newer request from the same user. The approved plan said five attempts per code,
but a wrong code cannot be attributed to any pending code. Instead, 100 failed
redemptions in ten minutes pause Matrix sign-in site-wide for that window, as
invite claims already limit guesses per game. The link
carries its secret in the URL fragment; the page posts it to the bridge, so
server URL previews and access logs never see or consume it.

- Add `AccountIdentity::Matrix` with a `matrix:` storage key and Matrix user ID
  grammar validation.
- A shared module owns the login-code table in `.web-sessions.db`. `nc-matrix`
  only issues codes; `nc-bridge` only redeems them and creates the session.
- Add the sign-in button, code entry and link landing to the games page, a
  community Matrix link to the space `#nostrian:matrix.org` beside the Discord
  one, how-to text, and the owner's `@niltempus:matrix.org` on the contact page.
- The bot account is `@nostrian-conquest-bot:matrix.org` (unregistered when
  checked on 2026-10-13); the page names it, so register exactly that ID.
- Operator linking already treats providers generically
  (`nc-host/src/admin/account_links.rs`); add `--matrix-id` to the Python
  `nc-admin` identity arguments for link, unlink and replace.
- Package `nc-community-matrix.service`, `matrix.env.example` and the release
  manifest entry. Invite codes and their redemption rules are unchanged.

Exit: issue/redeem, expiry, attempt-limit, replay, cancellation and
mixed-provider seat regressions against a mock homeserver; CLI tests;
Chromium checks of button, code entry and link landing; full workspace suite.
Attended sign-in with a real Matrix client and deployment are reported
separately. Beta 1 stays recruiting at year 3000 without a deadline.

## t140 implementation evidence

Built on 2026-10-13; not yet run with a real bot account or deployed.
The full workspace passed **3,290 tests, zero failures, 11 ignored**; all 13
Python `nc-admin` tests pass. New regressions cover Matrix ID grammar and the
255-byte limit; code and link sign-in, single use, expiry, cancellation by a
newer request, the 30-second request interval and the site-wide failure limit;
the bridge route's origin, body and throttling checks; Matrix invite claims and
three-provider linking; and the bot's sync decisions. An end-to-end test runs
`nc-matrix` against a mock homeserver: old invitations are declined silently,
a new one yields one unencrypted private room and one message, a rapid second
request sends nothing, and the code in that message signs in once.

The Chromium listing smoke covers the Matrix button, a rejected code, code
sign-in and link sign-in, which also clears the fragment from the address bar.
Desktop and 390px screenshots of the listing and contact pages were inspected.
The rebuilt player manual has the Matrix paragraph in section 2.2 and grew from
48 to 49 pages. Clippy and rustfmt are not installed for the default nightly
toolchain; stable rustfmt formatted the changed files only. matrix.org's
login discovery offers `m.login.password`, which `nc-matrix login` uses.

## t140 deployment evidence

Signed release `4d0fc4be` is live on 2026-10-14. The release real-door smoke
passed with the staged binaries. On the host, all 21 artifact hashes and the
signature verified against the stored release key. A manual Beta 1 backup and a
copy of the session database were taken first. Game database dumps matched the
post-backup baseline after the switch; Beta 1 remains recruiting at year 3000
with no deadline and three open seats. All four browser sessions survived. The
bridge added the Matrix tables on start. Live pages and scripts match source;
`/auth/matrix` rejects a wrong code (401) and a foreign origin (403).

The bot runs as `@nostrian-conquest-bot:matrix.org` (registered through Google
with a password added) and joined the community room. Its token was created by
`nc-matrix login` and entered with the credentials helper; neither appears in
notes or chat. The owner then signed in from `@niltempus:matrix.org` in a real
browser; the session and profile name `niltempus` were confirmed on the host.
The Matrix identity is not linked to the owner's Beta 1 seat.

### t142

The owner found the bot flow awkward: find the room or bot, then type a request.
matrix.org's authentication service supports a standard OAuth sign-in with only
the `openid` scope. A live probe on 2026-10-14 registered a web client for
`https://play.nostrian-conquest.com` (redirect `/auth/matrix/callback`), and the
owner signed in. The granted scope was exactly `openid`; userinfo returned the
account's `sub` and `username` (`niltempus`), and no consent screen followed the
normal matrix.org sign-in. The token was revoked afterward. No Matrix API scope
is requested, so the site never gains access to messages, rooms or the account.

Make **Sign in with Matrix** this one-click flow for matrix.org accounts:
`/auth/matrix/login` redirects with a single-use state and PKCE;
`/auth/matrix/callback` redeems the code, maps `username` to
`@username:matrix.org`, takes the public display name as the player handle,
creates the session, and revokes the token. Keep the t140 bot code form as the
"Not on matrix.org?" fallback. Both routes yield the same Matrix identity.
The client ID lives in `web.env`; the secret is entered with the credentials
helper. A player-entered Matrix ID flow, where the bot messages the player,
waits until players on other homeservers need it.

Exit: redirect, state/PKCE, replay and failure regressions against a mock
authorization server; updated listing smoke and docs; full workspace suite;
deployment and an attended sign-in.

Built on 2026-10-14. The full workspace passed **3,295 tests, zero failures,
11 ignored**; 13 Python tests pass. Five bridge tests run the flow against a
mock authorization server and homeserver: the redirect carries only `openid`,
the registered redirect URI and an S256 challenge matching the verifier later
sent with Basic client authentication; state is single use and bound to its
cookie; the access token is revoked even when userinfo lacks a username;
cancellation and bad userinfo create no session; the display name falls back to
the Matrix ID; and an unconfigured bridge returns 404. The listing smoke checks
the button's link and the fallback code form; desktop and 390px screenshots
were inspected. The rebuilt player manual remains 49 pages.

### t141

Have `nc-matrix` post the host's game announcements (recruitment, completed
turns, next deadline) to the community room and answer simple commands such
as `!games`, mirroring `nc-discord`. Depends on t140 and the Discord
announcement work in t120.

## Connections

- [Discord and Nostr browser identities](8vi3s0k3-discord-and-nostr-browser-identities-with-shared-invite-seats.md)
- [Hosting plan](qv5x08zs-discord-community-hosting-and-python-game-administration.md)
- [Chat and web doors decision](../decisions/m7x3b9v1-chat-and-web-doors-bridge-bbs-and-nostr-onboarding.md)
- [Community hosting](../../dev/community-hosting.md)
