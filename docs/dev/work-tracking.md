# Track Work With todo.txt and zk

**Role:** repository work-tracking contract. The Rust manuals and specs keep
their authority over game behavior.

`todo.md` uses the [todo.txt format](https://github.com/todotxt/todo.txt): one
task per line. Its filename stays familiar, but it holds no Markdown headings,
multiline tasks, checklists, or diary. Monthly `done-YYYY-MM.md` files beside
`todo.md` use the same format for completed tasks. `done.md` is a short guide.
All of them are indexed by `zk`.

Plans hold task detail, dependencies, and exits. Investigations hold diagnoses
and evidence. Concepts explain reusable ideas. ADRs record lasting rule and
architecture decisions. Milestone records explain a completion, deferral, or
changed exit. The [notebook guide](../notes/README.md) defines each record.

## One owner for each fact

| Fact | Owner |
| --- | --- |
| Open task, lane, priority, stable ID, order | `todo.md` |
| Dated completion and retained task link | `done-YYYY-MM.md` |
| Task detail, dependencies, measurable exit | Linked plan or investigation |
| Evidence, failed hypotheses, validation limits | Investigation or milestone note |
| Rule or architecture choice and its consequences | ADR, with current behavior in the manuals and specs |
| Session restart point | [next-session.md](next-session.md), which links here instead of copying tasks |

Do not copy task checkboxes or an active queue into notes. A note can explain
what was found or built without claiming its task is complete.

## Task format and order

```text
(A) 2026-10-09 Put blitz landing fire on the 10x hit-cost scale. +critical +combat @development id:t007 order:020 [details](docs/notes/plans/c39y8ydp-follow-ups-from-the-october-2026-bug-sweep.md#t007)
```

- `(A)` is the critical path and `(B)` is admitted parallel work. Candidate
  and deferred tasks have no priority until promoted.
- Exactly one lane project is required: `+critical`, `+parallel`,
  `+candidate`, or `+deferred`. Other projects name the area, such as
  `+combat`, `+engine`, `+economy`, `+reports`, `+door`, `+compat`, or `+fuzz`.
- Contexts describe what the work needs: `@development`, `@planning`,
  `@oracle` for runs of the original binaries under DOSBox, and `@playtest`
  for live BBS or localhost play.
- `id:` is a stable, repository-unique identity. Never reuse one: a new task
  takes the next number above every `id:` ever used in `todo.md` and the
  `done-*.md` files together. Notes and commit messages cite tasks by ID.
- `order:` is the reviewed ordering key. Take the lowest open `+critical`
  order unless the user names other work. Keep file order consistent with it.
- The Markdown link points at the task's detail. In a plan, give each task a
  heading that is just its ID, such as `### t007`, so `#t007` resolves.
- `depends:tNNN,tMMM` names prerequisites.

Candidate work needs a decision before implementation. Deferred work is out of
the current scope. User-selected work takes precedence over the default order.

## Commands

Run these from the repository root or a subdirectory:

```sh
zk queue
zk tasks ls +parallel
zk tasks ls id:t007
zk tasks add "..."
zk investigate --title "Describe the question" --print-path --no-input
zk plan --title "The scope and exit"
zk adr --title "Describe the decision"
zk milestone --title "The result or change"
zk completed
```

`zk tasks` runs the unmodified [todo.txt CLI](https://github.com/todotxt/todo.txt-cli)
with `.todo/config`, then reindexes. The configuration derives its paths from
its own checkout, so a worktree's commands never edit another checkout's queue.
No global todo configuration changes. The development host has the CLI at
`~/src/todo.txt-cli/todo.sh`; set `NC_TODO_CLI` to use another copy. The
aliases run under `/bin/sh`, so they work from fish.

For a new task, search the notes first. Reuse the relevant plan or create an
investigation, write its scope and exit, then add one short task line with a
new ID, its lane, an order key, and the link. The CLI prepends the creation
date.

## Finish and keep evidence

Before completing a task, update its note with the result, the validation that
ran, and what remains. Update the manuals or specs when a rule changes.

Find the task by ID, check its current line number, then complete it with the
CLI: after `zk tasks ls id:t007`, run `zk tasks do N` with the number shown.
The CLI adds today's completion date and moves the line to the current month's
`done-YYYY-MM.md`, keeping its ID and link. Re-read the row before any
number-based change when another agent may be editing the queue. When a script
reads a line number, use `zk tasks -p`: colored output embeds digits in its
escape codes. Do not mark a
task done just because its note exists. Direct edits are fine; run `zk index`
afterward.

## Checks before committing tracking changes

Run `zk index`, `python3 scripts/check_note_links.py`, and `git diff --check`.
The script lists links in the notes, `todo.md` and `done-*.md` whose target
file is missing, and exits non-zero if any are. Do not use
`zk list --broken-links` for this: zk also flags working links to code, data
and manuals, because it resolves links only to its own notes.
Task IDs must be unique across `todo.md` and every `done-*.md` file, and each
open task needs one lane, one order key, and a link that resolves.
