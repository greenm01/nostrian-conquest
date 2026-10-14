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
position across games. Run four-player escort-opening campaigns, 150 seeds and
80 years each, on one binary: four `value` empires (the control), four
`destroyers` empires, one destroyer empire with three `value` empires, and
one with three `cruisers` empires. Compare, per empire, warship points spent
and lost, worlds held at years 20 and 80, captures, eliminations and wins; in
mixed games compare the destroyer seat with its opponents in the same games.

Exit: the four cells with provenance, a summary table with intervals, and a
plain-language conclusion for the manual and balance work. Runs use bean-void,
not crunch.

## Connections

- [Hostile-map price controls](n7fzd2vk-unit-prices-on-generated-maps-with-hostile-openings.md)
- [Destroyer DS 8 decision](../decisions/8pc4t661-destroyers-become-ds-8-for-the-first-playtest.md)
- [Campaign tool](../../../tools/price-campaigns/README.md)
