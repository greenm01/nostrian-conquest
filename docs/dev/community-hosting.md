# Community hosting

Players find games, claim a seat with Discord, a NIP-07 Nostr signer or Matrix, and play
the existing 80x25 door in a browser. `nc-admin` handles operator work over SSH. There is no web dashboard.
The existing community is <https://discord.gg/eZMRpdQTu>.

The Discord browser and bot services are deployed for acceptance testing.
The shared-identity/NIP-07 update is deployed with the offline account conversion.
Attended nos2x acceptance remains outstanding. HTTPS,
the login redirect and the initial seven guild commands have been verified. The
owner has tested sign-in, empire setup, saved orders and reports after two manual
turns. A fresh invite-only lobby is recruiting, with its owner seat reserved. The turn
and daily-backup timers are enabled; the lobby has no deadline until its owner
starts it. The browser test game has been archived and is no longer listed. Both games have verified
off-server copies and passed isolated paused restores. Two-account attended
acceptance and automated off-server copying remain pending.

## Components

- `nc-host admin` provides local JSON operations and durable turn receipts.
- Python `nc-admin` invokes those operations without editing game databases.
- `nc-bridge --games-root` serves a public game list, Discord/Nostr/Matrix login, authorized
  downloads and terminal sessions. It launches `nc-door` with a server-selected
  seat. Browser query parameters cannot choose an identity or seat.
- `nc-discord` registers commands in one configured Discord server. `/games`,
  `/join`, `/play`, `/status`, `/map`, `/report` and `/guide` use the same host
  operations. All command replies are private. Maps and reports are generated
  for the verified account's seat; the bot never scans a shared export directory.
- `/map` defaults to one ZIP with TXT, grid CSV, known-world-details CSV, ODS and
  XLSX files from one authorized snapshot. The workbooks contain Starmap and
  Known Worlds sheets. Individual Text, Csv, Ods and Excel choices remain
  available. The website provides a single Download map ZIP button. Reports remain in the
  game inbox and `/report`; individual map formats remain available through the bot.
- `nc-matrix run` is the Matrix sign-in bot. It sends one-time sign-in links and
  codes and has no game commands.
- `/help` lists commands; `/about` explains the game; `/manuals` links the
  quick-start and player-manual PDFs. `/guide` remains the quick-start shortcut.
  The [welcome post](discord-welcome.md) is pinned and explains private invite codes.
- The systemd turn timer checks independent game deadlines. The bot delivers
  queued recruitment and completed-turn announcements to a configured channel.

No embedded Discord Activity is required. Private reports stay private;
announcements contain only recruitment facts or the actual completed game year
and next deadline. Delivery retries can duplicate a message if the bot crashes
between sending and recording success. They cannot repeat maintenance.

## Build and stage

Build from `rust/` through the normal cargo-slot wrapper:

```sh
cargo-slot nostrian-conquest cargo build --release -p nc-host -p nc-game -p nc-bridge -p nc-discord -p nc-matrix --bin nc-host --bin nc-door --bin nc-bridge --bin nc-discord --bin nc-matrix
```

Stage these five binaries, the Python zipapp, and `rust/nc-bridge/static/` under
one versioned release directory:

