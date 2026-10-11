# Nostrian Conquest Windows Sysop Package

This is the public Windows BBS/sysop package. GitHub Releases may publish it
in three variants: Windows x64, Windows x86 (32-bit), and Windows 7+ x86
(32-bit legacy). It contains `nc-door.exe`, `nc-sysop.exe`, both public PDF
manuals, the shipped `docs/sysop/` markdown guides, `config.kdl`, and
`BUILD-INFO.txt`. It does not contain `nc-game.exe`, and it does not bundle
preserved Esterian Conquest executables, manuals, or DOS helper files.

Use this package when you are hosting Nostrian Conquest as a native Windows
BBS door. If you want localhost or direct console play on Windows, build from
source and use `nc-game.exe`. If you want the VPS/Nostr daemon stack, build
from tagged source on Linux and use `scripts/install_vps.sh`.

## Player Guide

Start with [NC Player Quick Start](nc_quick_start.pdf). The full
[player manual](nc_player_manual.pdf) covers the rules and tables.
Both are included with the sysop manual.

## BBS Quick Start

1. Choose a game directory and copy `config.kdl` into it. Adjust `players`
   and any `reservations`, then initialize the BBS campaign:

```text
nc-sysop.exe new-game --bbs C:\nc\games\night-shift
```

2. If you want a reproducible test map, keep the seed on the command line:

```text
nc-sysop.exe new-game --bbs C:\nc\games\night-shift --seed 1515
```

3. Point the BBS door at `nc-door.exe` with the dropfile path:

```text
nc-door.exe --dir C:\nc\games\night-shift --dropfile <dropfile>
```

4. On native Windows Synchronet, pass the inherited socket descriptor too:

```text
nc-door.exe --dir C:\nc\games\night-shift --dropfile %f --socket-descriptor %H
```

For working host-specific entries, see the bundled Mystic, Synchronet,
ENiGMA½, and WWIV setup guides under `docs/sysop/`.

## Notes

New unreserved BBS callers land on the first-time menu. Reserved callers and
returning callers skip that generic first-time screen. The signed
`SHA256SUMS.txt` manifest on the GitHub release page covers this package.

If your machine and BBS stack are already 64-bit, use the Windows x64
archive. If you need a normal 32-bit build for newer Windows, use the
Windows x86 archive. If you are keeping a legacy 32-bit Windows 7 host alive,
use the Windows 7+ x86 archive.
