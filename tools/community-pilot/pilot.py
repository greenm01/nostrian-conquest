# /// script
# requires-python = ">=3.12,<3.14"
# dependencies = ["pyte", "coincurve", "websockets", "httpx", "playwright"]
# ///
"""The t121 private pilot: two disposable games played end to end.

    uv run tools/community-pilot/pilot.py --local               # loopback stack
    uv run tools/community-pilot/pilot.py --remote archvps      # live server

Players sign in with real NIP-98 Nostr logins from throwaway keys (and with a
spare Matrix account when the Keychain item `nc-pilot-matrix` holds its
password). Terminals are real doors; their bytes go through a terminal
emulator to read the 80x25 screen. Operator steps send the same JSON requests
as `nc-admin` to `nc-host admin`, locally or over SSH as `nccommunity`.

Only games named `pilot-*` are changed. On the live server the pilot backs up
every listed game first, leaves the turn timer to run turns, and archives its
games at the end. Results go to stdout as JSON lines and a summary.
"""
import argparse
import asyncio
import base64
import concurrent.futures
import hashlib
import json
import os
import re
import shlex
import socket
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path

import coincurve
import httpx
import pyte
import websockets

REPO = Path(__file__).resolve().parents[2]
RUST = REPO / 'rust'
REMOTE_ROOT = '/srv/nc/community/games'
REMOTE_HOST_BIN = '/opt/nc-community/current/bin/nc-host'
REMOTE_ORIGIN = 'https://play.nostrian-conquest.com'
READ_ONLY = {'list', 'inspect', 'history', 'list_invites', 'verify_backup', 'backup'}


class Failed(Exception):
    pass


# ---- operator side ---------------------------------------------------------

class Admin:
    """`nc-host admin` requests, refusing writes to any game not named pilot-*."""

    def __init__(self, remote=None, root=None, host_bin=None):
        self.remote, self.root, self.host_bin = remote, root, host_bin

    def command(self, root=None):
        root = root or self.root
        if self.remote:
            return ['ssh', self.remote, f'sudo -u nccommunity {REMOTE_HOST_BIN} admin --root {shlex.quote(root)}']
        return [str(self.host_bin), 'admin', '--root', str(root)]

    def __call__(self, root=None, **request):
        game = request.get('game', '')
        if request['action'] not in READ_ONLY and request['action'] != 'restore' and not game.startswith('pilot-'):
            raise Failed(f'refusing {request["action"]} on {game or "all games"}')
        done = subprocess.run(self.command(root), input=json.dumps(request), text=True, capture_output=True)
        try:
            out = json.loads(done.stdout.strip().splitlines()[-1])
        except (IndexError, json.JSONDecodeError):
            raise Failed(f'{request["action"]}: {done.stderr.strip() or done.stdout.strip()}')
        if not out['ok']:
            raise Failed(f'{request["action"]}: {out["error"]}')
        return out['data']

    def shell(self, script):
        """Run a shell snippet as nccommunity (remote) or locally."""
        if self.remote:
            cmd = ['ssh', self.remote, f'sudo -u nccommunity sh -c {shlex.quote(script)}']
        else:
            cmd = ['sh', '-c', script]
        return subprocess.run(cmd, text=True, capture_output=True)

    def doors(self):
        """Running door processes (remote) or open terminal leases (local)."""
        if self.remote:
            out = subprocess.run(['ssh', self.remote, "pgrep -c -f '/bin/[n]c-door' || true"], text=True, capture_output=True)
            return int(out.stdout.strip() or 0)
        return len(list((Path(self.root) / '.nodes').glob('*')))


# ---- players ---------------------------------------------------------------

def nostr_login_event(secret, origin, body):
    key = coincurve.PrivateKey(secret)
    pubkey = key.public_key_xonly.format().hex()
    event = {'pubkey': pubkey, 'kind': 27235, 'created_at': int(time.time()), 'content': '',
             'tags': [['u', f'{origin}/auth/nostr'], ['method', 'POST'],
                      ['payload', hashlib.sha256(body.encode()).hexdigest()]]}
    serial = json.dumps([0, pubkey, event['created_at'], event['kind'], event['tags'], event['content']],
                        separators=(',', ':'), ensure_ascii=False)
    event['id'] = hashlib.sha256(serial.encode()).hexdigest()
    event['sig'] = key.sign_schnorr(bytes.fromhex(event['id'])).hex()
    return event