```text
/opt/nc-community/releases/<release>/bin/{nc-host,nc-door,nc-bridge,nc-discord,nc-matrix,nc-admin}
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
`web.env`, `discord.env` and `matrix.env`; systemd reads them before changing to the service
account. Copy the examples from [packaging/community](../../packaging/community).
Never put secrets in git, command arguments, public notes or shell history.
The optional `nc-community-credentials.py` helper can be installed as
`/usr/local/sbin/nc-community-credentials`. Run it through sudo with `bot`, `oauth`,
`matrix` or `matrix-oauth` in an attended terminal. It hides input and updates the corresponding
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

The account provider and verified ID identify a seat. Discord uses its numeric
account ID; Nostr uses the signing public key. The verified account display name
becomes the in-game player handle when the browser terminal opens; the username
is the fallback when no display name is set. Names may change or match another
player's name without changing ownership. The empire name remains the player's
choice. Handles use the existing 25-character game text limits and discard
control characters. A fresh Discord sign-in refreshes the display name;
older sessions without a saved profile must sign in again.

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
Invite-only games require a single-use code through the browser game card or
`/join` in the configured Discord server. `/invite` issues one privately and verifies the server owner
before creating it. Both browser providers and the bot use one invite store. A successful claim
consumes the code globally; it cannot claim or link an identity on another
provider. Returning owners need no new code.
Closed games require an operator replacement, never a public seat selector.

## Nostr browser login

The browser offers **Sign in with Nostr** through `window.nostr.signEvent`, as
provided by nos2x and other NIP-07 extensions. The signer authorizes a NIP-98
POST to `/auth/nostr`; no event is published and no relay is required. Private
keys never reach the host. Discord configuration remains required for this
combined service.

`POST /auth/nostr/challenge` creates a five-minute random challenge tied to an
HttpOnly browser cookie. The signed event must have kind 27235, empty content,
a timestamp within 60 seconds, the exact configured URL and POST method, and a
SHA-256 hash of the raw login body. Signature, event ID, origin, browser cookie
and challenge are verified. Challenge consumption and session creation commit
together. Login bodies contain `challenge` and optional `display_name` only.

Nostr players can enter a player name, retain their saved name by leaving it
blank, or initially use `Nostr-<12 hex characters>`. Names are untrusted labels;
the verified public key determines access. Existing handle sanitization applies.
The login chooser displays the current provider and account before claiming.
After sign-in, only the identity line and Sign out remain. Each game card checks
the current account: Join and invite entry for an open recruiting seat, Play
and downloads for an existing seat, or an explanation when play is unavailable.
Reconnect appears only when that seat already has a terminal open. Returning
from the terminal refreshes the listing, including browser Back navigation.
`GET /api/games/{game}/access` requires a session and reports nullable `seat`,
`can_play`, `state`, and `connected` for that account only. A null seat conveys
no terminal or export permission.

`POST /api/games/{game}/join` accepts `{ "code": "..." }`; identity always comes
from the authenticated session. `/api/me` returns `account` (provider and ID)
and `display_name`. Local host requests now use `account` instead of
`discord_id`; ship host, bridge, bot and Python client together.

See the [operator identity and conversion instructions](../../tools/nc-admin/README.md#operator-account-access)
for restricted account administration and the offline upgrade. These controls
are not exposed in the player UI or Discord commands.

The public `/howto` and `/contact` pages are available without signing in. They
share the listing's Tokyo Night Storm colors through `community.css`; the terminal
keeps its game palette. Help links to the official nos2x listing and Alby extension
and Nostr guide. Contact shows the operator-supplied Discord username/profile and
Nostr public key, with clipboard controls and manual-copy fallback. No contact
message is sent by the website.

## Matrix sign-in

**Sign in with Matrix** is one-click OAuth for matrix.org accounts.
`/auth/matrix/login` redirects to `https://account.matrix.org/authorize` with
only the `openid` scope, a five-minute single-use state bound to an HttpOnly
cookie, and an S256 PKCE challenge. `/auth/matrix/callback` redeems the code,
reads `username` from userinfo, and signs in `@username:matrix.org`. It takes
the public display name as the player handle, falling back to the Matrix ID,
then revokes the access token. No Matrix API scope is requested, so the site
never gains access to messages, rooms or the account. matrix.org shows its
normal sign-in page when needed and no separate consent screen.

The OAuth client was registered once through matrix.org's dynamic registration
endpoint as a confidential web client: `client_uri` is the public origin and
the only redirect URI is `<origin>/auth/matrix/callback`. Put its client ID in
`web.env` as `NC_MATRIX_OAUTH_CLIENT_ID` and enter its secret with
`sudo nc-community-credentials matrix-oauth`. Without both, the button's route
returns 404 and players use the bot.

