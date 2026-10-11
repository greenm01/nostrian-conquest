"""t087 staged raid controls: orbital battle followed by bombardment.

Fixed contact, fresh nominal fleets, no surprise; ROE10 on both sides.
Approved fleet rules/DD8, carried crippled steps, battery execution fire
on unchanged support CRT, delayed battery shield and third-round breakthrough.
No movement/detection/return-trip simulation; travel priced separately.
"""
import argparse
import json
import math
from pathlib import Path
import random
import sys

sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'combat-model'))
from fleet_model import Model, force
from rules import RULESETS, variant, load_units
from assault_collateral import bombard_as, shot
from confidence import wilson_interval

UNITS=load_units('dd-10-8-5')
RULE=variant(RULESETS['engine-t060'],'approved-raid-control',roe='after1',
             withdrawal='none',mutual_withdrawal_exchange=False,unarmed_escape=True)
FLEET=Model(RULE,UNITS)
BATTERY=Model(variant(RULE,'planetary-discard',remainder='discard'),UNITS)


def gate(rng,a,b):
    exchange=lambda a,b,**kw:FLEET.exchange_sampled(rng,a,b,**kw)
    r=1
    while True:
        branches=FLEET.step(a,b,r,(10,10),exchange)
        assert len(branches)==1
        _,_,nxt=branches[0]
        if nxt[0]=='terminal':
            _,a,b,wa,wb=nxt
            return a,b,not wa and any(a[0]) and (not any(b[0]) or wb)
        _,a,b,r=nxt


