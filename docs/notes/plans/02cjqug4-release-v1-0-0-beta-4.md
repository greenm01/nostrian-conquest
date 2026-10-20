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
