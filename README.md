# Nostrian Conquest

Nostrian Conquest is an asynchronous space strategy game inspired by the 1992
BBS game Esterian Conquest. Build fleets, settle worlds, and outmaneuver rival
empires. Issue orders when it suits you; return after the next turn to see the
results.

**Version:** `v1.0.0-beta.4` — playable beta, with active testing and bug fixes.

![Nostrian Conquest title screen](docs/assets/nc-hero.png)

Beyond the old Nostrian frontier lies a galaxy of contested worlds. You begin
with one planet and a few small fleets. Explore, build, bargain, and fight for
an empire. Victory is decided by industry, cunning, and will.

## Play

**[Play in your browser](https://play.nostrian-conquest.com).** No game client
needs to be installed. The browser terminal uses xterm.js to keep the ANSI
colors, text menus, and keyboard commands of a BBS.

1. Open the [game list](https://play.nostrian-conquest.com) and sign in with
   Discord, Nostr (a NIP-07 extension such as nos2x) or Matrix.
2. Find a recruiting game. For an invite-only game, get a private code from its
   host and enter it on the game card.
3. Choose *Play*. Return later by signing in the same way.

Players using any of the three share the same games. A code works once across
all sign-in methods.

Meet other players, find new games and ask for invite codes in the
[Discord server](https://discord.gg/eZMRpdQTu) or the
[`#nostrian:matrix.org`](https://matrix.to/#/#nostrian:matrix.org) Matrix
space. Discord players can also join and play with the bot's `/games`, `/join`
and `/play` commands. New here? Read
[How to play](https://play.nostrian-conquest.com/howto), or
[contact the sysop](https://play.nostrian-conquest.com/contact).


Each turn represents a year. Maintenance resolves everyone's orders on the
host's schedule; you do not need to stay online. A recruiting game waits for
its host to start it, even when all seats are taken. The pace leaves time to
plan and negotiate. Keep a printed map or spreadsheet beside your terminal.

Traditional BBS play remains available: connect to a board that hosts NC and
open it from the doors menu. You can also run a local game from source, as
shown below.

[View screenshots](https://nostrian-conquest.com/screenshots.html)

## Manuals

- **[Quick Start](https://nostrian-conquest.com/manuals/nc_quick_start.pdf)** — your first orders and turn.
- **[Player Manual](https://nostrian-conquest.com/manuals/nc_player_manual.pdf)** — joining games, rules, and reference tables.
- **[Sysop Manual](https://nostrian-conquest.com/manuals/nc_sysop_manual.pdf)** — local and BBS setup and administration.

The [manuals page](https://nostrian-conquest.com/manuals.html) opens the PDFs in
your browser. Their [sources and PDFs](docs/manuals/) are also in this repository.

## Host a game

For **browser games**, use the
[community hosting guide](docs/dev/community-hosting.md). The Python
[`nc-admin` CLI](tools/nc-admin/README.md) manages games, seats, invite codes,
schedules, and backups over SSH. No dashboard is required. Hosts choose when
to start each game. Daily local-midnight schedules follow daylight saving.
The hosting stack is in beta; the guide records its remaining acceptance work.

For **BBS hosting**, get the packages from
[GitHub Releases](https://github.com/greenm01/nostrian-conquest/releases).
They include `nc-door`, `nc-sysop`, and the manuals. Packages are available for
Windows x64, Windows x86, Windows 7+ x86, and Linux x64. See the
[release policy](docs/release-policy.md) and
[signature verification instructions](docs/release-signing.md).

Board setup guides: [Mystic](docs/sysop/bbs/mystic-bbs-setup.md),
[Synchronet](docs/sysop/bbs/synchronet-bbs-setup.md),
[ENiGMA½](docs/sysop/bbs/enigma-bbs-setup.md), and
[WWIV](docs/sysop/bbs/wwiv-bbs-setup.md).

## Build and contribute

The game engine is written in Rust. Campaigns use SQLite; classic EC files can
be imported or exported through separate compatibility tools. See
[how the game was recovered](docs/dev/approach.md#how-the-game-was-recovered)
for the history and [the architecture guide](docs/dev/rust-architecture.md)
for the code structure.

### Run a local game

Install Rust and a C compiler and linker. NC builds its bundled SQLite
dependency from source. Python 3 is used by the repository's administration
and development scripts. On Linux, `nix develop` provides a development shell
if you use Nix; enter it from the
repository root before the commands below.

Clone the repository and build the local player and sysop tools:

```sh
git clone https://github.com/greenm01/nostrian-conquest.git
cd nostrian-conquest/rust
cargo build -p nc-game -p nc-sysop
```

Run the remaining commands from `rust/`. Choose an unused directory for the
practice game; this example keeps it under your home directory.

```sh
cargo run -q -p nc-sysop -- new-game "$HOME/nc-games/practice" --name "Practice" --players 4
cargo run -q -p nc-game --bin nc-game -- --dir "$HOME/nc-games/practice" --player 1
```

The first command creates the campaign and its `ncgame.db`. The second opens
player 1's terminal. Name your empire, issue orders, and quit the client. Then
advance one year and return to read the results:

```sh
cargo run -q -p nc-sysop -- maint "$HOME/nc-games/practice" 1
cargo run -q -p nc-game --bin nc-game -- --dir "$HOME/nc-games/practice" --player 1
```

Keep `--bin nc-game`: that package also builds `nc-door`.

### Test and explore

Run the engine, player, and sysop tests from `rust/`:

```sh
cargo test -p nc-data -p nc-engine -p nc-game -p nc-sysop
```

Use `cargo test --workspace` for the full suite. It also builds the graphical
client and needs its platform libraries. The [Nix development shell](flake.nix)
includes the Linux display, font, and graphics dependencies.

| Program | Purpose |
| --- | --- |
| `nc-game` | Play a local campaign. |
| `nc-door` | Run the player terminal behind a BBS or browser host. |
| `nc-sysop` | Create and administer local or BBS games. |
| `nc-admin` | Administer community browser games through a Python CLI. |
| `nc-host` | Provide community game operations and the in-development Nostr host. |
| `nc-bridge` | Serve browser login, game access, and terminal sessions. |
| `nc-discord` | Provide Discord game discovery and player commands. |
| `nc-matrix` | Send one-time Matrix sign-in links and codes for the browser host. |
| `nc-cli` | Inspect game data, run scenarios, and import or export classic files. |
| `nc-helm` | Nostr player client in development; not yet joinable. |

[Developer commands](docs/dev/developer-commands.md) cover data inspection,
scripted turns, fuzzing, performance checks, balance models, and Nostr client
experiments. Keep those experiments separate from live campaigns.

Before contributing, read [the current handoff](docs/dev/next-session.md),
[project approach](docs/dev/approach.md), [architecture](docs/dev/rust-architecture.md),
and [work tracking](docs/dev/work-tracking.md). Agents must also follow
[AGENTS.md](AGENTS.md), including the build-slot rules for shared development
machines.

The main directories are `rust/` for code, `docs/` for manuals and engineering
notes, `tools/` for analysis and test tooling, and `scripts/` for build and
release helpers. `original/` preserves the historical game materials.

To rebuild the manuals, install Typst and run this from the repository root:

```sh
python3 scripts/refresh_manual_revision_date.py --doc all
```

## Roadmap

Current work focuses on playtesting, clearer menus and reports, and reliable
community hosting. Decentralized play through Nostr is planned, but is **not
available for players to join yet**. The [Nostr design notes](docs/nostr/README.md)
are developer references, not instructions for joining a live game.

## License

Source code and tooling use the **O'Saasy License Agreement**; see
[LICENSE](LICENSE). Original material in the NC manuals uses **CC BY-NC-SA 4.0**,
with historical material excluded as stated in each manual.

This is an independent recreation, not an official Esterian Conquest release.
Preserved historical materials remain credited to their original authors and
are not included in Nostrian package archives.
