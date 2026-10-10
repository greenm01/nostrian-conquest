# Nostrian Conquest

_Nostrian Conquest is a from-scratch Rust recreation of the 1990's BBS door game Esterian Conquest. It currently ships as a direct localhost client and BBS door stack, with a planned Nostr GameServer path as the modern network mode. All code is original. It is not affiliated with any historical release._

**Status:** `v1.0.0-beta.3`  
Active beta. The Rust player and sysop stack is playable now. The main work is live playtesting, bug fixing, and tightening the rough edges.

![Nostrian Conquest title screen](docs/assets/nc-hero.png)

[View screenshots](https://nostrian-conquest.com/screenshots.html)

## What It Is

Beyond the old Nostrian frontier lies an abandoned galaxy. The old stations are silent.
The borders are open. You start with four fleets, an isolated homeworld, and enough industry
to kickstart an empire.

NC preserves the yearly-turn campaign rhythm, the reports, the maps, and the
old-school pressure of the original game. The engine is modern Rust. Campaigns
run from SQLite. Classic EC compatibility stays at the oracle and import/export
boundary.

If you want the recovery background, see
[How the Game Was Recovered](docs/dev/approach.md#how-the-game-was-recovered).

## Current Public Roles

Keep the binaries straight:

- `nc-game`: direct localhost player client
- `nc-door`: BBS door entrypoint
- `nc-sysop`: sysop and BBS/local campaign administration tool
- `nc-helm`: experimental TEA-based hosted player client for the Nostr path
- `nc-cli`: internal developer, oracle, and compatibility tool

A normal non-BBS game directory contains `ncgame.db` and nothing else. BBS
campaigns add a small per-game `config.kdl` beside it for `players` and
reserved aliases. That runtime DB is scoped to localhost/BBS play; the planned
Nostr GameServer path is expected to use its own schema.

## Play

NC is built for native Windows, Linux, and macOS clients. No web app is
required. Use `nc-game` for direct same-machine play, `nc-door` when the host
is a BBS, and keep the planned Nostr GameServer path in mind as the intended
modern network mode.

### Public Beta Downloads

During the current beta, public GitHub Releases publish `nc-sysop` archives for
Windows x64, Windows x86 (32-bit), Windows 7+ x86 (32-bit), and Linux x64.
Those archives are the public BBS/sysop package and include `nc-door`,
`nc-sysop`, and the manuals. Localhost play remains a source-build path. See
[Release Policy](docs/release-policy.md).

## Learn The Game

Start with the current Rust manuals:

- **[NC Player Manual (PDF)](docs/manuals/nc_player_manual.pdf)**
- **[NC Sysop Manual (PDF)](docs/manuals/nc_sysop_manual.pdf)**

Historical `.DOC` files remain preserved in [original/v1.5](original/v1.5).

## Quick Start

### 1. Run One Local Game

Create one game:

```bash
cd rust
cargo run -q -p nc-sysop -- new-game /srv/nc/games/friday-night --name "Friday Night NC" --players 4
```

That game directory contains one runtime file:

```text
/srv/nc/games/friday-night/
  ncgame.db
```

Open the player client directly:

```bash
cd rust
cargo run -q -p nc-game --bin nc-game -- --dir /srv/nc/games/friday-night --player 1
```

Run maintenance when needed:

```bash
cd rust
cargo run -q -p nc-sysop -- maint /srv/nc/games/friday-night 1
```

IMPORTANT: use `--bin nc-game` in source builds. The `nc-game` package also
ships `nc-door`, so plain `cargo run -p nc-game` is ambiguous.

### 2. Run `nc-door` As A BBS Door

For BBS campaigns, write a minimal per-game `config.kdl` first. For example:

```kdl
players 4
reservations {
  seat player=1 alias="SYSOP"
}
```

Then initialize the campaign in BBS mode:

```bash
cd rust
cargo run -q -p nc-sysop -- new-game --bbs /srv/nc/games/night-shift
```

BBS campaigns keep that `config.kdl` beside `ncgame.db`.

Stage `nc-door` as the live BBS binary. For working host setups, see:

- [Mystic BBS Setup](docs/sysop/bbs/mystic-bbs-setup.md)
- [Synchronet BBS Setup](docs/sysop/bbs/synchronet-bbs-setup.md)
- [ENiGMA½ BBS Setup](docs/sysop/bbs/enigma-bbs-setup.md)
- [WWIV BBS Setup](docs/sysop/bbs/wwiv-bbs-setup.md)

### 3. Planned Nostr GameServer Path

The planned modern network path is a Nostr-backed GameServer with `nc-helm` as
the player client. That stack is still under construction, but it is the
intended long-term way to host and join NC over the network without direct BBS
or localhost access.

`nc-helm` is the hosted client architecture. Today it implements the local
bootstrap, encrypted SQLite keychain flow, lobby shell, and background
catalog/notices sync on a fresh `winit`/`wgpu`/`glyphon` runtime. Hosted
dashboard edits stage local turn drafts for build, fleet, tax, diplomacy, and
quick in-game message orders so they survive returning to the lobby and
reopening the game.

Try the new client with:

```bash
cd rust
cargo run -q -p nc-helm -- --windowed
```

Or open a seeded local dashboard directly:

```bash
cd rust
cargo run -q -p nc-helm -- --dir /tmp/nc-helm-lab/map45-p25 --windowed
```

## Operator Docs

- [NC Sysop Manual (PDF)](docs/manuals/nc_sysop_manual.pdf)
- [Sysop Documentation Index](docs/sysop/README.md)
- [NC Player Manual (PDF)](docs/manuals/nc_player_manual.pdf)
- [Nostr Roadmap Notes](docs/nostr/README.md)

## Developer Commands

Inspect a game directory:

```bash
cd rust
cargo run -q -p nc-cli -- core-report /tmp/nc-game
```

Inspect player mail:

```bash
cd rust
cargo run -q -p nc-cli -- inspect-messages /tmp/nc-game
```

Seed one `nc-helm` stress lab with all four map-size tiers:

```bash
cd rust
cargo run -q -p nc-cli -- harness seed-nc-helm-lab --root /tmp/nc-helm-lab
```

Or use the repo wrapper from the root:

```bash
python3 scripts/setup_nc_helm_lab.py --root /tmp/nc-helm-lab --force
```

Submit a turn file without opening the TUI:

```bash
cd rust
cargo run -q -p nc-game --bin nc-game -- submit-turn --check --dir /tmp/nc-game --player 1 --file /tmp/turn.kdl
```

Fuzz the engine with generated multi-turn games. A short run of each driver
is part of `cargo test`; `NC_FUZZ_CASES` sets the number of games and
`NC_FUZZ_TURNS` the most turns per game for longer runs:

```bash
cd rust
NC_FUZZ_CASES=2000 NC_FUZZ_TURNS=12 cargo test -p nc-fuzz --test hegel_driver
NC_FUZZ_CASES=2000 cargo test -p nc-fuzz --test proptest_driver
```

A failing game is shrunk and written as an `nc-harness` combat scenario with
its turn files under `$NC_FUZZ_OUT/<driver>` (default `nc-fuzz/<driver>` in
the system temp directory). Replay it with:

```bash
cargo run -q -p nc-cli -- harness run-combat --file /tmp/nc-fuzz/hegel/scenario.kdl
```

`nc-cli` remains the internal developer, oracle, and compatibility surface.
Normal player and sysop workflows should prefer `nc-game`, `nc-door`, and
`nc-sysop`.

## Local Dependencies

- Rust toolchain
- Python 3
- `sccache` (recommended)

### NixOS

For NixOS, enter the repo dev shell before building:

```bash
nix develop
cd rust
cargo build
```

The flake shell provides the Linux desktop/system libraries used by the native
clients (`winit`/`wgpu`, X11, Wayland, clipboard support) plus the normal Rust
tooling.

For DOSBox-X, Ghidra, and other compatibility tooling, see the contributor
docs under `docs/dev/`.

## For Contributors

Read these first:

- [docs/dev/next-session.md](docs/dev/next-session.md)
- [docs/dev/approach.md](docs/dev/approach.md)
- [docs/dev/rust-architecture.md](docs/dev/rust-architecture.md)
- [docs/dev/work-tracking.md](docs/dev/work-tracking.md)

## Repository Layout

- `original/`: preserved original binaries and manuals
- `docs/`: manuals, sysop docs, and engineering notes
- `rust/`: engine, clients, sysop tools, and support crates
- `tools/`: oracle runners and analysis scripts
- `scripts/`: install, packaging, and release helpers

## License

Source code and tooling are licensed under the **O'Saasy License Agreement**.
See [LICENSE](LICENSE).

The repository also preserves original `Esterian Conquest` materials for
research and compatibility work, but Nostrian package archives do not include
them.
