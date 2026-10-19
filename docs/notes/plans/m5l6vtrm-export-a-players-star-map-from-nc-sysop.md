---
id: m5l6vtrm
date: 2026-10-19
kind: plan
tags: [plan, sysop, map]
---
# Export a player's star map from nc-sysop

## Scope and exit

Players get their star maps from the in-game export, the website download
after login, and the Discord `/map` command. The sysop had no way to export
one from `nc-sysop`; only the developer tool `nc-cli map-export` could.

**Exit:** `nc-sysop map-export --dir <game_dir> --player <N> [--out-dir <path>]`
writes the player's star map with the in-game export's names and contents
(`ECMAP-P<N>-Y<year>` `.TXT`, `.CSV`, `-DETAILS.CSV`, default folder
`<game_dir>/exports`), refuses a seat outside the game, and the sysop
manual documents it. nc-sysop tests pass.

## Task details

### t209

Export a player's star map from nc-sysop.

## Result (2026-10-19)

`rust/nc-sysop/src/map_export.rs` reuses `nc_data::build_player_map_export_data`
(what the player knows of the galaxy this year) and names the files from the
campaign's current year. `nc_sysop_map_export_writes_the_players_star_map`
in `rust/nc-sysop/tests/basic.rs` checks the default and `--out-dir`
folders, that the text matches the shared export, and the seat range. The
sysop manual has a section under Player Management and a CLI reference line.
