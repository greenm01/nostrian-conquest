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
as a hash. It is single use, expires after ten minutes, allows at most five
attempts, and is cancelled by a newer request from the same user. The link
carries its secret in the URL fragment; the page posts it to the bridge, so
server URL previews and access logs never see or consume it.

- Add `AccountIdentity::Matrix` with a `matrix:` storage key and Matrix user ID
  grammar validation.
- A shared module owns the login-code table in `.web-sessions.db`. `nc-matrix`
  only issues codes; `nc-bridge` only redeems them and creates the session.
- Add the sign-in button, code entry and link landing to the games page, a
  community Matrix link beside the Discord one, and how-to text.
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
