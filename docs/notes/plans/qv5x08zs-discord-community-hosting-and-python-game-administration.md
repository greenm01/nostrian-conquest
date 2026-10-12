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

## Connections

- [Hosting review](../investigations/dp6dayf5-review-web-and-discord-hosting-readiness.md)
- [Current architecture](../../dev/rust-architecture.md)
- [Work tracking](../../dev/work-tracking.md)
