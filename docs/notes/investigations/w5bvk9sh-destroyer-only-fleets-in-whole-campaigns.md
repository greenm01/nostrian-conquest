---
id: w5bvk9sh
date: 2026-10-14
kind: investigation
tags: [investigation, combat, balance, campaigns]
---
# Destroyer-only fleets in whole campaigns

## Question

What happens when an empire builds nothing but destroyers for the whole game,
because they are cheap? How does its cost and benefit compare with empires that
build cruisers and battleships, and what happens when every empire does it?
Battle controls ([destroyer durability](wnwxz2et-destroyer-durability-under-withdrawal-and-recovery.md))
show equal-cost destroyer fleets losing about 70% of fleet battles, but they do
not price speed, reach or cheap production at small colonies. No campaign run
has tried a destroyer-only player.

## Method

### t146

Add a `destroyers` hull doctrine to the price-campaign runner: every warship
it buys is a destroyer. Garrisons, batteries, starbases, ETACs, invasion
transports and the war policy stay the shared policy, so only the fleet choice
differs. Add a seat-mix option rotated by seed so a doctrine occupies every seat
position across games. Run four-player escort-opening campaigns, 1,000 seeds and
80 years each, on one binary: four `value` empires (the control), four
`destroyers` empires, one destroyer empire with three `value` empires, and
one with three `cruisers` empires. Compare, per empire, warship points spent
and lost, worlds held at years 20 and 80, captures, eliminations and wins; in
mixed games compare the destroyer seat with its opponents in the same games.

Exit: the four cells with provenance, a summary table with intervals, and a
plain-language conclusion for the manual and balance work. Runs use bean-void,
not crunch.

## Findings

There is no incentive to build only destroyers. Against cruiser and battleship
builders, a destroyer-only empire ends the game with about one world fewer than
its opponents, captures fewer worlds, and survives no better. When every empire
builds only destroyers, nobody gains and every empire loses more ships.

Each cell is 1,000 four-player campaigns of 80 years. A game holds 20 worlds,
so an average seat holds 5. Differences are the destroyer seat minus the mean
of its opponents in the same games, with 95% intervals from resampling games.

| Cell | Destroyer seat worlds at 80 | Opponents | Difference | Captures difference | Survival difference |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 destroyer vs 3 balanced (`fixed`) | 4.15 | 5.28 | -1.13 [-1.59, -0.67] | -1.40 [-1.68, -1.13] | -3.0% [-6.9, 0.6] |
| 1 destroyer vs 3 battleship-first (`value`) | 4.14 | 5.29 | -1.15 [-1.58, -0.70] | -1.11 [-1.39, -0.83] | -2.3% [-6.1, 1.4] |
| 1 destroyer vs 3 cruiser builders | 4.53 | 5.16 | -0.63 [-1.08, -0.16] | -0.89 [-1.15, -0.59] | -5.8% [-9.5, -2.1] |
| 2 destroyer vs 2 balanced | 4.64 | 5.36 | -0.72 [-1.20, -0.26] | -1.23 [-1.47, -0.99] | -4.4% [-7.9, -0.6] |

The destroyer seat is already behind at year 20 (by 0.2 to 0.45 worlds) and
falls further behind by year 80. It orders fewer warship points over the game
(about 900 to 1,800 fewer), because it holds fewer producing worlds, while
losing as many points as its opponents. Cheap hulls do not compensate.

When all four empires build only destroyers, the outcome is symmetric, as in
every all-same cell, but each empire loses about 1,200 warship points over the
game against 890 to 990 in the cruiser, balanced and battleship cells.

Limits: the empires are scripted, see the whole map, and attack with one strike
force at ROE 8. They do not use destroyer speed to raid, scout or pick fights,
so these runs do not price that reach. Most games end at the year limit without
a winner (about 1 in 1,000 has one), so win rates are not meaningful; worlds
held, captures and survival are the measures. `built_*` counts are units
ordered, not completed. Destroyer-only empires still receive the two starting
cruisers and the shared escort opening.

## Provenance

Commit `82097436`, built on bean-void with Rust 1.98.0; runner binary SHA-256
`cfaa18e063766401c64bf7adbf5efb1971c3b0924d1de227ca78eedb9b513a52`. Every cell
used `--players 4 --seeds 0..1000 --years 80 --opening escort --goods 50`.
[Summary](../../../tools/price-campaigns/destroyer_doctrine.md) from
`tools/price-campaigns/destroyer_doctrine.py`. Raw TSVs and their SHA256SUMS
are archived outside the repository under the development host's
`~/.local/state/nostrian-conquest/evidence/t146/`. TSV SHA-256 prefixes:
2dd-vs-2fixed `662029ea592f4b92`, all-cruisers `4a1b3644074ef7ab`,
all-destroyers `e5d1097a7412d208`, all-fixed `ffafff1c59cfd787`, all-value
`1145b8e489f6d404`, dd-vs-cruisers `503e0c05247c07ef`, dd-vs-fixed
`53e6fea5632762dd`, dd-vs-value `b4ddf4abde80d9d1`.

## Connections

- [Hostile-map price controls](n7fzd2vk-unit-prices-on-generated-maps-with-hostile-openings.md)
- [Destroyer DS 8 decision](../decisions/8pc4t661-destroyers-become-ds-8-for-the-first-playtest.md)
- [Campaign tool](../../../tools/price-campaigns/README.md)
