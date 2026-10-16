---
id: r4hd41gi
date: 2026-10-16
kind: adr
status: accepted
tags: [adr, combat, hulls, balance]
---
# Mixed fleets get no combat bonus

## Context

A fleet with at least two of destroyers, cruisers, and battleships adds one
to its die in fleet battles, and a mixed fleet shifts the column by one in
bombardment and invasion suppression
([nc-combat-spec.md](../../dev/nc-combat-spec.md), "Die modifiers" and the
assault section; `exchange.rs` and `assault.rs` in `nc-engine`). Neither
source game has a composition bonus: VBAM separates hulls by command rating
and special abilities, and Empire of the Sun rewards a balanced force through
escort and hit-application rules.

The [hull balance investigation](../investigations/yqg056tf-nc-hull-balance-against-vbam-and-empire-of-the-sun.md)
found that at the default ROE 6 each hull already has a role of its own:
battleships hold the field and bombard at 1.5 times a cruiser's rate per
point, cruisers are the faster line, and destroyers destroy 1.4-1.7 times
the build points they lose. Without the bonus, destroyers keep that
attrition role (1.67), and mixed fleets lose about 0.02-0.05 field-win rate.
The [independent balance analysis](../investigations/rolr9ti0-independent-nc-combat-balance-analysis.md)
also recommended removing it.

## Decision

The owner decided on 2026-10-16 to remove the mixed-fleet bonus everywhere:
the fleet-battle die modifier and the bombardment and invasion suppression
column shift. Hulls must earn a place in a fleet through speed, durability,
fire type, and bombardment, not a composition rule. Ship stats stay as they
are (DD 10/8/5, CA 30/30/15, BB 90/100/45).

The owner's standard: remove the bonus unless it proves worthwhile. The
evidence did not show it was. Revisit only if the engine check of the ROE 6
results or the campaign trials show that mixed fleets lose their purpose
without it.

Implementation waits until the Mac mini's current t189 campaign runs finish,
so their comparisons stay on one rule set (t198).

## Alternatives

- **Keep the bonus.** It is the only thing that makes a destroyer-and-
  cruiser mix beat a pure cruiser line at ROE 10, but it hides the real
  trade-off between holding the field and making losses stick behind a flat
  modifier, and it has no precedent in either source.
- **Drop it in fleet battles only.** The battle evidence covers fleet
  battles, not bombardment. The owner chose one rule for every theater:
  battleships already carry bombardment through their 1.5x multiplier.

## Consequences

- Mixed fleets lose a small amount of field-win rate; pure battleship
  fleets become the strongest field holders at equal cost.
- Starbase and surprise modifiers are unaffected.
- Siege assaults by mixed escorts lose one column; the siege-doctrine
  campaign test in the investigation should run under the new rule.
- Open work (t198): remove the modifier and the column shift in the engine,
  update the engine tests and the combat model's `engine-t065` preset, and
  update the combat spec and the player manual (die modifiers, Cripple or
  Destroy, Late Game, the appendix tables), with a new revision date.

## Acceptance and connections

Accepted by the owner on 2026-10-16 in a Claude session on crunch, after
review of the [hull balance investigation](../investigations/yqg056tf-nc-hull-balance-against-vbam-and-empire-of-the-sun.md).
The [combat spec](../../dev/nc-combat-spec.md) and the
[player manual](../../manuals/nc_player_manual.typ) carry the contract once
t198 lands.
