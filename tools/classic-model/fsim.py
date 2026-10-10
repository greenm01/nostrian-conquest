"""Python model of ECMAINT fleet battle 1b05:7b74 (decoded for t036)."""
import math
from sim import R, rnd, roll, STATS, VALUE, U, roe_willing
STATS[8]=(25,60,36,0,3,60)
RANGE={1:10,2:15,3:20,8:25}; SPEED={1:6,2:5,3:4,4:6,5:5,6:5,7:3,8:1}; STRW={1:2,2:7,3:21,8:30}
def build(f):
    u=[]
    for t,k in ((3,'bb'),(2,'ca'),(1,'dd')): u+=[U(t) for _ in range(f.get(k,0))]
    c=len(u)
    for t,k in ((4,'sc'),(5,'tt'),(6,'tl'),(7,'et')): u+=[U(t) for _ in range(f.get(k,0))]
    return u,c
def engaged(units,s):
    A=sum(VALUE.get(u.t,0) for u in units[s]); B=sum(VALUE.get(u.t,0) for u in units[3-s])
    if A==0: return 0
    if B==0 or B>=A: return len(units[s])
    y=math.exp(math.log(A/B)/3.3)
    if y>1.3: y=(y+1.3)/2
    if y>2.0: y=2.0
    k=int(B*y+0.5); i=0
    while len(units[s])>i and k>0:
        i+=1; k-=VALUE.get(units[s][i-1].t,0)
    for u in units[s][i:]: u.tgt=1 if (len(units[3-s])>0 and u.t==8) else 0
    return i
def strength(f): return 21*f.get('bb',0)+7*f.get('ca',0)+2*f.get('dd',0)
def roe_result(roe,S,E,spd_own,spd_en):
    # 6cfe: 0 fight, 1 retreat, 2 scatter
    if roe_willing(roe,S,E): return 0
    if S==0: return 2
    if E/S>4 and spd_en>=spd_own: return 2
    return 1
