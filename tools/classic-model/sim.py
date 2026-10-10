"""Python model of ECMAINT planetary assault (110a) as decoded for t036."""
import random, math
R=random.Random()
def rnd(n): return R.randrange(n) if n>0 else 0
def roll(p): return R.random()*100 <= p
# type: (range, hp, dmg, speed, armor, hit)
STATS={1:(10,3,4,6,0,55),2:(15,9,12,5,1,50),3:(20,30,36,4,2,45),4:(0,3,0,6,0,55),
       5:(0,5,0,5,1,40),6:(0,5,0,5,1,40),7:(0,18,0,3,1,40),11:(20,18,36,0,1,65)}
VALUE={1:1,2:3,3:9,8:15,11:8}
STR={1:2,2:7,3:21,8:30}
class U:
    __slots__=('t','rng','hp','dmg','spd','arm','hit','tgt')
    def __init__(s,t):
        s.t=t; s.rng,s.hp,s.dmg,s.spd,s.arm,s.hit=STATS[t]; s.tgt=0
def roe_willing(roe,S,E):
    if S>0 and E>0:
        r=S/E
        return {0:False,1:False,2:r>=4,3:r>=3,4:r>=2,5:r>=1.5,6:r>=1,7:E/S<=1.5,8:E/S<=2,9:E/S<=3,10:True}[roe]
    return S!=0
def engaged(units,s,expo=4.0,cap=1.5,ycap2=None):
    A=sum(VALUE.get(u.t,0) for u in units[s]); B=sum(VALUE.get(u.t,0) for u in units[3-s])
    if A==0: return 0
    if B==0 or B>=A: return len(units[s])
    y=math.exp(math.log(A/B)/expo)
    if y>cap: y=(y+cap)/2
    if ycap2 is not None and y>ycap2: y=ycap2
    k=int(B*y+0.5); i=0
    while len(units[s])>i and k>0:
        i+=1; k-=VALUE.get(units[s][i-1].t,0)
    for u in units[s][i:]: u.tgt=1 if (len(units[3-s])>0 and u.t==8) else 0
    return i
