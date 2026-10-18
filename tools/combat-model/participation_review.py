"""t073 joined-force and bounded-surprise controls on approved fleet rules.

Conditional battles, not a tick-loop implementation or a Rust test run.
Sequential contacts preserve crippled steps and reset carry; contact
eligibility is imposed, not derived from movement or orders. Joining does not join planetary mission execution.
"""
import argparse
import json
from pathlib import Path
import random
from collections import Counter

from fleet_model import Model,force,roe_satisfied
from rules import RULESETS,load_units,variant
from confidence import wilson_interval

# t206: the engine's current fleet rules and hulls; the t073 run used
# engine-t060 with DS 8 destroyers.
RULES=variant(RULESETS['engine-t206'],'decisions1-3-t206',withdrawal='none')
UNITS=load_units('t200')
MODEL=Model(RULES,UNITS)


def plus(a,b):
    return tuple(tuple(x+y for x,y in zip(a[k],b[k])) for k in range(2))+((0,0),)


def bonus_shot(model,side,enemy,roll,bonus):
    n,c,_=side
    shift=int(bool(model.rules['mixed_shift']) and sum(n[i]>0 for i in model.mixed)>=2)+int(n[model.sb]>0)+int(model.heavy_line(n,enemy[0]))
    extra=min(model.rules['shift_cap'],shift+bonus)-shift
    sup,exe,_=model.volley(side,model.side_as(enemy),min(9,roll+extra),enemy=enemy)
    return sup,exe,bool(model.side_as(side) and roll==9)


def fight(a,b,rng,surprise=False,roe=(10,10)):
    r=1
    exchanges=0
    def exchange(a,b,**kw):
        nonlocal exchanges
        ra,rb=rng.randrange(10),rng.randrange(10)
        sa=bonus_shot(MODEL,a,b,ra,int(surprise and exchanges==0))
        sb=MODEL.volley(b,MODEL.side_as(a),rb,enemy=a)
        exchanges+=1
        a2=MODEL.apply(a,*sb)[0][1]
        b2=MODEL.apply(b,*sa)[0][1]
        return [(1.,(a2,b2))]
    while True:
        _,_,nxt=MODEL.step(a,b,r,roe,exchange)[0]
        if nxt[0]=='terminal':
            return nxt[1],nxt[2]
        _,a,b,r=nxt


def willing(strengths,roes,enemy,assembled):
    selected=list(range(len(strengths)))
    while True:
        own=sum(strengths[i] for i in selected)
        after=[i for i in selected if strengths[i]>0 and roe_satisfied(roes[i],own if assembled else strengths[i],enemy)]
        if after==selected:
            return after
        selected=after


def withdrawals(strengths, roes, bases=(0, 0)):
    """Both sides prune simultaneously; departed fleets cannot return."""
    selected = [list(range(len(side))) for side in strengths]
    history = []
    while True:
        power = [bases[k] + sum(strengths[k][i] for i in selected[k]) for k in (0, 1)]
        after = [[i for i in selected[k] if strengths[k][i] > 0
                  and roe_satisfied(roes[k][i], power[k], power[1-k])]
                 for k in (0, 1)]
        history.append(dict(before=selected, power=power, after=after))
        if after == selected:
            return dict(remaining=after, iterations=history)
        selected = after


def transport_losses(loaded, empty, loss, rng):
    """One already-selected class/state/RT cohort, loaded-first across fleets."""
    loaded_loss = min(loss, sum(loaded))
    return dict(loaded=apportion(loaded, loaded_loss, rng),
                empty=apportion(empty, loss-loaded_loss, rng))


