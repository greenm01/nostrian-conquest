# AI training tools

Python side of the [learned AI opponent plan](../../docs/notes/plans/qkgqypzw-learned-ai-opponent-harness-hierarchical-agent-and-fleet-level-bootstrap-on-the-mac-mini.md).
Rust plays the games through the `nc_ai_env` extension, built by maturin
from [`rust/nc-ai-py`](../../rust/nc-ai-py). PyTorch learns on the GPU (MPS
on the Mac mini). PyTorch, rather than MLX, exports a trained agent to ONNX,
which the game runs from Rust ([`rust/nc-ai-onnx`](../../rust/nc-ai-onnx)).

These are the repository's only Python dependencies. The game's own build
never needs them: `nc-ai-py` sits outside the `rust/` workspace.

## Setup

Install [uv](https://docs.astral.sh/uv/), then from this directory:

```sh
uv sync
```

`uv sync` builds the extension in release mode and rebuilds it when a Rust
source under `rust/` changes. To force a rebuild:

```sh
uv sync --reinstall-package nc-ai-env
```

## Play and learn

```sh
uv run random_agent.py --out /tmp/nc-shards --games 256 --years 60
uv run learner.py /tmp/nc-shards
uv run python -m unittest discover -s tests
```

`random_agent.py` plays seeded games with random choices inside the masks,
writes [Parquet shards](shards.py), and prints throughput and seat-mask cost.
`learner.py` reads the shards and fits each fleet's mission on the GPU; on
random play its loss should settle at the uniform-within-mask baseline it
prints.

## Train the strategist (t155)

```sh
uv run imitate.py --out /tmp/nc-ai/imitate.pt
uv run ppo.py --init /tmp/nc-ai/imitate.pt --out /tmp/nc-ai/ppo
uv run evaluate.py --policy /tmp/nc-ai/ppo/latest.pt --opponent executor
```

- [`policy.py`](policy.py): the strategist network, an entity
  transformer with a head for each goal and a value head.
- [`imitate.py`](imitate.py): online DAgger toward the executor's own goals.
- [`ppo.py`](ppo.py): PPO against the tuned executor, Aggressor and frozen
  snapshots. Its `log.jsonl` records each iteration.
- [`evaluate.py`](evaluate.py): win share of one strategist seat against
  three scripted seats, rotated across held-out maps 100000 onward. An
  even seat wins 0.25. `--policy labels` plays the executor's own goals,
  the reference to beat.

Run long jobs under `caffeinate -i` so the mini does not sleep, and run one
PPO job at a time: two at once push the mini into heavy swap.

## The league (t156)

```sh
uv run league.py --db league.db add-agent executor --scripted executor
uv run league.py --db league.db add-agent final --checkpoint strategist-final.pt
uv run league.py --db league.db play --games 1024
uv run league.py --db league.db ratings
uv run league.py --db league.db verify
```

[`league.py`](league.py) plays seat-rotated four-player matches between
registered agents:

- **Agents:** scripted policies (`executor`, `baseline`, `classic`,
  `aggressor`, `autopilot`) or strategist checkpoints. A checkpoint's
  SHA-256 is checked before every run.
- **Stored per match:** in SQLite, its seeds, seats, result, the commit it
  was played at, and every turn applied, as compressed turn KDL.
- **Ratings:** OpenSkill's Plackett-Luce model ranks seats by win share,
  then production. It works like TrueSkill, without TrueSkill's patent and
  trademark. The table shows mu - 3 sigma.
- **`verify`:** replays every match from its seeds and turns through
  `nc_ai_env.replay`, checks each stored result, and rebuilds the rating
  table from the replays.

Map seeds from 5,000,000 keep league games apart from training and
evaluation maps.

## Export an agent for real games (t165)

```sh
uv run export_onnx.py fleet-final.pt --out fleet-final.onnx
uv run export_onnx.py --fixture ../../rust/nc-ai-onnx/tests/fixtures
```

The first command exports a fleet-level agent for `nc_ai_onnx::NetworkSeat`.
The graph plays one seat: raw observation and masks in, each head's choice
out. It takes Gumbel noise per head, so the caller sets the temperature.
The export is checked against PyTorch on recorded seats, greedy and
sampled.

The second command rewrites the Rust test fixture: a small seeded network
and a game it played. Rerun it after changing the observation, the masks
or the heads.

## Splitting executor tests across machines (t189)

`split_matches.py` runs an `executor_matches` test on this machine and one
remote together, half the games each by map range, and prints the combined
result (the mean of the halves, and the standard error as
`sqrt(se1^2 + se2^2) / 2`). Both machines need the same commit and the
examples built; the engine is deterministic, so a half gives the same
numbers on either. Printed means are the halves' rounded values averaged,
so the third decimal can differ from one run by 0.001.

```sh
python3 split_matches.py --remote bean-void -- --weights tuned --opponent tuned \
    --with watch --games 1024 --years 100 --first-map 1200000 --paired
```

## The environment

```python
import nc_ai_env

env = nc_ai_env.Env(players=4, years=60, threads=0)  # 0: every core
env.reset(map_seeds, campaign_seeds, seats)  # seats: one lineup per game
obs = env.observe()    # global, planets, fleets, contacts, starbases (+ *_valid)
masks = env.masks()    # diplomacy, build, mission, target, roe, load, detach
goal_masks = env.goal_masks()  # strike, colonize
labels = env.goal_labels()     # the executor's own goals
step = env.step(actions, goals)  # reward, done, production, worlds
```

Each seat in a lineup is one of:

- `learned`: takes `actions`, the whole turn;
- `strategist`: takes `goals`, which the tuned executor carries out;
- `executor`, `baseline`, `aggressor` or `autopilot`: scripted.

Seats are all learned by default. `env.learned` and `env.strategist` flag
the seats each kind of choice is read for.

Every array is indexed `[game, seat, ...]`, padded to the largest map (`W`,
`env.worlds`) and the most fleets any seat has (`F`, `env.fleets`). The
arrays own the Rust buffers, so nothing is copied. `actions` is a dict of
int64 arrays:

- `tax` `[G, S]`
- `diplomacy` `[G, S, S]`
- `build` and `amount` `[G, S, W, BUILD_SLOTS]`: up to two builds per world
  a year, sharing its budget
- `mission`, `target`, `waypoint`, `speed`, `roe`, `load` and `detach`
  `[G, S, F]`; `waypoint` stops a Move, Patrol, Scout Sector or Rendezvous
  short of its target world, in deep space;
  `detach` keeps the fleet whole (0) or splits off half its warships, one
  warship or its transports, which wait on Hold until the next year

`goals` is a dict of int64 arrays, shaped as `env.goal_labels()`:

- `tax`, `posture`, `allocation`, `strike` and `colonize` `[G, S]`
- `diplomacy` `[G, S, S]`

The fields:

- `posture`: balanced, expand, defend or attack build targets.
- `allocation`: 0 strikes with everything; 1 also guards threatened
  worlds; 2 also keeps half of each striking home force at home. The label
  is the executor's rule, by threat and opportunity.
- `strike`: 0 holds every strike. `k` makes world `k - 1` the target of
  every home force strong enough to take it.
- `colonize`: 0 lets ETACs choose. `k` sends the first free ETAC to world
  `k - 1`.

See `rust/nc-ai/src/goals.rs`.

A choice outside the masks raises `ValueError` and leaves every game as it
was. Rewards are paid when a game ends: the winner takes 1. Without a declared
winner, the empires with the most production share it. Row layouts are
documented in `rust/nc-ai/src/observe.rs` and `rust/nc-ai/src/action.rs`.

## macOS 27 linker workaround

Xcode 27's `ld` can leave a dylib's string pool 4 bytes off 8-byte alignment,
and macOS 27's dyld then refuses to load it ("mis-aligned LINKEDIT string
pool"). `rust/nc-ai-py/.cargo/config.toml` links through `macos-link.sh`,
which realigns the pool and re-signs the file. The crate's release profile
turns off `strip`, which would undo the fix.