class Assault:
    def __init__(s,fl,pl,order,ghost=True):
        # fl: dict bb ca dd sc tt tl et roe ; pl: dict armies bat r3 r9 prod2 dock(list8) x60
        s.fl=dict(fl); s.pl=dict(pl); s.pl['dock']=list(pl.get('dock',[0]*8)); s.order=order; s.ghost=ghost
        u=[]
        for t,k in ((3,'bb'),(2,'ca'),(1,'dd')):
            u+= [U(t) for _ in range(fl.get(k,0))]
        s.ncombat=len(u)
        for t,k in ((4,'sc'),(5,'tt'),(6,'tl'),(7,'et')): u+=[U(t) for _ in range(fl.get(k,0))]
        s.units={1:u,2:[U(11) for _ in range(pl['bat'])]}
        s.B0=pl['bat']; s.retreat=False; s.round=0; s.kills={1:{},2:{}}
        s.w={6:(10,20,30,40),7:(20,25,25,30),8:(0,0,0,100)}[order]
        s.roe=fl.get('roe',10) if (fl.get('bb',0)+fl.get('ca',0)+fl.get('dd',0))>0 else 0
        s.fleet_dead=False
    def recompute(s):
        S=0; sp=10
        for u in s.units[1]:
            S+=STR.get(u.t,0); sp=min(sp,u.spd)
        return S,17*len(s.units[2]),sp
    def remove_dead(s,on_fleet_index=None):
        done=False
        for side in (1,2):
            t=3-side; i=0
            while i<len(s.units[side]):
                i+=1
                if s.units[side][i-1].hp<1:
                    s.kills[side][s.units[side][i-1].t]=s.kills[side].get(s.units[side][i-1].t,0)+1
                    for e in s.units[t]:
                        if e.tgt==i: e.tgt=0
                        elif e.tgt>i: e.tgt-=1
                    dead=s.units[side].pop(i-1)
                    if len(s.units[side])<1:
                        done=True
                        if side==1: s.fleet_dead=True; s.ghost_unit=dead
                    else:
                        if on_fleet_index: on_fleet_index(side,i)
                    i-=1
        return done
    def fire(s,side,n,dist,landed=None,blitz=None):
        t=3-side; focus=0; D=0
        for u in s.units[side][:n]:
            if u.dmg<=0 or u.rng<dist: continue
            if blitz is not None and side==1 and landed: continue
            if u.tgt==0:
                if focus and roll(30) and s.units[t][focus-1].hp>0:
                    u.tgt=focus
                    if u.dmg>=s.units[t][focus-1].hp: focus=0
                else:
                    if t==1:
                        if blitz is not None:
                            if blitz['approach'] and roll(40): u.tgt=rnd(blitz['cnt'])+blitz['first']
                            else: u.tgt=rnd(blitz['last'])+1
                        else: u.tgt=rnd(s.ncombat)+1
                    else: u.tgt=rnd(len(s.units[t]))+1
                    focus=u.tgt
            if roll(u.hit) and u.tgt:
                e=s.units[t][u.tgt-1]
                d=rnd(u.dmg)+1-e.arm
                if blitz is not None and d<0:   # classic unsigned-compare bug in 53bb
                    d16=d&0xffff
                    if t==2: D+=d16
                    e.hp=(e.hp-d16)&0xffff
                    if e.hp>=0x8000: e.hp-=0x10000
                    if e.hp<0: e.hp=0
                    continue
                d=max(0,d)
                if t==2: D+=d
                e.hp=max(0,e.hp-d)
        return D
    def battle(s,rounds=10):     # 110a:4ba4
        for side in (1,2):
            for u in s.units[side]: u.tgt=0
        s.round=1; dist=25; D=0; done=False
        def fidx(side,i):
            if side==1 and i<=s.ncombat:
                s.ncombat-=1
        while not done and s.round<=rounds:
            S,E,sp=s.recompute()
            if dist<20 and not s.retreat and not roe_willing(s.roe,S,E): s.retreat=True
            for side in (1,2):
                n=min(engaged(s.units,side),len(s.units[side]))
                D+=s.fire(side,n,dist)
            # removal with combat-count bookkeeping
            for side in (1,2):
                t=3-side; i=0
                while i<len(s.units[side]):
                    i+=1
                    if s.units[side][i-1].hp<1:
                        k=s.units[side][i-1].t; s.kills[side][k]=s.kills[side].get(k,0)+1
                        for e in s.units[t]:
                            if e.tgt==i: e.tgt=0
                            elif e.tgt>i: e.tgt-=1
                        dead=s.units[side].pop(i-1)
                        if len(s.units[side])<1:
                            done=True
                            if side==1: s.fleet_dead=True; s.ghost_unit=dead
                        else:
                            if i<=s.ncombat and side==1:
                                s.ncombat-=1
                                if s.ncombat==0: done=True
                        i-=1
            if dist<0: dist=0
            if s.retreat and dist>=25: done=True
            if not done:
                if s.retreat and dist<25: dist+=sp
                elif dist>5 and not s.retreat: dist-=sp
            s.round+=1
        s.collateral(D)
    def dock_value(s):
        d=s.pl['dock']; return 3*d[0]+18*d[1]+30*d[2]+3*d[3]+5*d[4]+18*d[5]+60*d[6]+2*d[7]
    def dock_remove_to(s,target,bytebug=False):
        while s.dock_value()>target:
            tot=sum(s.pl['dock'])
            r=rnd(tot&0xffff)+1
            if bytebug: r&=0xff
            c=0; cum=0
            while r>cum+s.pl['dock'][c]:
                cum+=s.pl['dock'][c]; c+=1
            s.pl['dock'][c]-=1
    def collateral(s,D):        # 110a:5d44
        p=s.pl
        k=math.sqrt(D*0.25)
        if k<1: k=1.0
        B1=len(s.units[2])
        if s.B0>0:
            alt=(s.B0-B1)*90/s.B0
            if alt<k: k=alt
        had=p['armies']>0
        f=min(1,max(0,(100-k)/100)); p['r3']*=f
        f=min(1,max(0,(100-1.5*k)/100)); p['r9']*=f
        f=min(1,max(0,(100-2*k)/100))
        tgt=int(s.dock_value()*f)
        s.dock_remove_to(tgt,bytebug=True)
        lost=p['armies']-round(p['armies']*f)
        cap=(D-(s.B0-B1)*18)//6 if (D-(s.B0-B1)*18)>=0 else -((-(D-(s.B0-B1)*18))//6)
        if lost>cap: lost=cap
        if lost<0: lost=0
        p['armies']-=lost
        if p['armies']<1 and had: p['armies']=1
        p['bat']=B1
    def flags(s,dockdmg):
        p=s.pl
        f1=p['armies']>1 or (p['armies']==1 and p['r3']/p['prod2']<0.03)
        return (f1, p['r3']>0, p['r9']>0, s.dock_value()>dockdmg)
    def bombard(s,R):           # 110a:6367
        w=s.w; thr=(w[0],w[0]+w[1],w[0]+w[1]+w[2],100)
        dockdmg=0; fl=s.flags(dockdmg); done=not any(fl)
        rnd_=s.round
        if s.retreat:
            rnd_=R
            if roll(5): rnd_-=1
        ships=s.units[1][:s.ncombat]
        if s.fleet_dead and s.ghost and s.ncombat>0: ships=[s.ghost_unit]   # classic ghost (only 1 stale slot modeled)
        if s.fleet_dead and not s.ghost: ships=[]
        while rnd_<R and not done:
            rnd_+=1
            for u in ships:
                while True:
                    r=rnd(100)+1; c=0
                    while thr[c]<r: c+=1
                    if fl[c] or done: break
                if roll(u.hit) and not done:
                    d=max(0,rnd(u.dmg)+1-2)
                    if c==0:
                        if d>=2 and s.pl['armies']>0: s.pl['armies']-=1
                    elif c==1: s.pl['r3']=max(0,s.pl['r3']-d*0.1)
                    elif c==2: s.pl['r9']=max(0,s.pl['r9']-d*0.5)
                    else:
                        dockdmg+=2*d
                        if s.dock_value()<dockdmg: dockdmg=s.dock_value()
                    fl=s.flags(dockdmg); done=not any(fl)
            if s.order==7 and s.pl['armies']*4<=s.fl.get('tl',0) and s.fl.get('tl',0)-s.pl['armies']>4: done=True
        s.dock_remove_to(s.dock_value()-dockdmg)
        if s.pl['r3']<1: s.pl['r3']=1.0
    def landing(s):             # 110a:53bb
        for side in (1,2):
            for u in s.units[side]: u.tgt=0
        idx=[i+1 for i,u in enumerate(s.units[1]) if u.t in (5,6)]
        b=dict(cnt=len(idx),first=idx[0] if idx else 0,approach=True)
        b['last']=b['first']+b['cnt']-1
        s.round=0; dist=25; D=0; landed=0
        done=not (s.fl.get('tl',0)>s.kills[1].get(6,0))
        while not done:
            for side in (1,2):
                n=min(engaged(s.units,1),len(s.units[1])) if side==1 else len(s.units[2])
                D+=s.fire(side,n,dist,landed=landed,blitz=b)
            for side in (1,2):
                t=3-side; i=0
                while i<len(s.units[side]):
                    i+=1
                    if s.units[side][i-1].hp<1:
                        k=s.units[side][i-1].t; s.kills[side][k]=s.kills[side].get(k,0)+1
                        for e in s.units[t]:
                            if e.tgt==i: e.tgt=0
                            elif e.tgt>i: e.tgt-=1
                        s.units[side].pop(i-1)
                        if len(s.units[side])<1:
                            done=True
                            if side==1: s.fleet_dead=True
                        elif side==1:
                            if s.ncombat>=i: s.ncombat-=1; b['last']-=1; b['first']-=1
                            else: b['last']-=1; b['cnt']-=1
                        i-=1
            if not s.units[1] or not s.units[2]: done=True
            if done and (not b['approach'] or b['cnt']==0):
                s.round+=1; continue
            if b['approach']:
                if dist>0 and not done:
                    if s.fl.get('tl',0)>s.kills[1].get(6,0):
                        dist=max(0,dist-5)
                    else: b['approach']=False
                else:
                    landed=max(0,s.fl.get('tl',0)-s.kills[1].get(6,0)); b['approach']=False
                    for u in s.units[1][b['first']-1:b['last']]:
                        if u.t==6: u.t=5
            else:
                dist+=5
                if dist>=25: done=True
            s.round+=1
        s.collateral(D)
        return landed
    def ground(s,L):           # 110a:5260
        pa=40+rnd(10); pd=30; Dn=s.pl['armies']
        while L>0 and Dn>0:
            n=Dn
            for _ in range(n):
                if L>0 and roll(pa): L-=1
            for _ in range(L):
                if Dn>0 and roll(pd): Dn-=1
        s.pl['armies']=max(0,Dn)
        return max(0,L)
    def capture(s,L):          # 110a:6a9f
        p=s.pl; arm0=s.arm0
        if L<=0: return False
        p['armies']=L
        if s.order==7:
            if p['r9']>1.0: p['r9']*=rnd(400)/1000+0.1
            else: p['r9']=1.0
            p['r3']*=rnd(200)/1000+0.2
            p['bat']=0
            r=R.random()
            p['dock']=[round(r*v*(rnd(800)/1000+0.2)) for v in p['dock']]
        else:
            f=min(1.0,math.sqrt(L/arm0)) if arm0>0 else 1.0
            p['r3']*=f*(rnd(150)/1000+0.85)
            p['r9']*=f*(rnd(450)/1000+0.55)
            p['bat']=round(p['bat']*f*(rnd(650)/1000+0.35))
            p['dock']=[round(v*f*(rnd(800)/1000+0.2)) for v in p['dock']]
        if p.get('x60'):
            p['r3']*=0.5; p['r9']*=0.5; p['dock']=[0]*8; p['x60']=0
        if p['r9']<1.0: pass
        if p['r3']<1.0: p['r3']=1.0
        p['captured']=True
        return True
    def run(s):
        s.arm0=s.pl['armies']
        L=0
        if s.order==6:
            if s.pl['bat']>0: s.battle(10)
            s.bombard(20)
        elif s.order==7:
            if s.pl['bat']>0: s.battle(10)
            if len(s.units[2])==0:
                s.bombard(12)
                L=s.landing()
                L=s.ground(L)
        else:
            L=s.landing()
            if L>0: L=s.ground(L)
        if L>0: s.capture(L)
        out={'fleet_dead':s.fleet_dead}
        cnt={}
        for u in s.units[1]: cnt[u.t]=cnt.get(u.t,0)+1
        out.update(bb=cnt.get(3,0),ca=cnt.get(2,0),dd=cnt.get(1,0),tt=cnt.get(5,0),tl=cnt.get(6,0))
        out.update(armies=s.pl['armies'],bat=s.pl['bat'],r3=s.pl['r3'],r9=s.pl['r9'],dock=s.pl['dock'],captured=s.pl.get('captured',False))
        return out
