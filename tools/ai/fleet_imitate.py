"""Train the fleet-level agent's fleet heads to give the executor's fleet
orders (t157, steps 1-3).

The agent starts as the final strategist (step 3: its strategy and value
weights carry over). In each game one or two hybrid seats, with every
fleet family handed over, play against the frozen strategist, the executor
and Aggressor. Each year the executor's fleet orders for the goals the seat
chose (``Env.fleet_labels``) are the targets of one gradient step, with the
target head trained on the label's mission. A KL term keeps the goal heads
near the frozen strategist, so imitation does not undo what its PPO
learned.

Online DAgger (step 2): fleets follow the executor's orders with
probability ``beta``, otherwise the student's, and ``beta`` falls from 1 to
``--beta-end``, so the student learns in the states its own orders reach.

    uv run fleet_imitate.py --init strategist-final.pt --out /tmp/nc-ai/fleet/imitate.pt
"""

import argparse
import time
from pathlib import Path

import numpy as np
import torch

import nc_ai_env
import policy
from fleet_policy import FLEET_HEADS, bucket, fleet_arrays, fleet_batch, from_strategist
from policy import HEADS, device, goals_for, save

PLAYERS = 4
OPPONENTS = ("strategist", "executor", "executor", "aggressor")


def lineups(rng, games):
    out = []
    for _ in range(games):
        hybrids = 1 + (rng.random() < 0.3)
        lineup = ["hybrid:colonize,scout,strike"] * hybrids + [
            OPPONENTS[i] for i in rng.integers(len(OPPONENTS), size=PLAYERS - hybrids)
        ]
        rng.shuffle(lineup)
        out.append(lineup)
    return out


def frozen_goals(frozen, obs, masks, goal_masks, select, dev):
    """The frozen strategist's goals for the ``select`` seats."""
    batch = policy.bucket(policy.seat_batch(obs, masks, goal_masks, select, dev))
    choices, _, _ = policy.act(frozen, batch)
    return goals_for(choices, select)


def actions_for(env, fleets):
    """A full actions dict: zero for everything a hybrid seat ignores."""
    games, seats = env.strategist.shape
    zeros = lambda *shape: np.zeros(shape, np.int64)
    return {
        "tax": zeros(games, seats),
        "diplomacy": zeros(games, seats, seats),
        "build": zeros(games, seats, env.worlds, nc_ai_env.BUILD_SLOTS),
        "amount": zeros(games, seats, env.worlds, nc_ai_env.BUILD_SLOTS),
        **fleets,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--init", required=True, help="strategist checkpoint")
    parser.add_argument("--out", required=True)
    parser.add_argument("--rounds", type=int, default=40)
    parser.add_argument("--games", type=int, default=128)
    parser.add_argument("--years", type=int, default=60)
    parser.add_argument("--beta-end", type=float, default=0.1)
    parser.add_argument("--lr", type=float, default=3e-4)
    parser.add_argument("--goal-kl", type=float, default=1.0)
    parser.add_argument("--seed", type=int, default=1)
    args = parser.parse_args()

    dev = device()
    rng = np.random.default_rng(args.seed)
    torch.manual_seed(args.seed)
    model = from_strategist(args.init, dev)
    frozen = policy.load(args.init, dev)
    optimizer = torch.optim.AdamW(model.parameters(), lr=args.lr)
    env = nc_ai_env.Env(PLAYERS, args.years)
    Path(args.out).parent.mkdir(parents=True, exist_ok=True)
    next_map = 2_000_001

    for round_ in range(args.rounds):
        beta = 1.0 + (args.beta_end - 1.0) * round_ / max(args.rounds - 1, 1)
        env.reset(list(range(next_map, next_map + args.games)), seats=lineups(rng, args.games))
        next_map += args.games
        started = time.perf_counter()
        done = np.zeros(args.games, bool)
        losses, hits, rows = [], {name: 0 for name in FLEET_HEADS}, 0
        while not done.all():
            live = ~done[:, None]
            hybrid = env.hybrid & live
            others = env.strategist & ~env.hybrid & live
            obs, masks, goal_masks = env.observe(), env.masks(), env.goal_masks()
            batch = bucket(fleet_batch(obs, masks, goal_masks, hybrid, dev))
            n = batch["n"]

            # The student's goals, then the executor's fleet orders for them.
            model.eval()
            with torch.no_grad():
                goals, sampled, *_ = model.act(batch)
            env_goals = goals_for(goals, hybrid)
            if others.any():
                theirs = frozen_goals(frozen, obs, masks, goal_masks, others, dev)
                for name in HEADS:
                    env_goals[name][others] = theirs[name][others]
            labels_np = env.fleet_labels(env_goals)
            fleets = env.fleets
            labels = {
                name: torch.from_numpy(labels_np[name][hybrid]).to(dev)
                for name in FLEET_HEADS
            }
            pad = batch["fleets"].shape[1] - fleets
            labels = {k: torch.nn.functional.pad(v, (0, pad)) for k, v in labels.items()}
            valid = batch["fleets_valid"][:n]

            model.train()
            g, worlds, fleet_tokens = model.encode(batch)
            dists = model.fleet_dists(batch, fleet_tokens, worlds, goals, labels["mission"])
            fleet_loss = sum(-(dists[name].log_prob(labels[name]) * valid).sum()
                             for name in FLEET_HEADS) / valid.sum().clamp(min=1)
            goal_logits, _ = model.goal_heads(g, worlds, n)
            with torch.no_grad():
                frozen_logits, _ = frozen(policy.bucket(
                    policy.seat_batch(obs, masks, goal_masks, hybrid, dev)))
            student = policy.distributions(goal_logits, batch)
            teacher = policy.distributions(frozen_logits, batch)
            goal_kl = sum(torch.distributions.kl_divergence(teacher[k], student[k]).sum(-1).mean()
                          if teacher[k].probs.ndim > 2 else
                          torch.distributions.kl_divergence(teacher[k], student[k]).mean()
                          for k in HEADS)
            loss = fleet_loss + args.goal_kl * goal_kl
            optimizer.zero_grad()
            loss.backward()
            torch.nn.utils.clip_grad_norm_(model.parameters(), 1.0)
            optimizer.step()
            losses.append((fleet_loss.item(), goal_kl.item()))
            with torch.no_grad():
                for name in FLEET_HEADS:
                    hit = (dists[name].probs.argmax(-1) == labels[name]) & valid
                    hits[name] += int(hit.sum())
                rows += int(valid.sum())

            # DAgger: each hybrid seat's fleets follow the executor with
            # probability beta, otherwise the student.
            chosen = fleet_arrays(sampled, hybrid, fleets)
            teach = hybrid & (rng.random(hybrid.shape) < beta)
            for name in FLEET_HEADS:
                chosen[name][teach] = labels_np[name][teach]
            done = env.step(actions_for(env, chosen), env_goals)["done"].copy()

        fleet_loss, goal_kl = np.mean(losses, axis=0)
        print(f"round {round_ + 1:3} beta {beta:.2f} fleet loss {fleet_loss:.3f} "
              f"goal kl {goal_kl:.4f} rows {rows} {time.perf_counter() - started:.1f}s  "
              + " ".join(f"{name} {hits[name] / max(rows, 1):.3f}" for name in FLEET_HEADS)
              + (f"  mps {torch.mps.driver_allocated_memory() / 2**30:.1f}GiB"
                 if dev.type == "mps" else ""),
              flush=True)
        save(model, args.out, round=round_ + 1, kind="fleet")
        if dev.type == "mps":
            torch.mps.empty_cache()


if __name__ == "__main__":
    main()
