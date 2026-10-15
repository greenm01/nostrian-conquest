"""The strategist network: an entity transformer over one seat's
observation, with a head for each goal and a value head.

Every entity row (the seat's global row, worlds, own fleets, contacts and
starbases) becomes a token. Tax, posture, allocation, diplomacy and value
read the
global token. Strike and colonize point at worlds: index 0 (hold, auto)
scores from the global token and each world scores against a query from
the global token.
"""

import math

import numpy as np
import torch
from torch import nn

import nc_ai_env

TABLES = (
    ("planets", nc_ai_env.PLANET_WIDTH),
    ("fleets", nc_ai_env.FLEET_WIDTH),
    ("contacts", nc_ai_env.CONTACT_WIDTH),
    ("starbases", nc_ai_env.STARBASE_WIDTH),
)
HEADS = ("tax", "posture", "allocation", "diplomacy", "strike", "colonize")
START_YEAR = 3000.0
# MPS compiles and keeps a graph for every input shape, so batches are padded
# up to a few shapes: seats to a multiple of SEAT_BUCKET, table rows to a
# multiple of ROW_BUCKET.
SEAT_BUCKET = 128
ROW_BUCKET = 16


def device():
    return torch.device("mps" if torch.backends.mps.is_available() else "cpu")


def symlog(x: torch.Tensor) -> torch.Tensor:
    return torch.sign(x) * torch.log1p(x.abs())


def seat_batch(obs, masks, goal_masks, select, dev):
    """Tensors for the seats where ``select`` (``[games, seats]``) holds,
    one row per seat. Pass them through ``bucket`` before the model."""
    take = lambda a: torch.from_numpy(np.ascontiguousarray(a[select])).to(dev)
    glob = take(obs["global"]).clone()
    glob[:, 0] -= START_YEAR
    batch = {"global": symlog(glob)}
    for name, _ in TABLES:
        batch[name] = symlog(take(obs[name]))
        batch[f"{name}_valid"] = take(obs[f"{name}_valid"])
    batch["mask_diplomacy"] = take(masks["diplomacy"])
    batch["mask_strike"] = take(goal_masks["strike"])
    batch["mask_colonize"] = take(goal_masks["colonize"])
    return batch


