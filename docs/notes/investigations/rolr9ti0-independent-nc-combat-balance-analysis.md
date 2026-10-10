---
id: rolr9ti0
date: 2026-10-10
kind: investigation
status: investigating
tags: [investigation, combat, balance, simulation]
---
# Independent NC combat balance analysis

NC should reward concentration, preparation, fast raids, and calculated invasions. It should also let players understand why those plans succeeded or failed. Keep dangerous batteries, distinct ship roles, and decisive battles. Replace the ratio-column damage multiplier and three-round retreat lock. Preserve damage between encounters. Join nearby friendly forces under explicit participation rules, allow conditional attacks on fleeing auxiliaries, and use a bounded surprise bonus. Keep destroyer and starbase execution fire; the numbers do not support the claim that destroyers dominate. Test DD 10/8 at cost 5 as the first hull adjustment. Before broad balancing, fix the mismatch between strike eligibility and actual resolution, the incomplete orbital gate, and the ground-combat threshold that turns one extra army into a 75-percentage-point increase in capture odds. Publish one clear rule set and make battle reports explain it.

**Scope and standard of judgment**

This is an independent design review of the supplied synthesis, its three source reviews, the current manual and combat spec, and the Rust combat, encounter, and movement code. The code findings are from inspection. I did not run cargo or modify the engine. Python experiments test specified combat arithmetic, not the full Rust maintenance process. The checkout inspected was `b8502a0af07af6464a5106d48edd8cf008f84b87`.

The principal NC references are the [combat spec](../../dev/nc-combat-spec.md), [player manual](../../manuals/nc_player_manual.typ), [exchange arithmetic](../../../rust/nc-engine/src/maint/combat/exchange.rs), [combat state](../../../rust/nc-engine/src/maint/combat/state.rs), and [fleet battle resolver](../../../rust/nc-engine/src/maint/combat/fleet_battle.rs). The [ec4x review](../../dev/combat-review/ec4x.md) supplies the third source comparison alongside the EotS and VBAM reviews discussed below.

The player's experience matters more than fidelity to a borrowed rule. A player submits orders before the year's battles. They cannot watch round two and click Retreat. ROE, route, fleet composition, and mission therefore carry much of the agency that a tabletop game supplies through live decisions. Those controls must work when they matter.

Fairness does not require equal odds or gentle losses. A prepared defender should punish an unsupported landing. A commander who concentrates force should usually win. A fast raider should sometimes profit without holding the battlefield. The relevant questions are whether the risk was understandable, whether the player had a useful choice, and whether a successful gamble earns something lasting. Free healing, hidden mission precedence, and overrides of the player's retreat order fail that test.

AS, DS, and build costs are design variables. The existing values provide a baseline, not a constraint. Change them to support roles after identifying which problems come from resolution, rounding, or encounter timing. Otherwise a stat change can compensate for one bug while making another matchup worse.

**What to take from each source**

They support an NC hybrid, but they do not establish one common “correct” combat system:

- **Empire of the Sun:** borrow the effectiveness roll, qualitative advantages, and lasting damage. Its planning and activation limits help balance concentration. NC should not borrow its combat rules while assuming those constraints also exist.
- **VBAM:** borrow meaningful retreat, paid repairs, and conditional protection of vulnerable ships. Its lower damage rate works with different damage steps and command limits; copying its 30% average directly made the experimental NC battles drag.
- **EC4X:** borrow coordinated forces, orbital control before landing, and per-round ROE. Reject automatic proportional transport losses and rules that trap every defending fleet beside a base.
- **Classic EC:** preserve the valuable operational structure—movement, detection, interception, speed, and layers. Its decoded battles use range, armor, targeting, and other mechanics that NC’s AS/DS system does not reproduce. Classic prices and ship names therefore cannot establish that NC’s current values are balanced.

My recommendation is **decisive simultaneous combat with lasting consequences and usable retreat orders**. Keep destroyer execution and dangerous batteries where they create useful roles. Reward reconnaissance with bounded surprise, and reward superior force without the extra ratio-column damage multiplier. The review tests those choices against both the numbers and the player’s ability to understand the risk.

The [decoded classic battle](../../dev/ec-battle-spec.md), sections 1.1–1.3, makes the distinction concrete. Classic DD and CA both have 0.6 raw HP per build point, but the cruiser gains armor and range. NC's current CA has twice the DD's DS per point, while DD gains execution fire. Classic also limits how many units of a superior force fire at once; NC fires the assembled force and adds an odds-column advantage. These are different systems. Do not infer equivalent balance from familiar hull names or import classic's hidden engagement-count formula to repair NC's concentration incentives.

The [completed-reference follow-up](o7w8k8uh-what-the-completed-classic-combat-reference-changes-for-nc.md) adds formulas, exact classic ground odds, and sampled comparisons. The owner explicitly rejects classic combat results as a balance target. EotS and VBAM remain the stronger foundations. The follow-up also identifies NC's existing Blitz reward of capturing surviving batteries, distinguishes a coherent VBAM durability experiment from copying its effectiveness table alone, and records corrections needed in the new classic summary.

The later [t055 sweep review](qjev5wpq-review-of-the-t055-combat-sweep-semantics.md) checks the implemented experiments, their solver, and their validation limits. The [economy investigation](tkce84f0-a-simple-nc-economy-informed-by-classic-ec-vbam-and-ec4x.md) extends the balance question to colony growth, prices, travel, and replacement costs.

**Where I disagree with the synthesis**

The [synthesis](ebrogywq-nc-combat-design-review-against-its-sources.md) correctly identifies double use of force ratios, ineffective retreat, free repair, and a leaky orbital gate. Its proposed replacement needs more scrutiny.

“Who has more destroyers” does not describe committed fleet battles at equal build cost. Destroyers have the same AS per point as cruisers and battleships, much less durability, and take suppression damage first. Execution fire gives them a useful function. It does not make them the best battle fleet.

