---
id: r5b5nc01
date: 2026-10-25
kind: plan
tags: [plan, release]
---
# Release v1.0.0-beta.5

## Scope and exit

Ship what landed after beta.4 (`4abc2140`) as `v1.0.0-beta.5`:

- each game's sysop settings in `config.kdl`, with Game Notices (t218);
- the autopilot report and the oldest-first inbox;
- the View World report order fix;
- bombardment intel for the attacker (t194);
- the executor's repair at bases, colony starbases, Blitz and convoy ferry (t191, t207).

**Not compatible with beta.4 (owner, 2026-10-25).** A beta.4 BBS game's
`config.kdl` has a `players` line, which beta.5 refuses. The release notes
tell sysops to delete that line. No compatibility code is added.

**Exit:** the Linux and three Windows `nc-sysop` archives and a signed
`SHA256SUMS.txt` replace beta.4 on `release-artifacts` with new notes, and
the website's three manual PDFs match the committed beta.5 builds.

## Task details

### t219

1. Bump every workspace `Cargo.toml`, `Cargo.lock`, `README.md` and the
   three manuals to `1.0.0-beta.5`; rebuild all three PDFs.
2. Build the Linux x64 archive on bean-void and the three Windows archives
   on bean with MSVC; each passes `build_release_bundle.py --verify`.
3. By hand on each: new game, two maintenance years, `check`, `map-export`,
   a door join through a drop file, and a beta.4 BBS game opened after its
   `players` line is deleted.
4. Sign `SHA256SUMS.txt`, replace the beta.4 assets and notes on
   `release-artifacts`, verify a fresh download, update the website PDFs.

## Release (2026-10-25)

The four `nc-sysop` archives were built from `0bb3b406` (after a usage-text
fix found in the first Linux build): Linux x64 on bean-void (rust 1.98.0),
the three Windows targets on bean with MSVC Build Tools 18. Each passed
`build_release_bundle.py --verify`, then by hand: new BBS game with
`--players`, two maintenance years, `check` and `map-export`, and the same
three PDFs as the commit. The Linux archive also passed a full door join
through DOOR32.SYS, and a game made with the published beta.4 archive ran
under beta.5 once its `players` line was deleted (before that, the expected
refusal). The Windows door join was not run. Defender found no threats in
the three zips.

| Archive suffix | SHA-256 |
| --- | --- |
| `linux-x64.tar.gz` | `bd34d5db352ae93327924445b59cb722b7869aea204cb43e0d7a55593e75afff` |
| `windows-x64.zip` | `cd5d57a940f57e93130879c2747aa230dd7f4b4d5220aa9ed0f1f98848851bf2` |
| `windows-x86.zip` | `e1d24a65311d893db75031eec14c4a70323baf9bf8589e336cd1737ca66ee796` |
| `windows7-x86.zip` | `f86586d8168b90a32445f0a233fafab3d7d3ab52568479d8cb516cf80b17a192` |

`SHA256SUMS.txt` is signed with release subkey `57184BB62A800062`. The
beta.5 archives and signed manifest replaced beta.4 on `release-artifacts`,
with new notes leading with the upgrade step; a fresh download passed GPG
and checksum verification. The website's three manual PDFs were replaced
(no old copies kept) and match the commit. The download page links to the
releases page and needed no change.

`scripts/cargo-msvc.cmd` still names the Visual Studio 2022 folder; bean now
has Build Tools 18, so the release build used that path directly.
