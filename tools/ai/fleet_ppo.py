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
saw used leave their start: they are left out of the KL penalty, and their
entropy earns ``--free-entropy`` on top of ``--entropy-coef``.

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
from fleet_policy import (FLEET_HEADS, FleetAgent, bucket, fleet_arrays, fleet_batch,
                          load_fleet_agent)
from policy import HEADS, device, goals_for, save
from ppo import advantages, pad_cat

PLAYERS = 4
FAMILIES = ("colonize", "scout", "strike")
# Observation fleet-row columns: ship counts start at 10, in the order
# battleships, cruisers, destroyers, empty and loaded transports, ETACs,
# scouts.
BB, CA, DD, TT, TTL, ETAC, SC = range(10, 17)


def fleet_families(fleets):
    """Each fleet row's family, as the Rust `fleet_family`: 0 colonize (has
    ETACs), 1 scout (scouts only, or one or two lone destroyers), 2 strike."""
    count = lambda column: fleets[..., column]
    troops = count(TT) + count(TTL)
    combat = count(BB) + count(CA) + count(DD)
    picket = ((count(DD) >= 1) & (count(DD) <= 2) & (count(BB) == 0) & (count(CA) == 0)
              & (count(SC) == 0) & (count(ETAC) == 0) & (troops == 0))
    scout = (combat == 0) & (count(SC) > 0) & (troops == 0)
    return np.where(count(ETAC) > 0, 0, np.where(picket | scout, 1, 2))


