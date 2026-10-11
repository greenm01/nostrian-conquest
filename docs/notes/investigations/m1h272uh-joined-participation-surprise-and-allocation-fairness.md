---
id: m1h272uh
date: 2026-10-10
kind: investigation
status: investigating
tags: [investigation, combat]
---
# Joined participation, surprise and allocation fairness

Recommend two empires per battle, with same-sector, same-layer fleets joining under their own ROE applied to assembled strength. Let defenders fight the first exchange, then let each fleet leave under the simultaneous, irrevocable withdrawal rule. Divide losses proportionally within the selected class and damage cohort. This rewards coordinated arrivals without making fleet record order decide who dies. Two 5-CA fleets beat 10 CA in 10.0% of sequential controls versus 49.7% joined. Surprise memory is separately [approved](../decisions/atdsvb5y-surprise-uses-local-sighting-memory.md); participation and removal of the first-contact ROE bypass remain proposals.
## t073

Review joined participation, allocation and bounded surprise. No Rust changes or engine runs in this review. The participation controls now use approved DD 10/8/5, flat 0.625 effectiveness, battle-local carry, full DS finishing, current mixed/base modifiers and the unchanged natural-9 critical. Battles use ROE 10 and no catch. Heterogeneous ROE and allocation have separate deterministic controls; they are not integrated into a tick-loop simulation.

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

## Approved memory contract and cost

Keep a bitset of sighted opponent empires per viewing empire, sector and layer. Raise a bit on actual sighting or combat. Share it among the viewer's local forces. Keep it while any viewer fleet or base remains there, across year boundaries; clear it when that local presence ends. An opponent leaving and returning does not clear the viewer's memory. This is local readiness, not permanent empire-wide knowledge.

Resolve eligibility before combat marks both sides as aware. Apply +1 only to the first ordinary exchange, within the existing +2 cap. Do not alter natural-9 criticals or refresh surprise for pursuit. This changes no detection or interception roll.

Cost: typed state and SQLite storage for local opponent masks; updates on sightings, movement, elimination and ownership changes; deterministic save/load and boundary tests. Classic export cannot represent it and omits it, with the loss disclosed. Export must not clear SQLite; a fresh classic import starts without that memory. No new menu or player-maintained setting is needed. The owner approved this cost in [atdsvb5y](../decisions/atdsvb5y-surprise-uses-local-sighting-memory.md); implementation is t079. Only the attacking side earns the bonus.

## Participation contract agreed with Claude, pending owner approval

Only attacking volunteers are filtered at admission, against the fixed defender roster. Each uses its own ROE threshold on assembled friendly versus enemy effective AS. Defenders present take the first exchange; ROE 0 does not confer immunity. After exchange 1, remove all failing fleets on both sides simultaneously, recompute and repeat. Removed fleets never rejoin that encounter. The set shrinks, which proves termination; the ratio need not worsen on every iteration. Unarmed survivors attempt escape automatically. Equal ROE-5 sides can both withdraw.

Name removal of the first-identification ROE bypass as a separate approval item. It changes the approved decision-2 encounter contract: first contact no longer makes a cautious fleet volunteer against superior forces. Attacked fleets still fight.

For allocation, eight eligible CA in one fleet and two in another divide three losses as two and one: exact shares 2.4 and 0.6, then the remaining loss goes to the larger remainder. Apply within the class, damage state and recovery cohort selected by targeting, and preserve completed steps. Loaded and empty transports need separate eligible subgroups so splitting cargo into another fleet cannot bypass loaded-first losses. Break tied remainders by a draw from the battle’s seeded roll stream, not by roster priority. The deterministic controls now verify conservation, seeded tie replay and the loaded/empty split example. They do not establish fleet-splitting invariance for every integer rounding case.

## Joined battle controls, refreshed for DD DS 8

Each row uses 20,000 trials. The sequential control sends one half, then the other against the survivors, retaining completed damage and resetting the hit bank. No healing occurs between contacts. The joined control pools both halves for one battle. Contact legality is imposed. Joining does not combine landing armies, planetary missions or different empires. No surprise bonus is used in these rows.

