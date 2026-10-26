"""Fine-tune the fleet-level agent by PPO (t157, steps 4-6).

Learner seats are hybrids: the network sets the goals and orders the fleets
of the handed-over families (``--families``); the executor orders the
other fleets and builds. Each game seats one or two learners against the
frozen final strategist, the executor and Aggressor. The objective covers
the goal heads and the fleet heads of handed-over fleets. A KL penalty
toward the imitation policy starts at ``--kl`` and fades over
``--kl-iterations`` to ``--kl-floor``, so a floor keeps the policy from
drifting far from imitation once the fade ends.

Hand-over (step 5) runs one family set at a time:
``--families colonize,scout`` first, then ``colonize,scout,strike``. A set
is kept only when the learner beats the final strategist head to head; see
``head_to_head``.

    uv run fleet_ppo.py --init /tmp/nc-ai/fleet/imitate.pt \\
        --strategist strategist-final.pt --families colonize,scout --out /tmp/nc-ai/fleet/ppo-a

With ``--league`` (t184), each game seats the learner against three copies
of one opponent drawn from a league: the scripted styles of
``--league-styles``, the frozen agents of ``--league-agents`` (past
snapshots, exploiters), and the learner's own last ``--league-self``
snapshots, each added as it is saved. An opponent is drawn in proportion to
how far the learner is from beating it (prioritized fictitious self-play,
as AlphaStar did): weight ``1 - min(1, 2s) + 0.1``, where ``s`` is the
learner's running win share against it (0.25 even, 0.5 or more beaten).

``--free-heads join,unload`` (t191) lets fleet heads that imitation never
saw used leave their start: they are left out of the KL penalty, and the
entropy of whether they act at all earns ``--free-entropy`` (see
``act_entropy``). That bonus peaks at acting half the time whatever acting
wins, so ``--free-entropy-until N`` fades it to nothing by iteration ``N``
and the rest of the run keeps the orders only if they pay. Imitation labels
these orders "none" until a head almost never acts, where the bonus has no
gradient; ``--reset-new-heads`` puts the join and unload heads back at their
start (see ``FleetAgent.reset_new_heads``).

``--teach-ferry W`` (t191) adds a teacher term: on the learner's transport
fleets without warships, the log-likelihood of the executor's mission,
target, load and unload (``Env.fleet_labels``), weighted ``W`` and faded to
nothing by iteration ``--teach-until``. The executor ferries spare armies
to threatened colonies; the term shows the learner the orders, and PPO
keeps them only if they pay.

``--mission-smooth E`` and ``--mission-bonus C`` (t207) open the mission
head to orders imitation taught away (Blitz, Salvage, Patrol). The KL on
the mission head is taken against the reference blended with ``E`` of an
even spread over the row's legal missions (``smoothed_kl``), so trying a
mission the copy never chose no longer costs more than the reward can
repay. The coverage bonus ``C`` raises each legal mission toward
``--mission-floor`` and no further (``coverage``), so it cannot flatten the
head as an entropy bonus would; ``--mission-until N`` fades it to nothing,
and a mission kept after that is kept because it wins.

``--build-smooth E`` (t213) takes the build head's KL against the
reference blended with ``E`` of an even spread over its legal builds, as
``--mission-smooth`` does for missions, so a build the executor never
makes no longer costs more than the reward can repay. ``--split-clip``
gives the build heads their own PPO ratio and clip, so the many fleet and
goal choices in a seat-year do not use up the builds' trust region.

``--anchor-ema H`` (t214) moves the KL reference after each iteration:
its weights become ``(1 - H)`` of themselves plus ``H`` of the policy's
(``ema_toward``), a trailing magnet as in Magnetic Mirror Descent. The KL
then charges for change since recent iterations, not for distance from
the imitation copy, so the policy can wander far while each step stays
close.

``--shape-lead W`` (t217) adds potential-based shaping (Ng 1999): each
year the learner earns ``W * (gamma * phi(next) - phi(now))``, where
``phi`` is its share of all present production minus its rivals' mean
share (``lead_potential``), and 0 before the first year, at game end and
once the seat owns no world. The terms telescope over a game to a constant,
so the strategy that wins most is unchanged; a lead shows up within years
instead of only in the final result. Logged win shares stay unshaped.

``--upgo C`` (t215) adds self-imitation in the form of AlphaStar's UPGO:
each seat-year's upgoing return bootstraps from the critic's next value
unless the game's actual continuation did better (``upgo_advantages``), and
the positive part of that return over the critic's value joins the policy's
advantage with weight ``C`` (in units of the GAE advantage's spread). Good
choices that won more than the critic expected are pushed harder; the value
target is unchanged.

Each iteration saves ``resume.pt`` in ``--out``: the weights, the optimizer,
the random states, the next training map and the league's shares.
``--resume``, with the run's other options unchanged, continues a run that
stopped from the iteration after the last one saved. Games sampled on the
GPU draw from its own generator, which is not saved, so a resumed run is
not bitwise the run that would have gone on.
"""

import argparse
import json
import time
from pathlib import Path

import numpy as np
import torch

import nc_ai_env
import policy
from fleet_imitate import actions_for, frozen_goals
from fleet_policy import (BUILD_HEADS, FLEET_HEADS, FleetAgent, bucket, trim_rows, build_arrays, build_logp, fleet_arrays, fleet_batch,
                          load_fleet_agent)
from policy import HEADS, device, goals_for, save
from ppo import advantages, pad_cat

