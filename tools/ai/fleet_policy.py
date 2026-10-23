"""The fleet-level agent (t157): the strategist network plus per-fleet
heads that order the seat's own fleets.

Each own-fleet token chooses a mission, then a target world (pointing at
world tokens, conditioned on the chosen mission and on the worlds the seat's
strike and colony goals name), a waypoint short of it for the sector
missions, a speed, rules of engagement, whether to
load armies, and what part of the fleet to detach. In a hybrid seat only the fleets of handed-over families
(colonize 0, scout 1, strike 2) take these choices; the executor orders the
rest.

Since t190 each fleet may also join another of the seat's fleets (pointing
at the other fleet tokens, or none) and unload its armies. These heads start
at "none" with zero weights, so a checkpoint saved before them loads and
plays as it did, and two loads of it start identical.

Since t207 each world token also chooses its builds: a kind (or none) and an
amount for each of ``BUILD_SLOTS`` slots. Builds are seat-level choices, so
with ``builds=True`` they ride with the goals: in the goal choices, the goal
log-probability and the goal distributions. A world whose first slot
chooses ``BUILD_EXECUTOR`` builds as the seat's executor would, and its other
slots and amounts are ignored (and not scored). The build head starts there,
so a seat that takes over its builds starts level with the executor, and
sampled play tries other builds at a few worlds a year (``reset_build_heads``).
"""

import math

import numpy as np
import torch
from torch import nn

import nc_ai_env
from policy import HEADS, Strategist, distributions, symlog, START_YEAR, TABLES

FLEET_HEADS = ("mission", "target", "waypoint", "speed", "roe", "load", "detach", "join", "unload")
BUILD_HEADS = ("build", "amount")
# Heads added after checkpoints were first saved, and the logit by which each
# starts on its first choice (none).
NEW_HEADS = {"fleet_join": 8.0, "fleet_unload": 8.0}
# Heads whose missing weights a saved agent may lack and start fresh.
FRESH_PREFIXES = ("fleet_join", "fleet_unload", "world_build", "world_amount")
SEAT_BUCKET = 128
ROW_BUCKET = 16


def greedy_join(logits):
    """The join head's greedy choice from its ``[..., 1 + F]`` logits: join
    when joining at all is likelier than none, then the likeliest host. A
    plain argmax splits joining among the hosts, so none wins at 0.4 against
    six hosts at 0.1 each (t191)."""
    hosts = logits[..., 1:]
    join = hosts.logsumexp(-1) > logits[..., 0]
    return torch.where(join, hosts.argmax(-1) + 1, torch.zeros_like(hosts[..., 0], dtype=torch.long))


def greedy_build(logits):
    """The build head's greedy choice from its ``[..., BUILD_CHOICES]``
    logits, as ``greedy_join``: building some kind (summed) against none and
    against building as the executor would, then the likeliest kind."""
    executor = nc_ai_env.BUILD_EXECUTOR
    kinds = logits[..., 1:executor]
    build = kinds.logsumexp(-1) > torch.maximum(logits[..., 0], logits[..., executor])
    rest = torch.where(logits[..., executor] > logits[..., 0], executor, 0)
    return torch.where(build, kinds.argmax(-1) + 1, rest)


def build_logp(dists, goals):
    """Per seat, the log-probability of its builds under the build and
    amount ``dists``. A world that defers to the executor scores only its
    first slot, and an amount counts only in a slot that builds."""
    executor = nc_ai_env.BUILD_EXECUTOR
    live = torch.ones_like(goals["build"], dtype=torch.bool)
    live[..., 1:] = (goals["build"][..., :1] != executor)
    ordered = live & (goals["build"] > 0) & (goals["build"] != executor)
    return ((dists["build"].log_prob(goals["build"]) * live).sum((1, 2))
            + (dists["amount"].log_prob(goals["amount"]) * ordered).sum((1, 2)))