def handed_rows(obs, select, families, rows):
    """``[N, rows]`` bools: the selected seats' real fleet rows whose family
    is handed over, padded to ``rows``."""
    family = fleet_families(obs["fleets"][select])
    handed = np.isin(family, [FAMILIES.index(f) for f in families]) & obs["fleets_valid"][select]
    return np.pad(handed, ((0, 0), (0, rows - handed.shape[1])))


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
              frozen=None):
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
            goals, fleets, goal_lp, fleet_lp, _, value, _, _ = model.act(batch, greedy=greedy)
            handed = torch.from_numpy(
                handed_rows(obs, learner, families, batch["fleets"].shape[1])).to(dev)
            logp = goal_lp + (fleet_lp * handed).sum(-1)
            chosen = goals_for(goals, learner)
            for name in HEADS:
                env_goals[name][learner] = chosen[name][learner]
            fleet_actions = fleet_arrays(fleets, learner, env.fleets)
            n = batch["n"]
            sample = {
                "select": learner,
                "batch": {k: v[:n].cpu() for k, v in batch.items() if k != "n"},
                "goals": {k: v.cpu() for k, v in goals.items()},
                "fleets": {k: v.cpu() for k, v in fleets.items()},
                "handed": handed.cpu(),
                "logp": logp.cpu(),
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
            goals, fleets, *_ = agent.act(batch, greedy=True)
            chosen = goals_for(goals, seats)
            for name in HEADS:
                env_goals[name][seats] = chosen[name][seats]
            placed = fleet_arrays(fleets, seats, env.fleets)
            for name in FLEET_HEADS:
                fleet_actions[name][seats] = placed[name][seats]
    started = time.perf_counter()
    step = env.step(actions_for(env, fleet_actions), env_goals)
    return step, sample, env_s + time.perf_counter() - started


def learner_name(families, style=None):
    """The env seat name of a hybrid learner whose executor plays
    ``style`` (a personality, t167), the default when ``None``."""
    return "hybrid:" + ",".join(families) + (f"@{style}" if style else "")


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


def kl_to(new, old, handed, free=()):
    """KL(new || old) per seat: the goal heads, plus the fleet heads of the
    handed-over rows, except the ``free`` ones."""
    total = 0
    for name in HEADS:
        kl = categorical_kl(new[0][name], old[0][name])
        total = total + (kl.sum(-1) if kl.ndim > 1 else kl)
    for name in FLEET_HEADS:
        if name in free:
            continue
        total = total + (categorical_kl(new[1][name], old[1][name]) * handed).sum(-1)
    return total


def update(model, ref, optimizer, steps, advs, kl_coef, args, dev):
    keys = steps[0]["batch"].keys()
    batch = {k: pad_cat([s["batch"][k] for s in steps]) for k in keys}
    rows = batch["fleets"].shape[1]
    pad_rows = lambda t: torch.nn.functional.pad(t, (0, rows - t.shape[1]))
    goals = {k: torch.cat([s["goals"][k] for s in steps]) for k in HEADS}
    fleets = {k: torch.cat([pad_rows(s["fleets"][k][:, :rows]) for s in steps])
              for k in FLEET_HEADS}
    handed = torch.cat([pad_rows(s["handed"][:, :rows]) for s in steps])
    old_logp = torch.cat([s["logp"] for s in steps])
    old_value = torch.cat([s["value"] for s in steps])
    adv = torch.from_numpy(np.concatenate(advs))
    ret = adv + old_value
    adv = (adv - adv.mean()) / (adv.std() + 1e-8)
    stats = []
    model.train()
    for _ in range(args.epochs):
        for idx in torch.randperm(len(adv)).split(args.minibatch):
            mb = bucket({k: v[idx].to(dev) for k, v in batch.items()})
            mb_fleets = {k: torch.nn.functional.pad(v[idx], (0, mb["fleets"].shape[1] - rows))
                         .to(dev) for k, v in fleets.items()}
            mb_handed = torch.nn.functional.pad(handed[idx], (0, mb["fleets"].shape[1] - rows)).to(dev)
            mb_goals = {k: v[idx].to(dev) for k, v in goals.items()}
            _, _, goal_lp, fleet_lp, entropy, value, goal_dists, fleet_dists = model.act(
                mb, fleet_choices=mb_fleets, goal_choices=mb_goals)
            with torch.no_grad():
                *_, ref_goal_dists, ref_fleet_dists = ref.act(
                    mb, fleet_choices=mb_fleets, goal_choices=mb_goals)
            logp = goal_lp + (fleet_lp * mb_handed).sum(-1)
            ratio = torch.exp(logp - old_logp[idx].to(dev))
            a = adv[idx].to(dev)
            policy_loss = -torch.min(ratio * a, ratio.clamp(1 - args.clip, 1 + args.clip) * a).mean()
            value_loss = (value - ret[idx].to(dev)).pow(2).mean()
            kl_ref = kl_to((goal_dists, fleet_dists), (ref_goal_dists, ref_fleet_dists),
                           mb_handed, args.free_heads).mean()
            ent = (entropy * mb_handed).sum(-1).mean() + policy.joint_entropy(goal_dists).mean()
            free_ent = sum(((fleet_dists[name].entropy() * mb_handed).sum(-1).mean()
                            for name in args.free_heads), torch.zeros((), device=dev))
            loss = (policy_loss + args.value_coef * value_loss + kl_coef * kl_ref
                    - args.entropy_coef * ent - args.free_entropy * free_ent)
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
    parser.add_argument("--limit-credit", type=float, default=1.0,
                        help="reward for a production lead at the year limit, against 1 "
                             "for sweeping the map")
    args = parser.parse_args()
    if unknown := set(args.free_heads) - set(FLEET_HEADS):
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
    strategist = policy.load(args.strategist, dev)
    # An exploiter trains against a frozen agent only (main exploiter).
    target = load_fleet_agent(args.exploit, dev) if args.exploit else None
    league = None
    if args.league:
        agents = [(f"{Path(path).parent.name}/{Path(path).stem}", load_fleet_agent(path, dev))
                  for path in args.league_agents]
        league = League([s for s in args.league_styles.split(",") if s], agents,
                        learner_name(families), args.league_self)
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
        if league:
            league.restore(saved["league"], lambda path: load_fleet_agent(path, dev))
        print(f"resuming at iteration {first}", flush=True)

    for it in range(first, args.iterations + 1):
        started = time.perf_counter()
        kl_coef = max(args.kl_floor, args.kl * max(0.0, 1 - (it - 1) / args.kl_iterations))
        drawn = choose_lineups(rng, args.games, learner,
                               learner_name(families) if target else None, league)
        names, roles = drawn[:2]
        env.reset(list(range(next_map, next_map + args.games)), seats=names)
        next_map += args.games
        model.eval()
        steps, done, env_s = [], np.zeros(args.games, bool), 0.0
        # Each game's learner win share, averaged over its learner seats.
        game_reward = np.zeros(args.games)
        while not done.all():
            step, sample, seconds = play_year(env, model, strategist, roles, done, families, dev,
                                              target=target,
                                              frozen=league.agents if league else None)
            env_s += seconds
            game_reward += (step["reward"] * (roles == 1)).sum(1) / (roles == 1).sum(1)
            if sample is not None:
                sample["reward"] = step["reward"][sample["select"]]
                sample["done"] = np.repeat(step["done"][:, None], PLAYERS, 1)[sample["select"]]
                steps.append(sample)
            done = step["done"].copy()
        played = time.perf_counter() - started
        (pl, vl, kl, ent), n = update(model, ref, optimizer, steps,
                                      advantages(steps, args.gamma, args.lam), kl_coef, args, dev)
        wins = sum(s["reward"].sum() for s in steps) / max((roles == 1).sum(), 1)
        row = dict(iteration=it, samples=n, win=float(wins), kl_coef=kl_coef, policy_loss=pl,
                   value_loss=vl, kl_ref=kl, entropy=ent, play_s=played, env_s=env_s,
                   total_s=time.perf_counter() - started)
        if league:
            league.update(drawn[2], game_reward)
            row["league"] = league.summary()
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
                      next_map=next_map, league=league.state() if league else None)
        torch.save(resume, out / "resume.tmp")
        (out / "resume.tmp").replace(out / "resume.pt")


if __name__ == "__main__":
    main()
