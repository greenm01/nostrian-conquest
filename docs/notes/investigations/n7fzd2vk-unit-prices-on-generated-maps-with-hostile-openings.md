---
id: n7fzd2vk
date: 2026-10-11
kind: investigation
status: complete
tags: [investigation, economy, prices, opening, campaign]
---
# Unit prices on generated maps with hostile openings

Keep the prices and opening unchanged. The 7,020 hostile-map campaigns support useful early expansion at today's ETAC cost, and do not show a reason to cut starting goods. Cheap battleships strengthen the policy that buys them; expensive ones make the value policy abandon them. At today's 45 points, its final territory is similar to a cruiser-focused policy. These are scripted-policy controls, not a solved balance model: only 267 games finish by year 80, the bot sees every world, and most attacks trade colonies rather than threaten homeworlds. Improve the attacker policy before using campaign length to change prices.

## Question

[p6boai3k](../decisions/p6boai3k-unit-prices-and-the-opening-stay-unchanged-for-the-first-playtest.md)
kept the prices (DD 5, CA 15, BB 45, SC 15, TT 5, ETAC 20, starbase 50,
army 2, battery 20) and the joined opening (50 goods, 2 CA, 2 ETAC, 2 DD)
on one condition: once the economy decisions were in the engine (t084),
test the prices on real generated maps with hostile openings before
changing any. Codex's evidence in
[y95fnqkm](y95fnqkm-unit-prices-and-opening-resources-under-the-approved-economy.md)
came from peaceful synthetic maps with no rivals, detection, or battles.
This note (t090) runs whole campaigns in the engine, with every empire at
war from the first year, under today's prices and the alternatives the ADR
names: ETAC 15 and 25, BB 30 and 60, and a joined opening of 35 goods.

## Method

The engine's own AI cannot run such a campaign: the autopilot builds only
armies, batteries, and starbases (`nc-engine/src/maint/economics/ai.rs`),
and the rogue and autopilot orders only send fleets home to guard
(`maint/economics/orders.rs`). Nothing in the engine buys a ship,
colonizes, or attacks. The fuzzer's policies (`nc-fuzz`) choose actions
at random. So the runs use a driver,
[`rust/nc-harness/examples/price_campaigns.rs`](../../../rust/nc-harness/examples/price_campaigns.rs),
that plays every empire with one scripted policy through the real
submission path (`TurnSubmission::apply_to`) and the real maintenance
turn (`ScenarioPlay::run_maintenance`) on maps from the real generator
(`build_seeded_initialized_game`: five systems per empire, 18x18 at two
and four empires, 27x27 at six). The opening is the joined one: each
homeworld at production and potential 100 with 50 goods at 50% tax, 10
armies, 4 batteries, two cruiser-escorted ETACs, and two lone destroyers.
Every empire declares every other an enemy before the first year.

The policy, the same for every empire, is in the driver's header. In
short: an opening purchase (`bare`: two extra ETACs; `escort`: two extra
cruiser-escorted ETACs; `base`: a starbase; `pressure`: a battleship);
ETACs colonize the nearest unclaimed raw world and are replaced while raw
worlds remain; each world keeps a garrison floor, buys the transports and
armies its next assault needs, buys a starbase at home once two colonies
exist, and spends the rest on hulls. The home base also becomes a priority from
year 6, and from year 10 the bot prepares troops for the nearest enemy
homeworld. Two hull doctrines: `value` buys the
affordable hull with the most attack plus defense per point (BB 190/45 =
4.22, CA 60/15 = 4.00, DD 18/5 = 3.60, so BB when 45 is free, else CA,
else DD); `fixed` buys BB, CA, CA, DD, DD in turn and waits for the next
one. Everything built at home joins one home fleet on Guard/Blockade; the
two starting destroyers guard home. The empire wears down one victim (the
enemy with the least navy, batteries, and bases, weighted by distance),
choosing each year the victim's world worth most for what taking it needs,
and detaches a strike force sized to 150% of that world's defense, never
more than 60% of the home fleet, with twice the defending armies plus one
aboard, to invade at ROE 8. The policy sees the whole map; it is not an
opponent model, only a consistent way to turn income into expansion,
defense, and attacks under each price.

Prices are compiled constants (`ProductionItemKind::build_cost` and
`BUILD_UNITS`), so each alternative is a separate build of the same
driver from a throwaway worktree with the two constants changed and
nothing else; every output row repeats the prices its binary was built
with. The 35-goods opening is the base build with `--goods 35`, which
sets each homeworld's stored goods before the first year.

The matrix, per price build (base, ETAC 15, ETAC 25, BB 30, BB 60) and
for base with 35 goods: symmetric games (every empire the same opening)
at 2, 4, and 6 empires, openings `bare` and `escort`, doctrines `value`
and `fixed`, seeds 0 to 59, 80 years; and mixed games at 4 empires, the
four openings rotated over the seats by seed, both doctrines, seeds 0 to
149, 80 years. A third control pits two `value` seats against two `cruisers` seats, which
buy cruisers or destroyers but never battleships, at four players with an
escorted opening, seeds 0 to 149. Seats alternate by seed. Each label has
1,170 campaigns; the full matrix has 7,020 campaigns and 28,080 empire
histories. All 90 output files have complete game and seat rows. Games end
at 80 years or when the engine declares a winner; the earliest winner is
year 31, so every game reaches the 5/10/20-year checkpoints.

