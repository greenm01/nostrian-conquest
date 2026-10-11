"""t088 approved-economy prices and opening controls.

Exact local order/build dates plus paired synthetic peaceful maps. No real
map generator, exploration, encounters or campaign win-rate claim. Existing
starter fleet is2CA+2ETAC+2DD; home P/K100,10armies,4batteries,joined G50.
"""
import json
import math
from pathlib import Path
import statistics

from accounting_review import step,whole_capacity,render
from fractions import Fraction as F
from campaign import sampled_map,travel_years,HOME

PRICES=dict(DD=5,CA=15,BB=45,SC=15,TT=5,ETAC=20,SB=50,army=2,battery=20)


def local_dates(k,cost,tax=50,years=40):
    p,g=F(min(3,k)),F(0)
    # Zero is founding year; new production starts next maintenance.
    for year in range(1,years+1):
        p,income=step(p,k,tax,'smooth')
        g+=income
        if min(g//1,whole_capacity(p))>=cost:
            return dict(order_after_pass=year,complete_pass=year+1,
                        commission_after_pass=year+1,first_departure_pass=year+2)
    return None


def home_orders(costs,goods=50,tax=50):
    # Sequential desired whole orders; never borrow or order beyond budget.
    # All these homeworld bundles cost <=100 and complete next maintenance.
    pending=list(costs)
    date=0
    rows=[]
    while pending:
        budget=min(int(goods),100)
        while pending and pending[0]<=budget:
            cost=pending.pop(0)
            goods-=cost
            budget-=cost
            rows.append(dict(cost=cost,order_after_pass=date,complete_pass=date+1,
                             commission_after_pass=date+1,first_departure_pass=date+2))
        if pending:
            goods+=tax
            date+=1
    return rows


def peaceful(seed,policy='two',goods=50,etac_cost=20,speed=3,worlds=24,years=25):
    targets=sampled_map(seed,worlds)
    planets=[]  # coords,potential,founding pass
    ships=[dict(coords=HOME,ready=1,target=None,arrival=None) for _ in range(2)]
    extras=0 if policy=='two' else 2
    price=etac_cost+(15 if policy=='guarded4' else 0)
    total_paid=0
    milestones={k:None for k in ('first_colony','four_colonies','eight_colonies','colony_income_ge_home')}
    snapshots={}
    for year in range(1,years+1):
        budget=min(int(goods),100)
        # An order affordable at the preceding order window completes now.
        while extras and budget>=price:
            budget-=price
            goods-=price
            total_paid+=price
            extras-=1
            ships.append(dict(coords=HOME,ready=year+1,target=None,arrival=None))
        goods+=50
        colony_income=sum(.5*(k-(k-min(k,3))*.875**(year-born))
                          for _,k,born in planets)
        if milestones['colony_income_ge_home'] is None and colony_income>=50:
            milestones['colony_income_ge_home']=year
        for ship in ships:
            if ship['target'] is None and ship['ready']<=year and targets:
                target=min(targets,key=lambda t:(math.dist(ship['coords'],t[0]),t[0]))
                targets.remove(target)
                ship['target']=target
                ship['arrival']=year+travel_years(ship['coords'],target[0],speed)-1
            if ship['target'] is not None and ship['arrival']==year:
                coords,k=ship['target']
                planets.append((coords,k,year))
                ship.update(coords=coords,ready=year+1,target=None,arrival=None)
        for key,n in (('first_colony',1),('four_colonies',4),('eight_colonies',8)):
            if milestones[key] is None and len(planets)>=n:
                milestones[key]=year
        if year in (5,10,20,25):
            snapshots[year]=dict(colonies=len(planets),colony_income=colony_income,
                                 home_goods=goods,expansion_paid=total_paid)
    return dict(milestones=milestones,at=snapshots)


def summary(rows):
    out={}
    for key in rows[0]['milestones']:
        values=sorted(r['milestones'][key] for r in rows if r['milestones'][key] is not None)
        out[key]=dict(reach_fraction=len(values)/len(rows),mean=statistics.mean(values) if values else None,
                      median=statistics.median(values) if values else None,
                      p10=values[int(.1*(len(values)-1))] if values else None,
                      p90=values[int(.9*(len(values)-1))] if values else None)
    return dict(milestones=out,at={year:{key:statistics.mean(r['at'][year][key] for r in rows)
                           for key in rows[0]['at'][year]} for year in rows[0]['at']})


def experiment(maps=1000):
    configs=[dict(policy=p) for p in ('two','rapid4','guarded4')]
    configs += [dict(policy='rapid4',etac_cost=c) for c in (15,25,30)]
    configs += [dict(policy='rapid4',goods=g) for g in (35,65)]
    configs += [dict(policy='guarded4',goods=g) for g in (35,65,75)]
    configs += [dict(policy='two',worlds=n) for n in (12,48)]
    results=[]
    for config in configs:
        rows=[peaceful(880001+i,**config) for i in range(maps)]
        results.append(dict(config=config,maps=maps,seed=880001,**summary(rows)))
    return dict(method=__doc__,prices=PRICES,maps=results,
        local=[dict(k=k,unit=unit,cost=cost,dates=local_dates(k,cost))
               for k in (10,20,50,100) for unit,cost in PRICES.items()],
        bb_price=[dict(k=k,cost=c,dates=local_dates(k,c)) for k in (20,50,100) for c in (30,45,60)],
        openings=[dict(goods=g,choice=name,orders=home_orders(costs,g))
                  for g in (35,50,65,75) for name,costs in
                  [('two_ETAC',[20,20]),('two_escorted_ETAC',[35,35]),('BB',[45]),
                   ('base',[50]),('CA_and_battery',[35]),('six_loaded_TT',[42]),
                   ('six_CA',[15]*6),('three_BB',[45]*3),('ten_CA',[15]*10)]])


if __name__=='__main__':
    Path(__file__).with_name('opening_review_results.json').write_text(
        json.dumps(experiment(),default=render,indent=2)+'\n')
