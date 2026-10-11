# Nostrian Conquest Hosted Architecture v2

> Status: future design draft, not the current shipped stack.
>
> Today the supported public gameplay surfaces are `nc-game` (localhost),
> `nc-door` (BBS), and `nc-sysop` (local/BBS administration). This document
> defines the separate future hosted stack centered on `nc-host` and
> `nc-helm`.

## 1. Core Direction

The hosted Nostr path is a clean split from localhost/BBS play.

- `nc-sysop` remains localhost/BBS-only.
- `nc-host` owns relay-native hosted play.
- `nc-helm` grows a hosted lobby plus hosted dashboard mode.
- `nc-helm` keeps its own local keychain/cache/settings in platform-specific
  user paths using KDL files.
- Hosted storage does not reuse localhost/BBS `ncgame.db`.

The daemon model is:

- one daemon process hosts many simultaneous games
- one dedicated relay/node belongs to that daemon
- one daemon identity keypair belongs to that daemon
- each hosted game keeps its own self-contained directory and DB

## 2. Hosted Topology

```text
+-----------+      +-----------+      +---------------------------+
|  nc-helm  | <--> |   relay   | <--> |        nc-host          |
|  lobby +  |      | (daemon-  |      | supervisor + game workers |
| dashboard |      | dedicated)|      +---------------------------+
+-----------+      +-----------+                    |
                                                    v
                                         <games-root>/<slug>/hosted.db
```

Localhost/BBS stays separate:

```text
+----------+     +----------+     +-----------+
| nc-game  | --> | nc-door  | --> | ncgame.db |
| nc-sysop |     |          |     |           |
+----------+     +----------+     +-----------+
```

## 3. Storage Boundary

Each hosted game is fully self-contained under its own directory.

Example:

```text
/srv/nc-host/games/friday-night/
  hosted.db
```

Daemon-global files live outside the games root:

```text
/etc/nc-host/host.kdl
/etc/nc-host/host.nsec
```

`hosted.db` is the authoritative per-game store for:

- game metadata and lobby settings
- seat roster and invite-code state
- maintenance schedule state
- pending turn submissions
- outbound publish/outbox jobs
- stored invite requests and audit trail

The hosted stack does not revive retired hosted tables inside localhost/BBS
`ncgame.db`.

## 4. Runtime Model

The daemon uses a hybrid TEA-style architecture.

- the process-level supervisor owns relay connectivity, game catalog loading,
  scheduling, and worker lifecycle
- each game runs as its own worker with a typed loop:
  `GameMsg -> update(GameRuntime, GameMsg) -> GameEffects`
- all DB mutation for one game is serialized through that game worker
- Nostr publishing is staged through a per-game outbox for retry-safe delivery

This keeps the code lean:

- no giant `serve.rs`
- no catch-all runtime object with every feature jammed into one module
- no shared multi-tenant mutation path across games

## 5. Lobby Model

`nc-helm --lobby` is the public hosted discovery surface.

The public lobby shows only games that are both:

- `lobby_visibility=public`
- actively recruiting

Recruiting values:

- `none`
- `new_players`
- `replacement_players`

Players see public recruiting metadata only:

- game name
- game tier
- current year/turn
- recruiting mode
- open seat count
- short lobby summary
- host alias

Players do not see:

- raw invite codes
- hidden seat roster details
- private per-player state

The player client keeps minimal local state only:

- encrypted keychain
- encrypted joined-games and invite cache
- relay/config/settings files

These live in platform-specific user paths as KDL files and are not the
authoritative hosted store.

First launch flow:

1. set player handle
2. set keychain password
3. generate one local identity
4. write encrypted keychain/cache plus local config/settings files
5. enter the lobby

Hosted handle ownership is host-local and tied to the player's `npub`.
Comparison is case-insensitive after trimming whitespace. The same `npub` may
keep or change its own handle, but a second player may not claim it on that
same `nc-host`. `nc-helm` may save a handle locally while offline, but that
choice remains unverified until `nc-host` accepts it through an explicit handle
check or a later hosted action.

All non-public hosted kinds are private-by-default:

- only `30500 GameDefinition` and `30516 LobbyNotice` remain public/plain
- `30507`, `30510`, `30513`, `30517`, `30518`, `30522`, and `30523` are
  player-authored private events
- `30525` is the player-authored private handle-check request
- `30511`, `30514`, `30515`, `30517`, `30520`, `30521`, `30523`, and `30524`
  are host-authored private events
- `30526` is the host-authored private handle-check result
- private event content uses NIP-44 plus an inner versioned envelope that may
  apply zstd compression for larger payloads

`nc-lobby` has two communication surfaces:

- one host-wide public notice board, sysop-authored only
- one encrypted direct-contact community `COMMS` surface keyed by known `npub`
- anonymous per-game `GAME INBOX` diplomacy remains available, but as an
  in-game surface rather than a lobby/community thread list

## 6. Invite and Join Flow

Hosted first joins in the normal `nc-helm` lobby flow are approval-based and
code-free for the player. Invite codes remain available only as a reserve,
operator-controlled path.

Reserve/manual invite codes still use the old human-readable format:

```text
{token}@{relay-host[:port]}
```

But the public lobby never exposes those codes. The server flow is:

1. `nc-host` publishes a public `30500 GameDefinition` for recruiting games.
2. `nc-helm --lobby` lists those games.
3. A player sends a join request over Nostr to the daemon.
4. The daemon stores the request in the target game's request queue and
   notifies the sysop contact identity.
5. The sysop approves or rejects the request through `nc-host`.
6. If approved, the daemon binds the chosen seat to the requesting pubkey as
   part of that approval transaction.
7. The daemon privately sends the approval result, including the assigned seat,
   to the player.
8. Later rejoin is by pubkey plus `game-id`, not by any invite token.

Seat lifecycle is intentionally small:

- `pending`
- `claimed`

Tier policy stays outside the low-level seat shape but is still part of the
hosted product contract:

- `sandbox`
  - low-friction try-the-game pool
  - join requests may be auto-approved while open seats exist
  - players who go MIA for 3 turns are ejected and the seat reopens
  - players who stay in the same claimed seat for 10 elapsed game turns are
    rotated out and the seat reopens
  - the game itself stays open; only the seat occupant changes
- `league`
  - long-form serious campaign
  - seats stay with the approved player unless the sysop resets them or some
    separate replacement policy applies

Admin actions:

- reissue invite
- reset seat binding
- open seat
- close seat

## 7. Hosted Turn Policy

Hosted games default to wall-clock/manual resolution, not early auto-resolve.

Per-game settings include:

- `maintenance_enabled`
- `maintenance_interval_minutes`
- `maintenance_next_due_unix_seconds`

`nc-host serve` polls each game's schedule once a minute and sends a due
game's maintenance to its worker, so it runs in turn with that game's other
work. A game whose `maintenance_next_due_unix_seconds` is unset is armed one
interval from now rather than run at once. After a run the next due time
moves one interval past now, whether or not the run succeeded, so a host
that was down past a due time runs one year when it comes back, not one per
missed interval. Turns do not run early because all players submitted.

Maintenance applies the pending turn submissions for the current turn, in
the order they arrived, through the same typed `TurnSubmission` path
`nc-game submit-turn` uses; runs the campaign pipeline `nc-sysop maint`
runs (`nc_engine::CampaignMaintenance`); and then queues every claimed seat
a `30521` delta against the last snapshot it was sent, or a `30520`
snapshot if it has none.

Sandbox maintenance has two extra hosted rules:

- 3-turn MIA players are ejected and their seats reopen
- claimed sandbox seats recycle after 10 elapsed turns so new players can keep
  sampling the same long-running sandbox

That recycle is seat-local, not game-local. It must not mark the game
`finished`.

## 8. Nostr Event Surface

Hosted `nc-host` owns these kinds:

| Kind | Name | Purpose |
|------|------|---------|
| `30500` | `GameDefinition` | Public recruiting-game catalog row |
| `30507` | `StateRequest` | Client requests a refresh |
| `30510` | `SeatClaimRequest` | Reserve/manual invite claim |
| `30511` | `SeatClaimResult` | Reserve/manual claim result |
| `30513` | `InviteRequest` | Player asks to join a recruiting game |
| `30514` | `InviteRequestReceipt` | Daemon acknowledges receipt/rejection |
| `30515` | `InviteDecision` | Sysop approval or rejection result |
| `30516` | `LobbyNotice` | Public host-wide notice board post |
| `30517` | `SysopThreadMessage` | Legacy/operator private thread surface |
| `30518` | `ContactMessage` | Encrypted direct contact chat |
| `30520` | `GameState` | Full fog-of-war-filtered snapshot |
| `30521` | `StateDelta` | Incremental hosted update |
| `30522` | `TurnCommands` | Submitted player orders |
| `30523` | `PlayerMessage` | Encrypted anonymous player-to-player diplomacy |
| `30524` | `TurnReceipt` | Turn accepted/rejected with details |
| `30525` | `HandleCheck` | Immediate host-local handle availability check |
| `30526` | `HandleCheckResult` | Immediate host-local handle ownership result |

Retired SSH bridge kinds `30501`/`30502`/`30503` remain legacy reference only
for the old `nc-connect` / `nc-gate` path.

For private request kinds, public tags stay routing-only:

- `d`
- `p`
- `game-id`
- `turn` for `30522`

