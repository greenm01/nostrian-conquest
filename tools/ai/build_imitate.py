"""Teach a fleet agent's build head the executor's builds (t207, step 2).

The agent starts from a saved fleet agent (the promoted one) and only its
build heads train, so its goals and fleet orders stay as PPO left them. In
each game one or two hybrid seats, with the ``--families`` fleets (as the
agent is deployed) and builds handed over, play their own goals and fleet
orders against scripted seats.
Each year the executor's builds for the seat's goals (``Env.fleet_labels``)
are the targets of one gradient step. Online DAgger: a seat builds as the
executor would with probability ``beta``, otherwise as the student, and
``beta`` falls from 1 to ``--beta-end``.

    uv run build_imitate.py --agent fleet-final.pt --out /tmp/nc-ai/builds/imitate.pt
"""

import argparse
import time
from pathlib import Path

import numpy as np
import torch

import nc_ai_env
from fleet_imitate import actions_for
from fleet_policy import BUILD_HEADS, bucket, build_arrays, fleet_arrays, fleet_batch, load_fleet_agent
from policy import device, goals_for, save

PLAYERS = 4
OPPONENTS = ("executor", "executor", "aggressor")


def lineups(rng, games, learner):
    out = []
    for _ in range(games):
        learners = 1 + (rng.random() < 0.3)
        lineup = [learner] * learners + [
            OPPONENTS[i] for i in rng.integers(len(OPPONENTS), size=PLAYERS - learners)
        ]
        rng.shuffle(lineup)
        out.append(lineup)
    return out


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--agent", required=True, help="fleet agent checkpoint")
    parser.add_argument("--out", required=True)
    parser.add_argument("--families", default="strike",
                        help="fleet families the agent orders, as deployed")
    parser.add_argument("--rounds", type=int, default=20)
    parser.add_argument("--games", type=int, default=128)
    parser.add_argument("--years", type=int, default=60)
    parser.add_argument("--beta-end", type=float, default=0.1)
    parser.add_argument("--lr", type=float, default=1e-3)
    parser.add_argument("--seed", type=int, default=1)
    args = parser.parse_args()

    dev = device()
    rng = np.random.default_rng(args.seed)
    torch.manual_seed(args.seed)
    model = load_fleet_agent(args.agent, dev)
    params = [*model.world_build.parameters(), *model.world_amount.parameters()]
    for param in model.parameters():
        param.requires_grad_(False)
    for param in params:
        param.requires_grad_(True)
    optimizer = torch.optim.AdamW(params, lr=args.lr)
    env = nc_ai_env.Env(PLAYERS, args.years)
    Path(args.out).parent.mkdir(parents=True, exist_ok=True)
    next_map = 3_000_001

    for round_ in range(args.rounds):
        beta = 1.0 + (args.beta_end - 1.0) * round_ / max(args.rounds - 1, 1)
        env.reset(list(range(next_map, next_map + args.games)), seats=lineups(rng, args.games, f"hybrid:{args.families},builds"))
        next_map += args.games
        started = time.perf_counter()
        done = np.zeros(args.games, bool)
        losses, slots, hits, built, labelled = [], 0, 0, 0, 0
        while not done.all():
            learner = env.hybrid & ~done[:, None]
            obs, masks, goal_masks = env.observe(), env.masks(), env.goal_masks()
            batch = bucket(fleet_batch(obs, masks, goal_masks, learner, dev))
            n = batch["n"]
            model.eval()
            with torch.no_grad():
                goals, fleets, *_ = model.act(batch, greedy=True, builds=True)
            env_goals = goals_for(goals, learner)
            labels_np = env.fleet_labels(env_goals)
            labels = {name: torch.from_numpy(labels_np[name][learner]).to(dev)
                      for name in BUILD_HEADS}
            # Slots with a real choice: a world of the seat's that can build.
            choosing = (batch["mask_build"][:n].sum(-1) > 1)[:, :, None].expand_as(labels["build"])
            ordered = labels["build"] > 0
            model.train()
            with torch.no_grad():
                _, worlds, _ = model.encode(batch)
            dists = model.build_dists(batch, worlds)
            loss = (-(dists["build"].log_prob(labels["build"]) * choosing).sum()
                    - (dists["amount"].log_prob(labels["amount"]) * ordered).sum()
                    ) / choosing.sum().clamp(min=1)
            optimizer.zero_grad()
            loss.backward()
            optimizer.step()
            losses.append(loss.item())
            with torch.no_grad():
                hits += int(((dists["build"].probs.argmax(-1) == labels["build"]) & choosing).sum())
                slots += int(choosing.sum())
                built += int((goals["build"][:n] > 0).sum())
                labelled += int(ordered.sum())
            # DAgger: each seat builds as the executor would with
            # probability beta, otherwise as the student.
            chosen = build_arrays(goals, learner, env.worlds)
            teach = learner & (rng.random(learner.shape) < beta)
            for name in BUILD_HEADS:
                chosen[name][teach] = labels_np[name][teach]
            actions = actions_for(env, fleet_arrays(fleets, learner, env.fleets))
            actions.update(chosen)
            done = env.step(actions, env_goals)["done"].copy()

        print(f"round {round_ + 1:3} beta {beta:.2f} build loss {np.mean(losses):.3f} "
              f"build accuracy {hits / max(slots, 1):.3f} slots {slots} "
              f"student builds {built} executor builds {labelled} "
              f"{time.perf_counter() - started:.1f}s", flush=True)
        save(model, args.out, round=round_ + 1, kind="fleet")
        if dev.type == "mps":
            torch.mps.empty_cache()


if __name__ == "__main__":
    main()
