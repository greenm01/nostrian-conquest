"""Rule-driven fleet battle model for the combat odds sweep (t055).

One battle step is written once, as a list of (probability, next state)
branches. `solve` follows every branch and gives exact odds; `simulate`
draws one branch per step with a seeded RNG. Both see the same rules, the
same unit table, and the same allocation code, so a sampled result for a
large battle and an exact result for a small one come from one model.

A side is one aggregated force: counts and crippled counts per class in
the engine's class order, plus the carried remainder per pool. The model
has no movement, sighting, pursuit geometry, or repair.
"""
import math
import sys

from rules import CLASSES

# Value vector layout for solve() and the sampled records: outcome
# probabilities, exchanges, build points destroyed and crippled per side,
# withdrawal flags, units destroyed per class for each side, then whether
# at least half of each side's transports survived (mission survival).
P_A, P_B, P_MUTUAL, P_STALEMATE, ROUNDS, LOSS_A, LOSS_B, CRIP_A, CRIP_B, WD_A, WD_B = range(11)
LOST_A = slice(11, 11 + len(CLASSES))
LOST_B = slice(11 + len(CLASSES), 11 + 2 * len(CLASSES))
VIABLE_A, VIABLE_B = 11 + 2 * len(CLASSES), 12 + 2 * len(CLASSES)
VECTOR = 13 + 2 * len(CLASSES)
OUTCOMES = ('A', 'B', 'mutual', 'stalemate')


class TooLarge(Exception):
    """The exact state space passed the caller's limit; sample instead."""


def roe_satisfied(roe, friendly_as, enemy_as):
    """The engine's rule_threshold_satisfied."""
    if roe == 0:
        return False
    if roe == 1:
        return enemy_as == 0
    if roe >= 10:
        return True
    need = {2: (1, 4), 3: (1, 3), 4: (1, 2), 5: (2, 3), 6: (1, 1), 7: (3, 2), 8: (2, 1), 9: (3, 1)}[roe]
    return friendly_as * need[0] >= enemy_as * need[1]


def base_column(our_as, enemy_as):
    if enemy_as == 0 or our_as >= enemy_as * 3:
        return 4
    if our_as * 2 < enemy_as:
        return 0
    if our_as < enemy_as:
        return 1
    if our_as * 2 < enemy_as * 3:
        return 2
    return 3


def force(**counts):
    """A side's starting state from class counts, e.g. force(CA=10)."""
    n = tuple(counts.get(cls, 0) for cls in CLASSES)
    return (n, (0,) * len(CLASSES), (0, 0))


