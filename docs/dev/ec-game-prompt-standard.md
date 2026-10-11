# `nc-game` Prompt Standard

This document defines the standard command-line prompt grammar for the Rust
player TUI.

It exists to stop screen-by-screen prompt drift. Shared prompt rows, inline
entry flows, and command-line confirmations should follow this spec unless a
screen has a narrow, explicit exception.

Read it together with:

- [bbs_door_client_rust.md](bbs_door_client_rust.md)
- [tui_style_guide.md](tui_style_guide.md)
- [nc-game-table-standard.md](nc-game-table-standard.md)

## Core Rules

- The player TUI targets a fixed `80x25` playfield.
- Live command-line prompts leave a space after `-> ` before the cursor.
- Any visible live input cursor blinks while the client is waiting for input,
  including one-key prompts.
- Command rails use the universal frame `LABEL <- ... ->`.
- For non-table, non-main-menu command-line prompts:
  - angle brackets `<...>` mean available command key(s)
  - square brackets `[...]` mean the default value only
- The only allowed exception is classic yes/no shorthand:
  - `Y/[N]`
  - `[Y]/N`
- Primary command-center menu rails keep their menu identity:
  - `MAIN COMMAND`
  - `GENERAL COMMAND`
  - `FLEET COMMAND`
  - `STARBASE COMMAND`
  - `PLANET COMMAND`
  - `BUILD COMMAND`
  - `FIRST TIME COMMAND`
- Outside those primary command-center menu rails, the default live prompt label
  is `COMMAND`.
- Sort and filter chooser rails are the explicit exception:
  - `SORT <- ? ... <Q> ->`
  - `FILTER <- ? ... <Q> ->`
  - they do not use a `COMMAND` / `COMMANDS` prefix
  - they stay terse, and `?` helper popups explain the choices
- Explicit startup/naming prompts may keep their classic labels such as:
  - `EMPIRE NAME`
  - `HOMEWORLD`
  - `WORLD NAME`
- On help-capable screens, `?` is the visible popup-help token.
- On classic primary menus, `H` is an implicit alias for the same popup help
  even when it is not repeated in the visible rail.
- Command rails never list `ENTER`.
- Command rails also do not list generic navigation keys such as `J/K`,
  arrows, `^U/^D`, `PgUp/PgDn`, or map movement keys. Those belong in `?`
  helper popups, not in the rail.
- Helper popups should stay command-focused too: list the visible screen
  commands plus only unobvious non-rail actions such as typed jump, `Enter`
  review/open actions, `Tab` focus switching, or editor send/cancel shortcuts.
- Helper popups should not become keyboard-reference sheets for generic
  movement, paging, or erase keys.
- Helper popups should list one command or action per row.
- Helper popups should not combine multiple commands on one row such as
  `Q/Esc`, `I/Enter`, or `M/R/A`.
- Helper popup bodies should stay flat: no internal category headings or
  grouped sections inside the popup body.
- Helper popup rows should use aligned `Key : Description` formatting.

## Standard Grammar

### Freeform input with a default

```text
LABEL <- Prompt text [default] <Q> ->
```

Examples:

```text
COMMAND <- Review Fleet # [2] <Q> ->
COMMAND <- Empire tax rate (0 - 100) [65] <Q> ->
COMMAND <- Planet coords [16,13] <Q> ->
```

### Freeform input with command choices and a default

```text
LABEL <- Prompt text <R>, <I>, <S> [R] <Q> ->
```

Examples:

```text
COMMAND <- Change <R>OE, <I>D, or <S>peed [R] <Q> ->
COMMAND <- Change <R>OE, <I>D, or <S>peed [R] <Q> ->
```

### Freeform input with command choices and no default

```text
LABEL <- Prompt text <available choices> <Q> ->
```

Examples:

```text
COMMAND <- Class <BB,DD> <Q> ->
COMMAND <- Class <BB,DD> <Q> ->
```

The repeated fleet examples are intentional: detach and transfer should follow
the same staged class-entry grammar.

### Yes / no confirmations

Keep the classic EC shorthand:

```text
LABEL <- Y/[N] ->
LABEL <- [Y]/N ->
```

Examples:

```text
SEND MESSAGE <- Y/[N] ->
GENERAL COMMAND <- Y/[N] ->
HOMEWORLD <- "Aurora Prime" <- Is this correct? Y/[N] ->
WORLD NAME <- "New Terra" <- Is this correct? [Y]/N ->
```

Do not rewrite these to `<Y>, <N> [N]`.

## Theming Rules

