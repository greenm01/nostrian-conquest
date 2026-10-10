# Development Notes and Decisions

Nostrian Conquest keeps investigations, reusable ideas, rule and architecture
decisions, milestone records, and plans as linked Markdown files. Use `zk` to
create, find, and connect them. Start from the question you are working on, and
read its evidence and related decisions before opening a new note.

Tasks live in [todo.md](../../todo.md) in todo.txt format. The
[work-tracking contract](../dev/work-tracking.md) defines `zk tasks`, the
monthly completion files, and which record owns each fact.

## Pinned

Long-running directions that are easy to lose track of. Find them with
`zk list docs/notes --tag pinned`.

- [Training an AI opponent](investigations/lenngda1-training-an-ai-opponent-for-nostrian-conquest.md)
  — published game-AI systems that fit this game, and a suggested path from
  a tuned scripted bot to a learned policy.

## Choose the right record

| Kind | Use it for | Status |
| --- | --- | --- |
| Investigation | One incident, experiment, or open question | `investigating`, `implemented`, `awaiting-oracle`, `closed` |
| Concept | One reusable insight with its evidence and limits | `draft`, `established`, `superseded` |
| ADR | A lasting rule or architecture choice and its consequences | `proposed`, `accepted`, `rejected`, `superseded` |
| Milestone | A completion, retargeting, deferral, or evidence review | `draft`, `recorded` |
| Plan | Task detail, dependencies, and measurable exits | Task state stays in the task files |

The [player](../manuals/nc_player_manual.typ) and
[sysop](../manuals/nc_sysop_manual.typ) manuals and the specs under
[docs/dev](../dev/README.md) define current behavior. ADRs explain why a choice was
made. Neither overrides a newer manual or spec; if they disagree, reconcile the
documents rather than treating an old note as permission to change behavior.

## Use zk

Run these from the repository root. `zk` is a documentation tool, not a build
or runtime dependency. Its SQLite index is local, ignored by Git, and
rebuildable with `zk index`.

```sh
zk investigate --title "Describe the question"
zk concept --title "State the reusable insight"
zk adr --title "Describe the decision"
zk milestone --title "The result or change"
zk plan --title "The scope and exit"
zk queue
zk recent
zk list docs/notes --match "engagement"
zk list --link-to docs/notes/decisions/mnb1phgg-engagement-follows-the-1992-player-guide.md
zk index
```
