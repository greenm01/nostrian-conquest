"""t087 recoverable Scorch controls. Enumerate all twenty annual factors.

Production is exact rational arithmetic. Goods use floating-point sqrt,
not a Real48 emulator. No new random economic events are proposed.
Compare one/two/three scheduled scorch passes, then cleared-order recovery
or capture with approved NC 0/0/full conversion. Tax50, no qualifying base.
"""
from collections import Counter
from fractions import Fraction as F
import json
import math
from pathlib import Path

from accounting_review import render


def yearly(p,g,k,roll):
    p=min(F(k),max(F(1),p*F(30+roll,100))) if k else F(0)
    # Explicit NC adaptation: destruction preserves imported debt.
    return p,min(g/10,math.sqrt(g)) if g>0 else g


def capture(p,g,k):
    p=min(F(k),max(F(1),p/2)) if k else F(0)
    return p,g/2 if g>0 else g


def recover(p,k,years=20,conversion=False):
    # At tax50, no base, no high-tax penalty: exact geometric sequence.
    n=max(0,years-2) if conversion else years
    q=F(7,8)
    total=F(k*n,2)-F(7,2)*(k-p)*(1-q**n)
    end=k-(k-p)*q**n
    reach90=next((i+(2 if conversion else 0) for i in range(0,101)
                  if k-(k-p)*q**i>=F(9*k,10)),None) if k else None
    return total,end,reach90


def control(k,start,goods,passes,captured):
    states=Counter({F(start):1})
    g=float(goods)
    for _ in range(passes):
        out=Counter()
        for p,count in states.items():
            for die in range(20):
                p2,_=yearly(p,g,k,die)
                out[p2]+=count
        states=out
        _,g=yearly(F(start),g,k,0)
    if captured:
        out=Counter()
        for p,count in states.items():
            p2,_=capture(p,g,k)
            out[p2]+=count
        states=out
        _,g=capture(F(start),g,k)
    count=sum(states.values())
    rows=[(p,c,*recover(p,k,conversion=captured)) for p,c in states.items()]
    return dict(k=k,start=start,initial_goods=goods,scorch_passes=passes,
                capture_after=captured,goods_left=g,production_min=min(states),
                production_max=max(states),production_mean=sum(p*c for p,c in states.items())/count,
                income_next20=sum(c*income for p,c,income,end,date in rows)/count,
                production_after20=sum(c*end for p,c,income,end,date in rows)/count,
                reach90_min=min(date for p,c,income,end,date in rows),
                reach90_max=max(date for p,c,income,end,date in rows))


def experiment():
    return dict(method=__doc__,unscorched_capture_baseline=dict(income_next20=recover(F(100),100,conversion=True)[0],production=100,goods=500),rows=[control(k,p,g,n,c) for k,p,g in
        ((10,10,50),(100,100,500),(100,50,50)) for n in (0,1,2,3) for c in (False,True)],
        debt=[dict(before=g,annual=yearly(F(100),g,100,0)[1],capture=capture(F(100),g,100)[1])
              for g in (-100,-1,0,50,100,500)])


if __name__=='__main__':
    Path(__file__).with_name('scorch_review_results.json').write_text(
        json.dumps(experiment(),default=render,indent=2)+'\n')
