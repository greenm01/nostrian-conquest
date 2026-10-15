---
id: kvqg7wv5
date: 2026-10-14
kind: plan
tags: [plan, milestone, ai]
---
# Executor doctrine follow-ups from strategy research

## Scope and exit

The scripted executor is the learned strategist's teacher and its hands
(t154, t155). Its fixed rules hold what the strategist cannot change: how
fleets attack, scout and blockade, and what each world builds. These tasks
teach it the remaining lessons from the
[strategy doctrine investigation](../investigations/v6jegy7f-strategy-doctrine-for-ai-seats-from-the-manuals-vbam-and-owner-guidance.md).
The sources are the owner's guidance, the Nostrian Conquest player manual
and quick start, VBAM, and Space Empires 4X. The land-grab build order is
already in.

Each lesson is its own task so its effect is measured alone. A change is
kept only when it wins.

**Exit for each task (t160–t164):**

- With t159's doctrine switches, an executor with the change plays one
  seat against three executors without it, over 1,024 seat-rotated maps
  from map 100000, 60 years. Both use `Weights::TUNED`.
- The changed executor wins at least 0.28, against an even 0.25. Report the
  production share and, for scale, the result over the first and second
  halves of the maps.
- If it wins, keep it, re-tune the weights, and record the result here.
  If it loses or ties, remove it and record why.
- `nc-ai` tests pass. The 1,000-game random action test still submits no
  rejected order.

**Exit for the plan:** every candidate is kept or rejected with evidence.
The strategist (t155) is then retrained on the final executor and
evaluated on 1,024 maps against it.

Lessons about tax, war timing and which empires to fight are left to the
strategist network, which chooses them itself. They are not written into
the executor.

**Principle (owner, 2026-10-15): risk, reward, return.** Each executor
rule should weigh expected gain against expected cost, not a bare
threshold:

- **Strikes:** the chance of taking a world, from the manual's
  ground-battle odds, times its value, against the warships, transports
  and armies expected to be lost.
- **Attack missions (t168):** time and the intact prize, against losses
  under battery fire.
- **Defence (t169, t175):** the losses expected from the threat seen,
  against the conquest the held-back ships would buy.
- **Ships posted for intelligence (t174):** what the picket costs against
  the warning it gives.

t160's value-per-defence targeting is the first such rule, and it was
among the largest gains. The learned agents optimise the final return,
winning, directly; this principle governs the hand-written rules they
start from and carry out.

**Correction to the evaluation method (2026-10-15).** Scope's test (one
candidate seat against three executors on 1,024 maps, judged against an
even 0.25) is too noisy for small effects:

- **Calibration.** With no switch at all, identical executors score 0.215
  and 0.246 on the two map blocks, so a block reading carries about ±0.03
  of map luck.
- **The paired test.** `executor_matches --paired` plays every map twice,
  the candidate seat with and without the switch against the same
  opponents, and reports the mean difference with its standard error. Map
  luck cancels. On 4,096 maps the standard error is about 0.009, or less
  when the switch rarely changes play.
- **What changes.** A switch is kept when its paired difference is clearly
  positive, about two standard errors or more, and confirmed on a second
  block of maps.
