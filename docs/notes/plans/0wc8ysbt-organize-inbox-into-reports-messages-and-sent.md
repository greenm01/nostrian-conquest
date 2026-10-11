---
id: 0wc8ysbt
date: 2026-10-11
kind: plan
tags: [plan, milestone]
---
# Organize inbox into Reports Messages and Sent

## Scope and exit

The owner approved three inbox folders in the existing 80-column door:
Reports, Messages, and Sent. Sent includes queued and delivered messages.
There is no fourth folder and no Type column. Reports show Stardate;
Messages show Year and From; Sent shows To and either Queued or the delivery
year in its Sent column. R/M/S selects a folder; the year filter stays.

### t113

General Command's Delete ALL clears Reports only. Messages and Sent have
individual removal, with confirmation. D on queued mail cancels delivery;
D on a delivered sent copy affects only the sender's view.

All reports and messages remain in the game database. Deletion changes
visibility, never removes the stored content. Sender and recipient visibility
are independent. Cancellation is retained as state and prevents delivery in
all later years. Bulk report removal must retain report bodies and leave all
mail untouched.

Verify folder isolation, pending and delivered labels, stable IDs after
cancellation, independent sender/recipient visibility, database round trips,
and 80x24/80x25 layouts. Update the player manual. No private BBS setup details
belong in repository evidence.

## Connections

- [Short command prompts](om9r4ejo-keep-command-prompts-short-and-input-focused.md)
- [Player manual](../../manuals/nc_player_manual.typ)

## Implementation

The door uses three folder tabs with counts. Sent puts queued messages first,
then delivered copies by delivery year. Item IDs survive cancellation within
an open inbox. The recipient-list queued-message shortcut remains available.

Mail now carries independent sender visibility and cancellation flags. SQLite
stores those flags beside the existing mail rows. Older rows default to
visible and uncancelled. The queue interchange reader accepts old records;
the writer includes the new flags. Cancelling restores the per-recipient
queue allowance, as removing a queued message did before.

A regression exposed a report-retention hole: loading runtime state omitted
hidden reports. Saving that state again could replace the year's snapshot
without those reports. Runtime loads now retain hidden rows; player-facing
projections still filter them. When maintenance advances the year, hidden
reports remain in the earlier snapshot rather than returning to the inbox.

## Validation

The full Rust workspace passes: 3,233 tests, zero failures, 11 ignored.
Retention tests cover cancellation across maintenance, sender-only removal,
queue-file round trips, same-year reload/save, and archived report bodies
after a year advance. Door tests cover all three folders at 80x24 and 80x25,
queued/delivered labels, report-only bulk clearing, and stable IDs.
The player manual PDF builds and `git diff --check` passes.
The release build also passes. Existing sessions are left running; the next
door launch uses the updated interface.

The owner shortened the notices to `No reports to delete.`, `Reports cleared.`,
and `Clear all reports?`. The manual keeps the explanation of what is retained.
All 486 existing prompt-render and update tests pass after the wording change.
