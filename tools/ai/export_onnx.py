"""Export a fleet-level agent (t157) to ONNX for AI seats in real games
(t165), so hosts run it from Rust without Python or a GPU.

    uv run export_onnx.py fleet-final.pt --out fleet-final.onnx
    uv run export_onnx.py --fixture ../../rust/nc-ai-onnx/tests/fixtures

The graph plays one seat. It takes the seat's raw observation tables and
masks, as ``nc_ai_env`` gives them for one seat, and one noise tensor per
head, and returns each head's choice: the argmax of its masked logits plus
the noise. Zero noise plays greedy. Noise of ``T`` times standard Gumbel
samples draws from the softmax at temperature ``T`` (the Gumbel-max trick),
so the caller sets the difficulty. The fleet target is chosen after the
mission, conditioned on it, as in training.

Every table needs at least one row: pad with invalid rows whose masks allow
only the first choice, as ``fleet_policy.bucket`` does. The export is
checked against the PyTorch model on recorded seats, with and without
noise.

``--fixture`` writes the Rust tests' fixture instead: a small seeded
network, untrained, exported as ``tiny.onnx``, and a game it played greedy
in seat 1 against three executors, recorded as turn KDL in
``tiny-game.json``. The Rust seat must make the same turns.
"""

import argparse
import json
from pathlib import Path

import numpy as np
import torch
from torch import nn

import nc_ai_env
from fleet_policy import FLEET_HEADS, FleetAgent, fleet_batch, load_fleet_agent
from fleet_ppo import play_year
from policy import HEADS, START_YEAR, TABLES, masked, symlog

GOAL_HEADS = HEADS
INPUTS = (
    ["global"]
    + [n for table, _ in TABLES for n in (table, f"{table}_valid")]
    + ["mask_diplomacy", "mask_strike", "mask_colonize",
       "fleet_mission", "fleet_target", "fleet_roe", "fleet_load", "fleet_detach"]
)


class Deployed(nn.Module):
    def __init__(self, model):
        super().__init__()
        self.model = model

    def forward(self, *args):
        raw = dict(zip(INPUTS, args[: len(INPUTS)]))
        noise = dict(zip(GOAL_HEADS + FLEET_HEADS, args[len(INPUTS):]))
        batch = {"n": 1}
        glob = raw["global"].clone()
        glob[:, 0] -= START_YEAR
        batch["global"] = symlog(glob)
        for table, _ in TABLES:
            batch[table] = symlog(raw[table])
            batch[f"{table}_valid"] = raw[f"{table}_valid"]
        for name in INPUTS:
            if name.startswith("mask_") or name.startswith("fleet_"):
                batch[name] = raw[name]
        load = raw["fleet_load"]
        batch["fleet_load"] = torch.stack([torch.ones_like(load), load], -1)

        g, worlds, fleets = self.model.encode(batch)
        goal_logits, _ = self.model.goal_heads(g, worlds, 1)
        goal_logits = masked(goal_logits, batch)
        goals = {name: (goal_logits[name] + noise[name]).argmax(-1) for name in GOAL_HEADS}
        mission_logits = self.model.fleet_logits(
            batch, fleets, worlds, goals, torch.zeros_like(raw["fleets_valid"], dtype=torch.long)
        )["mission"]
        mission = (mission_logits + noise["mission"]).argmax(-1)
        logits = self.model.fleet_logits(batch, fleets, worlds, goals, mission)
        choices = {"mission": mission}
        for name in FLEET_HEADS[1:]:
            choices[name] = (logits[name] + noise[name]).argmax(-1)
        return tuple(goals[name] for name in GOAL_HEADS) + tuple(
            choices[name] for name in FLEET_HEADS)


