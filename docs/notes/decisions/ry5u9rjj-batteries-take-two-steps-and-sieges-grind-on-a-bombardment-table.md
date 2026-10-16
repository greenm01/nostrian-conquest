---
id: ry5u9rjj
date: 2026-10-16
kind: adr
status: accepted
tags: [adr, combat, bombardment, batteries]
---
# Batteries take two steps and sieges grind on a bombardment table

## Context

Ground batteries cost 20, fire 90 and die to 20 hits. Battleships bombard
at 1.5 times their attack (weights 6/33/135). A siege therefore ends in
one exchange: one battleship clears two batteries in a year 95% of the
time, and a sized fleet clears any fortress in a year for about two
battleships. The combat spec asks for the opposite: a small raiding fleet
should need "several turns of three-round suppression before breaking
through" ([lzvpk22s](lzvpk22s-planetary-hits-pay-defense-strength.md)).

Classic EC's batteries hold: a battery is about 0.6 of a battleship in
durability and its equal in firepower, so undersized attacks fail and
fortresses cost about twice their value to take (decoded model,
`tools/classic-model/sim.py`). The owner prefers that challenge on design
grounds, not as a classic balance target (the classic-reference notes record
classic combat results as not a target). VBAM, Starfire, Space Empires 4X,
Galac-Tac, Dark Nebula, Throneworld and Empire of the Sun were reviewed
(2026-10-16): none gives a bombardment bonus by hull class, and multi-turn
sieges come from durable defences, a landing gate, and progress that
persists, not from more rounds.

## Decision

1. **Batteries take two steps.** A battery costs 20 and fires 90 healthy, 45
   crippled. Bombardment hits cripple healthy batteries for 66 each, and
   once every battery is crippled, finish crippled ones for 66 each, in
   the same exchange; hits below the next battery's cost are lost, as
   today. 132 hits in all is 0.6 of a battleship's 220.
2. **No bombard bonus.** Every ship bombards with its attack strength:
   destroyer 12, cruiser 33, battleship 90, half when crippled.
3. **Bombardment table.** Ships' fire at ground targets (bombardment and
   invasion suppression, softening, blitz cover fire) and the batteries'
   fire at descending transports read the planetary ratio table at a third
   of its values (the quarter values read as twelfths). The batteries'
   return fire at ships in orbit reads it in full. Hits round up; columns
   still come from the attack ratio; army ground combat keeps its table.
4. **Fleets bombard together.** When an empire's fleet strikes a world with
   Bombard, every other fleet of that empire in orbit there under Bombard
   and ready to strike joins it as one force, and does not strike again
   that year.
5. **Invade suppresses for a year.** Its suppression takes the year's three
   exchanges, as a bombardment does, striking the batteries only, and the
   landing follows in the same year if they all fall.
6. **A blocked Invade keeps its order.** If batteries survive its year, the
   fleet keeps its armies aboard and its Invade order and suppresses again
   next year.
7. **Captures damage the world by mission**, as classic's capture step does,
   with seeded draws: an invasion keeps 20-40% of production, 10-50% of
   positive goods, a share of the dock, and no batteries; a blitz keeps
   production, goods, dock, and batteries in proportion to
   f = min(1, sqrt(survivors / starting defenders)). This supersedes the
   capture part of [3htuxw8w](3htuxw8w-captured-worlds-convert-in-two-quiet-years-and-keep-what-survives.md).
8. **No repair under siege.** Crippled batteries recover at the end of a
   year in which nobody bombarded the world. Destroyed batteries stay
   destroyed; the owner can always build more.

There is no cap on batteries destroyed per exchange: how many fall depends
on the attacking force against the batteries.

## Alternatives

- **Battery DS 66 in one step:** a fortress fires at full strength until its
  last battery dies, so sieges are all-or-nothing in the first exchange.
- **More rounds a year:** the first exchange decides the year under NC's
  table, so 3, 6 or 10 rounds gave identical results.
- **A cap on batteries destroyed per exchange, fixed or a share:** a fixed
  cap makes a 15-battery world impregnable; a share caps progress by an
  artificial rule rather than by the force sent.
- **Carry-over of sub-kill damage or a yearly column shift:** no effect,
  because no fleet survived a year without clearing the world.
