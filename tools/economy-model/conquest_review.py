"""t086 exact conquest/conversion and exposed-industry controls.

Approved smooth economy, not current integer engine or a DOS emulator.
Capture occurs after the economy in year N; row 1 is N+1. No random combat.
The half-income candidate uses half growth, full emergency penalty at status1;
status0 is frozen as NC today. This is not classic's entire economy.
"""
import json
from fractions import Fraction as F
from pathlib import Path

from accounting_review import render, step, whole_capacity
from starbase_review import bonus


def conversion(p, k, tax, status, schedule, base=False):
    if status == 0 or (status == 1 and schedule == 'zero_zero_full'):
        return F(p), F(0)
    if status >= 2:
        return step(p, k, tax, 'smooth', base)
    scale = F(1,2)
    growth = (k-p)*F(100-min(tax,95),400)*(1+bonus(tax) if base else 1)
    penalty = p*F(max(0,tax-65),500)
    updated = min(F(k),max(F(0),p+scale*growth-penalty))
    return updated,updated*F(tax,100)*scale


def captured(k, start, tax, schedule, goods=0, owed=0, base=False, years=20,
             recapture_after=()):
    p, goods, income_total = F(start),F(goods),F(0)
    status = 0
    rows = []
    for year in range(1,years+1):
        # One already-existing order. No new orders or implicit credit.
        paid = min(owed,max(0,goods//1),whole_capacity(p,base))
        owed -= paid
        goods -= paid
        p,income = conversion(p,k,tax,status,schedule,base)
        status = min(2,status+1)
        goods += income
        income_total += income
        rows.append(dict(pass_number=year,production=p,income=income,
                         income_total=income_total,goods=goods,paid=paid,owed=owed))
        if year in recapture_after:
            status = 0
    return rows


def industry(p, goods, exchanges):
    """Proposed fractional-safe loss arithmetic at today's rates.

Inputs are hits actually reaching industry, after other targets; no fleet
AS conversion. Remainders expire per exchange. Destruction cannot erase debt.
"""
    p,goods=F(p),F(goods)
    for hits in exchanges:
        units = hits//10
        p -= min(p,units)
        goods -= min(max(F(0),goods),100*units)
    return p,goods


def bombard_control(start, goods, hits, tax=50, base=False, years=20):
    damaged,g = industry(start,goods,hits)
    p0,p1 = F(start),damaged
    lost = F(0)
    at = {}
    for year in range(1,years+1):
        p0,y0=step(p0,100,tax,'smooth',base)
        p1,y1=step(p1,100,tax,'smooth',base)
        lost+=y0-y1
        if year in (1,5,10,20):
            at[year]=dict(revenue_lost=lost,remaining_production_gap=p0-p1)
    return dict(start=start,goods=goods,exposed_hits=hits,tax=tax,base=base,
                production_lost=start-damaged,goods_lost=goods-g,at=at)


def experiment():
    result=dict(method='Exact fractions; approved smooth economy; no engine or classic-oracle execution, battle sampling, or campaign win-rate claim.',conversion=[],inherited_queue=[],bombardment=[])
    for k,p in ((10,10),(20,10),(100,10),(100,50),(100,100)):
        for tax in (50,65,70):
            for schedule in ('zero_zero_full','zero_half_full'):
                rows=captured(k,p,tax,schedule)
                result['conversion'].append(dict(k=k,start=p,tax=tax,schedule=schedule,
                    at={n:rows[n-1] for n in (1,2,3,5,10,20)}))
    for goods in (-10,F(9,2),0,50,200):
        for schedule in ('zero_zero_full','zero_half_full'):
            rows=captured(100,100,50,schedule,goods=goods,owed=12)
            result['inherited_queue'].append(dict(goods=goods,schedule=schedule,
                first_payment=next((r['pass_number'] for r in rows if r['paid']),None),
                completion=next((r['pass_number'] for r in rows if not r['owed']),None),
                first_four=rows[:4]))
    result['recapture']=captured(100,100,50,'zero_half_full',years=6,
                                 recapture_after=(1,2,3,4,5))
    for goods in (-10,0,50,500):
        for hits in ([9],[10],[30],[9,9,9],[100]):
            result['bombardment'].append(bombard_control(100,goods,hits))
    result['base_bombardment']=bombard_control(100,500,[10],base=True)
    result['scorch_potential_control']={str(k):captured(k,0,50,'zero_zero_full')
                                       for k in (0,100)}
    return result


if __name__ == '__main__':
    Path(__file__).with_name('conquest_review_results.json').write_text(
        json.dumps(experiment(),default=render,indent=2)+'\n')
