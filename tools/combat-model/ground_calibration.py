"""t068: exact ground candidates and conditional blitz landing odds.

Shared-roll candidates enumerate the acyclic army/remainder state graph.
Independent fire uses a sparse transition matrix for exactly 64 rounds.
No Monte Carlo, no homeworld bonus, and no free ground critical in candidates.
Run with: uv run --no-project --with scipy python -B tools/combat-model/ground_calibration.py
"""
from collections import Counter
from functools import lru_cache
import json
import math
from pathlib import Path

from rules import EOTS_X4, VBAM_X10, CRT_X4
from fleet_model import base_column
import nc_combat_analysis as legacy


class Shared:
    def __init__(self, rows, unit, first=False, shift=0, rounding='carry'):
        self.unit, self.first, self.rounding = unit, first, rounding
        self.attacks = tuple((k, n / 10) for k, n in Counter(rows).items())
        self.defends = tuple((k, n / 10) for k, n in Counter(rows[min(9, r + shift)] for r in range(10)).items())
        self.solve = lru_cache(None)(self._solve)

    def damage(self, n, k, bank):
        hits = n * k + bank
        if self.rounding == 'ceil':
            return (hits + self.unit - 1) // self.unit, 0
        return divmod(hits, self.unit)

    def _solve(self, a, d, bank_a=0, bank_d=0):
        # capture, expected rounds, mutual destruction, longest roll path
        if not a or not d:
            return float(a > 0), 0., float(a == d == 0), 0, float(a), float(d)
        win = mutual = survivors_a = survivors_d = 0.
        rounds, longest = 1., 0
        for kd, pd in self.defends:
            kills_a, ra = self.damage(d, kd, bank_a)
            na = max(0, a - kills_a)
            for ka, pa in self.attacks:
                kills_d, rd = self.damage(na if self.first else a, ka, bank_d)
                nd = max(0, d - kills_d)
                w, r, m, length, sa, sd = self.solve(na, nd, ra, rd)
                win += pd * pa * w
                rounds += pd * pa * r
                mutual += pd * pa * m
                longest = max(longest, length)
                survivors_a += pd * pa * sa
                survivors_d += pd * pa * sd
        return win, rounds, mutual, longest + 1, survivors_a, survivors_d


def independent(max_a, max_d, pa=.30, pd=.35, cap=64):
    """Finite-horizon exact enumeration, including no-hit rounds and cap mass."""
    import numpy as np
    from scipy.sparse import csr_matrix
    size = (max_a + 1) * (max_d + 1)
    index = lambda a, d: a * (max_d + 1) + d
    rows, cols, probs = [], [], []
    reward = np.zeros((size, 3))  # immediate capture, mutual, rounds
    alive = np.zeros(size)
    for a in range(1, max_a + 1):
        qa = [math.comb(a, k) * pa**k * (1-pa)**(a-k) for k in range(a+1)]
        for d in range(1, max_d + 1):
            qb = [math.comb(d, k) * pd**k * (1-pd)**(d-k) for k in range(d+1)]
            i = index(a, d)
            reward[i, 2] = alive[i] = 1
            for ka, p in enumerate(qa):
                for kd, q in enumerate(qb):
                    na, nd = max(0, a-kd), max(0, d-ka)
                    if na and nd:
                        rows.append(i); cols.append(index(na, nd)); probs.append(p*q)
                    else:
                        reward[i, 0] += p*q*(na > 0)
                        reward[i, 1] += p*q*(na == nd == 0)
    transition = csr_matrix((probs, (rows, cols)), shape=(size, size))
    values = np.zeros_like(reward)
    for _ in range(cap):
        values = reward + transition @ values
        alive = transition @ alive
    def result(a, d):
        if not a or not d:
            return dict(capture=float(a > 0), rounds=0., mutual=float(a == d == 0), cap=0.)
        w, m, r = values[index(a, d)]
        return dict(capture=float(w), rounds=float(r), mutual=float(m), cap=float(alive[index(a, d)]))
    return result


