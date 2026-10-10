"""Independent balance model. No Rust execution or campaign simulation.
C = current combat arithmetic at ROE 10.
S = a concrete, literal synthesis candidate: VBAM CER; all suppression;
    free highest-DS critical kill; no fractional-hit carry.
R = alternative experiment: EotS CER; all suppression; budgeted critical
    focus; stochastic last step (expected residual/DS); no mixed bonus.
All modes keep full DS on both damage steps, and half AS when crippled.
"""
import random, math, json, hashlib
from collections import Counter

AS = [10,30,90,100,0,0,0]
DS = [5,30,100,120,10,10,20]
COST = [5,15,45,50,15,5,20]
LINE = [0,1,2,3]
AUX = [4,5,6]
CRIT = [0,4,5,6,1,2,3]
CRT = [[0,1,2,3,4],[1,2,3,4,5],[1,2,4,5,6],[2,3,4,5,6],
       [2,3,4,6,7],[2,4,5,6,7],[3,4,5,6,8],[3,4,6,7,8],
       [4,5,6,7,8],[4,6,7,8,10]]
CER = [1,2,2,3,3,3,3,4,4,5]  # tenths
EOTS = [1,1,1,2,2,2,4,4,4,4] # quarters; mean .625

def stats(n,c):
    a = sum((n[i]*2-c[i])*AS[i]//2 for i in LINE)
    sup = sum((n[i]*2-c[i])*AS[i]//2 for i in (1,2))
    return a,sup

def col(a,b):
    return 4 if b == 0 or a >= b*3 else 0 if a*2 < b else 1 if a < b else 2 if a*2 < b*3 else 3

def mixed(n): return sum(n[i]>0 for i in (0,1,2)) >= 2
def order(n): return LINE if any(n[i] for i in LINE) else AUX

def current_apply(n,c,sup,exe,critical,high=False):
    while sup > 0:
        target = order(n)
        progress = False
        for i in target:
            d=DS[i]
            if sup < d: break
            k=min(n[i]-c[i],sup//d)
            c[i]+=k; sup-=k*d; progress |= k>0
        if all(n[i]==c[i] for i in target):
            for i in target:
                d=DS[i]
                if sup<d: break
                k=min(c[i],sup//d)
                c[i]-=k; n[i]-=k; sup-=k*d; progress |= k>0
        if not progress: break
    while exe > 0:
        target=order(n); progress=False
        for i in target:
            d=DS[i]
            k=min(c[i],exe//d)
            c[i]-=k; n[i]-=k; exe-=k*d; progress |= k>0
            k=min(n[i]-c[i],exe//(2*d))
            n[i]-=k; exe-=k*2*d; progress |= k>0
        if not progress: break
    if critical:
        if high:
            eligible=order(n)
            candidates=[i for i in eligible if n[i]]
            if candidates:
                i=max(candidates,key=lambda i:DS[i])
                n[i]-=1
                if c[i]: c[i]-=1
        else:
            for pool in (True,False):
                for i in CRIT:
                    if (c[i] if pool else n[i]-c[i])>0:
                        n[i]-=1
                        if pool: c[i]-=1
                        return

def recommended_apply(n,c,hits,critical,rng):
    # Critical focuses its existing budget on expensive affordable combat
    # steps, permitting finishing before all other hulls are crippled.
    if critical:
        while hits:
            eligible=[i for i in order(n) if n[i] and hits>=DS[i]]
            if not eligible: break
            i=max(eligible,key=lambda i:DS[i])
            hits-=DS[i]
            if c[i]: c[i]-=1; n[i]-=1
            else: c[i]+=1
    while hits:
        eligible=order(n)
        nominal=[i for i in eligible if n[i]>c[i]]
        targets=nominal or [i for i in eligible if c[i]]
        if not targets: break
        i=targets[0]; d=DS[i]
        available=(n[i]-c[i]) if nominal else c[i]
        k=min(available,hits//d)
        if k:
            hits-=k*d
            if nominal: c[i]+=k
            else: c[i]-=k; n[i]-=k
            continue
        # A residual may buy one step, with probability proportional to cost.
        if rng.random() < hits/d:
            if nominal: c[i]+=1
            else: c[i]-=1; n[i]-=1
        break

def execution_apply(n,c,hits,rng):
    while hits:
        targets=[i for i in order(n) if n[i]]
        if not targets: break
        i=targets[0]
        if c[i]:
            d=DS[i]; available=c[i]; crippled=True
        else:
            d=2*DS[i]; available=n[i]; crippled=False
        k=min(available,hits//d)
        if k:
            n[i]-=k
            if crippled: c[i]-=k
            hits-=k*d
            continue
        if rng.random()<hits/d:
            n[i]-=1
            if crippled: c[i]-=1
        break

def battle(a0,b0,mode,rng,roe=None):
    a,b=list(a0),list(b0); ca,cb=[0]*7,[0]*7
    for r in range(1,65):
        aa,sa=stats(a,ca); ab,sb=stats(b,cb)
        if not aa or not ab: break
        rolls=[rng.randrange(10),rng.randrange(10)]
        shots=[]
        for n,c,own,enemy,supp,roll in ((a,ca,aa,ab,sa,rolls[0]),(b,cb,ab,aa,sb,rolls[1])):
            if mode=='C':
                column=min(4,col(own,enemy)+mixed(n)+(n[3]>0))
                h=(own*CRT[roll][column]+3)//4
                s=math.floor(h*supp/own+0.5)
                shots.append((s,h-s,roll==9))
            else:
                shift=(mixed(n)+(n[3]>0)) if mode=='S' else int(n[3]>ca[3] if n is a else n[3]>cb[3])
                h=(own*CER[min(9,roll+shift)]+9)//10 if mode=='S' else (own*EOTS[min(9,roll+shift)]+3)//4
                s=math.floor(h*supp/own+0.5) if mode=='X' else h
                shots.append((s,h-s,roll==9))
        if mode in ('R','X'):
            recommended_apply(a,ca,shots[1][0],shots[1][2],rng)
            recommended_apply(b,cb,shots[0][0],shots[0][2],rng)
            execution_apply(a,ca,shots[1][1],rng)
            execution_apply(b,cb,shots[0][1],rng)
        else:
            current_apply(a,ca,*shots[1],high=mode=='S')
            current_apply(b,cb,*shots[0],high=mode=='S')
        assert all(0<=ca[i]<=a[i] and 0<=cb[i]<=b[i] for i in range(7))
        na=stats(a,ca)[0]; nb=stats(b,cb)[0]
        if not na or not nb: break
        if roe is not None and mode in ('R','X'):
            # Per-round ROE6 only, for pacing illustration. No pursuit.
            if na<nb or nb<na: break
    na=stats(a,ca)[0]; nb=stats(b,cb)[0]
    outcome='A' if na>0 and nb==0 else 'B' if nb>0 and na==0 else 'mutual' if na==nb==0 else 'cap' if roe is None else 'retreat'
    return outcome,r,a,b,ca,cb

def force(dd=0,ca=0,bb=0,sb=0,sc=0,tt=0,et=0): return [dd,ca,bb,sb,sc,tt,et]

def run(label,a,b,mode,N=12000,roe=None):
    rng=random.Random(20261010); outcomes=Counter(); totalr=0; losta=lostb=0; cripa=cripb=0
    for _ in range(N):
        out,r,na,nb,ca,cb=battle(a,b,mode,rng,roe)
        outcomes[out]+=1; totalr+=r
        losta+=sum((a[i]-na[i])*COST[i] for i in range(7))
        lostb+=sum((b[i]-nb[i])*COST[i] for i in range(7))
        cripa+=sum(ca[i]*COST[i] for i in range(7))
        cripb+=sum(cb[i]*COST[i] for i in range(7))
    return dict(case=label,mode=mode,N=N,outcomes={k:round(v/N,5) for k,v in outcomes.items()},rounds=round(totalr/N,3),lost_cost_A=round(losta/N,3),lost_cost_B=round(lostb/N,3),crippled_cost_A=round(cripa/N,3),crippled_cost_B=round(cripb/N,3))

def ground(a,b,defender_tie=False):
    # Exact dynamic program: enumerate all 100 roll pairs each exchange.
    # Counts always fall, so recursion is acyclic.
    from functools import lru_cache
    @lru_cache(None)
    def f(a,b):
        if not b: return 1.,0.,0.
        if not a: return 0.,0.,0.
        win=rounds=mutual=0.
        for da in range(10):
            for db in range(10):
                ha=(a*CRT[da][col(a,b)]+3)//4+(da==9)
                hb=(b*CRT[db][min(4,col(b,a)+1)]+3)//4+(db==9)
                na=max(0,a-hb); nb=max(0,b-ha)
                if not na and not nb:
                    win+=a>b and not defender_tie; mutual+=1; rounds+=1
                else:
                    w,r,m=f(na,nb); win+=w; rounds+=1+r; mutual+=m
        return win/100,rounds/100,mutual/100
    return f(a,b)

def flat_ground(a,b):
    from functools import lru_cache
    @lru_cache(None)
    def f(a,b):
        if not b: return 1.,0.,0.
        if not a: return 0.,0.,0.
        win=rounds=mutual=0.
        for da in range(10):
            for db in range(10):
                ha=(a*EOTS[da]+3)//4
                hb=(b*EOTS[min(9,db+1)]+3)//4
                na=max(0,a-hb); nb=max(0,b-ha)
                if not na and not nb:
                    mutual+=1; rounds+=1
                else:
                    w,r,m=f(na,nb); win+=w; rounds+=1+r; mutual+=m
        return win/100,rounds/100,mutual/100
    return f(a,b)

def binomial_ground(a,b):
    """Simultaneous per-army fire: attacker .30, defender .35; no criticals.
    Mutual elimination leaves the owner in control. Exact finite-state sum.
    """
    from functools import lru_cache
    @lru_cache(None)
    def pmf(n,p): return [math.comb(n,k)*p**k*(1-p)**(n-k) for k in range(n+1)]
    @lru_cache(None)
    def f(a,b):
        if not b: return 1.,0.
        if not a: return 0.,0.
        pa,pb=pmf(a,.30),pmf(b,.35)
        win=0.; rounds=1.; stay=pa[0]*pb[0]
        for ka,qa in enumerate(pa):
            for kb,qb in enumerate(pb):
                if ka==kb==0: continue
                na=max(0,a-kb); nb=max(0,b-ka)
                w,r=f(na,nb) if na or nb else (0.,0.)
                win+=qa*qb*w; rounds+=qa*qb*r
        return win/(1-stay),rounds/(1-stay)
    return f(a,b)

if __name__=='__main__':
    cases=[('30 DD vs 10 CA',force(dd=30),force(ca=10)),
           ('90 DD vs 10 BB',force(dd=90),force(bb=10)),
           ('30 CA vs 10 BB',force(ca=30),force(bb=10)),
           ('3 DD vs 1 CA',force(dd=3),force(ca=1)),
           ('3 CA vs 1 BB',force(ca=3),force(bb=1)),
           ('1 DD vs 1 BB',force(dd=1),force(bb=1)),
           ('10 DD vs 1 SB',force(dd=10),force(sb=1)),
           ('29 CA + 3 DD vs 30 CA',force(ca=29,dd=3),force(ca=30))]
    cases += [(f'{n} CA vs 10 CA',force(ca=n),force(ca=10)) for n in (10,11,15,20,30)]
    data=[]
    for label,a,b in cases:
        for mode in ('C','S','R','X'):
            result=run(label,a,b,mode)
            data.append(result); print(json.dumps(result),flush=True)
    for n in (10,11,15,20):
        result=run(f'{n} CA vs 10 CA ROE6',force(ca=n),force(ca=10),'R',roe=6)
        data.append(result); print(json.dumps(result),flush=True)
    grounds={str(n):{'current':ground(n,10),'defender_mutual':ground(n,10,True)} for n in (10,11,15,20,30)}
    print('GROUND',json.dumps(grounds),flush=True)
    print('MEANS',[sum(row[c] for row in CRT)/40 for c in range(5)],flush=True)
    with open(__import__('os').path.join(__import__('os').path.dirname(__import__('os').path.abspath(__file__)),'nc_combat_results.json'),'w') as f: json.dump({'battles':data,'ground':grounds},f,indent=2)
