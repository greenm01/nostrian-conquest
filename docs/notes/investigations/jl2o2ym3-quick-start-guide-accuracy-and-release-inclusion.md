---
id: jl2o2ym3
date: 2026-10-11
kind: investigation
status: resolved
tags: [investigation, docs, release]
---
# Quick-start guide accuracy and release inclusion

## Scope

t114 reviews the supplied player quick-start draft against the full manual and
current code, links it from the README, and includes its PDF in release bundles.
No gameplay rules change. The draft needed substantial corrections before use.

## Corrections and evidence

- Opening assets: Production 100, treasury 35, 10 armies, four batteries;
  two CA/ETAC fleets and two DD fleets. No armies aboard the ETACs and no
  population count. See [baseline initialization](../../../rust/nc-data/src/directory/current_known_baseline.rs).
- Colonization uses mission 12 and leaves the ETAC available for reuse. It
  does not require loading an army. New colonies start at min(3, potential).
  See [colonization](../../../rust/nc-engine/src/maint/merging/colonization.rs).
- Taxes are empire-wide. The draft's per-world advice and its claim that
  high taxes permanently lower potential were wrong. Removed the unsupported
  tax-triggered rebellion story. The full manual had three misleading
  per-colony tax recommendations; these now explicitly say empire-wide.
  See [economy arithmetic](../../../rust/nc-data/src/economy.rs) and
  [yearly economics](../../../rust/nc-engine/src/maint/economics/planets.rs).
- Corrected speeds to DD/SC 6, CA/TT 5, BB 4, ETAC 3, SB 1. Separated TT
  from ETAC and added scouts. World travel includes climbing out and diving
  in; the draft's guaranteed next-year colonization was wrong.
  See [ship speed](../../../rust/nc-data/src/fleet_ships.rs) and the manual's
  Movement and Arrival section.
- Builds complete during maintenance, before movement; batteries are not
  instant purchases. Ships need commissioning. See
  [build completion](../../../rust/nc-engine/src/maint/economics/build.rs) and
  [maintenance order](../../../rust/nc-engine/src/maint/mod.rs).
- Recovery is automatic and free, with no speed penalty. RT 2 excludes the
  damage year. A full year beside an owned orbital base helps older cripples;
  it is not a paid repair yard. See
  [recovery](../../../rust/nc-engine/src/maint/ship_recovery.rs).
- Replaced invented two-posture ROE with levels 0–10; removed guaranteed
  destruction of unescorted transports and automatic-war claims. Explained
  retreat risk and the critical exception to screening. See
  [fleet battles](../../../rust/nc-engine/src/maint/combat/fleet_battle.rs),
  [ROE boundary cases](../../../rust/nc-engine/tests/encounter/decisions.rs),
  and [assault resolution](../../../rust/nc-engine/src/maint/combat/assault.rs).
- Replaced fabricated main-menu keys with actual submenu paths. Documented
  Reports/Messages/Sent and separate class/quantity prompts for `3!`.
  Checked [main menu](../../../rust/nc-game/src/domains/startup/screens/main_menu.rs),
  [general menu](../../../rust/nc-game/src/domains/startup/screens/general_menu.rs),
  [fleet screens](../../../rust/nc-game/src/domains/fleet/screens/fleet.rs),
  and [screen help](../../../rust/nc-game/src/app/help.rs).

## Deliverables and validation

The corrected [Typst guide](../../manuals/nc_quick_start.typ) is a three-page
[PDF](../../manuals/nc_quick_start.pdf). All three pages were rendered and
visually checked. The player manual PDF was rebuilt after its tax wording fixes.
Both compiled without Typst warnings.

The repository README and Windows/Linux package READMEs link the guide.
Bundle staging copies it beside the player manual; archive verification
requires it. Staged and extracted all four public sysop archive layouts with
placeholder binaries: Linux x64, Windows x64, Windows x86, Windows 7 x86.
All four included byte-identical PDFs and valid guide links. Removing the guide
made verification fail in all four negative controls. These are packaging
checks, not executable or Windows runtime smoke tests. Python compilation
checks passed. Rust source was unchanged; no new Rust test run was needed.

The revision helper accepts `--doc quick-start` or `--doc all`; `both` retains
its old player/sysop meaning. Text-only `git diff --check` excludes PDFs:
Git otherwise treats the generated PDF cross-reference CRLF records as text.

## Connections and remaining release work

[Release plan](../plans/fv6fg33o-release-v1-0-0-beta-3.md) tracks the remaining
binary, signing, and publication work. The guide is included by future builds;
it has not been uploaded into an existing public archive.
