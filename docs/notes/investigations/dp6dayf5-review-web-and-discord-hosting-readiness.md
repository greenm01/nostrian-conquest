---
id: dp6dayf5
date: 2026-10-11
kind: investigation
status: resolved
tags: [investigation, hosting, discord]
---
# Review web and Discord hosting readiness

The crates provide a working transport skeleton, but public hosting is not ready.
The bridge trusts browser-supplied identities. Private Discord commands do not
bind files to the requesting player. Fix those boundaries before exposing a
campaign. This is a source review of `d655c585`, not a live Discord acceptance
test. No application code or running service was changed.

## t116

Review the bridge, herald, bot and deployment runbook for a real campaign.
The review is complete. At that commit, the fixes below were unimplemented.
The [community hosting plan](../plans/qv5x08zs-discord-community-hosting-and-python-game-administration.md)
tracks implementation and acceptance. An ephemeral
Discord reply hides a message from other viewers. It does not establish that
the recipient is entitled to the attachment.

## Findings, in priority order

### 1. Caller identity is unauthenticated — public deployment blocker

[server.rs](../../../rust/nc-bridge/src/server.rs), `ConnectQuery` and
`handle_socket`, accepts `handle`, `platform_id` and `display_name` from the
WebSocket URL. There is no authenticated session check before node allocation
or process launch. [lib.rs](../../../rust/nc-bridge/src/lib.rs),
`prepare_session`, writes the supplied handle into DOOR32.SYS. The actual
process receives that file, not the richer DOOR.KDL written alongside it.

The shared [launch binding](../../../rust/nc-session/src/launch.rs) deliberately
trusts a BBS caller alias: it selects the reserved seat or existing stored
handle. That is valid behind a trusted BBS login. The bridge currently extends
that trust to arbitrary browser input. A supplied alias matching an existing
player can therefore select that player's seat. A Discord ID string in the URL
does not authenticate its sender.

Require a server-verified login and map a stable platform identity to a campaign
seat. Derive door credentials on the server. Reject unauthenticated WebSocket
upgrades before allocating a node. Do not rely on display names for ownership.

### 2. Private exports are not scoped to the requester — public deployment blocker

[nc-discord/lib.rs](../../../rust/nc-discord/src/lib.rs), `map` and `report`,
read the Discord author ID only for logging. Both scan a shared campaign
directory. `map` selects whichever matching CSV and TXT files it encounters
last; `report` takes the first matching report. Neither filters by seat, year
or authenticated ownership. Different players can receive the same private
export, and CSV/TXT files can come from different players or turns.

The game exports seat and year in filenames, as shown by
[export_starmap](../../../rust/nc-game/src/domains/starmap/controller.rs).
It also emits a separate details CSV, which the bot's broad CSV match can select
in place of the grid. Resolve the authenticated seat first, then select exact
files from one known export generation. Deny unmapped users. Test two players,
two years and both CSV variants. A reply's ephemeral flag cannot replace this.

### 3. Activity authentication is not connected end to end

[app.js](../../../rust/nc-bridge/static/app.js), `resolveIdentity`, tries SDK
readiness and a user property, then falls back to supplied or random handles.
It never calls the herald's token-exchange endpoint or creates a server-side
session. The bridge does not validate a herald result. The runbook routes public
traffic to the bridge, whose router does not expose the herald's auth route.

There is also an unsafe configuration interaction: [herald/main.rs](../../../rust/nc-herald/src/main.rs)
forces `dry_run` when no announcement webhook is configured. In
[herald/lib.rs](../../../rust/nc-herald/src/lib.rs), the same flag makes OAuth
return a fixed mock profile for any code when client credentials are configured.
Disabling announcements must not make authentication synthetic. Keep mock
identity confined to tests. Wire one real login flow through the browser,
server-side exchange, session and seat binding before claiming Activity support.

### 4. Callback authentication and child cleanup need hardening

The bridge's callback secret is `tok_node_<node>_<loopback-port>`, not a random
secret. Its internal event route shares the publicly served router. Do not
expose that route through the public proxy; generate unpredictable credentials
and keep the herald's event/broadcast routes private or authenticated.

After spawning the door, the bridge's accept-error and five-second-timeout paths
release the node but do not kill and reap the child. `kill_on_drop` is not set.
The normal proxy path does kill the child; the failure paths need the same
ownership guarantee. Test a child that never connects and a disconnect during
startup. User-controlled names also need line-break rejection before being
serialized into a line-oriented legacy dropfile.

### 5. The maintenance recipe and public announcements are placeholders

The [runbook](../../sysop/discord-bridge-runbook.md) uses
`nc-sysop maint run --dir ...`, but the shipped CLI takes `nc-sysop maint <dir>`.
It supplies the operating-system calendar year to the herald, rather than the
campaign year. Its herald option placement also differs from the CLI's
root-level options and should be tested verbatim.

`nc-herald broadcast-turn` contains fixed empire standings; `nc-discord /status`
prints a fixed 04:00 UTC schedule. Neither reads live campaign state for those
claims. Connect broadcasts to successful maintenance and its resulting year.
Publish only approved public information; do not forward private tactical
summaries to a shared channel indiscriminately.

### 6. The callback and export contracts need an integration test

The bridge writes callbacks and platform identity to DOOR.KDL but launches the
child with DOOR32.SYS. The legacy file cannot convey the KDL callback secret or
platform identity. Existing tests validate both files independently; that does
not prove the launched process receives the intended capabilities.

The runbook assumes staged map/report directories but does not demonstrate a
full door-export-to-bot delivery cycle. Test the actual process arguments,
export locations, current turn, callback delivery and disconnect cleanup using
a disposable campaign. Keep the ordinary BBS alias behavior unchanged while
making the web boundary trustworthy.

### 7. Smaller operator issues

The herald logs full Discord webhook URLs, which contain credentials. Redact
those URLs and avoid logging authorization codes. The bot's `/guide` expects
`docs/manuals/nc_quick_start.pdf` relative to its working directory, but the
runbook's installation layout does not provision that path. The guide response
also still calls the revised PDF a three-page guide. Its absence currently
produces a successful message without an attachment.

## Recommended sequence

1. Authenticate web/Activity sessions and establish one stable identity-to-seat mapping.
2. Use that mapping for exact private export selection and denial of unmapped users.
3. Correct maintenance, real campaign status and explicitly public announcements.
4. Complete process cleanup, callback isolation and a full local integration test.
5. Deploy a private test campaign behind the existing HTTPS proxy, then test the
   official Discord client and browser before inviting players.

Keep the public website separate from the game endpoint. Bind the bridge and
herald to unused loopback ports. Existing hosting services may already occupy
the runbook defaults; inspect the chosen server before assigning ports. Personal
server inventory and credentials are intentionally excluded from this note.

## Validation and limits

The current Rust tree is identical to the tree in the beta.3 workspace run:
3,233 tests passed, 11 ignored. The bridge integration test covers health,
static serving and callback token comparisons; it does not test authenticated
WebSocket admission or attachment ownership. No new tests were run for this
source-only review, and no messages were sent to Discord. The findings above
are code-path findings, not claims of an observed attack on a live campaign.

See the [release plan](../plans/fv6fg33o-release-v1-0-0-beta-3.md) for the existing
BBS package acceptance. Those checks did not certify this separate hosting stack.