class Player:
    def __init__(self, origin, name):
        self.origin, self.name = origin, name
        self.http = httpx.Client(base_url=origin, headers={'Origin': origin}, timeout=30)
        self.secret = None

    def sign_in_nostr(self, secret=None):
        self.secret = secret or os.urandom(32)
        challenge = self.http.post('/auth/nostr/challenge').json()['challenge']
        body = json.dumps({'challenge': challenge}, separators=(',', ':'))
        event = nostr_login_event(self.secret, self.origin, body)
        auth = 'Nostr ' + base64.b64encode(json.dumps(event).encode()).decode()
        response = self.http.post('/auth/nostr', content=body,
                                  headers={'Content-Type': 'application/json', 'Authorization': auth})
        if response.status_code != 200:
            raise Failed(f'{self.name}: Nostr sign-in {response.status_code} {response.text}')
        self.account = self.me()['account']
        return self

    def me(self):
        return self.http.get('/api/me').json()

    def join(self, game, code=None):
        return self.http.post(f'/api/games/{game}/join', json={'code': code} if code else {})

    def access(self, game):
        return self.http.get(f'/api/games/{game}/access').json()

    @property
    def cookie(self):
        return '; '.join(f'{k}={v}' for k, v in self.http.cookies.items())


class Terminal:
    """One door session over the game WebSocket, read through pyte at 80x25."""

    def __init__(self, player, game):
        self.player, self.game = player, game
        self.screen = pyte.Screen(80, 25)
        self.stream = pyte.ByteStream(self.screen)
        self.ws, self.closed, self.bytes = None, False, 0

    async def open(self):
        url = self.player.origin.replace('http', 'ws', 1) + f'/games/{self.game}/ws'
        try:
            self.ws = await websockets.connect(url, additional_headers={'Origin': self.player.origin,
                                                                       'Cookie': self.player.cookie})
        except websockets.InvalidStatus as error:
            return error.response.status_code
        self.reader = asyncio.create_task(self._read())
        return 101

    async def _read(self):
        try:
            async for data in self.ws:
                self.bytes += len(data)
                self.stream.feed(data if isinstance(data, bytes) else data.encode())
        except websockets.ConnectionClosed:
            pass
        self.closed = True

    def text(self):
        return '\n'.join(self.screen.display)

    async def wait_for(self, pattern, timeout=15):
        end = time.monotonic() + timeout
        while time.monotonic() < end:
            match = re.search(pattern, self.text())
            if match:
                return match
            if self.closed:
                break
            await asyncio.sleep(0.1)
        raise Failed(f'{self.player.name}/{self.game}: waiting for {pattern!r}; screen:\n{self.text()}')

    async def send(self, keys, pause=0.3):
        await self.ws.send(keys)
        await asyncio.sleep(pause)

    async def wait_closed(self, timeout=15):
        end = time.monotonic() + timeout
        while not self.closed and time.monotonic() < end:
            await asyncio.sleep(0.1)
        return self.closed

    async def close(self):
        if self.ws:
            await self.ws.close()
            await self.wait_closed(5)


async def setup_empire(term, empire, homeworld):
    await term.wait_for(r'View the game introduction')
    await term.send('n')
    await term.wait_for(r'FIRST TIME COMMAND')
    await term.send('j')
    await term.wait_for(r'EMPIRE NAME <-')
    await term.send(empire + '\r')
    await term.wait_for(r'Is this correct\? \[Y\]/N')
    await term.send('y')
    for _ in range(2):
        await term.wait_for(r'slap a key')
        await term.send('\r')
    await term.wait_for(r'HOMEWORLD <- Name this world')
    await term.send(homeworld + '\r')
    await term.wait_for(r'Is this correct\? Y/\[N\]')
    await term.send('y')
    await term.wait_for(r'MAIN COMMAND <-')


async def reach_main_menu(term):
    """From a returning player's login screens to the main menu."""
    end = time.monotonic() + 30
    while time.monotonic() < end:
        screen = term.text()
        if 'MAIN COMMAND <-' in screen:
            return
        if 'View the game introduction' in screen:
            await term.send('n')
        elif 'slap a key' in screen or 'more' in screen.lower():
            await term.send('\r')
        else:
            await asyncio.sleep(0.3)
    raise Failed(f'{term.player.name}: no main menu; screen:\n{term.text()}')


