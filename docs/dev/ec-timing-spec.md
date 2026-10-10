# ECMAINT Timing / Stardate Notes

This document captures the current stable timing-related findings for
`ECMAINT.EXE`.

It is intentionally narrower than a full maintenance-phase spec. The goal here
is to separate what is already supported by historical logs and static RE from
the low-value report-writer details that remain only partially mapped.

This includes both:

- the internal `1..52` timing model behind report emission
- the player-visible `Stardate` header contract that classic reports render

For the broader recovered phase ordering, see
[ec-turn-cycle-spec.md](ec-turn-cycle-spec.md).
For phase placement in the Rust target engine, also see
[rust-turn-cycle-implementation.md](rust-turn-cycle-implementation.md).

## Settled So Far

- player-facing `Stardate` values are not just yearly labels
- historical player logs show a `1..52` in-year component
- historical logs also show year rollover from `52/YYYY` to `1/YYYY+1`
- the `1..52` scale is not just narrative flavor text:
  - preserved fleet logs show mission outcomes landing at specific in-year
    stardates
  - one fleet can emit multiple ordered reports within the same year at
    different stardates as a mission progresses
- multiple report families use the same week/year form:
  - fleet reports
  - planet reports
  - starbase reports
  - Fleet Command Center reports

Examples from the shipped historical captures:

- `ec8.txt` reaches `Stardate: 52/3009`
- `ec9.txt` begins with `Stardate: 1/3010`
- `ec47.txt` reaches `Stardate: 52/3049`
- `ec48.txt` begins with `Stardate: 1/3052`

This is strong evidence for a real internal yearly tick scale rather than a
purely cosmetic year stamp. The leading semantic interpretation is now
"week-of-year", since `52` fits weeks much better than literal days or months.

## Report Header Contract

The classic player-visible report shape is now clear enough to treat as a
formatting requirement, not just an implementation detail.

Settled presentation rule:

- every classic player report family carries `Stardate: <week>/<year>` on the
  **first
  line** of the report
- that `Stardate` text is **right-justified** on the first line, after the
  source clause, rather than emitted as its own separate header line
- Rust should preserve that classic first-line shape for fleet, planet,
  starbase, and Fleet Command Center reports
- the week field is an integer `1..52`:
  - classic EC renders it with no zero padding (`Stardate: 1/3010`)
  - the Rust engine intentionally zero-pads it to two digits
    (`Stardate: 02/3031`) so the TUI can align and colorize it; this is a
    deliberate Rust presentation choice, not a classic-compatibility claim
- the year field is rendered as a four-digit year
- this weekly first-line rule applies to player report entries, not to the
  separate rankings banner text
- preserved rankings headers use a separate year-only form:
  `Stardate: YYYY A.D.`

Examples from the shipped historical logs (classic, unpadded week):

```text
 -> From your 1st Fleet, located in System(13,15)          Stardate: 32/3001
 -> From planet "you're my bitch" in System(23,5):          Stardate: 1/3021
```

Practical meaning:

- the surface format is `Stardate: <week>/<year>`
- the leading field behaves like a week-of-year tick, not a literal calendar
  month
- do not emit Rust report timestamps on a separate line above the report body
- do not left-align `Stardate:` directly after the source phrase with no
  spacing; preserve the classic padded first-line look
- the exact formatter routine is still not statically recovered, but the
  output contract is strong enough to implement directly

## Strongest Behavioral Evidence

The clearest preserved evidence comes from the shipped campaign text captures.

In `ec.txt` (`3001 A.D.`), the starting fleet review shows:

- `1st Fleet` colonize `(13,15)` with `Travel Time: 1 year`
- `2nd Fleet` colonize `(20,11)` with `Travel Time: 2 years`
- `3rd Fleet` view `(23,5)` with `Travel Time: 2 years`

Then `ec2.txt` later shows the resulting unread reports:

- `1st Fleet` colonization at `Stardate: 32/3001`
- `4th Fleet` starbase-guard arrival at `Stardate: 1/3002`
- `3rd Fleet` viewing mission report at `Stardate: 12/3002`
- `3rd Fleet` later move-complete report at `Stardate: 21/3002`
- `2nd Fleet` colonization at `Stardate: 25/3002`

That is strong evidence for a real intra-year scheduler:

- the same maintenance year can contain multiple ordered event times
- the same fleet can generate more than one report at different in-year ticks
- those ticks line up with mission progress, not just with a single decorative
  timestamp chosen after the fact

Practical conclusion:

- `ECMAINT` is using a real sub-year event timeline for movement/report
  sequencing
- the leading semantic interpretation remains week-of-year
- the remaining low-value follow-up is the exact helper path that formats and
  emits some of that already-recovered weekly output, not whether the weekly
  scheduler exists

## Full Log Corpus Patterns

