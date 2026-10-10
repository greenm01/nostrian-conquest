# NC Report Spec

This doc is the canonical player-facing report spec for the Rust engine.

Use it to guide report wording, format, and family-specific presentation across
`nc-engine`, `nc-cli`, and `nc-game`.

Read this together with:

- [ec-timing-spec.md](ec-timing-spec.md)
  for `Stardate` placement and weekly assignment
- [nc-combat-spec.md](nc-combat-spec.md)
  for combat and hostile-world mechanics
- [rust-turn-cycle-implementation.md](rust-turn-cycle-implementation.md)
  for where reports are emitted in the yearly loop

## Report Policy

The Rust engine uses a hybrid report surface:

- combat and hostile-world aftermath use a structured briefing format
- movement, logistics, intel, and administrative flow stay narrative by default
- repeated join logistics should collapse into one compact Fleet Command summary
- the shell stays consistent across both styles
- hostile contact and combat reports are viewer-targeted, not broadcast; each
  directly involved empire must receive at least one report for the event

The report surface is no longer governed by classic EC prose templates. Classic
behavior still matters at the file/export boundary, but Rust report wording is
owned here.

## Shared Shell

Every player-visible report keeps the same outer shell:

- first line: source header plus right-justified `Stardate: <week>/<year>`
- body: one or more wrapped 72-column lines
- final line: lowercase `<end of transmission>`

Shell rules:

- keep the source clause specific: fleet, planet, starbase, or Fleet Command
  Center
- fleet-origin reports must name a fleet whenever that identity is known
- use full ship names, not abbreviations such as `BB`, `CA`, or `DD`
- wrap on word boundaries
- preserve explicit blank separator lines in structured reports
- the week is zero-padded to two digits (`Stardate: 02/3031`); see
  [ec-timing-spec.md](ec-timing-spec.md)
- report text is printable ASCII only; write `--` rather than an em dash, and
  expect any other character (for example in imported names) to render as `?`
- lines a report builder already laid out (labeled rows, hanging indents)
  pass through unchanged; only lines longer than 72 columns are re-wrapped

## Format Choice

Use the report family to choose the body style.

### Structured Reports

Use structured reports when the player is comparing multiple facts in one event:

- fleet battles
- Fleet Command Center lost-fleet and lost-starbase combat aftermath
- bombardment reports
- invasion reports
- blitz reports
- defender-side captured-world reports

Structured body shape:

```text
[family title]

[context rows]

[force or defense rows]

[short outcome lines]
```

Rules:

- use left-justified labeled rows
- group structured combat reports into three sections after the title:
  context rows, force/defense rows, then outcomes
- use blank lines between title and each non-empty section
- outcome lines should be short and direct
- do not restate facts already shown in the labeled rows
- prefer labels such as `Our forces:`, `Alien forces:`, `Our defenses:`,
  `Attacking force:`, `Our losses:`, and `Enemy losses:`
- keep `Fleet lost:`, `Starbase lost:`, `Last contact:`, `Enemy:`,
  `Attacker:`, `Invader:`, and `Target world:` in the context section
- for Fleet Command Center destruction telemetry, prefer `destroyed by ...`
  or `destroyed while intercepting ...` over `was attacked by ...`
- emit the same lost-contact telemetry family when a bombardment, invasion, or
  blitz assault force is completely destroyed
- keep starbase-loss telemetry defender-side; the attacker learns about a
  destroyed starbase from the ordinary battle report
- when all initial planetary defenses are eliminated, prefer
  `All planetary defenses were destroyed.`

### Narrative Reports

Use narrative reports when the player is following a single event flow rather
than scanning a comparison:

- move / patrol / seek-home / join / rendezvous reports
- except that per-turn `JoinAnotherFleet` churn should be summarized once from
  Fleet Command instead of emitting one notice per affected fleet
- retarget and follow-on notices
- colonization and salvage reports
- most abort notices
- administrative and validation notices

Rules:

- prefer one clear event sentence followed by only the minimum supporting
  detail
- avoid clause chains that repeat the same fact with slightly different words
- keep the tone report-like, not bureaucratic

### Hybrid Intel Reports

Scouting and viewing reports may mix a short narrative lead-in with compact
fact rows when that reads better than either pure style.

Use hybrid formatting for dense intel only when the extra structure materially
improves readability.

## Style Rules

- keep viewpoint consistent within a report
- use first person for the viewer's own forces and status
- use direct, concrete wording over filler
- avoid robotic stitch-phrases like `Our defenses had X. We lost X.` when a
  single outcome line can say the same thing
- avoid freeform flourish; the tone should still feel like a concise
  transmission
- use labels for factual inventory, not for obvious one-step narrative events

Zero-count rules:

- do not emit `0 ground battery(ies)` or `0 army(ies)`
- use `none`, `undefended`, or a family-specific zero phrase instead
- omit meaningless zero-loss prose

## Family Guidance

### Combat And Hostile-World Reports

