---
id: 9qvwd3ke
date: 2026-11-07
kind: plan
tags: [plan, release]
---
# Release v1.0.0-beta.6

## Scope and exit

Ship what landed after beta.5 (`0bb3b406`) as `v1.0.0-beta.6`, at the
owner's request (2026-11-07):

- combat: each hull fires on its own (t255), batteries share their fire by
  ship count (t258), Guard/Blockade fleets defend the orbit (t250), and the
  bombardment and invasion rules of t232–t238, t252, t257 and t259;
- fleets: Hold takes no coordinates, a Join order shows its ETA,
  Commission offers to merge new fleets into one in orbit, warships that
  merge into an unarmed fleet keep their ROE, and ordering from the fleet
  list returns to it;
- planets: Fortify puts a world in defensive mode;
- reports: the many classic-wording and ordering fixes, and the Scorch-Earth
  report ahead of a same-week capture;
- AI: logged turns (t166), the t256 style retunes, and t260's ETAC sizing
  and salvage.

The runtime schema is still 17, as in beta.5; the defensive-mode table is
created on open. A beta.5 game should open unchanged, and the hand checks
test it.

**Exit:** the Linux and three Windows `nc-sysop` archives and a signed
`SHA256SUMS.txt` replace beta.5 on `release-artifacts` with new notes, and
the website's three manual PDFs (archvps `/var/www/html/manuals/`) match the
committed beta.6 builds.

## Task details

### t261

1. Bump every workspace `Cargo.toml`, `Cargo.lock`, `README.md` and the
   three manuals to `1.0.0-beta.6`; rebuild all three PDFs. The player
   manual also gains two rules it lacked: Join's host lost to civil
   disorder, and the Orders prompt's idle-fleet default.
2. Build the Linux x64 archive on bean-void and the three Windows archives
   on bean with MSVC; each passes `build_release_bundle.py --verify`.
3. By hand on each: new game, two maintenance years, `check`, `map-export`;
   on Linux also a door join through a drop file and a game made with the
   published beta.5 archive opened under beta.6.
4. Sign `SHA256SUMS.txt` with release subkey `57184BB62A800062`, replace
   the beta.5 assets and notes on `release-artifacts`, verify a fresh
   download, and replace the website PDFs.

## Connections

- [Release v1.0.0-beta.5](r5b5nc01-release-v1-0-0-beta-5.md), whose steps
  this follows.
