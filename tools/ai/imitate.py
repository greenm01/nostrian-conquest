"""Train the strategist to set the executor's goals (t155, stage 1).

Online DAgger: games are played with every seat a strategist, or one or two
strategists against scripted seats. Each year the executor's own goals for
every strategist seat (``Env.goal_labels``) are the targets of one gradient
step. Seats play the executor's goals with probability ``beta``, otherwise
the student's sample, and ``beta`` falls from 1 to ``--beta-end``, so the
student learns in the states its own play reaches.

    uv run imitate.py --out /tmp/nc-ai/imitate.pt

``--init`` starts from a trained strategist that lacks some goal heads,
and ``--heads`` trains only the named heads, the rest frozen and playing
the student's own goals: a new goal joins a trained strategist without
pulling its other goals back to the executor's.
"""

import argparse
import time
from pathlib import Path

import numpy as np
import torch

import nc_ai_env
from evaluate import evaluate
from policy import (HEADS, Strategist, act, bucket, device, distributions, goals_for,
                    load_chooser, save, seat_batch)

PLAYERS = 4
LINEUPS = (
    ["strategist"] * 4,
    ["strategist", "executor", "executor", "executor"],
    ["strategist", "strategist", "aggressor", "executor"],
    ["strategist", "aggressor", "aggressor", "aggressor"],
)


def random_lineups(rng, games):
    out = []
    for _ in range(games):
        lineup = list(LINEUPS[rng.integers(len(LINEUPS))])
        rng.shuffle(lineup)
        out.append(lineup)
    return out


def imitation_loss(logits, batch, labels, heads=HEADS):
    dists = distributions(logits, batch)
    loss = 0
    accuracy = {}
    for name in heads:
        target = labels[name]
        lp = dists[name].log_prob(target)
        loss = loss - (lp.sum(-1) if lp.ndim > 1 else lp).mean()
        hit = dists[name].probs.argmax(-1) == target
        accuracy[name] = (hit.all(-1) if hit.ndim > 1 else hit).float().mean().item()
    return loss, accuracy


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--out", required=True)
    parser.add_argument("--rounds", type=int, default=40)
    parser.add_argument("--games", type=int, default=128)
    parser.add_argument("--years", type=int, default=60)
    parser.add_argument("--beta-end", type=float, default=0.1)
    parser.add_argument("--lr", type=float, default=3e-4)
    parser.add_argument("--eval-games", type=int, default=128)
    parser.add_argument("--seed", type=int, default=1)
    parser.add_argument("--init", help="trained strategist to start from")
    parser.add_argument("--heads", default=",".join(HEADS),
                        help="the goal heads to train; the rest stay frozen")
    args = parser.parse_args()

    dev = device()
    rng = np.random.default_rng(args.seed)
    torch.manual_seed(args.seed)
    heads = tuple(args.heads.split(","))
    model = Strategist(PLAYERS).to(dev)
    if args.init:
        checkpoint = torch.load(args.init, map_location=dev, weights_only=False)
        missing, unexpected = model.load_state_dict(checkpoint["state"], strict=False)
        if unexpected or any(key.split(".")[0] not in heads for key in missing):
            raise SystemExit(f"--init does not fit: {missing} {unexpected}")
    params = (model.parameters() if heads == HEADS
              else [p for name in heads for p in getattr(model, name).parameters()])
    optimizer = torch.optim.AdamW(params, lr=args.lr)
    env = nc_ai_env.Env(PLAYERS, args.years)
    Path(args.out).parent.mkdir(parents=True, exist_ok=True)
    next_map = 1

    for round_ in range(args.rounds):
        beta = 1.0 + (args.beta_end - 1.0) * round_ / max(args.rounds - 1, 1)
        maps = list(range(next_map, next_map + args.games))
        next_map += args.games
        env.reset(maps, seats=random_lineups(rng, args.games))
        started = time.perf_counter()
        done = np.zeros(args.games, bool)
        losses, accuracies, samples = [], [], 0
        while not done.all():
            select = env.strategist & ~done[:, None]
            batch = bucket(seat_batch(env.observe(), env.masks(), env.goal_masks(), select, dev))
            labels_np = env.goal_labels()
            labels = {name: torch.from_numpy(labels_np[name][select]).to(dev) for name in HEADS}

            model.train()
            logits, _ = model(batch)
            loss, accuracy = imitation_loss(logits, batch, labels, heads)
            optimizer.zero_grad()
            loss.backward()
            torch.nn.utils.clip_grad_norm_(model.parameters(), 1.0)
            optimizer.step()
            losses.append(loss.item())
            accuracies.append(accuracy)
            samples += int(select.sum())

            with torch.no_grad():
                model.eval()
                student, _, _ = act(model, batch)
            goals = goals_for(student, select)
            teach = select & (rng.random(select.shape) < beta)
            for name in heads:
                goals[name][teach] = labels_np[name][teach]
            done = env.step(goals=goals)["done"].copy()

        mean_acc = {name: np.mean([a[name] for a in accuracies]) for name in heads}
        print(f"round {round_ + 1:3} beta {beta:.2f} loss {np.mean(losses):.3f} "
              f"samples {samples} {time.perf_counter() - started:.1f}s  "
              + " ".join(f"{name} {acc:.3f}" for name, acc in mean_acc.items())
              + (f"  mps {torch.mps.driver_allocated_memory() / 2**30:.1f}GiB"
                 if dev.type == "mps" else ""),
              flush=True)
        save(model, args.out, round=round_ + 1)

    choose = load_chooser(args.out, greedy=True)
    for opponent in ("executor", "aggressor"):
        wins, production = evaluate(choose, opponent, args.eval_games, args.years, 100000)
        print(f"greedy student vs 3x {opponent}: win share {wins.mean():.3f}, "
              f"production share {production.mean():.3f}")


if __name__ == "__main__":
    main()
