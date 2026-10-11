# DOOR.KDL: Modern Dropfile Specification for Chat & Web Doors

This document specifies **`DOOR.KDL`**, a modern, structured dropfile format designed to replace legacy 1990s flat-file dropfiles (`DOOR32.SYS`, `DOOR.SYS`, `CHAIN.TXT`) when hosting terminal door games within modern chat platforms (Discord, Matrix, Nostr, Zulip) and web environments.

---

## 1. Motivation: The Limits of Legacy Dropfiles

Classic dropfiles were invented in the era of physical modems, dial-up BBSes, and single-tasking DOS systems. While `nc-door` supports `DOOR32.SYS`, legacy formats suffer from severe structural limitations:

| Legacy Problem (`DOOR32.SYS`) | Impact on Modern Chat & Web Hosting |
|---|---|
| **Fragile Positional Syntax** | Lines are indexed by number (e.g., Line 7 = handle). Missing or blank lines offset the parser, corrupting data. |
| **Dial-Up & Hardware Cruft** | Fields for baud rate, DTE speed, COM port IRQ, and serial flow control are useless over WebSockets and PTYs. |
| **Single Flat Identity String** | Modern platforms have distinct concepts: immutable IDs (Discord Snowflake / Matrix MXID), unique handles (`@starmaster`), and cosmetic display names (`Lord Vorn`). Classic files only offer a single 15–30 character string. |
| **Crude Terminal Flags (0=TTY, 1=ANSI)** | Modern terminals can negotiate UTF-8 vs CP437, 16-color ANSI, xterm-256, TrueColor, and dynamic window dimensions. |
| **No Inbound Event Hook** | A door game cannot push in-game events (combat dispatches, player surrenders) back to the host chat room without hardcoding proprietary bot tokens inside the engine. |
| **No Device or Form-Factor Context** | The game cannot distinguish between a desktop user with a physical keyboard and a mobile phone user with a touchscreen. |