- **Repair every year:** a steady siege never grinds a world down (a 1x
  force reinforced by 1x a year cleared 0-7% within 15 years).
- **Battery return fire at a third, like the ships' fire:** a sized attack
  lost 0.1-0.3x the batteries' cost against classic's 1.5-1.9x; at two
  thirds, bombard-then-invade became nearly certain at a third of classic's
  losses.
- **Landing fire at full strength:** blitz captured 1-26% where classic
  manages 41-67%.
- **Captures keep everything** ([3htuxw8w](3htuxw8w-captured-worlds-convert-in-two-quiet-years-and-keep-what-survives.md)):
  invasion and blitz would differ only in losses, not in what they take.

## Consequences

Engine (`nc-fuzz`'s `siege_odds` and `assault_odds` examples, 400 trials a
case on bean-void, results in `tools/combat-model/nc_siege_engine_t202.json`
and `nc_assault_engine_t202.json`), against the decoded classic assault
(`tools/combat-model/assault_classic_compare.py`). The model
([siege_review.py](../../../tools/combat-model/siege_review.py),
`nc_siege_review.json`) agrees with the engine's siege table within a few
points.

Battleship sieges (force is battleship points over battery points):

| Siege | Before t202 | t202 | Classic |
| --- | --- | --- | --- |
| 3x force, 4-50 batteries | 100% in 1 year, 0.5-0.8x lost | 72-86% in 1.7-1.8 years, 1.4-1.7x lost | 79-97% in 1.1-1.7 years, 1.7-1.9x lost |
| 2x force, 10 batteries | 100% in 1 year | 0%, 2.0x lost | 32%, 1.8x lost |
| 2x force, +0.5x a year, 10 batteries | 100% in 1 year | 2% in 15 years, 8.3x lost | |

Assaults (capture rate, ship points lost, production kept):

| Attack | Classic | Before t202 | t202 |
| --- | --- | --- | --- |
| Invade, 4 batteries, 5 BB + 20 armies | 72%, 133, 46% | 100%, 43, 100% | 77%, 107, 46% |
| Invade, 8 batteries, 10 BB + 40 armies | 66%, 280, 49% | 100%, 108, 100% | 73%, 278, 49% |
| Blitz, 4 batteries, 2 BB + 20 armies | 41%, 30, 86% | 93%, 1, 100% | 33%, 40, 92% |
| Blitz, 8 batteries, 5 BB + 40 armies | 57%, 68, 79% | 94%, 0, 100% | 43%, 67, 90% |
| Blitz, 8 batteries, 10 BB + 40 armies | 80%, 75, 77% | 94%, 0, 100% | 73%, 33, 87% |
| Bombard then invade, 8 batteries, 10 BB | 84%, 287, 33% | 100%, 104, 14% | 77%, 263, 45% |

Before t202 invasion and blitz took almost any world cheaply and kept all of
it. With t202 the whole chain tracks classic: fortresses need about three
times their cost in warships and cost about 1.5 times their value in ships,
an invasion keeps about a third of a world's industry, and a blitz that
lands in strength keeps most of it. Undersized attacks fail and lose their
fleet; a 2x force no longer grinds a world down, because the batteries'
full-strength fire outlasts it.

Work: the engine (`exchange.rs`, `assault.rs`, the encounter strike, the
year's repair), a crippled-battery count on each planet (NC-only, like
crippled hulls; classic export writes the total), battle and siege reports,
tests (planetary, invasion, blitz, collateral, the classic replay pin, the
ONNX fixture), the combat model preset, the player manual and the combat
spec. The AI's tuned weights buy about ten batteries a colony, so AI worlds
become fortresses and the AI needs a retune.

## Acceptance and connections

Accepted by the owner on 2026-10-16 after the engine numbers above, in a
Claude session on the Mac mini (task t202). Amends
[lzvpk22s](lzvpk22s-planetary-hits-pay-defense-strength.md) (battery DS)
and the bombard weights of
[6vqrcldx](6vqrcldx-hulls-follow-a-punch-ladder.md). Evidence: the
[hull balance investigation](../investigations/yqg056tf-nc-hull-balance-against-vbam-and-empire-of-the-sun.md)
and the model above.
