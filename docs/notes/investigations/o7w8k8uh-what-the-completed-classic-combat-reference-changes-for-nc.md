---
id: o7w8k8uh
date: 2026-10-10
kind: investigation
status: investigating
tags: [investigation, combat, balance, simulation]
---
# What the completed classic combat reference changes for NC

Use EotS and VBAM as the foundations for NC's battles. Use classic to understand movement, encounters, mission consequences, and historical defects. Its combat results are not a balance target: the owner explicitly dislikes them. The completed reference strengthens the case for independent NC values. Classic's per-target overkill, armor, range, and partial engagement produce radically different outcomes from NC's pooled hits. Keep the earlier recommendations for decisive simultaneous fire, meaningful retreat, lasting damage, and transparent risk. Add a proper comparison of mission rewards, including the batteries a successful Blitz already captures. Do not import classic's destruction of captured industry just to give Blitz a purpose.

**Evidence and scope**

This follows the [independent combat review](rolr9ti0-independent-nc-combat-balance-analysis.md). It examines the completed [classic combat reference](../../dev/ec-combat-reference.md), [battle spec](../../dev/ec-battle-spec.md), [encounter spec](../../dev/ec-encounter-spec.md), and relevant [economy reference](../../dev/ec-economy-reference.md) at `4825ba5e`. Rust timing was rechecked after t048 merged, at `2520200d`. The [EotS](../../dev/combat-review/empire-of-the-sun.md) and [VBAM](../../dev/combat-review/victory-by-any-means.md) reviews remain the design sources. This is an investigation, not a new rules decision.

The formulas below make the consequences reviewable. The [reproduction tool](../../../tools/combat-model/classic_reference.py) and [results](../../../tools/combat-model/classic_reference_results.json) stay in the repository. No cargo or new DOS probes were run.

The [classic model guide](../../../tools/classic-model/README.md) describes the decoded battle models; the [NC combat model guide](../../../tools/combat-model/README.md) documents the comparison commands. Later findings are in the [t055 sweep review](qjev5wpq-review-of-the-t055-combat-sweep-semantics.md) and the [economy investigation](tkce84f0-a-simple-nc-economy-informed-by-classic-ec-vbam-and-ec4x.md).

**The foundation should be EotS and VBAM**

EotS supplies a clear attack-strength-to-damage calculation, qualitative advantages, and meaningful reduced units. VBAM supplies usable retreat, conditional protection of vulnerable ships, and an economic reason to save damaged ships. These are better starting points for the experience NC wants than classic's hidden targeting and range machinery.

One concrete EotS-derived experimental baseline is:

```text
effective_AS = nominal_AS + crippled_AS
hits = ceil(effective_AS × CER[clamp(d10 + modifier, 0, 9)])
CER = [0.25, 0.25, 0.25, 0.50, 0.50, 0.50, 1, 1, 1, 1]
E[CER] = 0.625
E[CER with +1] = 0.700
```

The modifier gives 12% more mean effectiveness. Two identical undamaged forces with a 3:1 AS ratio generate about 3:1 expected hits, before rounding and allocation. NC's current ratio columns turn that into about 9.41:1. Concentration remains powerful without an extra multiplier for already having more guns.

This does not establish that 0.625 is uniquely correct. A coherent VBAM experiment should also consider its reduced defense value after crippling. Ordinary VBAM ships have roughly `DS + DS/2 = 1.5 DS` across their two damage steps; NC currently uses `2 DS`. Copying VBAM's 0.30 mean while retaining NC's full second step and other NC rules is an incomplete adaptation. The earlier 14-round result diagnoses that particular hybrid, not VBAM itself. AS, DS, costs, and step durability are all design variables.

Keep the three remainder experiments controlled: discard, residual probability, and carried partial damage under otherwise identical rules. Test a coherent VBAM durability variant separately. Neither the residual roll nor a particular ship table has been selected. NC also lacks EotS's activation constraints and VBAM's command limits; neither source automatically solves concentration in NC.

