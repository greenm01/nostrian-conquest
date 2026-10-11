"""t068: conditional capture force and casualty price, not orbital gate odds.

All TT initially loaded and nominal. Softening cases have batteries already
cleared and stated nominal escorts. Blitz retains existing planetary CRT.
Costs exclude the earlier cost of winning orbit/removing batteries.
"""
from collections import Counter
from functools import lru_cache
import json
import math
from pathlib import Path

from ground_calibration import Shared, blitz_landing, blitz_outcomes
from rules import CRT_X4, EOTS_X4
from fleet_model import base_column

CANDIDATE=Shared(EOTS_X4,4,first=True)


@lru_cache(None)
def current(a,d):
    """Capture, mean surviving attackers, mean surviving defenders."""
    if not a or not d:
        return float(a>0),float(a),float(d)
    result=[0.,0.,0.]
    for ra in range(10):
        for rd in range(10):
            ka=(a*CRT_X4[ra][base_column(a,d)]+3)//4+(ra==9)
            kd=(d*CRT_X4[rd][min(4,base_column(d,a)+1)]+3)//4+(rd==9)
            na,nd=max(0,a-kd),max(0,d-ka)
            if not na and not nd:
                out=(1.,1.,0.) if a>d else (0.,0.,1.)
            else:
                out=current(na,nd)
            result=[x+y/100 for x,y in zip(result,out)]
    return tuple(result)


def ground(a,d,candidate):
    if not candidate:
        return current(a,d)
    out=CANDIDATE.solve(a,d)
    return out[0],out[4],out[5]


def softened(d,ca,flat=False):
    result=Counter()
    strength=30*ca
    for roll in range(10):
        multiplier=EOTS_X4[roll] if flat else CRT_X4[roll][base_column(strength,10*d)]
        hits=(strength*multiplier+3)//4 + (roll==9 and strength>0 and not flat)
        loss=min(d//2,hits//10)
        result[d-loss]+=.1
    return result


def attempt(a,d,candidate,mission,ca=0,batteries=0,flat_soften=False):
    lands=blitz_landing(a,ca,batteries) if mission=='blitz' else {a:1.}
    defenders=softened(d,ca,flat_soften) if mission=='invade' else {d:1.}
    escort_losses=prize=0.
    if mission=='blitz':
        for (n, ca_left, b_left), p in blitz_outcomes(a,ca,batteries).items():
            escort_losses += p*(ca-ca_left)
            prize += p*b_left*ground(n,d,candidate)[0]
    capture=survivors=defender_survivors=0.
    for landed,p in lands.items():
        for remaining,q in defenders.items():
            w,sa,sd=ground(landed,remaining,candidate)
            capture+=p*q*w
            survivors+=p*q*sa
            defender_survivors+=p*q*sd
    tt_lost=a-sum(n*p for n,p in lands.items())
    return dict(capture=capture,army_losses=a-survivors,tt_losses=tt_lost,
                casualty_cost=2*(a-survivors)+5*tt_lost+15*escort_losses,
                ca_losses=escort_losses,expected_captured_batteries=prize,
                force_cost=7*a+15*ca,army_tt_cost=7*a,escort_cost=15*ca,
                defender_losses=d-defender_survivors)


def run():
    rows=[]
    for candidate in (False,True):
        for mission,ca,batteries in [('landed',0,0),('invade',1,0),('invade',3,0),('invade',6,0),('blitz',0,0),('blitz',3,3),('blitz',6,3)]:
            for d in (10,11,12):
                for a in range(1,81):
                    row=attempt(a,d,candidate,mission,ca,batteries)
                    if row['capture']>=.90:
                        rows.append(dict(rule='candidate' if candidate else 'current',mission=mission,ca=ca,batteries=batteries,defending_armies=d,attacking_armies=a,**row))
                        break
    comparisons=[dict(armies=a,defenders=10,ca=ca,flat_soften=flat,**attempt(a,10,True,'invade',ca,flat_soften=flat)) for a in (10,11,15,20) for ca in (1,3,6) for flat in (False,True)]
    travel=[dict(map_distance=d,speed=5,ideal_ticks=math.ceil(52*(d+2)/5),maintenance_passes=math.ceil((d+2)/5)) for d in (3,8,13)]
    data=dict(method='Exact ground and cover/landing rolls; nominal escorts, one army per TT. No sampling. Costs are build points, not a simulated treasury. Single-attempt confidence, not expected retries against a changing garrison.',
              limits='Conditional on orbital gate won. Invade cases also assume batteries already cleared and escorts nominal at softening. Blitz cost includes armies, TT and CA lost during landing fire; earlier gate losses excluded. Same force as defender state changes is not a campaign.',
              costs=dict(army=2,TT=5,CA=15,ground_battery=20),threshold=.9,rows=rows,softening_controls=comparisons,travel=travel)
    Path(__file__).with_name('nc_invasion_price.json').write_text(json.dumps(data,indent=2)+'\n')
    for r in rows:
        if r['defending_armies']==10:
            print(r['rule'],r['mission'],r['ca'],r['batteries'],'A',r['attacking_armies'],'odds',round(100*r['capture'],2),'commit',r['force_cost'],'loss',round(r['casualty_cost'],2),'army',round(r['army_losses'],2),'TT',round(r['tt_losses'],2))


if __name__=='__main__':
    run()
