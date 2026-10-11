# Discord Door Architecture: Sovereign BBS Door Integration

This document specifies the architectural design for hosting Nostrian Conquest as a classic BBS door accessible directly within Discord (via Discord Embedded Activities), while maintaining complete self-sovereignty and zero structural dependency on Discord.

---

## 1. Design Philosophy: Discord as "Dumb Glass"

Nostrian Conquest is designed around self-sovereignty and protocol agnosticism. Discord is treated strictly as **an external, untrusted, and disposable frontend**—conceptually identical to classic BBS software such as Mystic, Synchronet, or ENiGMA½.

### Core Tenets

1. **Zero Engine Awareness:** Neither `nc-door`, `nc-game`, nor `nc-engine` contains Discord-specific code, libraries, or concepts.
2. **Standard Dropfile Interface:** Communication between Discord and the game occurs through standard BBS dropfiles (`DOOR32.SYS`) and socket loopback streams (`--socket-port`).
3. **Sovereign State & Identity:** Game state lives exclusively in `ncgame.db` (SQLite). Player seats are bound to handles. If Discord bans the bot, changes its API policies, or shuts down, the campaign continues without disruption on Telnet, SSH, or native web.
4. **Isolated Sysop Control:** Campaign administration, turn maintenance, and emergency rollbacks are managed exclusively through `nc-sysop` on the host machine. Discord users never have execution or administrative access to the underlying server.

---

## 2. Macro Architecture

The architecture decouples the player interface into three distinct layers:

1. **Client / Activity Frame (Discord App):** Runs in the Discord client (desktop, web, or mobile) via the Embedded App SDK, hosting an `xterm.js` terminal in a browser iframe.
2. **Discord BBS Bridge Daemon (Virtual Node Manager):** A lightweight daemon that authenticates Discord sessions, allocates virtual BBS nodes, generates `DOOR32.SYS` dropfiles, and proxies WebSocket traffic to local TCP sockets.
3. **Core NC Runtime (`nc-door` + `ncgame.db`):** The standard Rust binaries executing locally on the host machine.

```text
 ┌──────────────────────────────────────────────────────────────────────────┐
 │                         DISCORD CLIENT / PLAYERS                         │
 │                                                                          │
 │   Desktop / Browser Client                       Mobile Client (iOS/And) │
 │  ┌─────────────────────────┐                    ┌───────────────────────┐│
 │  │ Discord Activity iframe │                    │ Discord Activity      ││
 │  │ • xterm.js (CP437/ANSI) │                    │ • xterm.js Canvas     ││
 │  │ • Hardware Key Inputs   │                    │ • Virtual Retro D-Pad ││
 │  └────────────┬────────────┘                    └───────────┬───────────┘│
 └───────────────┼─────────────────────────────────────────────┼────────────┘
                 │                                             │
                 │ HTTPS (Activity Assets) / WSS (Terminal)    │
                 ▼                                             ▼
 ┌──────────────────────────────────────────────────────────────────────────┐
 │                        EDGE & INGRESS LAYER                              │
 │                                                                          │
 │   Cloudflare Tunnel (`cloudflared`)  OR  Reverse Proxy (Caddy / Nginx)   │
 │   • Terminates TLS (WSS / HTTPS)                                         │
 │   • Forwards clean local traffic to Bridge Daemon                        │
 └───────────────────────────────────┬──────────────────────────────────────┘
                                     │
                                     ▼
 ┌──────────────────────────────────────────────────────────────────────────┐
 │                     DISCORD BBS BRIDGE DAEMON                            │
 │                                                                          │
 │   1. Validates Discord OAuth2 Token / Retrieves User ID & Handle         │
 │   2. Allocates Virtual Node slot (e.g. Node 1, Node 2)                   │
 │   3. Generates `/tmp/nodes/node_N/DOOR32.SYS`                            │
 │   4. Spawns `nc-door` on ephemeral loopback port                         │
 │   5. Bi-directional proxy: WebSocket (Browser) <──> TCP Socket (nc-door) │
 └───────────────────────────────────┬──────────────────────────────────────┘
                                     │
                 ┌───────────────────┴───────────────────┐
                 │ Process Exec & Socket Loopback        │
                 ▼                                       ▼
 ┌───────────────────────────────────┐   ┌───────────────────────────────────┐
 │       NC-DOOR (Node 1)            │   │       NC-DOOR (Node 2)            │
 │  • Reads DOOR32.SYS               │   │  • Reads DOOR32.SYS               │
 │  • Resolves caller to Seat        │   │  • Resolves caller to Seat        │
 │  • Enforces dropfile timer        │   │  • Enforces dropfile timer        │
 │  • Renders 80x25 CP437 ANSI       │   │  • Renders 80x25 CP437 ANSI       │
 └─────────────────┬─────────────────┘   └─────────────────┬─────────────────┘
                   │                                       │
                   └───────────────────┬───────────────────┘
                                       │ Local Read/Write
                                       ▼
 ┌──────────────────────────────────────────────────────────────────────────┐
 │                        HOST CAMPAIGN DIRECTORY                           │
 │                                                                          │
 │   • `ncgame.db` (SQLite campaign database)                               │
 │   • `config.kdl` (Player reservations, aliases, and campaign limits)     │
 │                                                                          │
 │   [Scheduled Cron / Systemd Timer]                                       │
 │         │                                                                │
 │         ▼                                                                │
 │   `nc-sysop maint run` (Yearly turn cycle calculation)                   │
 │         │                                                                │
 │         ▼                                                                │
 │   Herald Dispatcher ───────► Discord Webhook (`#galactic-bulletin`)     │
 └──────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Alternative Access: Preserving Sovereignty

