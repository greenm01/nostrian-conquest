"""t075: DD durability with approved withdrawal and RT2 recovery.

Fixed contacts, not campaign win odds. No-catch and certain immediate-catch
controls bound the extra exchange, not the chance of contact or escape.
One aggregated fleet per side; decision 6 is not assumed. No new rules.
"""
import argparse
from collections import Counter
import copy
import json
from pathlib import Path
import random

from confidence import wilson_interval
from fleet_model import Model, force, P_A, P_B, P_MUTUAL, P_STALEMATE, LOSS_A, LOSS_B, ROUNDS, LOST_A, VIABLE_A
from rules import CLASSES, RULESETS, UNITS, variant
import recovery as rt
from rt_campaign import Tracked, SCHEDULES, cohorts_for, fleet

MATRIX = [
    ('DD-CA', dict(DD=30), dict(CA=10)),
    ('DD-CA-large', dict(DD=90), dict(CA=30)),
    ('DD-BB', dict(DD=90), dict(BB=10)),
    ('CA-BB', dict(CA=30), dict(BB=10)),
    ('DD-CA-mix', dict(DD=15, CA=5), dict(CA=10)),
    ('three-class', dict(DD=6, CA=3, BB=1), dict(CA=8)),
    ('DD-base', dict(DD=10), dict(SB=1)),
    ('mixed-base', dict(DD=1, CA=3), dict(SB=1)),
    ('CA-parity', dict(CA=10), dict(CA=10)),
]
ESCORTS = [('DD', dict(DD=15)), ('CA', dict(CA=5)), ('mixed', dict(DD=6, CA=3))]


def units_for(ds):
    units = copy.deepcopy(UNITS)
    units['DD']['DS'] = ds
    return units


def rules_for(catch='none', mixed=1):
    return variant(RULESETS['engine-t060'], 'decision2-control', roe='after1',
                   withdrawal=catch, unarmed_escape=True, screen='absolute',
                   mixed_shift=mixed, mutual_withdrawal_exchange=False)


def battle(name, left, right, ds, roe, catch, mixed, trials, seed):
    model = Model(rules_for(catch, mixed), units_for(ds))
    a, b = force(**left), force(**right)
    sums = None
    for i in range(trials):
        v = model.simulate(a, b, random.Random(seed+i), roe)
        if sums is None:
            sums = [0.0]*len(v)
        for j, x in enumerate(v):
            sums[j] += x
    mean = [x/trials for x in sums]
    tt = CLASSES.index('TT')
    return dict(case=name, left=left, right=right, dd_ds=ds, roe=roe,
                catch=catch, mixed_shift=mixed, trials=trials,
                left_win=mean[P_A], left_win_ci95=wilson_interval(round(sums[P_A]),trials),
                right_win=mean[P_B], mutual=mean[P_MUTUAL], unresolved=mean[P_STALEMATE],
                left_lost_cost=mean[LOSS_A], right_lost_cost=mean[LOSS_B],
                exchanges=mean[ROUNDS],
                transports_surviving=left.get('TT',0)-mean[LOST_A][tt],
                half_transports_survive=mean[VIABLE_A] if left.get('TT',0) else None)


class HullTracked(Tracked):
    def __init__(self, ds, catch):
        super().__init__(SCHEDULES['uniform2'], catch)
        Model.__init__(self, rules_for(catch), units_for(ds))


def campaign(name, start, enemy, ds, gap, catch, trials, seed):
    model = HullTracked(ds, catch)
    dates = [i*gap for i in range(4)]
    totals = Counter()
    entries = [Counter() for _ in dates]
    for trial in range(trials):
        own = cohorts_for(start)
        wave = 0
        for year in range(dates[-1]+1):
            own = [rt.begin_year(c) for c in own]
            while wave < 4 and dates[wave] == year:
                before = fleet(own)
                if any(before[0]):
                    e = entries[wave]
                    e['alive'] += 1
                    e['damaged'] += int(any(before[1]))
                    e['dd_live'] += before[0][0]
                    e['dd_damaged'] += before[1][0]
                    model.fight(own, cohorts_for(enemy), random.Random(seed+trial*100+wave), (6,10))
                wave += 1
            own = [rt.end_year(c)[0] for c in own]
        final = fleet(own)
        totals['wipe'] += not any(final[0])
        totals['loss'] += model.cost_of(force(**start)[0])-model.cost_of(final[0])
        totals['DD_lost'] += start.get('DD',0)-final[0][0]
    return dict(case=name, start=start, enemy=enemy, dd_ds=ds, gap=gap, catch=catch,
                roe=[6,10], rt=2, assisted=False, trials=trials,
                wipe=totals['wipe']/trials, wipe_ci95=wilson_interval(totals['wipe'],trials),
                loss=totals['loss']/trials, dd_lost=totals['DD_lost']/trials,
                entries=[dict(year=dates[i], alive=e['alive']/trials,
                              damaged_given_alive=e['damaged']/e['alive'] if e['alive'] else None,
                              dd_damaged_fraction=e['dd_damaged']/e['dd_live'] if e['dd_live'] else None)
                         for i,e in enumerate(entries)])


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--trials',type=int,default=20000)
    parser.add_argument('--campaign-trials',type=int,default=5000)
    parser.add_argument('--seed',type=int,default=750000)
    args=parser.parse_args()
    rows=[]
    output=dict(seed=args.seed, trials=args.trials, campaign_trials=args.campaign_trials, units=UNITS,
                rules=rules_for(), method='Seeded sampling, Wilson 95% intervals for binary outcomes. Shared trial seeds across controls. Single imposed contact or four fresh-enemy contacts without replacements. No movement, pursuit geometry, willingness frequency, production, arrival or landing simulation. Transport survival is not delivery.',
                battles=rows, campaigns=[])
    path=Path(__file__).with_name('nc_hull_review.json')
    def save():
        path.write_text(json.dumps(output,indent=2)+'\n')
    def add(case,ds,roe,catch='none',mixed=1):
        row=battle(*case,ds,roe,catch,mixed,args.trials,args.seed)
        rows.append(row)
        print(case[0],ds,roe,catch,mixed,round(row['left_win']*100,2),round(row['transports_surviving'],2),flush=True)
    for ds in (5,6,8,10):
        for case in MATRIX:
            add(case,ds,(10,10))
    for ds in (5,8):
        for case in MATRIX:
            for catch in ('none','pursuit'):
                add(case,ds,(6,10),catch)
        for case in MATRIX[4:6]:
            add(case,ds,(10,10),mixed=0)
        for name,escort in ESCORTS:
            for catch in ('none','pursuit'):
                add(('convoy-'+name,dict(escort,TT=10),dict(CA=10)),ds,(6,10),catch)
        for n in (6,15,20,30):
            add((f'{n}DD-base',dict(DD=n),dict(SB=1)),ds,(10,10))
        for n in (10,20):
            add((f'{n}DD-guarded-base',dict(DD=n),dict(SB=1,CA=3)),ds,(10,10))
        save()
    for ds in (5,8):
        for name,start,enemy in MATRIX[:1]+[MATRIX[4],MATRIX[5]]:
            for gap in (0,1,2,3):
                for catch in ('none','pursuit'):
                    row=campaign(name,start,enemy,ds,gap,catch,args.campaign_trials,args.seed)
                    output['campaigns'].append(row)
                    print('RT2',name,ds,gap,catch,round(row['loss'],2),round(row['wipe']*100,2),flush=True)
            save()
    save()


if __name__=='__main__':
    main()
