"""Rule sets and unit tables for the combat odds sweep (t055).

A rule set is a plain dict so it can be printed, saved as JSON, loaded
from JSON, and overridden from the command line. `fleet_model.py` reads
it; nothing here runs a battle. The field reference is in README.md.
"""
import copy
import json

# Class order is fixed: the engine's index order. Each entry gives attack
# strength, defence strength (the cost of one damage step), build cost,
# which hit pool the class fires into, and whether it stands in the
# combat line (auxiliaries are screened while the line stands).
UNITS = {
    'DD': dict(AS=10, DS=5, cost=5, fire='execution', line=True),
    'CA': dict(AS=30, DS=30, cost=15, fire='suppression', line=True),
    'BB': dict(AS=90, DS=100, cost=45, fire='suppression', line=True),
    'SB': dict(AS=100, DS=120, cost=50, fire='execution', line=True),
    'SC': dict(AS=0, DS=10, cost=15, fire=None, line=False),
    'TT': dict(AS=0, DS=10, cost=5, fire=None, line=False),
    'ET': dict(AS=0, DS=20, cost=20, fire=None, line=False),
}
CLASSES = tuple(UNITS)

# The engine's ratio CRT, multipliers in quarters, one row per d10 value and
# one column per ratio band: Disadvantaged, Pressed, Even, Advantaged,
# Overwhelming (nc-combat-spec.md).
CRT_X4 = [
    [0, 1, 2, 3, 4], [1, 2, 3, 4, 5], [1, 2, 4, 5, 6], [2, 3, 4, 5, 6],
    [2, 3, 4, 6, 7], [2, 4, 5, 6, 7], [3, 4, 5, 6, 8], [3, 4, 6, 7, 8],
    [4, 5, 6, 7, 8], [4, 6, 7, 8, 10],
]
# EotS air-naval table in quarters: 0-2 give 0.25, 3-5 give 0.50, 6-9 give
# 1.00; mean 0.625 (Codex's P/X experiment).
EOTS_X4 = [1, 1, 1, 2, 2, 2, 4, 4, 4, 4]
# Scale every EotS row by 0.72: same relative spread, mean 0.45.
MID_X50 = [9, 9, 9, 18, 18, 18, 36, 36, 36, 36]
# VBAM's CER in tenths; mean 0.30 (the synthesis's literal reading).
VBAM_X10 = [1, 2, 2, 3, 3, 3, 3, 4, 4, 5]

CURRENT = dict(
    name='current',
    # Effectiveness: 'crt' picks a column from the AS ratio and shifts it;
    # 'flat' uses one row per die value and adds shifts to the die.
    effect=dict(kind='crt', rows=CRT_X4, unit=4),
    # Column (or die) shifts. mixed: any two of DD/CA/BB present (engine's
    # is_mixed). base: 'present' counts any starbase, 'undamaged' only a
    # nominal one, 'none' never. shift_cap bounds the flat-table die shift.
    mixed_shift=1,
    base_shift=1,
    base_condition='present',
    shift_cap=2,
    # Classes whose AS fires into the execution pool (finish crippled at
    # 1xDS, destroy nominal at 2xDS); every other armed class suppresses.
    execution_classes=['DD', 'SB'],
    # The finishing step of a crippled unit: 'full' costs its DS again
    # (engine); 'half' costs half its DS, rounded down like crippled AS, so
    # a crippled ship has half AS and half DS as in VBAM.
    crippled_ds='full',
    # What happens to hits too few for the next step: 'discard' (engine),
    # 'residual' (they buy that step with probability hits/cost), 'carry'
    # (they are kept against the same side for the next round of this
    # battle, per pool; dropped when the battle ends or that side leaves).
    remainder='discard',
    # Natural 9: 'free-kill' destroys one unit for free, crippled first,
    # cheapest first (engine); 'focus' lets the suppression pool buy the
    # highest-DS affordable line step before ordinary allocation, finishing
    # a crippled unit if there is one; 'none' ignores it.
    critical='free-kill',
    # 'absolute': auxiliaries are hit only once the line is gone.
    # 'conditional': also exposed, first, to a pursuit exchange.
    screen='absolute',
    # ROE: 'lock3' checks only from round 4 (engine); 'after1' checks every
    # side after every round from the first.
    roe='lock3',
    # Leaving: 'pressed' is the engine's withdrawal exchange (both sides
    # fire once at the Pressed column); 'pursuit' is one ordinary exchange
    # in which the pursuer's fire reaches auxiliaries under the conditional
    # screen; 'none' is a clean break.
    withdrawal='pressed',
    # Ordinary battle iterations; a final withdrawal/pursuit exchange can
    # add one more volley. The result's ROUNDS counts every exchange.
    round_cap=64,
)


def variant(base, name, **changes):
    rules = copy.deepcopy(base)
    rules['name'] = name
    rules.update(changes)
    return rules


