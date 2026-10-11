"""t083 exact rational controls for NC accounting and tax/growth candidates.

No engine, DOS, map, combat or policy-optimality claim. Fractions avoid
floating point noise; final JSON values are rounded presentation only.
"""
import json
from fractions import Fraction as F
from pathlib import Path

RULES = ('current', 'income_carry', 'smooth', 'smooth_min1', 'gated_min1')


def ceil(x):
    return -(-x.numerator // x.denominator)


def step(p, k, tax, rule, base=False):
    p = F(p)
    gap = max(F(0), k-p)
    growth = gap * (100-min(tax,95)) / 400
    bonus = F(1,2) if tax <= 50 else max(F(0), F(65-tax,30))
    penalty = p * max(0,tax-65) / 500
    if rule in ('current', 'income_carry'):
        growth = ceil(growth)
        if base:
            percent = 50 if tax <= 50 else max(0,(65-tax)*50//15)
            growth += ceil(F(growth*percent,100))
        growth = min(gap,max(1,growth))
        penalty = ceil(penalty)
    else:
        growth *= 1+bonus if base else 1
        if rule == 'smooth_min1' or (rule == 'gated_min1' and tax < 65):
            growth = max(1,growth)
        growth = min(gap,growth)
    p = F(min(k,max(F(0),p+growth-penalty)))
    income = p*tax/100
    if rule == 'current':
        income = F(income.numerator//income.denominator)
    return p,income


def whole_capacity(p, base=False):
    # Preserve the existing nearest-whole production capacity convention.
    return (5 if base else 1) * ((p + F(1,2)) // 1)


def trajectory(rule, k, taxes, start=1, base=False):
    p, goods = F(start),F(0)
    rows = []
    for tax in taxes:
        p, income = step(p,k,tax,rule,base)
        goods += income
        rows.append(dict(production=p,income=income,goods=goods,
                         order_budget=max(0,min(goods//1,whole_capacity(p,base)))))
    return rows


def equilibrium(k,tax,base=False):
    a = F(100-min(tax,95),400)
    bonus = F(1,2) if tax <= 50 else max(F(0),F(65-tax,30))
    if base:
        a *= 1+bonus
    b = F(max(0,tax-65),500)
    return F(k)*a/(a+b)


def experiment():
    result = dict(method='Exact rational arithmetic, no random sampling. Integer baseline at6420c7b8; candidate spendable goods floored, production capacity rounded nearest as current helpers, no purchases. No Real48 conformance claim.', rules=list(RULES), colony=[], mature=[], policies=[], emergency=[], tax_enumeration=[])
    for rule in RULES:
        for k in (10,20,50,100,150):
            for tax in (35,50,64,65,66,70,71):
                rows = trajectory(rule,k,[tax]*40)
                result['colony'].append(dict(rule=rule,k=k,tax=tax,
                    at={year:rows[year-1] for year in (1,5,10,20)},
                    first_order={cost:next((i+1 for i,r in enumerate(rows) if r['order_budget']>=cost),None) for cost in (5,15,45)},
                    reach90=next((i+1 for i,r in enumerate(rows) if r['production']>=F(9*k,10)),None)))
        for tax in (50,65,66,70,71,75,80,100):
            rows = trajectory(rule,100,[tax]*60,start=100)
            result['mature'].append(dict(rule=rule,tax=tax,at={year:rows[year-1] for year in (1,5,10,20,60)}))
        for policy,taxes in [('35',[35]*20),('50',[50]*20),('65',[65]*20),('70',[70]*20),('35x5_then65',[35]*5+[65]*15)]:
            home=trajectory(rule,100,taxes,start=100)
            colony=trajectory(rule,100,taxes)
            result['policies'].append(dict(rule=rule,policy=policy,one_home_four_colonies={year:home[year-1]['goods']+4*colony[year-1]['goods'] for year in (5,10,20)}))
        for high in (70,80,100):
            rows=trajectory(rule,100,[high]+[65]*19,start=100)
            result['emergency'].append(dict(rule=rule,tax_one_year=high,at={year:rows[year-1] for year in (1,2,5,10,20)}))
        # Enumerate the one-control choice at fixed horizons; this does not
        # search reactive policies, combat pressure or changing colony ages.
        for k,start in ((10,1),(100,1),(100,100)):
            rows=[trajectory(rule,k,[tax]*20,start=start) for tax in range(101)]
            result['tax_enumeration'].append(dict(rule=rule,k=k,start=start,
                best_constant_tax={year:[t for t in range(101) if rows[t][year-1]['goods']==max(r[year-1]['goods'] for r in rows)] for year in (1,5,10,20)}))
    result['smooth_equilibria']=[dict(tax=t,production=equilibrium(100,t),income=equilibrium(100,t)*t/100) for t in (65,66,70,71,75,80,100)]
    result['gate_boundary'] = [dict(k=k,start=p,tax=t,
        production=step(p,k,t,'gated_min1')[0],income=step(p,k,t,'gated_min1')[1])
        for k,p in ((10,1),(10,9),(20,1),(100,1),(100,95),(100,99),(100,100))
        for t in (64,65,66)]
    result['gate_tax_cycles'] = []
    for high in (70,75,80,100):
        # On a mature K100 world, the shock removes (high-65)/5 points.
        # Tax64 recovery with the gated floor restores one point per pass.
        recovery = ceil(F(high-65,5))
        taxes = [high]+[64]*recovery
        rows = trajectory('gated_min1',100,taxes,start=100)
        result['gate_tax_cycles'].append(dict(taxes=taxes,
            final_production=rows[-1]['production'],income=rows[-1]['goods'],
            mean_income=rows[-1]['goods']/len(taxes),constant65_income=65*len(taxes)))
    result['colony_head_start'] = []
    for k in (10,20,100):
        for start in (1,min(k,3)):
            rows = trajectory('smooth',k,[50]*40,start=start)
            result['colony_head_start'].append(dict(k=k,start=start,
                at10=rows[9],first_order={cost:next((i+1 for i,r in enumerate(rows)
                    if r['order_budget']>=cost),None) for cost in (5,15,45)}))
    result['head_start_policies'] = []
    for policy,taxes in [('65',[65]*20),('35x5_then65',[35]*5+[65]*15)]:
        home = trajectory('smooth',100,taxes,start=100)
        for start in (1,3):
            colony = trajectory('smooth',100,taxes,start=start)
            result['head_start_policies'].append(dict(policy=policy,colony_start=start,
                one_home_four_colonies={year:home[year-1]['goods']+4*colony[year-1]['goods']
                                       for year in (5,10,20)}))
    result['base_control']=[dict(rule=rule,tax=t,at10=trajectory(rule,100,[t]*10,base=True)[-1]) for rule in RULES for t in (50,60,65)]
    return result


def render(value):
    if isinstance(value,F):
        return round(float(value),8)
    raise TypeError(type(value).__name__)


if __name__ == '__main__':
    Path(__file__).with_name('accounting_review_results.json').write_text(
        json.dumps(experiment(),default=render,indent=2)+'\n')
