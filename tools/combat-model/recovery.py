"""t062 RT timing model. Counts per class/cohort, never per-hull objects.

Each call handles one ship class. A cohort key is
(turns remaining, crippled this pass, qualified for starbase assistance).
Qualification is supplied by the movement layer: same base/owner/orbit for
all 52 ticks. This model does not simulate movement or choose class RTs.
"""
from collections import Counter
import json
from pathlib import Path

NOMINAL = (0, False, False)


def force(nominal=0, damaged=()):
    state = Counter({NOMINAL: nominal})
    for turns, count in damaged:
        state[turns, False, False] += count
    return +state


def begin_year(state, at_base=False):
    out = Counter()
    for (turns, fresh, _), count in state.items():
        assert not fresh, 'end_year must commit the previous pass first'
        out[turns, False, at_base and turns > 0] += count
    return out


def cripple(state, count, rt):
    assert rt >= 1 and 0 <= count <= state[NOMINAL]
    out = state.copy()
    out[NOMINAL] -= count
    out[rt, True, False] += count
    return +out


def lose_assistance(state):
    """Movement, base loss, or an ownership/orbit change voids this bonus."""
    out = Counter()
    for (turns, fresh, _), count in state.items():
        out[turns, fresh, False] += count
    return out


def merge(a, b):
    return a + b


def detach_crippled(state, count):
    """Worst recovery time first; caller already selected crippled hulls.

For equal time, unassisted hulls go first. This tie is model bookkeeping,
not a proposed player control. A same-location transfer retains eligibility;
movement must explicitly call lose_assistance on the departing group.
"""
    out, taken = state.copy(), Counter()
    keys = sorted((k for k in state if k[0]), key=lambda k: (-(k[0] + k[1]), k[2]))
    assert 0 <= count <= sum(state[k] for k in keys)
    for key in keys:
        n = min(count, out[key])
        out[key] -= n
        taken[key] += n
        count -= n
    return +out, +taken


def finish_crippled(state, count):
    return detach_crippled(state, count)[0]


def end_year(state):
    """B: after combat/merges/colonization, before AI and reports."""
    out = Counter()
    recovered = 0
    for (turns, fresh, assisted), count in state.items():
        if turns == 0:
            out[NOMINAL] += count
            continue
        left = turns if fresh else max(0, turns - 1 - int(assisted))
        recovered += count if left == 0 else 0
        out[left, False, False] += count
    return +out, recovered


def timeline(rt, assisted):
    state = cripple(force(1), 1, rt)
    state, recovered = end_year(state)
    rows = [dict(year_after_damage=0, nominal=state[NOMINAL], recovered=recovered)]
    for year in range(1, rt + 1):
        state, recovered = end_year(begin_year(state, at_base=assisted))
        rows.append(dict(year_after_damage=year, nominal=state[NOMINAL], recovered=recovered))
    return rows


if __name__ == '__main__':
    rows = [dict(rt=rt, starbase=assisted, timeline=timeline(rt, assisted))
            for rt in range(1, 5) for assisted in (False, True)]
    Path(__file__).with_name('nc_recovery_timing.json').write_text(json.dumps(rows, indent=2)+'\n')