def battle(A,T,base=False,start_dist=None,bug_flee=True):
    """A,T: dicts with ships + roe + maxsp (+0x09) (+order/speed for guard). base: T guards a base here (adds SB unit)."""
    u1,c1=build(A); u2,c2=build(T)
    if base: u2.append(U(8))
    units={1:u1,2:u2}; c={1:c1,2:c2}
    init={1:len(u1),2:len(u2)}
    S={1:strength(A)+(30 if A.get('guarding') else 0), 2:strength(T)+(30 if base else 0)}
    spd={1:A['maxsp'],2:T.get('maxsp',0)}
    rng={1:0,2:0}
    roe={1:A['roe'],2:T['roe']}
    dist=5+rnd(20) if start_dist is None else start_dist
    retreat={1:False,2:False}; scatter={1:False,2:False}; flee_start=0
    base_killed=False; esc={1:[],2:[]}; dead={1:[],2:[]}
    rnd_=1; done=False
    r=roe_result(roe[1],S[1],S[2],spd[1],spd[2])   # pre-check (only matters for surprise)
    def inrange(s,u):
        t=3-s
        if scatter[t] and u.tgt:
            e=units[t][u.tgt-1]
            fs=flee_start if bug_flee else (rnd_-flee_start)
            d=dist-(u.spd-e.spd)*fs
        else: d=dist
        return u.rng>=d
    while not done:
        for s in (1,2):
            t=3-s; focus=0
            n=engaged(units,s)
            for i,u in enumerate(units[s][:n],1):
                if u.dmg==0: continue
                if not (u.tgt and units[t][u.tgt-1].hp>=1):
                    if focus and roll(30) and units[t][focus-1].hp>0:
                        u.tgt=focus
                        if u.dmg>=units[t][focus-1].hp: focus=0
                    else:
                        if i<=c[t]: u.tgt=rnd(c[t])+1
                        elif retreat[t] or scatter[t] or c[t]==0: u.tgt=rnd(len(units[t]))+1
                        else: u.tgt=rnd(c[t])+1
                        focus=u.tgt
                if inrange(s,u) and u.tgt and roll(u.hit):
                    e=units[t][u.tgt-1]
                    d=max(0,rnd(u.dmg)+1-e.arm); e.hp=max(0,e.hp-d)
        for s in (1,2):
            t=3-s; i=0
            while i<len(units[s]):
                i+=1
                if units[s][i-1].hp<1:
                    if units[s][i-1].t==8: base_killed=True
                    for e in units[t]:
                        if e.tgt==i: e.tgt=0
                        elif e.tgt>i: e.tgt-=1
                    dead[s].append(units[s].pop(i-1).t)
                    if not units[s]: done=True
                    if i<=c[s]: c[s]-=1
                    i-=1
        if (scatter[1] or scatter[2]) and not done:
            X=2 if scatter[2] else 1; Y=3-X
            el=rnd_-flee_start; i=0
            while i<len(units[X]):
                i+=1; u=units[X][i-1]
                if (30-dist)>el*u.spd: continue
                ok=True
                if (30-dist)>el*(u.spd-spd[Y]):
                    for p in units[Y]:
                        if not ok: break
                        if p.tgt==i:
                            g=el*(u.spd-p.spd)
                            if p.rng>=g or p.rng>=dist or p.spd>u.spd: ok=False
                if ok:
                    for p in units[Y]:
                        if p.tgt==i: p.tgt=0
                        elif p.tgt>i: p.tgt-=1
                    esc[X].append(units[X].pop(i-1).t)
                    if u.t in (1,2,3): c[X]-=1
                    i-=1
                    if not units[X]: done=True
        else:
            for s in (1,2):
                Ssum=0; sp=10; mr=0
                for u in units[s]:
                    Ssum+=STRW.get(u.t,0); sp=min(sp,SPEED[u.t]); mr=max(mr,RANGE.get(u.t,0))
                S[s]=Ssum; spd[s]=sp; rng[s]=mr
            ro2=10 if (base and not base_killed) else roe[2]
            r2=roe_result(ro2,S[2],S[1],spd[2],spd[1])
            if r2==0: scatter[2]=retreat[2]=False
            elif r2==1: retreat[2]=True
            else: scatter[2]=True; flee_start=rnd_
            if base and not base_killed: retreat[2]=False
            if init[2]==1 or (init[2]==2 and base): scatter[2]=False
            if not scatter[2]:
                r1=roe_result(roe[1],S[1],S[2],spd[1],spd[2])
                if r1==0: scatter[1]=retreat[1]=False
                elif r1==1: retreat[1]=True
                else: scatter[1]=True; flee_start=rnd_
        if init[1]==1: scatter[1]=False
        dist+= spd[1] if (scatter[1] or retreat[1]) else -spd[1]
        dist+= spd[2] if (scatter[2] or retreat[2]) else -spd[2]
        if (scatter[1] or retreat[1]) and rng[2]<dist and spd[1]>=spd[2]: done=True
        if (scatter[2] or retreat[2]) and rng[1]<dist and spd[2]>=spd[1]: done=True
        rnd_+=1
        if dist<0: dist=0
        if (c[1]==0 and c[2]==0) or dist>30: done=True
        if not done and (retreat[1] or retreat[2] or scatter[1] or scatter[2]) and spd[1]==spd[2]:
            done=True
            for s in (1,2):
                for u in units[s]:
                    if u.tgt and inrange(s,u): done=False
        if rnd_>2000: break
    out={}
    for s in (1,2):
        cnt={}
        for u in units[s]+[U(t) for t in esc[s]]:
            cnt[u.t]=cnt.get(u.t,0)+1
        out[s]=dict(bb=cnt.get(3,0),ca=cnt.get(2,0),dd=cnt.get(1,0),sc=cnt.get(4,0),tt=cnt.get(5,0),tl=cnt.get(6,0),et=cnt.get(7,0),sb=cnt.get(8,0),
                    esc=len(esc[s]),retreat=retreat[s],scatter=scatter[s])
    out['rounds']=rnd_-1; out['base_killed']=base_killed
    return out
