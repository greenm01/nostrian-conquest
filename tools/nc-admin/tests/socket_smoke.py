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


def request(port, method, path, token=None):
    conn=http.client.HTTPConnection('127.0.0.1',port,timeout=10)
    headers={'Origin':f'http://127.0.0.1:{port}'}
    if token: headers['Cookie']=f'nc_session={token}'
    conn.request(method,path,headers=headers)
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
                # Copy the hash from the one-time login nonce. It uses exactly
                # the same hash function as sessions, with no test auth endpoint.
                conn=http.client.HTTPConnection('127.0.0.1',port)
                conn.request('GET','/auth/login');response=conn.getresponse()
                token=response.getheader('Set-Cookie').split(';')[0].split('=')[1];response.read();conn.close()
                with sqlite3.connect(root/'.web-sessions.db') as db:
                    digest=db.execute('SELECT token FROM oauth_states').fetchone()[0]
                    db.execute('INSERT INTO web_sessions VALUES (?,?,?)',(digest,'123',int(time.time())+60))
                    db.execute('INSERT INTO discord_profiles VALUES (?,?)',('123','Smoke Commander'))
                assert request(port,'POST','/api/games/one/join',token)[0]==200
                assert request(port,'GET','/api/games/two/exports/reports',token)[0]==409
                sock,status=websocket(port,token);assert status==101,status
                output=b''
                for _ in range(20):
                    opcode,payload=receive(sock)
                    assert opcode!=8,'door closed before rendering'
                    output+=payload
                    if len(output)>100:break
                assert len(output)>100,'door produced no terminal output'
                with sqlite3.connect(root/'one/ncgame.db') as db:
                    handle=db.execute('SELECT handle FROM snapshot_players WHERE record_index=1 ORDER BY snapshot_id DESC LIMIT 1').fetchone()[0]
                    assert handle=='Smoke Commander',handle
                other,status=websocket(port,token);other.close();assert status==409,status
                assert request(port,'POST','/auth/logout',token)[0]==200
                deadline=time.monotonic()+10
                while time.monotonic()<deadline:
                    try:
                        opcode,_=receive(sock)
                        if opcode==8:break
                    except EOFError:break
                else:raise AssertionError('logout did not close terminal')
                sock.close()
                for _ in range(100):
                    if not list((root/'.nodes').iterdir()):break
                    time.sleep(.05)
                else:raise AssertionError('terminal lease was not cleaned up')
                print(json.dumps({'ok':True,'terminal_bytes':len(output),'checks':['real-door-render','wrong-game-export','duplicate-seat','logout-revocation','lease-cleanup']}))
            except Exception:
                log.flush();log.seek(0);print(log.read());raise
            finally:
                process.terminate()
                try:process.wait(timeout=10)
                except subprocess.TimeoutExpired:process.kill();process.wait()

if __name__=='__main__':main()
