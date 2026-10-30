# Game history backups

Played games are kept for history and as future AI training data. Every
saved copy of a game database uses one file name format, so a copy can be
traced to its server, game and day without opening it.

## File name format

```
<server>-<game>-<YYYY-MM-DD>.db
```

- `<server>` is the host that runs the game: `crunch` or `archvps`.
- `<game>` is the game's directory name under the server's games root, such as
  `ai` for `~/bbs/nc-testing/games/ai/ncgame.db`.
- `<YYYY-MM-DD>` is the local date the copy was taken (`date +%F`).

For example, `crunch-ai-2026-10-30.db` is the crunch lab's `ai` game as it
stood on 30 October 2026. Use lowercase, and hyphens only. A game name holds
no hyphen-separated date of its own, so the last three fields are always the
date.

A copy taken twice on one day replaces that day's file. Each database holds
every year's snapshot, so the latest copy of a game is its full history; the
dated copies keep a game that is later reset or deleted.

## What a copy holds for AI training

Each database keeps one snapshot per year. A door player's orders are
written into that year's snapshot, so a stored year is the position with
the orders its people gave. AI seats order during maintenance, and since
2026-10-30 maintenance logs their turns in the `ai_turns` table (only in
games with AI seats). With them, `nc_ai::opening_position` replays the year
before and rebuilds the position as maintenance left it, before anyone gave
orders. Games played before then can seed training positions but cannot
separate their people's orders from the position.

## Taking a copy

Copy a live database with sqlite's online backup, never with `cp`, so a game
mid-turn copies whole:

```
sqlite3 <games-root>/<game>/ncgame.db ".backup /tmp/<server>-<game>-<YYYY-MM-DD>.db"
```

## Where copies live

On the Mac mini, `~/.local/bin/nc-ai-backup` copies every crunch game to
`~/.local/state/nostrian-conquest/game-history/` and sends that directory to
the AI model repositories on archvps and crunch
(`sftp:archvps:backups/nc-ai` and `sftp:crunch:backups/nc-ai`, tag
`nc-games`; 7 daily, 8 weekly and 24 monthly snapshots). The local directory
is never pruned. The script is not scheduled; run it after a session of play
and after each model promotion.

The community server's own backups have their own naming and job; see
[community hosting](community-hosting.md).

To recover a game:

```
restic -r sftp:crunch:backups/nc-ai restore latest --tag nc-games --target /tmp/nc-restore
```