PLAYERS = 4
FAMILIES = ("colonize", "scout", "strike", "convoy")
# Observation fleet-row columns: ship counts start at 10, in the order
# battleships, cruisers, destroyers, empty and loaded transports, ETACs,
# scouts.
BB, CA, DD, TT, TTL, ETAC, SC = range(10, 17)


def fleet_families(fleets):
    """Each fleet row's family, as the Rust `fleet_family`: 0 colonize (has
    ETACs), 1 scout (scouts only, or one or two lone destroyers), 3 convoy
    (transports without warships), 2 strike (the rest)."""
    count = lambda column: fleets[..., column]
    troops = count(TT) + count(TTL)
    combat = count(BB) + count(CA) + count(DD)
    picket = ((count(DD) >= 1) & (count(DD) <= 2) & (count(BB) == 0) & (count(CA) == 0)
              & (count(SC) == 0) & (count(ETAC) == 0) & (troops == 0))
    scout = (combat == 0) & (count(SC) > 0) & (troops == 0)
    convoy = (troops > 0) & (combat == 0)
    return np.where(count(ETAC) > 0, 0, np.where(picket | scout, 1, np.where(convoy, 3, 2)))


def handed_rows(obs, select, families, rows):
    """``[N, rows]`` bools: the selected seats' real fleet rows whose family
    is handed over, padded to ``rows``."""
    family = fleet_families(obs["fleets"][select])
    handed = (np.isin(family, [FAMILIES.index(f) for f in families if f in FAMILIES])
              & obs["fleets_valid"][select])
    return np.pad(handed, ((0, 0), (0, rows - handed.shape[1])))


# The mission head's choices: 0 keeps the fleet's orders, then the Rust
# `nc_ai::MISSIONS` in order.
MISSION_NAMES = ("keep", "hold", "move", "seek_home", "patrol", "guard_blockade", "bombard",
                 "invade", "blitz", "view", "scout_sector", "scout_system", "colonize",
                 "rendezvous", "salvage", "guard_starbase")


def mission_shares(steps, handed_rows_n):
    """Each mission's share of the handed-over rows' sampled choices, to
    three places; missions never sampled are left out (t207)."""
    counts = np.zeros(len(MISSION_NAMES))
    for s in steps:
        handed = s["handed"].numpy().astype(bool)
        chosen = s["fleets"]["mission"][:len(handed), :handed.shape[1]].numpy()[handed]
        counts += np.bincount(chosen, minlength=len(MISSION_NAMES))[:len(MISSION_NAMES)]
    return {name: round(float(n) / handed_rows_n, 3)
            for name, n in zip(MISSION_NAMES, counts) if n}


# The fleet heads `--teach-ferry` teaches.
TAUGHT = ("mission", "target", "load", "unload")


def convoy_rows(obs, select, rows):
    """``[N, rows]`` bools: the selected seats' real fleet rows of
    transports without warships, padded to ``rows``."""
    fleets = obs["fleets"][select]
    count = lambda column: fleets[..., column]
    convoy = ((count(TT) + count(TTL) > 0) & (count(BB) + count(CA) + count(DD) == 0)
              & obs["fleets_valid"][select])
    return np.pad(convoy, ((0, 0), (0, rows - convoy.shape[1])))


def teacher_loss(dists, choices, labels, convoy):
    """Mean negative log-likelihood of the executor's orders on convoy rows.
    The target counts only where the taken mission is the label's, since the
    target head is conditioned on the taken mission."""
    total = torch.zeros((), device=convoy.device)
    for name in TAUGHT:
        rows = convoy if name != "target" else convoy & (choices["mission"] == labels["mission"])
        lp = dists[name].log_prob(labels[name])
        lp = torch.where(torch.isfinite(lp), lp, torch.zeros_like(lp))
        total = total - (lp * rows).sum()
    return total / convoy.sum().clamp(min=1)


class League:
    """Training opponents and the learner's running win share against each
    (t184); see the module docs. Frozen agents play role ``3 + k``, ``k``
    their index in :attr:`agents`."""

    PRIOR, RATE = 0.25, 0.1

    def __init__(self, styles, agents, frozen_seat, keep_self):
        self.frozen_seat, self.keep_self = frozen_seat, keep_self
        self.entries = [{"label": style, "seat": style, "model": None, "self": False}
                        for style in styles]
        for label, model in agents:
            self.entries.append({"label": label, "seat": frozen_seat, "model": model,
                                 "self": False})
        for entry in self.entries:
            entry["share"] = self.PRIOR

    def add_self(self, label, model, path):
        """Add the learner's snapshot saved at ``path``, dropping the oldest
        beyond ``keep_self``."""
        self.entries.append({"label": label, "seat": self.frozen_seat, "model": model,
                             "self": True, "share": self.PRIOR, "path": str(path)})
        own = [e for e in self.entries if e["self"]]
        for entry in own[:max(0, len(own) - self.keep_self)]:
            self.entries.remove(entry)

    @property
    def agents(self):
        return [e["model"] for e in self.entries if e["model"] is not None]

    def role(self, idx):
        entry = self.entries[idx]
        if entry["model"] is not None:
            return 3 + [e for e in self.entries if e["model"] is not None].index(entry)
        return 2 if entry["seat"] == "strategist" else 0

    def draw(self, rng):
        shares = np.array([e["share"] for e in self.entries])
        weights = 1 - np.minimum(1, 2 * shares) + 0.1
        return int(rng.choice(len(self.entries), p=weights / weights.sum()))

    def update(self, opponents, rewards):
        """Fold one iteration's games in: ``opponents[g]`` the entry game
        ``g`` drew, ``rewards[g]`` the learner's mean win share in it."""
        for idx, entry in enumerate(self.entries):
            mine = rewards[opponents == idx]
            if len(mine):
                entry["share"] += self.RATE * (mine.mean() - entry["share"])

    def summary(self):
        return {e["label"]: round(float(e["share"]), 3) for e in self.entries}

    def state(self):
        return [{"label": e["label"], "share": float(e["share"]), "path": e.get("path")}
                for e in self.entries]

    def restore(self, saved, load):
        """Return to a saved :meth:`state`: the learner's snapshots then in
        the league, loaded by ``load``, and every running share."""
        for entry in saved:
            if entry["path"]:
                self.add_self(entry["label"], load(entry["path"]), entry["path"])
        shares = {e["label"]: e["share"] for e in saved}
        for entry in self.entries:
            entry["share"] = shares[entry["label"]]


