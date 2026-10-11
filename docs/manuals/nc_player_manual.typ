// Nostrian Conquest — Player Manual
// Typst source — generates US Letter PDF with proper table layout

#set document(
  title: "Nostrian Conquest — Player Manual",
  author: "Mason A. Green",
  date: datetime(year: 2026, month: 10, day: 11),
)

#set page(
  paper: "us-letter",
  margin: (x: 1in, y: 1in),
  footer: none,
)

#set text(
  font: "IBM Plex Serif",
  size: 11pt,
)

#show raw: set text(font: "IBM Plex Mono")
#show table.cell: set par(justify: false)

#set par(
  justify: true,
  leading: 0.65em,
)

#set heading(numbering: "1.")

// Bold header row styling for all tables
#show table.cell.where(y: 0): strong

// Admonition helper
#let admonition(kind, body) = {
  let icon = if kind == "NOTE" { "i" }
    else if kind == "WARNING" { "!" }
    else if kind == "IMPORTANT" { "!!" }
    else { "?" }
  block(
    width: 100%,
    inset: 10pt,
    stroke: 0.5pt + luma(160),
    radius: 3pt,
    fill: luma(245),
  )[
    #text(weight: "bold")[#kind:] #body
  ]
}

// ─── Title Page ───────────────────────────────────────────────────────────

#let manual_license_notice = [
  New text, layout, and explanatory material in this manual
  © 2026 Mason A. Green and are licensed under CC BY-NC-SA 4.0. Original
  historical Esterian Conquest references and other preserved 1992 source
  material are excluded from that grant and remain credited to their original
  authors and rights holders.
]

#let numbered_footer = context align(center)[
  #set text(size: 9pt, fill: luma(120))
  Page #counter(page).get().first() of #counter(page).final().first()
]

#align(center + horizon)[
  #text(size: 24pt, weight: "bold")[Nostrian Conquest]
  #linebreak()
  #text(size: 16pt)[Player Manual]
  #v(1em)
  #text(size: 10pt, style: "italic")[A Rust recreation inspired by the classic 1990s BBS door game Esterian Conquest.]
  #v(0.5em)
  #text(size: 10pt, fill: luma(120))[Built for localhost and BBS play. A Nostr GameServer path is planned.]
  #v(0.5em)
  #text(size: 10pt, fill: luma(120))[All code, UI, and assets in this edition are original. Not affiliated with any original release.]
  #v(0.5em)
  #text(size: 10pt, fill: luma(120))[Revision date: October 11, 2026]
  #v(0.5em)
  #text(size: 10pt, fill: luma(120))[Version 1.0.0-beta.3 — Beta]
]

#pagebreak()

#align(center + horizon)[
  #v(3em)
  #image("assets/cc-by-nc-sa-4.0-badge.svg", width: 3.3in)
  #v(1em)
  #block(width: 80%)[
    #set text(size: 9pt, fill: luma(110))
    #manual_license_notice
  ]
  #v(0.5em)
  #text(size: 9pt, fill: luma(110))[
    License text: #link("https://creativecommons.org/licenses/by-nc-sa/4.0/")
  ]
]

#pagebreak()
#counter(page).update(1)
#set page(footer: numbered_footer)

// ─── Table of Contents ───────────────────────────────────────────────────

#outline(
  title: "Contents and Appendices",
  indent: 1.5em,
)

#pagebreak()

// ─── 1. Introduction ────────────────────────────────────────────────────

= Introduction

Beyond the mapped frontiers of the old Nostrian dominion lies a small galaxy of contested solar systems. The old masters are gone. Their stations are silent, their patrols vanished, and their subjects left with fleets, industry, and enough knowledge to build empires.

You rise as one of the new Star Masters. From a single world and a few small fleets, you must tax, build, scout, bargain, threaten, and strike before rival powers can do the same. Some systems will join your banner willingly. Others will require persuasion from orbit.

Each maintenance marks the passage of a year. In that span, fleets cross the dark between stars, colonies grow or starve, alliances turn cold, and wars are decided by distance, industry, mathematics, and will.

Nostrian Conquest is a Rust recreation inspired by the classic 1990s BBS door game Esterian Conquest. All code, UI, and assets in this edition are original. It is not affiliated with any original release.

In homage to the 1990s door-game pioneers and to the ancient dreamers, strategists, and storytellers whose visions of galactic dominion still light the way among these stars.

#pagebreak()

// ─── 2. Connecting to a Game ────────────────────────────────────────────

= Connecting to a Game

== Three Ways to Play

Nostrian Conquest has three practical ways to play.

The direct path is *Localhost*. In that mode you or your sysop runs `nc-game`
directly on the same machine. Use this for solo play, hotseat testing, and
trusted same-machine sessions. If you are setting that up yourself, see the
*Sysop Manual* section *Localhost Session Setup*.

The classic path is *BBS*. In that mode you log into a bulletin board with a
terminal client and launch the game from the doors menu. The sysop stages
`nc-door` behind the BBS. If you are the sysop, see the *Sysop Manual* section
*BBS Door Setup*.

The planned network path is *Nostr GameServer*. That path is still under
development. For now, treat localhost and BBS as the normal ways to play.

== Finding New Games

If you are joining a private localhost campaign, your sysop tells you which
machine and player slot to use.

If you are joining a BBS campaign, connect to the bulletin board normally and
launch the game from the doors menu. Your sysop may reserve a seat for your BBS
alias, or may leave open seats available from the first-time join menu.

If you are waiting for the Nostr GameServer path, watch the main project docs.
It is planned, but it is not the normal join path yet.

== Joining a Game

For localhost play, the normal direct launch is:

```sh
nc-game --dir /path/to/mygame --player 1
```

Your sysop may give you a different player number or run the command for you on
the local machine.

For BBS play, the sysop launches `nc-door` from the board software and the
caller enters through the normal first-time or returning-player flow inside the
game. If your BBS alias is reserved for a seat, the door routes you there
automatically.

#pagebreak()

// ─── 3. Quick Start: Gameplay Basics ──────────────────────────────────────

= Quick Start: Gameplay Basics

== Your Objective

Your objective is simple: become Emperor by dominating rivals or eliminating every serious threat.

You begin with one planet at 100 production and four fleets --- two carrying an ETAC and cruiser for colonization, and two lone destroyers for screening and early contact. Set a tax rate around 50--65% on your homeworld, and use the revenue to build ships, armies, batteries, and starbases. Keep taxes low on new colonies so they develop quickly.

Each round represents one year. You submit orders during the year, and maintenance resolves all empires simultaneously on an internal 52-week timeline. Reports are dated as `Stardate WK/YYYY` (week/year).

== Turn Progression

There is no "End Turn" button. You set your economy, assign your missions, and
log off. The sysop or schedule decides when maintenance runs. When it does, the
game resolves every empire at once, advances the year, and writes your next set
of reports.

If maintenance runs while you are logged in, the game cannot save your next
change. It says so on the menu notice line: "Maintenance has run; your last
change was not saved. Log in again." Log in again to carry on. In a BBS door
the session ends and returns you to the BBS.

== Assets You Can Build

The table below lists everything your planets can produce. *AS* is Attack Strength (firepower) and *DS* is Defense Strength (hull/shields).

#figure(
  table(
    columns: (auto, auto, auto, auto, auto, auto, 1fr),
    align: (left, right, center, center, right, right, left),
    inset: 6pt,
    table.header(
      [Item], [Cost], [Size], [Speed], [AS], [DS], [Purpose],
    ),
    [Destroyer],       [05], [S], [6], [10], [08], [Fast combat, screening, defense],
    [Cruiser],         [15], [M], [5], [30], [30], [Balanced combat],
    [Battleship],      [45], [L], [4], [90], [100], [Heavy combat],
    [Scout],           [15], [S], [6], [00], [10], [Reconnaissance],
    [Troop Transport], [05], [M], [5], [00], [10], [Deliver armies],
    [ETAC],            [20], [L], [3], [00], [20], [Colonize raw planets (reusable)],
    [Ground Battery],  [20], [L], [--], [90], [20], [Planetary defense],
    [Army],            [02], [S], [--], [10], [10], [Surface defense and capture],
    [Starbase],        [50], [L], [1], [100], [120], [Defense, production boost],
  ),
)

== Fleet Missions (Summary)

Fleet missions fall into three classes. _One-shot_ missions travel, act, and
then fall back to Hold Position. _Persistent_ missions stay in force until you
replace them or game rules cancel them. _Hostile_ missions attack the year they reach
orbit around the target world. Bombard keeps bombarding each year; Invade and
Blitz fall back to Hold Position in orbit after the assault, win or lose.
@movement explains how long a trip takes.

#figure(
  table(
    columns: (auto, auto, auto, 1.2fr, 1.8fr),
    align: (right, left, left, left, left),
    inset: 6pt,
    table.header(
      [No], [Mission], [Type], [Requirements], [On Arrival / Completion],
    ),
    [00], [Hold position],         [Persistent], [Any ships],                   [Idle at current position],
    [01], [Move to sector],        [One-shot],   [Any],                         [Reverts to Hold on arrival],
    [02], [Seek Home],             [One-shot],   [Any],                         [Reverts to Hold; retargets if destination captured],
    [03], [Patrol],                [Persistent], [Any],                         [Patrols sector continuously],
    [04], [Guard starbase],        [Persistent], [Combat ships],                [Escorts starbase continuously],
    [05], [Guard/blockade world],  [Persistent], [Combat ships],                [Blockades world continuously],
    [06], [Bombard world],         [Hostile],    [Combat ships],                [Bombards when in orbit at tick start],
    [07], [Invade world],          [Hostile],    [Combat + loaded transports],  [Invades when in orbit at tick start],
    [08], [Blitz world],           [Hostile],    [Loaded transports (combat recommended)], [Blitzes when in orbit at tick start],
    [09], [View],                  [One-shot],   [Any],                         [Dives in, scans, climbs back out; reverts to Hold],
    [10], [Scout sector],          [Persistent], [At least one scout],          [Reports each turn while on station],
    [11], [Scout system],          [Persistent], [At least one scout],          [Reports each turn while on station],
    [12], [Colonize world],        [One-shot],   [At least one ETAC],           [Colonizes if unowned; ETAC survives for reuse],
    [13], [Join Fleet],            [Persistent], [Any],                         [Chases host until merge; abandons if host lost],
    [14], [Rendezvous],            [Persistent], [Any],                         [Waits at sector; lowest fleet ID becomes host],
    [15], [Salvage],               [One-shot],   [Any],                         [Scraps ships for half their build cost],
  ),
)