The shipped `ec*.txt` report logs also show stable ordering rules across the
whole corpus.

Current aggregate sweep:

- `47` log files with `735` timestamped events
- every file is strictly nondecreasing by `(year, week)`
- `5` files span multiple report years, which shows unread reports can persist
  into later login sessions

The reusable aggregate report is generated by:

```bash
python3 tools/analyze_ec_report_logs.py
```

Current output:

- `artifacts/ec-report-log-analysis.txt`

High-signal sequencing patterns:

- same-week bundles are common for one source
  - especially `sensor contact -> identification -> interception`
  - these appear to be emitted as one weekly batch
- same-week ordering is stable rather than arbitrary
  - the aggregate corpus now shows `38` direct `sensor contact ->
    identification` pairs
  - it also shows repeated longer chains including
    `sensor contact -> identification -> interception`
- multi-week sequences from the same source are also common
  - examples include `extended orbit` followed by later contact/update reports
  - this strongly supports mission progress advancing across multiple in-year
    ticks
- adjacent report timing is concentrated at zero or one week of separation
  - current corpus counts:
    - `350` adjacent transitions with week-gap `0`
    - `67` adjacent transitions with week-gap `1`
  - this fits a real ordered weekly event stream much better than post-hoc
    decorative timestamps
- Fleet Command Center reports are interleaved into the same weekly order
  - they usually read like administrative loss summaries after combat or
    interception outcomes
  - they are not a separate out-of-band yearly appendix
- targeted recurring transitions also show immediate follow-on consequences in
  that same ordered stream:
  - `identified -> fleet-lost` same week: `4x`
  - `fleet-lost -> join-retarget` same week: `2x`
  - `fleet-lost -> planet-bombarded` same week: `4x`
  - `intercepted -> planet-bombarded` next week: `3x`

Practical conclusion:

- report ordering is structured, not arbitrary
- week values are participating in event batching and progression
- the corpus supports a real sub-year scheduler with same-week and cross-week
  report phases

## Concrete Report-Family Placement Constraints

The shipped corpus now closes several concrete scheduler families strongly
enough to promote into the Rust target spec.

Focused source-split extract:

- `artifacts/ec-report-transition-focus.txt`
- `artifacts/ec-report-transition-splits.txt`
- `artifacts/ec-report-cadence-focus.txt`

Direct same-source / same-year progression:

| Transition | Observed placement | Practical meaning |
| --- | --- | --- |
| `sensor-contact -> identified` | same week in all focused shipped-log cases (`48x`) | contact and identification form one ordered same-week bundle |
| `identified -> intercepted` | same week where directly chained (`3x`) | direct interception can continue in that same weekly bundle |
| `entered-system -> attacked` | both same-week and next-week cases (`1x/1x`) | arrival and hostile combat are separate weekly-stream events; do not derive attack week directly from system-entry week |
| `identified -> orbit-world` | same-source/year gaps `0/1/4`; the zero-gap cases are all week `1` in the preserved corpus | `extended orbit` is a standing mission/status family, not a fixed post-identification delay; the zero-gap cases are best explained by fleets already orbiting at round start |
| `orbit-world -> sensor-contact` | wide-gap periodic family (`1/2/3/5/8/10/12/14/16/26/28/36`) | later contact while orbiting reflects independent hostile traffic/detection while the standing orbit status persists, not a self-timed orbit countdown |
| `attacked -> bombing-run` | same-source/year gaps `0/5/6/7`; the zero-gap case is week `1` in the preserved corpus | bombardment continuation after hostile contact is a standing mission cadence, not one fixed post-attack delay; the immediate variant is a round-start continuation case |
| `intercepted -> bombing-run` | one direct same-source case at gap `6` | the bombardment continuation family is not specific to the `attacked` wording; it follows hostile encounter while a bombardment mission is already active |

Cross-source same-week interleaving in the shared weekly stream:

| Transition | Observed placement | Practical meaning |
| --- | --- | --- |
| `identified -> fleet-lost` | same-week cross-source adjacent in `4x`, with one later outlier at gap `4` | Fleet Command Center loss summaries are separate stream entries, not same-source mission progression |
| `attacked -> fleet-lost` | next-week cross-source adjacent in `2x` | attack reports do not imply immediate same-week FCC loss summaries |
| `fleet-lost -> join-retarget` | same-week cross-source adjacent in `2x` | some administrative follow-ons share the same weekly stream as the loss summary |
| `fleet-lost -> planet-bombarded` | same-week cross-source adjacent in `4x`, with delayed variants at gaps `3` and `16` | hostile-world aftermath shares the stream, but not as one fixed direct timing chain |

Practical consequence:

- the Rust-facing timing gap is now closed; there is no missing delay table
  that Rust still needs in order to implement these four families