def recorded_seats(count, years):
    """Real seat inputs: every seat of a few games against executors, one
    year at a time, until ``count`` seats are collected."""
    env = nc_ai_env.Env(4, years)
    env.reset(list(range(900000, 900004)),
              seats=[["hybrid:strike", "executor", "executor", "executor"]] * 4)
    seats = []
    while len(seats) < count:
        obs, masks, goal_masks = env.observe(), env.masks(), env.goal_masks()
        for game in range(4):
            select = np.zeros(env.hybrid.shape, bool)
            select[game, 0] = True
            batch = fleet_batch(obs, masks, goal_masks, select, torch.device("cpu"))
            seats.append(seat_inputs(obs, masks, goal_masks, game, batch))
        labels = env.goal_labels()
        fleets = env.fleet_labels(labels)
        step = env.step({**zero_actions(env), **{k: fleets[k] for k in FLEET_HEADS}}, labels)
        if step["done"].all():
            break
    return seats[:count]


def zero_actions(env):
    games, seats = env.hybrid.shape
    return {
        "tax": np.zeros((games, seats), np.int64),
        "diplomacy": np.zeros((games, seats, seats), np.int64),
        "build": np.zeros((games, seats, env.worlds, nc_ai_env.BUILD_SLOTS), np.int64),
        "amount": np.zeros((games, seats, env.worlds, nc_ai_env.BUILD_SLOTS), np.int64),
    }


def seat_inputs(obs, masks, goal_masks, game, batch):
    """One seat's graph inputs, raw, trimmed to its real rows (at least one
    row per table, padded with invalid rows that allow the first choice)."""
    take = lambda a: torch.from_numpy(np.ascontiguousarray(a[game, 0][None]))
    out = {"global": take(obs["global"])}
    rows = {}
    for table, _ in TABLES:
        valid = obs[f"{table}_valid"][game, 0]
        rows[table] = max(1, int(valid.sum()) if table != "planets" else len(valid))
        for name in (table, f"{table}_valid"):
            value = take(obs[name])[:, : rows[table]]
            short = rows[table] - value.shape[1]
            pad = [0, 0] * (value.ndim - 2) + [0, short]
            out[name] = torch.nn.functional.pad(value, pad) if short else value
    worlds = out["planets"].shape[1]
    f = rows["fleets"]
    out["mask_diplomacy"] = take(masks["diplomacy"])
    out["mask_strike"] = take(goal_masks["strike"])[:, : 1 + worlds]
    out["mask_colonize"] = take(goal_masks["colonize"])[:, : 1 + worlds]
    out["fleet_mission"] = take(masks["mission"])[:, :f].clone()
    out["fleet_target"] = take(masks["target"])[:, :f, :, :worlds].clone()
    out["fleet_roe"] = take(masks["roe"])[:, :f].clone()
    out["fleet_load"] = take(masks["load"])[:, :f].clone()
    out["fleet_detach"] = take(masks["detach"])[:, :f].clone()
    for name in ("fleet_mission", "fleet_roe", "fleet_detach"):
        empty = ~out[name].any(-1)
        out[name][..., 0] |= empty
    empty = ~out["fleet_target"].any(-1)
    out["fleet_target"][..., 0] |= empty
    return [out[name] for name in INPUTS]


def noise_like(inputs, deployed, scale, rng):
    """Gumbel noise of each head's shape, times ``scale``."""
    with torch.no_grad():
        shapes = head_shapes(inputs, deployed)
    return [torch.from_numpy((scale * rng.gumbel(size=shape)).astype(np.float32))
            for shape in shapes]


def head_shapes(inputs, deployed):
    raw = dict(zip(INPUTS, inputs))
    worlds = raw["planets"].shape[1]
    fleets = raw["fleets"].shape[1]
    players = raw["mask_diplomacy"].shape[1]
    model = deployed.model
    return [
        (1, model.tax.out_features),
        (1, model.posture.out_features),
        (1, model.allocation.out_features),
        (1, players, nc_ai_env.DIPLOMACY_CHOICES),
        (1, 1 + worlds),
        (1, 1 + worlds),
        (1, fleets, nc_ai_env.MISSION_CHOICES),
        (1, fleets, worlds),
        (1, fleets, nc_ai_env.WAYPOINT_CHOICES),
        (1, fleets, nc_ai_env.SPEED_CHOICES),
        (1, fleets, nc_ai_env.ROE_CHOICES),
        (1, fleets, 2),
        (1, fleets, nc_ai_env.DETACH_CHOICES),
    ]