#pagebreak()

// ─── 4. Forces at Your Command ──────────────────────────────────────────

= Forces at Your Command

You control five major force types: planets, ships, starbases, armies, and ground batteries.

== Planets

Planets are the foundation of your empire. Each one has a *Potential Production* --- the ceiling it can reach at full efficiency, ranging from 10 to 150 across the galaxy --- and a *Production*, which is what the planet can actually deliver right now. Production grows toward Potential over time, and the speed of that growth depends heavily on your tax rate.

Taxes convert Production into spendable build points each year. See @economy for the full tax rate, growth, and starbase economics model.

== Ships

Ships operate in fleets. A fleet always moves at the speed of its slowest member and can hold anywhere from one to *3,000 ships* of mixed types.

#figure(
  table(
    columns: (auto, auto, auto, auto, auto, auto, 1fr),
    align: (left, right, center, center, right, right, left),
    inset: 6pt,
    table.header(
      [Ship Type], [Cost], [Speed], [Size], [AS], [DS], [Tactical Role],
    ),
    [*Destroyer*],  [05], [6], [S], [10], [08], [Fast raider and line support. Escapes heavy fleets. Good for fast response and interception.],
    [*Cruiser*],    [15], [5], [M], [30], [30], [Balanced fighter. About 3x the power of a destroyer.],
    [*Battleship*], [45], [4], [L], [90], [100], [Heavy firepower anchor. About 3x the power of a cruiser. Slow but durable.],
    [*Scout*],      [15], [6], [S], [00], [10], [Stealthy spy. A lone scout is hardest to detect.],
    [*Transport*],  [05], [5], [M], [00], [10], [Unarmed. Carries one army. Essential for conquest.],
    [*ETAC*],       [20], [3], [L], [00], [20], [Colony ship. Reusable --- survives colonization.],
  ),
)

== Starbases

Starbases are large space fortresses (Cost: 50, AS: 100, DS: 120). In orbit,
they strengthen a world's defense and economy. They help weak colonies grow at
sensible tax rates, and they let a planet spend up to *5x* its Production in
one turn when enough points have been saved (see @economy). They are not a
license to tax a world to death. In deep space they serve as watch posts with
slightly more firepower than a battleship, but they crawl at speed 1.

Unlike ships, starbases do not belong to fleets. You commission them one at a
time from stardock and move them through the *Starbase Command* submenu. Older
docs sometimes say a starbase is "hauled." In practice, that just means moving
a very slow unit. You may order combat fleets to escort a starbase with
Mission 4 (Guard Starbase), but the starbase remains its own unit.

A starbase moves one sector a year. A base resting in orbit first spends a
year climbing out to deep space. It enters orbit at its destination only if
you own the world there; anywhere else it stops in deep space. Its escort, the
guard fleet at rest beside it, rides along. Other guard fleets head for the
base's new position whenever it changes sector or climbs out of a system, and
a guard that reaches the base joins the escort, unless the two together would
pass 3000 ships.

A starbase in orbit needs a friendly world beneath it. If you lose that world,
the base leaves the system for deep space in its own sector. A base bound for
a world you no longer own stops where it is and awaits new orders. Moving a
starbase out of orbit also cancels every build order of the world beneath it
when those builds rely on the base: when a queued unit, or the whole queue,
costs more than the world's Production and no other starbase rests in orbit
there.

#admonition("NOTE")[Starbases must be commissioned from stardock before they can be moved or provide orbital benefits. An uncommissioned starbase sitting in stardock has no effect.]

== Ground Forces

*Armies* (Cost: 2, AS: 10, DS: 10) defend your planets from invasion and are the only way to capture enemy worlds. Each troop transport carries one army, and a successful invasion requires landing enough armies to overwhelm the defending garrison.

*Ground Batteries* (Cost: 20, AS: 90, DS: 20) are immobile land-based cannons that offer massive firepower per cost --- roughly equal to a battleship at less than half the price. During an invasion, all batteries must be destroyed before transports can land. During a blitz, surviving batteries fire directly on descending transports, inflicting heavy losses. If a planet is captured via blitz, any surviving batteries transfer intact to the new owner.

#pagebreak()

// ─── 5. Economy and Taxes ───────────────────────────────────────────────

= Economy and Taxes <economy>

Your empire runs on production. Every owned planet generates tax revenue each maintenance cycle, and that revenue pays for ships, defenses, and expansion. Managing the tension between short-term revenue and long-term growth is one of the deepest strategic challenges in the game.

== Key Terms

Every planet has a *Potential Production* --- the maximum productive capacity it can ever reach --- and a *Production*, which is what it can deliver right now. Production grows toward Potential over time. A planet's *Revenue* is the tax it will add to its Treasury at the next maintenance, assuming your tax rate, the planet's owner, and its starbase stay as they are. Your empire's *Empire Revenue* is the sum of that revenue across all your planets. Any revenue a planet does not spend accumulates on it as its *Treasury* --- the reserve of saved production points available to fund future builds. Each turn, how much of that treasury a planet can actually spend is limited by its *build capacity*; that per-turn spending limit is the planet's *Budget*.

== Tax Revenue

The tax rate is empire-wide: you set one rate for all your planets. Revenue per
planet per year is a fixed percentage of Production, and your empire's
Empire Revenue is the sum of that revenue across all owned planets. The screens
show next year's revenue, rounded to the nearest whole point, and setting the
tax rate reports the new Empire Revenue. Production and the Treasury keep their
fractions from year to year; only what you read and spend is a whole number.
See @appendix-economy for the exact formula.

== Growth Toward Potential

Each maintenance turn, every owned planet grows toward its Potential. Lower
taxes grow faster. A planet at 30% tax develops far faster than one at 60%.
Growth also slows as the planet nears its ceiling: each year a planet closes
a share of its gap to Potential, so it comes close to Potential without
reaching it on a fixed date. Every point of tax costs growth, and there is no
free yearly minimum.

Growth is based on the gap to Potential and the remaining tax headroom. See
@appendix-economy for the exact formula.

== The 65% Tax Threshold

#admonition("WARNING")[Setting taxes above *65%* can directly _reduce_ Production on your planets, not just slow growth.]

Below 65%, growth is always positive --- lower is faster. Above 65%, a penalty kicks in that actively damages Production, and a mature planet settles below its Potential: a Potential-100 world taxed at 70% settles at Production 88 and earns 62 a year, and at 71% it settles at 86 and earns 61. A commissioned starbase does *not* remove this danger. Its value is that it helps young colonies grow faster before you reach punitive tax rates, and it still preserves the powerful build-capacity bonus described below.

The recommended early-game rate is around 50--65%. Drop taxes on new colonies to accelerate their development.
See @appendix-economy for the exact penalty and yearly update formulas.

== Starbase Economic Effects

A commissioned starbase in orbit provides two major benefits. First, the
planet's yearly production growth gets a strong boost when taxes are modest ---
half again as fast at *50% tax or lower*, fading steadily to nothing as taxes
climb toward *65%*. This means underdeveloped planets with starbases catch up
significantly faster when you are managing them sensibly, but the bonus is not
meant to offset punitive taxes. A planet already at its Potential gains
nothing from it. Second, the planet gains a *build capacity
multiplier* --- it can spend up to *5x* its Production on builds in a
single turn, drawing from its treasury. Without a starbase, a
planet can only spend up to 1x its Production per turn. See
@appendix-economy for the exact bonus fade and build-capacity formulas.

More starbases at one world add defense, not growth or capacity. A starbase
that is crippled in battle but survives keeps both bonuses.

#admonition("NOTE")[These bonuses require an active, commissioned starbase in orbit --- not an uncommissioned starbase sitting in stardock. The growth bonus needs a starbase of yours at rest in orbit of your world. The build-capacity bonus counts a starbase of yours in orbit, or on its way down to the planet's orbit, so a starbase arriving at a world can raise its budget a year before it raises its growth. A starbase of another empire counts for neither.]

== Treasury <treasury>

Tax revenue that a planet does not spend accumulates in its *Treasury* --- the planet's savings reserve. The treasury is what allows starbase worlds to execute large builds: up to 5x Production in a single turn. Each turn, a planet's *Budget* is the lesser of its treasury and its build capacity. That is what the build screen shows as *BUDGET* --- how many production points remain uncommitted this turn. When maintenance processes a build queue, only the points actually spent that year are deducted from the treasury; unfinished builds keep their remaining cost for later turns. You spend whole points. The treasury keeps any fraction, and it can carry a debt: a negative treasury stays negative, buys nothing, and later revenue pays it down.

== Newly Colonized Planets

A freshly colonized planet starts at Production 3 (or at its Potential, if that is lower) and with no treasury. It does not collect revenue or growth on the same maintenance turn that establishes the colony. Growth begins on the next maintenance turn. Each year growth is applied first and tax revenue is then credited on the grown Production, so at moderate tax rates most colonies have a small budget after their first full turn. Very small worlds, or colonies under low taxes, can still sit at zero budget for a turn or more, because the treasury only counts whole points. The head start comes only with founding, never with capture, recapture, or recovery, and does not depend on the tax rate. Low taxes still develop new colonies fastest; they just fill the treasury more slowly at first.

== Conquered Planets

When you capture an enemy planet by invasion or blitz, the planet's industry needs *two quiet years* before it is converted. It pays no revenue and does not grow in the next two maintenance turns, and it pays full revenue from the third. Its Revenue reads 0 until then. Plan your logistics around this delay.

== Scorch Earth <scorch-earth>

Scorch Earth denies a world to an invader. The *Scorch* command sets the order, and the same command rescinds it. Setting it asks three confirmations; rescinding it asks one. The order changes nothing at once. It stays in force until you rescind it or the world is captured, and it first takes effect at the next maintenance, before the year's fighting, so it is already in force when an assault strikes that year.

