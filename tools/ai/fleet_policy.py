"""The fleet-level agent (t157): the strategist network plus per-fleet
heads that order the seat's own fleets.

Each own-fleet token chooses a mission, then a target world (pointing at
world tokens, conditioned on the chosen mission and on the worlds the seat's
strike and colony goals name), a speed, rules of engagement and whether to
load armies. In a hybrid seat only the fleets of handed-over families
(colonize 0, scout 1, strike 2) take these choices; the executor orders the
rest.
"""

import math

import numpy as np
import torch
from torch import nn

import nc_ai_env
from policy import HEADS, Strategist, distributions, symlog, START_YEAR, TABLES

FLEET_HEADS = ("mission", "target", "speed", "roe", "load")
SEAT_BUCKET = 128
ROW_BUCKET = 16


def fleet_batch(obs, masks, goal_masks, select, dev):
    """Seat tensors as ``policy.seat_batch``, plus each fleet row's masks:
    ``fleet_mission`` ``[N, F, MISSIONS]``, ``fleet_target`` ``[N, F,
    MISSIONS, W]``, ``fleet_roe`` and ``fleet_load`` ``[N, F, 2]``."""
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
        elif table in {t for t, _ in TABLES} and table != "planets":
            pad[-3] = _round_up(value.shape[1], ROW_BUCKET) - value.shape[1]
        padded = torch.nn.functional.pad(value, pad)
        if name.startswith("mask_") or name.startswith("fleet_"):
            empty = ~padded.any(-1)
            padded[..., 0] |= empty
        out[name] = padded
    return out


class FleetAgent(Strategist):
    def __init__(self, players=4, width=128, layers=3, heads=4):
        super().__init__(players, width, layers, heads)
        missions = nc_ai_env.MISSION_CHOICES
        self.fleet_mission = nn.Linear(width, missions)
        self.mission_embed = nn.Embedding(missions, width)
        self.target_query = nn.Linear(width, width)
        self.goal_bias = nn.Linear(2, 1, bias=False)
        self.fleet_speed = nn.Linear(width, nc_ai_env.SPEED_CHOICES)
        self.fleet_roe = nn.Linear(width, nc_ai_env.ROE_CHOICES)
        self.fleet_load = nn.Linear(width, 2)

    def fleet_dists(self, batch, fleets, worlds, goals, mission):
        """Fleet-head distributions for the real seats; ``mission`` is the
        chosen mission per fleet row, which the target head conditions on."""
        n = batch["n"]
        fleets, worlds = fleets[:n], worlds[:n]
        mission_logits = self.fleet_mission(fleets)
        query = self.target_query(fleets + self.mission_embed(mission))
        target_logits = (query @ worlds.transpose(1, 2)) / math.sqrt(fleets.shape[-1])
        # Flag the worlds the seat's own strike and colony goals name.
        flags = torch.zeros(*worlds.shape[:2], 2, device=worlds.device)
        for column, name in enumerate(("strike", "colonize")):
            goal = goals[name]
            named = goal > 0
            flags[named, (goal[named] - 1).clamp(min=0), column] = 1.0
        target_logits = target_logits + self.goal_bias(flags)[:, None, :, 0]
        target_mask = torch.gather(
            batch["fleet_target"][:n],
            2,
            mission[:, :, None, None].expand(-1, -1, 1, worlds.shape[1]),
        )[:, :, 0, :]
        masked = lambda logits, mask: logits.masked_fill(~mask, -torch.inf)
        return {
            "mission": torch.distributions.Categorical(
                logits=masked(mission_logits, batch["fleet_mission"][:n])),
            "target": torch.distributions.Categorical(
                logits=masked(target_logits, target_mask)),
            "speed": torch.distributions.Categorical(logits=self.fleet_speed(fleets)),
            "roe": torch.distributions.Categorical(
                logits=masked(self.fleet_roe(fleets), batch["fleet_roe"][:n])),
            "load": torch.distributions.Categorical(
                logits=masked(self.fleet_load(fleets), batch["fleet_load"][:n])),
        }

    def act(self, batch, greedy=False, fleet_choices=None, goal_choices=None):
        """Goals and fleet choices, sampled (or greedy) unless given, with
        the goal and per-fleet-row log-probabilities and the value."""
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
        goal_logp = 0
        for name, dist in goal_dists.items():
            lp = dist.log_prob(goals[name])
            goal_logp = goal_logp + (lp.sum(-1) if lp.ndim > 1 else lp)
        fleet_logp = sum(dists[name].log_prob(fleet_choices[name]) for name in FLEET_HEADS)
        entropy = sum(dist.entropy() for dist in dists.values())
        return goals, fleet_choices, goal_logp, fleet_logp, entropy, value, goal_dists


def from_strategist(path, dev):
    """A fleet agent whose shared parts start as the strategist at
    ``path``; the fleet heads start fresh."""
    checkpoint = torch.load(path, map_location=dev, weights_only=False)
    model = FleetAgent(**checkpoint["config"]).to(dev)
    missing, unexpected = model.load_state_dict(checkpoint["state"], strict=False)
    if unexpected or any(not key.startswith(("fleet_", "mission_embed", "target_query",
                                             "goal_bias")) for key in missing):
        raise SystemExit(f"strategist weights do not fit: {missing} {unexpected}")
    return model


def load_fleet_agent(path, dev):
    checkpoint = torch.load(path, map_location=dev, weights_only=False)
    model = FleetAgent(**checkpoint["config"]).to(dev)
    model.load_state_dict(checkpoint["state"])
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
