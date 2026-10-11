"""t068 repeated-contact RT stress model, not a generated-map campaign.

Reuse fleet allocation and approved RT cohorts. Four scheduled encounters
with fresh enemies; no reinforcement or economic production. New orders
between encounters allow another fight. Catch=none/certain are immediate
pursuit controls, not movement simulations. Ships destroyed never recover.
"""
import argparse
import json
from pathlib import Path
import random
from collections import Counter

import recovery as rt
from fleet_model import Model, force
from rules import CLASSES, UNITS, RULESETS, variant

SCHEDULES = {
    'uniform2': dict.fromkeys(CLASSES, 2),
    'uniform1': dict.fromkeys(CLASSES, 1),
    'dd1-only': dict(dict.fromkeys(CLASSES, 2), DD=1),
    'class123-sb2': dict(dict.fromkeys(CLASSES, 2), DD=1, BB=3),
    'class123-sb3': dict(dict.fromkeys(CLASSES, 2), DD=1, BB=3, SB=3),
}
SCENARIOS = {
    'screen': (dict(DD=12, CA=6, BB=2), dict(CA=12), (6,10)),
    'capital': (dict(BB=6), dict(CA=15), (6,10)),
    'base': (dict(SB=2, CA=4), dict(CA=10), (10,10)),
}


def fleet(cohorts):
    return (tuple(sum(c.values()) for c in cohorts),
            tuple(sum(n for k,n in c.items() if k[0]) for c in cohorts), (0,0))


def cohorts_for(counts):
    return [rt.force(counts.get(c,0)) for c in CLASSES]


class Tracked(Model):
    """Observe allocation stages; reuse the existing hit allocation unchanged."""
    def __init__(self, values, pursuit='none'):
        rules = variant(RULESETS['engine-t060'], 'decision2-rt', roe='after1',
                        withdrawal=pursuit, unarmed_escape=True,
                        mutual_withdrawal_exchange=False)
        super().__init__(rules, UNITS)
        self.values = values
        self.active = None

    def _record(self, before, n, c, suppress=False):
        nb, cb = before
        for i, name in enumerate(CLASSES):
            if nb[i] == n[i] and cb[i] == c[i]:
                continue
            cohort = self.active[i]
            if suppress:
                fresh = (nb[i]-cb[i]) - (n[i]-c[i])
                cohort = rt.cripple(cohort, fresh, self.values[name])
                cohort = rt.finish_crippled(cohort, nb[i]-n[i])
            else:
                cohort = rt.finish_crippled(cohort, cb[i]-c[i])
                cohort[rt.NOMINAL] -= (nb[i]-cb[i]) - (n[i]-c[i])
                cohort = +cohort
            self.active[i] = cohort
            assert sum(cohort.values()) == n[i]
            assert sum(v for k,v in cohort.items() if k[0]) == c[i]

    def _suppress(self, n, c, hits, order_fn):
        before = n[:], c[:]
        left = super()._suppress(n,c,hits,order_fn)
        self._record(before,n,c,True)
        return left

    def _execute(self, n,c,hits,order_fn):
        before = n[:], c[:]
        left = super()._execute(n,c,hits,order_fn)
        self._record(before,n,c)
        return left

    def _free_kill(self,n,c):
        before = n[:], c[:]
        super()._free_kill(n,c)
        self._record(before,n,c)

    def fight(self, own, enemy, rng, roe):
        a,b = fleet(own), fleet(enemy)
        r = 1
        def exchange(a,b,pressed=False,expose_a=False,expose_b=False):
            sa = self.volley(a,self.side_as(b),rng.randrange(10),pressed)
            sb = self.volley(b,self.side_as(a),rng.randrange(10),pressed)
            self.active = enemy
            b2 = self.apply(b,*sa,exposed=expose_b)[0][1]
            self.active = own
            a2 = self.apply(a,*sb,exposed=expose_a)[0][1]
            return [(1.,(a2,b2))]
        while True:
            branches = self.step(a,b,r,roe,exchange)
            assert len(branches)==1 and branches[0][0]==1
            _, _, nxt = branches[0]
            if nxt[0]=='terminal':
                return nxt[1:3]
            _,a,b,r = nxt