Because the host uses the standard `nc-door` binary, Discord is merely one of several simultaneous doorways into the same campaign:

```text
                               ┌───────────────────────────┐
                               │   Discord Activity Iframe │
                               └─────────────┬─────────────┘
                                             │
      ┌──────────────────────┐               │               ┌──────────────────────┐
      │  Standard Web Browser│               │               │  Telnet / SSH BBS    │
      │  (Direct HTTPS / WSS)├───────────────┼───────────────┤ (Synchronet / Mystic)│
      └──────────┬───────────┘               │               └──────────┬───────────┘
                 │                           │                          │
                 ▼                           ▼                          ▼
      ┌──────────────────────┐   ┌───────────────────────┐   ┌──────────────────────┐
      │ Web Bridge           │   │ Discord Bridge        │   │ Native BBS Node      │
      │ (Local Login Screen) │   │ (Discord OAuth Token) │   │ (DOOR32.SYS Dropfile)│
      └──────────┬───────────┘   └───────────┬───────────┘   └──────────┬───────────┘
                 │                           │                          │
                 └───────────────────┬───────┴──────────────────────────┘
                                     │
                                     ▼
                      ┌─────────────────────────────┐
                      │    `nc-door` Executable     │
                      │  `nc-door --dropfile ...`   │
                      └──────────────┬──────────────┘
                                     │
                                     ▼
                      ┌─────────────────────────────┐
                      │    Campaign `ncgame.db`     │
                      └─────────────────────────────┘
```

If Discord access is ever decommissioned or banned, players simply connect via direct browser URL or Telnet/SSH, claiming their existing seat using their registered handle.

---

## 4. The Dropfile Handoff (`DOOR32.SYS`)

`nc-door` natively understands the modern `DOOR32.SYS` dropfile format. When a player clicks "Play" in Discord, the bridge daemon constructs a compliant `DOOR32.SYS` file in a temporary per-node directory.

### Mapping Discord Context to DOOR32.SYS

| Line # | DOOR32 Field | Injected Value | Purpose |
|---|---|---|---|
| **1** | Comm type | `2` (Telnet / TCP Socket) | Tells `nc-door` to communicate over network stream |
| **2** | Comm or socket handle | `0` (Unused when using `--socket-port`) | Reserved for raw file descriptor passing |
| **3** | Baud rate | `38400` | Classic line speed indicator |
| **4** | BBS Name | `Discord Galactic Federation` | Cosmetic BBS name shown in system banners |
| **5** | User record number | `1` | Placeholder numeric ID |
| **6** | User real name | `<Discord Display Name>` | Used for logging / seat reservation fallback |
| **7** | User handle / alias | `<Discord Username>` or configured alias | **Primary key** `nc-door` matches against seat reservations |
| **8** | Security level | `30` | Normal player level |
| **9** | Time remaining | `30` (Minutes) | In-door session timeout enforced by `nc-door` |
| **10** | Emulation | `1` (Color ANSI) | Guarantees 16-color ANSI output |
| **11** | Node number | `1` (Unique per concurrent worker) | Virtual node identifier |

### Process Execution Command

The bridge starts an ephemeral TCP listener on an available port (e.g. `5041`) and spawns:

```bash
nc-door \
  --dir /srv/nc/campaigns/alpha \
  --dropfile /tmp/nc-nodes/node1/DOOR32.SYS \
  --socket-port 5041 \
  --encoding cp437 \
  --color-mode ansi16
```

`nc-door` connects to localhost port `5041`, takes ownership of the I/O stream, matches the caller handle to their seat in `ncgame.db`, and launches the game loop.

---

## 5. Discord Activity Client Implementation

The client running inside the Discord Activity iframe consists of a minimal static web package.

