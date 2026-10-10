# Turn Submission KDL

`nc-game submit-turn` accepts one player turn file in KDL and applies it to the
campaign's Rust runtime state. It is an advanced file-based submission
interface for localhost, shared-host, and custom-client workflows when the TUI
is not the best fit.

Command shape:

```bash
cd rust
cargo run -q -p nc-game -- submit-turn --check --dir /tmp/nc-game --player 1 --file /tmp/player1-turn.kdl
cargo run -q -p nc-game -- submit-turn --dir /tmp/nc-game --player 1 --file /tmp/player1-turn.kdl
```

Important behavior:

- `--check` validates the file without mutating the campaign
- apply mode updates `ncgame.db`
- `nc-game` and `nc-sysop` continue from the updated runtime state directly
- if any command in the file is invalid, the entire submission is rejected and nothing is written
- this is a direct file-based submission path, not a queued upload protocol

The CLI `--player` value must match the `turn player=...` header in the file.

Recommended workflow:

1. Draft `turn.kdl` in your editor or another client.
2. Run `nc-game submit-turn --check ...` until validation is clean.
3. Re-run without `--check` to apply the orders to the live campaign.

## First Schema

Required top-level header:

```kdl
turn player=1 year=3000
```

Supported top-level nodes:

- `tax`
- `diplomacy`
- `planet`
- `fleet`
- `message`

## Example

```kdl
turn player=1 year=3000

tax rate=37

diplomacy to=2 relation="enemy"

planet record=16 {
  rename name="New Aurora"
  clear_build_queue
  build points=4 kind="scout"
  build points=8 kind="army"
  commission slot=1
}

fleet record=1 {
  roe value=4
  order speed=3 kind="scout_system" x=16 y=13
}

fleet record=2 {
  transfer to=1 destroyers=1
}

message to=2 subject="Border" body="Watching the north lane."
```

## Node Reference

### `tax`

```kdl
tax rate=37
```

- `rate` must be `0..=100`

### `diplomacy`

```kdl
diplomacy to=2 relation="enemy"
```

- `relation` currently supports:
  - `"neutral"`
  - `"enemy"`

### `planet`

```kdl
planet record=16 {
  rename name="New Aurora"
  clear_build_queue
  build points=4 kind="scout"
  commission slot=1
}
```

Supported child actions:

- `rename name="..."`
- `clear_build_queue`
- `build points=<u8> kind="<unit>"`
- `commission slot=<1-based stardock slot>`

Build `kind` values:

- `destroyer`
- `cruiser`
- `battleship`
- `scout`
- `transport`
- `etac`
- `ground_battery`
- `army`
- `starbase`

### `fleet`

```kdl
fleet record=1 {
  roe value=4
  order speed=3 kind="scout_system" x=16 y=13
  join host=2
  detach destroyers=1 new_roe=5
  transfer to=2 destroyers=1
  load_armies planet=16 qty=4
  unload_armies planet=16 qty=2
}
```

Supported child actions:

- `roe value=<0..10>`
- `order speed=<u8> kind="..." x=<u8> y=<u8> [aux0=<u8>] [aux1=<u8>]`
  (`seek_home` and `salvage` may leave out `x` and `y`; the fleet then heads
  for the owned world nearest it, as ECGAME picks it)
- `join host=<fleet record>`
- `detach ... [donor_speed=<u8>] [new_roe=<u8>]`
- `transfer to=<fleet record> ...`
- `load_armies planet=<planet record> qty=<u16>`
- `unload_armies planet=<planet record> qty=<u16>`

Detach/transfer ship-count fields:

- `battleships`
- `cruisers`
- `destroyers`
- `full_transports`
- `empty_transports`
- `scouts`
- `etacs`

Order `kind` values:

- `hold`
- `move`
- `seek_home`
- `patrol`
- `guard_starbase`
- `guard_blockade`
- `bombard`
- `invade`
- `blitz`
- `view`
- `scout_sector`
- `scout_system`
- `colonize`
- `join_fleet`
- `rendezvous`
- `salvage`

### `message`

```kdl
message to=2 subject="Border" body="Watching the north lane."
```

- `to` must target another empire in the current campaign
- `subject` is optional
- `body` is required
- current limits match the Rust client compose screens:
  - subject: `60` characters
  - body: `1000` characters
