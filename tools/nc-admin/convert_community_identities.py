#!/usr/bin/env python3
"""One-time offline conversion of the October 2026 Discord community pilot.

Stop bridge, bot, turn and backup services first. Default is read-only preview.
This is a deployment tool, not a runtime schema migration framework.
"""
import argparse
from contextlib import ExitStack, closing
import fcntl
import hashlib
import json
from pathlib import Path
import sqlite3


def connect(path, readonly=False):
    if path.is_symlink() or not path.is_file():
        raise ValueError(f"expected a regular database: {path}")
    return sqlite3.connect(path.as_uri() + ("?mode=ro" if readonly else "?mode=rw"), uri=True)


def tables(db):
    return {row[0] for row in db.execute("SELECT name FROM sqlite_master WHERE type='table'")}


def valid_discord(value):
    return isinstance(value, str) and 1 <= len(value) <= 20 and value.isascii() and value.isdecimal()


def inspect(root):
    files, changes = [], []
    for game in sorted(root.iterdir()):
        if not (game / 'hosted.db').exists():
            continue
        if game.is_symlink() or not game.is_dir():
            raise ValueError(f"expected regular game directory: {game}")
        hosted = game / 'hosted.db'
        with closing(connect(hosted, True)) as db:
            names = tables(db)
            if ('discord_seats' in names) == ('web_seats' in names):
                raise ValueError(f"unknown or mixed community schema: {hosted}")
            if 'discord_seats' in names:
                for (account,) in db.execute('SELECT discord_id FROM discord_seats'):
                    if not valid_discord(account): raise ValueError('invalid legacy Discord identity')
                for table, column in [('web_invites','claimed_by'),('web_invite_attempts','account')]:
                    if table in names:
                        for (account,) in db.execute(f'SELECT {column} FROM {table} WHERE {column} IS NOT NULL'):
                            if not valid_discord(account): raise ValueError('invalid legacy invite identity')
                changes.append((hosted, 'game'))
        files.extend([hosted, game / 'ncgame.db'])
    sessions = root / '.web-sessions.db'
    if sessions.exists():
        with closing(connect(sessions, True)) as db:
            columns = {row[1] for row in db.execute('PRAGMA table_info(web_sessions)')}
            if 'discord_id' in columns and 'discord_profiles' in tables(db):
                for (account,) in db.execute('SELECT discord_id FROM discord_profiles'):
                    if not valid_discord(account): raise ValueError('invalid legacy profile identity')
                changes.append((sessions, 'sessions'))
            elif 'account' not in columns:
                raise ValueError('unsupported session schema')
        files.append(sessions)
    for path in files:
        with closing(connect(path, True)) as db:
            if db.execute('PRAGMA integrity_check').fetchone()[0] != 'ok':
                raise ValueError(f'database integrity failure: {path}')
    return files, changes


def convert_database(path, kind):
    with closing(connect(path)) as db, db:
        db.execute('BEGIN IMMEDIATE')
        if kind == 'game':
            db.execute('CREATE TABLE web_seats(seat INTEGER NOT NULL, account TEXT PRIMARY KEY NOT NULL)')
            db.execute("INSERT INTO web_seats SELECT seat, 'discord:' || discord_id FROM discord_seats")
            db.execute('DROP TABLE discord_seats')
            names = tables(db)
            if 'web_invites' in names:
                db.execute("UPDATE web_invites SET claimed_by='discord:' || claimed_by WHERE claimed_by IS NOT NULL")
            if 'web_invite_attempts' in names:
                db.execute("UPDATE web_invite_attempts SET account='discord:' || account")
        else:
            db.execute('DELETE FROM web_sessions')
            db.execute('DELETE FROM oauth_states')
            db.execute('ALTER TABLE web_sessions RENAME COLUMN discord_id TO account')
            db.execute('ALTER TABLE discord_profiles RENAME TO account_profiles')
            db.execute('ALTER TABLE account_profiles RENAME COLUMN discord_id TO account')
            db.execute("UPDATE account_profiles SET account='discord:' || account")


def convert(root, backup=None):
    root = root.resolve(strict=True)
    if backup is None:
        _, changes = inspect(root)
        return {'preview':True, 'databases':[str(p.relative_to(root)) for p,_ in changes]}
    backup = backup.resolve()
    if backup.is_relative_to(root) or root.is_relative_to(backup):
        raise ValueError('backup must be outside the games root')
    with ExitStack() as stack:
        for name in ['.bridge.lock', '.admin.lock']:
            lock = stack.enter_context(open(root / name, 'a+b'))
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        files, changes = inspect(root)
        if not changes:
            return {'converted':[], 'already_current':True}
        backup.mkdir(mode=0o700, parents=False, exist_ok=False)
        hashes = {}
        for path in files:
            dest = backup / path.relative_to(root)
            dest.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
            with closing(connect(path, True)) as source, closing(sqlite3.connect(dest)) as target:
                source.backup(target)
                if target.execute('PRAGMA integrity_check').fetchone()[0] != 'ok':
                    raise ValueError(f'backup integrity failure: {dest}')
            hashes[str(path.relative_to(root))] = hashlib.sha256(dest.read_bytes()).hexdigest()
        (backup / 'identity-conversion.json').write_text(json.dumps({'sha256':hashes}, indent=2)+'\n')
        attempted = []
        try:
            for path, kind in changes:
                attempted.append(path)
                convert_database(path, kind)
        except Exception:
            # Restore via SQLite backup, including any WAL state, before releasing locks.
            for path in attempted:
                with closing(connect(backup / path.relative_to(root), True)) as source, closing(connect(path)) as target:
                    source.backup(target)
            raise
    return {'converted':[str(p.relative_to(root)) for p,_ in changes], 'backup':str(backup), 'sessions_expired':True}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, required=True)
    parser.add_argument('--apply', action='store_true')
    parser.add_argument('--backup-dir', type=Path)
    args = parser.parse_args()
    if args.apply != bool(args.backup_dir):
        parser.error('--apply requires --backup-dir naming a new directory; preview needs neither')
    try:
        print(json.dumps(convert(args.root, args.backup_dir), indent=2))
    except (OSError, ValueError, sqlite3.Error) as error:
        parser.exit(1, f'Conversion failed: {error}\n')


if __name__ == '__main__':
    main()