```text
┌────────────────────────────────────────────────────────────────────────┐
│                   DISCORD EMBEDDED ACTIVITY IFRAME                     │
│                                                                        │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │  TOP STATUS BAR (Empire Name, Turn Year, Stardate, Disconnect)   │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                                                                  │  │
│  │                     80x25 xterm.js TERMINAL                      │  │
│  │                                                                  │  │
│  │           • Authentic CP437 / VGA font rendering                 │  │
│  │           • 16-Color classic ANSI palette                        │  │
│  │           • Optional CRT scanline & bloom shader                 │  │
│  │                                                                  │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │  MOBILE VIRTUAL CONTROLLER (Visible only on touch devices)       │  │
│  │                                                                  │  │
│  │   [ ESC ]    [ ▲ ]    [ ENTER ]         [ 1 ] [ 2 ] [ 3 ]        │  │
│  │   [ ◄ ]      [ ▼ ]    [ ► ]             [ 4 ] [ 5 ] [ 6 ]        │  │
│  │   [ SPACE ]  [ B ]    [ F ] [ R ]       [ 7 ] [ 8 ] [ 9 ]        │  │
│  └──────────────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────────────┘
```

### Key Components

1. **SDK Authentication:**
   * Utilizes `@discord/embedded-app-sdk`.
   * Obtains the Discord user authorization code and exchanges it with the bridge backend for an ephemeral session token.
2. **Terminal Renderer (`xterm.js`):**
   * Fixed 80-column by 25-row geometry.
   * `allowProposedApi: true` with canvas or WebGL renderer.
   * Classic VGA font (`Px437_IBM_VGA_8x16` or `Perfect DOS VGA 437`).
3. **Mobile Touch Adaptation:**
   * Desktop users drive the terminal with physical keyboard strokes.
   * Mobile Discord clients expose an on-screen touch overlay mapping soft taps directly to terminal escape sequences (Arrow keys, Return, Escape, Space, and primary menu hotkeys).

---

## 6. Daily Maintenance & The Discord Herald

Maintenance occurs completely asynchronously from player sessions.

```text
 ┌────────────────────────┐
 │   Cron / Systemd Timer │
 └───────────┬────────────┘
             │ Once every 24 hours (or blitz schedule)
             ▼
 ┌────────────────────────┐
 │   `nc-sysop maint run` │ ───► Processes movement, battles, builds, taxation
 └───────────┬────────────┘
             │
             │ Generates battle results & turn summaries
             ▼
 ┌────────────────────────┐
 │   Herald Dispatcher    │
 └───────────┬────────────┘
             │
             ▼
 ┌────────────────────────┐
 │  Discord Webhook API   │
 └───────────┬────────────┘
             │
             ▼
 ┌────────────────────────────────────────────────────────────────────┐
 │                  DISCORD: #galactic-bulletin                       │
 │                                                                    │
 │  📢 **GALACTIC GAZETTE - YEAR 2348**                               │
 │                                                                    │
 │  ⚔️ **Major Fleet Engagement at Sector (14, 08):**                  │
 │     • The Vorn Hegemony collided with 4th Imperial Taskforce       │
 │     • 3 Cruisers destroyed, 1 Starbase under siege                 │
 │                                                                    │
 │  🪐 **Planetary Uprisings:**                                       │
 │     • System *Capella IV* has declared independence                │
 │                                                                    │
 │  🏆 **Standings:** 1. Orion Syndicate (340 pts)                    │
 │                   2. Sol Federation (312 pts)                      │
 │                                                                    │
 │  ⏱️ *Orders for Year 2349 are now open. Maintenance in 23h 45m.*   │
 └────────────────────────────────────────────────────────────────────┘
```

### Herald Rules
* **No Secret Leaks:** Public bulletins only report battles, planetary rebellions, and score rankings.
* **Private Briefings (Optional):** The Herald can optionally send ephemeral or direct turn reports to individual players containing their specific fleet losses and revenue summaries.

---

## 7. Security & Sandbox Boundary

1. **Unprivileged Execution:** The Bridge Daemon and `nc-door` child processes run under a restricted service account (`nostrian:nostrian`) with no write permissions outside `/srv/nc/campaigns` and `/tmp/nc-nodes`.
2. **Resource Constraints:**
   * Max session duration enforced by dropfile (e.g. 30 minutes).
   * Process memory capped via `systemd` cgroups or container resource limits (maximum 64MB per active node).
3. **Network Isolation:** `nc-door` does not require outbound internet access. It communicates exclusively via local Unix sockets or loopback TCP.
4. **No Administrative Commands over Discord:** Sysop functions (creating games, modifying parameters, resetting seeds) are physically disabled in the bridge; they can only be executed by a sysop logged into the host console.
