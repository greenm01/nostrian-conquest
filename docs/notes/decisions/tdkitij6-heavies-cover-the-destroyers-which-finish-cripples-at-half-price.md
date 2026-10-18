---
id: tdkitij6
date: 2026-10-18
kind: adr
status: accepted
tags: [adr, combat, hulls, balance]
---
# Heavies cover the destroyers, which finish cripples at half price

## Context

Destroyers are meant to be glass cannons whose execution fire is deadly
inside a mixed fleet and weak alone. Under the rules before t206 the
opposite held. The t189 hull-role trials
([plan](../plans/qkgqypzw-learned-ai-opponent-harness-hierarchical-agent-and-fleet-level-bootstrap-on-the-mac-mini.md#t189),
"Hull roles under today's rules") found pure cruisers beating every hull
mix at equal cost, and the executor's `mixed_hulls` doctrine lost 0.03-0.05
of win share over 60 years. Three causes, confirmed by an independent
Python battle model and by Codex in the engine (10,000 seeds per seat,
ROE 6, 450 points a side):

- Incoming fire landed on the combat line cheapest hull first (DD, CA, BB,
  SB), so the destroyers were the shield rather than the shielded.
- Execution fire removed the same AS per hit as suppression: a healthy kill
  cost `2x DS`, a cripple's finish `1x DS`. The "deadly pair" had no
  multiplier.
- Destroyers have the least DS per point, so a fleet that spends on them
  loses hulls fastest.

A survey of the 4X rulebooks on crunch (Empire of the Sun 9.2.F,
Federation & Empire 308.4 mauler consorts, Space Empires 4X 5.1.4 fleet-size
bonus and 5.7 screening, VBAM 4.6.2) is in the session's
`rulebook-survey.md`; the owner's constraint is that battles are fully
automated, so every rule must be fixed. The consort theme (a destroyer
needing two heavies to fire at full strength) was rejected; EotS's
reduce-before-eliminate gate on execution ("B") was measured and dropped,
because per-hull rechecks let a lone destroyer swarm cripple-then-finish
healthy cruisers at `1.5x DS` (90 DD beat 30 CA 63.6%) and a protected
crippled auxiliary could block execution against the line.

## Decision

The owner approved on 2026-10-18, with Codex's conditional buy-off, three
fixed rules for fleet and starbase fire (ship-only; battery return fire,
landing fire, bombardment and ground combat keep today's rules):

1. **Cover (A).** Ordinary ship and starbase fire works through the combat
   line heaviest hull first: BB, CA, DD, SB. A class with nothing left to
   hit is skipped; a class that stands but cannot be afforded shields the
   classes behind it, and the hits carry. Critical hits keep their own
   order (cheapest hull first, cripples before nominal).
2. **Half-price finishing (D).** Execution fire finishes a crippled ship at
   `ceil(DS / 2)`: DD 4, CA 15, BB 55, SB 65. A healthy ship still costs
   `2x DS`; suppression finishing stays full DS.
3. **Heavy-line bonus.** A side whose cruisers, battleships and starbases
   cost at least twice the enemy's, by build cost with crippled hulls at
   full price, adds +1 to its die, judged again every exchange. Destroyers
   count for neither side, and a side with no heavies never gets it. It
   never makes an unmodified 9 out of a lower roll.

Engine results (Codex, ROE 6, 10,000 seeds per seat, first force's wins):
27CA+9DD and 24CA+18DD beat 30CA 69.8% (36% before); 90DD beats 30CA
33.7% (43.8%); 8BB+18DD beats 10BB 61.7% (35.9%); 5BB+15CA beats 30CA
53.7% (45.4%, the accepted cover buff); 4CA and 3CA+3DD against one
starbase 42.1% / 13.6% (unchanged). The participation model agrees: two
5DD+5CA fleets joined against 12CA win 76.0% (60.9% before).

## Alternatives

- **B, execution finishes first with a cripple fallback.** Arithmetically
  neutral per hit; the literal per-hull form opens the swarm and
  protected-cripple exploits above. Dropped.
- **Consort rules (F&E).** Strong anti-swarm numbers (lone DD 23%) but a
  theme the owner disliked and a second rule on top of cover. Not taken.
- **A hull-count fleet-size bonus (SE4X).** Taxed battleships (10BB vs
  30CA fell from 60% to 49%). The cost-weighted heavy-line bonus keeps
  the anti-swarm effect without it.
- **Cover for battery fire too.** Would cut siege clearance (four
  batteries at 3x cost: 73.9% to 33.2%). Batteries keep the old order and
  prices.

## Consequences

- Mixed fleets with a moderate destroyer share beat pure cruisers at equal
  cost; lone destroyer swarms lose clearly; BB-and-CA lines gain about
  eight points against cruisers from cover alone.
- The 2:1 threshold is discrete: campaigns may show threshold-building or
  concentration incentives. Codex accepted it for testing, not for
  automatic smoothing; a measured exploit pauses adoption.
- The AI executor's `mixed_hulls` doctrine and the tuned weights (t189,
  t191) were fitted under the old rules and need retuning.
- The engine, model and docs landed from branch
  `t206-destroyer-rules` (t206); adoption was gated on a paired executor campaign
  (old and new rules x `mixed_hulls` on and off, 1,024 held-out maps, 60
  and 100 years): the rules merge only if mixed-on beats mixed-off with a
  confidence interval above zero at both horizons. The first gate run
  (2026-10-18) failed: mixed fleets still lose -0.02 to -0.04 at 60 years
  under both rule sets, though under the new rules more destroyers lose
  less rather than more. The results are in the
  [plan](../plans/qkgqypzw-learned-ai-opponent-harness-hierarchical-agent-and-fleet-level-bootstrap-on-the-mac-mini.md#t206);
  the owner merged the rules regardless on 2026-10-18, judging the
  executor's failure to profit from them an AI problem (t189, t191), not
  a game-incentive problem.

## Acceptance and connections

Accepted by the owner on 2026-10-18 in a Claude session on the Mac mini,
with Codex's buy-off in the same session. The
[combat spec](../../dev/nc-combat-spec.md) ("Fleet Battle Effectiveness",
"Carried Hits", "Tactical Roles and Split Fire", "Target Priority") and the
[player manual](../../manuals/nc_player_manual.typ) (Cripple or Destroy,
Rules of Battle, the combat appendix) carry the contract; the combat
model's `engine-t206` preset in
[tools/combat-model](../../../tools/combat-model/README.md) reproduces it.
Earlier: [r4hd41gi](r4hd41gi-mixed-fleets-get-no-combat-bonus.md) removed
the mixed-fleet bonus; this decision replaces nothing in it.