- **What stands.** The large results (t159's land grab, t160 and t161
  gains, t163's loss) are far outside the noise. t164's scaled garrisons
  was wrongly rejected and is now kept; see t164.

## Task details

### t159

Make doctrine changes comparable head to head.

The old and new build orders could not meet in one game, so the land-grab
change was judged only through a retrained strategist. Add a `Doctrine` set
of switches beside `Weights`, one per behaviour that t160–t164 add.

- `SeatPolicy::Executor` and strategist seats carry it.
- `Doctrine::CURRENT` holds what has been accepted.
- The `executor_matches` example and `evaluate.py` take a doctrine for
  the candidate seat and one for the opponents.
- The switches are not tuning weights. Remove a switch once its change is
  accepted or rejected, so they do not pile up.

**Exit:**

- With every switch at its current setting, `play_match` results equal
  those recorded before the change, as the goals refactor checked.
- A test shows a switch changes play.

**Result (2026-10-14):**

- **The design.** `Doctrine` lives inside `Weights` as a set of switches.
  Tuning ignores it: `from_array` sets `Doctrine::CURRENT`. So every
  executor seat, scripted or strategist, carries one with no new
  parameters.
- **The first switch, `land_grab`.** Off, it restores t154's build order
  (one build a year, garrison first), the warship ROE for colony fleets,
  and no pickets. `Weights::CLASSIC` is t154's tuned weights with the
  switch off.
- **Fidelity.**
  - With the switch off, all 96 `play_match` results recorded before the
    goals refactor replay exactly.
  - With it on, 48 results match the previous commit exactly.
  - `the_classic_doctrine_replays_t154_exactly` keeps two of the recorded
    results, and `the_land_grab_switch_changes_play` shows the switch
    matters.
- **The land-grab doctrine against the classic one.**
  `executor_matches --weights tuned --opponent classic`, 1,024 maps from
  100000, 60 years:

  | One seat vs three | Win rate | Production share |
  | --- | ---: | ---: |
  | Land-grab vs classic | 0.282 | 0.260 |
  | Classic vs land-grab | 0.250 | 0.247 |

  The land-grab doctrine is ahead, but only just. It meets the 0.28 bar by
  about two standard errors, and the classic seat holds even in reverse.
  Each set of weights was tuned against copies of itself. The tuned
  land-grab weights also declare war at year 3, so early ETACs compete
  with early strikes for the same budget. The doctrine is kept. Remove the
  switch and `Weights::CLASSIC` once t160–t164 no longer need the
  comparison.

### t160

Choose strike targets by value over estimated defence.

Today a home force strikes the most valuable world it can take, discounted
by distance. Space Empires 4X players agree you should strike where the
enemy is weakest and count losses against gains. Score candidate worlds by
value divided by estimated defence, still discounted by distance, so cheap
conquests come first.

**Exit:** as in Scope.

**Result (2026-10-14): accepted.**

- **The switch.** `Doctrine::strike_weakest` divides a target's value by
  its estimated defence, at least 1. Feasibility still requires
  `attack_ratio` times that defence.
- **Labels stay exact.** Every home force scores from the same sector with
  the same function, so the strongest force's pick still decides the
  strike label.
- **Head to head.** `executor_matches --weights tuned --with
  strike_weakest --opponent tuned`, 60 years, one candidate seat against
  three executors without the switch:

  | Maps | Win rate | Production share |
  | --- | ---: | ---: |
  | 100000–101023 | 0.341 | 0.280 |
  | 200000–201023 | 0.344 | 0.281 |

  That is about nine points over an even seat on both blocks, far more
  than the land-grab change's three.
- **Now current.** `Doctrine::CURRENT` turns it on. `Weights::CLASSIC`
  keeps it off, so t154 still replays exactly.
  `the_strike_weakest_switch_changes_play` shows the switch matters.
- **Re-tuned.** `tune_executor` ran 40 generations of 32, about 7 minutes,
  with 29 champion changes. The new weights:
  - strike at 2.1 times a target's defence (was 2.9);
  - ROE 2.9 (was 9.9);
  - declare war at year 10 (was 3);
  - build no battleships, preferring cruisers;
  - keep 9.4 batteries and 6.3 ETACs, and tax 64%.

  They attack more often, but only where the target is cheap, and avoid
  even fights.
  - Head to head on 1,024 maps: one new-weights seat wins 0.289 against
    three of the previous weights; one previous-weights seat wins 0.198
    against three new.
  - They became `Weights::TUNED`. On 256 held-out maps:

  | One seat vs three | Win rate | Production share |
  | --- | ---: | ---: |
  | Tuned vs baseline | 0.717 | 0.371 |
  | Baseline vs tuned | 0.000 | 0.064 |
  | Tuned vs Aggressor | 1.000 | 0.914 |
  | Tuned vs Autopilot | 1.000 | 0.892 |
  | Tuned vs classic (t154) | 0.316 | 0.277 |
  | Classic vs tuned | 0.184 | 0.222 |

  The last two rows are the cumulative gain since t154. The strategist
  (t155) was trained on the land-grab executor; retrain it when t161–t164
  are done.

### t161

Bombard a target's batteries down before invading.

The quick start says surviving batteries block a landing and "equal numbers
are a poor gamble". VBAM agrees that bombardment should suppress, not wreck.
Today the executor invades whenever it carries enough armies, and bombards
otherwise.

Change it so that:

- while the target's known batteries stand, the force bombards;
- once intel shows no batteries, it invades with its margin of armies;
- a world it means to take is never bombarded beyond that.

**Exit:** as in Scope, plus a test that no invasion starts while the
seat's intel shows batteries.

**Result (2026-10-14): accepted in a revised form.**

- **As planned, it failed badly.** "Bombard until intel shows no
  batteries, then invade" won 0.107 and 0.112 on two blocks of 1,024 maps.
  A probe of six 60-year games found why. In 9,426 of about 10,200
  bombarding fleet-years at a target, the seat's intel did not know the
  target's batteries at all, so the fleet never left. One stayed 39 years.
  This is the delay the owner warned of under t162.
- **Revised: bombard on arrival, invade the next year.** A force that
  arrives while the target's batteries may stand bombards for one year.
  The next year it invades from the target if it carries enough armies,
  otherwise it goes home. It invades straight from home only when the
  seat knows the batteries are gone. A strategist's hold stops these
  invasions too.
  - Head to head, one seat with the revised switch against three without,
    1,024 maps each: **0.493** (from 100000) and **0.528** (from 200000).
    Production shares were 0.342 and 0.346.
  - This is the largest gain so far. Before, a force bombarded once and
    went home, or invaded straight into standing batteries. The manual
    says bombardment wears batteries down, and round 3 reaches the armies
    once they fall.
  - `suppress_first_invades_only_after_bombarding` checks every invasion
    follows a year of bombardment there, or known-gone batteries.
- **Re-tuned:** 40 generations, 30 champion changes. The weights adapt to
  bombarding first:
  - strike at 0.75 times a target's estimated defence (was 2.1);
  - colony garrisons of 3.3 armies, and 6.4 batteries;
  - war at year 12;
  - almost no scouts. Destroyer pickets do the early looking; t162
    revisits scouting.

  Head to head on 1,024 maps, one new-weights seat wins 0.270 against
  three of the previous weights, which win 0.171 in reverse. They became
  `Weights::TUNED`. On 256 held-out maps:

  | One seat vs three | Win rate | Production share |
  | --- | ---: | ---: |
  | Tuned vs baseline | 0.770 | 0.505 |
  | Baseline vs tuned | 0.008 | 0.061 |
  | Tuned vs Aggressor | 1.000 | 0.958 |
  | Tuned vs Autopilot | 1.000 | 0.963 |
  | Tuned vs classic (t154) | 0.508 | 0.372 |
  | Classic vs tuned | 0.051 | 0.136 |

- **Open question for the seat view (t151).** The bombarding empire's
  memory rarely learns the target's battery count from its own attacks.
  Check whether a human player's bombardment reports show remaining
  batteries. If they do, the seat memory is missing that intel.

### t162

Scout toward rivals, and weigh stale intel when striking.

Space Empires 4X: "focus early exploration on your front row". VBAM and
the quick start: scout before you strike. Today scouts and pickets go to the
world the seat knows least about, and a strike reads the target's last
known defence however old it is.

Scouting first must not become a rule. Waiting for fresh intel before every
strike would cost a year or more each time and hold back an AI that tuning
shows should be aggressive: war at year 3, strikes at 2.9 times defence,
ROE 10. The executor assesses the risk instead:

- **Front-row scouting.** Weight scout and picket targets toward worlds
  between the seat and known rival worlds.
- **Age-adjusted defence.** Grow the target's estimated defence with the
  age of its intel: what the world could have built since it was seen, from
  its known production. A strike on old intel needs a bigger margin; it is
  never refused only because the intel is old.
- **Strike and scout together.** When the force strikes on intel older
  than a year, send a scout or picket to the target in the same year. Its
  report arrives for the next decision; the strike does not wait for it.
- **Hold only when it is marginal.** The force waits a year for intel only
  when its strength falls between the fresh and the age-adjusted estimates
  and the intel is older than a threshold. The threshold is a tuning
  weight, so evolution, and through goals the strategist, decide how bold
  to be.

**Exit:** as in Scope, plus a check that the change does not slow
aggression. Over the same 1,024 games, record the year of each seat's first
strike and the share of strikes delayed for intel. The median first strike
must be no later than without the change.

**Result (2026-10-14): front-row scouting rejected; the stale-intel half
not tried.**

- **The switch tried.** `front_scouting` did three things:
  - lone destroyers picket all game, not only during the land grab;
  - worlds within 4 sectors of a known rival world count double as
    scouting targets;
  - the first free scout or picket views the next strike's target when
    its intel is more than a year old. The strike never waits.
- **With the current weights it lost:** 0.212 and 0.237 on two blocks of
  1,024 maps. Pickets that used to join the strike force went scouting
  instead.
- **Fairer test, tuned with the switch on.** The tuner found a rush:
  - about 1 ETAC, no scouts, no starbase;
  - war at year 1;
  - strikes at 0.61 times a target's estimated defence.

  It beat the current executor, 0.290 and 0.289, with 0.221 and 0.228 in
  reverse. The gain was not the switch's, though. With the rush weights,
  the switch on against off won only 0.252 and 0.233. The rush weights
  with the switch off still beat the current executor, 0.266 and 0.295.
- **Removed,** per the Scope rule.
- **The rush is not adopted (owner, 2026-10-14).** The executor keeps the
  land grab: expand and scout first. The rush is recorded as a finding.
  The strategist sets its own war timing and strike goals, so a learned
  seat may still find a balance between expanding and rushing.
- **The stale-intel half was not tried.** The tuned executors build no
  scouts, so better intel has little to work with.

### t163

Blockade worlds the strike force can hold but not yet take.

VBAM: blockading "is an effective way to deny resources ... even if you do
not have the troop strength to try and capture". Today a force without
enough armies waits at home or bombards.

When the force beats a target's known fleets and batteries but has too few
armies to invade, order Guard/Blockade at the target. Keep the rest of the
force gathering at home.

**Exit:** as in Scope.

**Result (2026-10-14): rejected.**

- **The switch tried.** `blockade` ordered a force that had bombarded a
  world but lacked the armies to invade to Guard/Blockade it, instead of
  going home. A force already blockading stayed while the world was still
  the enemy's.
- **Head to head:** one seat with the switch against three without won
  0.092 and 0.104 on two blocks of 1,024 maps, against an even 0.25.
- **Why.** As with t161's first version, the strike force stayed out
  indefinitely. It never brought troops home to reload, and its own worlds
  lay open. A blockade for a fixed year would only repeat t161's
  bombard-then-leave, so no variant was tried. The code is removed.

### t164

Avoid contested colonies, and scale colony garrisons to production.

- **Contested colonies.** VBAM loses both colony fleets when two empires
  colonize one system. NC's ETAC survives, but the trip is wasted. Skip a
  colony target where the seat's contacts show a rival ETAC fleet heading
  there or already present.
- **Garrisons.** VBAM: "a number of troops equal to the system's Population
  is usually sufficient". Scale each colony's army target to its current
  production, not a flat count, within the weights' bounds.

Measure the two changes separately: they are separate switches.

**Exit:** as in Scope, for each switch.

**Result (2026-10-14): both switches rejected.**

- **`avoid_contested`.** Skip a colony target where a rival fleet was seen
  within a sector in the last two years. Contacts give size classes, not
  ship types, so a rival ETAC cannot be told apart. With the current
  weights it won 0.219 and 0.257 on two blocks of 1,024 maps: no gain.
- **`scaled_garrisons`.** A colony's army target is `colony_armies` per 25
  points of production, at least one.
  - With the current weights: 0.263 and 0.260.
  - Tuned with the switch on, then the same weights with the switch on
    against off: 0.279 and 0.248. That is inconsistent and below the bar.
- **The rush again.** That tuning run found a rush too: war at year 1.9,
  about 3 ETACs, strikes at 0.67 times defence. With the switch off, those
  weights beat the current land-grab executor 0.326 and 0.309, more than
  the first rush's 0.28. Two independent tunings now favour early war
  over long expansion.
- Both switches' code is removed.

**Revised (2026-10-15) by the paired test:** scaled garrisons is kept.

| Switch | 4,096 paired maps from 100000 | Second block, 200000 |
| --- | ---: | ---: |
| Scaled garrisons on | +0.037 ± 0.009 | Flat garrisons cost −0.018 ± 0.009 |
| Avoid contested colonies | −0.000 ± 0.004 | |

Scaled garrisons is now in `Doctrine::CURRENT`. Avoid contested colonies
stays rejected.

**Re-tuned with scaled garrisons.** 40 generations. The champion:

- taxes 65%;
- keeps 20 armies at home, 10 per 25 production at colonies, and 9.7
  batteries;
- keeps about 4 ETACs and 1.7 starbases;
- prefers cruisers and battleships;
- strikes at 2.43 times defence, with ROE 6.3;
- declares war at year 21.

It is a steady land grab with strong defence, the owner's doctrine, and
unlike the rushes earlier tunings found.

- **Paired against the previous weights,** 4,096 maps per block: +0.168
  ± 0.010 (from 100000) and +0.147 ± 0.010 (from 200000). One seat with
  the new weights wins 0.42 where the old ones won 0.255. They became
  `Weights::TUNED`.
- **Standard checks,** 256 held-out maps:

  | One seat vs three | Win rate | Production share |
  | --- | ---: | ---: |
  | Tuned vs baseline | 0.770 | 0.494 |
  | Baseline vs tuned | 0.047 | 0.108 |
  | Tuned vs Aggressor | 1.000 | 0.991 |
  | Tuned vs Autopilot | 1.000 | 0.990 |
  | Tuned vs classic (t154) | 0.852 | 0.535 |
  | Classic vs tuned | 0.004 | 0.063 |

  Today's executor wins 85% of its games against three of t154's.

**Plan outcome.** Of t159–t164:

- Kept: the land grab (t159), striking where weakest (t160), and
  bombarding then invading (t161). Together, the current executor beats
  t154's 0.508, and t154's wins 0.051 in reverse.
- Rejected: front-row scouting, blockades, contested-colony avoidance and
  scaled garrisons.
- **The strategist retrained on the final executor.**
  - The executor's own goals win 0.250 against three final executors on
    1,024 maps.
  - Imitation (40 rounds) ended at strike 0.78, colonize 0.94 and
    diplomacy 0.95. Greedy, the student won 0.215.
  - PPO with entropy 0.001 crashed at about iteration 230. MPS ran out of
    memory, with 25 GiB of live tensors: late-game seats pad to long
    contact lists. The fix: minibatches of 512, and the MPS cache released
    after each update. A continuation from iteration 220 ran 200 more
    iterations.
  - On 1,024 maps against three final executors, sampled:

    | Checkpoint | Win share | Production share |
    | --- | ---: | ---: |
    | Iteration 60 | 0.371 | |
    | Iteration 100 | 0.436 | |
    | Iteration 140 | 0.419 | |
    | Iteration 180 | 0.472 | |
    | Iteration 220 | 0.511 | |
    | Continuation 80 (300 overall) | 0.496 | 0.388 |
    | Continuation 140 | 0.481 | 0.383 |
    | Continuation 200 (420 overall) | 0.504 | 0.389 |

    Greedy play matched sampled play (0.499 at 220).
  - The strategist levels off at about 0.50, twice an even seat, against
    an executor that itself beats t154's 0.508. The final checkpoint wins
    1.000 against Aggressor, with 0.996 of production. It is kept on the
    mini as `~/.local/state/nostrian-conquest/ai/strategist-final.pt`.

### t168

Choose between bombarding, invading and blitzing by the target's defences.

The owner (2026-10-15) wants the AI to know when to blitz and when to
invade. The player manual's attack doctrine ("Match the attack to the
prize"):

- **Invade** takes a world intact, but only once its batteries are down;
  if any survive the first round, no troops land.
- **Blitz** lands at once under the surviving batteries' fire, which picks
  off transports, but captures factories, goods and surviving batteries.
  It is for lightly defended worlds, or when speed matters more than
  losses.
- **Bombard** breaks batteries, but destroys part of the prize.
- **The ground battle:** equal armies is a poor bet, 1.5 times the
  defenders is about even, and twice the defenders wins nine times in ten.

Since t161 the executor only bombards, then invades. It loses a year on
lightly defended worlds a blitz would take at once.

The switch `blitz_light`: blitz when the target's known batteries are few
enough for the escort to suppress, and the force carries a wider margin
than an invasion needs, because transports are lost on the way down.
Otherwise bombard, then invade, as now. The thresholds are tuning weights,
or are measured at a few settings.

The fleet-level agent (t157) may choose Blitz itself. Report how often its
strike orders use Bombard, Invade and Blitz, and the captures that follow,
from the recorded turns of its evaluation games.

**Exit:** as in Scope, plus the counts of each attack mission and the
captures that follow, with and without the switch.

**Result (2026-10-15): the executor rule is rejected.**

- **The switch tried.** `blitz_light` blitzed at once when a target's
  known batteries were at most two and the force carried a wide margin of
  armies: three times the defenders plus two, then, as a looser variant,
  the invasion margin plus one per battery.
- **It almost never fires.** Over 128 executor games: about 35,600
  bombard and 5,950 invade orders, but only 14 to 18 blitzes. 96% of
  assaults (5,157 of 5,368) already capture their world, because the
  executor bombards the batteries down first.
- **No effect.** On 4,096 paired maps the difference is −0.000 ± 0.001.
- **Choosing to blitz is left to the fleet-level agent,** which can order
  Blitz itself. t157 reports how often it does.
- The code is removed.
### Fleet defence, blockades, hunting and pickets (t169–t175)

The owner (2026-10-15) asked whether the AI defends its worlds with fleets
and blockades enemy worlds. It does not yet:

- It defends with garrisons (armies and batteries), a home starbase, and
  warships that happen to gather at home between strikes.
- No fleet is ever posted on Guard/Blockade. The manual's blockade watches
  five times as hard and attacks any fleet that comes to bombard, invade
  or blitz.
- Guard a Starbase is not in the action space: it needs a starbase target
  (t152).
- t163 rejected blockading with the whole strike force.
- The action space has no Detach, so a force cannot be split.

Each task below is a switch measured as in Scope. The fleet-level agent
(t157) can already choose Guard/Blockade itself; t157's reports note
whether it does.

**Principle (owner, 2026-10-15): fleets serve both offence and defence.**
The executor puts every warship in one strike force and defends with
garrisons alone. t169–t174 give fleets defensive roles: guards, a home
guard, pickets and hunters. t175 decides how warships are shared between
those roles and the strike force, year by year.

### t169

Guard threatened worlds, and keep a home guard.

- **Threatened colonies.** When the seat's contacts show a rival fleet
  within a few sectors of a colony, that colony builds a warship and posts
  it on Guard/Blockade there. Without Detach (t172), a guard fleet has to
  be commissioned where it is needed.
- **A home guard.** When the strike force leaves home, a guard stays
  behind on Guard/Blockade. Until t172, that means warships built after the
  strike force sailed.

**Exit:** as in Scope. Also report how often a guarded world is attacked,
and how often it falls, with and without the switch.

**Result (2026-10-15): rejected.** A `guard` switch posted new warships on
Guard a Starbase or Guard/Blockade: at a colony with a foreign contact
within 4 sectors in the last 2 years, and at home once the strike force
had sailed. Each version was measured by the paired test against the tuned
executor (4,096 games a block unless noted).

| Version | Block 100000 | Block 200000 |
| --- | --- | --- |
| First cut, any warship fleet takes a post | −0.150 ± 0.008 | |
| Home guard only (2,048 games) | −0.003 ± 0.012 | |
| Colony guards only (2,048 games) | −0.167 ± 0.010 | |
| Colony guards, no guard build (2,048 games) | −0.144 ± 0.011 | |
| Guards of one ship, built alone | −0.013 ± 0.008 | −0.020 ± 0.008 |
| Same, threats only from declared enemies | −0.006 ± 0.008 | −0.022 ± 0.008 |

The colony guard was the cost. A colony's hulls for a year commission as
one fleet, and posting it as a guard kept about 23% of the seat's warship
strength at home worlds. Losses fell only from 1,037 to 884 worlds over 128
games, while the strike force shrank about 30%. Capping a guard at one
battleship's worth, built alone, made the cost small but not a gain.

Guards do protect their own world. Over 128 games, a one-ship guard's
world fell in 28 of 794 attacks (3.5%), and an unguarded world fell in 991
of 3,040 (33%). Yet total losses barely moved (1,019 against 1,037),
because attackers turned to unguarded worlds. The executor's attackers
judge a world by its ground defences and do not see guards. A guard pays
only when it covers the worlds an enemy can actually reach. That
judgement is t175's, by threat, opportunity and state, so the rule is not
retried here. A home guard, once only new fleets of one ship qualified,
rarely formed, because home commissions are large.

### t170

Offer Guard a Starbase.

Add Guard a Starbase to the seat actions, with a starbase target: the
seat's own starbases, from its own records. The masks follow
`validate_fleet_order_payload`, and decoding builds the order directly, as
for world targets.

- **The executor:** a home guard (t169) guards the home starbase when it
  has one.
- **The network:** gets a pointer over the seat's own starbase rows.

**Exit:**

- t152's random-play test still submits no rejected order, with Guard a
  Starbase among the choices.
- The masks reveal nothing about other empires.
- The executor change is measured as in Scope.

**Result (2026-10-15): accepted.** Guard a Starbase is the fifteenth
mission. Its target is a world sector, as for the other missions, and the
mask allows only worlds where one of the seat's own starbases sits; the
order names that base. No separate pointer over starbase rows was needed,
because a starbase always sits at a world. The mask uses only the seat's
own bases, and the mask leak test still passes. Over 1,000 random games
(30,000 game-years), random play gave 1,176,041 fleet orders, 51,621 of
them Guard a Starbase, and none was rejected. The executor change moves to
t169's home guard. Mission choices go from 15 to 16, so fleet-agent
checkpoints from before this change no longer load; t157 retrains on the
final executor anyway.

### t171

A dedicated blockader.

t163 failed because the whole strike force sat on a blockade. Here a
separate warship fleet blockades a rival world the strike force cannot yet
take, chosen by value per estimated defence, while the strike force goes on
reloading and striking. The blockader comes home when its target falls,
or when the seat's home is threatened.

**Exit:** as in Scope. Also report the production denied to the
blockaded worlds.

**Result (2026-10-15): accepted.** The `blockader` switch works as
follows:

- **Who blockades.** Once war is declared, a new warship fleet at home
  (on Hold, without transports) that cannot strike anything takes the
  post, if no fleet of the seat already blockades.
- **The target.** It goes on Guard/Blockade to the enemy world with the
  most value per unit of estimated defence, among worlds no fleet of the
  seat strikes. The fleet must be `attack_ratio` times the world's
  batteries and starbases.
- **Coming home.** It comes home when the world is no longer a declared
  enemy's, or when a declared enemy's force is reported within 4 sectors
  of home in the last 2 years.

Paired against the tuned executor (4,096 games a block), it scored
+0.016 ± 0.007 on block 100000 and +0.013 ± 0.006 on block 200000. It is
now in `Doctrine::CURRENT`. A re-tune with the switch on found weights
that lost to the current ones by −0.029 ± 0.009 and −0.030 ± 0.009 on the
two blocks, so `Weights::TUNED` is unchanged.

Over 96 games, a blockader formed in 73 and stood on station for 506
fleet-years in all. A blockader averaged strength 9.9, about three
cruisers. The blockaded worlds averaged 51.4 production a year.

**Production denied: none directly.** In this engine a blockade does not
cut a world's production (player manual, *Deny what you cannot take* and
*Blockades*); only bombardment grinds it down. The blockade's gain comes
from what it intercepts: fleets that come to attack or colonize the
world, and a declared enemy's fleets as they arrive or climb out of
orbit, including ships newly commissioned there.

