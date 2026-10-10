# Harness

`nc-cli harness` now supports three related workflows:

- campaign play orchestration for multi-bot or mixed human/LLM games
- runtime scenario setup for TUI playtesting and repros
- combat scenario and sweep execution for combat stress testing and metrics

The harness is Rust-first:

- KDL expresses stable scenario intent
- Rust validates, builds runtime state, and executes turns/combat
- SQLite/runtime state is the primary output
- classic `.DAT` export is optional via `--export-classic`

Start here:

- [campaign-play.md](campaign-play.md)
  - reproducible conductor workflow for "play to turn 5, then inspect in the TUI"
- [../llm-player-guide.md](../llm-player-guide.md)
  - strict visible-state operating guide for bots acting as real players

Quick four-player human TUI bootstrap:

```bash
python3 tools/harness/four_player_human_campaign.py bootstrap --reset
```

Then inspect any house directly:

```bash
cd rust
cargo run -q -p nc-game -- --dir .tmp/campaigns/human-tui-four-player/game --player 1
```

## Commands

```bash
cd rust
cargo run -q -p nc-cli -- harness init-campaign --file /tmp/scenario.kdl --dir /tmp/ec-bot-campaign --game-id tui-polish --bundle-profile llm
cargo run -q -p nc-cli -- harness open-turn --dir /tmp/ec-bot-campaign
cargo run -q -p nc-cli -- harness claim-turn --dir /tmp/ec-bot-campaign --player 2
cargo run -q -p nc-cli -- harness scan-turn --dir /tmp/ec-bot-campaign
cargo run -q -p nc-cli -- harness apply-turn-batch --dir /tmp/ec-bot-campaign
cargo run -q -p nc-cli -- harness play-until --file /tmp/scenario.kdl --dir /tmp/ec-bot-campaign --game-id tui-polish --turn 5 --bundle-profile llm
cargo run -q -p nc-cli -- harness check-scenario --file /tmp/scenario.kdl
cargo run -q -p nc-cli -- harness run-scenario --file /tmp/scenario.kdl --dir /tmp/ec-scenario
cargo run -q -p nc-cli -- harness check-combat --file /tmp/combat-scenario.kdl
cargo run -q -p nc-cli -- harness run-combat --file /tmp/combat-scenario.kdl
cargo run -q -p nc-cli -- harness run-sweep --file /tmp/combat-sweep.kdl
cargo run -q -p nc-cli -- harness list-report-families
cargo run -q -p nc-cli -- harness preview-reports --family bombard --seed 1515 --samples 3
```

## Report Preview

Use the report preview flow when you want quick narrative inspection instead of
an end-to-end runtime scenario.

- `list-report-families`
  - prints every registered family plus whether it is implemented or still a stub
- `preview-reports`
  - generates seeded synthetic `MaintenanceEvents`
  - runs the real Rust results composer
  - prints stacked viewer sections such as attacker/defender so wording can be reviewed quickly

Example:

```bash
cd rust
cargo run -q -p nc-cli -- harness preview-reports --family bombard --seed 1515 --samples 3
```

Notes:

- `--family all` runs every implemented family and prints a final `stubbed families skipped=...` line
- families still marked `stub` are discoverable now, but the previewer will report `not implemented yet` for direct queries
- the preview flow is deterministic for the same `--seed` and `--samples`

## Campaign Play

Use the campaign-play workflow when you want a real in-process game that bots
or humans can keep playing turn by turn.

In practice this usually means:

- `nc-cli harness` acts as the authoritative in-repo conductor
- an outer operator or LLM coordinator claims player turns and spawns one player worker per bundle
- each worker writes only its own `turn-<nnnn>.kdl`

The conductor owns turn advancement:

- it opens the current turn
- publishes per-player bundles under `.tmp/llm-turns/<game_id>/player-<n>/`
- waits for one legal `turn-<nnnn>.kdl` from every active player
- applies the whole batch
- runs Rust maintenance exactly once
- opens the next turn

Per-player coordination files:

```text
.tmp/llm-turns/<game_id>/campaign/manifest.kdl
.tmp/llm-turns/<game_id>/player-<n>/bundle-turn-0005/
.tmp/llm-turns/<game_id>/player-<n>/bundle-turn-0005/.llm/spatial.kdl   # llm profile only
.tmp/llm-turns/<game_id>/player-<n>/status-turn-0005.kdl
.tmp/llm-turns/<game_id>/player-<n>/turn-0005.kdl
.tmp/llm-turns/<game_id>/player-<n>/notes-0005.md
```

Status states:

- `ready`
  - conductor opened the turn and the bot may start
- `claimed`
  - bot/operator marked the turn in progress with `harness claim-turn`
- `submitted`
  - the turn file exists and is waiting for validation
- `validated`
  - the conductor accepted the file for this year
- `rejected`
  - the file failed validation; fix it and rescan
- `applied`
  - the whole year batch was applied and maintenance completed

Fog-of-war boundary:

- player bundles include only player-visible starmap/intel projections
- they include owned assets, diplomacy, economy summaries, and incoming player mail
- they include coordinator-generated legal action hints per fleet to reduce invalid bot turns
- `--bundle-profile llm` campaigns also emit hidden `.llm/spatial.kdl` files derived only from that same player-visible state
- bundle `README.md` files are refreshed with current turn status and any
  rejection error, so rerun workers see the latest validation feedback in-place
- they currently expose review flags only, not raw global `RESULTS.DAT` or `MESSAGES.DAT` text
- bots should treat the bundle as the safe source of truth and not inspect `ncgame.db`

For the full reproducible operator flow, including "play to turn 5 then open the
TUI", and for the sub-agent coordination pattern layered on top of it, see
[campaign-play.md](campaign-play.md).

## `scenario.kdl`

Use this when you want a coherent runtime campaign snapshot for playtesting.

When using the scenario harness with bots or LLMs, keep generated turn files in
the ignored local workspace described in
[../llm-player-guide.md](../llm-player-guide.md):

```text
.tmp/llm-turns/<game_id>/player-<n>/turn-0005.kdl
```

That keeps per-game/per-player agent turns organized without bloating the repo.

```kdl
scenario player_count=4 year=3000 baseline="builder-compatible" seed=1515 label="Turn 3 Playtest"

house record=1 handle="SYSOP" empire="Aurora" homeworld="Aurora Prime" tax=30
house record=2 handle="RIVAL" empire="Helios Crown"

relation from=1 to=2 status="enemy"
relation from=2 to=1 status="enemy"

planet record=1 {
  name "Aurora Prime"
  production potential=140 present=120 stored=80
  defenses armies=14 batteries=6
  stardock slot=1 kind="destroyer" count=3
  commission slot=1
}

fleet record=1 {
  coords x=10 y=10
  ships bb=1 ca=2 dd=3 sc=0 tt=1 armies=1 etac=0
  roe value=8
  order kind="hold" speed=0 x=10 y=10
}

turn-file path="player1-turn.kdl"
queued-mail from=1 to=2 year=3000 subject="Border" body="Hold line."
results-block player=1 "Command summary\nTurn 3 ready."
messages-block player=1 "Incoming traffic\nStand by."
```

Supported top-level nodes:

- `house`
  - set handle, empire name, homeworld name, and tax for a player record
- `relation`
  - stored diplomacy between houses
- `planet`
  - override planet coords, owner, name, production, defenses, stardock slots, and optional `commission` actions; each stardock slot holds one item type, in the order destroyer, cruiser, battleship, scout, troop transport, ETAC, starbase
- `fleet`
  - override an existing or commissioned fleet record's location, composition, ROE, speed, invasion armies, and order
- `turn-file`
  - apply an existing `turn.kdl` after scenario setup
- `queued-mail`
  - seed Rust queued outbox mail
- `results-block` / `messages-block`
  - seed startup review text for the referenced player

Notes:

- `baseline` can be `"builder-compatible"` or `"joinable-new-game"`
- the harness does not support arbitrary raw field mutation
- fleet ownership reassignment is not supported in the first version; use the baseline owner-linked fleet records or commission new fleets from stardock

## `combat-scenario.kdl`

Use this when you want a combat-focused scenario executed immediately through Rust maintenance.

```kdl
combat-scenario player_count=4 year=3001 baseline="builder-compatible" seed=1515 turns=1 label="Skirmish"

relation from=1 to=2 status="enemy"
relation from=2 to=1 status="enemy"

fleet record=1 {
  coords x=10 y=10
  ships bb=1 ca=0 dd=0 sc=0 tt=0 armies=0 etac=0
  roe value=10
  order kind="hold" speed=0 x=10 y=10
}

fleet record=5 {
  coords x=10 y=10
  ships bb=1 ca=0 dd=0 sc=0 tt=0 armies=0 etac=0
  roe value=10
  order kind="hold" speed=0 x=10 y=10
}
```

This uses the same child-node vocabulary as `scenario.kdl`, but adds:

- `turns`
  - how many Rust maintenance turns to execute

## `combat-sweep.kdl`

Use this when you want many generated combat cases from one base combat scenario.

```kdl
combat-sweep scenario="combat.kdl" turns=1 seed=99 max_cases=8

fleet-ship fleet=1 kind="bb" 1 2 3
fleet-roe fleet=5 6 10
planet-stat planet=14 field="armies" 0 5 10
relation-variation from=1 to=2 "neutral" "enemy"
```

Supported sweep dimensions:

- `fleet-ship`
  - vary `bb`, `ca`, `dd`, `sc`, `tt`, `armies`, or `etac`
- `fleet-roe`
  - vary fleet ROE values
- `planet-stat`
  - vary `armies` or `batteries`
- `relation-variation`
  - vary diplomacy between `neutral` and `enemy`

Current sweep behavior:

- expansion is deterministic
- `max_cases` caps how many generated cases execute
- output focuses on event counts and elapsed time metrics
- the base scenario must already define the fleets/planets referenced by the sweep dimensions