Agreement among the sources is not enough to select a rule. EotS restricts activation and stacking. VBAM restricts task forces through command capacity and reinforcement rules. NC lacks those constraints. Borrowing their combat participation rules alone can strengthen concentration. See the [EotS review](../../dev/combat-review/empire-of-the-sun.md), rules 7.21, 8.24, and 9.2, and the [VBAM review](../../dev/combat-review/victory-by-any-means.md), sections 4.6.2.1–2, pp. 39–40.

The sources also do not share a universal defender-wins-mutual-annihilation rule. EotS gives an empty air-naval battlefield to the offensive player, but ground mutual annihilation leaves control with the reaction player. VBAM uses different tie rules for different decisions. NC should choose an understandable ownership rule, not claim unanimous authority. See EotS 9.31 and 9.4.C.2, and VBAM 4.6.2 and 4.6.4.

“Joined sides or two sides” is a false choice. Two opposing empires can each have a joined side. The real decisions are which fleets participate, whose ROE controls them, and whether participation replaces the classic observer-versus-target rule.

Finally, classic pursuit is not a complete substitute for a withdrawal rule. It still needs sighting, hostility, interception, and movement in the same layer. Simply retaining ROE can cause the retreating fleet to volunteer for another battle. Several complete battles during one climb can be harsher than the withdrawal exchange being removed.

**What the arithmetic says**

The current five columns average 0.550, 0.850, 1.150, 1.425, and 1.725 times AS per exchange, before rounding and criticals. A 3:1 attacker therefore produces about `3 × 1.725 / 0.550 = 9.41` times the defender's hits. This is an additional advantage beyond having more guns and surviving longer.

The most troublesome boundary is near equality. Ten cruisers each way start at 345 expected hits per side. Eleven against ten produce 379.5 against 255. One extra cruiser increases the opening expected-hit ratio to 1.49. Even 100 cruisers against 99 produce a ratio of about 1.37. These are discontinuities, not gradual rewards for strength.

I ran 12,000 sampled battles per main matchup and rule variant, using seed 20261010. Both sides commit at ROE 10. Fire is simultaneous. Ships retain full DS for both damage steps and half AS when crippled. The experiments omit movement, detection, pursuit, repairs, planetary attacks, and campaign economics. “Win” means the left force retains combat ships and eliminates the other's combat ships; mutual destruction is separate. These are conditional battle odds, not mission or campaign success rates.

The current model reproduces the CRT, hit-pool split, target order, mixed bonus, base bonus, discarded remainders, and extra critical loss. It samples ordinary independent d10 rolls rather than reproducing Rust's contextual RNG. This is an independent arithmetic model and still needs comparison against engine scenarios before implementation decisions become release claims.

The alternative below is a concrete experiment, called **P** here:

- Flat effectiveness: d10 0–2 gives 0.25 AS, 3–5 gives 0.50, and 6–9 gives 1.00. Mean effectiveness is 0.625. This is the EotS air-naval table, also used by ec4x's space proposal.
- No mixed-fleet modifier. An undamaged base gives +1 to the die, capped at 9. No surprise modifier is included in these tests.
- CA and BB retain suppression; DD and bases retain execution. Ordinary targeting still follows the current combat-line order.
- A suppression critical may concentrate its existing hits on affordable high-DS combat steps, including finishing a crippled target. It does not create a free destroyed ship. Execution already concentrates damage.
- The last insufficient hit budget buys a chance of the next result: remaining hits divided by that result's DS cost. For example, 12 hits against a 30-DS step give a 40% chance of that step. Execution against a nominal ship uses its full 2×DS kill cost. The remainder is then consumed.

That last rule is an experimental answer to wasted damage and immunity thresholds. It adds one explicit partial-damage roll. Carrying partial damage forward is another option, but needs additional state. The reported P results use the probability rule; they must not be presented as results for a carry-forward system.

| Forces, left versus right | Current left win | P left win | Current mean rounds | P mean rounds |
| --- | ---: | ---: | ---: | ---: |
| 10 CA versus 10 CA | 50.3% | 49.8% | 2.67 | 4.73 |
| 11 CA versus 10 CA | 84.8% | 67.4% | 2.33 | 4.57 |
| 15 CA versus 10 CA | 100% observed | 97.6% | 1.40 | 3.28 |
| 20 CA versus 10 CA | 100% observed | 99.99% | 1.10 | 2.07 |
| 30 CA versus 10 CA | 100% observed | 100% observed | 1.00 | 1.70 |
| 30 DD versus 10 CA, cost 150 each | 4.5% | 9.9% | 1.23 | 2.34 |
| 90 DD versus 10 BB, cost 450 each | 1.8% | 5.5% | 1.23 | 2.32 |
| 30 CA versus 10 BB, cost 450 each | 37.3% | 39.0% | 2.75 | 5.07 |
| 10 DD versus 1 SB, cost 50 each | 1.0% | 14.9% | 1.09 | 1.47 |

SciPy exact 95% binomial intervals put the current 11-versus-10 result at 84.15–85.45%, and P at 66.56–68.25%. The current DD-versus-CA result is 4.11–4.86%. Sampling error cannot explain these differences. Near 50%, 12,000 trials give roughly ±0.9 percentage points. A result of 12,000 wins is not proof of certainty: its two-sided exact lower bound is about 99.97%.

P preserves the reward for overwhelming force. It also allows a smaller defender to impose a cost. In 15 CA versus 10 CA, current attackers lose about 2.95 build points and finish with 126.80 points of crippled ships that immediately heal. P loses about 30.44 points and retains 116.48 points of crippled ships. At a proposed 25% repair price, that is about 59.56 points of destruction plus repair, before travel and repair delay. Winning still pays; small defenders cease to be nearly free targets.

I also tested one literal interpretation of the synthesis: the VBAM 0.30-mean effectiveness table, all suppression, retained qualitative bonuses, discarded remainders, and a free highest-DS critical kill. This is one candidate, not a fully specified rule supplied by the synthesis. Equal ten-cruiser battles averaged 14.20 rounds. Three CA beat one BB 74.3% of the time, while thirty CA beat ten BB only 9.2%. Fixed per-side critical kills and damage thresholds create a large scale effect. Merely selecting a flat table does not balance the system.

**The seven decisions**

