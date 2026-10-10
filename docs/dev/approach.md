# Preservation Approach

This repository does not recover the Pascal source code verbatim. The goal is
to preserve Esterian Conquest v1.5 as a working historical artifact. We reverse
engineer its file formats and rules to build Rust tooling that generates 100%
compliant gamestate files. These files are accepted by `ECMAINT` and the 1992
executables. That compliance target is our first milestone toward a faithful
modern reimplementation. The DOS binaries and data remain the reference
throughout.

## Principles

### Manuals as Spec, Binaries as Oracle

The Rust manuals define the intended player rules and mechanics. The 1992 `.DOC`
manuals are historical references and an fallback for ambiguity. The three
executables each serve a distinct role: `ECGAME.EXE` is the player UI,
`ECUTIL.EXE` is the sysop utility, and `ECMAINT.EXE` is the maintenance engine.

When semantics diverge, prefer the current manuals for gameplay meaning and the
binaries for file compatibility. Use the 1992 `.DOC` set to clarify operator
intent, then fold that into the Rust manuals. Do not chase byte-perfect parity
if it forces Rust away from documented rules. Document logic bugs when they
matter for oracle work, but do not reproduce them unless required for file
safety.

### Confirmed Behavior Over Guessed Structure

Name fields only after confirming them through diffs, screenshots, or manuals.
Keep unknown bytes raw until mapped with confidence. When the UI exposes a
high-level label, prefer that term. For example, Rust may carry internal field
names like `factories` during development, but player surfaces must align to
current terms like **Production**, **Potential Production**, and **Total
Available Points**.

### Meaningful Fidelity Over Ghost Fidelity

We do not preserve every hidden numeric quirk or ambiguous artifact. Our
fidelity target is:

- player-facing rules and timing that affect decisions
- compatibility required for file safety and oracle workflows
- stable semantics that improve the Rust engine

If an oracle thread bottoms out in hidden state or inconsistent behavior,
prefer a documented Rust rule. The Rust combat system is the model: preserve the
turn placement and visible consequences, but do not freeze the engine waiting
for perfect historical internals.

### Reverse Engineering is a Closed Phase

The discovery phase is complete. The 1992 manuals, binaries, fixtures, and
archived notes remain essential resources, but they are no longer the daily
center of gravity.

From here forward:

- Oracle and Ghidra work are targeted tools, not a standing project stream.
- The default path is to refine the Rust engine against the documented rules.
- Deep investigation reopens only for a concrete regression, crash, or
  file-safety issue.
- Unresolved low-level quirks stay documented without blocking the engine.

### Stable Docs vs. Lab Notes

`docs/dev/archive/RE_NOTES.md` is the chronological investigation notebook. It
is archival material. `docs/` holds stable, reusable engineering docs.

### Layered Architecture

The Rust workspace is organized into focused crates. Each owns a slice of the
problem: classic codecs, runtime state, the engine/rules boundary, and the
player-facing TUI. The implementation is data-oriented. It uses explicit record
layouts, focused free functions, and feature-oriented submodules. See
[rust-architecture.md](rust-architecture.md).

### Fixtures to Lock in Behavior

Fixtures cover the key states for regression testing: shipped state,
initialized state, post-maintenance state, and targeted scenario snapshots.

### Engine Outputs Over UI Playback

`ECMAINT` writes the underlying state and report data. `ECGAME` remains a
viewer/validation layer. Decode changes in `.DAT` files first and use report
viewing second. Historical text captures remain reference evidence.

The Rust `nc-game` client must preserve the 1992 pre-menu flow. Startup is more
than a splash screen. We model the full path: the ASCII splash, the intro text,
and the onboarding flow. Startup presentation is game-owned and lives in the
Rust client. The classic sequence is preserved while modernizing friction:
ANSI/CP437 by default, cleaner prompt handling, and safer input validation.

### How the Game Was Recovered

The Rust engine was not built from guesswork. The model and docs came from
cross-checking the 1992 manuals, the binaries, fixtures, and Rust-generated
scenarios.

| Tool / Source | Purpose | Impact |
| --- | --- | --- |
| Rust Manuals + 1992 `.DOC` set | Authoritative gameplay and sysop docs | Grounded the Rust clone in intended behavior |
| Ghidra Disassembly | Static recovery of file layouts and maint flow | Turned opaque Pascal paths into stable specs |
| DOSBox-X Debugger | Dynamic tracing of file I/O and state changes | Proved phase ordering and report boundaries |
| File Diffs | Compared Rust directories against classic `.DAT` outputs | Exposed cross-file invariants |
| Report Analysis | Studied `RESULTS.DAT` and `MESSAGES.DAT` | Recovered timing and event sequencing |
| Scenario Sweeps | Created test cases and ran binaries as oracle | Turned RE into reusable implementation guidance |

