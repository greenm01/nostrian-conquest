"""Read experience shards and fit a small masked policy on the GPU: each
fleet's mission from its own row. This checks the shard-to-learner path.

On random play the policy that made the data scores the uniform choice
within each mask, printed as the baseline; a fit close to it shows the path
works (a small dataset can be overfit a little below it).

    uv run learner.py /tmp/nc-shards
"""

import argparse
import time

import numpy as np
import torch

import nc_ai_env
from shards import read_shards, rows


def fleet_examples(table):
    count = table.column("n_fleets").to_numpy()
    features = np.concatenate(rows(table, "fleets", count, nc_ai_env.FLEET_WIDTH))
    masks = np.concatenate(
        rows(table, "mask_mission", count, nc_ai_env.MISSION_CHOICES)
    )
    chosen = np.concatenate(rows(table, "mission", count, 1))[:, 0].astype(np.int64)
    if not masks[np.arange(len(chosen)), chosen].all():
        raise SystemExit("a stored mission falls outside its stored mask")
    return features, masks, chosen


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("shards")
    parser.add_argument("--epochs", type=int, default=20)
    parser.add_argument("--batch", type=int, default=4096)
    args = parser.parse_args()

    started = time.perf_counter()
    table = read_shards(args.shards)
    features, masks, chosen = fleet_examples(table)
    print(f"read {table.num_rows} seat-years, {len(chosen)} fleet choices "
          f"in {time.perf_counter() - started:.2f}s")

    device = torch.device("mps" if torch.backends.mps.is_available() else "cpu")
    x = torch.tensor(features, device=device)
    x = (x - x.mean(0)) / (x.std(0) + 1e-6)
    m = torch.tensor(masks, device=device)
    y = torch.tensor(chosen, device=device)
    model = torch.nn.Sequential(
        torch.nn.Linear(nc_ai_env.FLEET_WIDTH, 64),
        torch.nn.ReLU(),
        torch.nn.Linear(64, nc_ai_env.MISSION_CHOICES),
    ).to(device)
    optimizer = torch.optim.Adam(model.parameters(), lr=3e-3)

    def loss_of(idx):
        logits = model(x[idx]).masked_fill(~m[idx], float("-inf"))
        return torch.nn.functional.cross_entropy(logits, y[idx])

    baseline = np.log(masks.sum(axis=1)).mean()
    started = time.perf_counter()
    for epoch in range(args.epochs):
        for idx in torch.randperm(len(y), device=device).split(args.batch):
            optimizer.zero_grad()
            loss = loss_of(idx)
            loss.backward()
            optimizer.step()
        with torch.no_grad():
            full = loss_of(torch.arange(len(y), device=device)).item()
        print(f"epoch {epoch + 1:3}: loss {full:.4f} (uniform within mask {baseline:.4f})")
    print(f"trained on {device} in {time.perf_counter() - started:.2f}s")


if __name__ == "__main__":
    main()