A dedicated raider that bombards instead was not tried. The executor
cannot yet tell such a raider from a strike force bombarding without
troops, which goes home after one year (t161).

### t172

Offer Detach.

Add a per-fleet choice to detach part of a fleet, by kind and share: for
example half its warships, or its transports. The masks follow the detach
validators; decoding builds the detach and orders the new fleet in the
same year where the rules allow, or the next. This changes the action
arrays, so learned seats are retrained afterwards.

- **The executor:** splits guards (t169) and a blockader (t171) off the
  home force instead of building them.

**Exit:**

- t152's random-play test still submits no rejected order, with Detach
  among the choices.
- A replay test covers detached fleets.
- The executor's use of Detach is measured as in Scope.

**Result (2026-10-15): Detach accepted in the seat actions; the
executor's split rejected.** Each fleet now has a detach choice: keep
whole, half of each kind of warship (rounded down), one warship of the
lightest kind, or every transport. The mask allows a part that exists and
leaves the donor a ship. Detaches are decoded after every other fleet
block, because a detach can re-sort the empire's fleet records, and each
donor is found again by its fleet number. The detached ships wait on Hold,
with the donor's rules of engagement, and take orders the next year. The
donor's mission applies to the ships it keeps.

- Over 1,000 random games (30,000 game-years), random play detached 39,283
  times, and no order was rejected.
