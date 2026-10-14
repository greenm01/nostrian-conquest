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

### t160

Choose strike targets by value over estimated defence.

Today a home force strikes the most valuable world it can take, discounted
by distance. Space Empires 4X players agree you should strike where the
enemy is weakest and count losses against gains. Score candidate worlds by
value divided by estimated defence, still discounted by distance, so cheap
conquests come first.

**Exit:** as in Scope.

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

### t163

Blockade worlds the strike force can hold but not yet take.

VBAM: blockading "is an effective way to deny resources ... even if you do
not have the troop strength to try and capture". Today a force without
enough armies waits at home or bombards.

When the force beats a target's known fleets and batteries but has too few
armies to invade, order Guard/Blockade at the target. Keep the rest of the
force gathering at home.

**Exit:** as in Scope.

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