1. **Join same-empire forces locally, while retaining two opposing empires per battle.**

   Ships in the same sector and layer should be able to support each other. A defender should not watch a nearby friendly fleet die solely because the engine selected a different fleet record. Do not join across layers, add allied empires, or introduce a general multiplayer battle resolver in this change.

   Offensive participation needs a willingness test on the actual assembled sides. “ROE greater than zero” is too broad: a cautious reinforcement should not be conscripted into a hopeless attack by an aggressive picket. Defenders present at the contested position can be attacked even at ROE 0, but each mobile fleet must retain its own right to withdraw after the first exchange. A base cannot retreat; it must not imprison every friendly fleet beside it.

   Assemble participants before firing. Prevent the same engagement from being repeated through another observer in the same tick. Allocate losses and persisted damage to participating fleets through a published rule, such as proportional allocation within each class with deterministic remainder handling. Do not turn fleet record order into casualty doctrine.

   This improves mutual support and reduces fleet-splitting tricks. It also strengthens concentration. Implement the deterministic orbital gate first if joined mobile battles need more work. Joining sides and using their AS for willingness deliberately changes the classic encounter contract; record that change rather than promising unchanged classic decisions.

2. **Use a flat, fairly strong effectiveness table with small qualitative die modifiers.**

   Start with P's 0.625 mean, not VBAM's 0.30 applied unchanged to NC's two full-DS steps. Remove ratio columns. More ships already supply more guns, more damage capacity, and an attrition advantage. The trials show that a 50% advantage remains decisive without the second multiplier.

   Remove the categorical mixed-fleet bonus. In the current model, replacing one of thirty cruisers with three equal-cost destroyers raises that side's win rate against thirty cruisers to 64.3%. Under P it is 47.1%. Mixed fleets should earn their place through speed, durability, bombardment, and finishing fire, rather than a token hull improving every gun.

   A +1 die modifier changes mean effectiveness from 0.625 to 0.700, a 12% relative increase. A total +2 gives 0.775. Cap combined ordinary modifiers at +2 initially. This gives prepared defense and surprise visible value without recreating the ratio multiplier under new names. Do not add an overkill bonus above 1.5× enemy DS; the existing damage budget already rewards overwhelming fire.

3. **Check each fleet's ROE after the first exchange; replace the lock and withdrawal volley with a bounded pursuit rule.**

   Require one simultaneous exchange when a fight is forced. Then let ROE work. In an additional flat-table, all-suppression experiment, equal ten-cruiser forces at ROE 6 produced a withdrawal in 99.8% of trials after an average 1.40 rounds. They still suffered damage. Aggressive ROE 10 forces continued to decisive outcomes. This gives the player's order an observable consequence.

   Preserve the chosen ROE. Add a temporary withdrawing state that prevents volunteering for fresh combat while escaping. Recompute speed after losses. Begin a real climb out of orbit, then travel toward a legal friendly destination. Do not teleport across the layer boundary.

   My initial pursuit rule would allow the opposing empire one successful pursuit exchange during that retreat episode, using the existing sighting and speed-based interception tests. It would not start another full battle or apply an automatic extra withdrawal volley. Separation or a later deliberate re-engagement ends the episode. Splitting the pursuing force must not multiply its allowance. This is a proposed combat limit, not something classic already guarantees.

   Faster forces still punish exposed retreats. A defeated fleet has a plausible escape instead of facing a fresh annihilation contest every tick. If unlimited classic re-engagement is retained instead, measure that choice explicitly; it could erase most of the benefit of per-round ROE.

4. **Keep execution fire for DD and starbases. Remove free full-ship critical kills.**

   Execution is a useful role and the equal-cost evidence does not justify removing it. Keep its full price: 2×DS to kill a nominal ship, 1×DS to finish a crippled ship. Preserve the difference between heavy ships disabling a line and light ships finishing targets.

   Current criticals can bypass the nominal auxiliary screen. They first search for crippled units, then nominal units, using DD, SC, TT, ETAC, CA, BB, SB order. A critical does not invariably kill an auxiliary, but the manual's absolute protection claim is false.

   Changing that rule to “kill the highest-value hull for free” would be worse. A tiny picket would get a fixed 10% chance to erase a capital ship regardless of damage. Currently, even one DD against one BB destroys the BB on its natural 9 while dying itself. The DD cannot otherwise afford the kill. That is a 10% capital-loss lottery from a cost-5 attack.

   Let a critical focus an existing damage budget on valuable exposed combat targets. Keep the high roll and a clear report event. In P, the lone DD causes mutual destruction about 3.3% of the time through its limited execution budget. Upsets remain possible and scale with committed force. Do not let a critical silently override the chosen screen rule.

5. **Keep batteries as dangerous assault defenses, not passive siege expenditure.**

   Their 90 AS is the same as a BB's space AS; a BB actually supplies 135 bombardment AS. A battery also has one 20-DS step, versus two 100-DS ship steps. “Batteries outshoot battleships” misses both distinctions.

   Under current rules, with no docked assets or other forces, one BB always destroys one battery in its first bombardment exchange. The battery destroys the BB only on its free critical: 10%. Its ordinary return fire never buys a 200-hit kill. Expected BB replacement cost is 4.5 points against a 20-point battery. Conversely, four DD cost the same as one battery, always die to its return fire, and destroy it only 20% of the time. One CA also always dies, destroying the battery 40% of the time. Batteries punish cheap unsupported bombardment; heavy siege ships have a purpose.

   Keep 90/20 at cost 20 as the first test baseline, not a permanent promise. P's residual-damage rule changes these thresholds substantially. A single battery averages 56.4 rounded hits against a BB under the flat table. If insufficient execution damage buys a proportional chance of a 200-hit kill, that BB's destruction chance becomes 28.2%, despite the lower mean effectiveness. This is an exact first-exchange calculation, not a full assault simulation. It is a reason to test allocation and battery stats together, rather than assume flattening always reduces lethality.

   Use battery-first targeting, limited assault exchanges, retreat decisions, and paid repair to create siege costs. Give Invade up to three battery-suppression exchanges, with the option to break off after each. Do not grant it Bombard's full infrastructure destruction as a free extra. Preserve Blitz as a committed landing through surviving defenses. Do not add a blanket attacker damage tax merely because EotS charges one.

