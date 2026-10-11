# Sysop Documentation

This section is for EC operators running the Rust game stack, staging
player-facing assets, and administering live campaigns.

Keep the buckets straight:

- shared docs in `docs/sysop/`
- non-BBS Rust stack docs in `docs/sysop/rust/`
- BBS host setup guides in `docs/sysop/bbs/`

Use these docs in roughly this order:

- [sysop-map-exports.md](sysop-map-exports.md)
  - player map export and queue/download staging across hosted, direct, and
    BBS workflows
- [discord-door-architecture.md](discord-door-architecture.md)
  - architecture specification for hosting `nc-door` inside Discord Embedded
    Activities while preserving 100% self-sovereignty
- [door-kdl-spec.md](door-kdl-spec.md)
  - modern KDL-based dropfile specification for chat rooms, WebSockets, and
    contemporary BBS doors
- [rust/campaign-settings.md](rust/campaign-settings.md)
  - non-BBS campaign settings and raw `nc-sysop settings show` reference
- [rust/turn-kdl.md](rust/turn-kdl.md)
  - KDL turn file format reference for file-based turn submission
- [bbs/mystic-bbs-setup.md](bbs/mystic-bbs-setup.md)
  - validated Mystic setup for `nc-door` on Unix-like hosts and native Windows
- [bbs/synchronet-bbs-setup.md](bbs/synchronet-bbs-setup.md)
  - validated Synchronet setup for `nc-door` on Windows and Linux
- [bbs/enigma-bbs-setup.md](bbs/enigma-bbs-setup.md)
  - validated ENiGMA½ setup for `nc-door` on Linux and Windows, plus the
    compatibility-only DOS path
- [bbs/wwiv-bbs-setup.md](bbs/wwiv-bbs-setup.md)
  - validated Linux WWIV + SyncTERM setup notes for `nc-door`

Practical posture:

For new deployments, prefer the Rust-native `nc-connect`, `nc-game`,
`nc-door`, and `nc-sysop` stack. Keep the roles straight. `nc-game` is the
direct localhost and SSH/VPS session client. `nc-door` is the BBS entrypoint on
both Windows and Linux. `nc-sysop` creates campaigns, edits settings, and runs
maintenance.

The public Windows x64, Windows x86 (32-bit), Windows 7+ x86 (32-bit), and
Linux x64 `nc-sysop` archives are the BBS/sysop packages. Use them when you
want a normal door-host handoff without a Cargo toolchain. Localhost play on
Windows, Linux, and macOS remains a source-build workflow. VPS/Nostr hosting
also remains a source-build workflow with `scripts/install_vps.sh`.

Hosted Rust campaigns are DB-only: one `ncgame.db` per game directory. BBS
door campaigns keep a minimal per-game `config.kdl` beside `ncgame.db`.
Hosted/Nostr game registry data remains global in `/etc/nc-gate/config.kdl`.

Current `nc-door` host wins here are:

- Mystic
- Synchronet
- ENiGMA½

WWIV is validated on Linux. Native Windows WWIV remains unvalidated here and
is not currently recommended; one local Windows 11 test with an older 2023
WWIV Windows package ended with Defender quarantining the downloaded binary,
and the tested `bbs.exe` then crashed before login.

Treat original DOS `ECGAME` hosting as a compatibility bridge, not the main
Rust operating model. Schedule `nc-sysop maint` or `nc-sysop maint-all` with
real host tooling such as `systemd`, `cron`, Task Scheduler, or BBS event
hooks rather than trying to schedule maintenance inside the campaign itself.