Each year the order is in force, the planet's Production falls to between 30% and 49% of itself, never below 1, and its stored points fall to a tenth, or to their square root if that is smaller. The planet neither grows nor pays revenue, and its Revenue reads 0. Debt is left as it is. Potential Production is not touched, so a world that is not captured recovers by the normal growth rules once you rescind the order. Your maintenance report tells you each year what the order cost the planet.

#pagebreak()

// ─── 6. Combat Mechanics ────────────────────────────────────────────────

= Combat Mechanics

Battle reports are short, but the fighting is not random brawling. Combat is
simultaneous and rule-driven. Both sides fire in the same round. Losses are
then applied. This keeps battle outcomes clear, severe, and consistent.

== The Rules of Battle

There is no "first strike" advantage. Each round, both sides fire at the same instant. Each side rolls one ten-sided die, and the roll decides how much of its total *Attack Strength (AS)* lands on the enemy: a quarter on 0--2, half on 3--5, and all of it on 6--9. A bigger fleet wins by bringing more AS, not by rolling on a better table. Rounds repeat until one side is destroyed or breaks off.

Damage reduces ships from "Nominal" to "Crippled" status before destroying them. All nominal ships must be crippled before any crippled ship is destroyed. A crippled ship fights at half strength in every later battle until it recovers on its own: a ship crippled this year fights crippled next year and the year after, and is whole from then on, with no order, payment, or shipyard. A fleet that rests in orbit of your world beside your starbase for a whole year gets its older cripples back a year sooner. Hits always target the combat line first --- destroyers, cruisers, battleships, and starbases. Scouts, transports, and ETACs are protected as long as any combat-line ships remain, which means your non-combat vessels survive as long as your warships hold the line.

Each damage step costs the target's full *Defense Strength (DS)*: once to cripple a ship and again to finish it. Hits too few for the next step are not wasted. They carry over to the next round of the same battle, so even a small force wears down a big ship if it stays in the fight. Leftover hits last only as long as the battle. When it ends, whether by destruction, withdrawal, or the round limit, they are lost, and the next battle starts from zero, even against the same enemy fleet. A fleet that breaks off forfeits the hits it has built up against the enemy.

Two bonuses add to your die roll. A fleet that mixes at least two of
destroyers, cruisers, and battleships gets a *combined arms bonus* of +1, so
mix your composition when you can. A starbase fighting on your side adds
another +1. The bonuses never add more than +2, and a roll pushed past 9
counts as 9. An unmodified 9 is a critical hit that destroys one more enemy
ship outright. In a draw, the defender wins. See @appendix-combat for the
exact ROE thresholds, combat values, effectiveness table, and assault
formulas.

== Who Fights Whom

Fleets meet only when they share a sector *and* a depth. Each sector has two
layers: deep space, and the solar system around its world (see @movement). A
fleet at least halfway down to a world is in its system. A fleet in orbit
cannot see a fleet passing overhead in deep space, and neither can fight the
other.

A year is 52 weeks. Each week, every fleet and starbase takes a turn, in an
order drawn at random once a year. In its turn a fleet moves one week's
distance, scans the alien fleets and starbases sharing its sector and layer,
and then carries out its order. Battles therefore happen in the middle of the
year, wherever fleets happen to be that week, and the report carries that
week's stardate.

*Sighting.* Each week a fleet has a chance of spotting each alien unit near
it. Big fleets are easier to see than small ones: battleships and ETACs show
most, then cruisers and transports, then destroyers; scouts add a little. A
fleet in a solar system is five times easier to see than one in deep space,
and a fast-moving fleet is easier to see than a slow one. A fleet at its
target on a bombard, invade, or blitz mission is twenty times easier to see,
and patrols, colonizers, and system scouts twice as easy. Starbases are fairly
conspicuous.

How hard a fleet looks depends on its order. A patrol that has reached its
sector looks ten times as hard as an ordinary fleet; a scout or a blockade on
station, five times. Starbases look hardest of all. Spotting a unit once makes
it easier to spot again. What one of your fleets or starbases learns about an
alien unit, all of them know, until that unit moves to another sector or
layer.

*Identification.* A fleet with ROE above 0 closes to identify what it spots,
and so do patrols, scouts, and blockades on station. Any other fleet only
reports a sensor contact. A fleet guarding a starbase stays with its base and
does not close.

*The decision.* A fleet that identifies an alien fleet attacks it when:

- your empire has declared the fleet's owner an enemy (his declaration
  against you does not count);
- the alien fleet is in one of your solar systems, whatever its order, and
  even if its owner is neutral;
- your fleet is blockading a world and the alien fleet has come to bombard,
  invade, or blitz it, or to colonize it while it is unowned (this holds at
  any world, yours or not);
- both fleets are colonizing the same unowned world; or
- your fleet is on a bombard, invade, blitz, or colonize mission and the
  alien fleet guards the target world. If your ROE forbids that fight, your
  fleet aborts its mission and seeks home instead.

Any other alien fleet is ignored. A sector with no world is nobody's system.

Whether your fleet then attacks is decided by its ROE and the odds, the first
time it identifies a target as on every later week (see @appendix-combat).
Identifying an enemy first commits nothing: a cautious fleet does not attack
a stronger enemy just because it saw it first. For these odds each warship
counts its attack strength --- a destroyer 10, a cruiser 30, a battleship 90
--- and a starbase 100, the same values the battle fires with. Scouts,
transports, and ETACs count nothing, and a fleet without warships never
attacks, whatever its ROE.

The odds are those of the sides as they would fight. Your fleets in the same
sector and layer fight together, so each of them weighs all of them against
all of the enemy's there: two fleets of five cruisers at ROE 6 would each
refuse ten cruisers alone, and take them on together. Each of your fleets
there volunteers under its own ROE: one that refuses the assembled odds stays
out, and the others re-check without it. If the fleet that spotted the enemy
is the one that refuses, there is no attack that week; another willing fleet
of yours attacks only when it spots and catches the enemy itself.

*Intercept.* A fleet with a higher maximum speed always catches its target.
Otherwise the target may notice the attack and run. A stopped target cannot
run; a moving one escapes more often the faster it moves and the slower its
pursuer. A target that already knows the attacker rarely notices it coming,
and then it stays to fight only if its own ROE accepts the odds.

*Battle.* Every battle is between two empires. Your side is your fleets in
the sector and layer that volunteered; the enemy's side is all of his fleets
there, whatever their ROE --- a fleet that is attacked fights, ROE 0 included
--- with the starbase that was attacked or that a guard fleet among them
guards. Nobody else joins: not a third empire, not allies, not fleets that
arrive later in the week. The fleet that started the attack spends the week
fighting instead of moving. A fight does not change the empires' diplomacy.

*Starbases* scan, and what they identify every fleet of their empire knows,
but they never start a fight and never join one to attack. A fleet attacks a
starbase only if its empire has declared the base's owner an enemy, or if
the base protects the world the fleet has come to bombard, invade, or blitz.
Either way the fleet weighs the base, its guard, and the owner's other fleets
there against its own side, and if the odds are against it, it avoids the
base or aborts its mission.

*Blockades.* A blockade does not stop a neutral fleet leaving the world or
arriving at it, unless the arrival comes to attack or colonize it. A blockade
catches a departing fleet only if your empire has declared that fleet's owner
an enemy, and only if it spots and intercepts the fleet in the few weeks the
fleet takes to climb out of orbit.

== Tactical Roles and Split Fire

Combat is more than just a numbers game. Different ship classes perform specific tactical roles based on how they deliver fire. Hits generated by a task force are split into two pools:

- *Suppression Fire (Cruisers and Battleships):* These heavy vessels provide volume and suppression. Their hits are dispersed across the enemy fleet, reducing nominal ships to a *crippled* state. This effectively knocks enemy guns offline and wins the immediate field, but it does not always result in permanent kills.
- *Execution Fire (Destroyers and Starbases):* These units use *focus fire* to eliminate specific targets. Their hits bypass the crippled state and allocate directly to the *destroyed* pool, paying the full 2x DS cost to blow ships up one by one.

This creates a deadly pair. Heavy ships soften the line. Destroyers and
starbases finish it. Each pool carries its own leftover hits: suppression hits
never pay for an execution kill, and execution hits never pay for a cripple.

#admonition("NOTE")[All planetary return fire from ground batteries is treated as *Execution Fire*. Fortified worlds do not "soften" an attacker; they destroy him.]

== Fleet Limits

A fleet can contain as few as one ship and as many as *3,000 ships* of mixed types. A fleet always moves at the speed of its slowest member.

== Rules of Engagement (ROE)

You assign an ROE level (0--10) to control when your fleet attacks. ROE 0
never attacks, but a fleet on ROE 0 that is attacked still fights the first
exchange. Any higher ROE attacks a hostile fleet when the force ratio of the
two sides, your fleets in the sector and layer against the enemy's, meets the
ROE threshold (see Who Fights Whom); seeing the enemy first changes nothing.
ROE also decides when your fleet breaks off a battle: after the compulsory
first exchange it checks the same odds after every exchange and leaves as
soon as they fail (see Withdrawal and Retreat).

The full ROE threshold table is collected in @appendix-combat.

Non-combat fleets (scouts, transports, ETACs only) are treated as ROE 0 automatically. Giving such a fleet any order sets its ROE to 0, and so does a detach or transfer that leaves a fleet without a battleship, cruiser, or destroyer. A fleet that gains warships keeps its ROE, even ROE 0; set a new one with ROE (*C*). A fleet that Detach creates starts at ROE 6, or at 0 when it has no warships, whatever the donor's ROE is.

== Withdrawal and Retreat

#admonition("IMPORTANT")[Low ROE does not guarantee safety once combat begins.]

Every battle opens with one compulsory exchange: both sides fire once,
whatever their ROE. After it, and after every exchange that follows, each of
your fleets in the battle checks its own ROE against the odds as they now
stand, your side's attack strength against the enemy's, and breaks off as
soon as they fail. Every fleet whose ROE fails leaves at once, on both sides
together, and the fleets that stay re-check the odds without them. A fleet
that has left does not come back, even if the odds later turn in your
favour. A fleet with no armed ship left, including one that has just lost
its last warship, tries to escape at once, whatever its ROE. A starbase
never breaks off; the fleets beside it, its guard included, leave under
their own ROE like any other.