6. **Use a conditional auxiliary screen with a clear trigger.**

   While the combat line holds, normal fire protects SC, TT, and ETAC. When a fleet withdraws, successful pursuit can target its exposed auxiliaries with the actual pursuit hit budget. When its line is destroyed, ordinary targeting also reaches them. This makes escorts useful without making a surviving token escort an unlimited shield during escape.

   Do not automatically destroy the same percentage of transports as escorts. That is an extra casualty tax disconnected from firepower and it penalizes a successful retreat twice. Do not base screening capacity on raw hull count; that would create a new reason to spam DD. A separate screen-capacity formula can wait until evidence shows it is necessary.

   Explain whether pursuit prioritizes loaded transports. Their loss includes the embarked army. Crucially, an unarmed fleet must also get a withdrawal attempt after its forced exchange. Current zero-AS fleets skip the ROE path and can be shot until eliminated or the guardrail is reached. Scout speed and an empty transport's escape route should matter after contact, too.

7. **Use surprise, initially as a first-exchange +1 die bonus rather than first fire.**

   Surprise should reward reconnaissance, concealment, and approach timing. First fire in a system where a single volley can cripple a whole side compounds the advantage sharply. A first-round modifier adds payoff while preserving simultaneous return fire. Test first fire later if the resulting casualty distributions support it; do not introduce it alongside several other lethality changes without isolation tests.

   Determine surprise from the defender's alert state before battle handling marks the contact as fought. Awareness resets at sector and layer changes, and its working state is recreated each maintenance year. Those resets are not proof that a defender forgot an enemy already in orbit. Keep a battle-readiness fact long enough to prevent repeated free surprises from layer hopping or the year boundary. Awareness 3 means fought, not an entitlement to ambush.

   The deterministic strike gate must not turn every battery-only world into an automatic surprise victim just because planets lack a starbase sighting unit. State what planetary alertness means. Scouts should help players discover defenses and prepare attacks; adding one scout should not become another mandatory fleet-wide damage coupon.

**Ship values, prices, and roles**

The following values and prices come from the [player manual](../../manuals/nc_player_manual.typ), ship table, ground forces, and Appendix B.

| Unit | Cost | Speed | AS / DS | AS per cost | DS per cost | Bombard AS per cost |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| DD | 5 | 6 | 10 / 5 | 2.00 | 1.00 | 1.00 |
| CA | 15 | 5 | 30 / 30 | 2.00 | 2.00 | 2.00 |
| BB | 45 | 4 | 90 / 100 | 2.00 | 2.22 | 3.00 |
| SC | 15 | 6 | 0 / 10 | 0 | 0.67 | 0 |
| TT | 5 | 5 | 0 / 10 | 0 | 2.00 | 0 |
| ETAC | 20 | 3 | 0 / 20 | 0 | 1.00 | 0 |
| SB | 50 | 1 | 100 / 120 | 2.00 | 2.40 | — |
| Battery | 20 | Fixed | 90 / 20 | 4.50 | 1.00 | — |

For ships and bases, DS is the cost of one damage step. Destruction normally consumes two steps; a battery consumes one. These are not directly comparable hit-point totals. A loaded TT costs 7 points including its army. Economic value and the value of an army arriving at the right planet can exceed replacement cost.

**Destroyers need survivability testing, not an execution nerf.** They are fast pickets, interceptors, finishing ships, and expendable escorts. At the current DS, they disappear too quickly to serve as reliable combat escorts. Cheap construction and speed still give them campaign uses, so an equal-budget fleet duel should not automatically be 50:50.

The following one-variable and combined experiments use 6,000 trials per cell, with ten CA costing 150 as the opponent. The DD force spends exactly 150. P otherwise uses the same experimental rules above.

| DD AS / DS / cost | Current DD win | P DD win |
| --- | ---: | ---: |
| 10 / 5 / 5 | 4.6% | 9.7% |
| 10 / 7 / 5 | 15.6% | 24.3% |
| 10 / 8 / 5 | 23.9% | 31.5% |
| 10 / 10 / 5 | 41.7% | 50.7% |
| 12 / 8 / 5 | 68.2% | 45.8% |
| 12 / 8 / 6 | 13.9% | 21.6% |

These trials have about ±1.3 percentage points of sampling uncertainty near 50%. A separate 12,000-trial check of P at DD 10/8/cost 5 gave 31.9% against CA, 23.9% for ninety DD against ten BB, and 35.3% for ten DD against one SB. Mutual destruction adds another 5.1% in that last matchup. The base still usually holds, but an isolated base faces a credible assault.

Start playtesting DD 10/8/cost 5. It improves escort survival while leaving cruisers and battleships better at committed fleet combat. DS 10 nearly equalizes the cruiser duel before pricing DD speed, making it a riskier starting point. Raising DD AS to 12 under the current ratio CRT also crosses a damage-column boundary; that is why a modest gun increase produces such a large jump. Increasing price to 6 removes a sixth of the affordable hulls. Price is a powerful balance variable, not bookkeeping.

**Cruisers already have a sound general-purpose role.** They are faster than BB, more durable per point than current DD, and escort transports without reducing their speed. Start at 30/30/cost 15. Raising CA DS to 35 moved thirty CA versus ten BB from about 39% to 54% under P. That may be appropriate if campaign tests show battleship dominance, but the duel alone does not demand it.

**Battleships should remain excellent at planned offensives.** Their modest durability efficiency, strong bombardment, and hull-slot efficiency justify their speed cost. Start at 90/100/cost 45. Raising BB DS to 120 lowered the cruisers' P win rate to about 25%, from 39%. There is no present evidence for that buff. Do not reduce BB guns merely to make all three warship types interchangeable.

