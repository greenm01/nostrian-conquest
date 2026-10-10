import json, sys, statistics as st
from collections import defaultdict
import sim
def dockval(d): return 3*d[0]+18*d[1]+30*d[2]+3*d[3]+5*d[4]+18*d[5]+60*d[6]+2*d[7]
DOCK=[4,3,2,2,4,1,1,0]
CASES={
 'B1_bomb_3bb_vs4': (6,dict(bb=3,roe=10),dict(bat=4,armies=10,dock=DOCK)),
 'B2_bomb_10ca_vs0':(6,dict(ca=10,roe=10),dict(bat=0,armies=10,dock=DOCK)),
 'B3_bomb_2ca_roe6':(6,dict(ca=2,roe=6),dict(bat=4,armies=10,dock=DOCK)),
 'B5_bomb_2ca_roe10':(6,dict(ca=2,roe=10),dict(bat=4,armies=10,dock=DOCK)),
 'B6_bomb_1ca_roe10':(6,dict(ca=1,roe=10),dict(bat=4,armies=10,dock=DOCK)),
 'B4_bomb_20dd_vs4':(6,dict(dd=20,roe=10),dict(bat=4,armies=10,dock=DOCK)),
 'I1_inv_3bb_20tl_vs0':(7,dict(bb=3,tl=20,roe=10),dict(bat=0,armies=10)),
 'I2_inv_2ca_12tl_vs0':(7,dict(ca=2,tl=12,roe=10),dict(bat=0,armies=10)),
 'I3_inv_4bb_15tl_vs4':(7,dict(bb=4,tl=15,roe=10),dict(bat=4,armies=10)),
 'L1_blitz_10tl_vs4':(8,dict(tl=10,roe=0),dict(bat=4,armies=10)),
 'L2_blitz_10tl_vs0':(8,dict(tl=10,roe=0),dict(bat=0,armies=10)),
 'L3_blitz_2bb_30tl_vs4':(8,dict(bb=2,tl=30,roe=10),dict(bat=4,armies=10)),
 'L5_blitz_20tl_vs0_a10':(8,dict(tl=20,roe=0),dict(bat=0,armies=10)),
 'L6_blitz_20tl_vs2_a5':(8,dict(tl=20,roe=0),dict(bat=2,armies=5)),
 'L7_blitz_20tl_vs1_a5':(8,dict(tl=20,roe=0),dict(bat=1,armies=5)),
 'I4_inv_2bb_10tl_vs0_a5':(7,dict(bb=2,tl=10,roe=10),dict(bat=0,armies=5)),
 'L4_blitz_14tl_vs0_a8':(8,dict(tl=14,roe=0),dict(bat=0,armies=8)),
}
def probe_rows(fn):
    rows=defaultdict(list)
    for l in open(fn):
        r=json.loads(l); m1=' '.join(m['text'] for m in r['msgs'] if m['to']==1)
        dead='lost all contact' in m1
        f=r['f']; p=r['p']
        rows[r['case']].append(dict(fleet_dead=dead,bb=f['bb'],ca=f['ca'],dd=f['dd'],tt=f['tt'],tl=f['tl'],armies=p['armies'],bat=p['bat'],r3=p['r3'],r9=p['r9'],dv=dockval(p['dock']),captured=p['own']==1))
    return rows
def sim_rows(case,n=4000):
    o,fl,pl=CASES[case]; out=[]
    for _ in range(n):
        P=dict(r3=100.0,r9=35.0,prod2=100,dock=[0]*8); P.update(pl)
        a=sim.Assault(fl,P,o); r=a.run(); r['dv']=dockval(r['dock'])
        if r['fleet_dead']: r.update(bb=0,ca=0,dd=0,tt=0,tl=0)
        out.append(r)
    return out
KEYS=['fleet_dead','captured','bb','ca','dd','tt','tl','armies','bat','r3','r9','dv']
def summ(rows,k):
    v=[float(r[k]) for r in rows]; m=st.mean(v); s=st.pstdev(v)
    return m,s
rows=probe_rows(sys.argv[1] if len(sys.argv)>1 else 'pl_out.jsonl')
for case in CASES:
    if case not in rows: continue
    pr=rows[case]
    # exclude dead-fleet records' ships (file shows resurrected originals)
    for r in pr:
        if r['fleet_dead']: r.update(bb=0,ca=0,dd=0,tt=0,tl=0)
    sr=sim_rows(case)
    print('==',case,'n=%d'%len(pr))
    for k in KEYS:
        pm,ps=summ(pr,k); sm,ss=summ(sr,k)
        if ps==0 and ss==0 and pm==sm: continue
        se=(ss/len(pr)**0.5) if ss>0 else 0.0
        z=(pm-sm)/se if se>0 else float('inf') if pm!=sm else 0
        print('  %-10s probe %8.3f (sd %6.2f)  sim %8.3f (sd %6.2f)  z=%5.1f'%(k,pm,ps,sm,ss,z))