**Classic's armor and overkill explain why copying its values fails**

For a shooter with hit probability `p`, a uniform damage roll from 1 through `M`, and target armor `a`, classic's ordinary shot gives:

```text
damage = max(0, roll(1..M) - a), if the shot hits; otherwise 0
E[damage] = (p/M) × sum(max(0, j-a), j=1..M)
          = p × (M-a)(M-a+1) / (2M), for integer 0 ≤ a ≤ M
```

These are HP, not NC hits. They assume the gun fires and ignore overkill. From battle spec §1.1:

| Shooter | Build cost used | Armor 0 target | Armor 1 target | Armor 2 target | Armor 3 target |
| --- | ---: | ---: | ---: | ---: | ---: |
| DD | 5 | 1.375 | 0.825 | 0.4125 | 0.1375 |
| CA | 15 | 3.250 | 2.750 | 2.2917 | 1.8750 |
| BB | 45 | 8.325 | 7.875 | 7.4375 | 7.0125 |
| Starbase | 50 | 11.100 | 10.500 | 9.9167 | 9.3500 |
| Battery | 20 | 12.025 | 11.375 | 10.7431 | 10.1292 |

At equal cost, nine DD produce 3.7125 expected damage against armor 2, versus 7.4375 from one BB. That looks like a capital advantage. But classic cannot carry a large shot's excess damage into another ship:

```text
HP actually removed = min(target HP remaining, damage)
E[HP removed by BB against a fresh DD]
  = 0.45 × (1 + 2 + 34×3) / 36
  = 1.3125
```

The BB's nominal 8.325 expected damage becomes only 1.3125 useful HP against a fresh destroyer. Roughly 84% is unusable in that particular shot. A CA removes 1.375 expected HP from a fresh DD despite costing one third as much as the BB. Target persistence and partially damaged targets alter subsequent shots, but the structural problem remains. NC's pooled hit allocation distributes a volley across multiple ships, so it changes this relationship completely.

Range supplies another advantage that NC does not model. With both sides closing at their full speeds, DD range 10 versus BB range 20, and initial distance uniformly 5 through 24, fourteen of twenty starting distances give the BB an opportunity to fire before the DD can reply. That is 70%, assuming both remain in the fight and retain those speeds. It is not a 70% surprise roll or a victory probability.

The retained classic model combines these effects as follows. All fleets use ROE 10. Costs are the current manual's build prices. Each classic case has 2,000 trials; NC columns come from the earlier 12,000-trial experiments.

| Left force versus right force | Classic model: left wins | Current NC model | Earlier P experiment |
| --- | ---: | ---: | ---: |
| 30 DD versus 10 CA | 94.6% | 4.5% | 9.9% |
| 90 DD versus 10 BB | 100% observed | 1.8% | 5.5% |
| 30 CA versus 10 BB | 96.8% | 37.3% | 39.0% |
| 10 DD versus one starbase | 0% observed | 1.0% | 14.9% |

These additional classic matchups have not been verified against DOS. The model retains known classic behavior and bugs; it is evidence about the decoded system, not a new oracle. Sampling uncertainty is at most about ±2.2 percentage points at 95% confidence for 2,000 trials. An observed 100% is not proof of certainty. P includes the disputed residual roll and is not an accepted design.

The conclusion is not that NC should restore classic's destroyer success. The conclusion is that the same names and prices conceal different games. Keep DD execution as a useful NC role, and test DD 10/8/cost 5 against the chosen damage rules. Do not tune toward this classic odds table.

There is also a willingness mismatch. Those 30 DD have classic strength 60; ten CA have 70. ROE 6 rejects the later strength check even though the classic model strongly favors the DD at ROE 10. Classic's `21 BB + 7 CA + 2 DD` score is not a calibrated win predictor. NC must either publish its two strength yardsticks clearly or deliberately adopt one after updating the encounter contract.

**Partial engagement makes classic's concentration behave differently**

For the larger fleet, let `A` be its total engagement value and `B` the enemy's. Values are DD 1, CA 3, BB 9, base 15. For `A > B > 0`:

```text
r = A/B
y = r^(1/3.3)
if y > 1.3: y = (y+1.3)/2
y = min(y, 2)
budget = floor(B×y + 0.5)
```

Classic activates the shortest prefix of its ship array whose value reaches that budget. BB precede CA, then DD. The final hull can overshoot the budget. Thus `y` approximates the firing-value ratio, not an exact ship count or damage ratio.

| Total value ratio | Approximate firing-value ratio |
| ---: | ---: |
| 1.5:1 | 1.131:1 |
| 2:1 | 1.234:1 |
| 3:1 | 1.348:1 |
| 5:1 | 1.464:1 |
| 10:1 | 1.655:1 |

Thirty CA against ten CA initially fire fourteen cruisers, not all thirty. Extra ships still supply reserves. In the new samples that force wins every trial but loses an average 6.08 cruisers. This is very different from NC firing every gun with an advantageous CRT column.

Do not copy this hidden fractional engagement cap. It makes the last-position starbase fail to fire behind a sufficiently large guard fleet. That punishes reinforcement without an obvious player-facing reason. Use mission coverage, travel time, and valuable separate objectives to create reasons to divide forces. If an explicit command system is later wanted, evaluate it as a distinct design choice.

**Retreat needs both a battle rule and an encounter rule**

The encounter spec's sine draw can be integrated. On the branch where the observer is not faster by maximum speed, define `r = observer maximum speed / target current speed`, with the target moving:

```text
I(r) = integral from 0 to 1 of min(1, r×sin(pi×u)) du
I(r) = 2r/pi                                      when r ≤ 1
I(r) = 1 - (2/pi)asin(1/r)
       + (2r/pi)(1-sqrt(1-1/r²))                  when r > 1

q = (100 - 33×awareness)/100
P(intercept | sighting, willing observer) = q×I(r) + (1-q)×W
```

`W` is 1 if the target's own ROE test accepts the fight, otherwise 0. The separate faster-maximum-speed branch catches automatically. For a stationary target, replace `I(r)` with 1 on the remaining branch; that does not remove the `q` and `W` conditions.

At equal current and maximum speeds, `I(1) = 2/pi ≈ 0.63662`. After a battle, awareness 3 makes `q = 0.01`. A target that refuses has only a 0.6366% catch probability per successful sighting on this branch. A target whose ROE accepts has a 99.6366% catch probability. Preserving its ROE without a separate withdrawal state can therefore undo the retreat.

For illustration, keep the per-tick sighting probability at 35% for seven ticks. A faster pursuer catches at least once with probability `1-0.65^7 = 95.10%`. An equal-speed pursuer against a refusing, already-aware target gets about `1-(1-0.35×0.006366)^7 = 1.55%`. Actual campaigns change position, speed, and awareness, so these are controlled examples.

Keep VBAM's useful decision to leave, adapted to precommitted NC orders. A successful retreat must stop voluntary recommitment while the fleet climbs out. Preserve speed's value through bounded pursuit, not repeated full battles that erase the retreat. The episode and allowance still need the explicit persistence semantics recorded in the earlier consultation. Neither classic's scatter bugs nor a blanket escape guarantee is needed.

**Classic ground combat gives a comparison, not the answer**

Classic draws defender accuracy once per battle from 0.40 through 0.49. For fixed accuracy `pD`, an exchange is:

```text
X ~ Binomial(defenders, pD)
attackers_next = max(0, attackers-X)
Y ~ Binomial(attackers_next, 0.30)
defenders_next = max(0, defenders-Y)
```

For exact odds, solve every declining state recursively. The unchanged-state probability is `(1-pD)^defenders × 0.7^attackers`; divide the sum of the other transitions by one minus that probability. Finally average the ten fixed-accuracy solutions. Substituting 0.445 into each exchange is not the same distribution as drawing accuracy once for the whole battle.