**Scouts need an information-value test.** At cost 15 they compete directly with a cruiser while contributing no combat AS. Speed, stealth, identification, and better decisions must repay that investment. Extra DS scarcely helps while the screen is absolute, and is useless if contact prevents an unarmed retreat. Fix those rules before trying to solve scouting through armor. Keep 0/10 initially; include cost 10 versus 15 and DS 10 versus 15 in reconnaissance mission tests. Those are test candidates, not measured recommendations. Keep scouts unarmed unless the intended role itself changes.

**Transports should remain vulnerable but rescuable.** Start at 0/10/cost 5, plus cost-2 cargo. Their value is access to conquest, not gun efficiency. A cautious convoy with adequate escorts should have a better survival distribution than an unsupported gamble. Test loaded troops delivered, mission success, and extraction losses. Test TT price changes against invasion frequency and the defender's cheap local armies, not against CA duel performance.

**ETACs should be expensive strategic targets.** Start at 0/20/cost 20. Reusable colonization gives lasting value, while speed 3 slows every accompanying hull. Their escape and interception mechanics dominate small DS changes. Test colonies founded per invested point and the cost of escorts. Cheaper or faster ETACs change the expansion race across the entire game; do not select their price from a combat-only optimizer.

**Starbases buy more than a combat unit.** At 100/120/cost 50 they also buy strong detection and economic benefits, including the treasury-backed 5× spending ceiling. Their lack of operational mobility is substantial. Keep the baseline initially and award at most one undamaged-base combat modifier per side. In the current ten-DD test, increasing base DS from 120 to 150 changed nothing: ordinary DD fire could not buy the kill either way, while the free critical ignored armor. Fixing that rule matters more than adding DS. Reassess base price against both fleet defense and economic payback.

**Concentration and the 2.5D year**

Flat effectiveness reduces one incentive to form a doomstack; it does not remove the normal benefits of concentration. Bigger forces lose a smaller fraction of their guns, preserve more damage capacity, and protect their auxiliaries. Joined sides make those advantages easier to assemble. At the 3,000-hull fleet limit, capitals also pack more power: 3,000 BB carry nine times the AS and twenty times the DS of 3,000 current DD, at nine times the price. Equal hull counts are not an economic comparison.

Do not add an arbitrary anti-stack penalty just to produce smaller battles. Make dispersed objectives, interception, repair time, and travel costs matter. A fleet holding one layer cannot also control the other. A capital stack that wins one world should leave opportunities elsewhere. Joined participation must not silently erase every practical consequence of the fleet-size limit.

The [movement spec](../../dev/ec-movement-spec.md) gives a 52-tick year and a real depth axis. An orbit-to-orbit journey spends one distance unit climbing and one descending. Ignoring tick rounding and interruptions, first-year horizontal reach after both legs is about four sectors for DD, three for CA/TT, two for BB, and one for ETAC. The cost of adding one slow hull is therefore large. A TT reduces a DD fleet from speed 6 to 5; a BB reduces a cruiser fleet to 4; an ETAC takes it to 3.

The last half-depth of descent, from layer entry at Z=0.5 to orbit at Z=1, lasts about `26/speed` ticks: 4.33 for DD, 5.2 for CA/TT, 6.5 for BB, and 8.67 for ETAC. Actual boundaries depend on fractional movement and tick order. That is a substantial interception window. Layers are useful theaters because they impose time and positioning, not simply because they supply labels for combat phases.

A base in orbit does not seal the deep-space transit layer. Pickets and patrols can defend the approach while the base covers the destination. Conversely, a deep-space defender must not count as an orbital obstacle after an attacker has reached the world. Show both positions and their expected arrival times to the player.

The [encounter rules](../../dev/ec-encounter-spec.md) also qualify the synthesis's pursuit claim. A faster pursuer always catches only after the sighting and attack conditions succeed. For a moving target with awareness 3 that declines combat, an equal-speed pursuer enters the relevant intercept branch only 1% of the time. Its conditional catch probability there averages `2/π`, giving about 0.64% per successful sighting. If the target instead accepts combat under its unchanged ROE, it can recommit through the other branch. A withdrawal state is essential.

Repeated chances matter. With a hypothetical 35% sighting chance and a faster willing pursuer, seven checks give a 95.1% chance of at least one interception. This is an illustrative calculation, not an estimate for every fleet. Retreat changes assault visibility and the actual chance varies by sensors, awareness, speed, and orders. A neutral blockade can also lose its assault-specific hostility trigger when the victim switches to Seek Home. The existing loop therefore supplies neither guaranteed pursuit nor guaranteed disengagement.

All tactical rounds resolve inside an encounter tick. Increasing a battle from three to five rounds does not delay it by two weeks or give distant reinforcements time to arrive. Extra strategic delay requires a rule outside the round loop. Likewise, serial unit stepping means a contact may use another unit's position before or after its move, depending on the year's shuffled processing order. Test layer-boundary arrivals and disclose timing uncertainty; do not promise perfectly simultaneous movement.

**Additional holes in the current spec and implementation**

**A strike's eligibility tick is not its resolution tick.** This is the most consequential omission from the reviews. The movement dispatcher records hostile strike fleet indices. The maintenance function runs the entire encounter year, processes later maintenance work including builds and economics, and only then calls `process_planetary_assaults`. See [movement/mod.rs](../../../rust/nc-engine/src/maint/movement/mod.rs), [maint/mod.rs](../../../rust/nc-engine/src/maint/mod.rs), and [assault.rs](../../../rust/nc-engine/src/maint/combat/assault.rs).

Consequently, an early eligible attacker can meet late-year relief or lose its mission before the assault resolves. Early and late entrants can be pooled. Newly completed defenses can participate in an assault whose eligibility occurred earlier. Ownership does not change for the remaining encounter ticks. The synthesis and source reviews describe an immediate strike that this call sequence does not implement. ADR b5u6lekb already requires the actual strike tick, so this is a correctness fix, not an open timing choice. Land it after t048 settles economy and builds before the tick year. Changes to the documented annual ownership cap and mission-class precedence still need to be recorded as NC policy amendments.

**The orbital gate is broader than an unsighted-base bug.** Supremacy selection works over strike entrants rather than every defending force at the position. A stationary defending fleet can therefore be omitted too. Its zero-combat-AS fallback can consider a base at the same coordinates without the required layer distinction. Build the gate from actual eligible layer-1 defenders and fight them. A numeric AS comparison must not silently award the orbit to one assault entrant.