- Prompt row background uses the semantic `prompt` style.
- Prompt label uses the semantic `title` style.
- Angle punctuation `<` and `>` use `prompt_angle_delimiter`.
- Square punctuation `[` and `]` use `prompt_square_delimiter`.
- Inner hotkey/default text inside those delimiters uses `prompt_hotkey`.
- Live typed input also uses `prompt_hotkey`.
- The special phrase `slap a key` keeps its current accent behavior through
  `prompt_notice_action` for the prose and `prompt_hotkey` for `key`.

Implementation consequence:

- any shared renderer that accepts prompt text must parse prompt markup instead
  of writing the prompt body as plain prompt-colored text
- custom prompt renderers must match the same style split if they do not use
  the shared helper

## Normalization Rules

- Use `<Q>` consistently in command-line prompts when quit/cancel is available.
- In command rails, `Q` is always rendered as `<Q>`.
- In command rails, other tokens stay bare and use space separation.
- In command rails, `[default]` stays rightmost and `<Q>` sits immediately to
  its left when both are present.
- In command rails, `ENTER` remains implicit and should not be repeated in the
  rail or nearby subtitle text.
- When a prompt is rendered through `draw_command_line_default_input_at(...)`,
  do not include `Q` in the prompt-body command list; that helper already
  appends the canonical trailing `<Q> ->`.
- Do not use square brackets for normal command choices such as:
  - `[R]OE`
  - `[D]elete`
  - `[A]ll`
  unless the prompt is a yes/no confirmation exception.
- Keep prompt wording short and input-oriented.
- Do not expand simple prompts into wordy forms like `<Y>es, <N>o` unless a
  specific screen genuinely needs the extra text for clarity.
- If a prompt already has a command-center label, do not repeat that title in
  the prompt body.

## Representative Non-Table Prompts

Fleet:

```text
COMMAND <- Review Fleet # [2] <Q> ->
COMMAND <- Order Fleet # [2] <Q> ->
COMMAND <- Change <R>OE, <I>D, or <S>peed [R] <Q> ->
COMMAND <- New ROE [0] <Q> ->
COMMAND <- New Fleet ID [4] <Q> ->
COMMAND <- New Speed [5] <Q> ->
COMMAND <- ETA Fleet # [2] <Q> ->
COMMAND <- Merge Fleet # [4] <Q> ->
COMMAND <- Into Fleet # [2] <Q> ->
COMMAND <- Transfer From Fleet # [4] <Q> ->
COMMAND <- Transfer To Fleet # [2] <Q> ->
COMMAND <- Load Fleet # [3] <Q> ->
COMMAND <- How many armies to load? [2] <Q> ->
COMMAND <- Unload Fleet # [2] <Q> ->
COMMAND <- How many armies to unload? [2] <Q> ->
COMMAND <- Class <BB,DD> <Q> ->
```

Startup / naming:

```text
EMPIRE NAME <- Name your empire (20 chars or less) <Q> ->
EMPIRE NAME <- "Aurora" <- Is this correct? [Y]/N ->
HOMEWORLD <- Name this world (20 chars or less) <Q> ->
HOMEWORLD <- "Aurora Prime" <- Is this correct? Y/[N] ->
WORLD NAME <- Name this world (20 chars or less) <Q> ->
WORLD NAME <- "New Terra" <- Is this correct? [Y]/N ->
```

Messaging and planet-side prompts:

```text
SEND MESSAGE <- Y/[N] ->
GENERAL COMMAND <- Y/[N] ->
COMMAND <- Planet coords [16,13] <Q> ->
COMMAND <- Empire tax rate (0 - 100) [65] <Q> ->
COMMAND <- Qty for Destroyers [00] <Q> ->
COMMAND <- How many new destroyers to build [1] <Q> ->
COMMAND <- <ENTER> commissions the drafted fleet. <Q> ->
COMMAND <- <ENTER> commissions the highlighted starbase. <Q> ->
COMMAND <- Delete <A> or 1-2 <Q> ->
```

## Command Rail Examples

```text
MAIN COMMAND <- ? X V C G P F T I B D <Q> ->
GENERAL COMMAND <- ? X V I A S P M C R D O E <Q> ->
MAP COMMAND <- ? <Q> ->
COMMANDS <- ? <Q> [03,03] ->
```

This document also covers terse table submenu rails such as
`SORT <- ? C L M <Q> ->` and `FILTER <- ? A R E M <Q> ->`.

The build queue starts deletion only with `D`. Enter has no action on the
queue list; it still submits a typed deletion quantity. Quantity and
confirmation prompts do not resize the queue table.

Ship-class staging prompts show only classes available from the current donor,
after subtracting staged ships and applying the action's limits. Keep the
composition's class order. Commission/transfer and cancel stay on their
separate action line. With no further ships available, show
`COMMAND <- No more ships to stage <Q> ->` instead of an empty class list.
