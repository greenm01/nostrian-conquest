#!/usr/bin/env python3
"""Real nc-door loopback smoke; synthetic auth exists only in this private fixture.

Usage: socket_smoke.py --host PATH --bridge PATH --door PATH --static PATH
No live Discord requests or existing campaign directories are used.
"""
import argparse
import base64
import http.client
import json
import os
from pathlib import Path
import socket
import sqlite3
import struct
import subprocess
import tempfile
import time


def admin(root, binary, **request):
    p = subprocess.run([binary, 'admin', '--root', str(root)], input=json.dumps(request), text=True, capture_output=True, check=True)
    return json.loads(p.stdout)['data']


def request(port, method, path, token=None, body=None):
    conn=http.client.HTTPConnection('127.0.0.1',port,timeout=10)
    headers={'Origin':f'http://127.0.0.1:{port}'}
    if token: headers['Cookie']=f'nc_session={token}'
    if body is not None: headers['Content-Type']='application/json'
    conn.request(method,path,body=json.dumps(body) if body is not None else None,headers=headers)
    response=conn.getresponse();status=response.status;body=response.read();conn.close()
    return status,body


def websocket(port, token, game='one'):
    conn=socket.create_connection(('127.0.0.1',port),timeout=10)
    key=base64.b64encode(os.urandom(16)).decode()
    conn.sendall((f'GET /games/{game}/ws HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: {key}\r\nOrigin: http://127.0.0.1:{port}\r\nCookie: nc_session={token}\r\n\r\n').encode())
    headers=b''
    while not headers.endswith(b'\r\n\r\n'):headers+=conn.recv(1)
    return conn,int(headers.split(b' ')[1])


def receive(conn):
    def exact(n):
        data=b''
        while len(data)<n:
            part=conn.recv(n-len(data))
            if not part:raise EOFError()
            data+=part
        return data
    first,second=exact(2);length=second&127
    if length==126:length=struct.unpack('!H',exact(2))[0]
    elif length==127:length=struct.unpack('!Q',exact(8))[0]
    return first&15,exact(length)


