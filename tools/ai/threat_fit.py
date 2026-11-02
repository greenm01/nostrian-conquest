"""Fit a threat score (t247): logistic regression on ``threat_stats --dump``
rows, from the owner's memory only, to the chance a world is attacked.

    uv run threat_fit.py exec.csv [--horizon 1] [--test-share 0.25]

Maps are split, not rows: the last ``--test-share`` of the maps are held
out. Strength and count features are fitted as ``log1p``. Prints, on the
held-out maps, the base rate, the average precision of the score, and the
precision of the score and of the old rule (the ``rule`` column) at the
rule's recall; then the weights, with each feature's transform, mean and
scale, so Rust can compute the same score.
"""

import argparse

import numpy as np
import torch

LOGGED = ("strength", "declared", "largest", "contacts", "batteries", "armies", "production",
          "potential", "enemy_world")


def average_precision(score, label):
    order = np.argsort(-score)
    hits = label[order]
    ranks = np.arange(1, len(hits) + 1)
    return float((np.cumsum(hits)[hits == 1] / ranks[hits == 1]).mean())


def precision_at_recall(score, label, recall):
    order = np.argsort(-score)
    hits = np.cumsum(label[order])
    k = int(np.searchsorted(hits, recall * label.sum())) + 1
    return float(hits[k - 1] / k), float(k / len(label))


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("csv")
    parser.add_argument("--horizon", type=int, default=1, choices=(1, 2))
    parser.add_argument("--test-share", type=float, default=0.25)
    args = parser.parse_args()

    with open(args.csv) as f:
        header = f.readline().strip().split(",")
    data = np.loadtxt(args.csv, delimiter=",", skiprows=1, dtype=np.float64)
    col = {name: i for i, name in enumerate(header)}
    label = data[:, col[f"attacked_{args.horizon}"]]
    data, label = data[label >= 0], label[label >= 0]
    features = header[col["batteries"]:col["rule"]]
    x = np.stack([data[:, col[name]] for name in features], axis=1)
    logged = np.array([name.startswith(LOGGED) for name in features])
    x[:, logged] = np.log1p(np.maximum(x[:, logged], 0.0))

    maps = np.unique(data[:, col["map"]])
    cut = maps[int(len(maps) * (1.0 - args.test_share))]
    train = data[:, col["map"]] < cut
    test = ~train
    mean, scale = x[train].mean(0), x[train].std(0) + 1e-9
    z = torch.tensor((x - mean) / scale, dtype=torch.float32)
    y = torch.tensor(label, dtype=torch.float32)

    weights = torch.zeros(z.shape[1], requires_grad=True)
    bias = torch.zeros(1, requires_grad=True)
    optimizer = torch.optim.LBFGS([weights, bias], max_iter=200, line_search_fn="strong_wolfe")
    loss_fn = torch.nn.BCEWithLogitsLoss()
    zt, yt = z[train], y[train]

    def closure():
        optimizer.zero_grad()
        loss = loss_fn(zt @ weights + bias, yt)
        loss.backward()
        return loss

    optimizer.step(closure)

    with torch.no_grad():
        score = (z[test] @ weights + bias).numpy()
    held = label[test]
    rule = data[test, col["rule"]]
    rule_recall = float(rule[held == 1].mean())
    rule_precision = float(held[rule == 1].mean())
    score_precision, flagged = precision_at_recall(score, held, rule_recall)
    print(f"{len(maps)} maps, {int(train.sum())} training and {int(test.sum())} held-out "
          f"world-years; attacked within {args.horizon} year(s)")
    print(f"held-out base rate {held.mean():.4f}")
    print(f"rule: recall {rule_recall:.3f}, precision {rule_precision:.4f}, "
          f"flagged {rule.mean():.3f}")
    print(f"score at the rule's recall: precision {score_precision:.4f}, flagged {flagged:.3f}")
    print(f"score average precision {average_precision(score, held):.4f} "
          f"(rule's single point {rule_precision:.4f})")
    print("\nrecall, precision, flagged, cutoff (logit)")
    order = np.sort(score)[::-1]
    for recall in (0.1, 0.2, 0.3, 0.5, rule_recall, 0.9):
        precision, share = precision_at_recall(score, held, recall)
        print(f"{recall:.3f}, {precision:.4f}, {share:.4f}, {order[int(share * len(score)) - 1]:+.4f}")
    print("\nfeature, transform, mean, scale, weight")
    for name, is_log, m, s, w in zip(features, logged, mean, scale, weights.detach().numpy()):
        print(f"{name}, {'log1p' if is_log else 'raw'}, {m:.6g}, {s:.6g}, {w:+.6g}")
    print(f"bias, , , , {bias.item():+.6g}")


if __name__ == "__main__":
    main()