Losses are shared among your fleets in proportion to each fleet's eligible
ships: of the ships the enemy's fire reaches, a fleet with eight cruisers and
one with two share three losses as two and one. Ties are broken at random.
Splitting or merging fleets can shift a rounding loss by one hull. Loaded
transports are hit before empty ones across all your fleets there, so moving
cargo into another fleet does not shelter it.

Breaking off is not always a clean escape. A fleet that leaves while the rest
of your side fights on gets away for now, but the enemy may still catch it
once later. When your whole side has broken off and the enemy stays with
armed ships, he may pursue each fleeing fleet. A pursuer whose ships are
faster than yours catches you for certain; otherwise it is a roll, likelier
the faster he is and the slower you are. A catch is one more exchange for
the fleets caught, with die bonuses and carried hits on both sides, and then
the battle is over. When both sides break off, nobody pursues. Each enemy
empire catches an escaping fleet at most once: in the battle it left, or, if
that pursuit failed, in one more attack while the fleet is still leaving the
sector. After that the fleet has got away from that empire until it leaves
the sector or you give it a new order. Your warships still screen your
auxiliaries during a pursuit exchange; only a critical hit reaches past the
line. A battle still undecided after 64 exchanges simply ends: nobody wins,
nobody retreats, and the next week's encounter rules decide whether the
fleets meet again.

A fleet that retreats, or aborts a mission its ROE will not let it fight for, seeks home: it heads for your nearest planet other than the one whose system it is in. With no other planet to go to, it stays in the sector. Its report says why it left --- its ROE, or no warships left --- and whether it was caught on the way out.

== Planetary Combat

When fleets attack planets through bombardment, invasion, or blitz, different rules apply. Orbital fire at a planet reads its die on a table that compares the two sides' strength, and leftover hits do not carry from round to round. The landing battle between armies has its own rules (see The Landing Battle below). Ground batteries are the planet's shield wall --- while they stand, they draw orbital fire and shoot back, protecting the stardock, armies, production, and industry behind them. Only combat ships --- destroyers, cruisers, and battleships --- contribute bombardment firepower. Scouts, transports, and ETACs do not.

#admonition("WARNING")[Planetary defenses use *focus fire*. Hits from ground batteries bypass the crippled state and directly destroy ships, making them far more lethal than dispersed ship-to-ship fire. A commander who brings a large fleet to orbit a fortified world will find his ships being picked off one by one by concentrated ground fire.]

Each bombardment turn resolves three rounds of fire. In rounds 1 and 2, your ships trade fire with the planet's batteries. While batteries stand at the start of a round, your hits land on the batteries only, and hits beyond what destroys them are wasted. Ships in the stardock are exposed from the next round, the first that begins with no batteries standing. Armies, stored goods, and factories are shielded in these rounds. In round 3, if all batteries have been destroyed, your remaining firepower breaks through to the stardock, then armies, stored goods, and factories. If batteries still stand, round 3 is another suppression exchange and the planet's vulnerable assets survive another turn.

Planetary targets have defense strength just like ships. Destroying a ground battery takes 20 hits, an army 10 hits, and a ship in the stardock as many hits as its own DS. A lone cruiser cannot level a fortified world in a single volley.

This means a well-defended planet takes sustained bombardment over multiple turns before you touch its production. Build batteries to buy time; bring heavy fleets to break through faster.

See @missions for the detailed mechanics of bombardment, invasion, and blitz missions. The exact bombardment weights, batteries-only return-fire rule, and combat tables are in @appendix-combat.

=== The Landing Battle

Armies that land fight on the same table as fleets, army against army, with no force-ratio columns. Each round the *defenders fire first*. Each side rolls one `d10` for its whole army: 0--2 deals 25%, 3--5 deals 50%, and 6--9 deals 100% of its armies' strength. The attackers' losses come off, and the attackers who survive fire back the same way. Rounds repeat until one side has no armies left.

Damage short of a whole army carries into the next round of the same landing and is lost when the landing ends. There is no defender bonus, no homeworld bonus, and no critical hit; firing first is the defender's advantage. The attackers fire only with what survives, so both sides are never destroyed together. Your reports show whole armies lost and remaining; the carried fractions stay hidden.

Capture odds follow the ratio of landed armies to defending armies, roughly the same at any scale. Equal numbers take a world about one time in five. At 1.5 to 1 the odds are about even, and at 2 to 1 the attackers win nine times in ten. Overwhelming numbers still win every time. Appendix B gives the table.

=== Captured Worlds

A world that falls passes to you with what the fight left standing: its stored goods, production, docked ships, and queued build work. There is no capture bonus, no loss for the kind of attack, and no forgiveness of debt. Your capture report lists the goods, the docked ships, and the queued work you inherit.

A defender can set Scorch Earth to deny a world to the invader. If the world falls with the order set, its production and positive goods are halved, its stardock is emptied, the order is lifted, and production is raised to at least 1.0. A world with no potential stays at 0, debt is untouched, and the build queue is not burned. The capture report says when the defenders had scorched the world. A failed assault leaves the order set.

=== Failed Assaults

Failure is not defeat. A fleet whose landing fails, or whose Invade is blocked because batteries survived suppression, keeps its survivors in orbit on Hold and receives a failure report. A blocked Invade lands nothing and its armies stay aboard. The fleet does not blockade the world on its own, and like any fleet in orbit it is open to the ordinary encounter rules. A fleet beaten in the orbital battle before it can strike turns for home under the withdrawal rules above; the world's batteries fire on no fleet except one that strikes.

#pagebreak()

// ─── 7. Missions and Orders ─────────────────────────────────────────────

= Missions and Orders <missions>

A fleet always has exactly one standing order. If you issue a new order before maintenance, it replaces whatever the fleet was doing. Missions fall into three categories. _One-shot_ missions cause the fleet to travel, perform an action, and revert to Hold Position --- you must issue new orders afterward. _Persistent_ missions remain active until you replace them or game rules invalidate them (for example, a Join mission is abandoned if the host fleet is destroyed). _Hostile_ missions send the fleet into orbit around a target world. It attacks the year it arrives. A bombarding fleet stays in orbit and bombards again each year until you change its orders. An invading or blitzing fleet makes one assault and, win or lose, its survivors hold position in orbit.

#figure(
  table(
    columns: (auto, auto, auto, 1fr),
    align: (right, left, left, left),
    inset: 6pt,
    table.header(
      [No], [Mission], [Type], [Description],
    ),
    [00], [None],            [Persistent], [Hold position at current location.],
    [01], [Move Fleet],      [One-shot],   [Travel to a specified sector, then revert to Hold.],
    [02], [Seek Home],       [One-shot],   [Return to nearest owned planet; retargets if that planet is captured en route.],
    [03], [Patrol],          [Persistent], [Move within sector deep space to intercept enemies.],
    [04], [Guard Starbase],  [Persistent], [Escort a starbase; fight alongside it.],
    [05], [Blockade],        [Persistent], [Watch a world. Attacks assaults, colonizers of an unowned world, and declared enemies.],
    [06], [Bombard],         [Hostile],    [Damage factories, batteries, armies, stardock contents, and production.],
    [07], [Invade],          [Hostile],    [Suppress batteries, bombard, then land armies. Requires loaded transports.],
    [08], [Blitz],           [Hostile],    [Drop armies immediately, dodging batteries. High army risk.],
    [09], [View],            [One-shot],   [Quick scan of owner and production; the fleet dives in and climbs back out.],
    [10], [Scout Sector],    [Persistent], [Watch a sector closely and identify traffic (requires Scout).],
    [11], [Scout System],    [Persistent], [Active spy mission into a solar system (requires Scout).],
    [12], [Colonize],        [One-shot],   [Terraform and claim unowned planet (requires ETAC). ETAC survives for reuse.],
    [13], [Join Fleet],      [Persistent], [Chase and merge with target fleet. Abandons if host is destroyed.],
    [14], [Rendezvous],      [Persistent], [Meet other fleets at sector. Lowest fleet ID becomes host.],
    [15], [Salvage],         [One-shot],   [Scrap ships at planet for half their build cost.],
  ),
)

== Movement and Arrival <movement>

A fleet's speed is the number of sectors it can travel in a year.

Movement is 2.5D. Think of the map as a flat board, with each world sitting
one step below it at the center of its sector. Fleets travel across the board.
To reach a world, a fleet steps down into its orbit; to leave one, it steps
back up. Each step costs one sector of movement, and a fleet can be partway
down.

A fleet never moves across and down at once. It climbs out of orbit, flies
straight across the map to the center of its target sector, and only then
heads down toward the world. A fleet leaving a planet spends its first sector of movement climbing
out, so it covers one sector less in its first year.

When a fleet arrives depends on its mission.

- *Sector missions* (Move, Patrol, Scout Sector, Rendezvous) arrive the year
  the fleet enters the target sector.
- *World missions* (all the others) continue to the world. Guard/Blockade,
  View, Scout System, Join, Salvage, Bombard, Invade, and Blitz arrive when
  the fleet reaches orbit. Seek Home and Colonize finish at the end of the
  first year the fleet is at least halfway in. Guard Starbase takes up station
  once the fleet is halfway in and its starbase is there.

A world mission often takes a year longer than a move to the same sector.
Distance is measured in a straight line, so three sectors on a diagonal is
about 4.2 sectors of travel.

The *ETA* command shows the year a fleet will arrive. For a world mission,
answer yes to "Include time to enter system?"

== Mission Details

=== One-Shot Missions

*Mission 1: Move to Sector.* A simple transit order. The fleet travels to the destination sector at the speed of its slowest ship, then stops and reverts to Hold Position. You must issue new orders if you want it to do anything else.