def choose_lineups(rng, games, learner, target=None, league=None):
    """Training lineups. Roles: 1 learner, 2 strategist, 3 the frozen target
    agent an exploiter trains against (``target`` its seat name), 0
    scripted. An exploiter meets only the target. With a ``league``, also
    returns each game's opponent entry."""
    names, roles = [], np.zeros((games, PLAYERS), np.int8)
    opponents = np.zeros(games, np.int64)
    for game in range(games):
        if league is not None:
            pick = league.draw(rng)
            opponents[game] = pick
            learners = 1 + (rng.random() < 0.3)
            lineup = [learner] * learners + [league.entries[pick]["seat"]] * (PLAYERS - learners)
            role = [1] * learners + [league.role(pick)] * (PLAYERS - learners)
            order = rng.permutation(PLAYERS)
            names.append([lineup[i] for i in order])
            roles[game] = np.array(role)[order]
            continue
        if target is not None:
            order = rng.permutation(PLAYERS)
            lineup, role = [learner] + [target] * 3, [1, 3, 3, 3]
            names.append([lineup[i] for i in order])
            roles[len(names) - 1] = np.array(role)[order]
            continue
        learners = 1 + (rng.random() < 0.3)
        lineup, role = [], []
        for seat in range(PLAYERS):
            if seat < learners:
                lineup.append(learner)
                role.append(1)
                continue
            pick = rng.random()
            if pick < 0.5:
                lineup.append("strategist")
                role.append(2)
            elif pick < 0.9:
                lineup.append("executor")
                role.append(0)
            else:
                lineup.append("aggressor")
                role.append(0)
        order = rng.permutation(PLAYERS)
        names.append([lineup[i] for i in order])
        roles[len(names) - 1] = np.array(role)[order]
    return (names, roles, opponents) if league is not None else (names, roles)


def play_year(env, model, strategist, roles, done, families, dev, greedy=False, target=None,
              frozen=None, teach=False):
    """Choose every learner's and strategist's goals and the learners'
    fleet orders, and the frozen agents' (greedy, as deployed) for their
    seats, and step the batch: ``frozen[k]`` plays role ``3 + k``, and
    ``target`` alone stands for ``frozen=[target]``. Returns the step result, for
    training the learners' samples, and the seconds spent in the games
    themselves (observing and stepping), apart from the networks."""
    live = ~done[:, None]
    learner = (roles == 1) & live
    others = (roles == 2) & live
    frozen = frozen if frozen is not None else ([target] if target is not None else [])
    started = time.perf_counter()
    obs, masks, goal_masks = env.observe(), env.masks(), env.goal_masks()
    env_goals = {name: np.zeros_like(value) for name, value in env.goal_labels().items()}
    env_s = time.perf_counter() - started
    sample = None
    with torch.no_grad():
        if learner.any():
            batch = bucket(fleet_batch(obs, masks, goal_masks, learner, dev))
            goals, fleets, goal_lp, fleet_lp, _, value, goal_dists, _ = model.act(
                batch, greedy=greedy, builds="builds" in families)
            handed = torch.from_numpy(
                handed_rows(obs, learner, families, batch["fleets"].shape[1])).to(dev)
            logp = goal_lp + (fleet_lp * handed).sum(-1)
            chosen = goals_for(goals, learner)
            for name in HEADS:
                env_goals[name][learner] = chosen[name][learner]
            fleet_actions = fleet_arrays(fleets, learner, env.fleets)
            if "builds" in families:
                fleet_actions.update(build_arrays(goals, learner, env.worlds))
            n = batch["n"]
            sample = {
                "select": learner,
                "batch": {k: v[:n].cpu() for k, v in batch.items() if k != "n"},
                "goals": {k: v.cpu() for k, v in goals.items()},
                "fleets": {k: v.cpu() for k, v in fleets.items()},
                "handed": handed.cpu(),
                "logp": logp.cpu(),
                "build_logp": (build_logp(goal_dists, goals) if "builds" in families
                               else torch.zeros_like(logp)).cpu(),
                "value": value.cpu(),
            }
        else:
            fleet_actions = {name: np.zeros((*learner.shape, env.fleets), np.int64)
                             for name in FLEET_HEADS}
        if others.any():
            theirs = frozen_goals(strategist, obs, masks, goal_masks, others, dev)
            for name in HEADS:
                env_goals[name][others] = theirs[name][others]
        for k, agent in enumerate(frozen):
            seats = (roles == 3 + k) & live
            if not seats.any():
                continue
            batch = bucket(fleet_batch(obs, masks, goal_masks, seats, dev))
            goals, fleets, *_ = agent.act(batch, greedy=True, builds="builds" in families)
            chosen = goals_for(goals, seats)
            for name in HEADS:
                env_goals[name][seats] = chosen[name][seats]
            placed = fleet_arrays(fleets, seats, env.fleets)
            if "builds" in families:
                # The exploit target builds (seats that do not hand builds over ignore these).
                placed.update(build_arrays(goals, seats, env.worlds))
            for name, value in placed.items():
                fleet_actions.setdefault(name, np.zeros_like(value))[seats] = value[seats]
    started = time.perf_counter()
    if teach and sample is not None:
        rows = sample["handed"].shape[1]
        labels = env.fleet_labels(env_goals)
        sample["labels"] = {name: torch.from_numpy(np.pad(
            labels[name][learner], ((0, 0), (0, rows - env.fleets)))) for name in TAUGHT}
        sample["convoy"] = torch.from_numpy(convoy_rows(obs, learner, rows))
    actions = actions_for(env, {k: v for k, v in fleet_actions.items() if k in FLEET_HEADS})
    actions.update({k: v for k, v in fleet_actions.items() if k in BUILD_HEADS})
    step = env.step(actions, env_goals)
    return step, sample, env_s + time.perf_counter() - started


