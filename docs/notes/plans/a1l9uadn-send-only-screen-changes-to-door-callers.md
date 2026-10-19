---
id: a1l9uadn
date: 2026-10-19
kind: plan
tags: [plan, door]
---
# Send only screen changes to door callers

## Scope and exit

The door renderer (`serialize_playfield_frame` in
`rust/nc-game/src/terminal/door.rs`) resends the whole 80x25 screen on every
render: a cursor move and an erase per row, and a full `ESC[0;fg;bgm` at each
colour change. It leaves the cursor visible while it draws, so callers see it
sweep down the rows. The local renderer (`rust/nc-ui/src/paint.rs`,
`diff_repaint`), which the xterm.js community stack uses, already sends only
changed spans and hides the cursor first.

Measured on the fixture game (2026-10-19), door output per render:

| Screen | Bytes | Visible characters | At 56k |
| --- | ---: | ---: | ---: |
| Main, planet and fleet menus | 1,400-1,570 | 360-440 | 0.25-0.28 s |
| Help popup | 2,725 | 1,622 | 0.49 s |
| Partial map | 8,203 | 1,323 | 1.46 s |

Every keypress, a digit at a prompt included, costs a full frame. Tentacle
BBS emulates a 56k modem by default, which made this plain (2026-10-19); on
fast links it hides, on slow ones it shows.

**Exit:** after the first frame, a door render sends only the changed spans
of changed rows; the cursor is hidden while drawing; a colour code sends only
the parts that change. A replay test proves that applying the diffed output
to a virtual screen gives the same screen as a full redraw, across real
screen sequences. A keypress that changes one prompt field costs under 100
bytes. All nc-game tests pass.

## Task details

### t208

Send only screen changes to door callers.

- Keep the last frame sent in `DoorTerminal`. Render a full frame on the
  first render and whenever the screen may be out of step: after
  `dump_text_capture`, `clear_and_restore`, a session warning, or a geometry
  change.
- Otherwise emit, row by row, the changed spans, each after one cursor move,
  as `diff_repaint` does. Hide the cursor first (`ESC[?25l`) and place and
  show it last.
- Track the terminal's current style while writing and send only what
  changes: a reset only when bold turns off, otherwise just the new
  foreground or background code.
- Test: a small ANSI interpreter in the test crate applies the bytes to an
  80x25 grid of characters and styles; for sequences of real screens
  (menus, help, map, prompt typing), the diffed stream and the full redraw
  give equal grids. Record byte counts before and after.

## Result (2026-10-19)

`DoorTerminal` keeps the screen it last sent (`SentScreen` in
`rust/nc-game/src/terminal/door_frame.rs`). The first render, and the first
after a text dump, a clear or the session warning, is the old full frame;
every other render sends `diff_frame`: changed spans of changed rows, gaps of
up to four unchanged cells folded in, `ESC[K` for a blank tail, the cursor
hidden while drawing, and colour codes for only the parts that change (a
reset only when bold turns off). A render that changes nothing sends nothing.

`tests/update/door_frames.rs` replays a door session through 20 renders
(menus, help, the planet database with scrolling, the partial map, a typed
coordinate prompt) on a small ANSI interpreter, and checks after each that
the diffed stream leaves the same screen and cursor as a full redraw. Bytes
sent per render, against the full frame before:

| Render | Before | After |
| --- | ---: | ---: |
| First frame (main menu) | 1,447 | 1,517 (unchanged path; this fixture shows a notice) |
| A key typed at a prompt | ~1,450 | 32-39 |
| Planet database, one row down | ~4,400 | 83-88 |
| Main menu to general menu | 1,541 | 926 |
| Main menu to partial map | 8,203 | 4,811 |

All 445 nc-game update tests pass.

## Connections

- [Keep command prompts short and input focused](om9r4ejo-keep-command-prompts-short-and-input-focused.md)