These should optimize for scanability. A player should be able to find:

- who engaged whom
- force composition
- defenses
- outcome
- friendly losses
- enemy losses

Coverage rules:

- if one empire loses a fleet or starbase, that empire may receive telemetry
  instead of a generic battle report
- if a planet repels or blocks an invasion or blitz, the defender should still
  receive a structured aftermath report; failed assaults must not be attacker-
  only
- blocked hostile-world reports should distinguish "orbit denied / landing
  blocked" from "assault capability destroyed"; do not reuse capability-loss
  wording when the fleet still has valid loaded transports

Prefer one short outcome sentence plus labeled loss rows over paragraph-style
battle narration.

### Empire Elimination And Game-End Reports

These should optimize for finality and clarity. A player should immediately
understand:

- whether one empire was eliminated or the whole campaign has ended
- who delivered the final blow, if any
- what access remains after the report

Rules:

- use Fleet Command Center as the source
- loser-side defeat reports should explicitly say the empire has been defeated
  and command authority is ending
- victor-side final-blow reports should identify the defeated empire and
  distinguish last-planet elimination from destruction of the last recovery
  force
- winner-declared reports should explicitly say the campaign is over
- the campaign-outlook notice reaches every joined empire; the sole remaining
  contender reads it as its own claim (`We now stand ... our claim`), and
  every other empire sees the contender named
- winner-side wording should mention survey-only access
- loser-side game-over wording should mention one final review-only pass
- already defeated and already blocked empires do not receive repeated
  winner-declared notices

### Movement And Logistics Reports

These should optimize for event flow. A player should be able to read them as a
status update:

- what changed
- where the fleet now is
- what happens next

Do not convert simple movement/retarget/status notices into form-like blocks
unless they become fact-dense enough to justify it.

### Intel Reports

These should present findings without sounding like a database dump.

Use a brief narrative lead-in, then compact fact presentation only for the
world/fleet details that matter to planning.

## Examples

### Structured Battle Report

```text
From your 3rd Fleet, located in System(9,5):           Stardate: 02/3031
ALERT: Enemy fleet contact!

Enemy:             The 8th Fleet of "Player2", (Empire #2)
Our forces:        2 battleships, 4 cruisers and 6 destroyers
Alien forces:      1 battleship, 5 cruisers and 3 destroyers

Interception successful.
The enemy fled the field.
Our losses:        1 cruiser and 2 destroyers
Alien losses:      2 cruisers and 1 destroyer
<end of transmission>
```

### Structured Captured-World Report

```text
From planet "Red" in System(9,6):                      Stardate: 03/3031
ALERT: Planet lost to enemy invasion!

Invader:           "Player1", (Empire #1)
Attacking force:   11 battleships, 28 cruisers, 8 destroyers and 106 troop
                   transport ships
Our defenses:      12 ground batteries and 40 armies

All planetary defenses were destroyed.
Enemy losses:      no ship losses and 14 armies
Our orbital softening losses: 20 armies
Our ground battle losses: 20 armies
<end of transmission>
```

### Structured Bombardment Report

```text
From planet "biggy" in System(1,9):                    Stardate: 03/3040
ALERT: Orbital bombardment underway!

Attacker:          The 10th Fleet of "Player2", (Empire #2)
Attacking force:   26 battleships, 58 cruisers and 68 troop transport ships
Our defenses:      3 armies

All planetary defenses were destroyed.
Local damage:      7 stardock items destroyed.
Local damage:      10497 points of industry destroyed.
Local damage:      42 stored production destroyed.
<end of transmission>
```

### Compact Join Summary

```text
From your Fleet Command Center:                        Stardate: 01/3031
Join mission summary
Completed joins: Fleets 8 and 11 merged into Fleet 3.
Retargeted to follow host: Fleets 4, 5, 6 and 7.
Lost hosts: Fleets 10 and 12 lost host Fleet 14 and are holding position.
<end of transmission>
```

### Winner Declaration Report

```text
From your Fleet Command Center:                        Stardate: 04/3059
ALERT: Victory declared! We have been recognized as Emperor. The campaign is
over. You may continue to review the final game state in survey mode, but no
further orders will be accepted.
<end of transmission>
```

### Hybrid Intel Report

```text
From your 4th Fleet, located in System(6,14):          Stardate: 04/3031
Scouting mission report: We are in extended orbit around planet "Spyglass"
and have compiled the following data:
Owned by:         Empire #2
Potential production: 100 points
Estimated production: 73 points
Estimated amount of stored goods: 41 points
Number of armies: 7
Number of ground batteries: 2
The planet's stardock appears to be empty.
<end of transmission>
```

## Implementation Rule

When Rust report wording and older classic prose differ:

- follow this doc for player-facing Rust behavior
- keep classic compatibility requirements in
  [classic-results-dat-compat.md](classic-results-dat-compat.md)
- do not reintroduce classic paragraph wording as the default authority unless
  there is a deliberate product decision to do so
