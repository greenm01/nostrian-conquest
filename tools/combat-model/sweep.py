#!/usr/bin/env python3
"""Combat odds sweep (t055): the resolution candidates from the combat
review, compared with numbers before any engine rule changes.

Runs each rule set in rules.py over the review's matchups, exactly where
the state space allows and by seeded sampling otherwise, and the ground
candidates exactly. Prints Markdown and writes JSON on request. With
--engine it compares the current-rule numbers with the output of the
engine-side harness (rust/nc-fuzz/examples/combat_odds.rs). README.md
explains the rule sets and the semantics each candidate assumes.
"""
import argparse
import json
import os
import random
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import ground as defender_first  # noqa: E402
import nc_combat_analysis as legacy  # noqa: E402
import rules as R  # noqa: E402
from confidence import binomial_interval, binomial_test  # noqa: E402
from fleet_model import (  # noqa: E402
    CRIP_A, CRIP_B, LOSS_A, LOSS_B, LOST_A, LOST_B, OUTCOMES, P_A, P_B, P_MUTUAL, P_STALEMATE,
    ROUNDS, VECTOR, WD_A, WD_B, Model, TooLarge, force,
)

DEFAULT_SEED = 20261010
OUTCOME_INDEX = {'A': P_A, 'B': P_B, 'mutual': P_MUTUAL, 'stalemate': P_STALEMATE}
RULE_FIELDS = ('effect', 'mixed_shift', 'base_condition', 'execution_classes', 'remainder',
               'critical', 'screen', 'roe', 'withdrawal')

# name, left, right, ROE (left, right), tags. The 'engine' tag marks the
# cases combat_odds.rs runs under the same names; 'dd' marks the cases
# repeated with the DD 10/8 at cost 5 hull variant.
CASES = [
    ('10 CA vs 10 CA', dict(CA=10), dict(CA=10), (10, 10), 'ca engine'),
    ('11 CA vs 10 CA', dict(CA=11), dict(CA=10), (10, 10), 'ca engine'),
    ('15 CA vs 10 CA', dict(CA=15), dict(CA=10), (10, 10), 'ca engine'),
    ('20 CA vs 10 CA', dict(CA=20), dict(CA=10), (10, 10), 'ca engine'),
    ('30 CA vs 10 CA', dict(CA=30), dict(CA=10), (10, 10), 'ca'),
    ('30 DD vs 10 CA', dict(DD=30), dict(CA=10), (10, 10), 'dd engine'),
    ('90 DD vs 10 BB', dict(DD=90), dict(BB=10), (10, 10), 'dd engine'),
    ('30 CA vs 10 BB', dict(CA=30), dict(BB=10), (10, 10), 'ca engine'),
    ('10 DD vs 1 SB', dict(DD=10), dict(SB=1), (10, 10), 'dd'),
    ('29 CA + 3 DD vs 30 CA', dict(CA=29, DD=3), dict(CA=30), (10, 10), 'mixed engine'),
    ('3 DD vs 1 CA', dict(DD=3), dict(CA=1), (10, 10), 'small dd'),
    ('3 CA vs 1 BB', dict(CA=3), dict(BB=1), (10, 10), 'small'),
    ('1 DD vs 1 BB', dict(DD=1), dict(BB=1), (10, 10), 'small'),
    ('10 CA ROE 6 vs 15 CA', dict(CA=10), dict(CA=15), (6, 10), 'roe engine'),
    ('10 CA ROE 6 vs 10 CA ROE 6', dict(CA=10), dict(CA=10), (6, 6), 'roe'),
    ('5 CA + 10 TT ROE 6 vs 10 CA', dict(CA=5, TT=10), dict(CA=10), (6, 10), 'convoy'),
]
GROUND_CASES = [(1, 1), (5, 5), (8, 10), (10, 10), (11, 10), (12, 10), (15, 10), (20, 10),
                (20, 20), (30, 10)]


def state_bound(left, right):
    """An upper bound on (counts, crippled) states, ignoring carry."""
    bound = 1
    for side in (left, right):
        for count in side.values():
            bound *= (count + 1) * (count + 2) // 2
    return bound


