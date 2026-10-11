"""t072 three-exchange bombardment: exact rolls, collateral-order controls.

Current ratio support fire and optional flat-rate sensitivity. One force,
no preceding orbital gate, no pursuit, no reinforcements. Same round-start
shots remain simultaneous. Dock items cost one DS, as in current code.
"""
from collections import Counter
from functools import lru_cache
import json
from pathlib import Path

from fleet_model import Model, force, base_column
from rules import RULESETS, UNITS, CLASSES, CRT_X4, EOTS_X4, variant

MODEL=Model(variant(RULESETS['engine-t060'],'planetary',remainder='discard'),UNITS)
DD,CA,BB,SB,SC,TT,ET=range(7)


def bombard_as(side):
    n,c,_=side
    return sum((n[i]-c[i])*full+c[i]*half for i,full,half in ((DD,5,2),(CA,30,15),(BB,135,67)))


def shot(attack, defense, roll, mixed=False, flat=False):
    if not attack:
        return 0,False
    k=EOTS_X4[min(9,roll+mixed)] if flat else CRT_X4[roll][min(4,base_column(attack,defense)+mixed)]
    return (attack*k+3)//4,roll==9


def dock_damage(dock,hits):
    out=[]
    for cls,count in dock:
        ds=UNITS[cls]['DS']
        killed=min(count,hits//ds)
        out.append((cls,count-killed))
        hits-=killed*ds
    return tuple(out),hits


def damage(planet,hits,round_,order):
    b,dock,a,g,p=planet
    breakthrough=round_==3 and b==0
    if order=='current':
        dock,hits=dock_damage(dock,hits)
        loss=min(b,hits//20);b-=loss;hits-=20*loss
    elif b:
        loss=min(b,hits//20);b-=loss;hits-=20*loss
        if order=='delayed':
            return b,dock,a,g,p
        if b==0:
            dock,hits=dock_damage(dock,hits)
    else:
        dock,hits=dock_damage(dock,hits)
    if breakthrough:
        loss=min(a,hits//10);a-=loss;hits-=10*loss
        p=max(0,p-hits//10)
        g=max(0,g-100*(hits//10))
    return b,dock,a,g,p


def solve(start,planet,order,flat=False):
    @lru_cache(None)
    def battle(side,world,r):
        if r>3 or bombard_as(side)==0:
            return (MODEL.cost_of(side[0]),world[0],sum(UNITS[c]['cost']*n for c,n in world[1]),world[2],world[3],world[4],float(any(side[0])))
        as_a,as_b=bombard_as(side),world[0]*90
        mixed=sum(side[0][i]>0 for i in (DD,CA,BB))>=2
        shots=Counter(shot(as_a,max(1,as_b),die,mixed,flat) for die in range(10))
        returns=Counter(shot(as_b,max(1,as_a),die,False,flat) for die in range(10))
        result=[0.]*7
        for (hits,crit),ka in shots.items():
            after=damage(world,hits+crit,r,order)
            for (back,critical),kd in returns.items():
                surviving=MODEL.apply(side,0,back,critical)[0][1]
                row=battle(surviving,after,r+1)
                result=[x+ka*kd*y/100 for x,y in zip(result,row)]
        return tuple(result)
    result=battle(start,planet,1)
    initial=(MODEL.cost_of(start[0]),planet[0],sum(UNITS[c]['cost']*n for c,n in planet[1]),planet[2],planet[3],planet[4])
    return dict(zip(('ship_cost_lost','batteries_lost','dock_cost_lost','armies_lost','goods_lost','production_lost'),(a-b for a,b in zip(initial,result))),survival=result[6],states=battle.cache_info().currsize)


def run():
    cases=[('BB1-vs-B2-empty',dict(BB=1),(2,(),10,500,100)),
           ('BB1-vs-B2-dockedBB',dict(BB=1),(2,(('BB',1),),10,500,100)),
           ('CA6-vs-B3-dockedBB',dict(CA=6),(3,(('BB',1),),10,500,100)),
           ('BB3-vs-B3-dockedBB',dict(BB=3),(3,(('BB',1),),10,500,100)),
           ('BB1-no-defense',dict(BB=1),(0,(),0,500,100)),
           ('BB1-garrison10',dict(BB=1),(0,(),10,500,100)),
           ('BB3-garrison10',dict(BB=3),(0,(),10,500,100)),
           ('DD1-no-defense',dict(DD=1),(0,(),0,500,100))]
    rows=[]
    for name,counts,planet in cases:
        for order in ('current','overflow','delayed'):
            for flat in (False,True) if order=='delayed' else (False,):
                row=dict(case=name,force=counts,planet=planet,order=order,flat=flat,**solve(force(**counts),planet,order,flat))
                rows.append(row)
                print(name,order,flat,*(round(row[k],2) for k in ('ship_cost_lost','batteries_lost','dock_cost_lost','production_lost')),flush=True)
    Path(__file__).with_name('nc_assault_collateral.json').write_text(json.dumps(dict(method='Exact enumeration in floating point; independent d10 rolls; current bombardment modifiers, fleet DS costs, free critical and three-round gate. No ROE inside bombardment, as today. New proposed collateral order or flat support only where labelled. No engine validation.',rows=rows),indent=2)+'\n')


if __name__=='__main__':
    run()
