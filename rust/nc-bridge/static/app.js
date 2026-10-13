// Nostrian Conquest — Web Door Client (xterm.js + WebGL + WebSocket)

(function () {
    'use strict';

    // State
    let term = null;
    let fitAddon = null;
    let webglAddon = null;
    let ws = null;
    let reconnectTimeout = null;

    // Elements
    const statusBadge = document.getElementById('status-badge');
    const mobileControls = document.getElementById('mobile-controls');
    const btnToggleTouch = document.getElementById('btn-toggle-touch');
    const btnToggleCrt = document.getElementById('btn-toggle-crt');
    const btnFontInc = document.getElementById('btn-font-inc');
    const btnFontDec = document.getElementById('btn-font-dec');
    const btnFullscreen = document.getElementById('btn-fullscreen');
    const crtFrame = document.getElementById('crt-frame');

    // Compute font size to make 80x25 fit comfortably on screen without being oversized
    function computeOptimalFontSize() {
        const saved = localStorage.getItem('nc_font_size');
        if (saved) {
            const parsed = parseInt(saved, 10);
            if (!isNaN(parsed) && parsed >= 12 && parsed <= 30) {
                return parsed;
            }
        }
        const wrapper = document.getElementById('terminal-wrapper');
        if (!wrapper) return 18;
        // Reserve space for wrapper padding (24px*2) + frame padding (20px*2) + bezel border
        const availW = Math.max(300, wrapper.clientWidth - 110);
        const availH = Math.max(200, wrapper.clientHeight - 110);
        // JetBrains Mono width/height is ~0.61; with lineHeight 1.0, row height is strictly fontSize
        const sizeW = Math.floor(availW / (80 * 0.61));
        const sizeH = Math.floor(availH / 25.5);
        // Comfortable default range: clamp between 14px and 20px
        const fit = Math.min(sizeW, sizeH);
        return Math.max(14, Math.min(fit, 20));
    }

    function setFontSize(newSize) {
        const clamped = Math.max(12, Math.min(newSize, 30));
        if (term) {
            term.options.fontSize = clamped;
            term.resize(80, 25);
        }
        localStorage.setItem('nc_font_size', clamped.toString());
    }

    // 1. Initialize Terminal
    function initTerminal() {
        const initialSize = computeOptimalFontSize();

        term = new Terminal({
            cols: 80,
            rows: 25,
            scrollback: 0,
            customGlyphs: true,
            cursorBlink: true,
            cursorStyle: 'block',
            fontFamily: "'Perfect DOS VGA 437', 'JetBrains Mono', 'Cascadia Code', 'Consolas', monospace",
            fontSize: initialSize,
            lineHeight: 1.0,
            letterSpacing: 0,
            allowProposedApi: true,
            theme: {
                background: '#000000',
                foreground: '#cccccc',
                cursor: '#33ff66',
                black: '#000000',
                red: '#cc0000',
                green: '#00cc00',
                yellow: '#cccc00',
                blue: '#0000cc',
                magenta: '#cc00cc',
                cyan: '#00cccc',
                white: '#bbbbbb',
                brightBlack: '#666666',
                brightRed: '#ff5555',
                brightGreen: '#55ff55',
                brightYellow: '#ffff55',
                brightBlue: '#5555ff',
                brightMagenta: '#ff55ff',
                brightCyan: '#55ffff',
                brightWhite: '#ffffff'
            }
        });

        const container = document.getElementById('terminal');
        term.open(container);
        term.resize(80, 25);
        term.focus();

        // Hardware-accelerated WebGL or Canvas rendering for seamless vector box-drawing
        let rendererActive = false;
        if (window.WebglAddon && window.WebglAddon.WebglAddon) {
            try {
                webglAddon = new window.WebglAddon.WebglAddon();
                webglAddon.onContextLoss(() => {
                    webglAddon.dispose();
                });
                term.loadAddon(webglAddon);
                rendererActive = true;
                console.log('xterm.js WebGL GPU renderer loaded (seamless vector box-drawing active)');
            } catch (err) {
                console.warn('WebGL addon failed, falling back to Canvas renderer:', err);
            }
        }

        if (!rendererActive && window.CanvasAddon && window.CanvasAddon.CanvasAddon) {
            try {
                const canvasAddon = new window.CanvasAddon.CanvasAddon();
                term.loadAddon(canvasAddon);
                rendererActive = true;
                console.log('xterm.js Canvas 2D renderer loaded (seamless vector box-drawing active)');
            } catch (err) {
                console.warn('Canvas addon failed:', err);
            }
        }

        window.addEventListener('resize', () => {
            if (!localStorage.getItem('nc_font_size')) {
                const newSize = computeOptimalFontSize();
                if (term && term.options.fontSize !== newSize) {
                    term.options.fontSize = newSize;
                    term.resize(80, 25);
                }
            }
        });

        // Global auto-focus capture: immediately focus terminal on click or keypress
        window.addEventListener('click', (e) => {
            if (e.target.closest('#header-bar') || e.target.closest('#mobile-controls')) {
                return;
            }
            if (term) term.focus();
        });

        window.addEventListener('keydown', (e) => {
            if (!term) return;
            if (document.activeElement !== term.textarea && !term.hasSelection()) {
                term.focus();
            }
        });

        // Intercept Ctrl+C / Cmd+C (copy to clipboard) and Ctrl+V (paste)
        term.attachCustomKeyEventHandler((event) => {
            if (event.ctrlKey || event.metaKey) {
                // If Ctrl+C or Cmd+C, copy selected text and prevent sending 0x03 interrupt
                if (event.key === 'c' || event.key === 'C') {
                    if (term.hasSelection()) {
                        navigator.clipboard.writeText(term.getSelection()).catch(() => {});
                    }
                    return false; // Prevent sending SIGINT (0x03) to nc-door child process!
                }
                // If Ctrl+V or Cmd+V, paste from system clipboard
                if (event.key === 'v' || event.key === 'V') {
                    if (event.type === 'keydown') {
                        navigator.clipboard.readText().then((text) => {
                            if (ws && ws.readyState === WebSocket.OPEN) {
                                ws.send(text);
                            }
                        }).catch(() => {});
                    }
                    return false;
                }
            }
            return true;
        });

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
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const game = new URLSearchParams(window.location.search).get('game');
        if (game) {
            document.getElementById('games-link').hidden = false;
            try {
                const response = await fetch(`/api/games/${encodeURIComponent(game)}/access`, {credentials: 'same-origin'});
                if (response.status === 401) {location.href = '/'; return;}
                const access = await response.json();
                if (!response.ok || !access.can_play) {
                    term.writeln(`\r\n${access.error || 'Campaign is not accepting sessions.'} Return to Games.\r\n`);
                    updateStatus('disconnected'); return;
                }
                if (access.connected) {
                    term.writeln('\r\nYour seat is already open in another terminal. Return to Games and choose Reconnect.\r\n');
                    updateStatus('disconnected'); return;
                }
            } catch (_) {
                term.writeln('\r\nCannot reach the game host. Return to Games and try again.\r\n');
                updateStatus('disconnected'); return;
            }
        }
        const identity = game ? null : await resolveIdentity();
        const wsUrl = game
            ? `${protocol}//${window.location.host}/games/${encodeURIComponent(game)}/ws`
            : `${protocol}//${window.location.host}/ws/connect?handle=${encodeURIComponent(identity.handle)}&platform_id=${encodeURIComponent(identity.platformId)}&display_name=${encodeURIComponent(identity.displayName || identity.handle)}`;

        try {
            ws = new WebSocket(wsUrl);
            ws.binaryType = 'arraybuffer';

            ws.onopen = () => {
                updateStatus('connected');
                term.reset();
                term.resize(80, 25);
                term.focus();
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
                term.writeln('\r\n[Session ended. Return to the game list to reconnect.]\r\n');
                if (!game) scheduleReconnect();
            };

            ws.onerror = (err) => {
                console.error('WebSocket error:', err);
                updateStatus('disconnected');
            };
        } catch (e) {
            console.error('Failed to instantiate WebSocket:', e);
            updateStatus('disconnected');
            if (!game) scheduleReconnect();
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
                    const payload = key.replace(/\\x1b/g, '\x1b').replace(/\\r/g, '\r');
                    ws.send(payload);
                }
            };

            btn.addEventListener('touchstart', sendKey, { passive: false });
            btn.addEventListener('click', sendKey);
        });

        // Font scaling buttons A- and A+
        if (btnFontInc) {
            btnFontInc.addEventListener('click', () => {
                const current = term ? term.options.fontSize : 18;
                setFontSize(current + 2);
            });
        }
        if (btnFontDec) {
            btnFontDec.addEventListener('click', () => {
                const current = term ? term.options.fontSize : 18;
                setFontSize(current - 2);
            });
        }

        // Toggle virtual controller manually
        btnToggleTouch.addEventListener('click', () => {
            if (mobileControls.classList.contains('hidden')) {
                mobileControls.classList.remove('hidden');
                mobileControls.classList.remove('manual-hidden');
            } else {
                mobileControls.classList.add('hidden');
                mobileControls.classList.add('manual-hidden');
            }
        });

        // Toggle CRT Scanlines
        if (btnToggleCrt && crtFrame) {
            btnToggleCrt.addEventListener('click', () => {
                if (crtFrame.classList.contains('crt-active')) {
                    crtFrame.classList.remove('crt-active');
                    btnToggleCrt.textContent = '📺 CRT: OFF';
                } else {
                    crtFrame.classList.add('crt-active');
                    btnToggleCrt.textContent = '📺 CRT: ON';
                }
            });
        }

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
