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


def choose_lineups(rng, games, learner, target=None):
    """Training lineups. Roles: 1 learner, 2 strategist, 3 the frozen target
    agent an exploiter trains against (``target`` its seat name), 0
    scripted. An exploiter meets only the target."""
    names, roles = [], np.zeros((games, PLAYERS), np.int8)
    for _ in range(games):
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
    return names, roles


def play_year(env, model, strategist, roles, done, families, dev, greedy=False, target=None):
    """Choose every learner's and strategist's goals and the learners'
    fleet orders, and the frozen ``target`` agent's (greedy, as deployed)
    for its seats, and step the batch. Returns the step result, for
    training the learners' samples, and the seconds spent in the games
    themselves (observing and stepping), apart from the networks."""
    live = ~done[:, None]
    learner = (roles == 1) & live
    others = (roles == 2) & live
    targets = (roles == 3) & live
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
        if targets.any():
            batch = bucket(fleet_batch(obs, masks, goal_masks, targets, dev))
            goals, fleets, *_ = target.act(batch, greedy=True)
            chosen = goals_for(goals, targets)
            for name in HEADS:
                env_goals[name][targets] = chosen[name][targets]
            placed = fleet_arrays(fleets, targets, env.fleets)
            for name in FLEET_HEADS:
                fleet_actions[name][targets] = placed[name][targets]
    started = time.perf_counter()
    step = env.step(actions_for(env, fleet_actions), env_goals)
    return step, sample, env_s + time.perf_counter() - started


def learner_name(families, style=None):
    """The env seat name of a hybrid learner whose executor plays
    ``style`` (a personality, t167), the default when ``None``."""
    return "hybrid:" + ",".join(families) + (f"@{style}" if style else "")


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


def kl_to(new, old, handed):
    """KL(new || old) per seat: the goal heads, plus the fleet heads of the
    handed-over rows."""
    total = 0
    for name in HEADS:
        kl = categorical_kl(new[0][name], old[0][name])
        total = total + (kl.sum(-1) if kl.ndim > 1 else kl)
    for name in FLEET_HEADS:
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
                           mb_handed).mean()
            ent = (entropy * mb_handed).sum(-1).mean() + policy.joint_entropy(goal_dists).mean()
            loss = (policy_loss + args.value_coef * value_loss + kl_coef * kl_ref
                    - args.entropy_coef * ent)
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
    parser.add_argument("--limit-credit", type=float, default=1.0,
                        help="reward for a production lead at the year limit, against 1 "
                             "for sweeping the map")
    args = parser.parse_args()
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
    optimizer = torch.optim.AdamW(model.parameters(), lr=args.lr)
    env = nc_ai_env.Env(PLAYERS, args.years, args.threads, limit_credit=args.limit_credit)
    log = open(out / "log.jsonl", "a")
    learner = learner_name(families, args.style)
    next_map = 3_000_000

    for it in range(1, args.iterations + 1):
        started = time.perf_counter()
        kl_coef = max(args.kl_floor, args.kl * max(0.0, 1 - (it - 1) / args.kl_iterations))
        names, roles = choose_lineups(rng, args.games, learner,
                                      learner_name(families) if target else None)
        env.reset(list(range(next_map, next_map + args.games)), seats=names)
        next_map += args.games
        model.eval()
        steps, done, env_s = [], np.zeros(args.games, bool), 0.0
        while not done.all():
            step, sample, seconds = play_year(env, model, strategist, roles, done, families, dev,
                                              target=target)
            env_s += seconds
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
        save(model, out / "latest.pt", iteration=it, kind="fleet", families=families)
        if it % args.eval_every == 0:
            save(model, out / f"snapshot-{it:04d}.pt", iteration=it, kind="fleet",
                 families=families)
            model.eval()
            for opponent in ("strategist", "executor") + (("target",) if target else ()):
                row[f"eval_{opponent}"] = float(head_to_head(
                    model, strategist, families, opponent, args.eval_games, args.years,
                    100000, dev, style=args.style, target=target, threads=args.threads))
        log.write(json.dumps(row) + "\n")
        log.flush()
        print(" ".join(f"{k} {v:.3f}" if isinstance(v, float) else f"{k} {v}"
                       for k, v in row.items()), flush=True)


if __name__ == "__main__":
    main()