def run_case(model, rules_name, units_name, case, trials, seed, limit):
    name, left, right, roe, _ = case
    a0, b0 = force(**left), force(**right)
    record = dict(ruleset=rules_name, units=units_name, case=name, left=left, right=right, roe=list(roe))
    started = time.time()
    vector = None
    if state_bound(left, right) <= limit:
        try:
            vector, states = model.solve(a0, b0, roe, limit)
            record.update(method='exact', states=states)
        except TooLarge:
            vector = None
    if vector is None:
        rng = random.Random(f'{seed}/{rules_name}/{units_name}/{name}')
        total = [0.0] * VECTOR
        for _ in range(trials):
            for i, x in enumerate(model.simulate(a0, b0, rng, roe)):
                total[i] += x
        vector = [x / trials for x in total]
        counts = {key: int(round(total[OUTCOME_INDEX[key]])) for key in OUTCOMES}
        ci = {}
        for key in OUTCOMES:
            low, high, method = binomial_interval(counts[key], trials)
            ci[key] = [low, high]
        record.update(method='sampled', trials=trials, counts=counts, ci=ci, ci_method=method)
    record.update(
        p={key: vector[OUTCOME_INDEX[key]] for key in OUTCOMES},
        rounds=vector[ROUNDS],
        loss_left=vector[LOSS_A], loss_right=vector[LOSS_B],
        crippled_left=vector[CRIP_A], crippled_right=vector[CRIP_B],
        withdrew_left=vector[WD_A], withdrew_right=vector[WD_B],
        lost_left=dict(zip(R.CLASSES, vector[LOST_A])),
        lost_right=dict(zip(R.CLASSES, vector[LOST_B])),
        seconds=round(time.time() - started, 2),
    )
    return record


def run_ground():
    records = []
    for attackers, defenders in GROUND_CASES:
        current = legacy.ground(attackers, defenders)
        defender_ties = legacy.ground(attackers, defenders, defender_tie=True)
        simultaneous = legacy.binomial_ground(attackers, defenders)
        sequential = defender_first.solve(attackers, defenders)
        records.append(dict(
            attackers=attackers, defenders=defenders, method='exact',
            current=dict(capture=current[0], rounds=current[1], mutual=current[2]),
            current_defender_ties=dict(capture=defender_ties[0], rounds=defender_ties[1]),
            simultaneous=dict(capture=simultaneous[0], rounds=simultaneous[1]),
            defender_first=dict(capture=sequential[0], rounds=sequential[1]),
        ))
    return records


# ----- Markdown ---------------------------------------------------------
def pct(p):
    return f'{100 * p:.1f}%'


def cell(record, outcome='A'):
    text = pct(record['p'][outcome])
    if record['method'] == 'sampled':
        low, high = record['ci'][outcome]
        text += f' [{100 * low:.1f}, {100 * high:.1f}]'
    return text


def ships(lost):
    return ' '.join(f'{cls} {count:.1f}' for cls, count in lost.items() if count) or '-'


def table(header, rows):
    out = ['| ' + ' | '.join(header) + ' |', '| ' + ' | '.join('---' for _ in header) + ' |']
    out.extend('| ' + ' | '.join(str(x) for x in row) + ' |' for row in rows)
    return '\n'.join(out)


def case_label(record):
    label = record['case']
    if record['units'] != 'baseline':
        label += f" ({record['units']})"
    return label


