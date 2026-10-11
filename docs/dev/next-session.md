# Next Session

Keep this file short. Historical detail belongs in
[archive/next-session-archive.md](archive/next-session-archive.md), not here.

## Completed This Session

**Typed game state (t039, 2026-10-10).** Game state is plain typed Rust
structs (`nc-data/src/model/`); SQLite stores them in typed columns; the
runtime reads and writes only the store; the classic `.DAT` codecs live in
`nc-compat/src/classic/`, guarded by `nc-data/tests/classic_boundary.rs`.
`.DAT` files appear only through `nc-cli db-export`/`db-import` and the
oracle tools. See the [plan](../notes/plans/7h5r6ne8-typed-game-state-in-rust-and-sqlite.md).

**Classic fidelity (t031-t064).** 2.5D movement, the per-tick encounter loop,
starbase movement, the orbital gate, and the classic home pickers match
ECMAINT; deviations are owner-approved classic bugs, listed in the specs.

**NC combat redesign.** The owner approved all seven combat decisions with
Claude and Codex's joint recommendations (ADRs in `docs/notes/decisions/`).
Decision 1 (the flat 25/50/100 table with carried hits) is in the engine;
the rest are tasks t065, t066, t071, t074, t078, t079, t080.

**Door and mail fixes.** Windows console input, output, arrow keys, and
flicker (t019); `?` types text in editors, Ctrl+G opens help; queued mail is
readable and cancellable and is delivered at maintenance (t067).

**Next:** the `*_raw` identifier cleanup, then the implementation waves in
[x8arlb3c](../notes/plans/x8arlb3c-implementation-waves-after-t039.md);
`zk queue` for the order.

## Current Goal

- Stabilize the localhost and BBS release surfaces for public beta use.
- Fix real playtest bugs and workflow rough edges quickly.
- Keep docs aligned with the local/BBS product story.
- Treat future Nostr work as a separate `nc-host` / `nc-helm` track.

## Biggest Blockers

- No known major engine/storage blocker remains.
- The main remaining risk is field bugs found by real BBS and localhost players.
- The hosted `nc-host` / `nc-helm` track now exists locally, but it is still a
  dev-only path and not the public shipped product story.
- The hosted client now stages and replays build, fleet, tax, diplomacy, and
  quick message drafts. Remaining gaps are narrower unsupported actions such as
  starbase moves and fleet renumbering.
- The BBS door renderer redraws every row each frame (no flash since t019)
  instead of using the retained-frame diffing already in local `nc-game`.

## Immediate Next Steps

1. Keep running real BBS and localhost playtests and tighten the rough edges.
2. Regenerate manuals/PDFs whenever the public command surface changes.
3. Keep runtime/storage code pointed at SQLite and away from raw offset-style
   regressions.
4. Keep the runtime DB scoped to localhost/BBS play; future hosted work should
   use its own schema rather than reviving retired hosted tables in `ncgame.db`.
5. Keep the localhost `nc-host` lab reproducible with the user-service install
   script and dev docs, but keep the public docs centered on local/BBS play.
6. Keep the shared local/hosted dashboard launch path stable while real users
   exercise the hosted dashboard.
7. Finish the remaining unsupported hosted dashboard actions or explicitly
   scope them out of the hosted flow.
8. If modal drag pacing still feels rough in `nc-helm`, evaluate a future
   GPU/vsynced native renderer path as follow-up work.
9. After `nc-helm` is stable on the shared ratatui render contract, replace
   the temporary `crossterm` event types with a small local input model.