- A new replay test plays random masked turns with detaches through the
  batch environment and rebuilds both games exactly from their turn KDL.
- The Python fleet agent has a detach head, so fleet-agent checkpoints
  from before this change no longer load.

The executor's guards were rejected in t169, so its use of Detach was
tried as a split instead. When half the home force's warships still met
`attack_ratio` against the target, the other half stayed home and struck
on its own the next year. Paired against the tuned executor on block
100000 (4,096 games), the split scored −0.010 ± 0.008. When the half left
at home also skipped worlds another fleet already struck, it scored
−0.035 ± 0.008. A concentrated force beats a split one here, so the
executor does not detach.

### t173

Hunt and intercept enemy fleets.

The owner (2026-10-15) asked whether the AI attacks enemy fleets, not
just worlds. Ships fight when they meet in a sector, each fleet's ROE
deciding whether it engages. Per the manual:

- Patrol a Sector watches ten times as hard as an ordinary fleet and
  attacks what the rules allow, as far as its ROE permits.
- A faster fleet always catches its target; destroyers suit interception.

Today the executor never hunts fleets. Its warships gather at home and
strike worlds. The action space targets only world sectors, so no
patrol line or intercept point in deep space can be ordered. Contacts are
remembered for five years, mostly as size classes; battle reports give
ship types.

