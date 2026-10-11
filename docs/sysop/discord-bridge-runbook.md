# Discord & Web Door Sysop Runbook

This runbook guides sysops through deploying and operating **`nc-bridge`** (the Web/PTY door manager) and **`nc-herald`** (the chat broadcast dispatcher) to host Nostrian Conquest inside Discord Embedded Activities or standalone browsers while maintaining complete self-sovereignty.

---

## 1. Architecture Recap

```text
 ┌──────────────────────┐         ┌──────────────────────┐
 │   Discord Client     │         │   Standalone Browser │
 │  (Activity iframe)   │         │  (Direct HTTPS link) │
 └──────────┬───────────┘         └──────────┬───────────┘
            │                                │
            └───────────────┬────────────────┘
                            ▼
 ┌───────────────────────────────────────────────────────┐
 │       Ingress: Cloudflare Tunnel / Caddy (HTTPS)      │
 └──────────────────────────┬────────────────────────────┘
                            ▼
 ┌───────────────────────────────────────────────────────┐
 │               `nc-bridge` (Port 8080)                 │
 │   • Serves static xterm.js CP437 client               │
 │   • Allocates virtual nodes & generates DOOR.KDL      │
 │   • Bridges WebSocket <──> TCP loopback stream        │
 └─────────────┬───────────────────────────┬─────────────┘
               │ Ephemeral socket          │ Internal callbacks
               ▼                           ▼
 ┌──────────────────────────┐    ┌──────────────────────────┐
 │    `nc-door` Process     │    │   `nc-herald` (Port 8081)│
 │  • Reads DOOR.KDL        │    │  • Listens for events    │
 │  • Reads/writes ncgame.db│    │  • Dispatches Discord    │
 └─────────────┬────────────┘    │    webhook embeds        │
               │                 └─────────────┬────────────┘
               │ Staged                        ▼
               │ exports         ┌──────────────────────────┐
               ▼                 │  Discord #galactic-news  │
 ┌──────────────────────────┐    └──────────────────────────┘
 │ `nc-discord` Gateway Bot │
 │  • Serves /map, /report  │
 │  • Ephemeral responses   │
 └──────────────────────────┘
```

---

## 2. Prerequisites & Directory Layout

### Recommended Directory Structure

```text
/srv/nc/
├── bin/
│   ├── nc-bridge
│   ├── nc-herald
│   ├── nc-discord
│   ├── nc-door
│   └── nc-sysop
├── static/              # Web client assets (index.html, style.css, app.js)
├── campaigns/
│   └── capella/
│       ├── ncgame.db    # Campaign SQLite database
│       └── config.kdl   # Campaign parameters and player seat aliases
└── nodes/               # Temporary dropfile directory (/tmp/nc-nodes or /srv/nc/nodes)
```

Create a dedicated system user:
```bash
sudo useradd -r -s /usr/sbin/nologin -d /srv/nc nostrian
sudo chown -R nostrian:nostrian /srv/nc
```

---

## 3. Deploying `nc-bridge`

`nc-bridge` manages incoming WebSocket connections, writes `DOOR.KDL` dropfiles, and spawns `nc-door`.

### Systemd Service: `/etc/systemd/system/nc-bridge.service`

```ini
[Unit]
Description=Nostrian Conquest Web & Discord Door Bridge
After=network.target

[Service]
Type=simple
User=nostrian
Group=nostrian
WorkingDirectory=/srv/nc
Environment=NC_BRIDGE_HOST=127.0.0.1
Environment=NC_BRIDGE_PORT=8080
Environment=NC_BRIDGE_CAMPAIGN_DIR=/srv/nc/campaigns/capella
Environment=NC_BRIDGE_DOOR_BINARY=/srv/nc/bin/nc-door
Environment=NC_BRIDGE_STATIC_DIR=/srv/nc/static
Environment=NC_BRIDGE_NODE_DIR=/srv/nc/nodes
Environment=NC_BRIDGE_MAX_NODES=16
Environment=NC_BRIDGE_TIMEOUT_MINS=30
Environment=NC_BRIDGE_EVENT_FORWARD_URL=http://127.0.0.1:8081/events
ExecStart=/srv/nc/bin/nc-bridge
Restart=always
RestartSec=5

# Sandboxing
ProtectSystem=full
ProtectHome=true
NoNewPrivileges=true

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable --now nc-bridge
```

Verify service status:
```bash
curl http://127.0.0.1:8080/health
# {"active_sessions":0,"max_nodes":16,"service":"nc-bridge","status":"ok"}
```

---

## 4. Deploying `nc-herald` (Outbound Event Hub & Activity OAuth)

`nc-herald` runs as a lightweight daemon receiving real-time event alerts from `nc-bridge` and posting them to Discord webhooks, as well as exchanging Discord Activity OAuth2 codes.

### Systemd Service: `/etc/systemd/system/nc-herald.service`

```ini
[Unit]
Description=Nostrian Conquest Discord Herald Daemon
After=network.target

[Service]
Type=simple
User=nostrian
Group=nostrian
WorkingDirectory=/srv/nc
Environment=NC_HERALD_HOST=127.0.0.1
Environment=NC_HERALD_PORT=8081
Environment=NC_DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/YOUR_WEBHOOK_ID/YOUR_WEBHOOK_TOKEN
Environment=NC_DISCORD_CLIENT_ID=YOUR_DISCORD_CLIENT_ID
Environment=NC_DISCORD_CLIENT_SECRET=YOUR_DISCORD_CLIENT_SECRET
ExecStart=/srv/nc/bin/nc-herald listen
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable --now nc-herald
```

