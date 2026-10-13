#!/usr/bin/env python3
"""Run the real game-list JavaScript in headless Chromium with mocked HTTP/signing."""
from pathlib import Path
import http.server
import threading
import subprocess
import tempfile
root=Path(__file__).resolve().parents[3]
with tempfile.TemporaryDirectory(prefix='nc-nostr-ui-') as d:
    dest=Path(d)
    html=(root/'rust/nc-bridge/static/community.html').read_text()
    mock='''<script>
    let logged=false, claimed=false, connected=false, accessError=false, provider='nostr';
    const game={game:'one',name:'One',state:'recruiting',open:2,players:4,year:3000,schedule:{mode:'manual'},invite_required:true};
    window.fetch=async (url,opts={}) => {
        let data={},status=200;
        if(url==='/api/me'){status=logged?200:401;data=logged?{account:{provider,id:provider==='nostr'?'a'.repeat(64):'123'},display_name:'Commander'}:{error:'Sign in'};}
        else if(url==='/api/games') data=[game];
        else if(url==='/api/games/one/access') {
            if(accessError){status=503;data={error:'Temporarily unavailable'};}
            else data={seat:claimed?1:null,can_play:claimed&&['recruiting','running'].includes(game.state),state:game.state,connected:claimed&&connected};
        }
        else if(url==='/api/games/one/reconnect')connected=false;
        else if(url==='/auth/nostr/challenge') data={challenge:'a'.repeat(64)};
        else if(url==='/auth/nostr'){
            const event=JSON.parse(atob(opts.headers.Authorization.slice(6)));
            if(event.kind!==27235 || event.tags[0][1]!==location.origin+'/auth/nostr' || event.tags[1][1]!=='POST')throw Error('bad event scope');
            const hash=Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',new TextEncoder().encode(opts.body))),b=>b.toString(16).padStart(2,'0')).join('');
            if(event.tags[2][1]!==hash)throw Error('bad payload');
            logged=true;
        }
        else if(url==='/api/games/one/join'){
            const body=JSON.parse(opts.body);
            if(body.code!=='AbCd2345' || Object.keys(body).length!==1)throw Error('bad claim');
            claimed=true;data={seat:1};
        }
        else if(url==='/auth/logout')logged=false;
        else throw Error('unexpected endpoint '+url);
        return {ok:status===200,status,json:async()=>data};
    };
    </script>'''
    html=html.replace('<script src=',mock+'<script src=')
    html=html.replace('/terminal/community.js','/community.js').replace('/terminal/community.css','/community.css')
    (dest/'community.css').write_text((root/'rust/nc-bridge/static/community.css').read_text())
    tests='''<script>
    window.addEventListener('load',async()=>{
        const result=document.createElement('pre'); result.id='result';document.body.append(result);
        try {
            const form=document.getElementById('nostr-login');
            const logout=document.getElementById('logout');
            const labels=()=>Array.from(document.querySelectorAll('article button')).map(b=>b.textContent).join(',');
            const check=(condition,message)=>{if(!condition)throw Error(message);};
            await loadIdentity();await loadGames();
            check(!form.hidden && logout.hidden,'signed-out controls');
            check(labels()==='' && !document.querySelector('article input'),'signed-out game actions');
            const event={preventDefault(){},currentTarget:form};
            await form.onsubmit(event);
            if(!document.getElementById('notice').textContent.includes('Install'))throw Error('missing extension');
            window.nostr={getPublicKey:async()=> 'a'.repeat(64),signEvent:async()=>{throw Error('User rejected');}};
            await form.onsubmit(event);
            if(!document.getElementById('notice').textContent.includes('User rejected'))throw Error('reject handling');
            window.nostr={getPublicKey:async()=>null,signEvent:async()=>{throw Error('must not sign without a key');}};
            await form.onsubmit(event);
            check(document.getElementById('notice').textContent.includes('public key'),'missing key handling');
            check(!logged && !form.querySelector('button').disabled,'missing key leaves login available');
            let keyRequested=false;
            window.nostr={getPublicKey:async()=>{keyRequested=true;return 'a'.repeat(64);},signEvent:async event=>{
                check(keyRequested && event.pubkey==='a'.repeat(64),'public key required before signing');
                return {...event,id:'f'.repeat(64),sig:'b'.repeat(128)};
            }};
            document.getElementById('nostr-name').value='Commander';
            await form.onsubmit(event);
            if(!logged || !document.getElementById('identity').textContent.includes('Nostr'))throw Error('login failed');
            check(form.hidden && !logout.hidden,'signed-in controls');
            check(labels()==='Join','unjoined actions');
            document.querySelector('article input').value='AbCd2345';
            await Array.from(document.querySelectorAll('article button')).find(b=>b.textContent==='Join').onclick();
            if(!claimed)throw Error('claim failed');
            check(labels()==='Play,Download map ZIP' && !document.querySelector('article input'),'joined actions');
            connected=true;await loadGames();
            check(labels()==='Reconnect,Download map ZIP','connected actions');
            await document.querySelector('article button').onclick();
            check(labels()==='Play,Download map ZIP','reconnect completion');
            game.state='paused';await loadGames();
            check(labels()==='Download map ZIP' && document.querySelector('article').textContent.includes('Paused by host'),'paused member');
            game.state='maintenance';await loadGames();
            check(!labels().includes('Play') && document.querySelector('article').textContent.includes('processing'),'maintenance member');
            claimed=false;game.state='running';await loadGames();
            check(labels()==='' && document.querySelector('article').textContent.includes('Recruitment is closed'),'closed recruitment');
            game.state='recruiting';game.open=0;await loadGames();
            check(labels()==='' && document.querySelector('article').textContent.includes('No seats'),'full game');
            game.open=2;accessError=true;await loadGames();
            check(labels()==='' && document.querySelector('article').textContent.includes('Cannot check'),'lookup failure');
            accessError=false;
            await logout.onclick();
            check(!form.hidden && logout.hidden && labels()==='','logout controls');
            if(logged || !document.getElementById('identity').textContent.includes('Choose'))throw Error('logout failed');
            logged=true;provider='discord';claimed=true;await loadIdentity();await loadGames();
            check(form.hidden && !logout.hidden && document.getElementById('identity').textContent.includes('Discord') && labels()==='Play,Download map ZIP','Discord member');
            await logout.onclick();
            result.textContent='PASS: signed-out, sign-in, invite claim, joined, connected, reconnect, paused, maintenance, closed, full, lookup failure, logout';
        }catch(error){result.textContent='FAIL: '+error.message;}
    });
    </script>'''
    (dest/'index.html').write_text(html.replace('</body>',tests+'</body>'))
    (dest/'community.js').write_text((root/'rust/nc-bridge/static/community.js').read_text())
    class Handler(http.server.SimpleHTTPRequestHandler):
        def __init__(self,*args,**kwargs):super().__init__(*args,directory=d,**kwargs)
        def log_message(self,*args):pass
    server=http.server.ThreadingHTTPServer(('127.0.0.1',0),Handler)
    threading.Thread(target=server.serve_forever,daemon=True).start()
    command=['chromium','--headless','--no-sandbox','--disable-gpu','--disable-dev-shm-usage','--no-first-run','--user-data-dir='+d+'/profile','--virtual-time-budget=3000','--dump-dom','http://127.0.0.1:'+str(server.server_port)]
    result=subprocess.run(command,capture_output=True,text=True,timeout=40)
    assert '<pre id="result">PASS:' in result.stdout,result.stdout[-3000:]+result.stderr[-1000:]
    print(result.stdout.split('<pre id="result">')[1].split('</pre>')[0])
    server.shutdown()