| Attackers versus ten defenders | Classic: exact capture odds | Current NC: exact | Earlier defender-first volley candidate: exact |
| --- | ---: | ---: | ---: |
| 8 | 0.33% | 0% | 0.87% |
| 10 | 3.34% | 8.00% | 15.16% |
| 11 | 7.82% | 83.00% | 34.22% |
| 12 | 15.47% | 86.08% | 56.89% |
| 15 | 54.56% | 100% | 96.57% |
| 20 | 96.56% | 100% | 99.997% |

These start after landing and softening. They do not include transports or batteries. Enumeration uses ideal uniform draws and floating-point probabilities, not the binary's full PRNG sequence. A separate 20,000-run check through the retained classic ground model gave 697 captures, 3.485%, consistent with 3.340%.

A rough deterministic approximation helps explain classic's strong defense. Ignoring integer losses and replacing each volley by its mean gives a break-even initial ratio:

```text
r_critical = (pD + sqrt(pD² + 4pD/pA))/2
at pD = 0.445 and pA = 0.30: r_critical ≈ 1.461
```

This is a large-force approximation, not a promised 50% point. Equal armies become less likely to capture as scale rises: classic gives 9.42% at 5:5, 3.34% at 10:10, and 0.56% at 20:20.

Do not replace NC's broken cliff by copying classic's defender advantage. The earlier volley candidate is much more favorable to invasions: 1.5:1 gives about 97%, versus classic's 55%. That is a consequential choice still needing calibration. Choose the intended risk, then tune it. Ground can use EotS/VBAM-inspired strength and damage rules without preserving classic's first fire or per-army dice.

**Mission rewards matter as much as casualties**

Classic's capture formulas give Blitz an economic prize. Let `P` and `G` be factories and goods immediately before capture damage, `L` the surviving attackers after ground combat, and `D0` the defending armies before the assault. For `D0 > 0`, ignoring the factory floor and integer rounding:

```text
Invade: E[P_after | capture, P] = 0.2995 P
        E[G_after | capture, G] = 0.2995 G, when G > 1

Blitz: f = min(1, sqrt(L/D0))
       E[P_after | capture, P, L, D0] = 0.9245 f P
       E[G_after | capture, G, L, D0] = 0.7745 f G
```

Those are capture-stage multipliers. Prior collateral and invasion softening can already have reduced the world. The denominator is the original defender, not the invading army. With ten initial defenders and ten surviving attackers, `f = 1`, even if the attacker began with a hundred armies. With five survivors it is about 0.707, preserving an average 65.37% of pre-capture factories. Describing this merely as the attacker's survival percentage is misleading.

Scorch-Earth has already cut factories to an average 39.5% in the pre-tick economy pass, then halves them again on capture. Ignoring other damage and floors, an invaded flagged world's expected factories are `0.395 × 0.2995 × 0.5 ≈ 5.92%` of the starting value. The goods pass is `min(G/10, sqrt(G))`, not always simply a tenth. The capture report's claim that defenders destroyed useful assets appears even without the Scorch order. These details explain why classic could feel arbitrary and unrewarding.

NC should not inherit those penalties. Its current invasion intentionally preserves production. More importantly, NC already has specific Blitz benefits: it can land without clearing every battery, allows a transport-only assault, and captures surviving batteries intact. That last reward is explicit in [assault.rs](../../../rust/nc-engine/src/maint/combat/assault.rs). Four surviving batteries represent 80 build points in addition to the world. An unescorted TT force also travels at speed 5; adding a BB reduces the whole fleet to 4. These are useful risks and rewards to measure before inventing a new capture penalty.

Compare missions using:

```text
expected net value = P(capture) × E(value retained | capture)
                     - E(destroyed build value + repair bill)
                     - value of delay
```

Include embarked armies in destroyed value, captured batteries in the prize, and the actual arrival year. Also report catastrophic loss probability; equal expected value can conceal very different experiences. Keep the monetary comparison alongside strategic objectives, since a vital position can justify a costly victory.