**Ground combat has an extreme, exact threshold.** The ground resolver feeds army counts into the same ratio CRT, adds the defender column, and kills one army per hit, plus the critical. If both forces vanish, the larger force entering that exchange receives one surviving army. Equality favors the defender. The following probabilities enumerate every d10 pair recursively; they are not Monte Carlo estimates. They start after landing and any softening fire.

| Landed attackers versus 10 defenders | Current capture chance | Current mean rounds | Candidate capture chance | Candidate mean rounds |
| --- | ---: | ---: | ---: | ---: |
| 8 | 0% | 1.00 | 7.8% | 3.43 |
| 10 | 8.0% | 1.02 | 34.5% | 4.36 |
| 11 | 83.0% | 1.10 | 52.9% | 4.49 |
| 12 | 86.1% | 1.10 | 69.9% | 4.37 |
| 15 | 100% | 1.00 | 96.2% | 3.34 |
| 20 | 100% | 1.00 | 99.98% | 2.31 |

The candidate is simple simultaneous per-army fire: each attacker hits with probability 0.30, each defender with 0.35, and a hit kills one army. There is no extra critical; mutual destruction leaves ownership with the defender. Those candidate figures are also exact finite-state calculations. They illustrate a smoother alternative, not a decision to port classic's sequential ground battle.

Merely awarding all current mutual destruction to the defender reduces eleven-army capture odds from 83% to 42.6%, but leaves the underlying damage cliffs. Merely applying the flat space table also leaves a large step between ten and eleven armies. Give ground combat its own calibrated rule. One-step armies do not behave like two-step ships. Do not add the synthesis's extra homeworld modifier until the base model works. A successful landing can be costly without depending on an obscure remnant exception.

**Retreat geometry and speed need a contract.** In [retreat.rs](../../../rust/nc-engine/src/maint/combat/retreat.rs), `set_mission_target(0, 0)` clears the depth target as well as mission identity. Seek Home therefore targets the deep-space layer of the selected home sector. That is not arrival at a repair-capable home orbit. Define the escape leg, safe destination, and eventual descent separately. Choose by travel distance through the movement model, not Manhattan distance. Avoid choosing the contested home orbit as an immediate destination that cancels its own escape. With no home, climb and disengage; do not invent destruction merely because a tabletop game lacks a legal retreat hex.

Loss distribution writes new ship counts, but I found no corresponding maximum-speed recomputation in the combat return path. A surviving DD fleet can retain the speed of a destroyed slow hull. That changes both travel and interception. Verify and fix this with an engine scenario before balancing pursuit. Also ensure fallback Hold does not reset a moving fleet's depth and position through an at-rest helper.

**Mission priority is a hidden coordination trap.** Assault resolution chooses one class per planet per year: Blitz before Invade before Bombard. A small same-empire Blitz component can suppress the intended work of a much larger bombardment or invasion force. Resolve chronological assault events, or require an explicit coordinated assault plan with a visible mission choice. Do not let an incidental order become an empire-wide override. If strikes move into ticks, reconsider the once-per-year ownership-change rule too: an early capture should not gain magical immunity from a later counterattack.

**Blitz already breaks the universal simultaneous-fire claim.** Its cover fire removes batteries before surviving batteries shoot at the landing transports. A BB covering a Blitz against one battery always clears it under the current table, so that battery never replies. Ordinary invasion suppression gives the battery its simultaneous shot and 10% free-critical chance to kill the BB. This can be a legitimate reward for overwhelming cover, but the manual must say so. On an undefended world, escorted Invade also gets free capped softening; Blitz does not inherently arrive faster. Define Blitz through its landing commitment and willingness to risk transports, not an undocumented speed advantage.

**Stardock cargo is ablative armor for active batteries.** Bombardment currently spends hits on docked assets before batteries. A docked ship can absorb damage that would otherwise silence the guns protecting it. This contradicts the stated battery shield and rewards an unintuitive form of preparation. Hit active batteries first. After they fall, use one published collateral sequence. Also state that current infrastructure damage removes stored goods and factories from the same damage allotment, rather than implying two sequential purchases of damage.

**Several player-facing promises disagree with code.** The manual says the mixed bonus requires DD, CA, and BB; `is_mixed` accepts any two. The starbase column bonus is not limited in code to an undamaged base at its owned world. Critical and execution fire contradict the introductory blanket rule that every nominal ship must be crippled first. The 64-round fallback uses role priority, AS, and empire ID rather than a simple defender tie rule. Ownership-based role classification also needs a layer check. Fix these as one rule-authority problem, not isolated wording edits. The cap should report a stalemate and preserve survivors, not manufacture a strategic victory from record identity.

**Persistent damage is essential, but repair needs limits.** Keep crippled counts through fleet combat, pursuit, and the later planetary attack. Otherwise a supposedly coordinated siege can start with fresh ships after its orbital battle. Repair at owned worlds for 25% of build price is a reasonable first experiment. Charge the ordinary production budget, require a stated repair interval, and aggregate fractional costs before rounding. Four cost-5 DD repairs should cost 5 points, not four separately rounded charges of 2. Do not allow an instantaneous full repair at a world just captured. Test whether home repair makes sustained offense too costly before adding further supply penalties. NC does not need VBAM's whole logistics system to make damage matter.

**Monte Carlo tooling and a durable balance workflow**

The reusable models and results are retained in [tools/combat-model](../../../tools/combat-model/README.md). The [fleet model](../../../tools/combat-model/nc_combat_analysis.py) and [ship sweep](../../../tools/combat-model/nc_ship_sweep.py) reproduce the review experiments. The [ground comparison](../../../tools/combat-model/ground.py) adds the defender-first candidate below, and [confidence.py](../../../tools/combat-model/confidence.py) reproduces the statistical intervals. The original fleet and ship result values reproduced exactly during preservation. These are experimental Python models; engine validation remains separate.

