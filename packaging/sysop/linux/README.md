# Nostrian Conquest Linux Sysop Package

This is the public Linux x64 BBS/sysop package. It contains `bin/nc-door`,
`bin/nc-sysop`, both public PDF manuals, the shipped `docs/sysop/` markdown
guides, `examples/config.kdl`, and `BUILD-INFO.txt`. It does not contain
`bin/nc-game`, and it does not bundle preserved Esterian Conquest
executables, manuals, or DOS helper files.

Use this package when you are hosting Nostrian Conquest as a Linux BBS door.
If you want localhost or direct SSH play, build from source and use
`nc-game`. If you want the VPS/Nostr daemon stack, build from tagged source
and use `scripts/install_vps.sh`.

## Player Guide

Start with [NC Player Quick Start](docs/nc_quick_start.pdf). The full
[player manual](docs/nc_player_manual.pdf) covers the rules and tables.
Both are included with the sysop manual.

## BBS Quick Start

1. Create a game directory and write `config.kdl` first. Start from
   `examples/config.kdl` and adjust any fixed reservations.

2. Initialize the BBS campaign:

```bash
./bin/nc-sysop new-game --bbs /srv/nc/games/night-shift
```

3. If you want a reproducible test map, keep the seed on the command line:

```bash
./bin/nc-sysop new-game --bbs /srv/nc/games/night-shift --seed 1515
```

4. Point your BBS door at `bin/nc-door` with the dropfile path:

```text
./bin/nc-door --dir /srv/nc/games/night-shift --dropfile <dropfile>
```

For working host-specific launch lines, see the bundled Mystic, Synchronet,
ENiGMA½, and WWIV setup guides under `docs/sysop/`.

## Notes

New unreserved BBS callers land on the first-time menu. Reserved callers and
returning callers skip that generic first-time screen. The signed
`SHA256SUMS.txt` manifest on the GitHub release page covers this package.
