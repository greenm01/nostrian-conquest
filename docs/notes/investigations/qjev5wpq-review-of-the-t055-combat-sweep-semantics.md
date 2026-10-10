---
id: qjev5wpq
date: 2026-10-10
kind: investigation
status: investigating
tags: [investigation, combat, balance, simulation]
---
# Review of the t055 combat sweep semantics

The sweep strengthens the case for flat effectiveness and usable withdrawal. It does not select a final rule set. Fix the exact solver's round limit before publishing its tables: its compressed round state can answer a different question from the sampler. Treat carry as temporary hit-budget accumulation, not persistent ship damage. Separate the chance of pursuit from whether a screen fails, and give unarmed survivors an explicit disengagement rule. Keep residual damage as a comparison, not the default. Judge combat by surviving transports, damaged ships, repair bills, and escape opportunities as well as who wins the field.

This is a review of t055, following the [independent balance analysis](rolr9ti0-independent-nc-combat-balance-analysis.md). It reads the Python transitions, solver, tests, presets, generated tables, Rust comparison driver, and engine withdrawal rules. The [balance workflow](../../dev/combat-balance-workflow.md) correctly distinguishes model experiments, engine comparisons, and campaign evidence. Keep those distinctions in every conclusion. No engine rule changes are authorized by this note.

The reproducible probes are in [review_t055.py](../../../tools/combat-model/review_t055.py). The [retained results](../../../tools/combat-model/nc_t055_review_before_fix.json) record the source hashes and pre-fix output. The broader comparisons come from the [t055 tables](../../../tools/combat-model/nc_sweep_results.md); their assumptions are in the [tool README](../../../tools/combat-model/README.md). Claude confirmed the solver defect and assigned t055b to fix it and regenerate affected tables. The other findings below remain review findings and candidates, not adopted policy. This review does not claim to validate that fix or the Rust suite.

**The exact solver loses the round limit.**

The reviewed `solve()` caches flat-rule states with round 1, and current-rule states with `min(round, 4)`. It then passes that compressed number to the shared transition function. The sampler passes the actual round. Compression preserves the post-lock rules, but loses the remaining time before the cap.

An independent finite-horizon traversal, using the same exchange and allocation functions but the actual round in its cache key, gives these results:

| Probe | Reviewed solver | Actual finite horizon |
| --- | --- | --- |
| Flat carry, 1 CA against 1 CA, cap 1 | 44.46% win each; 11.09% mutual destruction; 3.56 exchanges | 100% stalemate after one exchange |
| Flat carry, CA AS 2 / DS 100, effectiveness always 1, cap 64 | 100% mutual destruction after 150 exchanges | 100% stalemate after 64 exchanges |
| Flat discard, 1 CA against 1 CA, cap 64 | Mean 19.4375 exchanges | Mean 19.046875 exchanges |

The slow probe uses legal tunable inputs. Each cruiser takes 50 rounds to become crippled, then another 100 to die because crippled AS halves. It exposes a real horizon error, not sampling noise. The discard probe exposes a second symptom: an absorbing stalemate receives a fresh full cap after the exchanges already spent reaching it.

Cache the remaining rounds, or expose clearly separate unlimited and capped solvers. A self-loop shortcut must account for the finite time remaining. Retain all three regressions. Compare exact enumeration and sampling under deliberately short caps as well as the default cap. Short ordinary fights can still have correct rounded win rates in the old tables; the defect does not make every reported outcome wrong.

Run the review without overwriting its historical evidence:

```sh
python3 -B tools/combat-model/review_t055.py
```

The checker independently tests time accounting. It shares damage allocation with the model, so agreement cannot establish that allocation matches Rust.

**The engine comparison has useful, limited coverage.**

The current-rule result for 11 CA against 10 CA is strong evidence: 85.0% in the model and 85.3%, with a reported 95% interval of 84.8–85.8%, in the engine. The other selected matchups also agree on left-side victory under the comparison's test. This supports using the model to investigate changes.

The automatic `agree` flag checks left-side victory only. For exact cases it uses a binomial test; for sampled cases it checks interval overlap. Mutual destruction and mean losses appear in the table but do not determine that flag. Missing cases and zero-battle rows are skipped. The Rust cases cover DD, CA, and BB fleets. They do not establish auxiliary, loaded-transport, starbase, persistent-damage, or round-cap agreement. The driver also lacks per-exchange and cripple-state comparisons.

Failure to reject a difference, or overlap between confidence intervals, does not prove equivalence. State the checked outcomes and scenarios. Report missing cases explicitly. Before adopting a candidate, compare losses by class, mutual destruction, withdrawal, and the cap outcome as well as victory. Use a few deterministic transitions for screening and targeting boundaries.