async def tax(term, rate=None):
    """Read the empire tax rate, and set it when `rate` is given."""
    await term.send('p')
    await term.wait_for(r'PLANET COMMAND <-')
    await term.send('t')
    current = int((await term.wait_for(r'\(0 - 100\) \[(\d+)\]')).group(1))
    if rate is None:
        await term.send('\x1b')
    else:
        await term.send(f'{rate}\r')
    await term.wait_for(r'PLANET COMMAND <-')
    await term.send('q')
    await term.wait_for(r'MAIN COMMAND <-')
    return current


# ---- the pilot -------------------------------------------------------------

def archive(admin, game):
    if admin(action='inspect', game=game)['state'] == 'running':
        admin(action='pause', game=game)
    admin(action='archive', game=game)


class Pilot:
    def __init__(self, admin, origin, tag):
        self.admin, self.origin, self.tag = admin, origin, tag
        self.results = []
        self.a, self.b = f'pilot-a-{tag}', f'pilot-b-{tag}'

    def check(self, name, ok, detail=''):
        self.results.append({'check': name, 'ok': None if ok is None else bool(ok), 'detail': detail})
        print(json.dumps(self.results[-1]), flush=True)
        return ok

    def inspect(self, game):
        return self.admin(action='inspect', game=game)

    def games(self):
        listed = self.admin(action='list')
        return {g['game']: g for g in (listed['games'] if isinstance(listed, dict) else listed)}

    def wait_year(self, game, year, timeout):
        end = time.monotonic() + timeout
        while time.monotonic() < end:
            state = self.inspect(game)
            if state['year'] >= year and state['state'] != 'maintenance':
                return state
            time.sleep(5)
        return self.inspect(game)

    def run(self, matrix=None, browser=True):
        admin = self.admin
        before = self.games()
        for game in before:
            if not game.startswith('pilot-'):
                path = admin(action='backup', game=game).get('path')
                self.check(f'backup {game} before the pilot', True, str(path))
        doors_at_start = admin.doors()

        admin(action='create', game=self.a, name=f'Pilot A ({self.tag}, test)', players=4, seed=None)
        admin(action='create', game=self.b, name=f'Pilot B ({self.tag}, test)', players=2, seed=None)
        for game in (self.a, self.b):
            admin(action='publish', game=game)
        listed = {g['game'] for g in httpx.get(self.origin + '/api/games', timeout=30).json()}
        self.check('both pilot games listed', {self.a, self.b} <= listed)

        players = [Player(self.origin, f'nostr-{i}').sign_in_nostr() for i in range(1, 4)]
        p1, p2, p3 = players
        self.check('Nostr sign-in (NIP-98) for three keys', all(p.account['provider'] == 'nostr' for p in players))

        seats = {p.name: p.join(self.a).json().get('seat') for p in players}
        self.check('three players join pilot A', sorted(seats.values()) == [1, 2, 3], seats)
        again = p1.join(self.a)
        claimed = self.inspect(self.a)['claimed']
        self.check('joining again keeps the same seat, no second claim',
                   again.json().get('seat') == seats['nostr-1'] and claimed == 3,
                   f'{again.status_code}, seat {again.json().get("seat")}, {claimed} claimed')

        if matrix:
            mp = matrix.sign_in(Player(self.origin, 'matrix'))
            self.check('Matrix sign-in by DM code', mp.account['provider'] == 'matrix', mp.account)
            seat = mp.join(self.a).json().get('seat')
            self.check('Matrix player joins pilot A', seat == 4, seat)
        else:
            self.check('Matrix sign-in', None, 'skipped: no nc-pilot-matrix Keychain item')

        # Two seats in pilot B: one claim, then a race for the last seat.
        self.check('player 1 joins pilot B', p1.join(self.b).status_code == 200)
        with concurrent.futures.ThreadPoolExecutor(2) as pool:
            race = list(pool.map(lambda p: p.join(self.b).status_code, (p2, p3)))
        self.check('race for the last seat: exactly one wins', sorted(race)[0] == 200 and race.count(200) == 1, race)
        loser = p3 if race[0] == 200 else p2

        own = p1.http.get(f'/api/games/{self.b}/exports/map-zip')
        self.check('own map ZIP download', own.status_code == 200 and own.content[:2] == b'PK',
                   f'{own.status_code}, {len(own.content)} bytes')
        other = loser.http.get(f'/api/games/{self.b}/exports/map-zip')
        self.check('map ZIP refused without a seat', other.status_code != 200, other.status_code)
        signed_out = httpx.get(f'{self.origin}/api/games/{self.a}/exports/map-zip', timeout=30)
        self.check('map ZIP refused signed out', signed_out.status_code != 200, signed_out.status_code)

        asyncio.run(self.terminals(p1, p2, p3))

        # Independent deadlines: A due in 2 minutes, B in 4, each on its own interval.
        now = int(time.time())
        admin(action='start', game=self.a, first_due=now + 120, interval_minutes=60)
        admin(action='start', game=self.b, first_due=now + 240, interval_minutes=1440)
        a = self.wait_year(self.a, 3001, 300)
        b = self.inspect(self.b)
        self.check('pilot A turn runs on its deadline (turn timer)', a['year'] == 3001, a['year'])
        self.check('pilot B untouched by pilot A deadline', b['year'] == 3000, b['year'])
        b = self.wait_year(self.b, 3001, 300)
        self.check('pilot B turn runs on its own deadline', b['year'] == 3001, b['year'])

        asyncio.run(self.after_turn(p1, p2, p3))
        self.pause_resume(p1)
        self.killed_turns()
        self.backup_restore()
        if browser:
            self.browser(p1)
        asyncio.run(self.logout(p2))

        time.sleep(3)
        doors = admin.doors()
        self.check('door processes cleaned up', doors <= doors_at_start, f'{doors} now, {doors_at_start} at start')

        for game in (self.a, self.b):
            archive(admin, game)
        listed = {g['game'] for g in httpx.get(self.origin + '/api/games', timeout=30).json()}
        self.check('pilot games archived and unlisted', not ({self.a, self.b} & listed))
        after = self.games()
        for game, state in before.items():
            if game.startswith('pilot-'):
                continue
            keep = ('year', 'state', 'claimed', 'next_due')
            same = all(after[game][k] == state[k] for k in keep)
            self.check(f'{game} unchanged by the pilot', same, {k: after[game][k] for k in keep})

    async def terminals(self, p1, p2, p3):
        # Players 1 and 2 set up and give orders in pilot A at the same time.
        t1, t2 = Terminal(p1, self.a), Terminal(p2, self.a)
        opened = [await t1.open(), await t2.open()]
        self.check('two terminals open in one game', opened == [101, 101], opened)
        await asyncio.gather(setup_empire(t1, 'Pilot One', 'Alpha Prime'),
                             setup_empire(t2, 'Pilot Two', 'Beta Prime'))
        await asyncio.gather(tax(t1, 37), tax(t2, 41))
        rates = await asyncio.gather(tax(t1), tax(t2))
        self.check('simultaneous orders both saved', rates == [37, 41], rates)
        grid = t1.screen.lines == 25 and t1.screen.columns == 80 and '�' not in t1.text()
        self.check('door screen is 80x25 with no undecodable bytes', grid)
        self.check('main menu renders', 'MAIN MENU' in t1.text())

        # Reconnect: a second terminal is refused until the first is closed.
        access = p1.access(self.a)
        self.check('access reports the open terminal', access.get('connected') is True, access)
        dup = Terminal(p1, self.a)
        status = await dup.open()
        self.check('second terminal for the same seat refused', status != 101, status)
        p1.http.post(f'/api/games/{self.a}/reconnect')
        self.check('Reconnect closes the old terminal', await t1.wait_closed(15))
        t1b = Terminal(p1, self.a)
        self.check('Play again after Reconnect', await t1b.open() == 101)
        await t1b.wait_for(r'View the game introduction|MAIN COMMAND|slap a key')
        await reach_main_menu(t1b)
        self.check('orders still there after Reconnect', await tax(t1b) == 37)

        # Player 3 sets up too, then everyone leaves before the turn.
        t3 = Terminal(p3, self.a)
        await t3.open()
        await setup_empire(t3, 'Pilot Three', 'Gamma Prime')
        for term in (t1b, t2, t3):
            await term.close()
        await asyncio.sleep(2)
        self.check('closed terminals release their seats',
                   not any(p.access(self.a).get('connected') for p in (p1, p2, p3)))

    async def after_turn(self, p1, p2, p3):
        rates = []
        for player in (p1, p2):
            term = Terminal(player, self.a)
            await term.open()
            await reach_main_menu(term)
            rates.append(await tax(term))
            await term.close()
        self.check('orders survive the turn', rates == [37, 41], rates)

    def pause_resume(self, p1):
        self.admin(action='pause', game=self.a)
        access = p1.access(self.a)
        status = asyncio.run(Terminal(p1, self.a).open())
        self.check('paused game refuses play', not access.get('can_play') and status != 101,
                   f'can_play={access.get("can_play")}, terminal {status}')
        self.admin(action='resume', game=self.a, first_due=int(time.time()) + 86400)
        access = p1.access(self.a)
        self.check('resumed game allows play', access.get('can_play') is True, access)

    def killed_turns(self):
        """Kill `maintain` at several points; each turn must still run exactly once."""
        game = self.b
        self.admin(action='schedule', game=game, first_due=int(time.time()) + 86400, interval_minutes=1440)
        for delay in (0.0, 0.005, 0.01, 0.02, 0.03, 0.05, 0.1):
            year = self.inspect(game)['year']
            request_id = f'{game}-kill-{year}'
            request = json.dumps({'action': 'maintain', 'game': game, 'expected_year': year,
                                  'request_id': request_id})
            host = REMOTE_HOST_BIN if self.admin.remote else str(self.admin.host_bin)
            root = REMOTE_ROOT if self.admin.remote else str(self.admin.root)
            script = (f'printf %s {shlex.quote(request)} | {host} admin --root {root} >/dev/null 2>&1 & '
                      f'pid=$!; sleep {delay}; kill -9 $pid 2>/dev/null && echo killed || echo finished; wait')
            outcome = self.admin.shell(script).stdout.strip()
            mid = self.inspect(game)
            phase = 'after the year was saved' if mid['year'] == year + 1 else 'before the year was saved'
            if mid['year'] == year or mid['unfinished_runs'] or mid['state'] == 'maintenance':
                self.admin(action='maintain', game=game, expected_year=year, request_id=request_id)
            end = self.inspect(game)
            ok = end['year'] == year + 1 and not end['unfinished_runs'] and end['state'] != 'maintenance'
            self.check(f'turn killed at {delay}s ({outcome}, {phase}) runs once', ok,
                       {'year': end['year'], 'state': end['state'], 'unfinished': end['unfinished_runs']})

    def backup_restore(self):
        backup = self.admin(action='backup', game=self.a)
        path = backup.get('path') if isinstance(backup, dict) else backup
        verified = self.admin(action='verify_backup', path=path)
        self.check('backup created and verified', bool(verified), path)
        if self.admin.remote:
            into = self.admin.shell('mktemp -d /tmp/nc-pilot-restore-XXXXXX').stdout.strip()
        else:
            into = tempfile.mkdtemp(prefix='nc-pilot-restore-')
        try:
            self.admin(root=into, action='restore', path=path, game=self.a)
            restored = self.admin(root=into, action='inspect', game=self.a)
            live = self.inspect(self.a)
            self.check('backup restores into an isolated root', restored['year'] == live['year'],
                       f'restored year {restored["year"]}, live {live["year"]}')
        finally:
            self.admin.shell(f'rm -rf {shlex.quote(into)}')

    async def logout(self, player):
        term = Terminal(player, self.a)
        await term.open()
        await reach_main_menu(term)
        player.http.post('/auth/logout')
        self.check('sign-out closes the open terminal', await term.wait_closed(15))
        self.check('signed out: /api/me refuses', player.http.get('/api/me').status_code == 401)
        status = await Terminal(player, self.a).open()
        self.check('signed out: terminal refused', status != 101, status)

    def browser(self, player):
        from playwright.sync_api import sync_playwright
        secret = player.secret
        shots = Path(tempfile.gettempdir()) / f'nc-pilot-{self.tag}'
        shots.mkdir(exist_ok=True)
        with sync_playwright() as pw:
            browser = pw.chromium.launch()
            page = browser.new_page(viewport={'width': 1280, 'height': 900}, accept_downloads=True)
            pubkey = coincurve.PrivateKey(secret).public_key_xonly.format().hex()

            def sign(event):
                event = dict(event)
                serial = json.dumps([0, event['pubkey'], event['created_at'], event['kind'], event['tags'],
                                     event['content']], separators=(',', ':'), ensure_ascii=False)
                event['id'] = hashlib.sha256(serial.encode()).hexdigest()
                event['sig'] = coincurve.PrivateKey(secret).sign_schnorr(bytes.fromhex(event['id'])).hex()
                return event

            page.expose_function('pilotSign', sign)
            page.add_init_script(f"window.nostr = {{getPublicKey: async () => '{pubkey}', "
                                 "signEvent: async e => await window.pilotSign(e)};")
            frames = []
            page.on('websocket', lambda ws: ws.on('framereceived', lambda f: frames.append(f)))
            console = []
            page.on('console', lambda m: console.append(m.text))
            page.goto(self.origin + '/')
            page.click('#nostr-login button')
            try:
                page.wait_for_selector('text=Signed in as', timeout=20000)
            except Exception:
                raise Failed(f'browser sign-in: notice {page.locator("#notice").inner_text()!r}, console {console}')
            card = page.locator('article', has_text=f'Pilot A ({self.tag}')
            card.get_by_text('Your seat: 1').wait_for(timeout=20000)
            self.check('browser: game card shows the seat', 'Your seat: 1' in card.inner_text())
            with page.expect_download() as download:
                card.get_by_role('button', name='Download map ZIP').click()
            zipped = download.value.path().read_bytes()
            self.check('browser: map ZIP downloads', zipped[:2] == b'PK', f'{len(zipped)} bytes')
            card.get_by_role('button', name='Play').click()
            page.wait_for_url('**/terminal/**')
            screen = pyte.Screen(80, 25)
            stream = pyte.ByteStream(screen)
            seen = 0
            end = time.monotonic() + 20
            while time.monotonic() < end:
                for frame in frames[seen:]:
                    stream.feed(frame if isinstance(frame, bytes) else frame.encode())
                seen = len(frames)
                if 'MAIN COMMAND' in '\n'.join(screen.display):
                    break
                text = '\n'.join(screen.display)
                if 'View the game introduction' in text:
                    page.keyboard.press('n')
                elif 'slap a key' in text:
                    page.keyboard.press('Enter')
                page.wait_for_timeout(500)
            page.screenshot(path=str(shots / 'main-menu.png'))
            self.check('browser: terminal reaches the main menu', 'MAIN COMMAND' in '\n'.join(screen.display))
            page.keyboard.press('p')
            page.wait_for_timeout(1500)
            for frame in frames[seen:]:
                stream.feed(frame if isinstance(frame, bytes) else frame.encode())
            page.screenshot(path=str(shots / 'planet-menu.png'))
            self.check('browser: keyboard input reaches the door', 'PLANET COMMAND' in '\n'.join(screen.display),
                       str(shots))
            size = page.evaluate("(() => { const r = document.querySelector('.xterm-screen').getBoundingClientRect();"
                                 " return [r.width, r.height]; })()")
            self.check('browser: terminal drawn', size[0] > 400 and size[1] > 300, size)
            page.click('#games-link') if page.locator('#games-link').is_visible() else page.goto(self.origin + '/')
            page.wait_for_selector('text=Signed in as')
            page.locator('article', has_text=f'Pilot A ({self.tag}').wait_for(timeout=20000)
            self.check('browser: back to the game list', page.locator('article').count() >= 1)
            browser.close()


