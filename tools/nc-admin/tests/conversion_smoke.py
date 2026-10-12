#!/usr/bin/env python3
"""Exercise the pilot conversion with real host-created campaign databases."""
import argparse
from contextlib import closing
import json
from pathlib import Path
import sqlite3
import subprocess
import sys
import tempfile
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from convert_community_identities import convert


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--host',required=True)
    args=parser.parse_args()
    with tempfile.TemporaryDirectory(prefix='nc-conversion-smoke-') as directory:
        root=Path(directory)/'games'
        def host(**request):
            result=subprocess.run([args.host,'admin','--root',str(root)],input=json.dumps(request),text=True,capture_output=True,check=True)
            return json.loads(result.stdout)['data']
        host(action='create',game='one',name='One',players=4,seed=1515)
        host(action='publish',game='one')
        host(action='invite_only',game='one')
        codes=host(action='create_invites',game='one',count=2)['invites']
        host(action='claim_invite',game='one',account={'provider':'discord','id':'123'},code=codes[0]['code'])
        before=(root/'one/ncgame.db').read_bytes()
        # Reconstruct only the identity tables in the deployed pilot format.
        with closing(sqlite3.connect(root/'one/hosted.db')) as db,db:
            db.execute('CREATE TABLE discord_seats(seat INTEGER PRIMARY KEY,discord_id TEXT NOT NULL UNIQUE)')
            db.execute('INSERT INTO discord_seats SELECT seat,substr(account,9) FROM web_seats')
            db.execute('DROP TABLE web_seats')
            db.execute('UPDATE web_invites SET claimed_by=substr(claimed_by,9) WHERE claimed_by IS NOT NULL')
            db.execute('UPDATE web_invite_attempts SET account=substr(account,9)')
        with closing(sqlite3.connect(root/'.web-sessions.db')) as db,db:
            db.executescript("CREATE TABLE web_sessions(token TEXT PRIMARY KEY,discord_id TEXT NOT NULL,expires INTEGER NOT NULL); CREATE TABLE oauth_states(token TEXT PRIMARY KEY,expires INTEGER NOT NULL); CREATE TABLE discord_profiles(discord_id TEXT PRIMARY KEY,display_name TEXT NOT NULL); INSERT INTO discord_profiles VALUES ('123','Commander');")
        preview=convert(root)
        assert len(preview['databases'])==2
        convert(root,Path(directory)/'backup')
        assert (root/'one/ncgame.db').read_bytes()==before
        assert host(action='authorize',game='one',account={'provider':'discord','id':'123'})['seat']==1
        invites=host(action='list_invites',game='one')['invites']
        assert sorted(i['state'] for i in invites)==['claimed','unused']
        key='79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798'
        assert host(action='claim_invite',game='one',account={'provider':'nostr','id':key},code=codes[1]['code'])['seat']==2
        game=host(action='inspect',game='one')
        assert (game['claimed'],game['state'],game['year'],game['next_due'])==(2,'recruiting',3000,None)
        assert convert(root)['databases']==[]
        print(json.dumps({'ok':True,'checks':['real-campaign-conversion','existing-owner','unused-code-claim','runtime-preserved','recruiting-hold','idempotency']}))


if __name__=='__main__':main()