def planet_hit(world,hits,r,goods_rate):
    b,dock,a,g,p=world
    if b:
        return (max(0,b-hits//20),dock,a,g,p)
    out=[]
    for cls,count in dock:
        killed=min(count,hits//UNITS[cls]['DS'])
        out.append((cls,count-killed))
        hits-=killed*UNITS[cls]['DS']
    if r==3:
        killed=min(a,hits//10)
        a-=killed
        hits-=10*killed
        units=hits//10
        g-=min(max(0,g),goods_rate*units)
        p=max(0,p-units)
    return b,tuple(out),a,g,p


def bombard(rng,side,world,goods_rate):
    # A new fire phase has no orbital battle bank. Damage steps survive.
    side=(side[0],side[1],(0,0))
    for r in range(1,4):
        attack=bombard_as(side)
        if not attack:
            break
        defense=world[0]*90
        mixed=sum(side[0][i]>0 for i in (0,1,2))>=2
        hits,critical=shot(attack,max(1,defense),rng.randrange(10),mixed)
        if defense:
            back,crit=shot(defense,max(1,attack),rng.randrange(10))
            branches=BATTERY.apply(side,0,back,crit)
            assert len(branches)==1
            side=branches[0][1]
        world=planet_hit(world,hits+critical,r,goods_rate)
    return side,world


def trial(rng,attack,guard,world,goods_rate):
    a0,b0=force(**attack),force(**guard)
    a,b,passed=gate(rng,a0,b0)
    before_battery_cost=FLEET.cost_of(a[0])
    after=world
    if passed:
        a,after=bombard(rng,a,world,goods_rate)
    dock_cost=lambda w:sum(UNITS[c]['cost']*n for c,n in w[1])
    return dict(gate_pass=float(passed),attacker_destroyed=FLEET.cost_of(a0[0])-FLEET.cost_of(a[0]),
        orbital_attacker_destroyed=FLEET.cost_of(a0[0])-before_battery_cost,
        battery_attacker_destroyed=before_battery_cost-FLEET.cost_of(a[0]),
        attacker_crippled=FLEET.cost_of(a[1]),attacker_wiped=float(not any(a[0])),
        guard_destroyed=FLEET.cost_of(b0[0])-FLEET.cost_of(b[0]),
        batteries_destroyed=world[0]-after[0],dock_destroyed=dock_cost(world)-dock_cost(after),
        armies_destroyed=world[2]-after[2],goods_destroyed=world[3]-after[3],
        production_destroyed=world[4]-after[4],industry_hit=float(after[4]<world[4]))


def run_case(name,attack,guard,world,rate,trials,seed):
    sums,squares={},{}
    for i in range(trials):
        row=trial(random.Random(seed+i),attack,guard,world,rate)
        for k,v in row.items():
            sums[k]=sums.get(k,0)+v
            squares[k]=squares.get(k,0)+v*v
    means={k:v/trials for k,v in sums.items()}
    intervals={}
    for k,mean in means.items():
        if k in ('gate_pass','attacker_wiped','industry_hit'):
            intervals[k]=wilson_interval(round(sums[k]),trials)
        else:
            se=math.sqrt(max(0,(squares[k]-trials*mean*mean)/(trials-1))/trials)
            intervals[k]=[mean-1.96*se,mean+1.96*se]
    return dict(case=name,attack=attack,guard=guard,world=world,goods_per_ten_hits=rate,
                committed=FLEET.cost_of(force(**attack)[0]),trials=trials,seed=seed,
                mean=means,ci95=intervals)


def repeat_open(attack,armies,rate,trials=10000,years=5):
    # A stationary Bombard order repeats annually. No returning defenders,
    # new builds or reinforcement: deliberately exposed-world control.
    totals={key:0. for key in ('goods_destroyed','final_goods','final_production','armies_left')}
    for i in range(trials):
        rng=random.Random(870001+i)
        side=force(**attack)
        world=(0,(),armies,500.,100.)
        destroyed=0.
        for year in range(years):
            if year:
                b,d,a,g,p=world
                p+=.125*(100-p)
                world=(b,d,a,g+.5*p,p)
            old=world[3]
            side,world=bombard(rng,side,world,rate)
            destroyed+=old-world[3]
        for key,value in zip(totals,(destroyed,world[3],world[4],world[2])):
            totals[key]+=value
    return dict(attack=attack,initial_armies=armies,goods_per_ten_hits=rate,
                trials=trials,years=years,mean={k:v/trials for k,v in totals.items()})


CASES=[
    ('DD-open',dict(DD=1),{},(0,(),0,500,100)),
    ('CA-open',dict(CA=1),{},(0,(),0,500,100)),
    ('BB-open',dict(BB=1),{},(0,(),0,500,100)),
    ('DD-army1',dict(DD=1),{},(0,(),1,500,100)),
    ('CA-army10',dict(CA=1),{},(0,(),10,500,100)),
    ('BB-army10',dict(BB=1),{},(0,(),10,500,100)),
    ('CA-battery1',dict(CA=1),{},(1,(),0,500,100)),
    ('BB-battery1',dict(BB=1),{},(1,(),0,500,100)),
    ('CA3-battery1',dict(CA=3),{},(1,(),10,500,100)),
    ('CA6-battery3',dict(CA=6),{},(3,(),10,500,100)),
    ('BB3-battery3',dict(BB=3),{},(3,(),10,500,100)),
    ('CA6-battery3-dockBB',dict(CA=6),{},(3,(('BB',1),),10,500,100)),
    ('CA-guardCA',dict(CA=1),dict(CA=1),(0,(),0,500,100)),
    ('CA3-guardCA2',dict(CA=3),dict(CA=2),(1,(),10,500,100)),
    ('CA6-base',dict(CA=6),dict(SB=1),(3,(),10,500,100)),
    ('BB3-base',dict(BB=3),dict(SB=1),(3,(),10,500,100)),
    ('CA6-guardCA3-base',dict(CA=6),dict(CA=3,SB=1),(3,(),10,500,100)),
    ('BB3-guardCA3-base',dict(BB=3),dict(CA=3,SB=1),(3,(),10,500,100)),
    ('CA-smallworld',dict(CA=1),{},(0,(),0,50,10)),
]


if __name__=='__main__':
    parser=argparse.ArgumentParser()
    parser.add_argument('--trials',type=int,default=10000)
    args=parser.parse_args()
    rows=[]
    for case in CASES:
        for rate in (100,25,10):
            row=run_case(*case,rate,args.trials,870001)
            rows.append(row)
            m=row['mean']
            print(case[0],rate,*(round(m[k],3) for k in ('gate_pass','attacker_destroyed','attacker_crippled','goods_destroyed','production_destroyed')),flush=True)
    Path(__file__).with_name('raid_review_results.json').write_text(json.dumps(dict(
        method=__doc__,rules=RULE,units=UNITS,rows=rows,
        repeated=[repeat_open(a,g,rate,args.trials) for a,g in
                  ((dict(DD=1),1),(dict(CA=1),10),(dict(BB=1),10))
                  for rate in (100,25,10)]),indent=2)+'\n')