def render(results, rulesets, args):
    lines = ['# Combat odds sweep', '']
    lines.append(f"Seed {args.seed}; {args.trials} trials per sampled case; exact enumeration up to "
                 f"{args.exact_limit} states. Percentages without an interval are exact; intervals are 95%.")
    lines.append('')
    lines.append('## Rule sets')
    lines.append('')
    lines.append(table(['rule set'] + list(RULE_FIELDS),
                       [[name] + [summarise(rules[field]) for field in RULE_FIELDS]
                        for name, rules in rulesets.items()]))
    lines.append('')
    battles = results['battles']
    if battles:
        names = list(rulesets)
        labels = []
        for record in battles:
            label = case_label(record)
            if label not in labels:
                labels.append(label)
        by_key = {(case_label(r), r['ruleset']): r for r in battles}
        for title, fn in (('Left win rate', lambda r: cell(r, 'A')),
                          ('Mean exchanges', lambda r: f"{r['rounds']:.2f}"),
                          ('Left losses, destroyed + crippled build points',
                           lambda r: f"{r['loss_left']:.1f} + {r['crippled_left']:.1f}"),
                          ('Right losses, destroyed + crippled build points',
                           lambda r: f"{r['loss_right']:.1f} + {r['crippled_right']:.1f}")):
            lines.append(f'## {title}')
            lines.append('')
            rows = []
            for label in labels:
                row = [label]
                for name in names:
                    record = by_key.get((label, name))
                    row.append(fn(record) if record else '')
                rows.append(row)
            lines.append(table(['case'] + names, rows))
            lines.append('')
        for name in names:
            lines.append(f'## Detail: {name}')
            lines.append('')
            rows = []
            for record in battles:
                if record['ruleset'] != name:
                    continue
                rows.append([
                    case_label(record),
                    record['method'] if record['method'] == 'exact' else f"sampled {record['trials']}",
                    cell(record, 'A'), cell(record, 'B'), cell(record, 'mutual'), cell(record, 'stalemate'),
                    f"{record['rounds']:.2f}",
                    f"{record['loss_left']:.1f}", f"{record['loss_right']:.1f}",
                    f"{record['crippled_left']:.1f}", f"{record['crippled_right']:.1f}",
                    ships(record['lost_left']), ships(record['lost_right']),
                    f"{pct(record['withdrew_left'])} / {pct(record['withdrew_right'])}",
                ])
            lines.append(table(['case', 'method', 'left win', 'right win', 'mutual', 'stalemate',
                                'exchanges', 'left lost', 'right lost', 'left crippled',
                                'right crippled', 'left ships lost', 'right ships lost',
                                'withdrew L / R'], rows))
            lines.append('')
    if results.get('ground'):
        lines.append('## Ground combat, capture chance and mean rounds (exact)')
        lines.append('')
        rows = []
        for record in results['ground']:
            rows.append([f"{record['attackers']} vs {record['defenders']}"] + [
                f"{pct(record[key]['capture'])} / {record[key]['rounds']:.2f}"
                for key in ('current', 'current_defender_ties', 'simultaneous', 'defender_first')])
        lines.append(table(['attackers vs defenders', 'current CRT', 'current, ties to defender',
                            'simultaneous .30/.35', 'defender-first'], rows))
        lines.append('')
    if results.get('engine'):
        lines.append('## Model (current rules) against the engine')
        lines.append('')
        rows = []
        for row in results['engine']:
            rows.append([row['case'], row['model_left'], row['engine_left'], row['agree'],
                         row['model_mutual'], row['engine_mutual'],
                         row['model_loss'], row['engine_loss'], row['years_per_battle']])
        lines.append(table(['case', 'model left win', 'engine left win [95%]', 'agree', 'model mutual',
                            'engine mutual', 'model lost L / R', 'engine lost L / R',
                            'years per battle'], rows))
        lines.append('')
        lines.append('An exact model value agrees when the engine count passes a two-sided exact binomial '
                     'test against it at 5%; a sampled one when the two intervals overlap.')
        lines.append('')
    return '\n'.join(lines)


def summarise(value):
    if isinstance(value, dict):
        return value.get('kind', json.dumps(value))
    if isinstance(value, list):
        return ' '.join(value) or 'none'
    return str(value)