The work:

- **Deep-space targets.** Add them for Move and Patrol a Sector in the
  seat actions, so the executor and the network can patrol between
  worlds and intercept on routes. The masks follow
  `validate_fleet_order_payload`.
- **A hunter group.** Fast warships, detached (t172) or built for it,
  patrol where rival fleets were seen recently, or the line to rival
  worlds. ROE is set from the estimated odds against the contacts' size.
- **Answer raiders.** When contacts show rival fleets near the seat's
  worlds, hunters move to intercept rather than waiting for an attack.
  This works alongside t169's guards.

**Exit:**

- t152's random-play test passes with deep-space targets.
- The executor change is measured as in Scope.
- Report the fleet battles fought and won, and the warships lost, with and
  without the switch.

**Result (2026-10-15): deep-space targets accepted; the executor's hunting
rejected.**

**Deep-space targets.** Each fleet now has a waypoint choice. For the
sector missions (Move, Patrol a Sector, Scout a Sector and Rendezvous),
the target sector lies all the way, three quarters, half or a quarter of
the way from the fleet to its target world, rounded to the nearest sector.
Other missions ignore it.

- **Why waypoints.** The network keeps pointing at world tokens and needs
  no per-sector target array. Patrol lines and intercept points still lie
  on the routes between worlds.
