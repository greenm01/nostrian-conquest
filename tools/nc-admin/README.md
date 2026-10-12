# nc-admin

Python 3.10+ administration for multiple Nostrian Conquest games. It uses only
the standard library. Rust `nc-host admin` owns every game-state mutation.
Run from the repository with `scripts/nc-admin`, or build an executable zipapp:

```sh
python3 tools/nc-admin/build.py /tmp/nc-admin
/tmp/nc-admin --help
```

`NC_GAMES_ROOT` selects the managed games directory. `NC_HOST_BINARY` selects
`nc-host`. Both have corresponding command-line options. Keep community games
in their own root, outside an existing BBS/Nostr maintenance configuration.

```sh
nc-admin games create practice --players 4 --name 'Practice' --dry-run --json
nc-admin games create practice --players 4 --name 'Practice'
nc-admin games publish practice
nc-admin games list --json
nc-admin games inspect practice --json
nc-admin games start practice --first-due 2026-11-01T04:00:00Z --interval-minutes 1440
nc-admin games pause practice
nc-admin games resume practice --first-due 2026-11-02T04:00:00Z
nc-admin maintenance run practice --expect-year 3000 --request-id practice-year-3000
nc-admin games history practice
nc-admin backup daily
nc-admin backup create practice
nc-admin backup verify /path/to/backup
nc-admin backup restore /path/to/backup --into-root /path/to/isolated-restore --confirm practice
nc-admin doctor --json
```

Replace example dates with future UTC dates. Each game has its own deadline;
`nc-admin maintenance due` checks all of them. Multiple missed deadlines pause
a game for operator review. A timeout is an unknown outcome: inspect history,
then retry with the **same** request ID and expected year. A completed receipt
returns the previous result without running another turn.

`--dry-run` prints the proposed request without invoking Rust. It is a preview,
not a validation against current game state. `--json` prints versioned results.
Exit status is nonzero for failures. Local filesystem access is the operator
credential; never expose the admin protocol as an unauthenticated HTTP route.

Games begin in draft, with writes and scheduling disabled. Publishing opens
recruitment and lets claimed players prepare orders before the first turn.
Starting closes recruitment. Pause blocks writes; resume requires a new date.
Archive hides a paused game without deleting its database or reports.
A practice game uses the same rules and does not recycle player empires.

Replacement preserves the empire and revokes the prior account's seat:

```sh
nc-admin games replace practice --seat 2 --discord-id 123456789012345678 --confirm practice
nc-admin games archive practice --confirm practice
```

Backups use SQLite's backup API while managed writes are blocked. Both databases
and a manifest with hashes form one backup. Restore checks integrity and hashes,
refuses an existing destination, and leaves the restored game paused. Copies
contain private player data; keep permissions restrictive. A manifest detects
corruption; it is not a signature against a malicious operator.

Tests:

```sh
python3 -m unittest discover -s tools/nc-admin/tests
```

Set `PYTHONPATH` to `tools/nc-admin` if running the tests outside their package.
See [community hosting](../../docs/dev/community-hosting.md) for server setup,
Discord installation, acceptance checks and recovery.