def run_case(name, schedule, gap, assisted, pursuit, trials, seed):
    start, enemy_counts, roe = SCENARIOS[name]
    model = Tracked(SCHEDULES[schedule], pursuit)
    # Gap in maintenance-pass labels: N+1 fights BEFORE N+1 recovery.
    dates = [i*gap for i in range(4)]
    total = Counter()
    entry_by_wave = [Counter() for _ in dates]
    for trial in range(trials):
        own = cohorts_for(start)
        # Same encounter-specific seeds across RT schedules, even when the
        # previous encounter took a different number of exchanges.
        wave = 0
        for year in range(dates[-1]+1):
            own = [rt.begin_year(c, at_base=assisted and year not in dates and cls!='SB') for cls,c in zip(CLASSES,own)]
            while wave < len(dates) and dates[wave]==year:
                entry = fleet(own)
                if any(entry[0]):
                    e = entry_by_wave[wave]
                    e['entries'] += 1
                    e['damaged'] += int(any(entry[1]))
                    e['as'] += model.side_as(entry)
                    for i,cls in enumerate(CLASSES):
                        e[cls+'_live'] += entry[0][i]
                        e[cls+'_damaged'] += entry[1][i]
                    opposing = cohorts_for(enemy_counts)
                    # A stationary base scenario always fights; mobile cases
                    # represent imposed contacts, including compulsory round 1.
                    model.fight(own, opposing, random.Random(seed+trial*100+wave), roe)
                wave += 1
            own = [rt.end_year(c)[0] for c in own]
        final = fleet(own)
        total['destroyed'] += int(not any(final[0]))
        total['loss_cost'] += sum((start.get(cls,0)-final[0][i])*UNITS[cls]['cost'] for i,cls in enumerate(CLASSES))
        for i,cls in enumerate(CLASSES):
            total[cls+'_lost'] += start.get(cls,0)-final[0][i]
    entries=[]
    for wave,e in enumerate(entry_by_wave):
        n=e['entries']
        entries.append(dict(wave=wave,year=dates[wave],alive_probability=n/trials,
                            damaged_probability_given_alive=e['damaged']/n if n else None,
                            mean_as_given_alive=e['as']/n if n else None,
                            class_damaged_fraction={cls:e[cls+'_damaged']/e[cls+'_live'] if e[cls+'_live'] else None for cls in CLASSES}))
    return dict(scenario=name,schedule=schedule,gap=gap,assisted=assisted,pursuit=pursuit,trials=trials,entries=entries,
                destroyed_probability=total['destroyed']/trials,mean_destroyed_cost=total['loss_cost']/trials,
                mean_hulls_destroyed={cls:total[cls+'_lost']/trials for cls in CLASSES})


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--trials',type=int,default=5000)
    args=parser.parse_args()
    rows=[]
    for name in SCENARIOS:
        schedules = ('uniform2','class123-sb2','class123-sb3') if name=='base' else ('uniform2','class123-sb2')
        for gap in (0,1,2,3,4):
            for assisted in (False,True) if name!='base' else (False,):
                for schedule in schedules:
                    row=run_case(name,schedule,gap,assisted,'none',args.trials,680000)
                    rows.append(row)
                    print(name,schedule,gap,assisted,round(row['mean_destroyed_cost'],2),flush=True)
    # Screen sensitivity to an immediate successful catch; no change in screen.
    for gap in (1,2,3):
        for schedule in ('uniform2','class123-sb2'):
            rows.append(run_case('screen',schedule,gap,False,'pursuit',args.trials,680000))
    for gap in (1,2,3):
        for pursuit in ('none','pursuit'):
            rows.append(run_case('screen','dd1-only',gap,False,pursuit,args.trials,680000))
    for name in ('screen','capital'):
        for gap in (2,3):
            rows.append(run_case(name,'uniform1',gap,False,'none',args.trials,680000))
    data=dict(seed=680000,method='Seeded repeated-contact cohort simulation. Four imposed encounters, no replacements, fixed fresh enemy each time. Assisted means a protected full quiet pass at a separate friendly base, never the combat pass; travel feasibility is not modelled. Cohort allocation reuses fleet_model. Not movement, map generation or an economic campaign.',
              schedules=SCHEDULES,scenarios=SCENARIOS,rows=rows)
    Path(__file__).with_name('nc_rt_campaign.json').write_text(json.dumps(data,indent=2)+'\n')


if __name__=='__main__':
    main()
