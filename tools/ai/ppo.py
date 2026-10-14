"""Fine-tune the strategist by PPO self-play (t155, stage 2).

Each iteration plays a batch of full games. In every game one or two seats
are the learner; the rest come from a pool of the tuned executor, Aggressor
and frozen snapshots of the learner. A seat's reward is its win share when
the game ends, plus a production-share shaping term each year that fades to
nothing over ``--shaping-iterations``.

    uv run ppo.py --init /tmp/nc-ai/imitate.pt --out /tmp/nc-ai/ppo
"""

import argparse
import json
import time
from pathlib import Path

import numpy as np
import torch

import nc_ai_env
from evaluate import evaluate
from policy import (HEADS, act, bucket, device, distributions, goals_for, joint_entropy,
                    joint_log_prob, load, load_chooser, save, seat_batch)

PLAYERS = 4


def pad_cat(tensors):
    """Concatenate along dim 0, padding later dims with zeros (False)."""
    shape = [max(t.shape[d] for t in tensors) for d in range(1, tensors[0].ndim)]
    out = []
    for t in tensors:
        pad = []
        for d in reversed(range(1, t.ndim)):
            pad += [0, shape[d - 1] - t.shape[d]]
        out.append(torch.nn.functional.pad(t, pad) if any(pad) else t)
    return torch.cat(out)


def choose_lineups(rng, games, snapshots):
    """Seat names for the env and a role per seat: 0 learner, k > 0 frozen
    snapshot k - 1, -1 scripted."""
    names, roles = [], np.full((games, PLAYERS), -1)
    for game in range(games):
        learners = 1 + (rng.random() < 0.3)
        lineup = []
        for seat in range(PLAYERS):
            if seat < learners:
                lineup.append("strategist")
                roles[game, seat] = 0
                continue
            pick = rng.random()
            if snapshots and pick < 0.3:
                lineup.append("strategist")
                roles[game, seat] = 1 + rng.integers(len(snapshots))
            elif pick < 0.9:
                lineup.append("executor")
            else:
                lineup.append("aggressor")
        order = rng.permutation(PLAYERS)
        names.append([lineup[i] for i in order])
        roles[game] = roles[game][order]
    return names, roles


def rollout(env, model, snapshots, roles, dev, shaping):
    """Play every game to its end. Returns the learner's samples and the
    per-seat rewards, indexed by year and learner seat."""
    games = roles.shape[0]
    learner = roles == 0
    done = np.zeros(games, bool)
    steps = []
    prev_share = None
    while not done.all():
        live = ~done[:, None]
        obs, masks, goal_masks = env.observe(), env.masks(), env.goal_masks()
        goals = None
        with torch.no_grad():
            select = learner & live
            batch = seat_batch(obs, masks, goal_masks, select, dev)
            choices, logp, value = act(model, bucket(batch))
            goals = goals_for(choices, select)
            for k, frozen in enumerate(snapshots):
                fsel = (roles == k + 1) & live
                if fsel.any():
                    fbatch = seat_batch(obs, masks, goal_masks, fsel, dev)
                    fchoices, _, _ = act(frozen, bucket(fbatch))
                    fgoals = goals_for(fchoices, fsel)
                    for name in HEADS:
                        goals[name][fsel] = fgoals[name][fsel]
        step = env.step(goals=goals)
        totals = step["production"].sum(axis=1, keepdims=True).clip(min=1)
        share = step["production"] / totals
        shaped = step["reward"].copy()
        if prev_share is not None:
            shaped += shaping * (share - prev_share)
        prev_share = share
        steps.append({
            "select": select,
            "batch": {k: v.cpu() for k, v in batch.items()},
            "choices": {k: v.cpu() for k, v in choices.items()},
            "logp": logp.cpu(),
            "value": value.cpu(),
            "reward": shaped[select],
            "win": step["reward"][select],
            "done": np.repeat(step["done"][:, None], PLAYERS, 1)[select],
        })
        done = step["done"].copy()
    return steps


def advantages(steps, gamma, lam):
    """GAE per learner seat. A seat's samples appear in consecutive years
    until its game ends, in the same order within each year's selection."""
    games, seats = steps[0]["select"].shape
    adv_next = np.zeros((games, seats), np.float32)
    value_next = np.zeros((games, seats), np.float32)
    out = [None] * len(steps)
    for t in reversed(range(len(steps))):
        s = steps[t]
        sel = s["select"]
        value = s["value"].numpy()
        ended = s["done"].astype(np.float32)
        nv = value_next[sel] * (1 - ended)
        delta = s["reward"] + gamma * nv - value
        adv = delta + gamma * lam * (1 - ended) * adv_next[sel]
        adv_next[sel] = adv
        value_next[sel] = value
        out[t] = adv
    return out


