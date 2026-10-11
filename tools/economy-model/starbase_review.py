"""t085 exact starbase economics controls on approved smooth/P3 economy.

No movement/engine/combat simulation. Arrival delays are imposed whole-pass
controls. Income payback is extra local revenue, not free transferable cash.
"""
import json
from fractions import Fraction as F
from pathlib import Path

from accounting_review import render, step, whole_capacity

COST = 50


def bonus(tax, peak=F(1,2)):
    return peak if tax <= 50 else peak*max(F(0),F(65-tax,15))


def grow(p,k,tax,bases,peak=F(1,2)):
    if not bases or not bonus(tax,peak):
        return step(p,k,tax,'smooth')
    # Extra base growth uses the old production gap, like the normal term.
    updated,_=step(p,k,tax,'smooth')
    extra=(k-p)*F(100-min(tax,95),400)*bonus(tax,peak)
    updated=min(F(k),updated+extra)
    return updated,updated*tax/100


def growth_control(k,start,tax,delay=0,peak=F(1,2),years=80):
    without,with_base=F(start),F(start)
    income_without=income_with=F(0)
    payback=None
    snapshots={}
    for year in range(1,years+1):
        without,y0=grow(without,k,tax,0,peak)
        with_base,y1=grow(with_base,k,tax,int(year>delay),peak)
        income_without+=y0
        income_with+=y1
        if payback is None and income_with-income_without>=COST:
            payback=year
        if year in (5,10,20,40,80):
            snapshots[year]=dict(production=with_base,production_without=without,
                extra_income=income_with-income_without,net_after_base_cost=income_with-income_without-COST)
    a=F(100-min(tax,95),400)
    b=bonus(tax,peak)
    # For these no-penalty bonus cases, the geometric extra-income sum is
    # finite, including the unboosted growth during the delay.
    lifetime_extra=F(tax,100)*(k-start)*(1-a)**delay*b/(a*(1+b)) if b else F(0)
    return dict(k=k,start=start,tax=tax,delay=delay,peak=peak,at=snapshots,
                payback_pass=payback,lifetime_extra_income=lifetime_extra)


def local_build(k,tax,years=60):
    p,goods=F(min(k,3)),F(0)
    order_pass=completion=active=None
    extra=F(0)
    payback=None
    baseline=F(min(k,3))
    snapshots={}
    for year in range(1,years+1):
        # Order after a pass, complete at next pass, commission after that;
        # first boosted economic pass is one pass later. No other spending.
        if completion==year:
            goods-=COST
        p,y=grow(p,k,tax,int(active is not None and year>=active))
        baseline,y0=grow(baseline,k,tax,0)
        extra+=y-y0
        if payback is None and extra>=COST:
            payback=year
        goods+=y
        if order_pass is None and min(goods//1,whole_capacity(p))>=COST:
            order_pass,completion,active=year,year+1,year+2
        if year in (10,20,40,60):
            snapshots[year]=dict(production=p,goods=goods,extra_income=extra,net_after_base_cost=extra-(COST if completion is not None and year>=completion else 0))
    return dict(k=k,tax=tax,order_pass=order_pass,completion_pass=completion,
                first_bonus_pass=active,payback_pass=payback,at=snapshots)


def surge(start_goods,bases,years=5):
    # Mature P100, tax50; fill every available spending point each pass.
    # Generic build-point throughput: no unit/slot/commission constraints.
    goods=F(start_goods)
    rows=[]
    for year in range(1,years+1):
        spent=min(goods//1,whole_capacity(F(100),bool(bases)))
        goods-=spent
        goods+=50
        rows.append(dict(year=year,spent=spent,goods=goods))
    return rows


def cycle_path(start,k,taxes,cycles=1,bases=1):
    p,goods=F(start),F(0)
    for tax in taxes*cycles:
        p,income=grow(p,k,tax,bases)
        goods+=income
    return dict(production=p,income=goods)


def cycle_control(start,k,taxes,bases=1):
    # Compose P_next=q*P+c over the cycle; all coefficients are rational.
    q,c=F(1),F(0)
    for tax in taxes:
        a=F(100-min(tax,95),400)*(1+bonus(tax) if bases else 1)
        b=F(max(0,tax-65),500)
        q,c=(1-a-b)*q,(1-a-b)*c+a*k
    steady_start=c/(1-q)
    steady=cycle_path(steady_start,k,taxes,bases=bases)
    assert steady['production']==steady_start
    comparisons=[]
    for cycles in (1,5,20):
        changed=cycle_path(start,k,taxes,cycles,bases)
        fixed=cycle_path(start,k,[65]*len(taxes),cycles,bases)
        comparisons.append(dict(cycles=cycles,cycle=changed,constant65=fixed,
                                extra_income=changed['income']-fixed['income']))
    return dict(start=start,k=k,taxes=taxes,bases=bases,comparisons=comparisons,
                steady_cycle_start=steady_start,
                steady_mean_income=steady['income']/len(taxes),
                steady65_mean_income=F(k*65,100))


def experiment():
    return dict(method='Exact rational arithmetic; approved smooth growth/P3, price50, nonstacking presence. Imposed delays, no engine/DOS runs or campaign odds.',
        growth=[growth_control(k,p,t) for k in (10,20,50,100,150)
                for p in (3,F(k,2),k) for t in (35,50,60,65,70)],
        delays=[growth_control(100,3,50,delay=d) for d in (0,2,5,10)],
        bonus_candidates=[growth_control(100,3,50,peak=b) for b in (F(0),F(1,4),F(1,2),F(1))],
        local_build=[local_build(k,t) for k in (10,20,50,100,150) for t in (35,50,60,65)],
        surge=[dict(initial_goods=g,bases=n,passes=surge(g,n)) for g in (50,500) for n in (0,1,3)],
        tax_cycles=[cycle_control(p,100,taxes,n) for p in (50,100) for taxes in ([75,64,64],[70,64]) for n in (0,1)],
        anti_stack=[dict(bases=n,production_after_one_pass=grow(F(3),100,50,n)[0],capacity=whole_capacity(F(100),bool(n))) for n in (0,1,2,5)])


if __name__=='__main__':
    Path(__file__).with_name('starbase_review_results.json').write_text(json.dumps(experiment(),default=render,indent=2)+'\n')