- the direct same-source families above are now better read as state-family
  rules inside one shared weekly stream:
  - `entered-system` is not a countdown seed for `attacked`
  - `orbit-world` is a standing extended-orbit status family with round-start
    week-`1` carry
  - later `sensor-contact` while orbiting is driven by independent hostile
    presence, not by one internal orbit timer
  - bombardment continuation after hostile contact belongs to a standing
    bombardment mission cadence rather than to one universal
    `attack -> bombing-run` offset
- the Fleet Command Center and planet-loss follow-ons above are better treated
  as same-stream cross-source interleaving, not as one hidden same-source delay
  rule that Rust still needs to recover

## Current Static Anchors

Static timing-focused analysis currently has these anchors:

- `2000:6fc6`
  - string cluster containing:
    `Today is ' - maintenance is not scheduled to run.`
  - this is the clearest current anchor for the maintenance schedule/date gate
- `2000:945b`
  - currently labeled `ecmaint_emit_timestamp_message_helper`
  - current evidence says this helper belongs to the token/schedule path, not
    the player report `Stardate` path
- `3000:39dc`
  - current time-query helper candidate from earlier token-anchor work
  - still not semantically decoded
- `3000:189c`
  - ranking-generation string cluster:
    `Enabling player-ranking text file generation...`
  - useful as a rankings-output anchor, but still not tied to a decoded
    stardate formatter path

The reusable headless extraction for this work is:

```bash
tools/run_ghidra_script.sh ecmaint-live ECMaintTimingFlow.java
```

Its current output is written to:

- `artifacts/ghidra/ecmaint-live/timing-flow.txt`

The follow-up direct-reference sweep is:

```bash
tools/run_ghidra_script.sh ecmaint-live ECMaintTimingRefs.java
```

Its current output is written to:

- `artifacts/ghidra/ecmaint-live/timing-refs.txt`

## Important Correction

The first timing-focused pass changed one earlier assumption:

- `2000:945b` has only four direct call sites in the current live dump
- all four sit in the token/schedule region around `0x97xx..0x9exx`
- none of those call sites are currently tied to player report generation

Practical interpretation:

- `2000:945b` is currently best treated as a current-date/status formatter used
  during maintenance scheduling/token handling
- do not treat it as the recovered player-report `Stardate` emitter yet
- `2000:6fc6` and `3000:189c` currently show no direct xrefs in the live dump,
  so the schedule/rankings strings are probably reached through indirect
  string-table handling rather than simple immediate references
- the startup `main.tok` / `Creating main work file...` / `Merging joint
  fleets...` cluster at `2000:841b..855a` also currently has no direct scalar
  xrefs in the live dump
  - current best interpretation is the same: the outer startup driver is likely
    reaching those messages through an indirect string/pointer path rather than
    inline `MOV DI, imm16` references

## Already Closed

- `ECMAINT` uses a real internal `1..52` yearly timeline rather than a
  decorative year-only timestamp
- the leading `Stardate` field in classic player reports is week-of-year, not
  month-of-year
- classic player report entries render `Stardate: <week>/<year>` as a
  right-justified first-line fragment
- preserved rankings headers use a separate year-only `Stardate: YYYY A.D.`
  banner form
- the weekly scheduler has an explicit late `1..52` loop with decode,
  timing-window derivation, and accept/reject testing
- timing-window constants for the recovered late scheduler path are bounded
- `CONQUEST.DAT[0x03..0x09]` is the maintenance-schedule block consulted by the
  outer schedule gate
- for implementation depth, the schedule gate is now best treated as:
  - current day-of-week query
  - `CONQUEST.DAT[0x03..0x09]` raw schedule bytes
  - no additional `CONQUEST.DAT` control field is currently required by the
    recovered model
- `2000:945b` is best treated as a schedule/status timestamp helper, not the
  player-report `Stardate` formatter
- player-report generation sits in the late output tail reached from
  `2000:8652 -> 2000:1da6 -> 2000:0c06 -> 2000:56be`
- rankings generation sits in the optional late output branch
  `2000:8665 -> 2000:7659`
- no recovered dedicated core `.DAT` field stores the weekly tick:
  - the recovered scheduler computes week placement in runtime/scratch state
  - the visible week then persists in emitted text outputs rather than as a
    named durable campaign-state field
- `0000:f1ba` is a recovered scratch-local timing-entry initializer:
  - it writes only `0` or `1` into the local timing-entry code byte
  - it also seeds companion local flags at offsets `-0x09`, `-0x08`, and
    `-0x07`
- `0000:f914` is a recovered late timing-entry tally pass over the live entry
  table at `0x5c8`:
  - it counts codes `1..7` into scratch counters rooted at
    `352c/352a/3528/3534/352e/3530/3532`
  - it then hands scratch block `3502` to `2000:ba44`