def update(model, optimizer, steps, advs, args, dev):
    batch = {k: pad_cat([s["batch"][k] for s in steps]) for k in steps[0]["batch"]}
    choices = {k: torch.cat([s["choices"][k] for s in steps]) for k in HEADS}
    old_logp = torch.cat([s["logp"] for s in steps])
    old_value = torch.cat([s["value"] for s in steps])
    adv = torch.from_numpy(np.concatenate(advs))
    ret = adv + old_value
    adv = (adv - adv.mean()) / (adv.std() + 1e-8)
    n = len(adv)
    stats = []
    model.train()
    for _ in range(args.epochs):
        for idx in torch.randperm(n).split(args.minibatch):
            mb = bucket({k: v[idx].to(dev) for k, v in batch.items()})
            logits, value = model(mb)
            dists = distributions(logits, mb)
            logp = joint_log_prob(dists, {k: v[idx].to(dev) for k, v in choices.items()})
            ratio = torch.exp(logp - old_logp[idx].to(dev))
            a = adv[idx].to(dev)
            policy_loss = -torch.min(ratio * a, ratio.clamp(1 - args.clip, 1 + args.clip) * a).mean()
            value_loss = (value - ret[idx].to(dev)).pow(2).mean()
            entropy = joint_entropy(dists).mean()
            loss = policy_loss + args.value_coef * value_loss - args.entropy_coef * entropy
            optimizer.zero_grad()
            loss.backward()
            torch.nn.utils.clip_grad_norm_(model.parameters(), 0.5)
            optimizer.step()
            with torch.no_grad():
                kl = (old_logp[idx].to(dev) - logp).mean().item()
            stats.append((policy_loss.item(), value_loss.item(), entropy.item(), kl))
    # Late-game batches pad to long contact lists; hand the cached memory back.
    if dev.type == "mps":
        torch.mps.empty_cache()
    return np.mean(stats, axis=0), n


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--init", required=True, help="imitation checkpoint")
    parser.add_argument("--out", required=True, help="output directory")
    parser.add_argument("--iterations", type=int, default=200)
    parser.add_argument("--games", type=int, default=128)
    parser.add_argument("--years", type=int, default=60)
    parser.add_argument("--lr", type=float, default=1e-4)
    parser.add_argument("--epochs", type=int, default=3)
    parser.add_argument("--minibatch", type=int, default=1024)
    parser.add_argument("--clip", type=float, default=0.2)
    parser.add_argument("--value-coef", type=float, default=0.5)
    parser.add_argument("--entropy-coef", type=float, default=0.003)
    parser.add_argument("--gamma", type=float, default=0.995)
    parser.add_argument("--lam", type=float, default=0.95)
    parser.add_argument("--shaping", type=float, default=1.0)
    parser.add_argument("--shaping-iterations", type=int, default=100)
    parser.add_argument("--snapshot-every", type=int, default=20)
    parser.add_argument("--eval-every", type=int, default=10)
    parser.add_argument("--eval-games", type=int, default=256)
    parser.add_argument("--seed", type=int, default=1)
    args = parser.parse_args()

    dev = device()
    rng = np.random.default_rng(args.seed)
    torch.manual_seed(args.seed)
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    model = load(args.init, dev)
    optimizer = torch.optim.AdamW(model.parameters(), lr=args.lr)
    snapshots = []
    env = nc_ai_env.Env(PLAYERS, args.years)
    log = open(out / "log.jsonl", "a")
    next_map = 1_000_000

    for it in range(1, args.iterations + 1):
        started = time.perf_counter()
        shaping = args.shaping * max(0.0, 1 - (it - 1) / args.shaping_iterations)
        names, roles = choose_lineups(rng, args.games, snapshots)
        env.reset(list(range(next_map, next_map + args.games)), seats=names)
        next_map += args.games
        model.eval()
        steps = rollout(env, model, snapshots, roles, dev, shaping)
        played = time.perf_counter() - started
        advs = advantages(steps, args.gamma, args.lam)
        (pl, vl, ent, kl), n = update(model, optimizer, steps, advs, args, dev)
        wins = np.concatenate([s["win"] for s in steps]).sum() / max((roles == 0).sum(), 1)
        row = dict(iteration=it, samples=n, win=float(wins), shaping=shaping,
                   policy_loss=pl, value_loss=vl, entropy=ent, kl=kl,
                   play_s=played, total_s=time.perf_counter() - started)
        save(model, out / "latest.pt", iteration=it)
        if it % args.snapshot_every == 0:
            path = out / f"snapshot-{it:04d}.pt"
            save(model, path, iteration=it)
            snapshots.append(load(path, dev))
        if it % args.eval_every == 0:
            choose = load_chooser(out / "latest.pt", greedy=False)
            for opponent in ("executor", "aggressor"):
                w, p = evaluate(choose, opponent, args.eval_games, args.years, 100000)
                row[f"eval_{opponent}"] = float(w.mean())
                row[f"eval_{opponent}_production"] = float(p.mean())
        log.write(json.dumps(row) + "\n")
        log.flush()
        print(" ".join(f"{k} {v:.3f}" if isinstance(v, float) else f"{k} {v}"
                       for k, v in row.items()), flush=True)


if __name__ == "__main__":
    main()
