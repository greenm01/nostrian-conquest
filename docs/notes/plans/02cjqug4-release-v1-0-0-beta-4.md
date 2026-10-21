---
id: 02cjqug4
date: 2026-10-19
kind: plan
tags: [plan, release]
---
# Release v1.0.0-beta.4

## Scope and exit

Ship what landed after the beta.3 archives (built from `ea1a6b3b`) as
`v1.0.0-beta.4`: door renders that send only screen changes (t208); the
nc-sysop `map-export`, `economy-report` and `check` commands (t209); the
first-login rename fix; and the executor's mixed hulls (t189) and ferry
(t191). The save format is unchanged since beta.3.

The owner tests beta.4 on the live website's Beta 1 game before any GitHub
package is built. **Exit:** the archvps community stack runs a signed
beta.4 release and the owner accepts it on Beta 1; then the Linux and three
Windows `nc-sysop` archives and a signed `SHA256SUMS.txt` replace beta.3 on
`release-artifacts`, and `website/landing/downloads.html` names beta.4.

## Task details

### t210

1. Bump every workspace `Cargo.toml`, `Cargo.lock`, `README.md` and the
   three manuals to `1.0.0-beta.4`; rebuild all three PDFs.
2. Build and stage the community stack on bean-void, sign the tarball on the
   Mac mini, deploy it on archvps after an `nc-admin` backup of Beta 1,
   keeping the previous release for rollback.
3. Owner acceptance on Beta 1, including door rendering (t208) in the
   browser terminal.
4. Then the BBS archives as in t023/t024, the download page and the website
   PDFs.

## Live test release (2026-10-19)

Signed community release `19fb1f5d` (beta.4) built on bean-void (rust
1.98.0), tarball SHA-256
`39d5638c4d5a0280053a12fad181da3202ecba71587bda1fcd082379818938f5`, went live
on archvps after an `nc-admin` backup of Beta 1
(`.backups/beta-1-1792453771-a72197954e0cb14c`). Beta 1 stays recruiting at
year 3000 with one seat claimed. All five units restarted on the new
binaries; `ea1a6b3b` is kept for rollback. The deploy helper now renames the
tarball's `nc-community-<commit>` folder to `releases/<commit>`; the first
attempt stopped before any service was touched. Next: owner acceptance on
Beta 1.

Owner acceptance on Beta 1 (2026-10-19): door rendering in the browser
terminal is correct, fast and responsive (t208); map downloads from the game
card work.

## Release (2026-10-19)

The four `nc-sysop` archives were built from `4abc2140`: Linux x64 on
bean-void (rust 1.98.0), the three Windows targets on bean with MSVC. Each
passed `build_release_bundle.py --verify`, then by hand: new game, two
maintenance years, `check`, `map-export` and a full door join through a
DOOR32.SYS drop file (Windows doors with keys from a file). Windows Defender
found no threats in the three zips. A campaign made with the published beta.3
Linux archive opened, played two years and passed `check` under beta.4.

| Archive suffix | SHA-256 |
| --- | --- |
| `linux-x64.tar.gz` | `c6af1f130ee12091d4629ae1710a3d9fe447d2b533d6b23090515af1e6973d9b` |
| `windows-x64.zip` | `b4edd4c22a2abbdf4359f87c5a63b3149fb9830a1e31a19429b27d9d96461d98` |
| `windows-x86.zip` | `775ce4d3a5eeb3b65905c15efb8b0e4420a5e995b151e8a2c8817fa91722fe61` |
| `windows7-x86.zip` | `7ec6046a2f8c5606dfcdf54541b02e649411a8c7c3bc6b74f8d45c8a9ed29e09` |

`SHA256SUMS.txt` is signed with release subkey `57184BB62A800062`. The beta.4
archives and signed manifest replaced the beta.3 archives on
`release-artifacts`, with new release notes; a fresh download of all six
assets passed GPG and checksum verification. The website's three manual
PDFs match the committed beta.4 builds. The download page names no version,
so it needed no change. Not tested on Windows 7 itself.

## Live update (2026-10-20)

Signed community release `7444c586` built on bean-void (rust 1.98.0),
tarball SHA-256
`6cbc5664f9cd04eb5877b91da418b0ed66bb2567845cf4ee5626d4760b1af73c`, went live
on archvps after an `nc-admin` backup of Beta 1
(`.backups/beta-1-1792542385-9f174ba0c63f7b94`). It adds Blitz for AI seats
(t207) and bombardment intel for the attacker (t194). Beta 1 stays
recruiting at year 3000. All five units restarted; `19fb1f5d` is kept for
rollback.