What is measured, per empire: the year of its first colony and the
colonies founded by years 5, 10, and 20; worlds held at years 10 and 20
and at the end; units ordered by class; the year of its first battleship
order and of its first starbase at home; years its homeworld could place
no order at all (`exhausted`) and, under `fixed`, years it waited for the
next hull (`wait`); assaults, captures, ship losses by class, the year it
lost its homeworld, elimination, survival, and victory. Per game:
battles, assaults resolved, assaults on homeworlds, failed assaults,
captures, eliminations, and the winner's year.

## Results

The complete tables and compact numeric results are in
[results.md](../../../tools/price-campaigns/results.md) and
[results.json](../../../tools/price-campaigns/results.json).
[provenance.json](../../../tools/price-campaigns/provenance.json) records
source, binary and raw-output hashes; no built binaries are copied into
the evidence.

Four-player escorted openings, value doctrine, 60 campaigns per row:

| Price/opening | First colony | Colonies by year 5 per empire | First BB order | First home base | Captures per game | Battles per game |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Current | 2.00 | 3.68 | 5.38 | 5.43 | 55.37 | 46.97 |
| ETAC 15 | 2.00 | 3.68 | 5.24 | 5.29 | 55.45 | 44.40 |
| ETAC 25 | 2.00 | 3.68 | 5.87 | 5.88 | 56.55 | 46.68 |
| BB 30 | 2.00 | 3.68 | 5.36 | 5.43 | 54.95 | 51.13 |
| BB 60 | 2.00 | 3.68 | Never | 5.43 | 55.42 | 44.63 |
| Starting goods 35 | 2.00 | 3.69 | 6.05 | 6.06 | 54.70 | 45.70 |

First BB means an order, not a completed or deployed hull. The first
colony date is set largely by the two starting ETAC fleets and movement.
ETAC prices affect the resources left for subsequent purchases, not those
starting fleets. At BB 60, the fixed doctrine still orders a battleship,
with a mean first order of 6.35 instead of 5.38. Thus "no battleships" is
a purchasing-policy result, not a rule or affordability barrier. In these
rows the mean number of years with no affordable homeworld order is zero;
that does not mean every desired order is affordable or queues never wait.

Mixed openings at current prices and the value doctrine, 150 campaigns,
one seat per opening in each game:

| Opening purchase | Colonies by year 10 | Worlds at game end | Lost homeworld | Eliminated |
| --- | ---: | ---: | ---: | ---: |
| Two bare ETACs | 4.94 | 5.60 | 37.3% | 33.3% |
| Two escorted pairs as affordable | 4.19 | 4.83 | 41.3% | 36.7% |
| Starbase first | 3.34 | 4.51 | 36.7% | 34.0% |
| Battleship first | 3.53 | 5.06 | 32.7% | 29.3% |

Bare expansion captures more empty worlds; pressure trades some of that
reach for fewer lost homeworlds in this control. No opening wins any of
these 150 games outright by year 80. These rates do not establish that
bare ETACs are always best or that escorts are worthless. The scripted
opponents have no early hunter policy, and occupied worlds are chosen
with perfect information.

The mixed-doctrine control separates a hull-price change from a rule that
forces everyone to buy the same ships. Both doctrines share the escorted
opening and all other policy decisions. Each row has 150 paired maps:

| BB price | Final worlds per value seat | Final worlds per cruiser seat | Paired difference, 95% map-bootstrap interval |
| --- | ---: | ---: | --- |
| 45 | 5.14 | 4.86 | +0.28 [-0.92, +1.50] |
| 30 | 6.86 | 3.14 | +3.71 [+2.61, +4.79] |
| 60 | 4.87 | 5.13 | -0.25 [-1.51, +1.03] |

At 60, both policies buy cruisers or destroyers; their small difference
comes from seating and campaign randomness, not a battleship role. At 30,
the value policy gains substantially more territory. At 45, the interval
includes either policy leading. Resampling treats a whole campaign as the
sample, not each of its four seats as an independent trial. The interval
is conditional on this policy and map-seed range; it is not a confidence
interval for human strategy. See
[doctrine_controls.json](../../../tools/price-campaigns/doctrine_controls.json).

The campaign finish rate is a warning about the model's reach. Only
267/7,020 games finish by year 80 (3.8%). At current prices, 25/1,170 finish:
20/240 two-player games, 5/690 four-player games and 0/240 six-player games.
The current-price mean is 2.93 assaults on homeworlds over an entire game,
while many colonies change hands repeatedly. The bot protects its home
fleet and favors cheap targets, so these runs cannot decide whether human
campaigns drag or whether defense is too strong.

## What the runs cannot show