# The consultation's flat-table family: EotS effectiveness, no mixed
# bonus, +1 die for an undamaged base, DD/SB execution kept, criticals as
# budget focus, per-fleet ROE after one compulsory exchange with a bounded
# pursuit exchange, conditional screen. The remainder members differ only
# in the remainder rule; flat-half-ds keeps discard and halves the crippled
# damage step instead. The VBAM-rate pair changes effectiveness alone,
# then the finishing step, retaining the other NC candidate rules.
FLAT = variant(
    CURRENT, 'flat-discard',
    effect=dict(kind='flat', rows=EOTS_X4, unit=4),
    mixed_shift=0,
    base_condition='undamaged',
    critical='focus',
    screen='conditional',
    roe='after1',
    withdrawal='pursuit',
)

VBAM_FLAT = variant(
    FLAT, 'vbam-full-ds',
    effect=dict(kind='flat', rows=VBAM_X10, unit=10),
)

RULESETS = {
    'current': CURRENT,
    # Implemented fleet rules at 28814c9d. Keep 'current' as CRT-era evidence.
    'engine-t060': variant(CURRENT, 'engine-t060',
                           effect=dict(kind='flat', rows=EOTS_X4, unit=4),
                           remainder='carry', mutual_withdrawal_exchange=True,
                           critical_order=['DD', 'SC', 'TT', 'ET', 'CA', 'BB', 'SB']),
    'flat-discard': FLAT,
    'flat-residual': variant(FLAT, 'flat-residual', remainder='residual'),
    'flat-carry': variant(FLAT, 'flat-carry', remainder='carry'),
    'flat-half-ds': variant(FLAT, 'flat-half-ds', crippled_ds='half'),
    'vbam-full-ds': VBAM_FLAT,
    'vbam-half-ds': variant(VBAM_FLAT, 'vbam-half-ds', crippled_ds='half'),
    'flat-carry-half-ds': variant(FLAT, 'flat-carry-half-ds', remainder='carry', crippled_ds='half'),
    'mid-carry': variant(FLAT, 'mid-carry', remainder='carry',
                         effect=dict(kind='flat', rows=MID_X50, unit=50)),
    'mid-carry-half-ds': variant(FLAT, 'mid-carry-half-ds', remainder='carry', crippled_ds='half',
                                 effect=dict(kind='flat', rows=MID_X50, unit=50)),
    'vbam-carry-half-ds': variant(VBAM_FLAT, 'vbam-carry-half-ds', remainder='carry', crippled_ds='half'),
}

# t065/t078 implementation. Speeds refer to surviving ships, not current
# movement speed; this preset models one fresh-contact escape episode.
RULESETS['engine-t065'] = variant(
    RULESETS['engine-t060'], 'engine-t065', roe='after1',
    withdrawal='speed-pursuit', mutual_withdrawal_exchange=False,
    unarmed_escape=True, base_cannot_withdraw=True,
    default_units='dd-10-8-5',
    ship_speeds=dict(DD=6, CA=5, BB=4, SC=6, TT=5, ET=3),
)

# t198: the engine without the mixed-fleet bonus, in any theater.
RULESETS['engine-t198'] = variant(RULESETS['engine-t065'], 'engine-t198', mixed_shift=0)

# Hull variants: unit-table overrides by name.
UNIT_VARIANTS = {
    'baseline': {},
    'dd-10-8-5': {'DD': dict(DS=8)},
}


def apply_overrides(rules, units, overrides):
    """Apply `key=value` overrides. Keys are rule fields (`remainder=carry`,
    `effect.kind=flat`) or unit fields (`units.DD.DS=8`). Values parse as
    JSON when they can, else as strings."""
    for item in overrides:
        key, _, raw = item.partition('=')
        if not _:
            raise ValueError(f'override needs key=value: {item!r}')
        try:
            value = json.loads(raw)
        except json.JSONDecodeError:
            value = raw
        path = key.split('.')
        target = units if path[0] == 'units' else rules
        if path[0] == 'units':
            path = path[1:]
        for part in path[:-1]:
            if part not in target:
                raise KeyError(f'unknown field {key!r}')
            target = target[part]
        if path[-1] not in target:
            raise KeyError(f'unknown field {key!r}')
        target[path[-1]] = value
    return rules, units


def load_rules(name_or_path):
    if name_or_path in RULESETS:
        return copy.deepcopy(RULESETS[name_or_path])
    with open(name_or_path) as handle:
        rules = json.load(handle)
    missing = set(CURRENT) - set(rules)
    if missing:
        raise KeyError(f'rule file lacks {sorted(missing)}')
    return rules


def load_units(variant_name):
    units = copy.deepcopy(UNITS)
    for cls, changes in UNIT_VARIANTS[variant_name].items():
        units[cls].update(changes)
    return units
