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

### t171

A dedicated blockader.

t163 failed because the whole strike force sat on a blockade. Here a
separate warship fleet blockades a rival world the strike force cannot yet
take, chosen by value per estimated defence, while the strike force goes on
reloading and striking. The blockader comes home when its target falls,
or when the seat's home is threatened.

**Exit:** as in Scope. Also report the production denied to the
blockaded worlds.

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
