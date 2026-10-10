# Next Session

Keep this file short. Historical detail belongs in
[archive/next-session-archive.md](archive/next-session-archive.md), not here.

## Completed This Session

**Engine and door bug sweep (2026-10-09).** A six-slice review found 41
confirmed bugs in combat, maintenance, economy, reports, and the door; all are
fixed with failing-first regression tests. Details, decisions, and validation
are in the
[milestone record](../notes/milestones/ym5u2c1x-engine-and-door-bug-sweep-of-october-2026.md).

**Work tracking adopted.** Open work now lives in [todo.md](../../todo.md),
tracked with todo.txt and zk per [work-tracking.md](work-tracking.md). Start
with `zk queue`.

**v1.0.0-beta.3 (2026-10-10).** The Linux nc-sysop archive is published. The
Windows archives (t024) and the Windows console check (t019) are next, on
Windows after `git pull`; the Windows download links 404 until then. The new
`nc-fuzz` crate runs Hegel and proptest drivers over engine invariants.

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
- The BBS door renderer still repaints full frames instead of using the
  retained-frame diffing already in local `nc-game`.

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
