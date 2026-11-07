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

## Release (2026-11-07)

The first Linux build from `ab1ab6ef` showed a bug present since beta.5:
`nc-sysop new-game` and `maint` resolved a relative game folder that did
not exist yet against the build machine's source tree, so `new-game g1`
wrote `/home/niltempus/dev/nostrian-conquest/g1`. `9c073ed1` takes relative
folders from where `nc-sysop` runs, with a regression test, and all four
archives were rebuilt from it.

The archives: Linux x64 on bean-void (rust 1.98.0), the three Windows targets
on bean (rust 1.99.0, nightly for the x86 standard library builds). Each passed
`build_release_bundle.py --verify`. By hand, from the unpacked archives:

- every archive: new game in a relative folder, two maintenance years,
  `check` and `map-export`;
- Linux: a BBS game joined through a DOOR32.SYS drop file (empire named,
  seat bound to the caller's handle, homeworld named, main menu reached),
  then a maintenance year and `check`; and a BBS game made with the
  published beta.5 archive, run one year, then checked and run two years
  under beta.6.

The door join was driven through a pty, as bean-void has no tmux. The
Windows door join was not run. Defender found no threats in the three zips.

| Archive suffix | SHA-256 |
| --- | --- |
| `linux-x64.tar.gz` | `90e5a871456e1ec9d148e448954cb2004ec44a037555823dddf6a7bcec605619` |
| `windows-x64.zip` | `75fb43e45c796738db1350e6863a5922db335d980b956bf19599fdc376801507` |
| `windows-x86.zip` | `9f541d17aa51654372abadfd73d7341cac357d5f56f15bf7fb0be1489a2be496` |
| `windows7-x86.zip` | `51c1756b7624ee23e30efb2a13ea2028f32d243272289019c713a2592659badb` |

`SHA256SUMS.txt` is signed with release subkey `57184BB62A800062`. The
beta.6 archives and signed manifest replaced beta.5 on `release-artifacts`
with new notes; a fresh download passed GPG and checksum verification. The
website's three manual PDFs were replaced and match the commit byte for
byte, served as `application/pdf`. The website names no version, so no
page changed.

bean's GitHub SSH key needs an interactive unlock there, so bean was
fast-forwarded over HTTPS from the public repository.

## Connections

- [Release v1.0.0-beta.5](r5b5nc01-release-v1-0-0-beta-5.md), whose steps
  this follows.