def blitz_outcomes(armies, cruisers=3, batteries=3):
    """Existing cover/landing CRT, nominal CA and one loaded TT per army.

Conditional on orbital gate already won. No prior escort damage. Loaded
transports take execution first (20 hits to sink); a natural 9 sinks one
more. Surviving escorts don't change how many loaded transports were hit.
    """
    outcomes = Counter()
    cover_as = 30 * cruisers
    for cover_roll in range(10):
        cover = (cover_as * CRT_X4[cover_roll][base_column(cover_as, max(1, 90*batteries))] + 3)//4 + (cover_roll == 9) if cruisers else 0
        left = max(0, batteries - cover//20)
        if not left:
            outcomes[armies, cruisers, 0] += .1
            continue
        for landing_roll in range(10):
            hits = (90*left * CRT_X4[landing_roll][base_column(90*left, max(1, cover_as))] + 3)//4
            tt_lost = min(armies, hits//20)
            ca_lost = min(cruisers, (hits-20*tt_lost)//60)
            if landing_roll == 9:
                if tt_lost < armies:
                    tt_lost += 1
                elif ca_lost < cruisers:
                    ca_lost += 1
            outcomes[armies-tt_lost, cruisers-ca_lost, left] += .01
    return dict(sorted(outcomes.items()))


def blitz_landing(armies, cruisers=3, batteries=3):
    landed = Counter()
    for (n, _, _), p in blitz_outcomes(armies, cruisers, batteries).items():
        landed[n] += p
    return dict(sorted(landed.items()))


def run():
    shared = {
        'eots-first-carry': Shared(EOTS_X4, 4, first=True),
        'vbam-first-carry': Shared(VBAM_X10, 10, first=True),
        'eots-simultaneous-carry': Shared(EOTS_X4, 4),
        'eots-simultaneous-plus1-carry': Shared(EOTS_X4, 4, shift=1),
        'eots-simultaneous-plus1-ceil': Shared(EOTS_X4, 4, shift=1, rounding='ceil'),
    }
    binomial = independent(40, 40)
    def results(a, d):
        out = {}
        for name, model in shared.items():
            w, r, m, maximum, sa, sd = model.solve(a, d)
            assert maximum <= 64, (name, a, d, maximum)
            out[name] = dict(capture=w, rounds=r, mutual=m, maximum_rounds=maximum, cap=0., attacker_survivors=sa, defender_survivors=sd)
        out['independent-30-35'] = binomial(a, d)
        for tie in (False, True):
            w, r, m = legacy.ground(a, d, defender_tie=tie)
            out['current-defender-tie' if tie else 'current'] = dict(capture=w, rounds=r, mutual=m)
        return out
    rows = [dict(attackers=a, defenders=10, rules=results(a, 10)) for a in range(5, 21)]
    scale_pairs = {(a,d) for d in (1,2,5,10,20,40) for a in {d, min(40, math.ceil(1.5*d)), min(40, 2*d)}} | {(3,1),(5,2)}
    scale = [dict(attackers=a, defenders=d, rules=results(a,d)) for a,d in sorted(scale_pairs, key=lambda x:(x[1],x[0]))]
    blitz = []
    for ca in (0, 3, 6):
        for a in range(5,21):
            landing = blitz_landing(a, ca)
            odds = {name: sum(p * model.solve(n,10)[0] for n,p in landing.items()) for name,model in shared.items()}
            odds['independent-30-35'] = sum(p*binomial(n,10)['capture'] for n,p in landing.items())
            odds['current'] = sum(p*legacy.ground(n,10)[0] for n,p in landing.items())
            blitz.append(dict(loaded_armies=a, cruisers=ca, batteries=3, defenders=10, landed_distribution=landing, capture=odds))
    data = dict(method='Exact finite-state enumeration in floating point, cap 64. Shared models also enumerate their longest path. Blitz uses existing planetary CRT, not the approved flat fleet table.', engine_validated=False,
                candidates='Shared: one roll per side; fractional army damage carries within landing only; no free critical. Independent: each army fires once per round, attacker p=.30, defender p=.35. Mutual annihilation leaves owner with zero armies.', ground=rows, scale=scale, blitz=blitz)
    root = Path(__file__).parent
    (root/'nc_ground_calibration.json').write_text(json.dumps(data, indent=2)+'\n')
    names = ['current','current-defender-tie','eots-first-carry','vbam-first-carry','eots-simultaneous-plus1-carry','independent-30-35']
    lines = ['# t068 exact ground calibration', '', 'Capture percentages. Counts are armies actually landed. Candidate mutual annihilation leaves the defender owning an empty planet. No homeworld bonus.', '', '| A vs 10 | '+' | '.join(names)+' |', '|---|'+'---:|'*len(names)]
    for row in rows:
        lines.append('| '+str(row['attackers'])+' | '+' | '.join(f"{100*row['rules'][n]['capture']:.2f}" for n in names)+' |')
    lines += ['', '## Blitz: 3 CA, one loaded TT per army, 3 batteries, 10 armies', '', 'Conditional on orbital gate won; existing cover and landing fire unchanged.', '', '| Loaded | Current | EotS defender-first carry | EotS simultaneous +1 carry | Independent .30/.35 |', '|---:|---:|---:|---:|---:|']
    for row in blitz:
        if row['cruisers']==3:
            lines.append('| '+str(row['loaded_armies'])+' | '+' | '.join(f"{100*row['capture'][n]:.2f}" for n in ('current','eots-first-carry','eots-simultaneous-plus1-carry','independent-30-35'))+' |')
    (root/'nc_ground_calibration.md').write_text('\n'.join(lines)+'\n')
    print('\n'.join(lines))


if __name__ == '__main__':
    run()
