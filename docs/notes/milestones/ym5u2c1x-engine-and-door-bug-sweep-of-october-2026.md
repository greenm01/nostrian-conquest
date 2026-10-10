---
id: ym5u2c1x
date: 2026-10-09
kind: milestone
status: recorded
tags: [milestone]
---
# Engine and door bug sweep of October 2026

## Result or change

Completion. Six parallel reviews of the maintenance engine and the BBS door
stack found 41 confirmed bugs. All 41 are fixed, each with a regression test
that failed before its fix. The work closed tasks t001 through t005 in
[done-2026-10.md](../../../done-2026-10.md).

- **Combat (t001):** the 1992 engagement rule; planetary hits pay defense
  strength; the 64-round cap ends in a stalemate instead of aborting
  maintenance; battle order is deterministic; only forces that traded fire
  win, retreat, or get reports; withdrawals resolve before attacks; each
  assault fleet runs its own mission class.
- **Fleet identity and map (t002):** one fleet-index remap at every fleet
  removal point; 1-based coordinates in pathfinding and map generation; fixed
  homeworld spacing; one shared player-count validator, now used by nc-host;
  scout merges that would overflow are refused; the arrival estimate shares
  the stepper's fraction rounding.
- **Economy and campaign (t003):** builds capped at the smaller of treasury
  and capacity, in maintenance and when queued; whole-unit orders; homeworld
  production can fall below potential once named; autopilot budgeting,
  starbase commissioning, and queue handling; rogue empires run the economy and
  can be eliminated; correct elimination causes.
- **Reports (t004):** report text is printable ASCII within 72-byte records;
  non-ASCII planet names are rejected; broadcasts reach every active empire;
  structured report columns stay aligned; bombardment loss lines match the
  bombarding fleet.
- **Door (t005):** telnet commands are filtered on socket doors; CR LF and CR
  NUL count as one Enter; session and idle time limits are enforced; dropfiles
  decode as CP437 when not UTF-8; the CP437 output table covers the full
  glyph set.

## Evidence and decisions

Accepted decisions:

- [Engagement follows the 1992 player guide](../decisions/mnb1phgg-engagement-follows-the-1992-player-guide.md)
- [Planetary hits pay defense strength](../decisions/lzvpk22s-planetary-hits-pay-defense-strength.md)
- [Homeworlds keep a fixed six-sector spacing](../decisions/0dt7sw73-homeworlds-keep-a-fixed-six-sector-spacing.md)

Three questions were settled by existing sources rather than new decisions:

- Unarmed contact does not abort missions. The manual's sensor check aborts
  only against an "overwhelming" enemy, and Scout Sector "remains on station."
- The zero-padded Stardate week is intended. It was added on purpose for the
  terminal client in March 2026; ec-timing-spec.md now says so.
- Growth is applied before revenue is credited. The code change "Fix revenue
  timing" postdates the manual text; both manuals now match the code.

One merge-time correction: the shared movement-fraction decode was clamped to
`0..=8`, which changed the classic FLEETS.DAT fraction byte written for speeds
that are not multiples of 3. Commit `65cb982d` widened the clamp to `0..=9`
and pinned the byte in a regression test.

Merges into main: `b032dedf` reports, `2cc68412` economy, `c3e16698` combat,
`eee1823f` door, `6703b774` fleet identity and map. The fully merged workspace
passed 2623 tests across 201 suites with no failures or compiler warnings.
The original binaries were not run against the new output; no oracle evidence
was collected for these changes.

## Remaining work

Open follow-ups live in [todo.md](../../../todo.md), with detail in
[Follow-ups from the October 2026 bug sweep](../plans/c39y8ydp-follow-ups-from-the-october-2026-bug-sweep.md).