There is a further clarity trap in the classic Blitz description. Forty percent of ordinary new target selections explicitly choose transports; the other sixty percent can choose them too. With `T` transports among `N` eligible ships through the last transport index:

```text
P(transport | fresh non-focus selection) = 0.4 + 0.6×T/N
```

Twenty transports behind five warships give 88%, not 40%. With transports alone it is 100%. Focus, persistent targets, casualties, and bug B4 affect later selections. NC should state its own transports-first allocation directly rather than describe it through classic's percentages.

**Corrections the new reference needs**

The reference is valuable, but a summary must not turn conditional rules or intended behavior into unconditional facts.

- **Strike timing is still not implemented as stated.** t048 now places builds and economy before the tick year. `maint/mod.rs` still calls `process_planetary_assaults` after `encounter::run_year`. The reference's claim that NC already strikes on the same tick as classic is premature. Its timing fix remains necessary.
- **NC Blitz cover fire is omitted.** The code removes batteries with cover fire before the survivors attack transports. The summary should include that ordering. An orbital gate described as a requirement should also be distinguished from the current implementation's incomplete supremacy selection.
- **A stationary target is not unconditionally caught.** The faster-observer branch is unconditional. Otherwise the awareness gate and the target's willingness still matter, as shown above.
- **The Blitz retention denominator needs a name.** It is initial defending armies. Likewise, forty percent is a targeted selection branch, not the eventual share of shots at transports.
- **Diplomacy has an exception.** Ordinary combat leaves diplomacy unchanged, but the same summary describes Rogues conversion resetting that empire's diplomacy. The conversion tests the world's owner after the capture step, not automatically its former owner. Keep those conditions visible.
- **Bug B3 has a model/spec discrepancy.** The spec says negative Blitz damage inflates collateral by 65,535 and gives a battery's roll of 1 against a BB as its reachable example. In retained `sim.py`, that shot heals the BB by one but contributes zero to planetary damage: the accumulator updates only for fire against side 2. A forced-roll check produced BB HP 30 to 31 and damage contribution 0. This identifies an inconsistency requiring binary or probe verification; it does not establish which description is correct. Do not cite the collateral explosion as validated by that model.

The statement that NC inherits everything before the first shot and nothing afterwards is also too broad. Survivor speed, strike timing after an orbital battle, intelligence updates, and ownership consequences cross that boundary. State the retained contracts explicitly. A player must not receive later knowledge retroactively or see a relief fleet alter a battle that already happened earlier in the year.

**Consequences for the existing recommendations**

The seven recommendations in the earlier review still stand as candidates: same-empire joined participation, flat effectiveness, per-round withdrawal with bounded pursuit, DD/base execution, dangerous batteries, conditional auxiliary protection, and bounded surprise. Classic supplies no reason to restore ratio columns, a three-round lock, hidden firing limits, automatic healing, or irreversible losses caused by unclear orders.

The strongest additions are to evaluate a coherent VBAM durability alternative, include captured assets and travel time in assault comparisons, and make the encounter strength test part of the tuning problem. Keep reports and Appendix B alongside the chosen rules. Players should see the effectiveness table, allocation costs, retreat threshold, and what surprise changes, with worked examples and odds for representative forces. They should not need to understand classic's formulas to play NC well.

Priority remains: finish strike timing and the orbital gate; fix survivor speed and retreat routing; replace the ground cliff; make withdrawal and damage persist; then choose resolution and hull values through controlled sweeps. Participation changes and surprise follow the state needed to support them. The new classic reference is evidence for doing that work carefully, not a reason to change NC back into classic.

**Validation**

The new tool retains exact formula values, eleven enumerated ground cases, and seven seeded fleet cases with raw outcome counts. Every fleet case accounted for all 2,000 trials; none hit an unresolved outcome. The ground solver matches the closed-form one-army result and absorbing states. Its 10:10 result was independently checked through the retained classic model as described above. The tool reuses the existing classic simulator rather than maintaining a second copy. No engine changes, cargo runs, or claims of new DOS conformance accompany this note.
