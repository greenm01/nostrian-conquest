# Developer commands

Start with the [README's local game workflow](../../README.md#run-a-local-game).
These commands are for inspecting data, testing rules, and working on unfinished
features. Use a disposable campaign rather than a live game.

Unless noted otherwise, run commands from `rust/`. The examples use the practice
game at `$HOME/nc-games/practice`. On shared development machines, agents must
run Cargo through `cargo-slot nostrian-conquest cargo …`, as required by
[AGENTS.md](../../AGENTS.md). `sccache` is optional but can shorten repeat builds.

## Inspect and exchange game data

Inspect the campaign and its player mail:

```sh
cargo run -q -p nc-cli -- core-report "$HOME/nc-games/practice"
cargo run -q -p nc-cli -- inspect-messages "$HOME/nc-games/practice"
```

The runtime uses `ncgame.db`, not classic `.DAT` files. Export a separate classic
directory for compatibility checks, or import classic files into a store:

```sh
cargo run -q -p nc-cli -- db-export "$HOME/nc-games/practice" "$HOME/nc-games/practice-classic"
cargo run -q -p nc-cli -- db-import "$HOME/nc-games/practice-classic"
```

Import and export are compatibility operations, not backups of modern account
bindings or runtime state. Community backups use the
[nc-admin workflow](../../tools/nc-admin/README.md).

## Validate scripted orders

Write a [turn KDL file](../sysop/rust/turn-kdl.md), then check it without opening
the player terminal. Replace the example filename with your own:

```sh
cargo run -q -p nc-game --bin nc-game -- submit-turn --check --dir "$HOME/nc-games/practice" --player 1 --file /path/to/turn.kdl
```

## Fuzz and replay

A short run of each driver is included in `cargo test`. Those runs use repeatable
inputs. For longer runs, `NC_FUZZ_CASES` sets the number of games and
`NC_FUZZ_TURNS` limits each game's length. A seed selects another batch;
`HEGEL_DERANDOMIZE=false` enables random exploration.

```sh
env NC_FUZZ_CASES=2000 NC_FUZZ_TURNS=12 HEGEL_SEED=7 cargo test -p nc-fuzz --test hegel_driver
env NC_FUZZ_CASES=2000 PROPTEST_RNG_SEED=7 cargo test -p nc-fuzz --test proptest_driver
```

A failure is reduced to a smaller `nc-harness` scenario with its turn files.
The files go under `$NC_FUZZ_OUT/<driver>`, or `nc-fuzz/<driver>` in the system
temporary directory when that variable is unset. Replay a saved failure:

```sh
cargo run -q -p nc-cli -- harness run-combat --file /tmp/nc-fuzz/hegel/scenario.kdl
```

Failures worth keeping belong in `rust/nc-fuzz/tests/scenarios/`. The checks in
`rust/nc-fuzz/tests/replayed_failures.rs` replay them. Those paths are relative
to the repository root.

## Measure performance and balance

Measure maintenance turns per second and games per hour for setups with 2–25
players:

```sh
cargo run --release -p nc-fuzz --example maint_throughput
```

The [combat models](../../tools/combat-model/README.md) compare candidate rules
without building Rust. The `combat_odds` example runs engine trials so model
results can be checked against actual combat. Read the
[comparison workflow](combat-balance-workflow.md) before interpreting agreement.

From the repository root:

```sh
python3 -I -B tools/combat-model/sweep.py --markdown /tmp/sweep.md --json /tmp/sweep.json
```

From `rust/`:

```sh
cargo run --release -p nc-fuzz --example combat_odds -- --trials 20000 --json /tmp/engine.json
```

From the repository root, compare using the preset that matches the engine
revision being tested:

```sh
python3 -I -B tools/combat-model/sweep.py --rules ENGINE_PRESET --engine-rules ENGINE_PRESET --units UNIT_PRESET --cases engine --engine /tmp/engine.json
```

The old CRT preset is not the current game's combat rule. Presets and their
assumptions are documented in the model README.

[Hostile-map price controls](../../tools/price-campaigns/README.md) run scripted
campaigns through the engine and record expansion, purchases, losses, and
captures. Their results describe those policies; they are not human win odds.

## Work on the Nostr client

`nc-helm` and Nostr hosting remain in development. These commands are local
experiments, not a way to join a public game. Read the
[Nostr notes](../nostr/README.md) and
[client architecture](../helm/lobby-architecture.md) for context.

Open the client or generate a lab with all four map-size tiers:

```sh
cargo run -q -p nc-helm -- --windowed
cargo run -q -p nc-cli -- harness seed-nc-helm-lab --root /tmp/nc-helm-lab
cargo run -q -p nc-helm -- --dir /tmp/nc-helm-lab/map45-p25 --windowed
```

The equivalent lab helper runs from the repository root. `--force` replaces
existing lab output; use it only for a disposable lab:

```sh
python3 scripts/setup_nc_helm_lab.py --root /tmp/nc-helm-lab --force
```

For Nostr host development, `nc-host serve` handles submitted turns when its
schedule is due; `nc-host maint <dir> [turns]` advances them manually. Follow the
host's own configuration and protocol docs. These commands are separate from
`nc-host admin`, which the community browser host uses.
