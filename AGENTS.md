# AGENTS.md

This file is the agent-facing operating guide for this repository.

1. Think Before Coding

Don't assume. Don't hide confusion. Surface tradeoffs.

Before implementing:

State your assumptions explicitly. If uncertain, ask.
If multiple interpretations exist, present them - don't pick silently.
If a simpler approach exists, say so. Push back when warranted.
If something is unclear, stop. Name what's confusing. Ask.

2. Simplicity First

Minimum code that solves the problem. Nothing speculative.

No features beyond what was asked.
No abstractions for single-use code.
No "flexibility" or "configurability" that wasn't requested.
No error handling for impossible scenarios.
If you write 200 lines and it could be 50, rewrite it.
Ask yourself: "Would a senior engineer say this is overcomplicated?" If yes, simplify.

3. Surgical Changes

Touch only what you must. Clean up only your own mess.

When editing existing code:

Don't "improve" adjacent code, comments, or formatting.
Don't refactor things that aren't broken.
Match existing style, even if you'd do it differently.
If you notice unrelated dead code, mention it - don't delete it.
When your changes create orphans:

Remove imports/variables/functions that YOUR changes made unused.
Don't remove pre-existing dead code unless asked.
The test: Every changed line should trace directly to the user's request.

4. Goal-Driven Execution

Define success criteria. Loop until verified.

Transform tasks into verifiable goals:

"Add validation" → "Write tests for invalid inputs, then make them pass"
"Fix the bug" → "Write a test that reproduces it, then make it pass"
"Refactor X" → "Ensure tests pass before and after"
For multi-step tasks, state a brief plan:

1. [Step] → verify: [check]
2. [Step] → verify: [check]
3. [Step] → verify: [check]

Strong success criteria let you loop independently. Weak criteria ("make it work") require constant clarification.

## Project Goal

Generate 100% compliant gamestate files from Rust, validate them against the
original binaries (especially `ECMAINT.EXE`), and use that compliant generator
as the bridge toward a full Rust reimplementation. The DOS binaries and
preserved fixtures are the acceptance oracle.

## Architecture Rules

- Data-oriented design: explicit record layouts, focused free functions, no deep object hierarchies
- Keep logic DRY; reuse pure validators rather than duplicating checks
- Keep binary layout explicit
- Keep files focused — if a file is getting crowded, split or reorganize before extending it further

Crate conventions:

- `nc-data`: organized around explicit file/record layouts
- `nc-cli`: organized around command-family submodules
- Shared parsing, path, and reporting helpers live in support modules

Pre-`v1.0` storage policy:

- Favor a clean forward schema over dead upgrade paths
- Do not preserve SQLite migration baggage for obsolete intermediate dev databases
- After `v1.0`, add migration support only for released, user-facing save formats

See [docs/rust-architecture.md](docs/dev/rust-architecture.md) for the fuller rationale.

## Session Bootstrap

Read these at the start of a session:

1. [docs/next-session.md](docs/dev/next-session.md) — current handoff and restart point
2. [docs/approach.md](docs/dev/approach.md) — project goal, milestone ladder, acceptance criteria
3. [docs/rust-architecture.md](docs/dev/rust-architecture.md) — DOD/DRY/module rules
4. [README.md](README.md) — current user-facing commands/workflows
5. [todo.md](todo.md) — open tasks, lanes, and order; run `zk queue` for the critical path

Keep `docs/next-session.md` compact. If it accumulates history, archive detail
into `docs/dev/archive/next-session-archive.md`.

## Work Tracking

Track work with todo.txt and zk as defined in
[docs/dev/work-tracking.md](docs/dev/work-tracking.md):

- `todo.md` owns open tasks; monthly `done-YYYY-MM.md` files own completions
- Task detail and evidence live in linked notes under `docs/notes/`
- Use `zk tasks` to add or complete tasks, and cite task IDs in commits
- Never reuse a task ID; run `zk index` and `zk list docs/notes --broken-links` before committing tracking changes

## Doc Authority

Before making gameplay or rules assumptions, check sources in this order:

1. **Rust manuals** (authoritative):
   [nc_player_manual.typ](docs/manuals/nc_player_manual.typ),
   [nc_sysop_manual.typ](docs/manuals/nc_sysop_manual.typ)
2. **Original game docs** (ambiguity fallback):
   [ECPLAYER.DOC](original/v1.5/ECPLAYER.DOC),
   [ECREADME.DOC](original/v1.5/ECREADME.DOC),
   [ECQSTART.DOC](original/v1.5/ECQSTART.DOC),
   [WHATSNEW.DOC](original/v1.5/WHATSNEW.DOC)
3. **RE notebook** (archival, read only relevant sections):
   [RE_NOTES.md](docs/dev/archive/RE_NOTES.md)

When the original docs resolve an ambiguity, reconcile the finding back into
the Rust manuals rather than leaving the originals as the only source.

## Testing Rules

- Keep tests in crate `tests/` directories, not inline `#[cfg(test)]` modules, unless there is a strong local reason
- Prefer regression tests that lock in preserved fixture behavior
- Split crowded test suites into focused test modules before they become unwieldy
- Run `cargo test` from `rust/` after meaningful Rust changes
- Do not claim compliance unless the relevant tests are green

## Rust Workflow

When extending Rust support:

1. Add or refine typed accessors in `nc-data`
2. Expose the behavior through focused `nc-cli` commands
3. Add regression tests using preserved fixtures or known-valid generated directories
4. Update docs for the new capability

Prefer rule-shaped generators and validators over preserved-byte blob emission.

## Shell Notes

- The user runs **fish shell** — never use bash `\` continuation or bash-specific syntax
- For `git add`, `gh release upload`, and similar commands with multiple file arguments, put all args on a single line — fish treats each newline as a new command

## Commit / Doc Workflow

- Keep commits scoped to a real milestone or coherent gain
- Update user-facing docs when commands, workflows, or project milestones change
- Use relative Markdown links for repo files; do not commit machine-local absolute paths
- Run ordering-dependent git operations serially (`git commit` before `git push`)
- Mention new commands in `README.md` when they become part of the normal workflow

## Temp Files

- Use `/tmp` for scratch directories, debug gamestates, temporary captures, and other disposable work
- Do not create repo-local temp directories (`tmp_*`, etc.) unless explicitly asked
- If a repo-local scratch directory is created accidentally, remove it before finishing the task

## Current Priorities

1. Advance the Rust-first engine and player/TUI experience on the stable runtime/model architecture
2. Preserve classic compatibility at the import/export/oracle boundary through focused regression tests
3. Replace scenario-specific constants with explicit validators/builders
4. Use typed KDL for setup, turn submission, and harness/scenario authoring — keep Rust types authoritative, avoid freeform arbitrary-mutation DSLs

## Avoid

- Copy-pasted scenario logic across commands
- Duplicated logic that should be shared through focused helpers
- Treating preserved fixture recreation as the end state; it is the first milestone