def learner_name(families, style=None):
    """The env seat name of a hybrid learner whose executor plays
    ``style`` (a personality, t167), the default when ``None``."""
    return "hybrid:" + ",".join(families) + (f"@{style}" if style else "")


def frozen_name(families):
    """The env seat name of a frozen league agent: the learner's fleet
    families, builds included. An agent saved before the build head gets it
    at ``BUILD_EXECUTOR``, which builds as the executor would (t207)."""
    return learner_name(families)


# Evaluation maps during training. Final confirmations use 100000 and up, so
# no checkpoint is picked on the maps it is confirmed on.
EVAL_MAPS = 5_000_000


def head_to_head(model, strategist, families, opponent, games, years, first_map, dev,
                 greedy=False, style=None, war_years=None, target=None, threads=0):
    """Win share of one learner seat against three ``opponent`` seats
    (``strategist``, ``target`` for the frozen ``target`` agent, or a
    scripted policy), the learner's seat rotated. With
    ``war_years`` (a list), each game's first year the learner set a war
    goal, counted from the start, is appended, or ``None`` for never."""
    env = nc_ai_env.Env(PLAYERS, years, threads)
    learner = learner_name(families, style)
    names, roles = [], np.zeros((games, PLAYERS), np.int8)
    other = learner_name(families) if opponent == "target" else opponent
    other_role = {"strategist": 2, "target": 3}.get(opponent, 0)
    for game in range(games):
        lineup = [other] * PLAYERS
        lineup[game % PLAYERS] = learner
        names.append(lineup)
        roles[game] = [1 if seat == game % PLAYERS else other_role for seat in range(PLAYERS)]
    env.reset(list(range(first_map, first_map + games)), seats=names)
    done = np.zeros(games, bool)
    wins = np.zeros(games)
    first_war = [None] * games
    year = 0
    while not done.all():
        live = np.flatnonzero(~done)
        step, sample, _ = play_year(env, model, strategist, roles, done, families, dev,
                                    greedy=greedy, target=target)
        if sample is not None:
            war = (sample["goals"]["diplomacy"] == 2).any(-1).numpy()
            for game, at_war in zip(live, war):
                if at_war and first_war[game] is None:
                    first_war[game] = year
        wins += step["reward"][np.arange(games), np.arange(games) % PLAYERS]
        done = step["done"].copy()
        year += 1
    if war_years is not None:
        war_years.extend(first_war)
    return wins.mean()


def categorical_kl(p, q):
    """KL(p || q) over the last axis, counting only choices p can take, so
    masked (-inf) choices contribute nothing rather than NaN."""
    can = p.probs > 0
    zero = torch.zeros_like(p.logits)
    # Zero the masked logits before subtracting: -inf - -inf would be NaN,
    # and NaN in an untaken torch.where branch still poisons the gradient.
    diff = torch.where(can, p.logits, zero) - torch.where(can, q.logits, zero)
    return (p.probs * diff).sum(-1)


def smoothed_kl(p, q, smooth):
    """KL(p || q'), where q' is ``q`` blended with ``smooth`` of an even
    spread over the choices ``p`` can take (t207): every legal choice keeps
    at least ``smooth / legal`` in the reference."""
    can = p.probs > 0
    legal = can.sum(-1, keepdim=True).clamp_min(1)
    blended = (1 - smooth) * q.probs + smooth * can / legal
    zero = torch.zeros_like(p.logits)
    log_ratio = torch.where(can, p.logits - blended.clamp_min(1e-12).log(), zero)
    return (p.probs * log_ratio).sum(-1)


def coverage(dist, floor):
    """Each legal choice's probability, counted up to ``floor`` (t207). Its
    gradient is zero for a choice already above ``floor``."""
    return (torch.minimum(dist.probs, torch.full_like(dist.probs, floor))
            * (dist.probs > 0)).sum(-1)


