# AI training tools

Python side of the [learned AI opponent plan](../../docs/notes/plans/qkgqypzw-learned-ai-opponent-harness-hierarchical-agent-and-fleet-level-bootstrap-on-the-mac-mini.md).
Rust plays the games through the `nc_ai_env` extension, built by maturin
from [`rust/nc-ai-py`](../../rust/nc-ai-py). PyTorch learns on the GPU (MPS
on the Mac mini). PyTorch, rather than MLX, can export a policy to ONNX for a
CPU actor on another host (t158).

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

## The environment

```python
import nc_ai_env

env = nc_ai_env.Env(players=4, years=60, threads=0)  # 0: every core
env.reset(map_seeds, campaign_seeds)  # campaign seeds default to map seeds
obs = env.observe()    # global, planets, fleets, contacts, starbases (+ *_valid)
masks = env.masks()    # diplomacy, build, mission, target, roe, load
step = env.step(actions)  # reward, done, production, worlds
```

Every array is indexed `[game, seat, ...]`, padded to the largest map (`W`,
`env.worlds`) and the most fleets any seat has (`F`, `env.fleets`). The
arrays own the Rust buffers, so nothing is copied. `actions` is a dict of
int64 arrays:

- `tax` `[G, S]`
- `diplomacy` `[G, S, S]`
- `build` and `amount` `[G, S, W]`
- `mission`, `target`, `speed`, `roe` and `load` `[G, S, F]`

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
