# Developer Documentation

This directory holds the current engineering docs for Esterian Conquest.

Use this file as the entrypoint. It tells you which docs are authoritative for
Rust implementation, which ones are workflow guides, and which ones are
reference/background only.

## Canonical Rust Implementation Docs

These are the docs that should drive engine/client behavior.

- [ec-turn-cycle-spec.md](ec-turn-cycle-spec.md)
  - canonical oracle-backed yearly maintenance ordering
- [rust-turn-cycle-implementation.md](rust-turn-cycle-implementation.md)
  - implementation-facing Rust companion for the turn cycle
- [nc-combat-spec.md](nc-combat-spec.md)
  - canonical Rust combat and hostile world-resolution mechanics
- [ec-timing-spec.md](ec-timing-spec.md)
  - weekly scheduler and `Stardate` behavior
- [economics.md](economics.md)
  - economy/build policy and post-loop world/player updates
- [ec-movement-spec.md](ec-movement-spec.md)
  - movement, contact, and planner-facing movement rules
- [ec-encounter-spec.md](ec-encounter-spec.md)
  - classic's per-tick encounter loop: sighting, decision, intercept
- [ec-starbase-spec.md](ec-starbase-spec.md)
  - the BASES.DAT layout and classic starbase movement
- [ec-file-formats.md](ec-file-formats.md)
  - every byte of the seven classic data files
- [order-selection-targeting.md](order-selection-targeting.md)
  - canonical fleet-order target default and validation behavior for the Rust client
- [ec-setup-spec.md](ec-setup-spec.md)
  - setup/generator expectations and compatibility boundaries
- [nc-reports.md](nc-reports.md)
  - canonical Rust-engine report wording, family style, and format policy
- [nc-game-prompt-standard.md](nc-game-prompt-standard.md)
  - canonical non-table command-line prompt grammar and theming for the Rust client

## Architecture And Workflow Docs

These docs explain how to work in the repo and how to structure the Rust side.

- [next-session.md](next-session.md)
  - short restart brief for the next development session
- [approach.md](approach.md)
  - project preservation strategy and evidence policy
- [rust-architecture.md](rust-architecture.md)
  - repository-wide Rust architecture and DOD rules
- [ghidra-workflow.md](ghidra-workflow.md)
  - Ghidra investigation workflow
- [dosbox-workflow.md](dosbox-workflow.md)
  - DOSBox-based oracle/dynamic investigation workflow
- [fixtures.md](fixtures.md)
  - fixture layout and intended usage
- [localhost-nc-host.md](localhost-nc-host.md)
  - dev-only localhost hosted lab for `nc-host` + `nc-helm`
- [../helm/lobby-architecture.md](../helm/lobby-architecture.md)
  - hosted lobby and client-state architecture for `nc-helm`
- [harness/README.md](harness/README.md)
  - typed KDL and CLI workflow for campaign play, runtime scenarios, combat scenarios, and combat sweeps
- [harness/campaign-play.md](harness/campaign-play.md)
  - reproducible conductor workflow for "play to turn N, then inspect in the TUI"
- [combat-balance-workflow.md](combat-balance-workflow.md)
  - how combat rule candidates get numbers: exact enumeration, seeded sampling, campaign scenarios
- [llm-player-guide.md](llm-player-guide.md)
  - strict visible-state operating guide for bots/LLMs playing real campaigns

## Reference And Background Docs

These docs are useful inputs, but they are not the source of truth for Rust
behavior by themselves.

- [ecmaint-combat-reference.md](ecmaint-combat-reference.md)
  - historical combat-oriented oracle/reference notes
- [ec-combat-reference.md](ec-combat-reference.md)
  - classic's combat in one place, with [ec-battle-spec.md](ec-battle-spec.md)
    for the decoded battle; NC keeps its own combat system
- [ec-economy-reference.md](ec-economy-reference.md)
  - classic's economy, decoded and probed; NC keeps its own economy
- [planet-report-reference.md](planet-report-reference.md)
  - report-side target-world reference profiles
- [classic-results-dat-compat.md](classic-results-dat-compat.md)
  - classic `RESULTS.DAT` binary/export compatibility contract
- [bbs_door_client_rust.md](bbs_door_client_rust.md)
  - client/delivery direction for the SQLite-native Rust player-side stack
- [config-architecture.md](config-architecture.md)
  - KDL/config extraction boundaries and sequencing
- [llm-test-harness.md](llm-test-harness.md)
  - original DOS/BBS black-box automation notes for RE-oriented LLM play

## Reading Order

For gameplay/engine work:

1. [next-session.md](next-session.md)
2. [approach.md](approach.md)
3. [rust-architecture.md](rust-architecture.md)
4. the canonical spec docs listed above for the subsystem you are touching

For client/UI work:

1. [next-session.md](next-session.md)
2. [bbs_door_client_rust.md](bbs_door_client_rust.md)
3. [nc-game-prompt-standard.md](nc-game-prompt-standard.md)
4. [rust-architecture.md](rust-architecture.md)

## Doc Policy

- keep authoritative Rust behavior in the canonical spec docs
- keep `next-session.md` short and current
- keep historical notebooks and bulky RE detail in `archive/`
- keep reference docs clearly labeled when they are useful but non-canonical