def kl_to(new, old, handed, free=(), mission_smooth=0.0, build_smooth=0.0):
    """KL(new || old) per seat: the goal heads and any build heads, plus the
    fleet heads of the handed-over rows, except the ``free`` ones; the
    mission head against a smoothed reference when ``mission_smooth`` is
    set, and the build head when ``build_smooth`` is (t213)."""
    total = 0
    for name in HEADS:
        kl = categorical_kl(new[0][name], old[0][name])
        total = total + (kl.sum(-1) if kl.ndim > 1 else kl)
    for name in BUILD_HEADS:
        if name in new[0] and name not in free:
            kl = (smoothed_kl(new[0][name], old[0][name], build_smooth)
                  if name == "build" and build_smooth > 0
                  else categorical_kl(new[0][name], old[0][name]))
            total = total + kl.sum((1, 2))
    for name in FLEET_HEADS:
        if name in free:
            continue
        if name == "mission" and mission_smooth > 0:
            kl = smoothed_kl(new[1][name], old[1][name], mission_smooth)
        else:
            kl = categorical_kl(new[1][name], old[1][name])
        total = total + (kl * handed).sum(-1)
    return total


def act_entropy(dist, rest=0):
    """The entropy of whether a head acts at all: its resting choice
    ``rest`` (none; for builds, as the executor would) against all the
    others together. A bonus on the whole distribution
    pushes a head with many targets toward a uniform choice among them, so
    it nearly always acts: in the t191 pilot, joins rose to 97% of fleet
    orders and the learner stopped winning."""
    none = dist.probs[..., rest].clamp(1e-6, 1 - 1e-6)
    return -(none * none.log() + (1 - none) * (1 - none).log())


@torch.no_grad()
def upgo_advantages(steps, gamma):
    """Self-imitation advantages, laid out as ``advantages``' (t215). The
    upgoing return is ``G_t = r_t + gamma * max(V_{t+1}, G_{t+1})``, cut at
    the game's end; the advantage is ``max(0, G_t - V_t)``."""
    games, seats = steps[0]["select"].shape
    g_next = np.zeros((games, seats), np.float32)
    value_next = np.zeros((games, seats), np.float32)
    out = [None] * len(steps)
    for t in reversed(range(len(steps))):
        s = steps[t]
        sel = s["select"]
        value = s["value"].numpy()
        ended = s["done"].astype(np.float32)
        g = s["reward"] + gamma * (1 - ended) * np.maximum(value_next[sel], g_next[sel])
        g_next[sel] = g
        value_next[sel] = value
        out[t] = np.maximum(0.0, g - value)
    return out


def lead_potential(production, worlds, done):
    """``[games, seats]``: each seat's share of its game's present production
    minus its rivals' mean share, and 0 for a seat without worlds or in a
    game that has ended, so shaping sums to a constant (t217)."""
    production = production.astype(np.float64)
    total = production.sum(1, keepdims=True)
    share = np.divide(production, total, out=np.zeros_like(production), where=total > 0)
    seats = production.shape[1]
    lead = share - (1 - share) / (seats - 1)
    lead[(worlds == 0) | done[:, None]] = 0.0
    return lead.astype(np.float32)


def ema_toward(ref, model, share):
    """Move every weight of ``ref`` ``share`` of the way toward ``model``'s."""
    for r, m in zip(ref.state_dict().values(), model.state_dict().values()):
        if r.is_floating_point():
            r.lerp_(m, share)