*Mission 2: Seek Home.* The fleet travels to the nearest planet you own. It keeps that planet for as long as you own it, even if another becomes nearer on the way. If you lose the planet while the fleet is en route, the fleet turns at once for the nearest planet you still own, measured from where it is; with none left, it stops and holds where it is. On arrival, it reverts to Hold Position. The game picks the planet; you enter no coordinates. When you order a single fleet that is already in the system of its nearest planet, the game asks "Do you wish this fleet to seek the next closest system?" Yes, the default, sends it to the next closest planet; no keeps it on the planet it is at. If that planet is the only one you own, the game says so and gives no order. In a group order each fleet seeks its own nearest planet without asking.

*Mission 9: View a World.* A quick look. The fleet dives into the target system, scans for owner and production data, and climbs straight back out into deep space. While it is in the system, the owner's fleets may spot and attack it. It reverts to Hold Position after reporting.

*Mission 12: Colonize a World.* Requires at least one ETAC (Environmental Transformation And Colonization ship). If the planet is unowned, it is terraformed and claimed. The new colony starts with one garrison army, no treasury, and very low current production. It does not receive revenue or growth until later maintenance turns, and then develops faster under low taxes. The ETAC is not consumed --- it survives and can colonize additional planets. If the planet is already owned, the ETAC aborts, reports the owner's identity and production potential, writes that partial result into your Total Planet Database, and waits for new orders.

*Mission 15: Salvage.* The fleet travels to one of your planets and scraps its ships for half their build cost, rounded up ship by ship, returned to that planet's treasury: 23 points for a battleship, 8 for a cruiser, 3 for a destroyer, 3 for a transport (loaded or empty), 10 for an ETAC, and 8 for a scout. Armies aboard loaded transports land on the planet, and the fleet is disbanded. If you lose the planet while the fleet is en route, the fleet salvages at the nearest planet you still own instead; with none left, it stops and holds where it is. The game offers the planet nearest the fleet as the default target; if you own only one planet, it takes that planet without asking. A group salvages at one planet.

=== Persistent Standing Missions

*Mission 0: Hold Position.* The default idle state. The fleet stays at its current location and takes defensive action based on ROE if hostile fleets approach.

*Mission 3: Patrol a Sector.* The fleet watches a sector, looking ten times as hard as an ordinary fleet once it arrives. It identifies everything it spots, sends a report, and attacks what the rules in Who Fights Whom allow, as far as your ROE permits. The mission remains active until you assign a new one.

*Mission 4: Guard Starbase.* The fleet escorts a starbase and fights alongside it: an attack on either one is a battle against both, and the base joins any battle its guard is drawn into as a defender. A guard stays with its base and does not close on the contacts it spots, but it attacks, or joins an attack by your other fleets there, when the ROE odds favor the fight.

*Mission 5: Guard/Blockade a World.* A strategy of denial. The blockade watches the world five times as hard as an ordinary fleet. It attacks any fleet that comes to bombard, invade, or blitz the world, or to colonize it while it is unowned, and any fleet of an empire you have declared an enemy, including ships leaving the world. Neutral fleets come and go. Blockading another player's world takes your fleet into his solar system, so his fleets there will attack it. It remains active until you assign a new mission.

*Missions 10 and 11: Scout Sector / Scout System.* Both require at least one Scout ship, and a fleet consisting of a single Scout is the least likely to be detected. Scout Sector is a stealthy watch over the sector: a scout on station looks five times as hard as an ordinary fleet and identifies what it spots, and with ROE 0 it never attacks. Scout System is an active spy run where the Scout penetrates the system to report on ground batteries, armies, current production, stardock contents, and orbiting fleets; entering the system means the owner's fleets there may attack it. Both are persistent missions: the fleet remains on station and generates a new report each maintenance turn for as long as it stays at its assigned sector. If you identify an *In Civil Disorder* fleet in a system that still belongs to that disorder state, the Total Planet Database marks that world's owner as `ICD` even before a full scout/view report is earned.

*Missions 13 and 14: Fleet Coordination.* Mission 13 (Join Fleet) causes the fleet to chase a specific host fleet and merge with it when they meet. If the host is destroyed before they rendezvous, the joining fleet abandons the mission. Mission 14 (Rendezvous) sends multiple fleets to a sector where the fleet with the lowest Fleet ID becomes the host of the combined force. The rendezvous point remains active so additional fleets can keep merging there.

=== Hostile (Delayed-Resolution) Missions

#admonition("IMPORTANT")[A hostile fleet attacks during the maintenance in which it reaches orbit (see @movement). The fleet stops in orbit. Bombardment repeats each year until you change the order. An invasion or blitz is a single assault: whether it takes the world, loses its armies, or is called off because the batteries survived, the surviving ships hold position in orbit, and you must order any new attack. A fleet told to attack the world it already orbits attacks at the next maintenance.]

A fleet in orbit on a hostile mission has entered the owner's solar system, and it is twenty times easier to see there, so his fleets in orbit are likely to attack it. Before it attacks, it must get past whatever the owner keeps in orbit: his starbase and his fleets there with warships fight it the week it reaches orbit, spotted or not, and your other fleets in that orbit join it under their own ROE. If its ROE will not take them on with the side it has, it turns for home without a shot; if it loses, it is driven off, and his batteries do not fire on it as it goes. Only a fleet that holds the orbit attacks, in that same week. Each fleet attacks on its own as it arrives, so two of your fleets at one world strike one after the other, and the second need not fight defenders the first has beaten.

*Mission 6: Bombard a World.* Only destroyers, cruisers, and battleships contribute bombardment firepower --- scouts, transports, and ETACs do not. Each turn your fleet bombards, the engine runs three rounds of fire. In rounds 1 and 2, your ships exchange fire with ground batteries --- while batteries stand, your hits destroy batteries only and the excess is wasted, and armies, stored goods, and industry are shielded behind the battery wall. Each battery takes 20 hits to destroy, each army 10, and each docked ship its own DS. Docked ships are exposed from the first round that begins with no batteries standing. In round 3, if batteries have been eliminated, your firepower breaks through to the stardock, then armies, stored goods, and finally industry. If batteries still stand after round 2, round 3 is another suppression exchange and the planet's production survives. Bombardment persists each turn until you issue new orders. Use it to grind down a world's defenses before invasion, or to deny resources to an enemy over time.

*Mission 7: Invade a World.* A three-stage deliberate assault. First, combat ships exchange fire with ground batteries in orbital suppression --- transports cannot land until all batteries are destroyed. Once batteries are gone, surviving combat ships fire on the defending armies to soften resistance before landing. Softening fire is matched against the garrison's full strength (10 AS per army), and each army destroyed takes 10 hits. Unlike bombardment, invasion softening targets armies only --- industry and stored goods are preserved because the goal is to capture the planet with its production intact. However, orbital softening can destroy at most *half* of the defending army stack; at least half of the original defenders must still be fought on the ground. Finally, transports land their armies and the landing battle runs in rounds until one side is wiped out. The defenders fire first in every round, and one side always survives. Capture requires destroying all defending armies, after which your surviving armies become the new garrison. If the batteries survive suppression, no troops land and the armies stay aboard. Win, lose, or called off, the fleet then holds position in orbit. Conquered planets need approximately two turns before they are fully converted to your production.

*Mission 8: Blitz a World.* Transports drop armies immediately in a fast assault that bypasses the full orbital suppression sequence. Escorting combat ships provide brief cover fire, but surviving batteries fire directly on descending transports, causing heavy losses. A fleet of loaded `TT*` may attempt a blitz even without escorting warships, but then it brings no orbital firepower of its own. The owner's ships without warships do not prevent that attempt, but his starbase or any fleet of his with warships in orbit stops it: with nothing to fight them, the transports turn for home. Once on the surface, armies fight the same landing battle as an invasion, defenders first, until one side is wiped out. If you take the planet, surviving ground batteries transfer intact to your control. The blitz preserves industry but carries high risk to your armies and transports --- a 2:1 army advantage or better is recommended. Win or lose, the fleet then holds position in orbit. Choose blitz when the planet has few or no batteries and you want to preserve its industry, or when speed matters more than casualties.

#pagebreak()

// ─── 8. Interface and Commands ──────────────────────────────────────────

= Interface and Commands

The game is organized around four primary menus. From the *Main Menu*, you access General Command (*G*) for autopilot, diplomacy, and reports; Planet Command (*P*) for economy and production; Fleet Command (*F*) for ship movement and missions; Information Database (*I*) to review known planet data; and View Starmap (*V*) for a graphic map of the galaxy.

== Visual Themes

The `nc-game` client supports several visual themes. In a local session, open
*C>olor Theme* from the Main Menu or First Time Menu to choose one. Your last
local choice is remembered for that empire in that campaign.

In BBS door mode, the game keeps the classic *A>nsi color ON/OFF* toggle.
Each session starts from the `mag16` theme so ANSI16 terminals stay
stable. Press *A* to switch between that view and monochrome for the current
session. Saved local theme preferences do not apply in door mode.

== General Command

General Command handles empire-wide business. Autopilot (*A*) lets the
computer mind your defenses if you miss turns. By default, autopilot turns on
after three missed turns. Your first real return that year --- logging into
`nc-game`, entering through `nc-door`, or submitting a valid turn file ---
turns that inactivity autopilot back off. Manual autopilot stays on until you
change it yourself. While autopilot runs, each year it sends every fleet on
Hold Position to the planet of yours that is best for its distance and
production. Warships already in orbit there take Guard/Blockade, raising an
ROE of 0 to 6, and escort a starbase of yours in the same orbit that has no
escort; fleets with any other order are left alone.
Diplomacy (*E*) lets you mark other empires as Neutral or
Enemy. Messages (*C*) lets you write to other empires. In the message editor, *^E* sends, *^X* cancels, and *^G* shows help; *?* types a question mark, as it does in name fields. Messages reach their recipients at the next maintenance. Until then, the ones you send this turn sit at the top of your inbox, marked *OUT*; *Enter* reads one and *D* cancels it. *D* on the recipient list opens the same queued messages with the same keys.

To keep messaging civil, you may send no more than three messages to any single opponent per turn. In a four-player game, for example, that is up to twelve outgoing messages in a turn. Messages and reports are never automatically purged --- they accumulate in your inbox across turns until you remove them yourself. The inbox supports type and year filters to help you find older items, and pressing *D* on a selected item prompts for deletion, defaulting to yes. General Command also offers a bulk delete to clear all messages at once. New reports and messages from the most recent maintenance turn are presented one by one through the scrolling intro review when you log in, giving you another opportunity to read and delete before reaching the main menu.

