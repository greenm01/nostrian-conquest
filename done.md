# Completed work

Completed task lines live in monthly todo.txt-format files named
`done-YYYY-MM.md` beside `todo.md`. The CLI writes directly to the current
month; this page stays a short guide. Keeping the task files together
preserves their relative Markdown links when the todo.txt CLI moves a line.
Each line keeps its stable task ID, completion date, and a link to the note
that holds its result and evidence.

Use `zk completed` to list the files, or `zk list --link-to NOTE_PATH` to find
completion records linked to a note. `zk tasks` refreshes the index after each
successful command. Direct edits need `zk index` afterward.

[Tracking contract](docs/dev/work-tracking.md) · [Notebook guide](docs/notes/README.md)
