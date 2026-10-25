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