The empire rankings table shows each joined empire in one of three
states: *Active*, *MIA*, or *Defeated*. *MIA* means inactivity autopilot is
currently running because that player missed three consecutive turns.
*Defeated* means the empire has been eliminated from active command.

== Defeat, Recovery, and Victory

Losing your last planet does not always defeat you immediately. If you still
have a recovery path, you remain *Active* and receive a *three-turn recovery
window* to reclaim a world. A recovery path means at least one loaded troop
transport (`TT*`) somewhere in your surviving fleets, or at least one ETAC
while any unowned planet still exists anywhere on the map. If you retake or
colonize a world before the window expires, the countdown clears and your
empire continues normally.

If your last recovery force is destroyed while you are planetless, or if the
three-turn recovery window expires first, your empire becomes *Defeated*.
Fleet Command sends you a final defeat report, and you receive one last
review-only login so you can read the closing reports and messages. After that
final review, you may no longer enter the campaign.

When you eliminate another empire, Fleet Command reports that you delivered the
final blow. This is separate from overall game victory. The game ends only
when one empire is recognized as Emperor and no other serious contender
remains. At that point every still-playable empire receives a game-over report
naming the victor. The winner may continue to log in in *survey mode* to
inspect the final state of the galaxy, but may not issue orders or submit
turns. All other players receive one final review-only pass and are then
blocked from further play.

== Planet Command

Planet Command controls your economy and ground operations. Tax (*T*) sets the empire-wide tax rate. From the main Planet Command menu, Scorch Earth (*S*) sets or rescinds the order that wears down your own industry to deny it to an invader (see @scorch-earth). Build (*B*) spends production points on ships, defenses, or starbases. Commission (*C*) assigns newly built ships from stardock into active fleets. *Mass Commission* (*M*) commissions every ship and starbase currently waiting in stardock. Load and Unload (*L* / *U*) move armies between the planet surface and troop transports.

The *Planet List* is the fast row-centric operations screen for owned worlds. Once you open it, the highlighted planet becomes the working row for the most common actions: Build (*B*), Display Queue (*D*), Abort Builds (*A*), Mass Commission (*M*), Commission (*C*), Load / Unload armies (*L* / *U*), and Scorch Earth (*X*). On that screen, *S* is reserved for *Sort*, while *I* or *Enter* opens planet information for the selected row. Owned-planet information includes the planet's *Budget* as well as its *Treasury*.

== Fleet Command

Fleet Command controls your ships in space. Mission (*O*) assigns missions 0--15. ROE (*C*) changes a fleet's rules of engagement, ID, or speed. A speed change takes 1 up to the fleet's maximum and touches nothing else; a fleet on Hold, or already at its target, keeps speed 0. Merge (*M*) combines fleets that are in the same sector. Transfer (*T*) moves individual ships between fleets.

The *Fleet List* is also the bulk-fleet work screen. It now includes a *Sel*
column. Press *Space* on a fleet row to check or uncheck it. When one or more
fleets are checked, *O* assigns one mission to the checked set, *C* changes
their shared ROE or speed, *M* merges the checked fleets using the lowest
Fleet ID as the host, and *T* opens ship transfer for a checked pair. Row-based
commands such as *Review*, *ETA*, *Detach*, *Load*, and *Unload* still use the
highlighted fleet.

Crippled ships show in two places. The Fleet List adds the number of damaged
ships to a fleet's Ships cell when it fits, and *Review* lists them by class.
Your maintenance report says when ships have recovered.

== Building and Commissioning

Each planet keeps a build queue, and your *budget* is the only limit on it: you may order any number of units of any kind, so long as the budget covers them. During maintenance, a planet processes as many queued build points as its current per-turn build capacity allows. Small orders may finish in one maintenance turn, while larger ones can stay queued across multiple years until the remaining cost reaches zero. As enough points are applied to complete individual units, ships and starbases move to *Stardock* --- a holding area on the planet where they sit idle and vulnerable until commissioned --- even if other units from the same order remain queued. Ships are commissioned into numbered fleets, while starbases are commissioned individually and managed through their own Starbase Command submenu. Armies and ground batteries, by contrast, deploy directly to the planet surface and do not pass through stardock.

Aborting the queue releases the points still unpaid. Points already spent on a part-built unit are not returned.

A planet without a starbase can spend up to its Production, rounded to the nearest point, in a single turn. A planet with an orbiting starbase can spend up to *5x* its Production, drawing from its treasury (see @treasury). The build screen shows *BUDGET* --- your treasury capped by build capacity, minus what you have already committed this turn.

#admonition("NOTE")[The build queue, stardock, and treasury are all linked. If stardock is full for a given unit type, that order stops drawing points. Blocked orders do not drain your treasury --- but they also do not free capacity for other work. Commission your ships promptly to keep the pipeline moving.]

#admonition("WARNING")[Troop transports are built empty. You must manually load armies onto them before sending them into battle.]

#admonition("WARNING")[Stardock contents are a prime target for enemy bombardment. Commission your ships promptly or risk losing them before they ever see combat.]

#admonition("NOTE")[The starmap can be exported as a TXT file, a CSV grid, and a CSV details sheet for offline planning. Local Rust-client play can export it directly from the in-game starmap view, and BBS operators can hand the exported files to players out of band if they want offline planning aids.]

#pagebreak()

// ─── 9. Strategy ────────────────────────────────────────────────────────

= Strategy

== Early Game: The Land Grab

The opening turns are a race for territory. ETACs are your most valuable early asset --- grab every raw planet you can find before your rivals do. Push lone destroyers forward as pickets to make early contact, but use true Scouts when you need stealth reconnaissance. Keep your tax rate at or below 65% to avoid damaging production, and drop taxes even lower on new colonies so they develop quickly. The temptation to tax at 100% for immediate cash is strong, but it cripples long-term growth.

== Mid-Game: Consolidation

Once the easy colonies are claimed, the game shifts to fortification and intelligence. Build starbases on your best worlds to boost both defense and production capacity. Never attack a planet blindly --- use Scout ships on Mission 11 to count enemy batteries and garrison strength before committing forces. This is the "get tough" phase: when raw planets are gone, the only way to grow is war.

== Late Game: Total War

In the endgame, fleet composition and denial matter more than raw numbers. Mix destroyers, cruisers, and battleships in every fleet to trigger the combined arms bonus and distribute damage across hull types --- pure battleship fleets are expensive and miss the bonus. If you cannot hold a planet, scorch it. If you cannot take a planet, blockade or bombard it into uselessness. And in a 25-player galaxy, diplomacy is not optional: you cannot fight everyone at once. Form alliances, even temporary ones, and break them only when the timing is right.

#pagebreak()

// ─── 10. Historical Context ────────────────────────────────────────────

= Historical Context

*The BBS Era (1990--1992)* \
The original game emerged between 1990 and 1992 as a "door game" for Bulletin Board Systems. In an era before the World Wide Web, players dialed into servers over phone lines to play strategy games against dozens of strangers. Everything ran through rigid 80x25 text terminals --- every menu, every star map, every battle report squeezed into that tiny viewport. It stood alongside classics like _TradeWars 2002_ and _Solar Realms Elite_, but was distinguished by its depth, its hands-off design, and the sheer scale of its campaigns.

*What Made It Special* \
Most multiplayer games of the era demanded constant attention. Esterian Conquest was different. You checked in once a day, submitted your orders, and went about your life. Overnight, the engine processed every empire simultaneously --- fleets moved, economies grew, battles resolved, and alliances were tested. When you logged in the next day, a stack of reports was waiting. Campaigns ran for months, and the stories they produced --- surprise invasions, desperate blockades, betrayals at the worst possible moment --- were the kind that stuck with players for years.

*The Rust Port (2026)* \
This edition rebuilds the game for modern machines while keeping the old
campaign rhythm intact. The core rules were recovered from the original game
and manuals, then rewritten into a clear modern engine. Where the old game
hid things behind opaque internals, this edition uses explicit documented
rules instead. If you played the original game on a BBS, it should feel
familiar. If you are new to it, this manual gives you the rules straight.

#pagebreak()

// ─── Appendix A. Economy Formula Reference ─────────────────────────────

#set text(size: 10pt)

= Appendix A: Economy Formula Reference <appendix-economy>

This appendix collects the exact economy formulas in one place.

== Yearly Tax Revenue

#align(left)[
  #stack(
    dir: ttb,
    spacing: 0.45em,
    [#text(font: "IBM Plex Mono")[revenue = present_production \* tax_rate / 100]],
  )
]

Maintenance credits this revenue to the planet's treasury after the year's
growth and any high-tax penalty, so `present_production` here is the updated
value. Nothing is rounded: production and the treasury keep their fractions.
The Revenue shown on the planet and empire screens is next year's figure,
rounded to the nearest whole point, on the assumption that the tax rate, the
owner, and the starbase stay as they are. Empire Revenue is the sum of those
figures over all owned planets.

== Yearly Growth

#align(left)[
  #stack(
    dir: ttb,
    spacing: 0.45em,
    [#text(font: "IBM Plex Mono")[gap = potential_production - present_production]],
    [#text(font: "IBM Plex Mono")[tax_headroom = 100 - min(tax_rate, 95)]],
    [#text(font: "IBM Plex Mono")[growth = gap \* tax_headroom / 400]],
  )
]

Growth never exceeds the gap, and there is no yearly minimum.

== High-Tax Penalty Above 65%

#align(left)[
  #stack(
    dir: ttb,
    spacing: 0.45em,
    [#text(font: "IBM Plex Mono")[if tax_rate > 65:]],
    [#text(font: "IBM Plex Mono")[  penalty = present_production \* (tax_rate - 65) / 500]],
  )
]

Final yearly Production is:

#align(left)[
  #stack(
    dir: ttb,
    spacing: 0.45em,
    [#text(font: "IBM Plex Mono")[present_production = present_production + growth - penalty]],
  )
]

