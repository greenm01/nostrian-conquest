import json, re, statistics as st, collections
import fsim
NAMES={'battleship':'bb','cruiser':'ca','destroyer':'dd','troop transport ship':'tt','ETAC ship':'et','scout':'sc'}
def parse_lost(txt):
    if 'suffered no losses' in txt: return {}
    m=re.search(r'We lost ((?:\d+ [a-zA-Z ]+?(?:, | and |\. ))+)',txt)
    out={}
    if not m: return None
    for n,name in re.findall(r'(\d+) (battleship|cruiser|destroyer|troop transport ship|ETAC ship|scout)s?',m.group(1)):
        out[NAMES[name]]=out.get(NAMES[name],0)+int(n)
    return out
CASES={
 'F1_10ca_vs_10ca_r10':(dict(ca=10,roe=10,maxsp=5),dict(ca=10,roe=10,maxsp=5)),
 'F2_10ca_vs_10ca_r0':(dict(ca=10,roe=10,maxsp=5),dict(ca=10,roe=0,maxsp=5)),
 'F3_5bb_vs_20dd_r0':(dict(bb=5,roe=10,maxsp=4),dict(dd=20,roe=0,maxsp=6)),
 'F4_3bb6dd_vs_4ca4dd_r10':(dict(bb=3,dd=6,roe=10,maxsp=4),dict(ca=4,dd=4,roe=10,maxsp=5)),
 'F5_6ca_r6_vs_3bb_r10':(dict(ca=6,roe=6,maxsp=5),dict(bb=3,roe=10,maxsp=4)),
 'F6_8ca_vs_4ca6tt_r0':(dict(ca=8,roe=10,maxsp=5),dict(ca=4,tt=6,roe=0,maxsp=5)),
}
def tot(d,keys): return sum(d.get(k,0) for k in keys)
rows=collections.defaultdict(list)
for l in open('fb_out.jsonl'):
    r=json.loads(l); A,T=CASES[r['case']]
    m1=[m for m in r['msgs'] if m['to']==1 and ('intercepted' in m['text'])]
    if not m1: continue
    tick=m1[0]['tick']
    a=m1[0]['text']; t=[m for m in r['msgs'] if m['to']==2 and m['tick']==tick and ('attacked by' in m['text'] or 'lost all contact' in m['text'])]
    if not t: continue
    t=t[0]['text']
    la=parse_lost(a) if 'lost all contact' not in a else {k:v for k,v in A.items() if k not in ('roe','maxsp')}
    lt=parse_lost(t) if 'lost all contact' not in t else {k:v for k,v in T.items() if k not in ('roe','maxsp')}
    if la is None or lt is None: print('PARSE?',a[:200],'||',t[:200]); continue
    rows[r['case']].append((la,lt,'fled' in t or 'scatter' in t))
for case,(A,T) in CASES.items():
    pr=rows[case]
    keys=['bb','ca','dd','tt']
    sims=[fsim.battle(A,T) for _ in range(4000)]
    print('==',case,'n=%d'%len(pr))
    for side,src,init in (('A',0,A),('T',1,T)):
        for k in keys:
            if not init.get(k): continue
            p=[x[src].get(k,0) for x in pr]
            s=[init[k]-sm[1 if side=='A' else 2][k] for sm in sims]
            se=st.pstdev(s)/len(p)**0.5
            print('  %s lost %-3s probe %6.2f (sd %5.2f)  sim %6.2f (sd %5.2f)  z=%5.1f'%(side,k,st.mean(p),st.pstdev(p),st.mean(s),st.pstdev(s),(st.mean(p)-st.mean(s))/se if se else 0))
