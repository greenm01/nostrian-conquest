import hashlib
import sqlite3
import sys
import tempfile
import unittest
from contextlib import closing
from pathlib import Path
from unittest.mock import patch
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import convert_community_identities as conversion


class ConversionTests(unittest.TestCase):
    def fixture(self, directory):
        root = Path(directory) / 'games'
        game = root / 'beta-1'
        game.mkdir(parents=True)
        with closing(sqlite3.connect(game / 'hosted.db')) as db, db:
            db.executescript('''
                CREATE TABLE discord_seats(seat INTEGER PRIMARY KEY,discord_id TEXT UNIQUE NOT NULL);
                INSERT INTO discord_seats VALUES (1,'123');
                CREATE TABLE web_invites(id TEXT PRIMARY KEY,seat INTEGER,code_hash TEXT,state TEXT,created_at INTEGER,claimed_by TEXT);
                INSERT INTO web_invites VALUES ('used',1,'used-hash','claimed',1,'123');
                INSERT INTO web_invites VALUES ('unused',2,'private-hash','unused',1,NULL);
                CREATE TABLE web_invite_attempts(account TEXT,at INTEGER);
                INSERT INTO web_invite_attempts VALUES ('456',1);
                CREATE TABLE game_metadata(status TEXT,year INTEGER,next_due INTEGER);
                INSERT INTO game_metadata VALUES ('recruiting',3000,NULL);
            ''')
        with closing(sqlite3.connect(game / 'ncgame.db')) as db, db:
            db.execute('CREATE TABLE snapshots(game_year INTEGER)')
            db.execute('INSERT INTO snapshots VALUES (3000)')
        with closing(sqlite3.connect(root / '.web-sessions.db')) as db, db:
            db.executescript('''
                CREATE TABLE web_sessions(token TEXT PRIMARY KEY,discord_id TEXT,expires INTEGER);
                INSERT INTO web_sessions VALUES ('session','123',9999999999);
                CREATE TABLE oauth_states(token TEXT PRIMARY KEY,expires INTEGER);
                INSERT INTO oauth_states VALUES ('oauth',9999999999);
                CREATE TABLE discord_profiles(discord_id TEXT PRIMARY KEY,display_name TEXT);
                INSERT INTO discord_profiles VALUES ('123','Commander');
            ''')
        return root

    def test_preview_conversion_preservation_and_idempotency(self):
        with tempfile.TemporaryDirectory() as directory:
            root = self.fixture(directory)
            before = {p:p.read_bytes() for p in root.rglob('*.db')}
            result = conversion.convert(root)
            self.assertEqual(len(result['databases']),2)
            self.assertEqual(before,{p:p.read_bytes() for p in before})
            backup = Path(directory) / 'backup'
            conversion.convert(root,backup)
            with closing(sqlite3.connect(root / 'beta-1/hosted.db')) as db:
                self.assertEqual(db.execute('SELECT * FROM web_seats').fetchall(),[(1,'discord:123')])
                self.assertEqual(db.execute('SELECT code_hash,state,claimed_by FROM web_invites ORDER BY id').fetchall(),[('private-hash','unused',None),('used-hash','claimed','discord:123')])
                self.assertEqual(db.execute('SELECT * FROM game_metadata').fetchone(),('recruiting',3000,None))
                self.assertEqual(db.execute('SELECT account FROM web_invite_attempts').fetchone(),('discord:456',))
                # New schema allows an operator's second provider on the same seat.
                db.execute('INSERT INTO web_seats VALUES (?,?)',(1,'nostr:'+'a'*64))
            with closing(sqlite3.connect(root / '.web-sessions.db')) as db:
                self.assertEqual(db.execute('SELECT * FROM account_profiles').fetchall(),[('discord:123','Commander')])
                self.assertEqual(db.execute('SELECT COUNT(*) FROM web_sessions').fetchone()[0],0)
                self.assertEqual(db.execute('SELECT COUNT(*) FROM oauth_states').fetchone()[0],0)
            self.assertEqual(before[root/'beta-1/ncgame.db'],(root/'beta-1/ncgame.db').read_bytes())
            self.assertEqual(conversion.convert(root)['databases'],[])
            self.assertTrue(conversion.convert(root,Path(directory)/'unused')['already_current'])
            self.assertFalse((Path(directory)/'unused').exists())
            with closing(sqlite3.connect(backup/'beta-1/hosted.db')) as db:
                self.assertEqual(db.execute('SELECT * FROM discord_seats').fetchall(),[(1,'123')])
            import json
            for file, digest in json.loads((backup/'identity-conversion.json').read_text())['sha256'].items():
                self.assertEqual(hashlib.sha256((backup/file).read_bytes()).hexdigest(),digest)

    def test_failure_restores_databases_already_converted(self):
        with tempfile.TemporaryDirectory() as directory:
            root = self.fixture(directory)
            real = conversion.convert_database
            def failing(path,kind):
                if kind == 'sessions': raise ValueError('injected failure')
                real(path,kind)
            with patch.object(conversion,'convert_database',side_effect=failing), self.assertRaises(ValueError):
                conversion.convert(root,Path(directory)/'backup')
            self.assertEqual(len(conversion.convert(root)['databases']),2)
            with closing(sqlite3.connect(root/'beta-1/hosted.db')) as db:
                self.assertEqual(db.execute('SELECT * FROM discord_seats').fetchall(),[(1,'123')])
                self.assertEqual(db.execute("SELECT claimed_by FROM web_invites WHERE id='used'").fetchone(),('123',))

    def test_running_bridge_prevents_conversion(self):
        import fcntl
        with tempfile.TemporaryDirectory() as directory:
            root = self.fixture(directory)
            with open(root/'.bridge.lock','a+b') as lock:
                fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
                with self.assertRaises(BlockingIOError): conversion.convert(root,Path(directory)/'backup')
            self.assertFalse((Path(directory)/'backup').exists())

    def test_invalid_identity_is_rejected_before_backups(self):
        with tempfile.TemporaryDirectory() as directory:
            root = self.fixture(directory)
            with closing(sqlite3.connect(root/'beta-1/hosted.db')) as db,db:
                db.execute("UPDATE discord_seats SET discord_id='invalid'")
            with self.assertRaises(ValueError): conversion.convert(root,Path(directory)/'backup')
            self.assertFalse((Path(directory)/'backup').exists())


if __name__ == '__main__': unittest.main()