and never more than Potential. At a steady tax rate above 65%, Production settles
at `potential * a / (a + b)`, where `a = tax_headroom / 400` and
`b = (tax_rate - 65) / 500`. For Potential 100 that is 88.2 at 70% tax (income
61.8) and 85.8 at 71% (income 60.9). Settled income is highest at 65% and falls
with every point above it.

== Starbase Growth Bonus

A commissioned starbase boosts growth at low and moderate tax rates, and the
bonus fades to nothing by `65%`. Only a base of yours resting in orbit over the
world when the year begins counts. A base under way, even one that arrives
during the year, gives no growth bonus, nor does a base in deep space. More
than one base does not add to the bonus.

#align(left)[
  #stack(
    dir: ttb,
    spacing: 0.45em,
    [#text(font: "IBM Plex Mono")[bonus = 0.5 if tax_rate <= 50]],
    [#text(font: "IBM Plex Mono")[bonus = (65 - tax_rate) / 30 if 50 < tax_rate < 65]],
    [#text(font: "IBM Plex Mono")[bonus = 0 if tax_rate >= 65]],
    [#text(font: "IBM Plex Mono")[growth = gap \* tax_headroom / 400 \* (1 + bonus)]],
  )
]

The bonus does not change the high-tax penalty.

== Build Capacity, Treasury, and Budget

Per-turn build capacity is:

#align(left)[
  #stack(
    dir: ttb,
    spacing: 0.45em,
    [#text(font: "IBM Plex Mono")[build_capacity = round(present_production) without starbase]],
    [#text(font: "IBM Plex Mono")[build_capacity = round(present_production) \* 5 with starbase]],
  )
]

Production is rounded to the nearest whole point. Here "with starbase" means
one of your bases in orbit over the world and bound there, or descending to it.
A base ordered elsewhere, or one in deep space, does not count. One base is
enough; more add nothing.

A planet's *treasury* accumulates unspent production points across turns, with
their fractions. You can spend only its whole points:

#align(left)[
  #stack(
    dir: ttb,
    spacing: 0.45em,
    [#text(font: "IBM Plex Mono")[spendable = floor(max(0, treasury))]],
    [#text(font: "IBM Plex Mono")[budget = min(spendable, build_capacity)]],
  )
]

A negative treasury is a debt. It stays negative, and later revenue pays it
down.

The BUDGET field on the build screen shows how many points remain after
committed queue entries are subtracted. Maintenance deducts only the build
points actually processed that year; any remaining build cost stays queued for
later turns, and blocked builds do not consume the treasury. Aborting the
queue releases the points still unpaid, not points already spent.

== New Colonies

A colony starts with `present_production = min(3, potential_production)` and no
treasury. This applies only when the colony is founded.

== Scorch Earth

While a planet's Scorch Earth order is in force, each yearly pass, after the
build queue and in place of growth and revenue, does this:

#align(left)[
  #stack(
    dir: ttb,
    spacing: 0.45em,
    [#text(font: "IBM Plex Mono")[present_production = max(1, present_production \* (0.30 + Random(20) / 100))]],
    [#text(font: "IBM Plex Mono")[treasury = min(treasury / 10, sqrt(treasury)) if treasury > 0]],
  )
]

`Random(20)` is a whole number from 0 to 19. A planet with Potential 0 stays at
Production 0. A debt (a negative treasury) is left as it is. Potential
Production is unchanged, and the planet's Revenue reads 0.

#pagebreak()

// ─── Appendix B. Combat Tables and Formula Reference ───────────────────

= Appendix B: Combat Tables and Formula Reference <appendix-combat>

This appendix collects the combat reference tables in one place.

== Rules of Engagement Thresholds

#figure(
  table(
    columns: (auto, auto, 1fr),
    align: (right, left, left),
    inset: 6pt,
    table.header(
      [ROE], [Force Requirement], [Behavior],
    ),
    [00], [Never engage],            [*Pacifist*: Flee from all hostile fleets.],
    [01], [Enemy defenseless],       [*Opportunist*: Engage only if the enemy has no combat capability.],
    [02], [4:1 or better],           [*Very Cautious*: Engage only with overwhelming advantage.],
    [03], [3:1 or better],           [*Cautious*: Engage only with strong advantage.],
    [04], [2:1 or better],           [*Favorable*: Engage with clear superiority.],
    [05], [3:2 or better],           [*Confident*: Engage with moderate advantage.],
    [06], [1:1 or better],           [*Balanced*: Engage equal or inferior forces.],
    [07], [Even if outgunned 3:2],   [*Bold*: Accept moderate disadvantage.],
    [08], [Even if outgunned 2:1],   [*Aggressive*: Accept significant disadvantage.],
    [09], [Even if outgunned 3:1],   [*Reckless*: Accept severe disadvantage.],
    [10], [Always],                  [*Suicidal*: Attack regardless of the odds.],
  ),
)

== Unit Combat Values

The combat tables use a *10x internal scale*. That keeps attrition granular and
lets crippled light ships keep contributing fire.

#figure(
  table(
    columns: (auto, auto, auto, 1fr),
    align: (left, right, right, left),
    inset: 6pt,
    table.header(
      [Unit], [AS], [DS], [Notes],
    ),
    [Destroyer], [10], [8],  [Fast raider and line support],
    [Cruiser],   [30], [30], [Balanced brawler],
    [Battleship],[90], [100],[Primary battle line],
    [Scout],     [0],  [10], [Non-combat hull],
    [Transport], [0],  [10], [Non-combat hull],
    [ETAC],      [0],  [20], [Colonization hull],
    [Starbase],  [100],[120],[Heavy orbital defender],
    [Battery],   [90], [20], [Planetary defense],
    [Army],      [10], [10], [Ground combatant],
  ),
)

== Fleet Battle Effectiveness

#figure(
  table(
    columns: (auto, auto),
    align: (left, right),
    inset: 6pt,
    table.header(
      [Modified d10], [Effectiveness],
    ),
    [`0`--`2`], [25% of AS],
    [`3`--`5`], [50% of AS],
    [`6`--`9`], [100% of AS],
  ),
)

Fleet battles, the pursuit exchange included, read the modified `d10` on
this table. The force ratio plays no part in a volley; it decides only
whether a fleet attacks and whether it stays, as attack strength against
attack strength, a starbase counting 100.

== Fleet Battle Die Modifiers and Hit Formula

- Mixed fleet (at least two of destroyers, cruisers, and battleships): `+1`
- Starbase on your side: `+1`
- Total modifier: at most `+2`; a modified roll above `9` counts as `9`
- Pursuit exchange: the same modifiers

#align(left)[
  #stack(
    dir: ttb,
    spacing: 0.45em,
    [#text(font: "IBM Plex Mono")[hits = ceil(total_AS \* effectiveness)]],
  )
]

An unmodified `9` on the `d10` is a critical hit and destroys one more ship
outright, a crippled ship first.

== Damage Steps and Carried Hits

- Crippling a nominal ship costs its full DS.
- Finishing a crippled ship costs its full DS again.
- Execution fire that destroys a nominal ship pays both: `2x DS`.
- Hits a pool cannot spend carry over, in that pool, to the side's next
  exchange in the same battle, including the pursuit exchange.
- Suppression and execution hits carry separately.
- Carried hits are lost when the battle ends: by destruction, by a side
  breaking off, or at the 64-round limit. The next battle starts from zero,
  a pursuit on a later week included.
- A crippled hull stays crippled after the battle and fights at half its AS (a
  crippled starbase at 50) in every later battle, the attack decision and the
  ROE checks included. It recovers on its own at the end of the second year
  after the year that crippled it, and fights whole from the third, for every
  class. A starbase brings it back a year early when the hull was crippled
  before the year began and its fleet rested in orbit of your own world beside
  that starbase for the whole year. Finishing fire takes the newest damage
  first, then the hulls with the most time left.

== Planetary Force Ratio to CRT Column

Bombardment, invasion suppression and softening, and blitz cover and landing
fire read the `d10` on the ratio CRT below. The landing battle between armies
does not; see Landing Battle.

#figure(
  table(
    columns: (auto, auto),
    align: (left, left),
    inset: 6pt,
    table.header(
      [Force Ratio], [CRT Column],
    ),
    [`< 0.5`],        [Disadvantaged],
    [`0.5 .. < 1.0`], [Pressed],
    [`1.0 .. < 1.5`], [Even],
    [`1.5 .. < 3.0`], [Advantaged],
    [`>= 3.0`],       [Overwhelming],
  ),
)

== Planetary Combat CRT

#figure(
  table(
    columns: 6,
    align: center,
    inset: 6pt,
    table.header(
      [d10], [Disadvantaged], [Pressed], [Even], [Advantaged], [Overwhelming],
    ),
    [0], [0.00], [0.25], [0.50], [0.75], [1.00],
    [1], [0.25], [0.50], [0.75], [1.00], [1.25],
    [2], [0.25], [0.50], [1.00], [1.25], [1.50],
    [3], [0.50], [0.75], [1.00], [1.25], [1.50],
    [4], [0.50], [0.75], [1.00], [1.50], [1.75],
    [5], [0.50], [1.00], [1.25], [1.50], [1.75],
    [6], [0.75], [1.00], [1.25], [1.50], [2.00],
    [7], [0.75], [1.00], [1.50], [1.75], [2.00],
    [8], [1.00], [1.25], [1.50], [1.75], [2.00],
    [9], [1.00], [1.50], [1.75], [2.00], [2.50],
  ),
)

== Planetary Column Shifts and Hit Formula

- Mixed `DD/CA/BB` fleet in bombardment or invasion suppression: `+1` CRT
  column
- Final columns are clamped to the table bounds

#align(left)[
  #stack(
    dir: ttb,
    spacing: 0.45em,
    [#text(font: "IBM Plex Mono")[hits = ceil(total_AS \* CRT_multiplier)]],
  )
]

An unmodified `9` on the `d10` is a critical hit and forces one extra bypass
loss allocation.

== Bombardment and Planetary Fire

Only destroyers, cruisers, and battleships contribute bombardment attack
strength:

- Destroyer bombardment AS: `0.5x`
- Cruiser bombardment AS: `1.0x`
- Battleship bombardment AS: `1.5x`

