---
id: xda0z587
date: 2026-10-24
kind: adr
status: accepted
tags: [adr, sysop, config]
---
# Each game keeps its sysop settings in config.kdl

## Context

Classic sysops set program options in ECUTIL (F4, "Setup The Programs"),
including "Autopilot any empires inactive for" N turns. In March 2026
hosted games became database-only (`bb1a380d`); in April, BBS games got
back a `config.kdl` limited to `players` and `reservations`
(`4b9e7f73`). Since then the autopilot threshold, session timeouts, snoop
and the maintenance schedule live only in the campaign database, changed
with `nc-sysop settings set`. The template `rust/nc-data/config/config.kdl`
still shows the old layout, but nothing reads it.

The owner found a fleet put on autopilot in a test game and wanted the
threshold to be a sysop setting in each game's KDL config, with 3 as the
default.

## Decision

Owner, 2026-10-24:

- Every game directory has a `config.kdl` holding the sysop's settings:
  game name, theme, snoop, session timeouts, the inactivity autopilot
  threshold (default 3, 0 turns it off), the maintenance schedule, seat
  reservations and AI seats.
- The number of players and the map's seed and size are locked once the
  game is created. `new-game` sets them (`--players`, `--seed`, also with
  `--bbs`) and they are not settings in the file: a `players`, `seed` or
  `map_size` field is refused. The file opens with a comment showing them
  for reference. Old files with `players` are not accepted; there is no
  backward compatibility.
- `inactivity.autopilot_after_turns` stays changeable, in case it was set
  wrongly.
- When a sync changes settings players would notice (game name, autopilot
  rule, session limits, maintenance schedule, AI seats), every empire gets
  one "Game Notice" report listing them. Theme, snoop and reservations are
  not announced.
- The other settings change freely; reservations and AI seats only for
  seats no player holds.
- Autopilot keeps switching on in the maintenance of the Nth missed year
  (with N = 3 and orders given in 3000, during 3003's maintenance).
- The database keeps runtime state: the next maintenance time, player
  activity and the game itself. It copies the file's settings at each
  login and maintenance run.

## Alternatives

- Keep the settings database-only and document `nc-sysop settings set`.
  Rejected: the owner wants the classic sysop model of an editable
  per-game configuration.
- Lock the autopilot threshold too. Rejected: a wrong value set at
  creation needs a fix.

## Consequences

This reverses the database-only rule of `bb1a380d` for hosted games: each
hosted game directory gets a `config.kdl`. `nc-sysop settings set` edits
the file. Work is tracked as t218.

## Acceptance and connections

Accepted by the owner on 2026-10-24. The sysop manual carries the
settings reference (section "Game settings: `config.kdl`").

## Result

Landed in `8f9c7e1b` on 2026-10-24. `new-game` (nc-sysop and nc-host)
writes the file and now stores its seed as the campaign seed; logins,
maintenance and the `settings` commands sync it. Validation: nc-data
`bbs_config` (17), nc-sysop `basic` (15), nc-game lib and nc-host
`new_game` tests pass, and an end-to-end `new-game --bbs --players 6
--seed 42` wrote the reference comment and refused a `players` field.
Remaining: games created before t218 (beta-1 on archvps) have no
`config.kdl` and keep their stored settings; nothing was deployed.
