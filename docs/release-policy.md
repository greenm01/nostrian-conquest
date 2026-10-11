# Release Policy

Nostrian Conquest is in active beta. Public releases are intentionally focused
on the local/BBS stack that is being playtested now.

## Current Binary Roles

The public Rust stack currently uses three active gameplay binaries.

- `nc-game`: direct localhost player client
- `nc-door`: BBS door entrypoint on Windows and Linux
- `nc-sysop`: administrator tool for creating games, editing settings, and
  running maintenance

`nc-cli` remains the internal developer/oracle tool and is not part of the
normal public handoff.

## Current Beta Policy

Public GitHub Releases currently publish Windows x64, Windows x86 (32-bit),
Windows 7+ x86 (32-bit), and Linux x64 `nc-sysop` archives for BBS and sysop
use. Those archives are the public BBS/sysop packages. They are expected to
carry:

- `nc-door`
- `nc-sysop`
- the sysop manual PDF
- the player manual PDF
- the player quick-start PDF
- minimal example campaign files for a normal door host handoff

Localhost play is supported on Windows, Linux, and macOS, but it remains a
source-build workflow. In that mode, the sysop builds `nc-game` and
`nc-sysop` from tagged source and runs `nc-game` directly in a local terminal
session.

In short, the current beta contract is:

1. BBS sysops use the public Windows or Linux `nc-sysop` package, whose BBS
   entrypoint is `nc-door`.
2. Localhost sysops build from source and run `nc-game` plus `nc-sysop`.

## Future Hosted Policy

The old SSH/Nostr hosted path is no longer the current public product story.
If a new hosted stack ships later, it will do so as a separate, explicit
release line with its own packaging and docs.

Until then:

- localhost and BBS are the supported gameplay surfaces
- legacy Nostr docs remain in-tree as design/archive material
- classic DOS bundles remain developer and compatibility artifacts, not the
  normal public Rust onboarding path