Planetary return fire is:

#align(left)[
  #stack(
    dir: ttb,
    spacing: 0.45em,
    [#text(font: "IBM Plex Mono")[return_fire_AS = battery_AS]],
  )
]

Planetary return fire from batteries uses the *focus fire* rule. Hits bypass
the crippled state and allocate directly to the destroyed pool by paying the
full `2x DS` cost per ship.

Orbital hits on a world pay each target's DS on the same 10x scale:

- Ground battery: `20` hits
- Army: `10` hits
- Ship in the stardock: its own DS from the unit table (docked batteries `20`,
  armies `10`)
- Stored goods and factories: remaining hits are divided by `10`, and each
  point then destroys `25` stored goods and `1` factory

Hits below one unit's cost do not destroy it, and partial hits do not carry
over to the next round. Once the stardock is exposed, hits too few for the next
docked item pass on to the next stardock entry. Invasion suppression and blitz
cover fire pay the same battery cost and never reach the stardock.

Blitz landing fire is one exchange of the surviving batteries' AS against the
escorts' bombardment AS. It is focus fire aimed at the descending transports
first: each transport destroyed costs `20` hits (2x its DS) and takes its army
with it, and any hits left over fall on the escorts.

Each bombardment turn runs three rounds. Batteries shield the stardock: an
exchange that begins with batteries standing hits batteries only, and hits
beyond what destroys them expire. Docked ships are exposed from the next
exchange.
- *Rounds 1--2 (Suppression):* While batteries stand at the start of the
  round, attacker hits go to batteries only; with none standing, they go to
  the stardock. Batteries fire back each round. Armies, stored goods, and
  industry are shielded.
- *Round 3 (Breakthrough):* If batteries reached zero before this round,
  attacker hits go to the stardock, then cascade into armies, stored goods,
  and industry. If batteries still stand, round 3 is another suppression
  exchange.

Invasion uses one suppression exchange against batteries. If batteries are
cleared, the softening pass targets armies only --- industry and stored goods
are not damaged during invasion. Its CRT column compares bombardment AS
against `armies x 10`. Orbital softening may destroy at most half of
the defender's starting armies, ensuring a meaningful landing battle whenever a
world began the assault with a garrison. Once armies land, the landing battle
in invasion and blitz is fought as described next.

A fleet whose landing fails, or whose Invade is blocked by surviving batteries,
holds in orbit on Hold with a failure report. It does not blockade the world
automatically.

== Landing Battle

Armies (AS 10, DS 10) fight on the same table as fleet battles, unmodified, in
rounds. In each round the defenders roll and fire first, the attackers' losses
come off, and the surviving attackers roll and fire back. A side with `n`
armies deals `n x 1`, `n x 2`, or `n x 4` quarters of an army on a roll of
`0`--`2`, `3`--`5`, or `6`--`9`; four quarters destroy one army. Damage short
of an army carries into the next round of the same landing and is lost when the
landing ends. There is no defender modifier, homeworld bonus, or critical hit,
and one side always survives. Rounds repeat until one side has no armies.

#figure(
  table(
    columns: (auto, auto),
    align: (left, right),
    inset: 6pt,
    table.header(
      [Attackers vs defenders], [Capture],
    ),
    [1 vs 1],   [33.6%],
    [2 vs 1],   [85.1%],
    [3 vs 2],   [55.6%],
    [3 vs 1],   [99.4%],
    [5 vs 5],   [22.6%],
    [10 vs 10], [19.9%],
    [11 vs 10], [27.6%],
    [15 vs 10], [56.1%],
    [20 vs 10], [91.8%],
    [20 vs 20], [19.0%],
    [40 vs 20], [92.3%],
  ),
)

The armies that land are those left after blitz landing fire, and the
defenders are those left after invasion softening. Capture odds follow that
ratio at every scale; overwhelming ratios still win every time.

#pagebreak()

// ─── Appendix C. Preservation and Original Sources ─────────────────────

= Appendix C: Preservation and Original Sources <appendix-preservation>

This edition preserves the game in a playable modern form. The aim is simple:
keep the campaign rules legible, playable, and documented.

== Source Hierarchy

This manual is the authoritative player manual for the Rust edition of
Nostrian Conquest.

The preserved originals in `original/v1.5/` remain historical references and
an ambiguity fallback:

- `ECQSTART.DOC` --- Quick-start guide
- `ECPLAYER.DOC` --- Detailed player manual
- `ECREADME.DOC` --- Release and package information
- `ECGAME.EXE` --- The original 1992 player client
- `ECMAINT.EXE` --- The original yearly maintenance program
- `ECUTIL.EXE` --- The original sysop utility for game initialization and management

== Preservation Policy

- This manual is the authoritative player-facing manual for the Rust edition.
- The original manuals are preserved historical references and an ambiguity
  fallback for classic intent and terminology.
- The original DOS binaries remain the final compatibility check.
- This edition keeps the player-visible classic behavior that matters.
- When an exact classic formula is still unknown, this manual states the rule
  plainly instead of pretending otherwise.
- When the preserved originals clarify an ambiguity, that clarification should
  be folded back into the current manuals/specs rather than left only in the
  legacy `.DOC` set.

== BBS Drop File Compatibility

Rust BBS hosting uses `nc-door`, not the old DOS binary path. It reads
`DOOR32.SYS`, `DOOR.SYS`, and `CHAIN.TXT` directly. For a player, the practical
point is simple: log into the board, launch the door, and play. If you are the
sysop, see the *Sysop Manual* for setup details.

== This Manual

This manual combines the original player guidance with the appendix reference
tables in @appendix-economy, @appendix-combat, and @appendix-table-ui. The
main body stays player-facing. The appendices collect formulas, tables, and
list-screen codes.

#pagebreak()

// ─── Appendix D. Table Filtering and Sorting ───────────────────────────

= Appendix D: Table Filtering and Sorting <appendix-table-ui>

The main list screens all use the same command-line filter system. This lets
you cut a long table down to the exact rows you want instead of paging
through dead weight.

One filter is active at a time on each table. If you choose a new filter, it
replaces the old one. If a filter matches nothing, the table stays filtered
and tells you no rows match.

== Filtering Procedure

1. Open the table you want.
2. Press `F`.
3. Type the column code, the visible column name, or the shortest unique
   prefix, and press `Enter`.
4. Type the value for that column and press `Enter`.
5. Type `all` at the column prompt to clear the current filter.

NOTE: `Q` or `Esc` cancels the current filter prompt without changing the
active filter.

If your prefix matches more than one code, the command line stays open and
shows the matching codes so you can narrow the entry.

== Value Rules

- *Text columns:* Enter plain text. Matching is case-insensitive and looks for
  that text anywhere in the cell.
- *Number columns:* Enter a bare number for an exact match, or use
  `>`, `>=`, `<`, `<=`, `=`, or `!=`.
- *Coordinate columns:* Enter `xx,yy` for one exact sector, or `xx,yy/r` for
  a radius filter.
- *Database unknown values:* Enter `?` to match worlds where that value is
  still unknown.

Examples:

- Fleet list, `ord`: `holding`
- Fleet list, `sel`: `yes`
- Fleet list, `roe`: `>=4`
- Planet list, `coo`: `12,7/3`
- Total planet database, `own`: `#3`
- Total planet database, `max`: `>=100`

== Sorting Procedure

1. Open the table you want.
2. Press `S`.
3. Type the column code, the visible column name, or the shortest unique
   prefix, and press `Enter`.
4. Press `S` again and submit the same code, or just press `Enter` on the
   default code, to flip `ASC`/`DESC`.

NOTE: On the Total Planet Database, `rng` or `range` is also accepted at the
sort prompt to sort by distance from a chosen sector.

The active sort and active filter both appear in the table title.

== Fleet List Codes

#figure(
  table(
    columns: (auto, auto, 1fr),
    align: (left, left, left),
    inset: 6pt,
    table.header(
      [Code], [Column], [Notes],
    ),
    [`id`],  [Fleet ID], [Fleet number],
    [`loc`], [Location], [Current sector],
    [`ord`], [Order], [Also accepts `holding`, `moving`, and `combat`],
    [`tar`], [Target], [Mission target sector],
    [`spd`], [Speed], [Current speed],
    [`eta`], [ETA], [Text match on the ETA column],
    [`roe`], [ROE], [Rules of engagement value],
    [`ars`], [Armies], [Loaded armies],
    [`shi`], [Ships], [Ship and force summary text],
    [`sel`], [Selected], [Use `yes` or `no`],
  ),
)

== Planet List Codes

#figure(
  table(
    columns: (auto, auto, 1fr),
    align: (left, left, left),
    inset: 6pt,
    table.header(
      [Code], [Column], [Notes],
    ),
    [`coo`], [Coord], [Planet coordinates],
    [`pla`], [Planet], [Planet name],
    [`max`], [Max], [Maximum production],
    [`cur`], [Curr], [Current production],
    [`trs`], [Points], [Stored treasury points],
    [`bdg`], [Bdgt], [Build budget],
    [`rev`], [Rev], [Revenue],
    [`gro`], [Grow], [Growth],
    [`bui`], [Queue], [Build queue size],
    [`sta`], [Dock], [Docked ships],
    [`sbs`], [SBs], [Friendly starbases],
    [`ars`], [ARs], [Planet armies],
    [`gbs`], [GBs], [Ground batteries],
  ),
)

== Total Planet Database Codes

#figure(
  table(
    columns: (auto, auto, 1fr),
    align: (left, left, left),
    inset: 6pt,
    table.header(
      [Code], [Column], [Notes],
    ),
    [`coo`], [Coord], [World coordinates],
    [`pla`], [Planet], [Known planet name],
    [`own`], [Owner], [Known owner text such as `#3` or `Unowned`],
    [`max`], [Max], [Known maximum production],
    [`see`], [Seen], [Last year seen],
    [`ars`], [ARs], [Known armies],
    [`gbs`], [GBs], [Known ground batteries],
    [`sbs`], [SBs], [Known starbase count],
    [`cur`], [Curr], [Known current production],
    [`trs`], [Points], [Known stored points],
    [`sco`], [Scout], [Last scout year],
  ),
)