class Matrix:
    """A spare Matrix account that reads the sign-in bot's DM."""

    def __init__(self, user, password):
        self.user = user
        self.server = 'https://' + user.split(':', 1)[1]
        self.http = httpx.Client(timeout=40)
        well_known = self.http.get(self.server + '/.well-known/matrix/client')
        if well_known.status_code == 200:
            self.server = well_known.json()['m.homeserver']['base_url'].rstrip('/')
        login = self.http.post(self.server + '/_matrix/client/v3/login', json={
            'type': 'm.login.password', 'identifier': {'type': 'm.id.user', 'user': user}, 'password': password})
        login.raise_for_status()
        self.http.headers['Authorization'] = 'Bearer ' + login.json()['access_token']
        self.since = self.sync(0)['next_batch']

    def sync(self, timeout=10000):
        params = {'timeout': timeout}
        if getattr(self, 'since', None):
            params['since'] = self.since
        return self.http.get(self.server + '/_matrix/client/v3/sync', params=params).json()

    def sign_in(self, player):
        player.http.post('/auth/matrix/request', json={'matrix_id': self.user}).raise_for_status()
        end = time.monotonic() + 90
        while time.monotonic() < end:
            data = self.sync()
            self.since = data['next_batch']
            for room in data.get('rooms', {}).get('invite', {}):
                self.http.post(f'{self.server}/_matrix/client/v3/rooms/{room}/join', json={})
            for room in data.get('rooms', {}).get('join', {}).values():
                for event in room.get('timeline', {}).get('events', []):
                    found = re.search(r'enter code (\S+) on the games page', event.get('content', {}).get('body', ''))
                    if found:
                        response = player.http.post('/auth/matrix', json={'code': found.group(1)})
                        if response.status_code != 200:
                            raise Failed(f'Matrix code refused: {response.text}')
                        player.account = player.me()['account']
                        return player
        raise Failed('no Matrix sign-in message arrived')

    def close(self):
        self.http.post(self.server + '/_matrix/client/v3/logout')