- no preserved ES-side writer currently feeds local timing code `2`
  - consumer-side helpers still recognize it
  - for implementation depth it is best treated as an unfed/reserved slot in
    the preserved image, not as an active event family Rust still needs to
    recover

## Rust-Facing Closure

No remaining timing questions in this document block Rust clone development.

The direct movement/combat families that once looked like week-placement
week-placement holes are now bounded strongly enough to implement:

- `entered-system -> attacked` is shared-stream arrival/contact behavior, not a
  fixed delay
- `identified -> orbit-world` and `orbit-world -> sensor-contact` belong to the
  standing extended-orbit status family
- bombardment continuation after hostile encounter is a standing mission
  cadence, not a single hidden `attack -> bombing-run` offset

What remains here is historical/static detail, not Rust-facing weekly
behavior.

## Low-Value Remaining RE Trivia

- which exact helper(s) inside the late player-report tail append the
  `Stardate: <week>/<year>` header fragment
- which exact helper(s) inside the optional rankings branch emit the year-only
  `Stardate: YYYY A.D.` banner text
- the exact historical label for the scratch-local code-`1` bucket now written
  by `0000:f1ba`

These are still mildly interesting RE targets, but they do not block Rust
implementation now that the output contract and phase placement are already
recovered.

## Optional Historical Follow-Up

- if useful later, identify the exact helper inside the late player-report tail
  that appends the already-settled `Stardate: <week>/<year>` fragment
- if useful later, identify the exact helper inside the rankings branch that
  emits the already-settled `Stardate: YYYY A.D.` banner
- if useful later, recover the historical label behind the scratch-local
  timing code `1`

## Strongest Late-Scheduler Static Path

The current strongest static timing seam is no longer just `1000:a26e` by
itself.

Recovered late weekly chain:

- `0000:127A..1361`
  - explicit outer `1..52` loop
- `0000:1333 -> 0000:02c0`
  - decodes active kind-`1` summary entries through `2000:c067`
  - seeds large stack-resident local timing state
- `0000:1339 -> 1000:a26e`
  - this is a mid-function entry inside `1000:9fa1`
  - it walks a local `0x0a`-byte code table and derives two timing-window
    families
- `1000:c102 -> 1000:9fa1 -> 1000:9c0e`
  - the same timing worker family is then consumed by `1000:c102`
  - `c102` calls `9c0e` twice with selector args `2` then `1`
  - current best reading is that `c102/9c0e` score the current week candidate
    against the computed timing windows and set a rejection flag when the slot
    is outside the acceptable range

The timing-code mapping still recovered in `a26e` remains:

- code `1` -> `+2`
- code `2` -> `+7`
- code `3` -> `+0x15`
- code `8` -> `+0x1e`
- codes `4..7` -> `+0`

Practical interpretation:

- the late weekly side now looks like an explicit placement mechanism with
  decode, window-derivation, and accept/reject testing
- this is stronger than the earlier "maybe offset shaping" read
- the preserved `0000:02c0` dispatch now bounds code `7` more tightly:
  - it is assigned only in the kind-`3` branch
  - the archived summary-dispatch RE already identifies kind `3` as the
    `IPBM` summary family
- whole-image timing-entry scans now also bound code `8` more tightly:
  - consumer-side helpers still compare against code `8`
  - no preserved ES-side writer feeds timing-entry code `8`
  - the only newly found `[-0x0a]` writes are SS-local scratch writes,
    not entry-table writes
- so code `8` is best treated as an unfed consumer-side case in the
  preserved image, not a reachable timing class
- the later local timing-entry path now tightens `1/2` further:
  - `0000:f1ba` writes only `0/1` into the scratch-local timing-entry code
    byte
  - `0000:f914` later tallies live entry-table codes `1..7` at `0x5c8`
  - no preserved writer currently feeds code `2` into either the scratch-local
    or ES-resident timing-entry tables captured so far
- practical implementation consequence:
  - the late static path no longer leaves a Rust-facing scheduler hole
  - the direct same-source variable-gap families above are better treated as
    standing mission/status behavior inside one shared weekly stream, not as
    evidence of one global delay table Rust still needed
  - the cross-source Fleet Command Center / planet-loss adjacency patterns are
    likewise better treated as same-stream interleaving, not as same-source
    mission timing

## Working Model

Current best model:

- each player round is still one year, matching the manuals
- within that yearly maintenance pass, `ECMAINT` appears to model a 52-step
  internal timeline
- the leading semantic interpretation of that timeline is week-of-year
- reports are timestamped with event ticks inside that yearly timeline, not
  just "the date maintenance ran"
- those timestamps are rendered in classic player reports as a right-justified
  first-line `Stardate: <week>/<year>` header fragment
- this timeline is mechanically relevant to mission/report sequencing, not
  just decorative report narration

That model is well supported by the shipped logs. The exact report-writer
helper path is still only partially mapped, but that no longer changes the
Rust timing contract.