For the full evidence entrypoint, see
[reverse_engineering/README.md](../reverse_engineering/README.md).

### Escalating Investigation Depth

Start with Rust scenarios and black-box `ECMAINT` testing. Promote deterministic
patterns into shared rules. Escalate to deep investigation only when a path
blocks compliant gamestate generation and the expected rule is reusable. Stop
once the rule is explicit enough to promote into Rust. The **Guard Starbase**
investigation is the template for a justified deep dive.

### Controlled Oracle Loops

The default workflow for a new mechanic: initialize a directory in Rust, submit
one scoped order family, run `ECMAINT` as the oracle, and diff the resulting
`.DAT` files. Promote deterministic effects into the engine. Deep investigation
is reserved for when this loop stops yielding rules. DOSBox traces are coarse
evidence; they prove broad clustering but not exact internal ordering.

### Setup and Map Generation

The manuals define galaxy size by player count and system count. The Rust
builder is infrastructure, but not yet a faithful initializer. Setup is a
manual-driven subsystem covering map dimensions, star count, and homeworld
rules.

The admin setup preserves the pre-join distinction. `nc-game` onboarding must see
inactive player slots and "Not Named Yet" homeworlds. When a player joins, he
immediately sees the starting production defined by the manuals. We do not
reproduce the hidden map RNG, but we strictly adhere to the documented setup
rules.

### Recovered Mechanics vs. Canonical Policy

Movement execution follows recovered behavior. Route selection and threat-aware
navigation are improved canonically in Rust. Smart pathfinding is a Rust policy
layer, not a recovered mechanic.

If the 1992 game shows that diagonal trips complete conservatively, Rust
preserves that outcome. Unresolved scratch-byte layouts are not a fidelity
target.

### Seeded Rust Combat

We do not reproduce the opaque combat RNG. Seeded, reproducible Rust combat is
the canonical replacement. Combat outcomes still fold into the maintenance
structure: turn order, weekly timing, and report emission.

Do not waste effort cloning Pascal-era randomness. Instead, recover where
combat happens in the turn and when its consequences land on the timeline. Rust
converges on seeded combat, faithful phase placement, and faithful event
scheduling.

### Declarative Sysop Config

Admin data is declarative and lives in KDL. The source of truth for setup
presets is machine-readable config covering player count, year, and
maintenance schedule. CLI and TUI surfaces are frontends over this config and
the shared Rust model.

### Storage and Compatibility

The engineering target is a full-game-capable Rust maintenance engine with
classic `.DAT` fidelity. Modern storage is an adjacent milestone. KDL remains
focused on authored setup and scenarios.

SQLite sits at the runtime center without replacing the compliance boundary.
**CoreGameData** is the in-memory snapshot, `ncgame.db` is the source of truth,
and `.DAT` files are projections for the oracle. `nc-game` and maintenance
operate on SQLite state. `nc-cli` provides the bridge for classic directories.
SQLite is bundled in the Rust application; the administrator does not need to
install it separately.

### Economy Semantics

The manuals define the economy tradeoff: taxes generate revenue, new colonies
start below potential, and starbases accelerate growth. If the `ECMAINT` replay
path is awkward, we prefer a documented canonical Rust rule. This rule must be
simple, monotonic, and auditable. Original evidence refines the rule, but we do
not freeze progress waiting for perfect replay coverage. The canonical economy
rule is in [docs/economics.md](economics.md).

### Diplomacy and Hostility

**Enemy** is a diplomatic relation set by a player. **Hostile** is the
maintenance state that triggers battle. A contact becomes hostile because a
player declared an enemy, attacked first, or entered a defended system. Rust
models this distinction. Where classic `PLAYER.DAT` bytes are known, they are
authoritative.

### Surrender as Campaign State

The manuals describe surrender and acknowledgement of an emperor as the victory
condition. `ECGAME` does not include a surrender action. Rust does not invent
one. Instead, we separate mechanical defeat (destruction of armies and planets)
from political victory (recognition of an emperor).

### Compatible Gamestate

