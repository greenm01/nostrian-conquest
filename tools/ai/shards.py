"""Experience shards: one Parquet row per learned seat per year of a live
game.

Entity tables and their masks are stored unpadded, as flat lists with a row
count, so a learner rebuilds each with ``rows(table, column, count, width)``.
Per row:

- ``map_seed``, ``campaign_seed``, ``game`` (batch slot), ``seat`` (0-based)
- ``global``: GLOBAL_WIDTH values; the year is ``global[0]``
- ``planets`` (``n_planets`` rows), ``fleets`` (``n_fleets``),
  ``contacts`` (``n_contacts``), ``starbases`` (``n_starbases``)
- masks: ``mask_diplomacy`` (seats x DIPLOMACY_CHOICES),
  ``mask_build`` (n_planets x BUILD_CHOICES),
  ``mask_mission`` (n_fleets x MISSION_CHOICES),
  ``mask_target`` (n_fleets x n_planets: targets allowed for the chosen
  mission), ``mask_roe`` (n_fleets x ROE_CHOICES), ``mask_load`` (n_fleets),
  ``mask_detach`` (n_fleets x DETACH_CHOICES)
- choices: ``tax``, ``diplomacy`` (seats), ``build`` and ``amount``
  (n_planets x BUILD_SLOTS), ``mission``, ``target``, ``speed``, ``roe``,
  ``load``, ``detach`` (n_fleets)
- ``reward`` and ``done`` from the step those choices played
"""

from pathlib import Path

import numpy as np
import pyarrow as pa
import pyarrow.parquet as pq

TABLES = ("planets", "fleets", "contacts", "starbases")


def ragged(values: np.ndarray, valid: np.ndarray) -> pa.ListArray:
    """One list per leading row: the values where ``valid`` holds, with any
    trailing axes beyond ``valid``'s flattened in."""
    per_item = int(np.prod(values.shape[valid.ndim :], dtype=np.int64))
    flat = values[valid].reshape(-1)
    counts = valid.reshape(len(valid), -1).sum(axis=1) * per_item
    offsets = np.concatenate([[0], np.cumsum(counts)]).astype(np.int32)
    return pa.ListArray.from_arrays(pa.array(offsets), pa.array(flat))


def experience_batch(
    map_seeds, campaign_seeds, learned, done_before, obs, masks, actions, step
):
    """The rows for one step. ``obs`` and ``masks`` are what the seats saw,
    ``actions`` what they chose and ``step`` what ``Env.step`` returned. Only
    ``learned`` seats (``Env.learned``) of games not ``done_before`` the step
    are kept."""
    games, seats = obs["global"].shape[:2]
    live = np.repeat(~done_before, seats) & learned.reshape(-1)
    flat = lambda a: a.reshape(games * seats, *a.shape[2:])[live]
    planets = flat(obs["planets_valid"])
    fleets = flat(obs["fleets_valid"])
    columns = {
        "map_seed": pa.array(np.repeat(map_seeds, seats)[live], pa.uint64()),
        "campaign_seed": pa.array(np.repeat(campaign_seeds, seats)[live], pa.uint64()),
        "game": pa.array(np.repeat(np.arange(games), seats)[live], pa.int32()),
        "seat": pa.array(np.tile(np.arange(seats), games)[live], pa.int8()),
        "global": ragged(flat(obs["global"]), np.ones(len(planets), bool)),
    }
    for name in TABLES:
        valid = flat(obs[f"{name}_valid"])
        columns[name] = ragged(flat(obs[name]), valid)
        columns[f"n_{name}"] = pa.array(valid.sum(axis=1), pa.int32())

    mission = flat(actions["mission"])
    chosen_targets = np.take_along_axis(
        flat(masks["target"]), mission[:, :, None, None], axis=2
    )[:, :, 0, :]
    every = np.ones((len(mission), seats), bool)
    columns |= {
        "mask_diplomacy": ragged(flat(masks["diplomacy"]), every),
        "mask_build": ragged(flat(masks["build"]), planets),
        "mask_mission": ragged(flat(masks["mission"]), fleets),
        "mask_target": ragged(chosen_targets, fleets[:, :, None] & planets[:, None, :]),
        "mask_roe": ragged(flat(masks["roe"]), fleets),
        "mask_load": ragged(flat(masks["load"]), fleets),
        "mask_detach": ragged(flat(masks["detach"]), fleets),
        "tax": pa.array(flat(actions["tax"]), pa.int8()),
        "diplomacy": ragged(flat(actions["diplomacy"]).astype(np.int8), every),
        "reward": pa.array(flat(step["reward"]), pa.float32()),
        "done": pa.array(np.repeat(step["done"], seats)[live]),
    }
    for name in ("build", "amount"):
        columns[name] = ragged(flat(actions[name]).astype(np.int8), planets)
    for name in ("mission", "target", "speed", "roe", "load", "detach"):
        columns[name] = ragged(flat(actions[name]).astype(np.int16), fleets)
    return pa.record_batch(columns)


class ShardWriter:
    """Writes record batches to ``out_dir/shard-NNNNN.parquet`` files of
    about ``rows_per_shard`` rows."""

    def __init__(self, out_dir, rows_per_shard=200_000):
        self.out_dir = Path(out_dir)
        self.out_dir.mkdir(parents=True, exist_ok=True)
        self.rows_per_shard = rows_per_shard
        self.pending = []
        self.rows = 0
        self.shards = 0

    def add(self, batch: pa.RecordBatch):
        self.pending.append(batch)
        self.rows += batch.num_rows
        if self.rows >= self.rows_per_shard:
            self.flush()

    def flush(self):
        if not self.pending:
            return
        path = self.out_dir / f"shard-{self.shards:05d}.parquet"
        pq.write_table(pa.Table.from_batches(self.pending), path, compression="zstd")
        self.shards += 1
        self.pending = []
        self.rows = 0

    def close(self):
        self.flush()


def read_shards(shard_dir) -> pa.Table:
    return pq.read_table(shard_dir)


def rows(table: pa.Table, column: str, count: np.ndarray, width: int):
    """Rebuild column ``column`` as one ``(count[i], width)`` array per row."""
    flat = table.column(column).combine_chunks()
    values = flat.values.to_numpy(zero_copy_only=False)
    starts = flat.offsets.to_numpy()[:-1]
    return [
        values[start : start + n * width].reshape(n, width)
        for start, n in zip(starts, count)
    ]