| Attacking forces against defender | Sequential win | Joined win | Sequential destroyed build points | Joined destroyed build points |
| --- | ---: | ---: | ---: | ---: |
| Two 5 CA vs 10 CA | 10.01% | 49.66% | 144.44 | 93.93 |
| Two 6 CA vs 10 CA | 36.79% | 78.15% | 152.58 | 62.67 |
| Two 4 BB vs 1 SB | 100% sampled | 100% sampled | 11.61 | 5.83 |
| Two (5 DD + 5 CA) vs 12 CA | 22.95% | 70.62% | 182.09 | 88.23 |

Wilson 95% intervals are in the JSON. For the first row, sequential is 9.60–10.43% and joined 48.96–50.35%. For the mixed row they are 22.37–23.54% and 69.98–71.25%. A sampled 100% is not a mathematical guarantee.

The large gain is concentration during combat. The proposal removes the penalty for dividing a co-located force among fleet records. It does not solve the strategic incentive to concentrate. Separate layers, arrival timing, vulnerable worlds and withdrawal still provide reasons to divide a navy. A player should not need to merge records before every battle to obtain combined firepower.

## Surprise battle controls

These quantify the already-approved bonus, not the probability of qualifying for it. Each row uses 20,000 trials per setting, fresh full-strength fleets and no catch.

| Attacker vs defender | No surprise | Attacker surprise |
| --- | ---: | ---: |
| 10 CA vs 10 CA | 48.91% | 54.39% |
| 8 CA vs 10 CA | 16.85% | 19.65% |
| 11 CA vs 10 CA | 64.95% | 71.07% |
| 5 CA + 5 DD vs 7 CA | 40.40% | 44.87% |

A single extra die step increases mean first-exchange effectiveness from 0.625 to 0.700 for an unmodified side: 300 AS produces 187.5 expected hits before the bonus and 210 after. Existing mixed plus base modifiers already at +2 leave no space for surprise. The natural-9 critical remains 10%. Surprise helps a prepared attack without making a smaller navy reliably defeat a larger one.

## Admission, withdrawal and allocation examples

Two 180-AS fleets at ROE 6 fail separate tests against 300 AS, but their assembled 360 AS passes. If one has ROE 0, remove it, recompute 180 versus 300, and remove the remaining ROE-6 volunteer. The defending roster stays fixed during admission; defenders cannot decline being attacked.

After an exchange, two equally strong ROE-5 sides both leave. With friendly fleets of 100 AS at ROE 5 and 100 AS at ROE 10 against 200 AS at ROE 5, both ROE-5 fleets leave in the same iteration. Only the ROE-10 fleet remains. Do not re-add its departed companion after the enemy leaves. Unarmed fleets attempt escape even if armed friends remain. A participating base remains and contributes its effective AS; it does not withdraw.

Allocate losses after each relevant damage step or allocation stage, before the next ROE check. Keep class targeting, nominal/crippled priority and worst-remaining-RT priority. Within an eligible pool, use largest remainder with a uniform seeded random draw for tied remainders at the allocation cutoff. The 8/2 CA example above produces 2/1 losses. If six loaded and four empty transports suffer four losses in the same eligible damage cohort, four loaded hulls die whether carried in one fleet or split into a loaded fleet and an empty fleet. Apply loaded-first across the side before apportioning within that subgroup; otherwise splitting saves two armies in this example.

Choose seeded random tie-breaking over roster order. Fix the roster when the battle starts; where a tied remainder group straddles the remaining allocation slots, choose uniformly without replacement from that group using the battle's seeded roll stream. No draw is needed if the whole group receives a loss. Each allocation is resolved once; never reroll completed losses. This needs no immutable identity surviving renumbering. Replaying the same battle state and seed reproduces it; changing fleet numbering need not preserve each individual result for that seed. Roster position grants no systematic tie priority.

Player wording: "Losses are shared in proportion to each fleet's eligible ships. Ties are broken at random. Splitting or merging fleets can shift a rounding loss by one hull." That last sentence concerns one allocation, not a bound on an entire battle: effects can accumulate or alter a later retreat. A split can also change the number of tied entries. Seeded draws remove deterministic roster priority; they do not prove fleet partitioning cannot influence outcomes. Never recompute earlier damage in a way that revives a completed casualty.

Preserve decision 2's pursuit episode allowance per opposing empire and fleeing fleet; joining cannot create additional pursuit exchanges. Only units physically present in the sector and layer at the trigger can join. Future arrivals that tick do not join retroactively. Third empires never join either side. If admission removes the initiating observer, cancel its trigger. Another willing fleet can initiate only through its own ordinary contact and intercept check. The rejected observer's actual sighting still updates shared surprise memory, but its catch is not inherited. Starbases never volunteer to attack; they join only as defenders under existing target and orbital-gate eligibility, never as offensive reinforcements by co-location. Assembled strength always uses effective NC AS, including crippled strength and each eligible defending base once. These contracts are agreed with Claude, pending the owner's participation approval.

