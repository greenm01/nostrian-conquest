# Campaign Settings

This page is the field reference for non-BBS campaign settings.

Use it when you need the raw `nc-sysop settings show` surface. Do not treat
every stored field here as a normal day-to-day sysop knob. The main sysop
manual keeps the operator workflow short on purpose.

## Ownership Boundary

Keep the split straight:

- Hosted / Nostr host and direct `nc-game` campaigns use `ncgame.db`
- BBS door campaigns use a minimal `config.kdl` plus `ncgame.db`
- BBS `config.kdl` supports only `players` and `reservations`
- `nc-sysop settings set` is for non-BBS campaigns only

## Raw `settings show` Output

For a non-BBS campaign, `nc-sysop settings show --dir /path/to/mygame` prints
rows like:

```text
slug=friday-night
game_name=Friday Night NC
default_theme_key=tokyo_night
snoop=true
session_max_idle_minutes=10
session_minimum_time_minutes=0
session_local_timeout=false
session_remote_timeout=true
inactivity_autopilot_after_turns=3
maintenance_enabled=false
maintenance_interval_minutes=1440
maintenance_next_due_unix_seconds=
reservation seat=1 alias=SYSOP
ai_seat seat=3 difficulty=hard personality=land-grab
```

A fresh game starts with maintenance scheduling disabled. The next-due field
stays blank until the sysop enables scheduling and sets the first due time.

For a BBS campaign, the output is different:

```text
mode=bbs
players=4
reservation seat=1 alias=SYSOP
reservation seat=2 alias=NightShade
```

## Main Non-BBS Operator Fields

- `game_name`
  Display name shown in the main menu header. If you omit `--name` at creation
  time, `nc-sysop` derives it from the directory slug.
- `default_theme_key`
  Bundled default color set for non-door sessions. This is a compiled-in key,
  not a file path. Current default: `tokyo_night`.
- `session_max_idle_minutes`
  Hosted sessions use this as the default idle lease timeout in minutes.
  Range: `0-120`. Door-mode sessions (`nc-door`, or `nc-game --dropfile`) also
  end after this many minutes without a keypress; `0` disables that. Direct
  localhost and manual SSH `nc-game` sessions do not currently get a separate
  timeout path from this field alone.
- `maintenance_enabled`
  Controls whether hosted `maint-all` treats the game as scheduled.
- `maintenance_interval_minutes`
  Hosted `maint-all` interval in minutes. Default: `1440`.
- `maintenance_next_due_unix_seconds`
  Next scheduled run as a Unix timestamp. Blank means no due time has been set
  yet.

## Alias Reservations

Non-BBS campaigns can also store alias reservations in `ncgame.db`:

```text
reservation seat=1 alias=SYSOP
```

Treat these as niche routing state. They are useful for controlled alias-based
seat binding. They are not part of the normal Nostr invite flow.

For BBS campaigns, reservations live in `config.kdl` instead.

## AI Seats

A non-BBS campaign can have seats played by the learned AI:

```sh
nc-sysop settings ai-seat --dir /path/to/mygame --player 3 --difficulty hard
nc-sysop settings ai-seat --dir /path/to/mygame --player 4 --difficulty normal --personality rusher
nc-sysop settings remove-ai-seat --dir /path/to/mygame --player 3
```

`--personality` picks the seat's style. The default is `land-grab`.

- `land-grab` expands hard and early, and strikes when it has the odds.
- `rusher` goes to war at once, with few colonists and long-odds attacks.
- `turtle` keeps strong garrisons, batteries and starbases, always guards
  threatened worlds, and attacks late.

Seats can mix styles, so a game has variety.

Difficulty is `easy`, `normal` or `hard`. Hard plays the agent at full
strength. On held-out test maps it won about two games in three against
three scripted executor seats. Normal won about three in five; it still
beats the executor, but is less predictable. Easy won about one in five.

A reserved seat cannot be an AI seat. Before each maintenance year,
`nc-sysop maint` has every AI seat commission its ships and submit its turn,
as a player would. Each seat's memory of what it has seen is kept in
`ncgame.db` between runs.

`maint` needs the agent: set `NC_AI_MODEL` to its `.onnx` file. A campaign
with AI seats refuses maintenance without it. Export the file with
`tools/ai/export_onnx.py`.

Hosted games play AI seats too. `nc-host` maintenance does, for Nostr
(`serve` and `maint`) and community (`admin`) games, so set `NC_AI_MODEL`
in those services' environments. Mark a hosted game's seat with
`nc-sysop settings ai-seat` on its game directory. Community joins skip AI
seats. For a Nostr game, also close the seat to claims with
`nc-host seats close`.

## Advanced Carried-Forward State

These fields still appear in non-BBS `settings show` output because they are
stored in campaign state and mirrored into the runtime snapshot. Do not read
them as first-class operator controls for every deployment mode.

- `snoop`
  Default: `true`. Carried in classic setup state. Not a main hosted or direct
  `nc-game` operator knob today.
- `session_minimum_time_minutes`
  Default: `0`. Stored in state. No separate live minimum-session path is
  documented for current hosted, direct, or BBS operation.
- `session_local_timeout`
  Default: `false`. Stored in state. Do not read this as a separate localhost
  or manual SSH timeout feature today.
- `session_remote_timeout`
  Default: `true`. Stored in state. Hosted sessions currently use
  `session_max_idle_minutes` for lease TTL. Do not treat this flag by itself
  as a separate VPS switch.
- `inactivity_autopilot_after_turns`
  Default: `3`. Stored in state. `0` disables the policy. When maintenance
  sees that an active human player has missed that many consecutive turns, it
  turns autopilot on automatically. The first real return --- interactive
  `nc-game` login or successful `submit-turn` --- turns that inactivity-driven
  autopilot back off again. Not a BBS `config.kdl` feature.

## Maintenance Note

Treat maintenance schedule fields as hosted `maint-all` metadata. They do not
create their own timer. They are not BBS `config.kdl` fields. For a direct
localhost or manual SSH game, schedule `nc-sysop maint /path/to/mygame`
yourself with host tooling.
