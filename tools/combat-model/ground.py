"""Exact defender-first volley experiment, independent of the NC engine.

Each army: AS10, DS10. Roll one shared CER per side each round.
Defender damage first; surviving attackers reply. Partial damage carries
against each side's next army throughout this battle, without rerolling.
"""
from functools import lru_cache
from collections import Counter
import argparse
import json
from pathlib import Path
import nc_combat_analysis as fleet

ROWS=(1,2,2,3,3,3,3,4,4,5)
DIST=tuple((k,v/10) for k,v in Counter(ROWS).items())

@lru_cache(None)
def solve(a,d,damage_a=0,damage_d=0):
    if a==0: return 0.,0.
    if d==0: return 1.,0.
    win=0.; rounds=1.
    for kd,pd in DIST:
        incoming=damage_a+d*kd
        na=max(0,a-incoming//10)
        if not na: continue
        ra=incoming%10
        for ka,pa in DIST:
            outgoing=damage_d+na*ka
            nd=max(0,d-outgoing//10)
            rd=outgoing%10
            w,r=solve(na,nd,ra,rd)
            win+=pd*pa*w
            rounds+=pd*pa*r
    return min(1.,max(0.,win)),rounds

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',type=Path,help='Write JSON here; otherwise print to stdout.')
    args=parser.parse_args()
    records=[]
    for a,d in ((1,1),(2,2),(3,3),(5,5),(8,10),(10,10),(11,10),
                (12,10),(15,10),(20,10),(30,10)):
        w,r=solve(a,d)
        current=fleet.ground(a,d)
        simultaneous=fleet.binomial_ground(a,d)
        records.append(dict(attackers=a,defenders=d,
                            current=dict(capture=current[0],rounds=current[1]),
                            simultaneous=dict(capture=simultaneous[0],rounds=simultaneous[1]),
                            defender_first=dict(capture=w,rounds=r)))
    data={'method':'Exact finite-state enumeration using floating-point probabilities; no sampling.',
                         'defender_first_cer_tenths':ROWS,'armies':dict(AS=10,DS=10),
                         'engine_validated':False,'battles':records}
    payload=json.dumps(data,indent=2)+'\n'
    if args.output is None:
        print(payload,end='')
    else:
        args.output.write_text(payload)
        print(f'Wrote {args.output}')
