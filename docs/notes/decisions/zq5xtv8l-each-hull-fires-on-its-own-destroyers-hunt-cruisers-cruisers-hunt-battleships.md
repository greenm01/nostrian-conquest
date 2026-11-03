---
id: zq5xtv8l
date: 2026-11-03
kind: adr
status: accepted
tags: [adr, combat, hulls, balance, game-theory]
---
# Each hull fires on its own: destroyers hunt cruisers, cruisers hunt battleships

## Context

t253 ([hltgp5ow](../investigations/hltgp5ow-fleet-mixes-as-a-game-one-mix-beats-all-others-and-roe-picks-which.md))
treated every equal-cost DD/CA/BB mix as a strategy and solved the game
between them on the engine. At each ROE, one mix beat every other:

| ROE | Winning mix's points, DD / CA / BB |
| --- | --- |
| ROE 10 | 22 / 11 / 67% |
| ROE 6 | 33 / 44 / 22% |

Composition was solved, with no counter-builds. The cause is that all fire
used one target order: battleships, then cruisers, then destroyers. The
owner wanted counters, and accepted hull-specific bonuses to get them
("asymetry is not bad imo").

## Decision

The owner chose R2 on 2026-11-03 and asked to raise the battery to match
the battleship.

1. **Each hull class fires on its own.** A side's volley still rolls once
   on its whole AS. Its hits split among destroyers, cruisers, battleships
   and starbases by each class's share of that AS. Each share is spent and
   carried as its own pool. Pools apply in this order: battleships,
   cruisers, destroyers, starbases.
2. **Destroyers hunt cruisers.** Destroyer fire aims at cruisers first, and
   its hits count three times against them, so a cruiser's DS costs it a
   third, rounded up.
3. **Cruisers hunt battleships.** Cruiser fire aims at battleships first,
   and its hits count twice against them.
4. **Battleships and starbases keep the line order** (BB, CA, DD, SB) with
   no bonus. Each pool keeps its fire type, as before: cruiser and
   battleship fire is dispersed; destroyer and starbase fire is focused and
   finishes cripples at half DS.
5. **Stats:**
   - BB AS 90 → 100, DS 110 → 105.
   - CA DS 30 → 37.
   - Ground battery AS 90 → 100, a battleship's again.
   - Prices, classic's engagement count, criticals and the heavy-line bonus
     are unchanged.

## Alternatives

- **Keep today's rules.** Composition stays solved, with one best mix for
  each ROE.
- **Bonuses with today's stats (candidate C).** At ROE 10 one
  battleship-heavy mix still beat every other, and cruisers dropped to 0% of
  the equilibrium.
- **R: R2 plus an engagement exponent of 4.3.** It played the same as R2.
  It was dropped to keep classic's engagement count
  ([almb7dbt](almb7dbt-a-larger-side-commits-part-of-its-force-and-batteries-punish-a-crowded-orbit.md)).
- **Keeping batteries at AS 90 under R2.** Battleships then cleared
  batteries far more easily. Twenty batteries against twice their AS fell
  85% of the time, against 53% before. The assault cases gained 5–10
  points. At AS 100, sieges and assaults stay within a few points of
  before.

## Consequences

**Combat.** On the engine (t255, branch `t255-combat-search`):
- No mix beats every other at 405 or 1,215 points, at ROE 10 or 6.
- All three hulls hold 19–55% of the points in each equilibrium.
- Single hulls form a cycle. At 405 points and ROE 10: battleships beat
  destroyers by +0.85, destroyers beat cruisers by +0.41, and cruisers
  beat battleships by +0.92.
- More force still always wins. With one side at 125% strength, its odds
  rose in all 882 pairings, at both ROEs.
- Main's engine reproduces the branch's R2 grids battle for battle.

**Other effects.**
- Bombardment by battleships rises with their AS.
- Batteries hit cruiser and destroyer fleets 11% harder. A cruiser's higher
  DS takes up most of that.
- The trained AI and the executors' build weights learned under the old
  rules, and must be re-checked.

## Acceptance and connections

Accepted by the owner on 2026-11-03 ("adopt r2, but should we increase GB
AS to 100 to match BB's? we advertise GB's as having the same firepower of
a BB"). Task t255.

- **Evidence:** the t255 section of
  [hltgp5ow](../investigations/hltgp5ow-fleet-mixes-as-a-game-one-mix-beats-all-others-and-roe-picks-which.md#search-for-counters-t255),
  and `tools/combat-model/nc_metagame_t255.json`.
- **Amends:**
  - [tdkitij6](tdkitij6-heavies-cover-the-destroyers-which-finish-cripples-at-half-price.md):
    cover now applies to battleship and starbase fire, and to the rest of
    each hull's order.
  - [almb7dbt](almb7dbt-a-larger-side-commits-part-of-its-force-and-batteries-punish-a-crowded-orbit.md):
    its dispersed suppression now runs per class.
- **Normative text:** the player manual's fleet combat sections and
  [nc-combat-spec.md](../../dev/nc-combat-spec.md).
- **Code:** `apply_hits_to_fleet` and `hull_fire_focus` in
  `rust/nc-engine/src/maint/combat/exchange.rs`, and `split_by_class` in
  `fleet_battle.rs`.