- **No mask needed.** Those missions accept any sector, so the waypoint is
  always allowed, like speed.
- **Tests.** Over 1,000 random games (30,000 game-years), random play sent
  313,174 orders to sectors without a world, and none was rejected.
- **The fleet agent** has a waypoint head, so older fleet-agent
  checkpoints no longer load.

**The hunter.** The `hunt` switch sent a warship fleet at home that did
not strike to patrol at the reachable sector nearest a declared enemy's
force. That force had to be reported the previous year near one of the
seat's worlds, and the fleet had to be `attack_ratio` times its estimated
strength (a small vessel counted as a destroyer). The fleet came home the
next year. Paired against the tuned executor (4,096 games a block):

| Version | Block 100000 | Block 200000 |
| --- | --- | --- |
| Forces within 4 sectors of a world | +0.004 ± 0.006 | |
| Forces within 1.5 sectors | +0.010 ± 0.006 | |
| Forces at a world's own sector | +0.011 ± 0.006 | +0.004 ± 0.006 |

Over 128 games, the seat's battles were these:

| | Battles | Held the field | Warships lost | Enemy warships destroyed |
| --- | --- | --- | --- | --- |
| Without the switch | 2,722 | 1,337 | 8,994 | 8,116 |
| With it (within 4 sectors) | 2,739 | 1,357 | 10,304 | 7,973 |
| With it (at the world's sector) | 2,739 | 1,359 | 9,623 | 8,116 |

Hunting added about 2.5 patrol fleet-years a game, but almost no battles.
A raider reported last year has usually moved on by the time hunters
arrive, and the hunters lost more ships on the way. Answering only forces
in the seat's own systems came closest to a gain, but did not confirm. The
code is removed. Early warning from t174's pickets is what would make
interception pay, so t174 measures the net with hunting off.

### t174

A picket net: cheap destroyers patrolling empty sectors.

The owner's own practice (2026-10-15): build cheap destroyers and station
one in each empty sector around the seat's worlds and on the common routes
between worlds, on Patrol a Sector, to detect enemy fleet movements.

- **Why it works.** Patrol a Sector watches ten times as hard as an
  ordinary fleet and reports everything it spots. Destroyers are cheap and
  the fastest ships (speed 6), so they can escape heavy fleets.
- **Early warning.** Sightings enter the seat's contacts memory years
  before an attack lands. t169's guards and t173's hunters respond to
  those contacts; t162 found extra scouting unrewarded, but that scouting
  had no responders.

The work, behind a switch:

- **Picket sectors.** Choose empty sectors one or two steps around the
  seat's worlds, weighted toward known rival worlds, plus sectors on the
  straight routes between the seat's worlds and the nearest rival worlds.
  They need t173's deep-space targets.
- **Build and post pickets.** Lone destroyers keep each picket sector
  filled. The home world adds a destroyer when a sector is unwatched, up
  to a tunable number of pickets.
- **Low ROE.** A picket reports and gets away; it does not fight.

**Exit:**

- As in Scope, with the hunters and guards present in both arms, so the
  comparison isolates the net.
- Report how many years ahead rival fleets are first seen before they
  reach the seat's worlds, with and without the net.

**Result (2026-10-15): rejected.** Guards (t169) and hunters (t173) were
both rejected, so the net was measured alone, and then with t173's hunter
as its responder. The `pickets` switch worked as follows:

- **Posts.** The halfway points from home to the three nearest worlds
  known to be a rival's.
- **Pickets.** Lone destroyers took empty posts on Patrol a Sector at
  ROE 0. A picket whose post was no longer one moved on.
- **Supply.** Home built a destroyer for each post no spare picket could
  take, and while a post was empty, one home fleet a year detached a
  destroyer (t172) to take it. A year's hulls commission as one fleet, so
  a built destroyer is rarely alone.

**Warning.** The measure was the share of attacks on the seat's worlds
where the seat had seen the attacker's forces within 4 sectors of that
world beforehand, and how many years earlier. Over 128 games:

| | Attacks | Warned | Mean warning |
| --- | --- | --- | --- |
| Without the net | 4,181 | 46% | 3.07 years |
| With the net | 4,051 | 56% | 3.31 years |

About 2.4 pickets stood out per seat-year. A first version let pickets
keep posts that were no longer posts. It kept about 4.8 out and warned
64%, but cost more.

**Paired results** against the tuned executor, block 100000 (4,096
games):

| Version | Difference |
| --- | --- |
| Net, first version | −0.023 ± 0.008 |
| Net | −0.009 ± 0.008 |
| Net with the hunter, forces within 4 sectors | −0.000 ± 0.009 |
| Net with the hunter, forces at a world's sector | −0.000 ± 0.009 |

The net sees more, and sooner. But no rule the executor has acts on
sightings well enough to repay the destroyers: the hunter only cancels
the net's cost. The code is removed. Whether to answer a sighting is a
judgement of threat and opportunity, which t175's allocation goal and the
learned fleet agent can make. The deep-space waypoints (t173) and Detach
(t172) let the network man a net if that pays.

### t175

Share the fleet between offence and defence.

Fleets should serve both offence and defence (owner, 2026-10-15). A new
strategist goal, the fleet allocation, holds back 0%, 25%, 50% or 75% of
the seat's warship strength for defence each year. The executor carries
it out:

- **The held share** guards threatened worlds and stays home as a guard
  (t169), with pickets (t174) watching and hunters (t173) intercepting.
- **The rest** forms the strike force.
- **Splitting existing forces** needs Detach (t172); new warships go where
  the share is short.

The balance must come from the tactical situation and the seat's goals
(owner, 2026-10-15), not a fixed share. It weighs:

- **Threat:** rival fleet strength seen near the seat's worlds, and how
  recently.
- **Opportunity:** how weak and how valuable the chosen strike target is.
- **State:** at war or not, expanding or consolidating, and how exposed
  the colonies are.

The strategist sees all of these in its observation, and PPO teaches it
which trade-offs win. Until it has learned the goal, the executor's rule
weighs threat against opportunity: hold back enough to meet the threat
seen, unless the strike target falls to the rest. The goal joins the goal
masks and labels, so the strategist imitates that rule before PPO.

**Exit:**

- As in Scope, for the executor's allocation rule against the
  strike-everything executor.
- After retraining, the strategist's allocation follows the situation:
  - the held share rises with nearby threat;
  - it falls when a cheap, valuable target is open;
  - it beats the executor's rule head to head.

  Report the mean held share by level of threat and of opportunity.

### t176

Early reconnaissance: find rival homeworlds and survey the map.

The owner (2026-10-15): early in the game it is a high priority to find
the other players' homeworlds and to view as many worlds as possible, to
learn their colonization status and production potential.

- **Today** the two starting destroyers view whichever world the seat
  knows least about, during the land grab only. Tuning has cut scouts to
  about none.
- **Probable cause.** As t162 found, scouting is unrewarded when nothing
  acts on what it finds. This task gives early intel specific consumers.

The work, behind a switch:

- **Survey order.** Early in the game, scouts and pickets view every
  unseen world, nearest first, in rings outward from home. A fixed
  minimum of early scouts (or picket destroyers) does it, independent of
  the tuned weights.
- **Rival homeworlds.** Rank unseen worlds by how likely each is to be a
  homeworld: far from the seat's own, and evenly spread across the map as
  starting positions are. Visit the likeliest first. A world seen at
  homeworld production marks a rival's home.
- **Standing surveillance** (owner, 2026-10-15). Once found, keep a scout
  watching each known rival homeworld and the richest known enemy worlds:
  the largest production, the most valuable targets. A lone scout is the
  hardest fleet to see, and Scout a Solar System reports in detail.
  Refresh a watch when its intel ages past a year or two. It keeps current:
  - defences, for strike estimates and the attack mission (t160, t168);
  - fleets gathering at a rival home, an early warning for the defence
    balance (t175);
  - production and growth, for picking the most valuable targets.
- **Use what is found:**
  - colony targets use known potential, not an assumed average;
  - strike targets (t160) see rival colonies' value and defences;
  - the defence balance (t175) points toward known rival homes;
  - war declarations, which the strategist sets, can depend on which
    rivals are near.

**Exit:**

- As in Scope.
- Report the share of worlds viewed by years 5, 10 and 20, and the year
  the first rival homeworld is found, with and without the switch.
- Report the mean age of the seat's intel on rival homeworlds and on its
  strike targets at the moment it strikes.

**Result (2026-10-15): rejected.**

- **The switch tried.** `recon` kept at least two scouts. Scouts went
  first to unseen worlds far from home, where rival homeworlds lie. Known
  rich rival worlds (production at least 50, or potential at least 80)
  were watched again once their intel was two years old.
- **Measured over 128 executor games,** without and with the switch:

  | | Without | With |
  | --- | ---: | ---: |
  | Worlds whose owner the seat knows, year 5 | 0.57 | 0.60 |
  | Worlds whose owner the seat knows, year 10 | 1.00 | 1.00 |
  | Year the first rival homeworld is identified | 4.9 | 4.6 |
  | Intel age at the moment of a strike | 6.7 years | 3.7 years |

  The starting pickets and the land grab already survey the map. Standing
  surveillance halves the age of strike intel.
- **No gain.** On 4,096 paired maps the difference is −0.018 ± 0.009.
  Tuned with the switch on, the same weights with it on against off won
  0.242 and 0.229. Even beside t177's colonization scoring, which uses
  rival positions, it lost.
- **Why.** As in t162, the executor's rules do not turn fresher intel into
  better choices, and the scouts cost production. Intel may pay once a
  learned strategist or fleet agent can use it; t157's agents see intel
  age.
- The code is removed.
### t177

Colonize for expected return: value, arrival time, and the race against
rivals.

The owner (2026-10-15): manage ETACs to colonize the most desirable
worlds, balanced against closer worlds that are easier to reach, while
gauging rivals' proximity and judging who is likelier to colonize first.

**Today.** Each free ETAC goes to the world with the best known potential
over (1 + distance), skipping worlds the seat's own ETACs already claim.
An unseen world counts as potential 40. Three things are missing:

- arrival time is not valued: colonies grow from founding, and ETACs move
  at speed 3;
- rivals are ignored, so it may race for worlds it will lose (VBAM: both
  convoys are lost);
- chains are not planned, though ETACs are reusable.

The work, behind a switch: score each candidate as an expected return,
following the risk-and-return principle.

- **Value.** Known production potential, discounted by the years until
  founding (the ETAC's travel time).
- **Chance of arriving first.** Compare the ETAC's travel time with the
  nearest rival's likely one, from known rival homeworlds and colonies
  (t176) and recent contacts near the world. Scale the value by that
  chance.
- **Risk.** Rival fleets seen near the world, and how far it is from the
  seat's own worlds once founded.
- **Chains.** Prefer a target that leaves the ETAC near its next good
  target.

**Exit:**

- As in Scope.
- Report colonies founded by years 10 and 20, and colonization trips lost
  to a rival arriving first, with and without the switch.

**Result (2026-10-15): rejected.**

- **The switch tried.** `colonize_return` scored each candidate world:
  - its potential, discounted by 0.9 per year of the ETAC's travel;
  - times the chance of arriving before the nearest known rival world
    (logistic in the difference of travel times at speed 3);
  - halved near rival fleets seen within two years;
  - plus a quarter of the best nearby follow-on colony.
- **No gain.** On 4,096 paired maps the difference is +0.011 ± 0.008; on
  top of scaled garrisons, +0.003 ± 0.009. Tuned with it on, the switch
  on against off won 0.234 and 0.239.
- **Why.** The simple potential-over-distance rule already captures most
  of the value. Expansion choices may matter more through the strategist's
  colony goal than through the executor's scoring.
- The code is removed.
### t178

Learn what a seat's own attacks reveal.

The AI must collect intelligence to make informed decisions (owner,
2026-10-15), and should never know less than a human player in its seat.
t161 found that a bombarding seat's memory almost never learns its
target's battery count: in 9,426 of about 10,200 bombarding fleet-years
the batteries were unknown.

- **Check what humans see.** List what the attacking player's bombard,
  invade, blitz and battle reports show about the target: batteries,
  armies, starbases and production remaining. Check the player manual and
  the report code.
- **Add the gap to `SeatMemory`.** Whatever those reports give a human but
  the seat's memory lacks is added: the intel grants the attacker
  receives, or the facts the reports state.
- **No more than a human knows.** t151's scramble tests are extended to
  attack reports, so the seat learns nothing a human player would not.

**Exit:**

- A test bombards a world and checks the attacker's memory then holds what
  its reports show.
- t151's leak tests still pass.
- Report how often a striking seat knows its target's batteries, before
  and after.

**Result (2026-10-15):**

- **What a human sees.** The attacker's bombardment report shows the
  world's defences going in (batteries and armies) and the enemy's losses,
  so a human player knows what is left. Assault reports, for invasions and
  blitzes, carry the same. The planet database's intel sources (scouting,
  View a World, colonization, civil disorder, assaults) never take battery
  counts from bombardment, and a failed assault records only owner and
  potential. The seat's memory had the same gap.
- **The fix.** `SeatMemory::learn_year` now reads the seat's own
  `bombard_events` and `assault_report_events`. It sets each target's
  known batteries and armies to the initial values less the losses
  inflicted (and invasion softening), dated to the year of the attack as
  scouting intel is. Only reports where the seat was the attacker count,
  and a world it captured is left to its owned view.
- **Tests.**
  - `nc-ai/tests/attack_intel.rs` checks, over four 60-year executor
    games, that each attacker's memory holds exactly what its report
    states.
  - t151's leak test now also scrambles other empires' bombard and
    assault reports, and still passes.
- **Before and after,** six 60-year executor games, counting fleet-years
  at a strike target:

  | | Batteries known | Unknown | Share known |
  | --- | ---: | ---: | ---: |
  | Before | 553 | 743 | 43% |
  | After | 1,254 | 178 | 88% |

  The executors' weights were tuned without this intel; later re-tunes
  include it.
- **The classic replay test changed with it.** t159's
  `the_classic_doctrine_replays_t154_exactly` pinned two t154 results. The
  classic seats now learn from their own attacks too, so those games play
  differently. The classic code path is unchanged; the test's values were
  re-recorded with the new intel, and its comment says why. The first
  t178 commit went in with this test failing: a test summary in the
  command chain did not stop the commit.

## Connections

- [Strategy doctrine for AI seats](../investigations/v6jegy7f-strategy-doctrine-for-ai-seats-from-the-manuals-vbam-and-owner-guidance.md):
  the sources, quotations, and what the executor already does.
- [Learned AI opponent plan](qkgqypzw-learned-ai-opponent-harness-hierarchical-agent-and-fleet-level-bootstrap-on-the-mac-mini.md):
  t154 (executor), t155 (strategist), t157 (the fleet-level agent, which
  will be able to learn these lessons itself).