Player handle, invite strings, state request bodies, and raw turn text live
inside the encrypted payload, not in public tags.

Normal hosted enforcement still happens even without `30525`: invite requests,
state refreshes, and turn submissions validate the supplied handle against the
host-local roster so an offline-saved conflicting handle is rejected the first
time it is actually used.

## 9. `GameDefinition` Requirements

`30500 GameDefinition` is the public lobby catalog event.

Required tags:

- `d` game id
- `name`
- `status`
- `catalog-state`
- `players`
- `recruiting`
- `open-seats`
- `year`
- `turn`

Optional tags:

- `summary`
- `host-alias`
- `host-contact-npub`
- `host-contact-label`
- `host-contact-nip05`
- `tier`
- `slot`

`slot` remains hashed-only:

```text
["slot", "<seat>", "<invite-code-hash>", "<player-pubkey-or-empty>", "<status>"]
```

Public events must never reveal raw invite codes.
The latest `30500` wins per `(daemon pubkey, game id)`. `catalog-state=retired`
is the app-level tombstone that removes stale games from public discovery.

## 10. Module Layout

The hosted implementation should be split like this.

### `nc-host`

- `src/main.rs`
- `src/dispatch.rs`
- `src/commands/`
  - `new_game.rs`
  - `serve.rs`
  - `maint.rs`
  - `settings.rs`
  - `games.rs`
  - `seats.rs`
  - `requests.rs`
  - `nostr.rs`
- `src/config/`
  - `host_config.rs`
  - `identity.rs`
  - `relay.rs`
  - `sysop_contact.rs`
- `src/supervisor/`
  - `catalog.rs`
  - `runtime.rs`
  - `scheduler.rs`
  - `routing.rs`
- `src/lobby/`
  - `catalog_publish.rs`
  - `catalog_view.rs`
  - `invite_requests.rs`
  - `notify_sysop.rs`
  - `rate_limit.rs`
- `src/game/`
  - `msg.rs`
  - `state.rs`
  - `update.rs`
  - `effects.rs`
  - `maint.rs`
  - `seats.rs`
  - `turns.rs`
  - `outbox.rs`
- `src/support/`

### `nc-data`

- `src/hosted/`
  - `store.rs`
  - `schema.rs`
  - `player_roster.rs`
  - `settings.rs`
  - `seats.rs`
  - `invite_requests.rs`
  - `turn_queue.rs`
  - `outbox.rs`
  - `snapshots.rs`

### `nc-nostr`

- `game_definition.rs`
- `seat_claim.rs`
- `handle_check.rs`
- `invite_request.rs`
- `state_sync.rs`
- `turn_commands.rs`
- `turn_receipt.rs`
- `tags.rs`
- `json.rs`

The ownership rule is the same as the rest of the repo:

- command modules orchestrate
- `nc-data` stores
- `nc-nostr` defines wire shapes
- `nc-engine` owns rules

## 11. Command Surface

### `nc-host`

```text
nc-host nostr init
nc-host new-game <dir> [--players N] [--name "Name"] [--seed N]
nc-host serve --root <games-root> [--config <path>]
nc-host status [--config <path>] [--root <path>] [--json]
nc-host games list
nc-host games status [--dir <path>]
nc-host maint <dir> [turns]
nc-host settings show --dir <path>
nc-host settings set --dir <path> ...
nc-host seats list --dir <path>
nc-host seats reissue --dir <path> --player N
nc-host seats reset --dir <path> --player N
nc-host seats open --dir <path> --player N
nc-host seats close --dir <path> --player N
nc-host requests list [--dir <path>]
nc-host requests show --dir <path> --request <id>
nc-host requests approve --dir <path> --request <id> --player N
nc-host requests reject --dir <path> --request <id> --message "..."
nc-host notices post --message "..." [--handle <name>]
nc-host threads list --dir <path>
nc-host threads show --dir <path> --player <npub>
nc-host threads send --dir <path> --player <npub> --message "..." [--handle <name>]
```

### `nc-helm`

```text
nc-helm
nc-helm --relay <url>
```

`nc-helm` is lobby-first for the hosted path. It keeps local keychain/cache
state in platform-specific KDL files and does not launch an SSH/PTy bridge.

See [../helm/lobby-architecture.md](../helm/lobby-architecture.md) for the
client-side hosted UI/state architecture.

## 12. Explicit Non-Goals

The first hosted spec does not require:

- relay federation
- per-game relay overrides
- a shared multi-tenant control DB
- direct player-to-player diplomacy events
- player-authored public lobby chat
- auto-issuing invites for public seats
- early auto-resolve when all turns are submitted

These can be added later without collapsing the per-game storage or per-game
worker boundaries.
