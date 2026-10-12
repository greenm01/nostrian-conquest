# nc-admin

Python 3.10+ administration for multiple Nostrian Conquest games. It uses only
the standard library. Rust `nc-host admin` owns every game-state mutation.
Run from the repository with `scripts/nc-admin`, or build an executable zipapp:

```sh
python3 tools/nc-admin/build.py /tmp/nc-admin
/tmp/nc-admin --help
```

On the community server, install `packaging/community/nc-admin-wrapper` as the
root-owned executable `/usr/local/bin/nc-admin`. It uses sudo to run as the game
service account with the correct binary and root. Operators retain their usual
sudo authorization. Then administer over SSH, for example:

```sh
ssh YOUR_SERVER nc-admin invites create beta-1
ssh YOUR_SERVER nc-admin invites list beta-1
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
Starting closes recruitment. Filling every seat never starts the game. Pause blocks writes; resume uses the configured calendar or an explicit new deadline.
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

## Returning to a recruiting game

A recruiting game waits as long as the host needs. There is no recruitment
countdown. Players can claim seats, name their empires and prepare orders.
Neither filling the game nor opening **Play** starts maintenance. Invite codes
remain usable until claimed or revoked, while recruitment stays open.

To check progress, use `/status game:beta-1` in Discord or:

```sh
ssh YOUR_SERVER nc-admin games inspect beta-1
ssh YOUR_SERVER nc-admin invites list beta-1
```

The waiting state is `recruiting`, with `next_due: null`. The daily timer skips
it. A configured midnight schedule only becomes active when the host starts
the game. Currently the launch control is the CLI `games start` command; there
is no browser or Discord **Launch** button. **Play** only opens the terminal.

For the current Beta 1 lobby, the owner has explicitly reserved the launch
decision while finding players. Agents must not start it, resume it into running
state, or advance a turn as part of setup, deployment or acceptance testing.
Wait for the owner's explicit launch action or instruction.

## Private invites and local midnight

Set invite-only **before** publishing. A game stays in recruitment until the
operator starts it. Players may prepare orders while they wait.

```sh
nc-admin games create beta-1 --players 4 --name 'Beta 1'
nc-admin games invite-only beta-1
nc-admin games schedule beta-1 --midnight America/New_York
nc-admin games publish beta-1
nc-admin invites create beta-1 --count 4
nc-admin invites list beta-1
nc-admin invites revoke beta-1 INVITE_ID
```

Generate only as many codes as there are unclaimed, unreserved seats. Each code
reserves one seat and appears only in the create response. Codes have eight
random letters or numbers, omit `0 O 1 I`, and ignore letter case. Store or share
them privately; the server stores hashes, not recoverable codes. If a code is
lost, revoke its invite ID and create another. Listing shows unused, claimed or
revoked status without revealing codes. Revoking a code never removes a player.
An operator replacement revokes any unused code for that seat.

The Discord server owner can also use `/invite game:beta-1` for one code in a
private reply. The friend uses `/join game:beta-1 code:YOURCODE` and then `/play
game:beta-1`. The authenticated Discord ID owns the seat; the code is not a login
password. Failed claims are limited to five per account and 100 per game in ten
minutes. The browser cannot bypass the invite requirement.

**Only when you are ready to launch**, run:

```sh
nc-admin games start beta-1
```

`games start` closes recruitment and sets the first deadline to the next local midnight. The daily
schedule follows daylight saving, including 23-hour and 25-hour days. Configure
it before starting; setting it on a running game replaces its next deadline.
`games resume beta-1` resumes the same schedule. An early manual turn keeps the
upcoming deadline. Fixed intervals remain available with `--first-due` and
`--interval-minutes`; those are elapsed-time schedules, not local-midnight ones.

For an operator-driven test game with no automatic deadline:

```sh
nc-admin games schedule practice --manual
nc-admin games start practice
nc-admin maintenance run practice --expect-year 3000 --request-id practice-year-3000
```

The global timer skips manual games and games still recruiting. Configuring a
schedule does not start a draft, recruiting or paused game. Changes to a running
game take effect immediately. Starting and resuming are operator commands, not
player bot commands.