Accounts on other homeservers choose **Matrix account not on matrix.org?**,
which opens a code form. The player messages the bot, or sends
`!login` in the community room. The bot reads only the sender's Matrix ID, which
stays visible in encrypted rooms, and declines every invitation. It replies in
an unencrypted private room it creates with that user. The reply holds a link
whose secret is in the URL fragment, and an eight-character code. The games page
posts either to `/auth/matrix` as `{ "code": "..." }`. Fragments never reach a
server, so URL previews and access logs cannot use the link.

The bot stores only hashes in `.web-sessions.db`; the bridge consumes the code
in the transaction that creates the session. A code or link works once, expires
after ten minutes, and is replaced by a newer request. Requests from one user
less than 30 seconds apart are ignored. A wrong code cannot be traced to a
pending one, so 100 failed redemptions within ten minutes pause Matrix sign-in
site-wide for the rest of that window. Discord and Nostr are unaffected. The
first sync after start only rebuilds room state; old messages and invitations
never produce codes. The Matrix display name becomes the player handle, with
the Matrix ID as the fallback.

The bot needs its own account; never use an operator's personal account.
On a workstation, create its access token once with the bot's password:

```sh
nc-matrix login --user @nostrian-conquest-bot:matrix.org
```

The password is read without echo and the token is printed once. Enter it on
the server with `sudo nc-community-credentials matrix`. The password login
uses matrix.org's compatibility API and gives a token that does not expire;
tokens copied from Element expire. Log the bot out of Element afterward if
desired; that does not end this token's session. Set the homeserver to the
client API base URL (`https://matrix-client.matrix.org` for matrix.org) and
the room to the community room alias. The bot joins that room at start.
The site links to the community space `#nostrian:matrix.org`, and the contact
page lists the operator's Matrix ID.

## Services and browser behavior

Review the sample units before installing them. The bridge binds loopback port
8082, behind Caddy with TLS and WebSocket proxying. Do not expose the legacy
single-game `/ws/connect` bridge or herald's development auth/event endpoints.
The managed router does not mount them. Keep credentials out of proxy logs;
do not log OAuth callback query strings.

After setting credentials, start the browser and bot services. Keep the turn
timer disabled until the private pilot passes. Games start unscheduled. Choose a fixed interval,
daily midnight in an IANA time zone, or manual turns. A daily calendar follows
daylight saving; use `America/New_York` for midnight Eastern. Recruitment never
starts maintenance, even when the game is full. An operator must start the game. Running `maintenance due` is the scheduler's only
operation. Two missed deadlines pause a game instead of silently catching up.

One identity can access one seat in each game; one terminal may occupy that seat,
including when the operator has enabled both providers for it.
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

Before deploying this update, exercise two isolated games with real Discord and
nos2x sign-ins: simultaneous orders; claim retry; wrong-account map/report rejection;
reconnect and logout; live maintenance; independent deadlines; pause/resume;
backup/restore; a killed turn before and after its receipt; replacement; and
child cleanup after terminal disconnect. On an attended browser, check login,
box-drawing, 80x25 menus, keyboard input, downloads and return to the game list.
A synthetic session test does not replace real Discord OAuth or nos2x extension
acceptance. Validate signed login, signing rejection, switching keys, single-use
invites across providers, and operator unlink revocation as well.

## Website PDF viewing

The public Manuals page links to `/manuals/nc_quick_start.pdf`,
`/manuals/nc_player_manual.pdf` and `/manuals/nc_sysop_manual.pdf` on the website.
Publish the PDFs from `docs/manuals/` into the web root's `manuals/` directory
whenever they are rebuilt; do not keep a second generated copy in the repository.
Serve them as `application/pdf` without an attachment disposition. These links
open a new tab and let the browser's PDF viewer display the document. A user's
browser setting to always download PDFs takes precedence. GitHub raw-file links
may force a download and are not used on this page.
