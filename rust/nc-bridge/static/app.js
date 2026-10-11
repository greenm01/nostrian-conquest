// Nostrian Conquest — Web Door Client (xterm.js + WebSocket)

(function () {
    'use strict';

    // State
    let term = null;
    let fitAddon = null;
    let ws = null;
    let reconnectTimeout = null;

    // Elements
    const statusBadge = document.getElementById('status-badge');
    const mobileControls = document.getElementById('mobile-controls');
    const btnToggleTouch = document.getElementById('btn-toggle-touch');
    const btnFullscreen = document.getElementById('btn-fullscreen');

    // 1. Initialize Terminal
    function initTerminal() {
        term = new Terminal({
            cols: 80,
            rows: 25,
            cursorBlink: true,
            cursorStyle: 'block',
            fontFamily: "'VT323', 'Courier New', monospace",
            fontSize: 18,
            lineHeight: 1.1,
            letterSpacing: 0,
            theme: {
                background: '#000000',
                foreground: '#aaaaaa',
                cursor: '#33ff66',
                black: '#000000',
                red: '#aa0000',
                green: '#00aa00',
                yellow: '#aa5500',
                blue: '#0000aa',
                magenta: '#aa00aa',
                cyan: '#00aaaa',
                white: '#aaaaaa',
                brightBlack: '#555555',
                brightRed: '#ff5555',
                brightGreen: '#55ff55',
                brightYellow: '#ffff55',
                brightBlue: '#5555ff',
                brightMagenta: '#ff55ff',
                brightCyan: '#55ffff',
                brightWhite: '#ffffff'
            }
        });

        if (window.FitAddon && window.FitAddon.FitAddon) {
            fitAddon = new window.FitAddon.FitAddon();
            term.loadAddon(fitAddon);
        }

        const container = document.getElementById('terminal');
        term.open(container);

        if (fitAddon) {
            fitAddon.fit();
            window.addEventListener('resize', () => {
                try { fitAddon.fit(); } catch (e) {}
            });
        }

        // Send keystrokes to WebSocket
        term.onData((data) => {
            if (ws && ws.readyState === WebSocket.OPEN) {
                ws.send(data);
            }
        });
    }

    // 2. Resolve User Profile (Discord Activity vs Standalone Browser)
    async function resolveIdentity() {
        const urlParams = new URLSearchParams(window.location.search);
        let handle = urlParams.get('handle');
        let platformId = urlParams.get('platform_id');
        let displayName = urlParams.get('display_name');

        // Check if running inside Discord Embedded Activity
        if (window.DiscordSDK) {
            try {
                const clientId = urlParams.get('client_id') || '109283746592837465';
                const discordSdk = new window.DiscordSDK.DiscordSDK(clientId);
                await discordSdk.ready();

                // Discord Activity context resolved
                if (discordSdk.user) {
                    handle = discordSdk.user.username;
                    platformId = discordSdk.user.id;
                    displayName = discordSdk.user.global_name || discordSdk.user.username;
                }
            } catch (err) {
                console.warn('Discord SDK not initialized or not in Discord:', err);
            }
        }

        // Fallbacks
        if (!handle) {
            handle = 'player_' + Math.floor(Math.random() * 900 + 100);
        }
        if (!platformId) {
            platformId = 'web_' + handle;
        }

        return { handle, platformId, displayName };
    }

    // 3. Connect to WebSocket
    async function connectWebSocket() {
        if (ws) {
            try { ws.close(); } catch (e) {}
            ws = null;
        }

        updateStatus('connecting');

        const identity = await resolveIdentity();
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const wsUrl = `${protocol}//${window.location.host}/ws/connect?handle=${encodeURIComponent(identity.handle)}&platform_id=${encodeURIComponent(identity.platformId)}` +
            (identity.displayName ? `&display_name=${encodeURIComponent(identity.displayName)}` : '');

        term.writeln('\x1b[33mConnecting to Nostrian Conquest door bridge...\x1b[0m\r');

        try {
            ws = new WebSocket(wsUrl);
            ws.binaryType = 'arraybuffer';

            ws.onopen = () => {
                updateStatus('connected');
                term.clear();
            };

            ws.onmessage = (event) => {
                if (event.data instanceof ArrayBuffer) {
                    const bytes = new Uint8Array(event.data);
                    term.write(bytes);
                } else if (typeof event.data === 'string') {
                    term.write(event.data);
                }
            };

            ws.onclose = () => {
                updateStatus('disconnected');
                term.writeln('\r\n\x1b[31m[Session disconnected. Reconnecting in 5s...]\x1b[0m\r\n');
                scheduleReconnect();
            };

            ws.onerror = (err) => {
                console.error('WebSocket error:', err);
                updateStatus('disconnected');
            };
        } catch (e) {
            console.error('Failed to instantiate WebSocket:', e);
            updateStatus('disconnected');
            scheduleReconnect();
        }
    }

    function scheduleReconnect() {
        if (reconnectTimeout) clearTimeout(reconnectTimeout);
        reconnectTimeout = setTimeout(() => {
            connectWebSocket();
        }, 5000);
    }

    function updateStatus(status) {
        statusBadge.className = 'badge ' + status;
        statusBadge.textContent = status.toUpperCase();
    }

    // 4. Mobile Controls & Virtual Keypad
    function setupControls() {
        const vkeys = document.querySelectorAll('.vkey');
        vkeys.forEach((btn) => {
            const sendKey = (e) => {
                e.preventDefault();
                const key = btn.getAttribute('data-key');
                if (key && ws && ws.readyState === WebSocket.OPEN) {
                    // Translate escape sequences
                    const payload = key.replace(/\\x1b/g, '\x1b').replace(/\\r/g, '\r');
                    ws.send(payload);
                }
            };

            btn.addEventListener('touchstart', sendKey, { passive: false });
            btn.addEventListener('click', sendKey);
        });

        // Toggle virtual controller manually
        btnToggleTouch.addEventListener('click', () => {
            if (mobileControls.classList.contains('hidden')) {
                mobileControls.classList.remove('hidden');
                mobileControls.classList.remove('manual-hidden');
            } else {
                mobileControls.classList.add('hidden');
                mobileControls.classList.add('manual-hidden');
            }
            if (fitAddon) setTimeout(() => fitAddon.fit(), 100);
        });

        // Fullscreen toggle
        btnFullscreen.addEventListener('click', () => {
            if (!document.fullscreenElement) {
                document.documentElement.requestFullscreen().catch(() => {});
            } else {
                if (document.exitFullscreen) {
                    document.exitFullscreen().catch(() => {});
                }
            }
        });
    }

    // Boot
    window.addEventListener('DOMContentLoaded', () => {
        initTerminal();
        setupControls();
        connectWebSocket();
    });
})();