Verify the webhook with a test announcement:
```bash
/srv/nc/bin/nc-herald test --discord-webhook "$NC_DISCORD_WEBHOOK_URL"
```

---

## 5. Deploying `nc-discord` (Interactive Slash Command Bot)

`nc-discord` connects directly to the Discord Gateway using `poise` and `serenity`, registering interactive slash commands and delivering private player exports.

### Systemd Service: `/etc/systemd/system/nc-discord.service`

```ini
[Unit]
Description=Nostrian Conquest Discord Bot Daemon
After=network.target

[Service]
Type=simple
User=nostrian
Group=nostrian
WorkingDirectory=/srv/nc
Environment=NC_DISCORD_BOT_TOKEN=YOUR_BOT_TOKEN
Environment=NC_DISCORD_CAMPAIGN_DIR=/srv/nc/campaigns/capella
Environment=NC_DISCORD_BRIDGE_URL=https://play.yourdomain.com
ExecStart=/srv/nc/bin/nc-discord
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable --now nc-discord
```

### Slash Commands Registered by `nc-discord`

* `/status` — Public campaign stardate, sector name, and maintenance countdown.
* `/map [format]` — **Ephemeral (Fog of War safe)** starmap delivery. Optional dropdown choice:
  * `Excel Workbook (.xlsx)` (default) — styled spreadsheet with frozen navy headers, sortable numeric cells, and a second tab containing the printable ASCII grid. Opens in Microsoft Excel, LibreOffice Calc, Apple Numbers, and Google Sheets.
  * `All Formats` — attaches `.xlsx`, `.csv`, and `.txt` simultaneously.
  * `Printable ASCII Grid (.txt)` — classic terminal dot-matrix starmap grid.
  * `Raw CSV (.csv)` — raw comma-separated values.
* `/report` — **Ephemeral** classified turn briefing delivering `ECREPORT-*.TXT`.
* `/play` — Direct launcher card linking to the Web Door and Discord Activity.

---

## 6. Daily Maintenance & The Galactic Gazette

Schedule daily maintenance via cron or systemd timer. At the end of maintenance, trigger the public turn broadcast:

### Cron Job: `/etc/cron.d/nostrian-maintenance`

```cron
# Run turn cycle daily at 04:00 UTC and broadcast Gazette
0 4 * * * nostrian /srv/nc/bin/nc-sysop maint run --dir /srv/nc/campaigns/capella && /srv/nc/bin/nc-herald broadcast-turn --campaign "Capella Cluster" --year $(date +\%Y)
```

---

## 6. Public Ingress via Cloudflare Tunnel (Zero Port-Forwarding)

Discord Activities require a valid HTTPS/WSS domain. Using Cloudflare Tunnel (`cloudflared`) avoids opening router ports or exposing your host IP.

1. **Install cloudflared:**
   ```bash
   sudo apt install cloudflared
   ```
2. **Authenticate and create tunnel:**
   ```bash
   cloudflared tunnel login
   cloudflared tunnel create nc-gateway
   ```
3. **Configure tunnel (`~/.cloudflared/config.yml`):**
   ```yaml
   tunnel: nc-gateway
   credentials-file: /etc/cloudflared/credentials.json

   ingress:
     - hostname: play.yourdomain.com
       service: http://127.0.0.1:8080
     - service: http_status:404
   ```
4. **Route DNS and run as a service:**
   ```bash
   cloudflared tunnel route dns nc-gateway play.yourdomain.com
   sudo cloudflared service install
   sudo systemctl start cloudflared
   ```

Your web door is now reachable at `https://play.yourdomain.com` with valid TLS and WebSocket support.

---

## 7. Discord Developer Portal Setup

To launch Nostrian Conquest as a Discord Activity:

1. Open [Discord Developer Portal](https://discord.com/developers/applications) and create **New Application** (*"Nostrian Conquest"*).
2. Navigate to **Embedded App SDK** (Activities) settings:
   * **URL Mappings:** Set `https://play.yourdomain.com` as the primary URL.
   * Enable Activities for Voice and Text channels.
3. Save changes.
4. Join any voice channel on your Discord server, click the **Rocket (Start an Activity)** icon, select *Nostrian Conquest*, and click Launch.

---

## 8. Operational Troubleshooting

| Symptom | Likely Cause | Solution |
|---|---|---|
| `nc-door` fails to connect | Loopback port blocked or timeout | Ensure `nc-door` binary path is correct in `NC_BRIDGE_DOOR_BINARY`. Inspect `/srv/nc/nodes/node_N/door.kdl`. |
| Web client loads black screen | WebSocket connection blocked | Check browser console (F12). Ensure reverse proxy / tunnel allows WebSocket upgrades (`Connection: Upgrade`). |
| Events not appearing in Discord | Webhook token expired or invalid URL | Test directly with `/srv/nc/bin/nc-herald test --discord-webhook <URL>`. Check `journalctl -u nc-herald -e`. |
| Max nodes error on launch | Concurrent session cap hit | Adjust `NC_BRIDGE_MAX_NODES` in `/etc/systemd/system/nc-bridge.service` and reload. |
