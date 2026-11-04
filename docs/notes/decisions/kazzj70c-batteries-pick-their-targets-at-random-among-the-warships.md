---
id: kazzj70c
date: 2026-11-03
kind: adr
status: accepted
tags: [adr, combat, batteries, bombardment, classic]
---
# Batteries pick their targets at random among the warships

## Context

Battery return fire worked the combat line cheapest hull first: destroyers,
then cruisers, then battleships. A cruiser screen therefore took every hit,
and battleships behind it could not be touched. In the x13 lab game, BB5 CA9
bombarding a world with 7 batteries lost CA7 and no battleships.

The order was NC's own, kept from before t206
([tdkitij6](tdkitij6-heavies-cover-the-destroyers-which-finish-cripples-at-half-price.md):
"Batteries keep the old order and prices"). It was not classic. Classic
ECMAINT's batteries pick a random warship, ship by ship, with a 30% chance
of joining the previous battery's target, and keep it until it dies
([ec-battle-spec.md](../../dev/ec-battle-spec.md) §2.4).

## Decision

The owner chose on 2026-11-03 to match classic: "random warship targeting,
by ship count. ships targeted in proportion to their numbers."

1. Battery hits split among DD, CA, BB and SB in proportion to each class's
   ship count, with cumulative rounding in that order.
2. Each share is focus fire on its own class at battery prices: it finishes
   a crippled hull for 1x DS, then destroys healthy hulls for 2x DS.
3. A share too small for another kill cripples one healthy hull of its class
   for 1x DS, as damage builds up on a classic battleship.
4. What the shares cannot spend splits again among the ships left. When no
   share can buy anything, the rest goes down DD, CA, BB, SB.
5. With no warships, battery fire takes scouts, transports and ETACs as
   before. Landing fire on a blitz keeps its order, transports first.

## Consequences

- 518 hits on BB5 CA9 now destroy five cruisers and cripple a battleship,
  where before they destroyed seven cruisers.
- Points lost to battery fire barely change: a cruiser and a battleship cost
  batteries about the same hits per point (4.9 and 4.7).
- Fleets that hide battleships behind a screen take battleship losses. Mixed
  siege fleets lose more of their bombardment strength per year.

## Balance check (t258, 2026-11-03)

Both measured on the Mac mini at 400 trials a case, against the commit
before this rule (d69c33b0, which has t257's bombardment rule).

**The t254 attack-and-defence game**, re-run unchanged (`assault_odds
--grid 400 --attack 400|600|800`, then `assault_game.py --boot 200`):

| Attack points | Capture value | Seen-defence capture | Attacker's BB mass | Attacker's CA mass |
|---|---|---|---|---|
| 400 (1x) | 0 → 0 | 0 → 0 | 0 → 0 | 0 → 0 |
| 600 (1.5x) | 0.245 → 0.224 | 0.335 → 0.297 | 0.37 → 0.21 | 0.32 → 0.49 |
| 800 (2x) | 0.589 → 0.583 | 0.965 → 0.955 | 0.34 → 0.14 | 0.25 → 0.43 |

- Sieges get a little harder: 2 to 4 points of capture chance at 1.5 times
  the defence's points, 1 point at twice. The bootstrap's 95% interval at
  1.5x is 0.220-0.244, so the drop is real but small.
- The best defence is unchanged: about three quarters batteries and a
  quarter armies (15 batteries and 50 armies; the starbase variant at 2x).
- The siege fleet moves from battleships to cruisers. Battleship sieges
  lose the most (best capture against 15 batteries and 50 armies at 1.5x:
  0.287 → 0.263), since a share too small for a kill now cripples one.
  The best mixed siege there falls 0.345 → 0.297.
- Blitzes are untouched (landing fire keeps its order), and so is every
  exchange-table equilibrium: they are all blitzes.

**The battleship siege table** (`siege_odds`, battleships alone):
3 times the batteries' cost still clears every fortress in year one at the
same losses. At 1.5 and 2 times, clearing drops 0 to 13 points, most at
small fortresses (4 batteries, 1.5x: 0.39 → 0.26); at 7 batteries and up,
2x clears 0.39-0.48, down from 0.42-0.54. The manual's "most of the time
with battleships" at twice the cost now reads "about half the time".

So the rule does what the owner asked, with a small cost to sieges and
none to the defence's best mix. Battleships are no longer the best siege
hull; cruisers are. Results in `tools/combat-model/nc_assault_game_t258.json`;
raw grids in the mini's `~/.local/state/nostrian-conquest/balance/t258/`.

## Acceptance and connections

Accepted by the owner on 2026-11-03. Task t258.

- **Amends:** [tdkitij6](tdkitij6-heavies-cover-the-destroyers-which-finish-cripples-at-half-price.md),
  whose battery order this replaces.
- **Normative text:** the player manual's Planetary return fire section and
  [nc-combat-spec.md](../../dev/nc-combat-spec.md) Target Priority.
- **Code:** `apply_battery_hits_to_line` in
  `rust/nc-engine/src/maint/combat/exchange.rs`; test
  `battery_fire_reaches_battleships_in_proportion_to_their_numbers`.