def update(model, ref, optimizer, steps, advs, kl_coef, free_coef, teach_coef, mission_coef,
           args, dev, upgo=None):
    keys = steps[0]["batch"].keys()
    batch = {k: pad_cat([s["batch"][k] for s in steps]) for k in keys}
    rows = batch["fleets"].shape[1]
    pad_rows = lambda t: torch.nn.functional.pad(t, (0, rows - t.shape[1]))
    goals = {k: torch.cat([s["goals"][k] for s in steps]) for k in steps[0]["goals"]}
    builds = "build" in goals
    fleets = {k: torch.cat([pad_rows(s["fleets"][k][:, :rows]) for s in steps])
              for k in FLEET_HEADS}
    handed = torch.cat([pad_rows(s["handed"][:, :rows]) for s in steps])
    teach = teach_coef > 0 and "labels" in steps[0]
    if teach:
        labels = {k: torch.cat([pad_rows(s["labels"][k][:, :rows]) for s in steps])
                  for k in TAUGHT}
        convoy = torch.cat([pad_rows(s["convoy"][:, :rows]) for s in steps])
    old_logp = torch.cat([s["logp"] for s in steps])
    old_build_logp = torch.cat([s["build_logp"] for s in steps])
    old_value = torch.cat([s["value"] for s in steps])
    adv = torch.from_numpy(np.concatenate(advs))
    ret = adv + old_value
    spread = adv.std() + 1e-8
    adv = (adv - adv.mean()) / spread
    if upgo is not None:
        adv = adv + args.upgo * torch.from_numpy(np.concatenate(upgo)) / spread
    stats = []
    model.train()
    for _ in range(args.epochs):
        for idx in torch.randperm(len(adv)).split(args.minibatch):
            mb = trim_rows({k: v[idx].to(dev) for k, v in batch.items()})
            # Rows past the trim take choice 0, the only one their padded masks allow.
            cut, mb = mb["fleets"].shape[1], bucket(mb)
            fit = lambda t: torch.nn.functional.pad(t[:, :cut], (0, mb["fleets"].shape[1] - cut)).to(dev)
            mb_fleets = {k: fit(v[idx]) for k, v in fleets.items()}
            mb_handed = fit(handed[idx])
            mb_goals = {k: v[idx].to(dev) for k, v in goals.items()}
            _, _, goal_lp, fleet_lp, entropy, value, goal_dists, fleet_dists = model.act(
                mb, fleet_choices=mb_fleets, goal_choices=mb_goals, builds=builds)
            with torch.no_grad():
                *_, ref_goal_dists, ref_fleet_dists = ref.act(
                    mb, fleet_choices=mb_fleets, goal_choices=mb_goals, builds=builds)
            logp = goal_lp + (fleet_lp * mb_handed).sum(-1)
            a = adv[idx].to(dev)
            clipped = lambda ratio: -torch.min(
                ratio * a, ratio.clamp(1 - args.clip, 1 + args.clip) * a).mean()
            if args.split_clip and builds:
                # The build heads take their own ratio and clip (t213), so
                # the many fleet and goal choices do not use up their trust
                # region. Both parts share the seat's advantage, so
                # normalizing it per part changes nothing.
                build_lp = build_logp(goal_dists, mb_goals)
                old_build = old_build_logp[idx].to(dev)
                policy_loss = (clipped(torch.exp(build_lp - old_build))
                               + clipped(torch.exp(logp - build_lp - (old_logp[idx].to(dev) - old_build))))
            else:
                policy_loss = clipped(torch.exp(logp - old_logp[idx].to(dev)))
            value_loss = (value - ret[idx].to(dev)).pow(2).mean()
            kl_ref = kl_to((goal_dists, fleet_dists), (ref_goal_dists, ref_fleet_dists),
                           mb_handed, args.free_heads, args.mission_smooth, args.build_smooth).mean()
            ent = (entropy * mb_handed).sum(-1).mean() + policy.joint_entropy(goal_dists).mean()
            free_ent = sum(((act_entropy(fleet_dists[name]) * mb_handed).sum(-1).mean()
                            for name in args.free_heads if name in FLEET_HEADS),
                           torch.zeros((), device=dev))
            if builds and "build" in args.free_heads:
                free_ent = free_ent + act_entropy(goal_dists["build"], nc_ai_env.BUILD_EXECUTOR).sum((1, 2)).mean()
            loss = (policy_loss + args.value_coef * value_loss + kl_coef * kl_ref
                    - args.entropy_coef * ent - free_coef * free_ent)
            if mission_coef > 0:
                covered = (coverage(fleet_dists["mission"], args.mission_floor)
                           * mb_handed).sum(-1).mean()
                loss = loss - mission_coef * covered
            if teach:
                mb_labels = {k: fit(v[idx]) for k, v in labels.items()}
                mb_convoy = fit(convoy[idx])
                loss = loss + teach_coef * teacher_loss(fleet_dists, mb_fleets, mb_labels,
                                                        mb_convoy & mb_handed.bool())
            optimizer.zero_grad()
            loss.backward()
            torch.nn.utils.clip_grad_norm_(model.parameters(), 0.5)
            optimizer.step()
            stats.append((policy_loss.item(), value_loss.item(), kl_ref.item(), ent.item()))
    if dev.type == "mps":
        torch.mps.empty_cache()
    return np.mean(stats, axis=0), len(adv)


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--init", required=True, help="fleet imitation checkpoint")
    parser.add_argument("--strategist", required=True, help="frozen opponent checkpoint")
    parser.add_argument("--families", default="colonize,scout")
    parser.add_argument("--out", required=True)
    parser.add_argument("--iterations", type=int, default=200)
    parser.add_argument("--games", type=int, default=128)
    parser.add_argument("--years", type=int, default=60)
    parser.add_argument("--lr", type=float, default=3e-5)
    parser.add_argument("--epochs", type=int, default=3)
    parser.add_argument("--minibatch", type=int, default=512)
    parser.add_argument("--clip", type=float, default=0.2)
    parser.add_argument("--value-coef", type=float, default=0.5)
    parser.add_argument("--entropy-coef", type=float, default=0.001)
    parser.add_argument("--kl", type=float, default=0.1)
    parser.add_argument("--kl-iterations", type=int, default=100)
    parser.add_argument("--kl-floor", type=float, default=0.0,
                        help="the penalty never fades below this")
    parser.add_argument("--gamma", type=float, default=0.995)
    parser.add_argument("--lam", type=float, default=0.95)
    parser.add_argument("--eval-every", type=int, default=20)
    parser.add_argument("--eval-games", type=int, default=256)
    parser.add_argument("--seed", type=int, default=1)
    parser.add_argument("--style", help="the learner's executor personality (t167)")
    parser.add_argument("--threads", type=int, default=0,
                        help="CPU threads for the games; 0 uses every core")
    parser.add_argument("--exploit", help="train a main exploiter against this frozen "
                                          "fleet agent: every game is the learner against "
                                          "three copies of it")
    parser.add_argument("--free-heads", type=lambda text: [h for h in text.split(",") if h],
                        default=[], help="fleet heads left out of the KL penalty (t191)")
    parser.add_argument("--free-entropy", type=float, default=0.0,
                        help="extra entropy weight for the free heads")
    parser.add_argument("--free-entropy-until", type=int, default=0,
                        help="fade --free-entropy to 0 by this iteration; 0 never fades it")
    parser.add_argument("--teach-ferry", type=float, default=0.0,
                        help="weight of the executor's orders on transport fleets (t191)")
    parser.add_argument("--teach-until", type=int, default=0,
                        help="fade --teach-ferry to 0 by this iteration; 0 never fades it")
    parser.add_argument("--build-smooth", type=float, default=0.0,
                        help="blend the build head's KL reference with this much of an even "
                             "spread over legal builds (t213)")
    parser.add_argument("--anchor-ema", type=float, default=0.0,
                        help="move the KL reference this share toward the policy after each "
                             "iteration (t214); 0 keeps it fixed")
    parser.add_argument("--split-clip", action="store_true",
                        help="give the build heads their own PPO ratio and clip (t213)")
    parser.add_argument("--mission-smooth", type=float, default=0.0,
                        help="blend the mission head's KL reference with this much of an "
                             "even spread over legal missions (t207)")
    parser.add_argument("--mission-bonus", type=float, default=0.0,
                        help="weight of the coverage bonus on legal missions (t207)")
    parser.add_argument("--mission-floor", type=float, default=0.03,
                        help="the coverage bonus lifts each legal mission to this share")
    parser.add_argument("--mission-until", type=int, default=0,
                        help="fade --mission-bonus to 0 by this iteration; 0 never fades it")
    parser.add_argument("--reset-new-heads", action="store_true",
                        help="start the join and unload heads at none again (t191)")
    parser.add_argument("--league", action="store_true",
                        help="draw each game's opponent from a league (t184)")
    parser.add_argument("--league-styles",
                        default="strategist,executor,executor@rusher,executor@turtle,"
                                "executor@armada,executor@raider,aggressor",
                        help="the league's scripted opponents, as env seat names")
    parser.add_argument("--league-agents", nargs="*", default=[],
                        help="frozen fleet-agent checkpoints in the league")
    parser.add_argument("--league-self", type=int, default=3,
                        help="the learner's latest snapshots kept in the league")
    parser.add_argument("--resume", action="store_true",
                        help="continue the run saved in --out")
    parser.add_argument("--upgo", type=float, default=0.0,
                        help="weight of UPGO self-imitation in the policy advantage (t215)")
    parser.add_argument("--shape-lead", type=float, default=0.0,
                        help="weight of potential-based shaping on the production lead (t217)")
    parser.add_argument("--limit-credit", type=float, default=1.0,
                        help="reward for a production lead at the year limit, against 1 "
                             "for sweeping the map")
    args = parser.parse_args()
    if unknown := set(args.free_heads) - set(FLEET_HEADS) - set(BUILD_HEADS):
        parser.error(f"unknown fleet heads {sorted(unknown)}")
    if args.threads:
        torch.set_num_threads(args.threads)

    families = [f for f in args.families.split(",") if f]
    dev = device()
    rng = np.random.default_rng(args.seed)
    torch.manual_seed(args.seed)
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    model = load_fleet_agent(args.init, dev)
    ref = load_fleet_agent(args.init, dev)
    if args.reset_new_heads:
        model.reset_new_heads()
        ref.reset_new_heads()
    strategist = policy.load(args.strategist, dev)
    # An exploiter trains against a frozen agent only (main exploiter).
    target = load_fleet_agent(args.exploit, dev) if args.exploit else None
    league = None
    if args.league:
        agents = [(f"{Path(path).parent.name}/{Path(path).stem}", load_fleet_agent(path, dev))
                  for path in args.league_agents]
        league = League([s for s in args.league_styles.split(",") if s], agents,
                        frozen_name(families), args.league_self)
    optimizer = torch.optim.AdamW(model.parameters(), lr=args.lr)
    env = nc_ai_env.Env(PLAYERS, args.years, args.threads, limit_credit=args.limit_credit)
    log = open(out / "log.jsonl", "a")
    learner = learner_name(families, args.style)
    next_map, first = 3_000_000, 1
    if args.resume:
        saved = torch.load(out / "resume.pt", map_location=dev, weights_only=False)
        model.load_state_dict(saved["model"])
        optimizer.load_state_dict(saved["optimizer"])
        rng.bit_generator.state = saved["rng"]
        torch.set_rng_state(saved["torch_rng"].cpu())
        next_map, first = saved["next_map"], saved["iteration"] + 1
        if saved.get("ref"):
            ref.load_state_dict(saved["ref"])
        if league:
            league.restore(saved["league"], lambda path: load_fleet_agent(path, dev))
        print(f"resuming at iteration {first}", flush=True)

    for it in range(first, args.iterations + 1):
        started = time.perf_counter()
        kl_coef = max(args.kl_floor, args.kl * max(0.0, 1 - (it - 1) / args.kl_iterations))
        free_coef = args.free_entropy * (max(0.0, 1 - (it - 1) / args.free_entropy_until)
                                         if args.free_entropy_until else 1.0)
        teach_coef = args.teach_ferry * (max(0.0, 1 - (it - 1) / args.teach_until)
                                         if args.teach_until else 1.0)
        mission_coef = args.mission_bonus * (max(0.0, 1 - (it - 1) / args.mission_until)
                                             if args.mission_until else 1.0)
        drawn = choose_lineups(rng, args.games, learner,
                               learner_name(families) if target else None, league)
        names, roles = drawn[:2]
        env.reset(list(range(next_map, next_map + args.games)), seats=names)
        next_map += args.games
        model.eval()
        steps, done, env_s = [], np.zeros(args.games, bool), 0.0
        # Each game's learner win share, averaged over its learner seats.
        game_reward = np.zeros(args.games)
        phi = np.zeros((args.games, PLAYERS), np.float32)
        while not done.all():
            step, sample, seconds = play_year(env, model, strategist, roles, done, families, dev,
                                              teach=teach_coef > 0,
                                              target=target,
                                              frozen=league.agents if league else None)
            env_s += seconds
            game_reward += (step["reward"] * (roles == 1)).sum(1) / (roles == 1).sum(1)
            shaped = step["reward"]
            if args.shape_lead:
                phi_next = lead_potential(step["production"], step["worlds"], step["done"])
                shaped = shaped + args.shape_lead * (args.gamma * phi_next - phi)
                phi = phi_next
            if sample is not None:
                sample["reward"] = shaped[sample["select"]]
                sample["win"] = step["reward"][sample["select"]]
                sample["done"] = np.repeat(step["done"][:, None], PLAYERS, 1)[sample["select"]]
                steps.append(sample)
            done = step["done"].copy()
        played = time.perf_counter() - started
        (pl, vl, kl, ent), n = update(model, ref, optimizer, steps,
                                      advantages(steps, args.gamma, args.lam), kl_coef, free_coef,
                                      teach_coef, mission_coef, args, dev,
                                      upgo_advantages(steps, args.gamma) if args.upgo else None)
        wins = sum(s["win"].sum() for s in steps) / max((roles == 1).sum(), 1)
        handed_rows_n = max(sum(float(s["handed"].sum()) for s in steps), 1.0)
        # How often the learner's handed-over fleets sampled a join or an
        # unload (t191).
        rate = lambda name: sum(float(((s["fleets"][name][:len(s["handed"]),
                                                          :s["handed"].shape[1]] > 0)
                                       * s["handed"]).sum()) for s in steps) / handed_rows_n
        row = dict(iteration=it, samples=n, win=float(wins), kl_coef=kl_coef,
                   free_coef=free_coef, teach_coef=teach_coef, policy_loss=pl,
                   value_loss=vl, kl_ref=kl, entropy=ent, join=rate("join"),
                   unload=rate("unload"), play_s=played, env_s=env_s,
                   total_s=time.perf_counter() - started)
        if "builds" in families:
            # The learner's sampled builds that take effect, by kind (build
            # choice 1 up, the last as the executor would), a count per
            # seat-year, over its own worlds and only the first slot of a
            # world that defers to the executor (t207).
            executor = nc_ai_env.BUILD_EXECUTOR
            kinds = 0
            for s in steps:
                build = s["goals"]["build"].numpy()
                owned = s["batch"]["mask_build"].numpy()[:, :build.shape[1], executor]
                live = owned[:, :, None] & np.ones(build.shape, bool)
                live[:, :, 1:] &= build[:, :, :1] != executor
                kinds = kinds + np.bincount(build[live], minlength=nc_ai_env.BUILD_CHOICES)
            seat_years = max(sum(len(s["goals"]["build"]) for s in steps), 1)
            row["builds"] = [round(float(k) / seat_years, 3) for k in kinds[1:]]
        if args.mission_smooth > 0 or args.mission_bonus > 0:
            row["mission_coef"] = mission_coef
            row["missions"] = mission_shares(steps, handed_rows_n)
        if teach_coef > 0:
            # Convoy rows the teacher saw, and those it said to unload.
            row["convoys"] = int(sum(int(s["convoy"].sum()) for s in steps))
            row["taught_unloads"] = int(sum(int((s["convoy"] & (s["labels"]["unload"] > 0)).sum())
                                            for s in steps))
        if league:
            league.update(drawn[2], game_reward)
            row["league"] = league.summary()
        if args.anchor_ema:
            ema_toward(ref, model, args.anchor_ema)
        save(model, out / "latest.pt", iteration=it, kind="fleet", families=families)
        if it % args.eval_every == 0:
            snapshot = out / f"snapshot-{it:04d}.pt"
            save(model, snapshot, iteration=it, kind="fleet", families=families)
            if league:
                league.add_self(f"self-{it}", load_fleet_agent(snapshot, dev), snapshot)
            model.eval()
            # Each evaluation plays a fresh block of maps, well away from
            # 100000 and up, where final confirmations are played (t193).
            first_eval = EVAL_MAPS + (it // args.eval_every - 1) * args.eval_games
            for opponent in ("strategist", "executor") + (("target",) if target else ()):
                share = float(head_to_head(
                    model, strategist, families, opponent, args.eval_games, args.years,
                    first_eval, dev, style=args.style, target=target, threads=args.threads))
                row[f"eval_{opponent}"] = share
                row[f"eval_{opponent}_se"] = (share * (1 - share) / args.eval_games) ** 0.5
        log.write(json.dumps(row) + "\n")
        log.flush()
        print(" ".join(f"{k} {v:.3f}" if isinstance(v, float) else f"{k} {v}"
                       for k, v in row.items()), flush=True)
        resume = dict(iteration=it, model=model.state_dict(), optimizer=optimizer.state_dict(),
                      rng=rng.bit_generator.state, torch_rng=torch.get_rng_state(),
                      next_map=next_map, league=league.state() if league else None,
                      ref=ref.state_dict() if args.anchor_ema else None)
        torch.save(resume, out / "resume.tmp")
        (out / "resume.tmp").replace(out / "resume.pt")


if __name__ == "__main__":
    main()