## Engine-test coverage and limits

Read-only review finds 33 tests across `tests/encounter/classic.rs`, `decisions.rs`, `odds.rs` and `aborts.rs`. This is a coverage inventory, not a denominator for an outcome-flip percentage. The awareness-sharing fixture has multiple friendly fleets but no hostile battle. The base-plus-guard fixture already joins that pair. The first-identification test deliberately expects the bypass and must change if the owner approves removing it.

The assault fixture with two 4-BB fleets versus a base supplies the third battle control above: joining changes losses even when all 20,000 controls clear the base. It does not establish the result of running the full engine fixture. The deterministic orbital gate already gathers defending owner fleets, so joined defense is not entirely new. Separate bombardment and invasion mission tests must continue to keep their missions separate.

We cannot report how often joining flips engine encounter outcomes until the candidate participant selection is implemented and compared over identical campaign inputs. Required engine regressions include layer separation, third-empire exclusion, compulsory defense for ROE 0, simultaneous pruning, cargo allocation, record permutation, and no extra pursuit allowance. No Rust tests were run for this design review.

## Joint recommendation for the remaining decision-6 items

1. Join willing fleets of the same empire in the same sector and layer into a two-empire battle. Test each attacker's ROE on assembled effective AS against the fixed defender roster. Defenders fight the first exchange; then both sides prune simultaneously and irrevocably. Unarmed survivors escape. In the controls, two 5-CA forces win 10.0% sequentially versus 49.7% joined; two mixed forces win 23.0% versus 70.6%.
2. Share casualties proportionally within the class, damage/RT cohort and cargo subgroup selected by targeting; break remainder ties with seeded random draws. The 8/2 CA example shares three losses 2/1. Splitting or merging can shift a rounding loss by one hull per allocation, with possible later consequences.
3. A rejected initiator loses its trigger, but its sighting still updates memory. Bases defend only under existing eligibility. Pursuit remains per fleeing fleet and opposing empire. A third empire stays outside that battle. Surprise is already approved separately; no disagreement remains on these recommended participation rules.
4. **Separate approval: first contact no longer bypasses ROE.** A cautious fleet will not volunteer against superior assembled strength merely because it identified the enemy first. It still fights if attacked. This explicitly changes the preserved first-identification clause in decision 2.

## Evidence and validation

- [Participation script](../../../tools/combat-model/participation_review.py), [20,000-trial results](../../../tools/combat-model/nc_participation_review.json), and [participation checks](../../../tools/combat-model/test_participation_review.py).
- Reproduce with `python3 tools/combat-model/participation_review.py --trials 20000`.
- `python3 -m unittest discover -s tools/combat-model -p 'test_*.py'`: 82 tests, one optional dependency skip. Ten participation checks cover DD 8, admission, simultaneous irrevocable exits, unarmed/base behavior, allocation, seeded tie draws and replay, cargo, modifier cap and natural-9 handling.


- [Sighting control script](../../../tools/combat-model/surprise_memory_review.py).
- [Exact results](../../../tools/combat-model/nc_surprise_memory_review.json).
- [Control checks](../../../tools/combat-model/test_surprise_memory_review.py).

Run `python3 tools/combat-model/surprise_memory_review.py` and `python3 -m unittest discover -s tools/combat-model -p test_surprise_memory_review.py`. Four checks cover the orbital formula, scout layer behavior, scan duration and zero prior scans. No Monte Carlo is needed for the unseen-contact probability. No campaign-frequency or Rust-parity claim follows from this calculation.

## Connections

[Decision 2](../decisions/kiswrzos-withdrawal-pursuit-and-one-strength-yardstick.md) supplies ROE and pursuit; first-contact removal requires explicit supersession. [The engagement contract](../decisions/gtkmxw3l-engagement-follows-the-classic-ecmaint-encounter-rule.md) supplies detection and interception. [Recovery timing](../decisions/p01cg7ll-rt-controls-automatic-ship-recovery.md) motivates the cheaper snapshot alternative but does not establish that co-presence means awareness.
