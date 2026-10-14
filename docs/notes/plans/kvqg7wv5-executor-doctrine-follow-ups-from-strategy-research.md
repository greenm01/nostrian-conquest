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

## Connections

- [Strategy doctrine for AI seats](../investigations/v6jegy7f-strategy-doctrine-for-ai-seats-from-the-manuals-vbam-and-owner-guidance.md):
  the sources, quotations, and what the executor already does.
- [Learned AI opponent plan](qkgqypzw-learned-ai-opponent-harness-hierarchical-agent-and-fleet-level-bootstrap-on-the-mac-mini.md):
  t154 (executor), t155 (strategist), t157 (the fleet-level agent, which
  will be able to learn these lessons itself).
