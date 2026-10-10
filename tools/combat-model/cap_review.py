"""t062: count actual round-cap exits, separately from all stalemates."""
import json
import random
from pathlib import Path
import rules as R
from fleet_model import Model, force
from sweep import CASES
from withdrawal_review import BASE


class CapModel(Model):
    def __init__(self, rules, units):
        super().__init__(rules, units)
        self.caps = 0

    def step(self, a, b, r, roe, exchange):
        if r > self.rules['round_cap'] and not self.done(a, b):
            self.caps += 1
        return super().step(a, b, r, roe, exchange)


def main():
    rows = []
    trials = 10000
    for variant in ('baseline', 'dd-10-8-5'):
        for name, a, b, roe, tags in CASES:
            if variant != 'baseline' and 'dd' not in tags:
                continue
            m = CapModel(BASE, R.load_units(variant))
            rng = random.Random(f'20261010/cap/{variant}/{name}')
            for _ in range(trials):
                m.simulate(force(**a), force(**b), rng, roe)
            rows.append(dict(case=name, units=variant, trials=trials, cap_exits=m.caps))
    Path(__file__).with_name('nc_cap_review.json').write_text(json.dumps(
        dict(seed=20261010, rules=BASE, rows=rows), indent=2)+'\n')
    print(sum(r['cap_exits'] for r in rows), 'caps in', sum(r['trials'] for r in rows), 'battles')


if __name__ == '__main__':
    main()