`DOOR.KDL` solves these limitations by using [KDL](https://kdl.dev/), the same human-readable, typed, and structured document language used throughout Nostrian Conquest.

---

## 2. Complete Schema & Node Reference

A `DOOR.KDL` file consists of structured, typed nodes:

```kdl
// DOOR.KDL Specification — Version 1
version 1

// Host platform and origin context
platform "discord" {
    guild_id "109283746592837465"
    channel_id "928374651029384756"
    session_id "sess_9f8a2b3c4d"
}

// Caller identity and permissions
caller {
    // Immutable account ID from the host platform
    platform_id "248102948102948192"
    
    // Account handle / username (matched against campaign seats)
    handle "starmaster"
    
    // Human-readable cosmetic display name
    display_name "Lord Vorn of Capella"
    
    // Optional cryptographic identity (e.g. Nostr npub)
    public_key "npub180cvv07tjdrrgpa0j7j7tmnyl2yr6yr7l8j4s3evf6u64th6gkwsyjh6w6" optional=#true
    
    // Session authorization role
    role "player" // "player" | "spectator" | "sysop"
    
    // User locale
    locale "en-US"
}

// Session limits and virtual node allocation
session {
    // Session timeout limits in seconds
    time_limit_seconds 1800     // 30 minutes wall-clock limit
    idle_timeout_seconds 300    // 5 minutes inactivity kick
    
    // Virtual node number for multi-node concurrency tracking
    node 2
}

// Transport connection handoff
transport {
    // Delivery mechanism
    mode "loopback-tcp" port=5042 host="127.0.0.1"
    
    // Supported modes:
    // mode "loopback-tcp" port=<int> [host="127.0.0.1"]
    // mode "unix-socket" path="/tmp/nc-node-2.sock"
    // mode "stdio"
}

// Client terminal capabilities and geometry
terminal {
    // Screen geometry
    cols 80
    rows 25
    resizable #false
    
    // Character set & color capabilities
    encoding "cp437" // "cp437" | "utf-8"
    color "ansi16"   // "mono" | "ansi16" | "xterm256" | "truecolor"
    
    // Client environment hints
    client "xterm.js"
    input_device "keyboard" // "keyboard" | "touchscreen" | "hybrid"
}

// Host event callback hooks (optional)
callbacks {
    // Outbound webhook for live game event broadcasts
    event_webhook "http://127.0.0.1:8080/internal/events/node2" secret="ephemeral_tok_7a8b9c"
}
```

---

## 3. Node Specifications

### 3.1. `platform`
Identifies the hosting ecosystem.
* **Arguments:** Platform identifier string (e.g. `"discord"`, `"matrix"`, `"nostr"`, `"web"`, `"telnet"`).
* **Properties:**
  * `guild_id` / `server_id`: Host community identifier.
  * `channel_id` / `room_id`: Channel where the activity was launched.
  * `session_id`: Unique trace ID for logging and diagnostic tracing.

### 3.2. `caller`
Encapsulates player identity.
* `platform_id` *(string, required)*: The permanent, immutable ID assigned by the host platform (e.g. Discord Snowflake, Matrix MXID). Used to ensure account continuity even if a user changes their handle.
* `handle` *(string, required)*: The primary username or handle. `nc-door` uses this value to match against empire seat reservations in `config.kdl` and `ncgame.db`.
* `display_name` *(string, optional)*: Current nickname or display name.
* `public_key` *(string, optional)*: Cryptographic public key (e.g. Nostr `npub` or hex ed25519) if the host bridge supports self-sovereign identity.
* `role` *(string, default: `"player"`)*:
  * `"player"`: Normal interactive seat holder.
  * `"spectator"`: Read-only observer mode. Allows server members to watch a galaxy map without holding a seat.
  * `"sysop"`: Administrative operator bypass.
* `locale` *(string, default: `"en-US"`)*: Language and regional format hint.

### 3.3. `session`
Defines operational boundaries for the active run:
* `time_limit_seconds` *(integer)*: Maximum allowed session duration. When this elapsed time expires, `nc-door` displays a warning screen, saves changes, and exits cleanly.
* `idle_timeout_seconds` *(integer)*: Threshold of user input inactivity before disconnect.
* `node` *(integer)*: Virtual node identifier (1-based index).

### 3.4. `transport`
Specifies how the game binary connects to the player's terminal stream:
* `mode "loopback-tcp"`: Connects to a local TCP socket via `port=<int>` and optional `host=<ip>` (standard for ENiGMA½, abracadabra, and WebSocket bridges).
* `mode "unix-socket"`: Connects to a Unix domain socket via `path=<string>` (optimal for Linux/macOS high-performance local IPC).
* `mode "stdio"`: Uses inherited standard input and standard output (standard for SSH commands and PTY wrappers).

### 3.5. `terminal`
Defines client rendering capabilities:
* `cols` / `rows` *(integers)*: Viewport geometry (default: 80x25).
* `resizable` *(boolean)*: Indicates if the client window can change dimensions during gameplay.
* `encoding` *(string)*: `"cp437"` (classic DOS code page) or `"utf-8"`.
* `color` *(string)*: `"mono"`, `"ansi16"` (classic 16 colors), `"xterm256"`, or `"truecolor"`.
* `input_device` *(string)*:
  * `"keyboard"`: Hardware keyboard connected.
  * `"touchscreen"`: Mobile / tablet touch client. The game or frontend can present touch-friendly shortcuts and avoid relying on physical arrow keys.
  * `"hybrid"`: Touchscreen device with hardware keyboard attached.

### 3.6. `callbacks`
Allows the door game to push events back to the host platform without embedding Discord bot tokens or platform secrets inside the game engine.
* `event_webhook` *(URL)*: Local loopback endpoint managed by the bridge daemon.
* `secret` *(string, optional)*: Ephemeral authorization bearer token passed in the `Authorization: Bearer <secret>` header.

When major milestones occur in the game, `nc-door` can send a POST request with a lightweight JSON event:
```json
{
  "node": 2,
  "player": "Lord Vorn",
  "seat": 3,
  "event": "bombardment_success",
  "system": "Capella IV",
  "summary": "Lord Vorn has successfully neutralized orbital defenses at Capella IV!"
}
```
The bridge forwards this message directly to the Discord channel.

---

## 4. Concrete Examples

### Example A: Discord Activity (Desktop Player)

```kdl
version 1

platform "discord" {
    guild_id "481029481029481920"
    channel_id "892019481029481020"
    session_id "disc_act_01J9"
}

caller {
    platform_id "392019401928381920"
    handle "sol_admiral"
    display_name "Admiral Vance"
    role "player"
}

session {
    time_limit_seconds 1800
    idle_timeout_seconds 300
    node 1
}

transport {
    mode "loopback-tcp" port=5041 host="127.0.0.1"
}

terminal {
    cols 80
    rows 25
    resizable #false
    encoding "cp437"
    color "ansi16"
    client "xterm.js"
    input_device "keyboard"
}

callbacks {
    event_webhook "http://127.0.0.1:8080/events/node1" secret="tok_99182a"
}
```

### Example B: Discord Mobile Activity (Touchscreen Player)

```kdl
version 1

platform "discord" {
    guild_id "481029481029481920"
    channel_id "892019481029481020"
}

caller {
    platform_id "781029481029481920"
    handle "nova_scout"
    display_name "Nova"
    role "player"
}

session {
    time_limit_seconds 1200
    node 4
}

transport {
    mode "loopback-tcp" port=5044 host="127.0.0.1"
}

terminal {
    cols 80
    rows 25
    resizable #false
    encoding "cp437"
    color "ansi16"
    client "xterm.js"
    input_device "touchscreen"
}
```

### Example C: Minimal Read-Only Spectator

```kdl
version 1

platform "web"

caller {
    platform_id "anon_9921"
    handle "guest"
    display_name "Observatory Spectator"
    role "spectator"
}

session {
    time_limit_seconds 600
    node 8
}

transport {
    mode "unix-socket" path="/run/nc/nodes/node8.sock"
}

terminal {
    cols 80
    rows 25
    encoding "cp437"
    color "ansi16"
}
```

---

## 5. Implementation & Invocation Pattern

The invocation of `nc-door` remains clean and backwards-compatible:

```bash
nc-door \
  --dir /srv/nc/campaigns/alpha \
  --dropfile /tmp/nc-nodes/node1/door.kdl
```

### Format Autodetection Rule
* If `--dropfile` targets a file named `DOOR32.SYS`, `DOOR.SYS`, or `CHAIN.TXT`, the engine parses it using the classic positional parser.
* If `--dropfile` targets `*.kdl` or `door.kdl`, the engine uses the `DOOR.KDL` parser.
* Command-line flags (such as `--socket-port` or `--encoding`) remain explicit overrides if provided on the CLI.

---

## 6. Crate Ecosystem & Data Flow

`DOOR.KDL` serves as the shared data contract between the web/chat bridge and the game player client. In the Rust workspace, this is encapsulated in the `nc-dropfile` crate:

```text
               ┌───────────────────────┐
               │      nc-dropfile      │  (Shared Data Crate)
               └───────────┬───────────┘
                           │
             ┌─────────────┴─────────────┐
             ▼                           ▼
  ┌─────────────────────┐     ┌─────────────────────┐
  │       nc-game       │     │      nc-bridge      │  (Web / PTY Daemon
  │     (`nc-door`)     │     └──────────┬──────────┘   & Node Manager)
  └─────────────────────┘                │
                                         ▼
                              ┌─────────────────────┐
                              │      nc-herald      │  (Outbound Event Hub
                              └──────────┬──────────┘   & Webhook Dispatcher)
                                         │
                 ┌───────────────────────┼───────────────────────┐
                 ▼                       ▼                       ▼
      ┌─────────────────────┐ ┌─────────────────────┐ ┌─────────────────────┐
      │     nc-discord      │ │      nc-matrix      │ │    (Future Bots)    │
      │   (Serenity/Poise)  │ │    (matrix-sdk)     │ │   Telegram / IRC    │
      └─────────────────────┘ └─────────────────────┘ └─────────────────────┘
```

* **Producer:** `nc-bridge` serializes `DoorKdl` into `/tmp/nc-nodes/node_N/door.kdl` when initializing a player session.
* **Consumer:** `nc-door` parses `DoorKdl` on startup to bind player seats, configure sockets, and enforce time limits.
* **Announcer:** `nc-herald` listens for outbound event callbacks fired by active nodes.
* **Bot Gateways:** `nc-discord` (and future `nc-matrix`) provides interactive player utilities (`/map`, `/report`, `/play`).