The Rust engine prefers **classic-compatible save directories** over mimicking every
hidden quirk. For unresolved mechanics, a documented canonical Rust rule is
acceptable if the resulting `.DAT` files remain loadable and sane. The rule must
be faithful to the manuals and regression-testable.

File compatibility is strict. Deterministic mechanics must match exactly.
Stochastic mechanics may diverge if the divergence is explicit and
reproducible.

### Own the Mechanics; Do Not Reproduce the RNG Stream

`ECMAINT` uses an internal RNG for combat resolution and AI decisions. We do
not emulate this state. Instead, we implement our own seeded versions of every
mechanic. We use the binaries to understand the structure of changes, then
define our own rules for the magnitude of effects. These rules are documented
in `RE_NOTES.md` and are auditable.

The acceptance criterion is internal consistency and plausibility, not a
byte-exact fixture match. The campaign seed belongs to the engine. The Rust
client may derive cosmetic choices from that seed, but they must not affect
gameplay. Byte-exact match is only for deterministic mechanics: movement,
build queues, and economy.

### Near-Term Acceptance Rule

A mechanic is "done" when Rust emits the state and the binaries accept it. The
1992 executables are a compatibility oracle, not a universal semantics oracle.
Bit-perfect parity is pursued only when it supports the manuals.

For stochastic mechanics, "done" means correct structure and plausible
magnitudes. Adherence to the manuals is a better target than reproducing a
hidden implementation artifact.

## What Counts As Success

Success means decoding the on-disk formats, reproducing `ECUTIL` behavior, and
understanding `ECMAINT` as a deterministic transformer. We must generate fully
compliant gamestate files.

Long-term, we reimplement the engine in Rust and build a usable client. We
preserve the original ANSI presentation for use in the Rust client. The
project will support both classic `.DAT` interchange and the richer
SQLite-backed runtime layer.

## Milestone Ladder

1. **Known Accepted Scenarios.** Rust emits preserved pre-maint scenarios. The
   binaries and fixtures are the oracle.
2. **Parameterized Scenario Generation.** Replace constants with explicit
   builders and validators. Generate families of accepted shapes.
3. **General Compliant Gamestate Generation.** Rust writes arbitrary gamestate
   directories that `ECMAINT` accepts. This requires cross-file linkage rules.
4. **Full Rust Maintenance Replacement.** Reimplement `ECMAINT` in Rust.
   Preserve compatibility with save directories and reports.
5. **Scenario KDL Layer.** Add a human-editable scenario format once the
   internal model stabilizes. KDL is a serialization layer. Rust remains the
   authority for maintenance sequencing.
6. **ANSI Preservation Layer.** Capture and preserve the original `ECGAME`
   output. Treat these as reference assets for the Rust client.

## RE Workflow

The default loop: generate a scenario in Rust, run the 1992 binary as the
oracle, and diff the resulting `.DAT` files. Promote strong patterns into
the engine.

## Event And Report Direction

Maintenance consequences are modeled as typed events and rendered into classic
reports. Formatting is not embedded in mechanic paths.

### Event Modeling Policy

Maintenance events are pushed through a single report pass. Scout arrival
reports use the generic mission-outcome backbone. `ViewWorld` uses the
intel-refresh path.

When combat forces a fleet off its orders, the system emits an `Aborted`
mission outcome. Hostile contact detection happens during the battle phase.
The contact event family is mission-aware so all reports share one detection
path.

Fleet contact events emit a **single merged report** that combines the sensor
contact flavor with the fleet identification inline:
`"Sensor contact -- detected and identified an alien fleet in {location}.
It is {enemy}. Their fleet contains {size_summary} of unknown type."`
This replaces the classic two-report prelude+identify pattern.
`ContactReportSource::Starbase` reports and `EncounterDispositionEvent`
reports (`Retreated`, `PursuitFire`) carry distinct information and remain
as separate reports.

### Recipient Scoping and Loss Reporting

We prefer recipient-scoped maintenance events. Bombardment, battle, and
scouting are modeled from the affected empire's point of view. Fleets and
starbases that are wiped out emit command-center loss reports. Every fleet
encounter emits a contact event even if no battle occurs.

### Classic Report File Compatibility

`RESULTS.DAT` is the canonical report target. Until the classic mail format is
recovered, Rust preserves existing `MESSAGES.DAT` payloads and keeps
Rust-originated mail in SQLite.

For the oracle runbooks, see
[reverse_engineering/README.md](../reverse_engineering/README.md#oracle-runbooks).
