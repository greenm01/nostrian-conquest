---
id: fv6fg33o
date: 2026-10-09
kind: plan
tags: [plan, release]
---
# Release v1.0.0-beta.3

## Scope and exit

Ship the October 2026 bug sweep and its follow-up tasks to BBS sysops as
`v1.0.0-beta.3`. The user chose beta.3 over v1.1: 1.0 never shipped, the
agent guide's post-1.0 storage policy would require save migrations, and the
sweep changed combat, map generation, economy, and door behavior before any
playtest. Exit: Linux and Windows `nc-sysop` archives for beta.3 uploaded to
the `release-artifacts` GitHub release, with a signed `SHA256SUMS.txt`. Task
state and order live in [todo.md](../../../todo.md).

## Task details

### t023

After every worker branch from the 2026-10-09 queue is merged and the full
suite passes:

- Bump `1.0.0-beta.2` to `1.0.0-beta.3` in every workspace `Cargo.toml`,
  `README.md`, both manuals, and `website/landing/downloads.html`.
- Rebuild both manual PDFs with
  `TYPST_IGNORE_SYSTEM_FONTS=true TYPST_FONT_PATHS=/usr/share/fonts/OTF python3 scripts/refresh_manual_revision_date.py --doc both`.
- Build, sign, and upload the Linux x64 `nc-sysop` archive with
  `scripts/publish_release_packages.py`. Signing uses the user's GPG key and
  prompts for its passphrase, so the user must be present.
- Push main and the version commit.

### t024

On Windows, after `git pull`: build the Windows x64, Windows x86, and
Windows 7+ x86 `nc-sysop` archives with the MSVC toolchain the release script
targets, upload them, and regenerate and re-sign `SHA256SUMS.txt`. Then point
the three Windows links in `website/landing/downloads.html` at beta.3; t023
moved only the Linux link so every link keeps resolving until then. Do this
in the same session as the t019 Windows console check.

## Connections

Ships the work recorded in the
[October 2026 bug sweep](../milestones/ym5u2c1x-engine-and-door-bug-sweep-of-october-2026.md)
and the [follow-up plan](c39y8ydp-follow-ups-from-the-october-2026-bug-sweep.md).
See [release-policy.md](../../release-policy.md) for what the archives carry.