The current 10 CA at ROE 6 against 15 CA case usually ends before the lock expires. It barely tests retreat. The Python test named `test_current_roe_lock_holds_three_rounds` compares the same withdrawal-plus-stalemate expression with itself. Its other assertion only checks a mean duration. Replace that assertion with a controlled case that survives three rounds and exercises the next withdrawal decision.

**Carry needs a precise name and lifetime.**

The implemented carry variant banks unused suppression and execution hits on the receiving side. Its state is two budgets per side, not damage attached to a particular class or hull step. Future allocation spends the budget in the next target order. Ending the battle does not produce persistent partial damage for another encounter.

The probe makes the distinction concrete. A fresh BB receiving 99 suppression hits stays at AS 90 and banks 99. One more hit in that battle cripples it. Resetting the battle before that extra hit leaves a fresh BB with one banked hit. Full AS until a damage threshold is a reasonable discrete rule. Losing all progress on disengagement is a separate campaign decision.

For a simple threshold of cost C and incoming budget H, discard spends `floor(H / C)` steps and loses the remainder. A residual roll adds the next step with probability `(H mod C) / C`. Carry retains that remainder for later allocation. These choices can have similar expected spending against an unlimited sequence of identical targets, yet produce very different survival and retreat incentives against finite forces with changing AS.

Keep all three comparisons. Do not call this temporary pool implementation the completed persistent-damage candidate. Before testing that candidate, specify who owns each remainder, which class and step it damages, and what split, merge, retreat, destruction, and repair do to it. Otherwise repeated short attacks can be ineffective while one long fight with the same damage succeeds. Conversely, transferring old damage freely between classes can create its own targeting exploit.

These are internal state rules. The player can still see a simple count of sound and damaged ships, a clear repair bill, and a useful battle report. Simplicity of controls does not justify unexplained damage resets.

**The remainder choice changes ship roles.**

These are rounded left-side win rates from the pre-fix t055 tables. Regenerate them after the solver fix; they are evidence for what to examine, not adopted balance targets.

| Equal-cost or role matchup | Current | Flat discard | Flat residual | Flat carry |
| --- | --- | --- | --- | --- |
| 3 CA against 1 BB | 16.6% | 0% | 43.8% | 20.0% |
| 3 DD against 1 CA | 2.0% | 0% | 23.0% | 0.3% |
| 10 DD against 1 SB | 1.0% | 0% | 15.2% | 0% |

Discard creates hard thresholds. Three CA have at most 90 suppression hits under the unshifted flat table, below a BB's 100-point damage step. Ten DD have at most 100 execution hits, below the 240-point cost of destroying a sound base. Budget-limited critical focus cannot manufacture the missing hits. These are immunities within those configurations, not merely low odds.

A displayed zero win rate is not itself proof of immunity. In the carry base case the base can die in mutual destruction: about 0.03584% in the retained pre-fix JSON. A win table hides that distinction. Include damage inflicted and mutual destruction when judging a sacrificial attack.

Residual rolls eliminate many thresholds by introducing additional lotteries. The change from 0.3% to 23% for three DD against a cruiser is substantial. That may create an appealing raider role, but it also changes how safely players can price a capital investment. It warrants deliberate selection, not adoption as a rounding fix.

The DD DS 8 experiment is worth retaining. At cost 5, 30 such DD win about 27.8% against 10 CA under carry, versus 7.1% at DS 5. That helps destroyers without demonstrating destroyer dominance. Test escorts and raids too. Equal-cost fleet duels cannot establish the value of speed or transport protection.

**Pursuit and screen failure are separate decisions.**

The flat presets combine certain pursuit with auxiliary-first targeting during pursuit. The screen switch does not test whether escorts physically failed to protect transports. The pursuit switch does not test speed, interception, position, or whether the permitted pursuit exchange was already spent. In the standalone fight, both sides exchange fire when pursuit occurs.

I separated the existing switches in a convoy probe: 5 CA and 10 empty TT at ROE 6 against 10 CA at ROE 10, using flat carry and baseline units. Each row has 10,000 trials with seed `20261010 + trial_index`. The same trial seeds are reused between configurations. Standard errors for the transport-loss means are about 0.045–0.049 ships.

| Pursuit | Screen during pursuit | Mean TT destroyed | Mean CA destroyed | Withdrawal recorded |
| --- | --- | --- | --- | --- |
| None | Absolute | 3.996 | 1.998 | 60.04% |
| None | Conditional | 3.996 | 1.998 | 60.04% |
| Certain | Absolute | 4.330 | 3.815 | 60.04% |
| Certain | Auxiliary first | 6.035 | 2.406 | 60.04% |