def matrix_from_keychain():
    found = subprocess.run(['security', 'find-generic-password', '-s', 'nc-pilot-matrix'], text=True,
                           capture_output=True)
    if found.returncode:
        return None
    user = re.search(r'"acct"<blob>="([^"]+)"', found.stdout).group(1)
    password = subprocess.run(['security', 'find-generic-password', '-s', 'nc-pilot-matrix', '-w'], text=True,
                              capture_output=True, check=True).stdout.strip()
    return Matrix(user, password)


# ---- local stack -----------------------------------------------------------

def local_stack(directory):
    root = Path(directory) / 'games'
    host = RUST / 'target/release/nc-host'
    admin = Admin(root=root, host_bin=host)
    admin(action='list')
    sock = socket.socket()
    sock.bind(('127.0.0.1', 0))
    port = sock.getsockname()[1]
    sock.close()
    origin = f'http://127.0.0.1:{port}'
    log = open(Path(directory) / 'bridge.log', 'w')
    bridge = subprocess.Popen(
        [RUST / 'target/release/nc-bridge', '--games-root', root, '--public-origin', origin,
         '--discord-client-id', '1', '--discord-client-secret', 'local-pilot-only', '--port', str(port),
         '--static-dir', RUST / 'nc-bridge/static', '--nc-door', RUST / 'target/release/nc-door'],
        stdout=log, stderr=log, env={**os.environ, 'NC_NOSTR_PROFILE_RELAYS': ''})
    for _ in range(100):
        try:
            if httpx.get(origin + '/health').status_code == 200:
                break
        except httpx.HTTPError:
            time.sleep(0.1)
    stop = threading.Event()

    def timer():
        # The live server's turns timer runs `due` every minute.
        while not stop.wait(15):
            subprocess.run([host, 'admin', '--root', root], input='{"action":"due"}', text=True, capture_output=True)

    threading.Thread(target=timer, daemon=True).start()
    return admin, origin, bridge, stop


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    where = parser.add_mutually_exclusive_group(required=True)
    where.add_argument('--local', action='store_true')
    where.add_argument('--remote', metavar='SSH_HOST')
    parser.add_argument('--origin', default=REMOTE_ORIGIN)
    parser.add_argument('--no-browser', action='store_true')
    parser.add_argument('--no-matrix', action='store_true')
    args = parser.parse_args()
    tag = time.strftime('%m%d%H%M')
    matrix = None if args.no_matrix or args.local else matrix_from_keychain()
    bridge = stop = None
    with tempfile.TemporaryDirectory(prefix='nc-pilot-') as directory:
        if args.local:
            admin, origin, bridge, stop = local_stack(directory)
        else:
            admin, origin = Admin(remote=args.remote, root=REMOTE_ROOT), args.origin
        pilot = Pilot(admin, origin, tag)
        try:
            pilot.run(matrix=matrix, browser=not args.no_browser)
        except Exception as error:
            pilot.check('pilot finished', False, f'{type(error).__name__}: {error}')
            for game in (pilot.a, pilot.b):
                try:
                    archive(admin, game)
                except Failed:
                    pass
        finally:
            if matrix:
                matrix.close()
            if bridge:
                stop.set()
                bridge.terminate()
                bridge.wait()
        failed = [r for r in pilot.results if r['ok'] is False]
        skipped = [r for r in pilot.results if r['ok'] is None]
        print(f'\n{len(pilot.results) - len(failed) - len(skipped)} passed, {len(failed)} failed, '
              f'{len(skipped)} skipped')
        for r in failed:
            print(f'FAILED: {r["check"]}: {r["detail"]}')
        sys.exit(1 if failed else 0)


if __name__ == '__main__':
    main()
