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