Here the withdrawal mode named `none` means no withdrawal exchange, not an inability to withdraw. The identical no-pursuit rows confirm that this conditional screen only changes the pursuit exchange.

Auxiliary-first targeting destroys more transports while preserving more cruisers. At CA cost 15 and TT cost 5, the mean destroyed-hull bill falls from about 78.87 to 66.26 build points. A metric based only on that bill can therefore label a worse convoy outcome as cheaper combat. Loaded armies, the missed landing, and the value of capturing the destination make the distinction larger.

Bracket no pursuit and a successful pursuit exchange in fleet experiments. Derive the chance of reaching that exchange from speed and encounter state in campaign tests. Vary auxiliary exposure independently. Preserve the proposed per-opposing-empire pursuit allowance and persistent withdrawal episode when testing repeated encounters. Fleet splitting must not buy extra pursuit attacks.

Report the distribution of transports arriving and the chance of retaining a viable landing force. Players should be able to take an informed invasion risk. A seemingly adequate escort that automatically offers its transports as the first targets on retreat undermines that choice unless the rule is explicit and the player can plan around it.

**Zero AS currently prevents a withdrawal request.**

Both the model and engine gate failed ROE on positive AS. A force whose last armed ship dies can therefore stop requesting withdrawal even though transports remain. This is current behavior to reproduce, not a reason to carry it into the preferred candidate.

It explains the convoy probe's large loss floor even with no pursuit. The flat table gives full effectiveness on four of ten rolls. Ten sound CA then produce 300 suppression hits, enough to destroy all five escort CA through their two full-DS steps. The remaining transports have zero AS. They do not request withdrawal and eventually die. That event alone accounts for approximately four lost transports per trial on average. The observed 3.996 is consistent with it.

Choose an explicit rule for unarmed survivors. I favor allowing them to attempt escape under the same movement and pursuit constraints as other survivors. Losing the guns should not disable the escape order. A faster scout facing a battleship should not be denied withdrawal merely because it cannot fire back. This is a proposed rule change, not part of the mechanical retreat-routing fix.

**What the results support next.**

Flat effectiveness still rewards concentration. In the reported carry samples, 11 CA against 10 win about 67%, compared with current 85%; 15 against 10 win about 97%. This removes much of the extra ratio-column amplification while retaining a strong advantage for preparation. It does not engineer equal odds for unequal forces.

The 0.625-mean effectiveness table gives an expected raw budget near `0.625 × AS` before rounding, pool division, and target thresholds. Ten equal cruisers per side take about 4.79 exchanges under the reported carry sample. That is much more decisive than the earlier 14-round experiment with a 0.30 mean and two full-DS damage steps.

This does not settle the VBAM comparison. The reviewed presets still charge full DS for the second damage step. The requested half-DS-after-cripple variant is not implemented merely because a VBAM effectiveness table exists in `rules.py`. Test the damage rate and durability together. Also state that the model's base bonus assumes an eligible location: its `undamaged` check does not represent owned-world and orbital-layer eligibility.

Winning remains costly if damage lasts. The carry sample for 15 CA against 10 loses about 31.9 build points and leaves 112.2 build points of crippled hulls. At the proposed 25% repair price, replacement plus repair is about `31.9 + 0.25 × 112.2 = 60` points. That exceeds one mature homeworld's 50-point annual income in the [economy investigation](tkce84f0-a-simple-nc-economy-informed-by-classic-ec-vbam-and-ec4x.md). This is an unconditional sample mean and an illustrative candidate repair rule, not a guaranteed winner's bill. Travel to repair and several encounters in one 52-tick year can make the operational cost larger.

My order of work is:

1. Fix the finite horizon and regenerate affected evidence. Label exactly what the engine comparison checks.
2. Separate pursuit success, auxiliary exposure, and unarmed disengagement. Measure mission survival alongside losses.
3. Define persistent damage before treating temporary carry as evidence about campaigns. Test repeated attacks, split and merge, and repair.
4. Add the coherent VBAM damage-step variant and retain the DD DS 8 experiment. Judge complete rule combinations, not isolated averages.
5. Feed chosen candidates into travel, invasion, replacement, and repair scenarios. Publish small odds examples and battle reports that explain the decisive events.

The [classic-reference review](o7w8k8uh-what-the-completed-classic-combat-reference-changes-for-nc.md) and [combat-system decision](../decisions/b5u6lekb-rust-keeps-its-own-combat-system.md) still set the frame: use classic for operational facts and diagnostics; build NC combat from the selected EotS and VBAM ideas. The player should understand the stakes without managing internal accounting. These probes ran in Python; no cargo command or Rust validation was run for this review.