Python is fast enough for the arithmetic experiments. On this machine, one measured run of 12,000 P battles between ten-cruiser forces took about 0.45 seconds, approximately 27,000 battles per second. This was ordinary Python with aggregated ship counts, not NumPy acceleration. It is a single local microbenchmark, not an engine-throughput claim. At that rate, a million comparable battles is roughly forty seconds. Larger forces, richer state, and campaign simulation need separate measurements.

Use [NumPy's seeded Generator](https://numpy.org/doc/stable/reference/random/index.html) for sampling and batched calculations, and [SciPy's binomial confidence intervals](https://docs.scipy.org/doc/scipy/reference/generated/scipy.stats.binomtest.html) for win and survival rates. SciPy was used for the intervals above. NumPy 2.5.4 and SciPy 1.18.1 were installed only in disposable scratch space for this analysis. The battle model itself uses the Python standard library. Optimization is optional: [Optuna's multi-objective search](https://optuna.readthedocs.io/en/stable/tutorial/20_recipes/002_multi_objective.html) can later expose tradeoffs among pacing, role usefulness, casualty cost, and mission success. It should not choose a design by minimizing distance from 50% wins everywhere.

The repository already has the more important foundation:

- [nc-fuzz](../../../rust/nc-fuzz/src/lib.rs) supplies Hegel/proptest drivers, policies, and engine invariants. Use these to find broken states and small reproductions. Their guided case distribution is not a probability distribution for real player battles.
- [nc-harness](../../../rust/nc-harness/src/play.rs) supplies scripted KDL scenarios through real turn submission and maintenance. Use fixed representative scenarios for balance measurements and regression checks.
- [maint_throughput.rs](../../../rust/nc-fuzz/examples/maint_throughput.rs) supplies an existing pattern for independent games and throughput measurement. A combat-odds example in nc-fuzz is a natural extension.
- [ncmaint_mission_sweep.py](../../../tools/ncmaint_mission_sweep.py) helps investigate classic-versus-Rust timing. The supplied classic `sim.py` and `fsim.py` models help explain historical behavior. Neither classic victory odds nor DOS bug compatibility should be NC's balance target; the [combat decision](../decisions/b5u6lekb-rust-keeps-its-own-combat-system.md) explicitly keeps NC combat.

One practical trap: `ScenarioPlay::run_maintenance` currently passes campaign seed 0 to the engine. Repeating the identical scenario through that method does not sample new combat luck. A future sweep must vary the actual combat seed through the existing seeded engine entry point or a small harness extension. Record map/setup seeds separately from combat seeds.

Run Rust scenarios in batches and let Python analyze compact results. Avoid launching a process for every battle. Keep the fast Python rule experiments for exploring alternatives, then compare selected cases against Rust so the model cannot drift unnoticed. Build-cost, AS, and DS tables should be explicit inputs to the experimental model; engine changes can stay narrow until a candidate is selected.

Use three levels of evidence. First, enumerate small cases exactly: a d10 exchange has only 100 paired outcomes. Second, sample fixed battles with fresh combat seeds and confidence intervals. Third, run campaign scenarios with travel, scouting, repair, production, and varied policies. Keep setup distributions fixed when comparing rules; otherwise a fleet change can be confused with a map change. Use separate validation seeds after selecting promising values.

The scenario set should include small and large equal-cost fleets, mixed fleets, 1.1:1 through 3:1 forces, convoy retreats, scout escapes, base relief, three assault missions, late arrival, repeated pursuit, and two empires racing for a world. Measure objective success, surviving build value, repairs, armies delivered, combat rounds, year-level delay, and catastrophic-loss probability. Record mutual destruction and stalemate separately. A fast raider's successful damage-and-exit mission is not a lost experiment merely because it leaves the field.

Finally, use human playtests to check whether players can predict broad outcomes and explain surprising losses after reading the report. Monte Carlo cannot measure whether a rule felt like an earned gamble or a hidden exception. Do not optimize only against the current AI policies; that risks tuning the game to their habits.

**What players need to understand**

Publish a short sequence: who joins; how AS becomes hits; how damage selects targets; when ROE permits withdrawal; what pursuit can do; when orbital control permits an assault; how survivors repair. Give the exact tables in Appendix B and worked examples in the combat chapters. Keep the longer explanation optional during normal play.

Separate facts the player knows from estimates about the enemy. The order screen should show the fleet's layer, speed, expected arrival window, current effective AS, ROE threshold, and whether its escorts or cargo slow it. A combat estimate may use identified enemy forces and clearly stated uncertainty. It must not reveal unseen ships through an exact probability preview.

Explain that ROE 0 means “do not initiate,” not immunity from attack. If classic's first-identification commitment remains, state that exception beside the ROE selector. Prefer one AS yardstick for willingness and withdrawal after deliberately updating the encounter contract. Otherwise price or gun adjustments can change battle capability while leaving the old `21/7/2` attack weights unchanged.

Reports should identify participants, surprise, rolls and modifiers, damage spent, crippled survivors, destroyed cargo, retreat reason, destination, and mission outcome. Give a short readable result first, with arithmetic available underneath. “The fleet withdrew after its combat strength fell below your ROE threshold” tells the player what their order did. “Mission aborted” alone does not.

Preserve excitement through valuable objectives, recoverable setbacks, and occasional costly successes. A player who knowingly risks transports to seize a weakly held world should sometimes win it. A small defense that damages a superior invasion force should create repair cost or time for relief. Neither result requires arbitrary comeback bonuses, compulsory ship recipes, or a guarantee that every class wins the same fight.

**Change order, ranked by gameplay gain per unit of work**

1. **Make the combat contract truthful.** Reconcile manual/code contradictions, publish actual mission precedence, and add meaningful retreat and strike reasons to reports. Recompute survivor speed and correct retreat destination semantics. These are relatively narrow changes with immediate value to player trust.
2. **Fix orbital entry and strike timing.** Enforce an actual same-layer defense battle before assault. Resolve strikes when they become eligible, with explicit economic ordering. This is larger work, but stat tuning cannot compensate for late relief or defenses appearing retroactively.
3. **Replace the ground cliff.** Separate ground resolution from the space CRT and remove the larger-force resurrection rule. Use exact odds to choose the replacement. Do this before an additional homeworld defense bonus.
4. **Make retreat and damage persistent as one playable package.** One compulsory exchange, per-fleet ROE, an escape state, bounded pursuit, auxiliary exposure, and paid repair. Ship persistence needs more storage work, but without it a successful raid often leaves no lasting result. Keep DD/base execution while this changes.
5. **Replace ratio columns and calibrate hulls with a repeatable sweep.** Start with the stronger flat table, budgeted criticals, and a tested remainder rule. Test DD 10/8/cost 5 first. Keep CA, BB, and auxiliary baselines until operational tests justify a change. Recheck battery losses because removing damage floors changes them sharply.
6. **Broaden joined participation, then add bounded surprise.** The orbital gate supplies the most urgent mutual defense first. Wider joining needs per-fleet withdrawal and fair loss allocation. Surprise needs reliable pre-battle alert state and clear reporting. Neither should precede the rules that let a player survive and understand a failed plan.

**Consultation with the synthesis author, 2026-10-10**

Claude independently checked the timing, survivor speed, retreat target, and mixed-bonus findings against main at `c2779ffa` and confirmed them. The following records agreement between reviewers. It does not turn this investigation into an accepted gameplay ADR or authorize every proposed balance change.

I accept a controlled comparison of three remainder treatments: discard, residual probability, and carry-forward. No default adoption of the residual roll. All other rules must remain the same across that comparison. The P tables above measure the probability variant only; a carry-forward version needs new measurements before selecting hull values. Persistent partial damage is feasible in the typed model, but define which side, class, and damage step owns it, how suppression and execution interact, and what split, merge, retreat, repair, and target destruction do to it. An unspent hit pool must not migrate to a different enemy.

I support A1 before A2: extract one eligible-strike operation and invoke it from tick dispatch after t048. Keep the orbital gate as the next focused change. Preserve once-per-year bombardment eligibility, one-shot invasion/blitz completion, stable fleet identity, and encounter-table updates after casualties and ownership changes. Do not accidentally fire a persistent bombardment order on all 52 ticks. The mandatory base gate restores classic behavior. Automatically joining every defending fleet changes participation and must be recorded as such.

Separate narrow fixes from policy decisions. Recomputing survivor speed and reusing [the existing nearest-world picker](../../../rust/nc-data/src/directory/mutators_nearest_world.rs) are narrow fixes. Dropping the mixed bonus, changing critical screening, replacing the round-cap outcome, introducing withdrawal immunity from voluntary combat, removing annual capture limits, and changing mission precedence affect gameplay. They remain candidates until selected. Suppressing first-identification recommitment already changes the encounter contract; that change cannot be deferred on paper to the later joined-sides ADR.

Define a pursuit episode by the fleeing fleet's retreat identity and its departure sector and layer. It begins at the withdrawal decision and ends on actual departure from that sector/layer or a deliberate new order. The pursuit allowance belongs to each opposing empire, not each pursuing fleet. A pursuer withdrawing ends its eligibility; it must not reset an allowance that another fleet can reopen. Preserve an unfinished episode across the year boundary. The temporary withdrawal state must suppress first-identification commitment. Alert state for surprise also needs an explicit reset or expiry rule: neither yearly amnesia nor permanent immunity after one old sighting is desirable.

For planetary fire, I accept testing batteries first with the shield lasting for the whole exchange. Destroying the final battery does not expose the dock to that same exchange's remainder; the dock becomes eligible from the next exchange. State that rule explicitly. It is separate from whether partial damage on one ship carries forward. For repair, determine eligibility from a maintenance-start snapshot: owned world, fleet at rest in orbit at Z=1, aggregate fractional costs before rounding. Mid-year arrivals and captures do not grant immediate repairs.

**An additional ground candidate: defender-first volleys**

Classic actually uses independent per-army hit rolls, with defenders firing first at 40–49% and surviving attackers at 30%. A count-based volley system can retain the first-shot advantage while being easier to explain through NC's AS and DS.

Keep army AS10 and DS10. Both sides use one d10 effectiveness roll per exchange, with percentages `10, 20, 20, 30, 30, 30, 30, 40, 40, 50`. Defenders fire first. Remove attacker casualties, then calculate return fire from surviving attackers. Damage below 10 remains on the next army on that side through the battle. There are no odds columns, extra criticals, homeworld bonuses, or remnant exceptions. A wiped-out attacker cannot reply, so mutual annihilation cannot occur. First fire alone supplies the defender advantage.

The 0.30 mean is being tested here against one-step armies. Its roughly five-exchange equal battle does not imply that the same table suits two-step fleet combat.

Example: ten defenders rolling 30% produce 30 damage and remove three of twelve attackers. The nine survivors rolling 30% produce 27 damage: two defenders die and seven damage remains on the next defender. That damage carries into the following exchange. The damaged army keeps its AS until destroyed; there is no army cripple state.

| Attackers versus 10 defenders | Exact capture chance | Mean exchanges |
| --- | ---: | ---: |
| 8 | 0.87% | 3.64 |
| 10 | 15.16% | 5.06 |
| 11 | 34.22% | 5.54 |
| 12 | 56.89% | 5.56 |
| 15 | 96.57% | 4.03 |
| 20 | 99.997% | 2.65 |

These are finite-state enumerations, not sampled estimates. Compare them with the simultaneous candidate above; neither is selected yet. Defender-first volleys are more defensive at equal numbers, while both reward a substantial invasion force. Test multiple scales too: equal forces of one, two, five, and ten armies give attacker odds of about 30.5%, 23.0%, 18.0%, and 15.2% in this candidate. Shared volley rolls and indivisible armies still matter. Choose the desired risk and defender advantage explicitly, rather than selecting a rule solely because it resembles classic.

The [retained ground results](../../../tools/combat-model/nc_ground_comparison.json) compare current, simultaneous, and defender-first rules. Five Python checks cover the original critical lottery and ground cliff, removal before reply, carried subthreshold damage, and seeded reproducibility. No cargo or engine tests were run for this consultation.
