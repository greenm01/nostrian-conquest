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

Follow-up: Defender signature update `1.459.670.0` to `1.459.676.0` did not clear
the standard x86 build. Switching from stable Rust 1.99 to nightly 1.101 alone
also failed. A dependency-free x86 Rust control passed an explicit scan. Building
modern x86 with the same source-built standard-library setup as the Windows 7+
target produced an archive that passed an explicit Defender scan, game creation,
two maintenance passes, door startup over a loopback socket and clean disconnect.
Real-time protection stayed enabled; no exclusions or allow rules were added.
These controls establish a usable build method, not the exact detection cause.

The release script now uses that method for both x86 targets and records it in
BUILD-INFO. Five Python tests cover artifact lookup and the build contract.
Windows x64 and Windows 7+ x86 also passed explicit Defender scans and the same
runtime smoke test on Windows build 26300. This is not a test on Windows 7 itself.

Final candidates were built from signed commit `1b993652`. All four contain the
same three reviewed PDFs. Linux uses stable Rust 1.98; Windows x64 uses stable
1.99; both Windows x86 targets use nightly 1.101 with source-built standard
libraries. The final Windows archives passed explicit Defender scans and the
runtime smoke checks above. Linux passed the same runtime checks. Windows PE
machine types and transferred archive hashes match their intended targets.
The Rust tree is unchanged since the 3,233-test workspace run.

| Archive suffix | SHA-256 |
| --- | --- |
| `linux-x64.tar.gz` | `ecb7a190b493665c4dd7db5f545467d3de17a9bad3b9ab5815171b923909e326` |
| `windows-x64.zip` | `68322facfa954bb967559da9739018db96d003d2c71293328171e3122c42b814` |
| `windows-x86.zip` | `fdd56666bb8997e76e36a5f7708cb02bed6240d899fba654486dafb0bdb89c72` |
| `windows7-x86.zip` | `b7226b73be998b8f6c0044549f735b1e13b9a306b449faf7bfcff7a28561d755` |

All names begin `nc-sysop-v1.0.0-beta.3-`. The complete four-archive manifest
is signed and verified. Archives, manifest and detached signature are staged
under `releases/`.

## Publication result, 2026-10-11

With the owner's approval, all four archives and the signed checksum manifest
were uploaded to the existing `release-artifacts` release. The preserved DOS
archives were left untouched. Release notes identify the build commit and the
Windows acceptance limit.

The download page was deployed to the production server. It links to beta.3
for all four platforms and lists the quick-start PDF above the player and sysop
manuals. The public page matches the committed HTML byte for byte. All seven
archive/manual links returned HTTP 200; downloaded bytes match the recorded
archive hashes or the committed PDFs. A separate download of all four archives,
the checksum manifest and its detached signature passed checksum and GPG
verification.

Live entry points: [downloads](https://nostrian-conquest.com/downloads.html)
and [release assets](https://github.com/greenm01/nostrian-conquest/releases/tag/release-artifacts).

## Combat refresh, 2026-10-16

All four archives were rebuilt from `1c1de2a7`, which carries the t198/t200
combat rules and the rebuilt player manual. Linux was built on bean-void
(Rust 1.98). Windows x64 used stable 1.99 and both x86 targets nightly 1.101,
all built on bean. Every archive passed a fresh game and two maintenance
passes, and the Windows archives passed explicit Defender scans. The door was
checked with `--help` only; no loopback session was run. The checksum
manifest was signed on the Mac mini with the release key. The website's
player manual PDF matches the committed one.

| Archive suffix | SHA-256 |
| --- | --- |
| `linux-x64.tar.gz` | `b68b26336d7cfecfabfd4db661ae8ded2af57c4c301ec97dc908dc98d3feda2b` |
| `windows-x64.zip` | `a9aab34e4e8228aafe77ab96f5ced3a2e029d54c889a8a8700edca8136f64b2e` |
| `windows-x86.zip` | `7ceda2d945d5a6b1784fe8160c4cab910b94c9150c6a2ac0e2d155be8a2e6574` |
| `windows7-x86.zip` | `55cfb736fdb27c612435d051909d83940a2fe1556ac0a55e36651e388121b2e8` |

The owner uploaded the four archives and the signed manifest to
`release-artifacts` and refreshed its note. A fresh download of all six
assets passed GPG and checksum verification.

## Planetary combat refresh, 2026-10-16

The four archives were rebuilt again from `08f44280`, and once more from
`6c381989`, which fixes a crash in t202's joint-bombardment scan (an empire
with more starbases than fleet records). The `6c381989` hashes are linux
`bbfc8b09075e2567...`, windows-x64 `7ee40f4a9c03eaef...`, windows-x86
`32b379ef6f7a7271...`, windows7-x86 `b2beb3aaa80da149...`. The `08f44280` build (t202: two-step
batteries, full-strength battery fire, classic capture damage) with the
rebuilt player manual and quick start; each passed a fresh game and two
maintenance passes, and the Windows archives passed Defender scans. The
manifest is signed with the release key on the Mac mini. The website's three
manual PDFs match the committed ones.

| Archive suffix | SHA-256 |
| --- | --- |
| `linux-x64.tar.gz` | `2acd78f99b8792db765c7cb297e43847ef7defbcb9c564891ae5e7a81b6096bb` |
| `windows-x64.zip` | `6c156baafe2e984a2df3a51d09d0c7fadebedf2e18140b1343f27112ab0e8520` |
| `windows-x86.zip` | `7882d4347989e3f7141ce6bef83c214a979638d58c2766b2603ab89d8ff27f4f` |
| `windows7-x86.zip` | `52f1c51db6c355136674433e388d227eca3c285fa727629455290a4369f03854` |

The owner uploaded them and the signed manifest to `release-artifacts` and
refreshed its note; a fresh download of all six assets passed GPG and
checksum verification.

## Connections

Ships the work recorded in the
[October 2026 bug sweep](../milestones/ym5u2c1x-engine-and-door-bug-sweep-of-october-2026.md)
and the [follow-up plan](c39y8ydp-follow-ups-from-the-october-2026-bug-sweep.md).
See [release-policy.md](../../release-policy.md) for what the archives carry.
