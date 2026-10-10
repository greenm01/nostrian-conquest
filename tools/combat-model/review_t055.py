"""Small semantic probes for t055; no Rust builds or changes to candidate rules."""

import argparse
from functools import lru_cache
import hashlib
import json
from pathlib import Path
import random
import statistics
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
from fleet_model import Model, P_STALEMATE, ROUNDS, VECTOR, WD_A, LOST_A, force
from rules import CLASSES, load_rules, load_units


def finite_horizon(model, left, right, roe=(10, 10)):
    """Enumerate small probes with the actual round in the cache key.

    Reuse allocation and transition code; independently check time accounting.
    """
    @lru_cache(None)
    def visit(a, b, round_number):
        total = [0.0] * VECTOR
        for probability, fought, nxt in model.step(a, b, round_number, roe, model.exchange):
            values = model.terminal(left, right, *nxt[1:]) if nxt[0] == 'terminal' else visit(*nxt[1:])
            for i, value in enumerate(values):
                total[i] += probability * value
            total[ROUNDS] += probability * fought
        return tuple(total)
    return visit(left, right, 1)


def summary(values):
    return dict(left=values[0], right=values[1], mutual=values[2],
                stalemate=values[P_STALEMATE], exchanges=values[ROUNDS])


def sample_convoy(screen, withdrawal, trials):
    rules = load_rules('flat-carry')
    rules.update(screen=screen, withdrawal=withdrawal)
    model = Model(rules, load_units('baseline'))
    left, right = force(CA=5, TT=10), force(CA=10)
    results = [model.simulate(left, right, random.Random(20261010 + i), (6, 10))
               for i in range(trials)]
    tt = [v[LOST_A.start + CLASSES.index('TT')] for v in results]
    ca = [v[LOST_A.start + CLASSES.index('CA')] for v in results]
    return dict(screen=screen, withdrawal=withdrawal, trials=trials,
                mean_TT_lost=statistics.mean(tt),
                standard_error_TT_lost=statistics.stdev(tt) / trials ** 0.5,
                mean_CA_lost=statistics.mean(ca),
                withdrew=sum(v[WD_A] for v in results) / trials)


def run(trials):
    cases = []
    for cap in (1, 64):
        for name in ('flat-discard', 'flat-residual', 'flat-carry'):
            rules = load_rules(name)
            rules['round_cap'] = cap
            model = Model(rules, load_units('baseline'))
            a, b = force(CA=1), force(CA=1)
            solver, _ = model.solve(a, b)
            finite = finite_horizon(model, a, b)
            cases.append(dict(rules=name, cap=cap, solver=summary(solver), finite=summary(finite)))
    rules = load_rules('flat-carry')
    units = load_units('baseline')
    units['CA'].update(AS=2, DS=100)
    # Deliberately slow, deterministic legal input: 50 rounds to cripple,
    # 100 more to destroy. The 64-round version must stop with survivors.
    rules['effect']['rows'] = [4] * 10
    model = Model(rules, units)
    a, b = force(CA=1), force(CA=1)
    solver, _ = model.solve(a, b)
    slow = dict(AS=2, DS=100, cap=64, solver=summary(solver),
                finite=summary(finite_horizon(model, a, b)))

    model = Model(load_rules('flat-carry'), load_units('baseline'))
    # An undamaged BB plus a 99-hit suppression budget is still full AS.
    a = force(BB=1)
    state = model.apply(a, 99, 0, False)[0][1]
    carry = dict(initial_AS=model.side_as(a), after_99_hits_AS=model.side_as(state),
                 bank=list(state[2]),
                 after_one_more_hit=model.apply(state, 1, 0, False)[0][1],
                 after_battle_reset_then_one_hit=model.apply(a, 1, 0, False)[0][1])
    paths = ('rules.py', 'fleet_model.py')
    return dict(
        source_sha256={p: hashlib.sha256(Path(__file__).with_name(p).read_bytes()).hexdigest() for p in paths},
        round_cap_cases=cases, slow_case=slow, carry_budget_probe=carry,
        convoy=[sample_convoy(screen, mode, trials) for mode in ('none', 'pursuit')
                for screen in ('absolute', 'conditional')],
        limitations=['No combat seed equivalence with Rust.',
                     'Finite-horizon checker reuses the model transition rules.',
                     'Convoy is 5 CA + 10 empty TT at ROE6 against 10 CA ROE10.',
                     'Pursuit is absent or certain; neither models speed or interception.'],
    )


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--trials', type=int, default=10000)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    if args.trials < 2:
        parser.error('--trials must be at least 2')
    rendered = json.dumps(run(args.trials), indent=2) + '\n'
    if args.output:
        args.output.write_text(rendered)
    else:
        print(rendered, end='')