def apportion(counts, loss, rng):
    """Largest remainder; sample a cutoff tie from the battle's seeded RNG.

    counts uses the roster fixed for this battle. No draw for untied awards.
    """
    total = sum(counts)
    assert 0 <= loss <= total
    if not total:
        return [0] * len(counts)
    result = [loss * n // total for n in counts]
    left = loss - sum(result)
    groups = {}
    for i, n in enumerate(counts):
        groups.setdefault(loss * n % total, []).append(i)
    for remainder in sorted(groups, reverse=True):
        if not left:
            break
        tied = groups[remainder]
        chosen = tied if len(tied) <= left else rng.sample(tied, left)
        for i in chosen:
            result[i] += 1
        left -= len(chosen)
    return result


def sample(trials):
    cases=[('split-CA-even',dict(CA=5),dict(CA=10)),
           ('split-CA-advantage',dict(CA=6),dict(CA=10)),
           ('assault-fixture-two-4BB',dict(BB=4),dict(SB=1)),
           ('split-DD-CA',dict(DD=5,CA=5),dict(CA=12))]
    rows=[]
    for name,half,opponent in cases:
        for joined in (False,True):
            totals=Counter()
            for seed in range(trials):
                rng=random.Random(730000+seed)
                part=force(**half); enemy=force(**opponent)
                initial=plus(part,part)
                if joined:
                    remaining,enemy=fight(initial,enemy,rng)
                else:
                    first,enemy=fight(part,enemy,rng)
                    enemy=enemy[:2]+((0,0),)
                    second=part
                    if any(enemy[0]):
                        second,enemy=fight(part,enemy,rng)
                    remaining=plus(first,second)
                totals['win']+=not any(enemy[0]) and any(remaining[0])
                totals['loss']+=MODEL.cost_of(initial[0])-MODEL.cost_of(remaining[0])
            rows.append(dict(case=name,joined=joined,trials=trials,capture_field=totals['win']/trials,win_ci95=wilson_interval(totals['win'],trials),loss_cost=totals['loss']/trials))
    surprise=[]
    for counts_a,counts_b in [(dict(CA=10),dict(CA=10)),(dict(CA=8),dict(CA=10)),(dict(CA=11),dict(CA=10)),(dict(CA=5,DD=5),dict(CA=7))]:
        for enabled in (False,True):
            wins=0
            for seed in range(trials):
                a,b=fight(force(**counts_a),force(**counts_b),random.Random(740000+seed),enabled)
                wins+=not any(b[0]) and any(a[0])
            surprise.append(dict(a=counts_a,b=counts_b,first_exchange_bonus=enabled,trials=trials,win=wins/trials,win_ci95=wilson_interval(wins,trials)))
    eligibility=[dict(strengths=s,roes=r,enemy=e,own_AS=willing(s,r,e,False),assembled_AS=willing(s,r,e,True)) for s,r,e in [([180,180],[6,6],300),([180,180],[6,8],300),([300,30],[6,0],300),([180,180],[6,6],390)]]
    return dict(method='Seeded battle controls, Python RNG; t200 hulls under the engine-t206 fleet rules; no tick simulation or engine execution; all-ROE10 attrition rows, no catch, surprise only exchange1 and combined die cap2, natural9 unchanged. Wilson 95% intervals. Sequential contacts reset carry and preserve completed damage; no recovery between contacts.',
                units=UNITS,
                rules=RULES,rows=rows,surprise=surprise,eligibility=eligibility,
                withdrawal_controls=[withdrawals(([300],[300]),([5],[5])),
                                     withdrawals(([180,180],[300]),([6,10],[10])),
                                     withdrawals(([300,0],[300]),([6,10],[6]))],
                allocation=dict(tie_rule='Uniform sample without replacement at the cutoff remainder, battle seeded RNG, fixed battle roster; not renumbering-invariant per seed.', eight_two_three=apportion([8,2],3,random.Random(750000)),
                                split_cargo=transport_losses([6,0],[0,4],4,random.Random(750000))))


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--trials',type=int,default=20000)
    args=parser.parse_args()
    data=sample(args.trials)
    Path(__file__).with_name('nc_participation_review.json').write_text(json.dumps(data,indent=2)+'\n')
    print(json.dumps(data,indent=2))