def main():
    parser=argparse.ArgumentParser()
    for name in ('host','bridge','door','static'):parser.add_argument('--'+name,required=True)
    args=parser.parse_args()
    with tempfile.TemporaryDirectory(prefix='nc-community-smoke-') as directory:
        root=Path(directory)/'games'
        for game in ('one','two'):
            admin(root,args.host,action='create',game=game,name=game,players=4,seed=1515)
            admin(root,args.host,action='publish',game=game)
        with socket.socket() as listener:
            listener.bind(('127.0.0.1',0));port=listener.getsockname()[1]
        with open(Path(directory)/'bridge.log','w+') as log:
            process=subprocess.Popen([args.bridge,'--games-root',str(root),'--public-origin',f'http://127.0.0.1:{port}','--discord-client-id','123','--discord-client-secret','fixture-only','--port',str(port),'--static-dir',args.static,'--nc-door',args.door],stdout=log,stderr=log)
            try:
                for _ in range(100):
                    try:
                        if request(port,'GET','/health')[0]==200:break
                    except OSError:pass
                    time.sleep(.1)
                else:raise AssertionError('bridge did not start')
                def session(account, name):
                    # Synthetic auth is confined to this disposable fixture.
                    conn=http.client.HTTPConnection('127.0.0.1',port)
                    conn.request('GET','/auth/login');response=conn.getresponse()
                    token=response.getheader('Set-Cookie').split(';')[0].split('=')[1];response.read();conn.close()
                    with sqlite3.connect(root/'.web-sessions.db') as db:
                        digest=db.execute('SELECT token FROM oauth_states ORDER BY rowid DESC LIMIT 1').fetchone()[0]
                        db.execute('INSERT INTO web_sessions VALUES (?,?,?)',(digest,account,int(time.time())+120))
                        db.execute('INSERT INTO account_profiles VALUES (?,?)',(account,name))
                    return token
                def rendered(sock):
                    output=b''
                    for _ in range(20):
                        opcode,payload=receive(sock)
                        assert opcode!=8,'door closed before rendering'
                        output+=payload
                        if len(output)>100:return len(output)
                    raise AssertionError('door produced no terminal output')
                def closed(sock):
                    deadline=time.monotonic()+10
                    while time.monotonic()<deadline:
                        try:
                            opcode,_=receive(sock)
                            if opcode==8:break
                        except EOFError:break
                    else:raise AssertionError('revocation did not close terminal')
                    sock.close()
                key='79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798'
                alias_key='c6047f9441ed7d6d3045406e95c07cd85c778e4b8cef3ca7abac09b95c709ee5'
                discord=session('discord:123','Discord Commander')
                nostr=session('nostr:'+key,'Nostr Commander')
                alias=session('nostr:'+alias_key,'Linked Commander')
                admin(root,args.host,action='invite_only',game='one')
                codes=admin(root,args.host,action='create_invites',game='one',count=2)['invites']
                assert request(port,'POST','/api/games/one/join',discord,{'code':codes[0]['code']})[0]==200
                assert request(port,'POST','/api/games/one/join',nostr,{'code':codes[0]['code']})[0]==409
                assert request(port,'POST','/api/games/one/join',nostr,{'code':codes[1]['code']})[0]==200
                assert request(port,'GET','/api/games/two/exports/reports',nostr)[0]==409
                assert request(port,'GET','/api/games/one/exports/map-zip',nostr)[0]==200
                sockets=[];total=0
                for token,seat,name in [(discord,1,'Discord Commander'),(nostr,2,'Nostr Commander')]:
                    sock,status=websocket(port,token);assert status==101,status
                    sockets.append(sock);total+=rendered(sock)
                    with sqlite3.connect(root/'one/ncgame.db') as db:
                        handle=db.execute('SELECT handle FROM snapshot_players WHERE record_index=? ORDER BY snapshot_id DESC LIMIT 1',(seat,)).fetchone()[0]
                        assert handle==name,handle
                account={'provider':'nostr','id':alias_key}
                admin(root,args.host,action='link_account',game='one',seat=1,account=account)
                access_status,access_body=request(port,'GET','/api/games/one/access',alias)
                assert access_status==200 and json.loads(access_body)['connected'] is True
                other,status=websocket(port,alias);other.close();assert status==409,status
                assert request(port,'POST','/auth/logout',discord)[0]==200
                closed(sockets[0])
                # The alternate identity can use the same empire after the old terminal closes.
                linked,status=websocket(port,alias);assert status==101,status
                total+=rendered(linked)
                admin(root,args.host,action='unlink_account',game='one',seat=1,account=account)
                closed(linked)
                access_status,access_body=request(port,'GET','/api/games/one/access',alias)
                assert access_status==200 and json.loads(access_body)['seat'] is None
                assert json.loads(access_body)['connected'] is False
                assert request(port,'POST','/auth/logout',nostr)[0]==200
                closed(sockets[1])
                for _ in range(100):
                    if not list((root/'.nodes').iterdir()):break
                    time.sleep(.05)
                else:raise AssertionError('terminal lease was not cleaned up')
                game=admin(root,args.host,action='inspect',game='one')
                assert (game['claimed'],game['state'],game['year'],game['next_due'])==(2,'recruiting',3000,None)
                print(json.dumps({'ok':True,'terminal_bytes':total,'checks':['mixed-provider-real-doors','invite-single-use','wrong-game-export','linked-seat-exclusion','operator-unlink-revocation','logout-revocation','lease-cleanup','recruiting-hold']}))

            except Exception:
                log.flush();log.seek(0);print(log.read());raise
            finally:
                process.terminate()
                try:process.wait(timeout=10)
                except subprocess.TimeoutExpired:process.kill();process.wait()

if __name__=='__main__':main()