def fleet_batch(obs, masks, goal_masks, select, dev):
    """Seat tensors as ``policy.seat_batch``, plus each fleet row's masks:
    ``fleet_mission`` ``[N, F, MISSIONS]``, ``fleet_target`` ``[N, F,
    MISSIONS, W]``, ``fleet_roe``, ``fleet_load`` ``[N, F, 2]`` and
    ``fleet_detach``, ``fleet_join`` ``[N, F, 1 + F]`` and ``fleet_unload``
    ``[N, F, 2]``; and each world's ``mask_build`` ``[N, W, BUILD_CHOICES]``."""
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
    batch["fleet_mission"] = take(masks["mission"])
    batch["fleet_target"] = take(masks["target"])
    batch["fleet_roe"] = take(masks["roe"])
    load = take(masks["load"])
    batch["fleet_load"] = torch.stack([torch.ones_like(load), load], -1)
    batch["fleet_detach"] = take(masks["detach"])
    batch["fleet_join"] = take(masks["join"])
    unload = take(masks["unload"])
    batch["fleet_unload"] = torch.stack([torch.ones_like(unload), unload], -1)
    batch["mask_build"] = take(masks["build"])
    return batch


def _round_up(n, step):
    return max(step, -(-n // step) * step)


def bucket(batch):
    """``batch`` padded to bucketed shapes, as ``policy.bucket``, with fleet
    masks padded along the fleet rows. Every mask row allows at least its
    first choice, so padded seats and rows always have a choice."""
    n = batch["global"].shape[0]
    rows = _round_up(n, SEAT_BUCKET)
    fleets = _round_up(batch["fleets"].shape[1], ROW_BUCKET)
    out = {"n": n}
    for name, value in batch.items():
        pad = [0, 0] * value.ndim
        pad[-1] = rows - n  # the seat axis, last in F.pad's order
        table = name.removesuffix("_valid")
        if name.startswith("fleet_") or table == "fleets":
            pad[-3] = fleets - value.shape[1]
        if name == "fleet_join":
            pad[1] = 1 + fleets - value.shape[-1]  # the host axis
        elif table in {t for t, _ in TABLES} and table != "planets":
            pad[-3] = _round_up(value.shape[1], ROW_BUCKET) - value.shape[1]
        padded = torch.nn.functional.pad(value, pad)
        if name.startswith("mask_") or name.startswith("fleet_"):
            empty = ~padded.any(-1)
            padded[..., 0] |= empty
        out[name] = padded
    return out


def goal_flags(goals, worlds):
    """``[N, W, 2]``: whether the seat's strike and colony goals name each
    world."""
    return torch.stack([
        torch.nn.functional.one_hot((goals[name] - 1).clamp(min=0), worlds.shape[1])
        * (goals[name] > 0)[:, None]
        for name in ("strike", "colonize")
    ], -1).to(worlds.dtype)


class FleetAgent(Strategist):
    def __init__(self, players=4, width=128, layers=3, heads=4):
        super().__init__(players, width, layers, heads)
        missions = nc_ai_env.MISSION_CHOICES
        self.fleet_mission = nn.Linear(width, missions)
        self.mission_embed = nn.Embedding(missions, width)
        self.target_query = nn.Linear(width, width)
        self.goal_bias = nn.Linear(2, 1, bias=False)
        self.fleet_waypoint = nn.Linear(width, nc_ai_env.WAYPOINT_CHOICES)
        self.fleet_speed = nn.Linear(width, nc_ai_env.SPEED_CHOICES)
        self.fleet_roe = nn.Linear(width, nc_ai_env.ROE_CHOICES)
        self.fleet_load = nn.Linear(width, 2)
        self.fleet_detach = nn.Linear(width, nc_ai_env.DETACH_CHOICES)
        self.fleet_join = nn.Linear(width, 1)
        self.fleet_join_query = nn.Linear(width, width, bias=False)
        self.fleet_join_key = nn.Linear(width, width, bias=False)
        self.fleet_unload = nn.Linear(width, 2)
        slots = nc_ai_env.BUILD_SLOTS
        # A world's builds see its token, the global token, whether the
        # seat's strike or colony goal names it, and the seat's posture and
        # allocation, as the executor's builds follow the goals. Summed
        # projections, not one over a concatenation, which tract mis-shapes.
        seat = self.posture.out_features + self.allocation.out_features
        self.world_build_hidden = nn.Linear(width, width)
        self.world_build_seat = nn.Linear(width + seat, width, bias=False)
        self.world_build_flags = nn.Linear(2, width, bias=False)
        self.world_build = nn.Linear(width, slots * nc_ai_env.BUILD_CHOICES)
        self.world_amount = nn.Linear(width, slots * nc_ai_env.BUILD_AMOUNT_CHOICES)
        self.reset_new_heads()
        self.reset_build_heads()

    def reset_new_heads(self):
        """Start the join and unload heads on "none", the same every time:
        zero weights, the key as the identity so the query learns."""
        with torch.no_grad():
            for name, bias in NEW_HEADS.items():
                head = getattr(self, name)
                head.weight.zero_()
                head.bias.zero_()
                head.bias[0] = bias
            self.fleet_join_query.weight.zero_()
            self.fleet_join_key.weight.copy_(torch.eye(self.fleet_join_key.in_features))

    def reset_build_heads(self):
        """Start the build head on building as the executor would in every
        slot, about 94% against the kinds and none, and the amount head
        even, the same every time."""
        with torch.no_grad():
            for head in (self.world_build, self.world_amount):
                head.weight.zero_()
                head.bias.zero_()
            self.world_build.bias.view(nc_ai_env.BUILD_SLOTS, -1)[:, nc_ai_env.BUILD_EXECUTOR] = 5.0

    def build_logits(self, batch, g, worlds, goals):
        """Per world and slot, ``build`` ``[N, W, S, BUILD_CHOICES]`` logits
        masked by the world's build mask, and ``amount`` ``[N, W, S,
        AMOUNTS]``, given the seat's ``goals``."""
        n = batch["n"]
        g, worlds = g[:n], worlds[:n]
        seat = torch.cat([
            g,
            torch.nn.functional.one_hot(goals["posture"], self.posture.out_features).to(g.dtype),
            torch.nn.functional.one_hot(goals["allocation"], self.allocation.out_features).to(g.dtype),
        ], -1)
        hidden = torch.relu(self.world_build_hidden(worlds) + self.world_build_seat(seat)[:, None]
                            + self.world_build_flags(goal_flags(goals, worlds)))
        slots = nc_ai_env.BUILD_SLOTS
        build = self.world_build(hidden).unflatten(-1, (slots, -1))
        mask = batch["mask_build"][:n, :, None, :]
        amount = self.world_amount(hidden).unflatten(-1, (slots, -1))
        return {"build": build.masked_fill(~mask, -torch.inf), "amount": amount}

    def build_dists(self, batch, g, worlds, goals):
        """:meth:`build_logits` as distributions."""
        return {name: torch.distributions.Categorical(logits=logits)
                for name, logits in self.build_logits(batch, g, worlds, goals).items()}

    def fleet_logits(self, batch, fleets, worlds, goals, mission):
        """Masked fleet-head logits for the real seats; ``mission`` is the
        chosen mission per fleet row, which the target head conditions on."""
        n = batch["n"]
        fleets, worlds = fleets[:n], worlds[:n]
        mission_logits = self.fleet_mission(fleets)
        query = self.target_query(fleets + self.mission_embed(mission))
        target_logits = (query @ worlds.transpose(1, 2)) / math.sqrt(fleets.shape[-1])
        target_logits = target_logits + self.goal_bias(goal_flags(goals, worlds))[:, None, :, 0]
        target_mask = torch.gather(
            batch["fleet_target"][:n],
            2,
            mission[:, :, None, None].expand(-1, -1, 1, worlds.shape[1]),
        )[:, :, 0, :]
        masked = lambda logits, mask: logits.masked_fill(~mask, -torch.inf)
        return {
            "mission": masked(mission_logits, batch["fleet_mission"][:n]),
            "target": masked(target_logits, target_mask),
            "waypoint": self.fleet_waypoint(fleets),
            "speed": self.fleet_speed(fleets),
            "roe": masked(self.fleet_roe(fleets), batch["fleet_roe"][:n]),
            "load": masked(self.fleet_load(fleets), batch["fleet_load"][:n]),
            "detach": masked(self.fleet_detach(fleets), batch["fleet_detach"][:n]),
            "join": masked(self.join_logits(fleets), batch["fleet_join"][:n]),
            "unload": masked(self.fleet_unload(fleets), batch["fleet_unload"][:n]),
        }

    def join_logits(self, fleets):
        """``[N, F, 1 + F]``: none, then each fleet row as the host."""
        hosts = (self.fleet_join_query(fleets) @ self.fleet_join_key(fleets).transpose(1, 2)
                 / math.sqrt(fleets.shape[-1]))
        return torch.cat([self.fleet_join(fleets), hosts], -1)

    def fleet_dists(self, batch, fleets, worlds, goals, mission):
        """Fleet-head distributions for the real seats, as
        :meth:`fleet_logits`."""
        return {name: torch.distributions.Categorical(logits=logits)
                for name, logits in self.fleet_logits(batch, fleets, worlds, goals,
                                                      mission).items()}

    def act(self, batch, greedy=False, fleet_choices=None, goal_choices=None, builds=False):
        """Goals and fleet choices, sampled (or greedy) unless given; the goal
        and per-fleet-row log-probabilities; the fleet heads' entropy; the
        value; and the goal and fleet distributions. With ``builds`` the
        goals also carry ``build`` and ``amount`` (see the module docs); an
        amount counts only in a slot that builds."""
        g, worlds, fleets = self.encode(batch)
        n = batch["n"]
        goal_logits, value = self.goal_heads(g, worlds, n)
        goal_dists = distributions(goal_logits, batch)
        pick = lambda dist: dist.probs.argmax(-1) if greedy else dist.sample()
        goals = goal_choices or {name: pick(dist) for name, dist in goal_dists.items()}
        mission = (fleet_choices["mission"] if fleet_choices
                   else pick(self.fleet_dists(batch, fleets, worlds, goals,
                                              torch.zeros_like(batch["fleets_valid"][:n],
                                                           dtype=torch.long))["mission"]))
        dists = self.fleet_dists(batch, fleets, worlds, goals, mission)
        if fleet_choices is None:
            fleet_choices = {"mission": mission}
            for name in FLEET_HEADS[1:]:
                fleet_choices[name] = pick(dists[name])
            if greedy:
                fleet_choices["join"] = greedy_join(dists["join"].logits)
        goal_logp = 0
        for name, dist in goal_dists.items():
            lp = dist.log_prob(goals[name])
            goal_logp = goal_logp + (lp.sum(-1) if lp.ndim > 1 else lp)
        if builds:
            build_dists = self.build_dists(batch, g, worlds, goals)
            if goal_choices is None or "build" not in goal_choices:
                goals = dict(goals)
                for name in BUILD_HEADS:
                    goals[name] = pick(build_dists[name])
                if greedy:
                    goals["build"] = greedy_build(build_dists["build"].logits)
            goal_logp = goal_logp + build_logp(build_dists, goals)
            goal_dists = {**goal_dists, **build_dists}
        fleet_logp = sum(dists[name].log_prob(fleet_choices[name]) for name in FLEET_HEADS)
        entropy = sum(dist.entropy() for dist in dists.values())
        return goals, fleet_choices, goal_logp, fleet_logp, entropy, value, goal_dists, dists


def from_strategist(path, dev):
    """A fleet agent whose shared parts start as the strategist at
    ``path``; the fleet heads start fresh."""
    checkpoint = torch.load(path, map_location=dev, weights_only=False)
    model = FleetAgent(**checkpoint["config"]).to(dev)
    missing, unexpected = model.load_state_dict(checkpoint["state"], strict=False)
    if unexpected or any(not key.startswith(("fleet_", "mission_embed", "target_query",
                                             "goal_bias", "world_")) for key in missing):
        raise SystemExit(f"strategist weights do not fit: {missing} {unexpected}")
    return model


def load_fleet_agent(path, dev):
    """A saved fleet agent; one saved before the join, unload or build heads
    gets them at their start (see :meth:`FleetAgent.reset_new_heads` and
    :meth:`FleetAgent.reset_build_heads`)."""
    checkpoint = torch.load(path, map_location=dev, weights_only=False)
    model = FleetAgent(**checkpoint["config"]).to(dev)
    missing, unexpected = model.load_state_dict(checkpoint["state"], strict=False)
    if unexpected or any(not key.startswith(FRESH_PREFIXES) for key in missing):
        raise SystemExit(f"fleet agent weights do not fit: {missing} {unexpected}")
    return model.eval()


def fleet_arrays(choices, select, fleets):
    """Env action arrays ``[games, seats, F]`` with ``choices`` (over the
    padded fleet rows of the selected seats) placed at ``select``."""
    games, seats = select.shape
    out = {}
    for name in FLEET_HEADS:
        array = np.zeros((games, seats, fleets), np.int64)
        array[select] = choices[name][:, :fleets].cpu().numpy()
        out[name] = array
    return out


def build_arrays(goals, select, worlds):
    """Env ``build`` and ``amount`` arrays ``[games, seats, W, BUILD_SLOTS]``
    with the selected seats' choices placed at ``select``."""
    games, seats = select.shape
    out = {}
    for name in BUILD_HEADS:
        array = np.zeros((games, seats, worlds, nc_ai_env.BUILD_SLOTS), np.int64)
        array[select] = goals[name][:, :worlds].cpu().numpy()
        out[name] = array
    return out
