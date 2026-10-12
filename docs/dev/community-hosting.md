# Community hosting

Players find games in Discord, claim a seat, and play the existing 80x25 door in
a browser. `nc-admin` handles operator work over SSH. There is no web dashboard.
The existing community is <https://discord.gg/eZMRpdQTu>.

This implementation is under local validation. Do not call it a deployed or
accepted service until Discord login and the private two-game pilot pass.

## Components

- `nc-host admin` provides local JSON operations and durable turn receipts.
- Python `nc-admin` invokes those operations without editing game databases.
- `nc-bridge --games-root` serves a public game list, Discord login, authorized
  downloads and terminal sessions. It launches `nc-door` with a server-selected
  seat. Browser query parameters cannot choose an identity or seat.
- `nc-discord` registers commands in one configured Discord server. `/games`,
  `/join`, `/play`, `/status`, `/map`, `/report` and `/guide` use the same host
  operations. All command replies are private. Maps and reports are generated
  for the verified account's seat; the bot never scans a shared export directory.
- The systemd turn timer checks independent game deadlines. The bot delivers
  queued recruitment and completed-turn announcements to a configured channel.

No embedded Discord Activity is required. Private reports stay private;
announcements contain only recruitment facts or the actual completed game year
and next deadline. Delivery retries can duplicate a message if the bot crashes
between sending and recording success. They cannot repeat maintenance.

## Build and stage

Build from `rust/` through the normal cargo-slot wrapper:

```sh
cargo-slot nostrian-conquest cargo build --release -p nc-host -p nc-game -p nc-bridge -p nc-discord --bin nc-host --bin nc-door --bin nc-bridge --bin nc-discord
```

Stage these four binaries, the Python zipapp, and `rust/nc-bridge/static/` under
one versioned release directory:

```text
/opt/nc-community/releases/<release>/bin/{nc-host,nc-door,nc-bridge,nc-discord,nc-admin}
/opt/nc-community/releases/<release>/static/
/opt/nc-community/current -> releases/<release>
```

Use verified release hashes and signatures. Keep the previous release for
rollback; do not copy a new binary over a running release. No game data lives in
the release directory. The repository's existing beta.3 BBS archives do not
include this new hosting stack.

Create a dedicated `nccommunity` service account and a mode-0700
`/srv/nc/community/games` owned by it. Keep this root separate from existing
BBS/Nostr game roots and timers. Root-owned `/etc/nc-community/` holds mode-0600
`web.env` and `discord.env`; systemd reads them before changing to the service
account. Copy the examples from [packaging/community](../../packaging/community).
Never put secrets in git, command arguments, public notes or shell history.
The optional `nc-community-credentials.py` helper can be installed as
`/usr/local/sbin/nc-community-credentials`. Run it through sudo with `bot` or
`oauth` in an attended terminal. It hides input and updates the corresponding
root-only environment file without printing the credential. Merge non-secret
settings into these files; never overwrite entered credentials with an example.

Use one bridge instance per games root. A process lock rejects a second one.
The service's control group owns its door children. Stop the whole service for
an upgrade, so no old child can keep writing with the prior executable.

## Discord setup

Use one Discord developer application for browser sign-in and the bot. Add
`https://<play-host>/auth/callback` as its exact OAuth redirect. Browser login
requests only `identify`; it does not need email or member-list access. The
server exchanges the code, verifies the account with Discord, and stores a
hashed random session cookie. OAuth state expires after five minutes and is
single use. Cookies are HttpOnly, Secure over HTTPS, and SameSite=Lax.

Install the bot into the **existing** community with `bot` and
`applications.commands` scopes. It needs View Channel, Send Messages and Attach
Files in the selected channels, not Administrator or Message Content intent.
Set the numeric guild ID; choose an announcement channel explicitly. Leaving
`NC_DISCORD_CHANNEL_ID` absent keeps announcement delivery disabled. No Discord
credentials are inferred from the public invite.

See Discord's [OAuth2 reference](https://docs.discord.com/developers/topics/oauth2)
and [interaction reference](https://docs.discord.com/developers/interactions/receiving-and-responding).
Commands register within the configured server. The browser lets any authenticated
Discord account claim an open recruiting seat; server membership is not checked.
Closed games require an operator replacement, never a public seat selector.

## Services and browser behavior

Review the sample units before installing them. The bridge binds loopback port
8082, behind Caddy with TLS and WebSocket proxying. Do not expose the legacy
single-game `/ws/connect` bridge or herald's development auth/event endpoints.
The managed router does not mount them. Keep credentials out of proxy logs;
do not log OAuth callback query strings.

After setting credentials, start the browser and bot services. Keep the turn
timer disabled until the private pilot passes. Deadlines default to daily UTC,
but games start unscheduled. Running `maintenance due` is the scheduler's only
operation. Two missed deadlines pause a game instead of silently catching up.

One account can own one seat in each game; one terminal may occupy that seat.
Reconnect closes the older session explicitly. Sign-out, expiry, replacement,
pause or maintenance ends its authorization; active sessions check before input
and every five seconds. A terminal has a thirty-minute maximum and a five-minute
idle timeout. It remains 80x25. The header links back to the game list.

Same-year stale saves fail and reload instead of overwriting another player's
orders. A player must retry the rejected action. A new-year stale session must
log in again. A paused action is discarded, not saved later on resume.

## Recovery and acceptance

[Operator commands](../../tools/nc-admin/README.md) include JSON previews, history,
backup verification and restoration into an unused directory. Retain 30 daily
backups and a pre-upgrade backup, and copy them off the server over encrypted
transport. The daily backup timer retains 30 daily copies; maintenance retains 30 pre-turn
copies per game. Manual/pre-upgrade copies are never pruned automatically.
Off-server automation must be configured and checked before public launch;
local snapshots alone are not disaster recovery.

If a turn stops with state `maintenance`, use `nc-admin games inspect GAME --json`
to find its request ID in `unfinished_runs` before retrying the same ID and expected year.
The runtime receipt commits with the new year. Recovery finishes metadata if
that receipt exists, otherwise it can resume the interrupted operation. Do not
invent a new request ID to bypass an unresolved turn. Failed runs remain paused.

For a binary rollback, first stop the timer and both services and make a backup.
Never load a newer incompatible database with an older binary. Restore a matching
backup into an unused root and verify it while paused before switching services.
Classic `.DAT` exports are not backups of modern runtime state or account bindings.

Before public recruitment, exercise two isolated games with two real Discord
accounts: simultaneous orders; claim retry; wrong-account map/report rejection;
reconnect and logout; live maintenance; independent deadlines; pause/resume;
backup/restore; a killed turn before and after its receipt; replacement; and
child cleanup after terminal disconnect. On an attended browser, check login,
box-drawing, 80x25 menus, keyboard input, downloads and return to the game list.
A synthetic session test does not replace the real Discord login check.