class Model:
    def __init__(self, rules, units):
        self.rules = rules
        self.AS = [units[c]['AS'] for c in CLASSES]
        self.DS = [units[c]['DS'] for c in CLASSES]
        self.COST = [units[c]['cost'] for c in CLASSES]
        k = range(len(CLASSES))
        self.line = [i for i in k if units[CLASSES[i]]['line']]
        self.aux = [i for i in k if not units[CLASSES[i]]['line']]
        self.exposed = self.aux + self.line
        self.execution = [i for i in k if CLASSES[i] in rules['execution_classes']]
        self.suppression = [i for i in k if self.AS[i] > 0 and i not in self.execution]
        self.crit_order = sorted(k, key=lambda i: self.DS[i])
        if 'critical_order' in rules:
            self.crit_order = [CLASSES.index(c) for c in rules['critical_order']]
        self.mixed = [CLASSES.index(c) for c in ('DD', 'CA', 'BB') if c in CLASSES]
        self.sb = CLASSES.index('SB')
        self.tt = CLASSES.index('TT')
        # The finishing step of a crippled unit costs its DS again, or half
        # of it (rounded down like crippled AS, at least 1) under
        # crippled_ds='half'.
        self.FIN = [max(1, d // 2) if rules['crippled_ds'] == 'half' else d for d in self.DS]
        self.KILL = [d + f for d, f in zip(self.DS, self.FIN)]

    # ----- strengths -------------------------------------------------
    def class_as(self, n, c, i):
        return (n[i] - c[i]) * self.AS[i] + c[i] * (self.AS[i] // 2)

    def side_as(self, side):
        n, c, _ = side
        return sum(self.class_as(n, c, i) for i in range(len(n)))

    def pool_as(self, side, pool):
        n, c, _ = side
        return sum(self.class_as(n, c, i) for i in pool)

    def has_line(self, n):
        return any(n[i] for i in self.line)

    def targets(self, n):
        return self.line if self.has_line(n) else self.aux

    def cost_of(self, n):
        return sum(n[i] * self.COST[i] for i in range(len(n)))

    # ----- one side's volley ----------------------------------------
    def volley(self, side, enemy_as, roll, pressed=False):
        """(suppression hits, execution hits, critical) for one d10 roll."""
        total = self.side_as(side)
        if total == 0:
            return (0, 0, False)
        rules = self.rules
        n, c, _ = side
        shift = 0
        if not pressed:
            if rules['mixed_shift'] and sum(n[i] > 0 for i in self.mixed) >= 2:
                shift += rules['mixed_shift']
            condition = rules['base_condition']
            if condition == 'present' and n[self.sb] > 0:
                shift += rules['base_shift']
            elif condition == 'undamaged' and n[self.sb] - c[self.sb] > 0:
                shift += rules['base_shift']
            shift = min(shift, rules['shift_cap'])
        effect = rules['effect']
        if effect['kind'] == 'crt':
            column = 1 if pressed else base_column(total, enemy_as)
            multiplier = effect['rows'][roll][max(0, min(4, column + shift))]
        else:
            multiplier = effect['rows'][min(9, roll + shift)]
        hits = -(-total * multiplier // effect['unit'])
        sup = int(math.floor(hits * self.pool_as(side, self.suppression) / total + 0.5)) if hits else 0
        return (sup, hits - sup, roll == 9)

    # ----- hit allocation -------------------------------------------
    def _suppress(self, n, c, hits, order_fn):
        """Cripple in order, destroy crippled once no nominal remain in the
        current target set; move on to the next target set when one is
        wiped out. Returns the unspent hits."""
        while hits > 0:
            order = order_fn(n)
            progress = False
            for i in order:
                d = self.DS[i]
                if d == 0 or hits < d:
                    continue
                k = min(n[i] - c[i], hits // d)
                if k:
                    c[i] += k
                    hits -= k * d
                    progress = True
            if all(n[i] == c[i] for i in order):
                for i in order:
                    d = self.FIN[i]
                    if hits < d:
                        continue
                    k = min(c[i], hits // d)
                    if k:
                        c[i] -= k
                        n[i] -= k
                        hits -= k * d
                        progress = True
            if not progress:
                break
        return hits

    def _execute(self, n, c, hits, order_fn):
        """Finish crippled at the finishing cost, destroy nominal at DS plus
        the finishing cost (2xDS unless crippled_ds is 'half'), in order."""
        while hits > 0:
            order = order_fn(n)
            progress = False
            for i in order:
                d, kill = self.FIN[i], self.KILL[i]
                if d == 0 or hits < d:
                    continue
                k = min(c[i], hits // d)
                if k:
                    c[i] -= k
                    n[i] -= k
                    hits -= k * d
                    progress = True
                if hits < kill:
                    continue
                k = min(n[i] - c[i], hits // kill)
                if k:
                    n[i] -= k
                    hits -= k * kill
                    progress = True
            if not progress:
                break
        return hits

    def _focus(self, n, c, hits, order_fn):
        """Critical as budget focus: the suppression pool buys the highest
        DS affordable step in the current target set, finishing a crippled
        unit before crippling a nominal one, until it can afford none."""
        while hits > 0:
            cost = lambda i: self.FIN[i] if c[i] else self.DS[i]
            eligible = [i for i in order_fn(n) if n[i] > 0 and hits >= cost(i)]
            if not eligible:
                break
            i = max(eligible, key=lambda i: self.DS[i])
            hits -= cost(i)
            if c[i]:
                c[i] -= 1
                n[i] -= 1
            else:
                c[i] += 1
        return hits

    def _free_kill(self, n, c):
        for i in self.crit_order:
            if c[i] > 0:
                c[i] -= 1
                n[i] -= 1
                return
        for i in self.crit_order:
            if n[i] - c[i] > 0:
                n[i] -= 1
                return

    def _residual_sup(self, n, c, hits, order_fn):
        order = order_fn(n)
        for i in order:
            if n[i] - c[i] > 0:
                return i, self.DS[i], True
        for i in order:
            if c[i] > 0:
                return i, self.FIN[i], False
        return None

    def _residual_exe(self, n, c, hits, order_fn):
        for i in order_fn(n):
            if n[i] > 0:
                return i, (self.FIN[i] if c[i] else self.KILL[i]), c[i] > 0
        return None

    def apply(self, side, sup, exe, crit, exposed=False):
        """Apply one volley to `side`. Returns [(probability, new side)];
        more than one branch only under the residual remainder rule."""
        rules = self.rules
        n, c, carry = list(side[0]), list(side[1]), side[2]
        order_fn = (lambda n: self.exposed) if exposed else self.targets
        remainder = rules['remainder']
        if remainder == 'carry':
            sup += carry[0]
            exe += carry[1]
        if crit and rules['critical'] == 'focus':
            sup = self._focus(n, c, sup, order_fn)
        sup = self._suppress(n, c, sup, order_fn)
        stages = [(1.0, n, c)]
        if remainder == 'residual' and sup > 0:
            target = self._residual_sup(n, c, sup, order_fn)
            if target is not None:
                i, cost, nominal = target
                p = sup / cost
                n2, c2 = list(n), list(c)
                if nominal:
                    c2[i] += 1
                else:
                    c2[i] -= 1
                    n2[i] -= 1
                stages = [(p, n2, c2), (1 - p, n, c)]
            sup = 0
        branches = {}
        for p0, n1, c1 in stages:
            if p0 == 0:
                continue
            left = self._execute(n1, c1, exe, order_fn)
            outs = [(p0, n1, c1)]
            if remainder == 'residual' and left > 0:
                target = self._residual_exe(n1, c1, left, order_fn)
                if target is not None:
                    i, cost, crippled = target
                    p = left / cost
                    n2, c2 = list(n1), list(c1)
                    n2[i] -= 1
                    if crippled:
                        c2[i] -= 1
                    outs = [(p0 * p, n2, c2), (p0 * (1 - p), n1, c1)]
                left = 0
            for p1, n3, c3 in outs:
                if p1 == 0:
                    continue
                if crit and rules['critical'] == 'free-kill':
                    self._free_kill(n3, c3)
                new_carry = (sup, left) if remainder == 'carry' else (0, 0)
                key = (tuple(n3), tuple(c3), new_carry)
                branches[key] = branches.get(key, 0.0) + p1
        return [(p, state) for state, p in branches.items()]

    # ----- exchanges ------------------------------------------------
    def exchange(self, a, b, pressed=False, expose_a=False, expose_b=False):
        """One simultaneous exchange, every roll pair enumerated:
        [(probability, (a', b'))]. expose_x: x's auxiliaries are exposed to
        the other side's fire (conditional screen during pursuit)."""
        as_a, as_b = self.side_as(a), self.side_as(b)
        shots_a, shots_b = {}, {}
        for roll in range(10):
            s = self.volley(a, as_b, roll, pressed)
            shots_a[s] = shots_a.get(s, 0.0) + 0.1
            s = self.volley(b, as_a, roll, pressed)
            shots_b[s] = shots_b.get(s, 0.0) + 0.1
        hits_on_b = {s: self.apply(b, *s, exposed=expose_b) for s in shots_a}
        hits_on_a = {s: self.apply(a, *s, exposed=expose_a) for s in shots_b}
        out = {}
        for sa, pa in shots_a.items():
            for sb, pb in shots_b.items():
                for p1, b2 in hits_on_b[sa]:
                    for p2, a2 in hits_on_a[sb]:
                        key = (a2, b2)
                        out[key] = out.get(key, 0.0) + pa * pb * p1 * p2
        return [(p, ab) for ab, p in out.items()]

    def exchange_sampled(self, rng, a, b, pressed=False, expose_a=False, expose_b=False):
        """One exchange with drawn rolls: [(1.0, (a', b'))]. Draws a's roll,
        b's roll, then any residual rolls for a's fire, then b's."""
        as_a, as_b = self.side_as(a), self.side_as(b)
        roll_a, roll_b = rng.randrange(10), rng.randrange(10)
        shots_a = self.volley(a, as_b, roll_a, pressed)
        shots_b = self.volley(b, as_a, roll_b, pressed)
        b2 = _draw(rng, self.apply(b, *shots_a, exposed=expose_b))
        a2 = _draw(rng, self.apply(a, *shots_b, exposed=expose_a))
        return [(1.0, (a2, b2))]

    # ----- the battle step ------------------------------------------
    def wants_withdrawal(self, side, enemy_as, roe):
        strength = self.side_as(side)
        if strength == 0:
            return bool(any(side[0]) and self.rules.get('unarmed_escape', False))
        return not roe_satisfied(roe, strength, enemy_as)

    def done(self, a, b):
        """Nobody left to shoot, or nobody able to shoot. A side whose
        combat line is gone is still shot at while it has units."""
        return not any(a[0]) or not any(b[0]) or (self.side_as(a) == 0 and self.side_as(b) == 0)

    def step(self, a, b, r, roe, exchange):
        """Transitions from round `r`: [(p, exchanges fought, next)], where
        next is ('state', a, b, r + 1) or ('terminal', a, b, wd_a, wd_b).
        The cap bounds ordinary rounds, not total exchanges: a withdrawal
        or pursuit exchange may follow the last ordinary round."""
        rules = self.rules
        if self.done(a, b) or r > rules['round_cap']:
            return [(1.0, 0, ('terminal', a, b, False, False))]
        as_a, as_b = self.side_as(a), self.side_as(b)
        fail_a = self.wants_withdrawal(a, as_b, roe[0])
        fail_b = self.wants_withdrawal(b, as_a, roe[1])
        if rules['roe'] == 'lock3' and r == 4 and (fail_a or fail_b):
            # First check of the engine's loop: declared before the round;
            # both sides may leave.
            return self._leave(a, b, fail_a, fail_b, 0, exchange)
        out = []
        for p, (a2, b2) in exchange(a, b):
            out.extend((p * q, 1 + fought, nxt) for q, fought, nxt in self._after_round(a2, b2, r, roe, exchange))
        return out

    def _after_round(self, a, b, r, roe, exchange):
        rules = self.rules
        if self.done(a, b):
            return [(1.0, 0, ('terminal', a, b, False, False))]
        as_a, as_b = self.side_as(a), self.side_as(b)
        fail_a = self.wants_withdrawal(a, as_b, roe[0])
        fail_b = self.wants_withdrawal(b, as_a, roe[1])
        if rules['roe'] == 'lock3':
            if r < 4 or not (fail_a or fail_b):
                return [(1.0, 0, ('state', a, b, r + 1))]
            # Post-round checks run in ascending (ROE, side) order and the
            # second side sees no opponent once the first has left.
            if fail_a and fail_b:
                if (roe[1], 1) < (roe[0], 0):
                    fail_a = False
                else:
                    fail_b = False
            return self._leave(a, b, fail_a, fail_b, 0, exchange)
        if not (fail_a or fail_b):
            return [(1.0, 0, ('state', a, b, r + 1))]
        return self._leave(a, b, fail_a, fail_b, 0, exchange)

    def _leave(self, a, b, wd_a, wd_b, fought, exchange):
        mode = self.rules['withdrawal']
        if mode == 'none' or (wd_a and wd_b and not self.rules.get('mutual_withdrawal_exchange', False)):
            return [(1.0, fought, ('terminal', a, b, wd_a, wd_b))]
        conditional = self.rules['screen'] == 'conditional'
        if mode == 'pressed':
            branches = exchange(a, b, pressed=True)
        else:
            branches = exchange(a, b, expose_a=wd_a and conditional, expose_b=wd_b and conditional)
        return [(p, fought + 1, ('terminal', a2, b2, wd_a, wd_b)) for p, (a2, b2) in branches]

    # ----- results --------------------------------------------------
    def terminal(self, a0, b0, a, b, wd_a, wd_b):
        v = [0.0] * VECTOR
        as_a, as_b = self.side_as(a), self.side_as(b)
        if as_a > 0 and as_b == 0:
            v[P_A] = 1.0
        elif as_b > 0 and as_a == 0:
            v[P_B] = 1.0
        elif as_a == 0 and as_b == 0:
            v[P_MUTUAL] = 1.0
        elif wd_b and not wd_a:
            v[P_A] = 1.0
        elif wd_a and not wd_b:
            v[P_B] = 1.0
        else:
            v[P_STALEMATE] = 1.0
        v[LOSS_A] = self.cost_of(a0[0]) - self.cost_of(a[0])
        v[LOSS_B] = self.cost_of(b0[0]) - self.cost_of(b[0])
        v[CRIP_A] = self.cost_of(a[1])
        v[CRIP_B] = self.cost_of(b[1])
        v[WD_A] = float(wd_a)
        v[WD_B] = float(wd_b)
        v[LOST_A] = [a0[0][i] - a[0][i] for i in range(len(CLASSES))]
        v[LOST_B] = [b0[0][i] - b[0][i] for i in range(len(CLASSES))]
        v[VIABLE_A] = float(2 * a[0][self.tt] >= a0[0][self.tt])
        v[VIABLE_B] = float(2 * b[0][self.tt] >= b0[0][self.tt])
        return v

    def solve(self, a0, b0, roe=(10, 10), limit=100_000):
        """Exact expectations over every roll path, round by round up to the
        round cap, past which `step` ends an undecided battle as a
        stalemate. Raises TooLarge past `limit` distinct (state, round)
        pairs. The round is part of the key, so no state leads back to
        itself: a state that cannot change is followed once per remaining
        round and reaches the cap like any other."""
        memo = {}
        exchanges = {}
        sys.setrecursionlimit(max(sys.getrecursionlimit(), 50_000))

        def exchange(a, b, pressed=False, expose_a=False, expose_b=False):
            key = (a, b, pressed, expose_a, expose_b)
            if key not in exchanges:
                exchanges[key] = self.exchange(a, b, pressed, expose_a, expose_b)
            return exchanges[key]

        def value(a, b, r):
            key = (a, b, r)
            if key in memo:
                return memo[key]
            if len(memo) >= limit:
                raise TooLarge(len(memo))
            total = [0.0] * VECTOR
            for p, fought, nxt in self.step(a, b, r, roe, exchange):
                total[ROUNDS] += p * fought
                if nxt[0] == 'terminal':
                    for i, x in enumerate(self.terminal(a0, b0, *nxt[1:])):
                        total[i] += p * x
                else:
                    for i, x in enumerate(value(*nxt[1:])):
                        total[i] += p * x
            memo[key] = total
            return total

        result = value(a0, b0, 1)
        return result, len(memo)

    def simulate(self, a0, b0, rng, roe=(10, 10)):
        """One sampled battle: the terminal value vector."""
        a, b, r = a0, b0, 1
        rounds = 0
        exchange = lambda a, b, **kw: self.exchange_sampled(rng, a, b, **kw)
        while True:
            fought, nxt = _draw(rng, [(p, (fought, nxt)) for p, fought, nxt in self.step(a, b, r, roe, exchange)])
            rounds += fought
            if nxt[0] == 'terminal':
                v = self.terminal(a0, b0, *nxt[1:])
                v[ROUNDS] = float(rounds)
                return v
            _, a, b, r = nxt


def _draw(rng, branches):
    if len(branches) == 1:
        return branches[0][1]
    u = rng.random()
    acc = 0.0
    for p, state in branches:
        acc += p
        if u < acc:
            return state
    return branches[-1][1]