- The policy, not a player, decides what to buy. The `value` doctrine
  ranks hulls by attack plus defense per point, so BB 30 makes it buy
  battleships whenever 30 is free and BB 60 makes it never buy one; the
  `fixed` doctrine buys the same sequence at every price, so there a
  price moves only dates and waits. The build mix by class is therefore
  the doctrine's, and a price's effect on it is the doctrine's response.
  The two doctrines bracket the question; a player sits somewhere between.
- The opening purchases are fixed by the opening policy: ETAC demand is
  the opening's two extra colonizers plus replacements while raw worlds
  remain, and raw worlds run out by about year 6 at 4 empires. An ETAC
  price therefore changes what the opening leaves over, not how many
  colonizers a human would buy.
- The policy sees the whole map, so it sizes strikes against the exact
  defense and never scouts; the value of scouts and surprise is not
  measured, and assaults succeed more often than a player's would.
- Both the home fleet and the strike rule are one design: everything
  guards home, strikes take at most 60% of it. Campaign length and
  eliminations follow from that as much as from any price.
- Tax is 50% throughout; no empire manages its rate.
- Map generation varies by seed. `ScenarioPlay` passes campaign seed 0
  to maintenance for every game. Encounter context still varies, but these
  are not independent combat-seed repetitions of each map. The intervals
  resample maps and seat rotations only.
- The matrix predates t099's rogue conversion and AI-merge corrections.
  The tested human policies are separate from autopilot, but post-defeat
  rogue behavior can affect survivors. This is a baseline at the recorded
  revision, not a claim about all later engine changes.
- The maps are the generator's 18x18 and 27x27 layouts at 2 to 6
  empires; larger games were not run.

## Recommendation

Keep ETAC 20, BB 45, the other prices, and starting goods 50. The ETAC
alternatives move later capital purchases more than first settlement;
35 starting goods delays home bases and capital orders by about two-thirds
of a year in the escorted control without a demonstrated fairness benefit.
BB 30 gives the battleship-buying policy a clear territorial advantage;
BB 60 removes battleships from the value policy without proving a better
role split. No control here establishes a pricing defect requiring change.

Do not make a balancing change to cure this bot's long wars. The next
useful control is an attacker that concentrates on a homeworld and a raider
that hunts early colonizers, with fresh combat seeds, limited intelligence
and the latest engine. Compare those against the present conservative
policy before treating these finish rates as a player-facing pacing
problem. Prices remain an owner decision; no engine price or opening was
changed.

## Reproduction

The runs use engine/data revision `7348955f`, with driver commit
`0c1c8f6f` plus the driver changes committed with this note. The exact
driver hash is in provenance.json. All five price builds have the same
driver source hash. Replaying on newer engine revisions is a new control.
From `rust/`:

```sh
cargo-slot nostrian-conquest cargo run --release -p nc-harness --example price_campaigns -- --label base --players 4 --seeds 0..60 --years 80 --opening escort --doctrine value --goods 50 > /tmp/base_4p_escort_value.tsv
cargo-slot nostrian-conquest cargo run --release -p nc-harness --example price_campaigns -- --label base --players 4 --seeds 0..150 --years 80 --opening mixed --doctrine value --goods 50 > /tmp/base_4p_mixed_value.tsv
cd ..
python3 tools/price-campaigns/summarize.py /tmp/base_4p_*.tsv
```

For an alternative price, change the two constants (`Self::Etac =>
Some(20)` in `nc-data/src/model/planet.rs` and the ETAC entry's `cost:
20` in `nc-engine/src/orders/build.rs`, or the battleship's 45) in a
throwaway worktree, build the example there, and run it with another
`--label`. The driver prints the prices it was built with on its first
line and in every row. The complete matrix ran in about three minutes on 30 threads after builds.
The price-campaigns README documents the matrix and regeneration commands.
The parser rejects truncated rows, duplicate campaigns and missing seats;
its four integrity tests pass.

## Connections

- [p6boai3k](../decisions/p6boai3k-unit-prices-and-the-opening-stay-unchanged-for-the-first-playtest.md):
  the decision this note re-tests; its "Result (t090)" line points here.
- [y95fnqkm](y95fnqkm-unit-prices-and-opening-resources-under-the-approved-economy.md):
  the peaceful synthetic controls this note follows up.
- [ubnjcchu](../decisions/ubnjcchu-fractional-economy-accounting-smooth-growth-and-a-colony-head-start.md),
  [iq6sfxde](../decisions/iq6sfxde-starbases-keep-their-growth-and-capacity-bonuses-without-stacking.md),
  [3htuxw8w](../decisions/3htuxw8w-captured-worlds-convert-in-two-quiet-years-and-keep-what-survives.md),
  [nrvey7jk](../decisions/nrvey7jk-lighter-bombardment-of-goods-and-classic-scorch-earth.md):
  the economy rules in force during the runs.
- [nc-combat-spec.md](../../dev/nc-combat-spec.md): the combat rules in
  force during the runs.
- [lenngda1](lenngda1-training-an-ai-opponent-for-nostrian-conquest.md):
  the driver's policy is a scripted bot of the kind that note starts from.
