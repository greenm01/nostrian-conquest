---
id: m1h272uh
date: 2026-10-10
kind: investigation
status: investigating
tags: [investigation, combat]
---
# Joined participation, surprise and allocation fairness

Recommend actual sighting memory for surprise, rather than inferring awareness from co-presence at the year boundary. The cheaper snapshot rule blocks recurring surprise in standoffs, but also removes surprise from forces that stayed hidden. In the controls, that difference ranges from 1.1% for ten cruisers watched by a blockade guard for a full year to 37.6% for one destroyer. These are conditional eligibility rates, not measured campaign ambush rates. Decision 6 remains a proposal.

## t073

Review joined participation, allocation and bounded surprise. No Rust changes or engine runs in this review. The existing participation battle controls remain preliminary; their baseline destroyer DS is 5 and they must be refreshed for approved DS 8 before the final joint recommendation.

## Surprise: snapshot versus sighting memory

The engine resets encounter awareness in `encounter::run_year` each year. Using that transient awareness alone would hand a recurring standoff another chance at surprise. Both proposed alternatives prevent this for previously sighted, continuously present forces.

The snapshot candidate denies surprise if both units occupied the sector and layer at maintenance start. The memory candidate denies it if the defending empire previously sighted that opposing empire there while maintaining local fleet or base presence. The comparison below assumes one stationary Hold target and one observer scanning every tick, no previous awareness, no attacks, no movement and no additional observers. A scan that succeeds ends the unknown state.

For these small fleets, visibility is `(sqrt(30 CA + 15 DD) * layer_factor + sqrt(3 SC)) / 52`, where layer_factor is 5 in orbit and 1 in deep space. Per-tick sighting probability is visibility times observer factor divided by 100. Probability of remaining unseen after n scans is `(1 - p)^n`. Hold, Patrol, Guard/Blockade and orbital starbase factors are 1, 10, 5 and 25. Guard Starbase orders alone do not give factor 5. Target orders here are Hold; active scouting orders can increase target visibility.

| Target / observer / layer | Unseen after 52 scans | Unseen after 4 scans | Unseen after 1 scan |
| --- | ---: | ---: | ---: |
| 10 CA / Hold / deep | 84.07% | 98.67% | 99.67% |
| 10 CA / Patrol / deep | 17.18% | 87.33% | 96.67% |
| 10 CA / Blockade guard / orbit | 1.09% | 70.63% | 91.67% |
| 1 DD / Blockade guard / orbit | 37.63% | 92.76% | 98.14% |
| 1 SC / Blockade guard / orbit | 91.70% | 99.34% | 99.83% |
| 1 SC / Starbase / orbit | 64.74% | 96.71% | 99.17% |

These percentages are where snapshot suppression and memory eligibility disagree on an unsighted holdover at the next boundary. They are not the probability that surprise is eventually used: the defender can sight the target before an attack, intercept may fail, and an unarmed scout cannot attack alone. The controls do not establish how often these situations arise in a campaign. Four scans represent late arrival with four scan opportunities, not a simulated movement trace.

The difference is small for large, long-established orbital forces. It is substantial for raiders, deep-space forces and late arrivals. The snapshot makes two otherwise similar approaches differ because one crosses the annual boundary. I recommend memory because preserving concealment rewards reconnaissance and risk without adding player orders or menus. This recommendation rests on the size and strategic relevance of the controlled cases, not a claim about their campaign prevalence.

There is also a joined-side definition to settle. A per-unit snapshot can let a fresh reinforcement qualify even when the enemy already knows its empire is present. An empire-level snapshot avoids that, but can make a hidden scout at year start suppress surprise for its own later arriving battle fleet. Actual opponent-specific sighting memory avoids both incentives.

## Proposed memory contract and cost

Keep a bitset of sighted opponent empires per viewing empire, sector and layer. Raise a bit on actual sighting or combat. Share it among the viewer's local forces. Keep it while any viewer fleet or base remains there, across year boundaries; clear it when that local presence ends. An opponent leaving and returning does not clear the viewer's memory. This is local readiness, not permanent empire-wide knowledge.

Resolve eligibility before combat marks both sides as aware. Apply +1 only to the first ordinary exchange, within the existing +2 cap. Do not alter natural-9 criticals or refresh surprise for pursuit. This changes no detection or interception roll.

Cost: typed state and SQLite storage for local opponent masks; updates on sightings, movement, elimination and ownership changes; deterministic save/load and boundary tests. Classic export cannot represent it and omits it, with the loss disclosed. Export must not clear SQLite; a fresh classic import starts without that memory. No new menu or player-maintained setting is needed. The owner must approve this cost; it is not implemented.

## Participation contract agreed with Claude, pending owner approval

Only attacking volunteers are filtered at admission, against the fixed defender roster. Each uses its own ROE threshold on assembled friendly versus enemy effective AS. Defenders present take the first exchange; ROE 0 does not confer immunity. After exchange 1, remove all failing fleets on both sides simultaneously, recompute and repeat. Removed fleets never rejoin that encounter. The set shrinks, which proves termination; the ratio need not worsen on every iteration. Unarmed survivors attempt escape automatically. Equal ROE-5 sides can both withdraw.

Name removal of the first-identification ROE bypass as a separate approval item. It changes the approved decision-2 encounter contract: first contact no longer makes a cautious fleet volunteer against superior forces. Attacked fleets still fight.

For allocation, eight eligible CA in one fleet and two in another divide three losses as two and one: exact shares 2.4 and 0.6, then the remaining loss goes to the larger remainder. Apply within the class, damage state and recovery cohort selected by targeting, and preserve completed steps. Loaded and empty transports need separate eligible subgroups so splitting cargo into another fleet cannot bypass loaded-first losses. Stable tie handling must not depend on record order. Full allocation controls remain open.

## Evidence and validation

- [Sighting control script](../../../tools/combat-model/surprise_memory_review.py).
- [Exact results](../../../tools/combat-model/nc_surprise_memory_review.json).
- [Control checks](../../../tools/combat-model/test_surprise_memory_review.py).

Run `python3 tools/combat-model/surprise_memory_review.py` and `python3 -m unittest discover -s tools/combat-model -p test_surprise_memory_review.py`. Four checks cover the orbital formula, scout layer behavior, scan duration and zero prior scans. No Monte Carlo is needed for the unseen-contact probability. No campaign-frequency or Rust-parity claim follows from this calculation.

## Connections

[Decision 2](../decisions/kiswrzos-withdrawal-pursuit-and-one-strength-yardstick.md) supplies ROE and pursuit; first-contact removal requires explicit supersession. [The engagement contract](../decisions/gtkmxw3l-engagement-follows-the-classic-ecmaint-encounter-rule.md) supplies detection and interception. [Recovery timing](../decisions/p01cg7ll-rt-controls-automatic-ship-recovery.md) motivates the cheaper snapshot alternative but does not establish that co-presence means awareness.
