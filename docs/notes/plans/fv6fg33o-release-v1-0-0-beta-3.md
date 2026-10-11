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
targets, upload them, and regenerate and re-sign `SHA256SUMS.txt`. Do this
in the same session as the t019 Windows console check. Until then the
Windows download links 404: see the t023 result.

Result of t023 (2026-10-10): the Linux x64 archive (built on stable rustc
1.98.0 from `ce383f2d`) is uploaded with a signed `SHA256SUMS.txt` that
covers only it, and main is pushed. `publish_release_packages.py` prunes every
nc-sysop and nc-connect asset not at the current version, so publishing
deleted the beta.2 Windows zips. All four links in
`website/landing/downloads.html` now name beta.3, so the Windows links work
again once t024 uploads those archives. Deploy the site after t024.

## Readiness review, 2026-10-11

At main `54663dcd`, the published Linux beta.3 archive still records
`ce383f2d` in BUILD-INFO. Its SHA-256 matches the manifest and its detached
signature verifies against the repository release key. It predates the latest
engine/report and door workflow fixes. No Windows Rust archives are currently
published. The live download page still names removed beta.2 assets, while
its repository source names beta.3.

Before closing t024:

- Fix the packager's binary lookup: `build_binaries` assumes `rust/target`,
  but cargo-slot puts artifacts elsewhere. Bundle the binaries actually built.
- Rebuild Linux and all three Windows packages from one final commit, with
  the reviewed [quick-start guide](../investigations/jl2o2ym3-quick-start-guide-accuracy-and-release-inclusion.md)
  and current manuals. Use the revision helper's `--doc all` for all three PDFs.
- Verify extracted packages and perform current Windows door acceptance.
  Earlier Windows console acceptance does not cover the latest source.
- Regenerate the complete checksum manifest, sign it, upload the packages,
  and deploy the matching download page. Check all four live links afterward.

Existing source evidence: 3,233 workspace tests passed, 11 ignored; after the
last prompt wording change, 486 affected tests passed. A Linux release build
and attended BBS menu testing passed. These do not substitute for fresh
Windows binaries or extracted-package acceptance. No release assets or live
website content were changed by this review.

## Packaging preparation, 2026-10-11

The packager now takes executable paths from Cargo's build messages. It no
longer assumes `rust/target`, so cargo-slot and configured target directories
use the artifacts from the actual build. Three Python tests cover slot paths,
Windows executable paths, and refusal to fall back when an artifact is missing.
All three manual PDFs were rebuilt, and the quick-start link leads both manual
lists in the project README. A fourth test checks that Windows 7 build metadata
names the nightly compiler actually used, rather than the host default.

At `ee34e2a2`, the workspace passed 3,233 tests, with zero failures and 11
ignored. Linux x64 and native MSVC Windows x64 archives passed content checks,
fresh-game creation, two maintenance passes, door startup rendering over a
loopback socket, and clean disconnect. Each bundled PDF matches the repository
PDF byte for byte. This scripted check does not replace attended Windows
console acceptance.

The Windows x86 build compiled, but Microsoft Defender quarantined its three
generated executables as `Trojan:Win32/RustyStealer.ARY!MTB` before packaging.
Defender reports that the threat did not execute. A false positive is not yet
established; no exclusion or protection change was made. Resolve this detection
before shipping that archive. No public release assets have been replaced.

## Connections

Ships the work recorded in the
[October 2026 bug sweep](../milestones/ym5u2c1x-engine-and-door-bug-sweep-of-october-2026.md)
and the [follow-up plan](c39y8ydp-follow-ups-from-the-october-2026-bug-sweep.md).
See [release-policy.md](../../release-policy.md) for what the archives carry.