def export(deployed, seats, out):
    """Export ``deployed`` to ``out`` and check it against PyTorch on
    ``seats``, greedy and sampled."""
    rng = np.random.default_rng(1)
    example = seats[len(seats) // 2]
    example_noise = noise_like(example, deployed, 0.0, rng)
    names = INPUTS + [f"noise_{h}" for h in GOAL_HEADS + FLEET_HEADS]
    worlds, fleets = torch.export.Dim("worlds", min=2), torch.export.Dim("fleets", min=1)
    contacts, starbases = torch.export.Dim("contacts", min=1), torch.export.Dim("starbases", min=1)
    dims = {"planets": {1: worlds}, "planets_valid": {1: worlds},
            "fleets": {1: fleets}, "fleets_valid": {1: fleets},
            "contacts": {1: contacts}, "contacts_valid": {1: contacts},
            "starbases": {1: starbases}, "starbases_valid": {1: starbases},
            "mask_strike": {1: worlds + 1}, "mask_colonize": {1: worlds + 1},
            "fleet_mission": {1: fleets}, "fleet_target": {1: fleets, 3: worlds},
            "fleet_roe": {1: fleets}, "fleet_load": {1: fleets},
            "fleet_detach": {1: fleets},
            "noise_strike": {1: worlds + 1}, "noise_colonize": {1: worlds + 1}}
    for head in FLEET_HEADS:
        dims[f"noise_{head}"] = {1: fleets}
    dims["noise_target"] = {1: fleets, 2: worlds}
    outputs = list(GOAL_HEADS) + list(FLEET_HEADS)
    program = torch.onnx.export(deployed, tuple(example + example_noise),
                                input_names=names, output_names=outputs,
                                dynamic_shapes=(tuple(dims.get(name) for name in names),),
                                opset_version=18, dynamo=True, external_data=False)
    program.save(str(out), external_data=False)

    import onnxruntime
    session = onnxruntime.InferenceSession(str(out))
    mismatches = 0
    for inputs in seats:
        for scale in (0.0, 1.0):
            noise = noise_like(inputs, deployed, scale, rng)
            with torch.no_grad():
                want = [t.numpy() for t in deployed(*inputs, *noise)]
            got = session.run(None, {n: t.numpy() for n, t in zip(names, inputs + noise)})
            mismatches += sum(not np.array_equal(a, b) for a, b in zip(want, got))
    print(f"exported {out}; {len(seats)} recorded seats, greedy and sampled: "
          f"{mismatches} mismatched outputs")
    if mismatches:
        raise SystemExit(1)


def fixture(directory):
    """The Rust tests' fixture; see the module docs."""
    directory = Path(directory)
    torch.manual_seed(7)
    model = FleetAgent(players=4, width=32, layers=1, heads=2).eval()
    export(Deployed(model).eval(), recorded_seats(16, 60), directory / "tiny.onnx")
    map_seed, years = 61, 30
    env = nc_ai_env.Env(4, years)
    env.reset([map_seed], seats=[["hybrid:strike", "executor", "executor", "executor"]])
    roles = np.array([[1, 0, 0, 0]], np.int8)
    done = np.zeros(1, bool)
    while not done.all():
        step, _, _ = play_year(env, model, None, roles, done, ["strike"],
                               torch.device("cpu"), greedy=True)
        done = step["done"].copy()
    record = {"map_seed": map_seed, "campaign_seed": map_seed, "years": years,
              "result": env.result(0), "turns": env.turns(0)}
    (directory / "tiny-game.json").write_text(json.dumps(record, indent=1) + "\n")
    print(f"recorded a {record['result']['years']}-year game in {directory}")


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("checkpoint", nargs="?")
    parser.add_argument("--out")
    parser.add_argument("--fixture", help="write the Rust test fixture to this directory")
    parser.add_argument("--check-seats", type=int, default=64)
    args = parser.parse_args()
    if args.fixture:
        fixture(args.fixture)
        return
    if not (args.checkpoint and args.out):
        parser.error("give a checkpoint and --out, or --fixture")
    deployed = Deployed(load_fleet_agent(args.checkpoint, torch.device("cpu"))).eval()
    export(deployed, recorded_seats(args.check_seats, 60), args.out)


if __name__ == "__main__":
    main()
