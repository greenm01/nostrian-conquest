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

## Owner wording review

The goal is to dominate the galaxy, not one sector. Describe player activity
as asynchronous, turn-based play, with deliberate time for planning and
diplomacy. Use fleet orders, not naval orders. Maintenance resolution and
simultaneous fire within a fleet battle are separate mechanics; they do not
mean players must act together. Updated the quick-start introduction, the
full manual introduction to gameplay, and the README accordingly.

A second tax wording pass makes the shared rate explicit in the first-turn
checklist, colony advice, Planet Command description, and formula appendix.
Individual planets cannot have separate tax rates. The 30%/60% growth example
now compares empire-wide policies, not differently taxed colonies.

Recovery wording now leads with automatic healing at the end of the second
turn after damage, then explains the one-turn orbital starbase bonus. This
is the existing RT contract, not a same-sector bonus or a timing change.

At the owner’s request, the quick-start now uses the player manual’s exact
CC BY-NC-SA 4.0 notice, shared badge, and license link on a dedicated final
page. The guide has three gameplay pages and one license page.

## Second complete quick-start audit

Re-read the current guide claim by claim against the running edition's code,
not the supplied draft. Additional source checks:

- [New-game builder](../../../rust/nc-data/src/builder.rs): actual joinable
  fleet blocks and `seeded_homeworld`, rather than only compatibility helpers.
- [Production kinds](../../../rust/nc-data/src/model/planet.rs): every listed
  cost; [ship speeds](../../../rust/nc-data/src/fleet_ships.rs): every maximum.
- [Fleet order validation](../../../rust/nc-data/src/directory/mutators_fleet.rs):
  editable speed, loaded-transport and warship requirements, scout-only missions.
- [ROE rules](../../../rust/nc-engine/src/maint/encounter/rules.rs): 0, 6, 10
  and the distinction between willingness and diplomatic hostility.
- [Recovery cohorts](../../../rust/nc-data/src/model/crippled.rs) and
  [assistance tests](../../../rust/nc-engine/src/maint/ship_recovery_tests.rs):
  no damage-year countdown; the fleet, base and ownership must qualify all year.
- [Ship notation](../../../rust/nc-data/src/ship_notation.rs): loaded/empty
  categories and crippled counts included in totals.
- [Fleet controller](../../../rust/nc-game/src/domains/fleet/controller.rs):
  ETA asks whether to include system entry. Rechecked main/general/fleet/planet
  menu entries and Build Commands' S key directly in screen code.
- [Campaign outcome](../../../rust/nc-engine/src/maint/campaign/state.rs):
  sole contender recognized as Emperor, not a one-sector victory condition.

Corrected remaining overstatements: speeds are maxima, builds may span turns,
ETACs carry no armies (an ETAC can still accompany an invasion fleet), colony
production is capped by Potential, the assisting base must also stay put,
and industry preservation has a Scorch Earth exception. Added the missing
load-armies step and Invade's warship requirement. Distinguished a failed
landing that leaves a fleet holding orbit from defeat in the orbital fight.
No game behavior changed.

Rebuilt the quick-start PDF without warnings and rendered every page to inspect
page breaks, table widths, and the license. The final rendering has three gameplay pages plus one license page, with no
stray overflow page.

The owner made offline map planning a first-turn priority. Both quick-start
sections now lead with a printed player map or a spreadsheet of known systems,
updated from reports. BBS wording directs players to the sysop for available
player-map exports rather than promising client-side file downloads.

The quick-start opens with the first paragraph of the game’s
[INTRO_PAGE_1](../../../rust/nc-game/src/domains/startup/screens/startup.rs),
quoted unchanged apart from wrapping. The map checklist remains the first
action. Rebuilt both PDFs and checked the quick-start page layout before
the signed documentation commit.