def _round_up(n, step):
    return max(step, -(-n // step) * step)


def bucket(batch):
    """``batch`` padded to bucketed shapes, with ``n`` its real seat count.
    Padded seats allow only the first choice of every head."""
    n = batch["global"].shape[0]
    rows = _round_up(n, SEAT_BUCKET)
    out = {"n": n}
    for name, value in batch.items():
        pad = []
        if name in {table for table, _ in TABLES} or name.endswith("_valid"):
            table = name.removesuffix("_valid")
            if table != "planets":
                extra = _round_up(value.shape[1], ROW_BUCKET) - value.shape[1]
                pad = [0, 0] * (value.ndim - 2) + [0, extra]
        pad += [0, 0] * (value.ndim - 1 - len(pad) // 2) + [0, rows - n]
        padded = torch.nn.functional.pad(value, pad)
        if name.startswith("mask_"):
            padded[n:, ..., 0] = True
        out[name] = padded
    return out


class Strategist(nn.Module):
    def __init__(self, players=4, width=128, layers=3, heads=4):
        super().__init__()
        self.config = dict(players=players, width=width, layers=layers, heads=heads)
        self.players = players
        self.embed_global = nn.Linear(nc_ai_env.GLOBAL_WIDTH, width)
        self.embed = nn.ModuleDict({name: nn.Linear(cols, width) for name, cols in TABLES})
        layer = nn.TransformerEncoderLayer(
            width, heads, 4 * width, dropout=0.0, batch_first=True, norm_first=True
        )
        self.encoder = nn.TransformerEncoder(layer, layers, enable_nested_tensor=False)
        self.norm = nn.LayerNorm(width)
        self.tax = nn.Linear(width, nc_ai_env.TAX_CHOICES)
        self.posture = nn.Linear(width, nc_ai_env.POSTURES)
        self.allocation = nn.Linear(width, nc_ai_env.ALLOCATIONS)
        self.diplomacy = nn.Linear(width, players * nc_ai_env.DIPLOMACY_CHOICES)
        self.value = nn.Sequential(nn.Linear(width, width), nn.ReLU(), nn.Linear(width, 1))
        self.pointers = nn.ModuleDict({
            name: nn.ModuleDict({"query": nn.Linear(width, width), "none": nn.Linear(width, 1)})
            for name in ("strike", "colonize")
        })

    def encode(self, batch):
        """Encoded tokens: the global token ``[N, d]``, world tokens
        ``[N, W, d]`` and own-fleet tokens ``[N, F, d]``."""
        glob = self.embed_global(batch["global"])[:, None]
        tokens = [glob]
        pads = [torch.zeros(glob.shape[:2], dtype=torch.bool, device=glob.device)]
        for name, _ in TABLES:
            tokens.append(self.embed[name](batch[name]))
            pads.append(~batch[f"{name}_valid"])
        x = self.encoder(torch.cat(tokens, 1), src_key_padding_mask=torch.cat(pads, 1))
        x = self.norm(x)
        worlds = batch["planets"].shape[1]
        fleets = batch["fleets"].shape[1]
        return x[:, 0], x[:, 1 : 1 + worlds], x[:, 1 + worlds : 1 + worlds + fleets]

    def forward(self, batch):
        g, worlds, _ = self.encode(batch)
        return self.goal_heads(g, worlds, batch["n"])

    def goal_heads(self, g, worlds, n):
        """Goal logits and value for the first ``n`` (real) seats."""
        logits = {
            "tax": self.tax(g),
            "posture": self.posture(g),
            "allocation": self.allocation(g),
            "diplomacy": self.diplomacy(g).view(-1, self.players, nc_ai_env.DIPLOMACY_CHOICES),
        }
        for name, head in self.pointers.items():
            query = head["query"](g)[:, :, None]
            per_world = (worlds @ query)[:, :, 0] / math.sqrt(g.shape[-1])
            logits[name] = torch.cat([head["none"](g), per_world], 1)
        return {k: v[:n] for k, v in logits.items()}, self.value(g)[:n, 0]

def masked(logits, batch):
    """Logits with disallowed choices at -inf."""
    n = batch["n"]
    out = dict(logits)
    out["diplomacy"] = logits["diplomacy"].masked_fill(~batch["mask_diplomacy"][:n], -torch.inf)
    for name in ("strike", "colonize"):
        out[name] = logits[name].masked_fill(~batch[f"mask_{name}"][:n], -torch.inf)
    return out


def distributions(logits, batch):
    return {name: torch.distributions.Categorical(logits=value)
            for name, value in masked(logits, batch).items()}


def act(model, batch, greedy=False):
    """Sampled (or greedy) goals, their joint log-probability, and value."""
    logits, value = model(batch)
    dists = distributions(logits, batch)
    choices = {name: dist.probs.argmax(-1) if greedy else dist.sample()
               for name, dist in dists.items()}
    return choices, joint_log_prob(dists, choices), value


def joint_log_prob(dists, choices):
    total = 0
    for name, dist in dists.items():
        lp = dist.log_prob(choices[name])
        total = total + (lp.sum(-1) if lp.ndim > 1 else lp)
    return total


def joint_entropy(dists):
    total = 0
    for dist in dists.values():
        ent = dist.entropy()
        total = total + (ent.sum(-1) if ent.ndim > 1 else ent)
    return total


def goals_for(choices, select):
    """Env goal arrays with ``choices`` at the ``select`` seats."""
    games, seats = select.shape
    out = {
        "tax": np.zeros((games, seats), np.int64),
        "diplomacy": np.zeros((games, seats, seats), np.int64),
        "posture": np.zeros((games, seats), np.int64),
        "allocation": np.zeros((games, seats), np.int64),
        "strike": np.zeros((games, seats), np.int64),
        "colonize": np.zeros((games, seats), np.int64),
    }
    for name in HEADS:
        out[name][select] = choices[name].cpu().numpy()
    return out


def save(model, path, **extra):
    torch.save({"config": model.config, "state": model.state_dict(), **extra}, path)


def load(path, dev=None):
    """A strategist checkpoint. One trained before the allocation goal
    (t175) gets a head that always chooses 0, striking with everything, as
    it played."""
    checkpoint = torch.load(path, map_location=dev or device(), weights_only=False)
    model = Strategist(**checkpoint["config"]).to(dev or device())
    missing, unexpected = model.load_state_dict(checkpoint["state"], strict=False)
    if unexpected or set(missing) - {"allocation.weight", "allocation.bias"}:
        raise RuntimeError(f"{path} does not fit: {missing} {unexpected}")
    if missing:
        with torch.no_grad():
            model.allocation.weight.zero_()
            model.allocation.bias.fill_(-1e4)
            model.allocation.bias[0] = 0.0
    return model.eval()


def load_chooser(path, greedy=False):
    """A ``choose(env)`` for every strategist seat of ``env`` from a
    checkpoint."""
    dev = device()
    model = load(path, dev)

    @torch.no_grad()
    def choose(env):
        select = env.strategist
        batch = bucket(seat_batch(env.observe(), env.masks(), env.goal_masks(), select, dev))
        choices, _, _ = act(model, batch, greedy)
        return goals_for(choices, select)

    return choose