def compare_engine(path, battles):
    with open(path) as handle:
        engine = json.load(handle)
    current = {r['case']: r for r in battles if r['ruleset'] == 'current' and r['units'] == 'baseline'}
    rows = []
    for case in engine['cases']:
        record = current.get(case['case'])
        if record is None or case['battles'] == 0:
            continue
        n = case['battles']
        counts = case['counts']
        low, high, _ = binomial_interval(counts['left'], n)
        model = record['p']['A']
        if record['method'] == 'exact':
            # The engine's count is tested against the model's exact odds.
            agree = binomial_test(counts['left'], n, model) >= 0.05
        else:
            m_low, m_high = record['ci']['A']
            agree = m_low <= high and low <= m_high
        rows.append(dict(
            case=case['case'],
            model_left=cell(record, 'A'),
            engine_left=f"{pct(counts['left'] / n)} [{100 * low:.1f}, {100 * high:.1f}] n={n}",
            agree='yes' if agree else 'NO',
            model_mutual=pct(record['p']['mutual']),
            engine_mutual=pct(counts['mutual'] / n),
            model_loss=f"{record['loss_left']:.1f} / {record['loss_right']:.1f}",
            engine_loss=f"{case['loss_left']:.1f} / {case['loss_right']:.1f}",
            years_per_battle=f"{case['years_per_battle']:.2f}",
        ))
    return rows


def select_cases(selection):
    if not selection:
        return CASES
    wanted = set(selection)
    return [case for case in CASES if case[0] in wanted or wanted & set(case[4].split())]


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--rules', default=','.join(R.RULESETS),
                        help='comma-separated rule set names or JSON files (default: every preset)')
    parser.add_argument('--set', action='append', default=[], metavar='KEY=VALUE',
                        help='override a rule or unit field in every selected rule set, e.g. '
                             'remainder=carry, critical=none, units.DD.DS=8')
    parser.add_argument('--units', default='all', choices=['all'] + list(R.UNIT_VARIANTS),
                        help="hull table: 'all' runs baseline everywhere and dd-10-8-5 on the DD cases")
    parser.add_argument('--cases', help='comma-separated case names or tags (ca, dd, mixed, small, roe, convoy, engine)')
    parser.add_argument('--trials', type=int, default=10_000)
    parser.add_argument('--seed', type=int, default=DEFAULT_SEED)
    parser.add_argument('--exact-limit', type=int, default=60_000,
                        help='enumerate exactly up to this many states, else sample')
    parser.add_argument('--no-ground', action='store_true')
    parser.add_argument('--engine', metavar='JSON', help='combat_odds.rs output to compare the current rules with')
    parser.add_argument('--json', metavar='PATH')
    parser.add_argument('--markdown', metavar='PATH')
    args = parser.parse_args()

    rulesets = {}
    for item in args.rules.split(','):
        rules = R.load_rules(item)
        units = R.load_units('baseline')
        if args.set:
            R.apply_overrides(rules, units, [s for s in args.set if not s.startswith('units.')])
            rules['name'] += '+' + '+'.join(s for s in args.set if not s.startswith('units.'))
        rulesets[rules['name']] = rules
    unit_overrides = [s for s in args.set if s.startswith('units.')]
    cases = select_cases(args.cases.split(',') if args.cases else None)
    variants = list(R.UNIT_VARIANTS) if args.units == 'all' else [args.units]

    battles = []
    for name, rules in rulesets.items():
        for variant in variants:
            units = R.load_units(variant)
            R.apply_overrides(rules, units, unit_overrides)
            model = Model(rules, units)
            for case in cases:
                if variant != 'baseline' and args.units == 'all' and 'dd' not in case[4].split():
                    continue
                record = run_case(model, name, variant, case, args.trials, args.seed, args.exact_limit)
                battles.append(record)
                print(f"{name:14} {variant:10} {case[0]:28} {record['method']:8} left {pct(record['p']['A']):>6}"
                      f" rounds {record['rounds']:.2f} ({record['seconds']}s)", file=sys.stderr)
    results = dict(seed=args.seed, trials=args.trials, exact_limit=args.exact_limit,
                   rulesets=rulesets, unit_variants={v: R.load_units(v) for v in variants},
                   unit_overrides=unit_overrides, battles=battles)
    if not args.no_ground:
        results['ground'] = run_ground()
    if args.engine:
        results['engine'] = compare_engine(args.engine, battles)
    markdown = render(results, rulesets, args)
    if args.json:
        with open(args.json, 'w') as handle:
            json.dump(results, handle, indent=2)
            handle.write('\n')
    if args.markdown:
        with open(args.markdown, 'w') as handle:
            handle.write(markdown + '\n')
    else:
        print(markdown)


if __name__ == '__main__':
    main()
