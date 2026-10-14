// Nostrian Conquest — Player Manual
// Typst source — generates US Letter PDF with proper table layout

#set document(
  title: "Nostrian Conquest — Player Manual",
  author: "Mason A. Green",
  date: datetime(year: 2026, month: 10, day: 13),
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
  #text(size: 10pt, fill: luma(120))[Find games on Discord. Play in your browser, on a BBS, or locally.]
  #v(0.5em)
  #text(size: 10pt, fill: luma(120))[All code, UI, and assets in this edition are original. Not affiliated with any original release.]
  #v(0.5em)
  #text(size: 10pt, fill: luma(120))[Revision date: October 13, 2026]
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

Each maintenance marks the passage of a year. In that span, fleets cross the dark between stars, colonies grow or starve, alliances turn cold, and victory is decided by industry, cunning, and will.

Nostrian Conquest is a Rust recreation inspired by the classic 1990s BBS door game Esterian Conquest. All code, UI, and assets in this edition are original. It is not affiliated with any original release.

In homage to the 1990s door-game pioneers and to the ancient dreamers, strategists, and storytellers whose visions of galactic dominion still light the way among these stars.

#pagebreak()

// ─── 2. Connecting to a Game ────────────────────────────────────────────

= Connecting to a Game

== Choose How to Play

*Discord and browser play* is the recommended way to join a community game.
Find a game in Discord, claim an empire, and open the game in your browser.
No game client needs to be installed. The browser terminal uses *xterm.js* to
preserve the old-school BBS feel: ANSI colors, text menus, and keyboard commands
in an 80×25 screen.

*Traditional BBS play* remains available. Connect to a bulletin board with a
terminal client and launch NC from its doors menu. The sysop runs `nc-door`
behind the board. See the *Sysop Manual* section *BBS Door Setup* for hosting
instructions.

*Localhost play* runs `nc-game` directly on the same machine as the campaign.
Use it for solo play, hotseat testing, or trusted local sessions. See the
*Sysop Manual* section *Localhost Session Setup* if you are hosting it yourself.

#admonition("NOTE", [
  Decentralized play through *Nostr GameServer* is on the roadmap. It is not
  available for players to join yet. Use Discord and browser play, a BBS, or a
  local game today.
])

== Find a Game on Discord

Join the #link("https://discord.gg/eZMRpdQTu")[Nostrian Conquest Discord server]
and read the pinned start-here post. Use `/games` to find games and their IDs.
For an invite-only game, ask its host for a private code.

1. Use `/join` with the game ID. If the game needs an invite, enter your code
   too: `/join game:beta-1 code:YOURCODE`. Replace the example game ID and code
   with the ones your host gave you.
2. Use `/play` with that game ID. Open the link and sign in with the *same
   Discord account* you used to claim your seat.
3. Name your empire and issue your orders. Return through `/play` or the
   #link("https://play.nostrian-conquest.com")[browser game list].

Each invite code claims one seat. Keep it private. You only need it once;
your Discord account owns the seat afterward. Your player handle comes from
your public profile name when you claim the seat and stays fixed for that game;
ask the host to change it. Your empire name is a separate name you choose.

On hosts offering *Sign in with Nostr*, you can instead use a NIP-07 browser
extension such as nos2x. Approve the login, enter your invite code on the game
card, and choose *Play*. Return using the same Nostr key. Your player handle
comes from your Nostr profile name, or `anon` plus a number if you have none.
This option is separate from relay-based Nostr GameServer play.

On hosts offering *Sign in with Matrix*, choose it and sign in to your
matrix.org account if asked. The game learns only your Matrix ID, not your
messages or rooms. Your Matrix display name becomes your player handle. If your
account is on another Matrix server, choose *Matrix account not on matrix.org?*
instead and enter your Matrix ID, such as `@you:example.org`. The host's bot
invites you to a private room and sends a sign-in link and a code that work
once and expire after ten minutes. Return using the same Matrix account.

Discord, Nostr and Matrix players share the same games. Invite codes are
single-use across all sign-in methods.

A recruiting game waits for its host to start it. Players can join and prepare
orders while they wait. Filling all seats or opening *Play* does not start the
game. The host sets the turn schedule; `/status` shows the next deadline once
scheduled. You do not need to stay online when maintenance runs.

#pagebreak()

== Useful Discord Commands

Give the game ID when a command asks for one. Bot replies and empire downloads
are private to you.

- `/status` — Check the game year and next turn.
- `/map` — Download your known map as a ZIP with text, CSV, ODS, and Excel files.
- `/report` — Download your empire's reports. You can also read them in the game
  inbox.
- `/manuals` — Open the quick-start guide and this manual.
- `/help` — See all bot commands.

Keep a printed map or spreadsheet beside your terminal to track your plans.

== Join Through a BBS or Localhost

For a BBS campaign, connect to the board normally and launch NC from its doors
menu. Your sysop may reserve a seat for your BBS alias or leave seats open in
the game's first-time join menu. A reserved alias takes you to its assigned
empire. Discord invite codes are for community browser games, not BBS logins.

For localhost play, your sysop tells you which machine and player slot to use.
The normal direct launch is:

```sh
nc-game --dir /path/to/mygame --player 1
```

Your sysop may give you a different player number or run the command for you.

#pagebreak()

// ─── 3. Your First Turns ──────────────────────────────────────────────────

= Your First Turns

You hold one world and a handful of ships. Somewhere beyond your sensors,
other Star Masters hold the same. The galaxy will belong to whoever builds,
claims, and strikes fastest. This chapter takes you through your first year in
command. @forces describes every unit you can build, and @missions every
mission you can give.

== What You Start With

Your empire begins with:

- One homeworld at Potential 100 and Production 100, with 10 armies and 4
  ground batteries on its surface and 50 goods in its treasury.
- An empire tax rate of 50%.
- Four fleets in orbit, all on Guard/Blockade a World at ROE 6: two colony
  fleets, each of one cruiser and one ETAC (an Environmental Transformation
  And Colonization ship), and two lone destroyers.

The colony fleets move at speed 3, the ETAC's pace. The destroyers move at
speed 6, as fast as anything in the game. Send them out early to see what
lies near you.

Your objective is to become Emperor. You win when every rival empire is
Defeated and no rogue empire still holds a world (see @victory).

== How a Year Works

There is no End Turn button. You read your reports, set your economy, give
your fleets their orders, and log off. The host's schedule decides when
maintenance runs. When it does, the game resolves every empire at once:
fleets move, battles are fought, colonies grow, builds complete, and the year
advances. Your next login opens with a fresh set of reports telling you what
happened.

Each maintenance is one year. Reports are dated by week and year, as
`Stardate WK/YYYY`, so a battle report tells you when in the year the fight
took place. You need not be online when maintenance runs. The orders you give
stand until it resolves them, and a new order replaces the old one.

The pace is deliberate. Players log in on their own schedules, which leaves
time to plan, to bargain, and to read the map.

#admonition("NOTE")[If maintenance runs while you are logged in, the game cannot save your next change. The menu notice line says: "Maintenance has run; your last change was not saved. Log in again." Log in again to carry on. In a BBS door the session ends and returns you to the BBS.]

== Your First Turn

Work through these steps each time you log in. The first turn is the longest;
later turns go faster.

1. *Read your reports.* The login review scrolls your newest reports and
   messages. Note which worlds grew, where your fleets are, and whom you have
   met.
2. *Look at the map.* Press *G*, then *M*, for the full galaxy map. Find your homeworld and the worlds nearest it. Back on the Main Menu, *T* opens the Total Planet Database, which lists what you know about each world. Keep a printed map or a
   spreadsheet beside you and update it every turn: coordinates, owners,
   Potential, and where your fleets are going. Localhost players can export a
   text map and CSV files from the starmap screen. On a BBS, ask your sysop
   for the export. On Discord, use `/map`.
3. *Check your tax rate.* Press *P* for Planet Command; its Tax (*T*) sets one rate for every
   world you own. You start at 50%. Leave it there, or lower, while your
   colonies grow. Above 65%, a yearly penalty eats into Production.
4. *Queue builds.* Build (*B*) in Planet Command spends your homeworld's
   budget. An ETAC colonizes a world and survives to colonize another, a
   transport carries one army, and a ground battery or a few more armies keep
   your homeworld safe while your fleets are away. @forces lists every unit
   with its cost, and @building explains the build queue.
5. *Commission and load.* Builds finish at maintenance, so this step starts on your next visit. Finished ships wait in stardock until you commission them (*C*, or *M* for everything at once) into a fleet. Transports are
   built empty: Load (*L*) armies aboard before they sail.
6. *Give missions and ROE.* Press *F* for Fleet Command; its Mission (*O*)
   sends a fleet out. Send your two colony fleets to Colonize a World (12) at
   two different promising worlds near you, and a destroyer on Move Fleet (1)
   or View a World (9) to see what is out there. When an ETAC has founded its
   colony, send it on to the next. ROE (*C*) sets how much of a fight each
   fleet will take; set colony fleets to 0. @missions describes every
   mission, and @combat explains ROE.
7. *Log off.* Your orders stand until maintenance runs.

== Three Warnings

#admonition("WARNING")[Troop transports are built empty. A transport with no army aboard lands nothing. Load armies before you send it to invade.]

#admonition("WARNING")[Ships in stardock are a bombardment target. Commission them as soon as they are built, or an enemy may destroy them before they ever fly.]

#admonition("WARNING")[Above 65% tax, a yearly penalty eats into Production and mature worlds decline. Held steady, 65% brings in the most over the years; a higher rate buys cash now at the cost of later years, and no starbase removes the penalty.]

#pagebreak()

// ─── 4. Forces at Your Command ──────────────────────────────────────────

= Forces at Your Command <forces>

You control five major force types: planets, ships, starbases, armies, and ground batteries. The table below lists everything your worlds can build. *AS* is Attack Strength (firepower) and *DS* is Defense Strength (hull and shields).

#figure(
  table(
    columns: (auto, auto, auto, auto, auto, auto, 1fr),
    align: (left, right, center, center, right, right, left),
    inset: 6pt,
    table.header(
      [Unit], [Cost], [Speed], [Size], [AS], [DS], [Role],
    ),
    [*Destroyer*],  [05], [6], [S], [10], [08], [Fast raider and line support. Escapes heavy fleets. Good for fast response and interception.],
    [*Cruiser*],    [15], [5], [M], [30], [30], [Balanced fighter. About 3x the power of a destroyer.],
    [*Battleship*], [45], [4], [L], [90], [100], [Heavy firepower anchor. About 3x the power of a cruiser. Slow but durable.],
    [*Scout*],      [15], [6], [S], [00], [10], [Stealthy spy. A lone scout is hardest to detect.],
    [*Transport*],  [05], [5], [M], [00], [10], [Unarmed. Carries one army. Essential for conquest.],
    [*ETAC*],       [20], [3], [L], [00], [20], [Colony ship. Reusable --- survives colonization.],
    [*Starbase*],   [50], [1], [L], [100], [120], [Orbital fortress. Defense and production boost.],
    [*Ground Battery*], [20], [--], [L], [90], [20], [Planetary defense.],
    [*Army*],       [02], [--], [S], [10], [10], [Surface defense and capture.],
  ),
)

== Planets

Planets are the foundation of your empire. Each one has a *Potential Production* --- the ceiling it can reach at full efficiency, ranging from 10 to 150 across the galaxy --- and a *Production*, which is what the planet can actually deliver right now. Production grows toward Potential over time, and the speed of that growth depends heavily on your tax rate.

Taxes convert Production into spendable build points each year. See @economy for the full tax rate, growth, and starbase economics model.

== Ships

Ships operate in fleets. A fleet always moves at the speed of its slowest member and can hold anywhere from one to *3,000 ships* of mixed types.

=== Reading a Fleet

The game writes a fleet as one line of ship codes, always in the same order.
`BB2!1 CA12!3 DD8 TT*4` is two battleships (one crippled), twelve cruisers
(three crippled), eight destroyers, and four loaded transports: `!` marks
crippled ships and `*` loaded transports. In the Fleet List, a fleet too long for its column ends in `+`; Review (*R*) shows the rest. @ship-notation explains every code.

== Starbases

Starbases are large space fortresses (Cost: 50, AS: 100, DS: 120). In orbit,
they strengthen a world's defense and economy. They help weak colonies grow at
sensible tax rates, and they let a planet spend up to *5x* its Production in
one turn when enough points have been saved (see @economy). They are not a
license to tax a world to death. In deep space they serve as watch posts with
slightly more firepower than a battleship, but they crawl at speed 1.

Unlike ships, starbases do not belong to fleets. You commission them one at a
time from stardock and move them through the *Starbase Command* submenu.
Warships can escort a base with Guard a Starbase (4), but the base stays its
own unit.

Build a starbase where you mean to keep it. A base crawls one sector a year
and spends its first year just climbing out of orbit, so a base ordered to a
world a few sectors off spends years on the way. It enters orbit only over a
world you own; anywhere else it stops in deep space. Its escort, the guard
fleet at rest beside it, rides along, and your other fleets guarding it follow
wherever it goes.

A base in orbit needs your world beneath it. Lose the world and the base
leaves for deep space in its own sector. A base bound for a world you no
longer own stops where it is and waits for orders. Moving a base out of orbit
also cancels the world's build orders that depend on the base's build
capacity, so finish or trim the queue before the base leaves. @appendix-economy
gives the exact condition.

#admonition("NOTE")[Starbases must be commissioned from stardock before they can be moved or provide orbital benefits. An uncommissioned starbase sitting in stardock has no effect.]

== Ground Forces

*Armies* (Cost: 2, AS: 10, DS: 10) defend your planets from invasion and are the only way to capture enemy worlds. Each troop transport carries one army, and a successful invasion requires landing enough armies to overwhelm the defending garrison.

*Ground Batteries* (Cost: 20, AS: 90, DS: 20) are immobile land-based cannons that offer massive firepower per cost --- roughly equal to a battleship at less than half the price. During an invasion, all batteries must be destroyed before transports can land. During a blitz, surviving batteries fire directly on descending transports, inflicting heavy losses. If a planet is captured via blitz, any surviving batteries transfer intact to the new owner.

#pagebreak()

// ─── 5. Economy and Taxes ───────────────────────────────────────────────

= Economy and Taxes <economy>

Your empire runs on production. Every owned planet generates tax revenue each maintenance cycle, and that revenue pays for ships, defenses, and expansion. Managing the tension between short-term revenue and long-term growth is one of the deepest strategic challenges in the game.

== Key Terms

- *Potential Production:* the most a world can ever produce.
- *Production:* what it produces now. It grows toward Potential.
- *Revenue:* the tax a world will add to its treasury at the next maintenance.
- *Empire Revenue:* the revenue of all your worlds together.
- *Treasury:* the points a world has saved and not yet spent.
- *Budget:* how much of its treasury a world may spend in one turn.

== Tax Revenue

*One tax rate applies to every planet you own.* You cannot set separate rates
for individual worlds. Planet Command’s Tax option changes this shared rate,
regardless of which planet you have selected. Revenue per
planet per year is a fixed percentage of Production, and your empire's
Empire Revenue is the sum of that revenue across all owned planets. The screens
show next year's revenue, rounded to the nearest whole point, and setting the
tax rate reports the new Empire Revenue. Production and the Treasury keep their
fractions from year to year; only what you read and spend is a whole number.
See @appendix-economy for the exact formula.

== Growth Toward Potential

Each maintenance turn, every owned planet grows toward its Potential. Lower
taxes grow faster: a world taxed at 30% grows twice as fast as one taxed at
65%, and three times as fast if a starbase orbits it.
Growth also slows as the planet nears its ceiling: each year a planet closes
a share of its gap to Potential, so it comes close to Potential without
reaching it on a fixed date. Every point of tax costs growth, and there is no
free yearly minimum.

Growth is based on the gap to Potential and the remaining tax headroom. See
@appendix-economy for the exact formula.

== The 65% Tax Threshold

#admonition("WARNING")[Setting taxes above *65%* can directly _reduce_ Production on your planets, not just slow growth.]

Below 65%, growth is always positive --- lower is faster. Above 65%, a penalty kicks in that actively damages Production, and a mature planet settles below its Potential: with the empire-wide rate at 70%, a Potential-100 world settles at Production 88 and earns 62 a year; at 71%, it settles at 86 and earns 61. A commissioned starbase does *not* remove this danger. Its value is that it helps young colonies grow faster before you reach punitive tax rates, and it still preserves the powerful build-capacity bonus described below.

The tax rate is a choice between today and tomorrow. Your empire starts at
50%. Keep it there, or lower, while your colonies grow, and raise it toward 65%
as they near their Potential: held steady, 65% gives a mature world its highest revenue over the years. A higher rate can raise cash briefly, but starts reducing Production at the next maintenance. A
starbase's growth bonus is full at 50% or below and gone at 65%, so the worlds
with bases gain most from a modest rate.
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

#admonition("NOTE")[A starbase helps only once it is commissioned; one waiting in stardock does nothing. A base of yours still on its way down to a world raises its budget a year before it speeds its growth. Another empire's base does neither. @appendix-economy says exactly what counts.]

== Treasury <treasury>

Tax revenue that a planet does not spend accumulates in its *Treasury* --- the planet's savings reserve. The treasury is what allows starbase worlds to execute large builds: up to 5x Production in a single turn. Each turn, a planet's *Budget* is the lesser of its treasury and its build capacity. That is what the build screen shows as *BUDGET* --- how many production points remain uncommitted this turn. When maintenance processes a build queue, only the points actually spent that year are deducted from the treasury; unfinished builds keep their remaining cost for later turns. You spend whole points, and the treasury keeps any fraction. Only a world imported from a classic game can start in debt: a negative treasury buys nothing until later revenue pays it down.

== Newly Colonized Planets

A new colony starts small: Production 3, or its Potential if that is lower,
and an empty treasury. It earns nothing in the year it is founded. From the
next year it grows and pays tax like any other world, so at a moderate tax
rate most colonies have a small budget after their first full year. A very
small world, or one in an empire with a low tax rate, may wait a year or more
for its first whole point. The head start comes only with founding, never
with capture. A low tax rate grows colonies fastest but fills their treasuries
slowest. @appendix-economy gives the details.

== Conquered Planets

When you capture an enemy planet by invasion or blitz, the planet's industry needs *two quiet years* before it is converted. It pays no revenue and does not grow in the next two maintenance turns, and it pays full revenue from the third. Its Revenue reads 0 until then. Plan your logistics around this delay.

== Scorch Earth <scorch-earth>

Scorch Earth denies a world to an invader. The *Scorch* command sets the order, and the same command rescinds it. Setting it asks three confirmations; rescinding it asks one. The order changes nothing at once. It stays in force until you rescind it or the world is captured, and it first takes effect at the next maintenance, before the year's fighting, so it is already in force when an assault strikes that year.

Each year the order is in force, the planet's Production falls to between 30% and 49% of itself, never below 1, and its stored points fall to a tenth, or to their square root if that is smaller. The planet neither grows nor pays revenue, and its Revenue reads 0. Debt is left as it is. Potential Production is not touched, so a world that is not captured recovers by the normal growth rules once you rescind the order. Your maintenance report tells you each year what the order cost the planet.

If the world falls while the order stands, the conqueror gets a ruin. Its
production and stored goods are halved, its stardock is emptied, and the order
is lifted. Debt and the build queue survive. Use Scorch Earth when you expect
to lose a world and want the victory to cost your enemy dearly.

#pagebreak()

// ─── 6. Combat ─────────────────────────────────────────────────────────

= Combat <combat>

Battles fight themselves. You decide where your fleets go, what they are made
of, and how much risk each one will take; maintenance does the rest, and your
reports tell you what happened. For the full rules, see @appendix-combat.

== What You Decide

*Where each fleet goes and what it does.* A fleet's mission decides whom it
meets. Patrols and blockades watch for intruders; bombard, invade, and blitz
missions attack a world (see @missions).

*How bold each fleet is.* Every fleet has a Rules of Engagement (ROE) level
from 0 to 10. At 0 a fleet never starts a fight; higher numbers accept worse
odds. The same level decides when a fleet breaks off a battle. Your fleets in
the same place count together when they weigh the odds.

#admonition("IMPORTANT")[A low ROE does not keep a fleet safe. A fleet that is attacked fights at least the first exchange, whatever its ROE.]

Every new fleet starts at ROE 6, or at 0 if it has no warships; change it with
ROE (*C*) in Fleet Command. Scouts, transports, and ETACs never fight, so a
fleet of only those ships is always ROE 0. Such a fleet cannot guard a world or
a starbase: if a detach or transfer strips a guard fleet of its warships, maintenance sends it home, unless autopilot is running.

*What each fleet is made of.* The ships you send decide what a victory is
worth (see Cripple or Destroy below). A fleet that mixes at least two of
destroyers, cruisers, and battleships fights better than one that does not.
Your warships screen your scouts, transports, and ETACs from ordinary fire while any of them stand, though a lucky hit can get through. A fleet holds up to 3,000 ships and moves at the speed of its
slowest one.

== Cripple or Destroy <cripple-or-destroy>

Warships deal damage in two ways, and the mix in a fleet decides what its
victories are worth.

Cruisers and battleships spread their fire across the enemy line. They cripple the enemy's combat line before they finish it, and a crippled ship fights at half strength. That wins the battle in front of you, but a crippled ship that gets
away recovers on its own two turns later. A fleet of heavy ships alone can
win a battle and still face the same enemy fleet again.

Destroyers and starbases pick out ships and destroy them outright. A healthy ship costs them twice the hits of a cripple, a crippled one no more than crippling it cost, and what they destroy stays destroyed. Destroyers are also your fastest and most fragile warships; on their own they tend to lose to heavier fleets of the same cost.

Together they do what neither does alone: the heavy ships break the enemy
line, and the destroyers turn that damage into lost ships. Lean on cruisers
and battleships to hold a system or win a skirmish; add destroyers when you
mean to wipe out an enemy fleet. Ground batteries fire the way destroyers do,
which is why ships attacking a fortified world are destroyed, not crippled.

== What to Expect

Fleets meet only when they share a sector and the same depth in it: deep space,
or the solar system around its world. You fight empires you have declared
enemies, and any fleet that enters one of your solar systems. Battles happen
during the year, wherever fleets are that week, and your report gives the
stardate.

Both sides fire at the same moment, so there is no advantage in shooting
first. Heavy fire cripples ships before it destroys them; destroyers, starbases, and ground batteries can destroy them outright. A crippled ship fights at
half strength and recovers on its own two turns later, a turn sooner if it
rests beside one of your starbases. A fleet that catches the enemy unseen
gains an edge in the first round. Once your forces have spotted an empire in a system, keeping a fleet or starbase there preserves that warning against it.

When the odds turn against a fleet, it breaks off and heads for your nearest
world. A faster enemy may catch it on the way out, so retreat is not always
clean.

== Attacking Worlds <attacking-worlds>

You attack a world with one of three missions. First the fleet must win the
orbit: the owner's starbase and warships there fight it before it can strike,
and a fleet that loses is driven off (see @missions).

*Bombard* wears a world down. Each year the fleet stays in orbit, it fires
three rounds. Its hits go to the ground batteries first; only when none are
left do they reach docked ships, then armies, stored goods, and factories.
Bombardment therefore destroys what you might later capture. Use it to break a
world's defenses or to ruin a world you cannot hold. Battleships bombard best;
a destroyer counts only half its strength.

*Invade* takes a world intact. The fleet fires one round at the batteries, and
if any survive, no troops land and the armies stay aboard; bombard first, or
bring enough firepower to clear the batteries at once. With the batteries gone,
your ships soften the defending armies, never more than half of them, and the
transports land. Invasion never touches stored goods or factories.

*Blitz* lands at once. Your warships fire on the batteries first, and the stronger the escort, the fewer batteries are left; then your armies come down under the survivors' fire, which picks off transports before they reach the ground, but if you win, the world's
factories, goods, and surviving batteries are yours. Use it against lightly
defended worlds, or when speed matters more than losses.

On the ground, invasion and blitz fight the same battle, and the defenders fire
first. Equal numbers are a poor bet: one army against one wins a third of the
time, and large equal forces about one time in five. Half again as many is an
even chance, and twice as many wins nine times in ten. A world with no armies
falls to the first army that lands. A failed assault
leaves your survivors in orbit on Hold.

*What a capture is worth.* A captured world passes to you with whatever the
fight left standing: stored goods, production, docked ships, queued builds, and
any debt. It pays no revenue for two years while its industry converts, then
full revenue. If its owner set Scorch Earth, its production and goods are
halved and its stardock emptied when it falls (see @scorch-earth). In short,
bombardment costs you part of the prize, invasion keeps the prize but needs the
batteries down first, and a blitz keeps everything at the price of armies and
transports.

== Defending Your Worlds

*Ground batteries* are your cheapest firepower: 20 points buys the attack of a
battleship. They destroy ships outright, they must fall before an invader can
land, and they fire on transports during a blitz. A world with many batteries
costs an attacker years of bombardment.

*Armies* decide the landing. Your defenders fire first, so an attacker needs
about twice your armies to be confident, and a world with none falls to a
single transport. At 2 points each, they are cheap
insurance for any world worth taking.

*A starbase and warships in orbit* fight every hostile fleet before it can
strike. A starbase adds 1 to your die and never retreats, and an attacker that
loses the orbit is driven off without bombarding or landing. Once your forces there have spotted an empire, keeping a fleet or starbase in the system denies that empire a surprise attack.

*Scorch Earth* is a last resort. It denies a conqueror half the world's
production and goods, but every year it is in force the world loses
production, grows nothing, and pays no revenue (see @scorch-earth).

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
    [00], [Hold Position],   [Persistent], [Stay at the current position and depth.],
    [01], [Move Fleet],      [One-shot],   [Travel to a specified sector, then revert to Hold.],
    [02], [Seek Home],       [One-shot],   [Return to nearest owned planet; retargets if that planet is captured en route.],
    [03], [Patrol a Sector],          [Persistent], [Move within sector deep space to intercept enemies.],
    [04], [Guard a Starbase],  [Persistent], [Escort a starbase; fight alongside it. Needs warships.],
    [05], [Guard/Blockade a World],        [Persistent], [Watch a world. Attacks assaults, colonizers of an unowned world, and declared enemies. Needs warships.],
    [06], [Bombard a World],         [Hostile],    [Damage factories, batteries, armies, stardock contents, and production.],
    [07], [Invade a World],          [Hostile],    [Suppress batteries, bombard, then land armies. Requires loaded transports.],
    [08], [Blitz a World],           [Hostile],    [Drop armies at once, under battery fire. High army risk.],
    [09], [View a World],            [One-shot],   [Quick scan of owner and production; the fleet dives in and climbs back out.],
    [10], [Scout a Sector],    [Persistent], [Watch a sector closely and identify traffic (requires Scout).],
    [11], [Scout a Solar System],    [Persistent], [Active spy mission into a solar system (requires Scout).],
    [12], [Colonize a World],        [One-shot],   [Terraform and claim unowned planet (requires ETAC). ETAC survives for reuse.],
    [13], [Join Another Fleet],      [Persistent], [Chase and merge with target fleet. Abandons if host is destroyed.],
    [14], [Rendezvous at Sector],      [Persistent], [Meet other fleets at sector. Lowest fleet ID becomes host.],
    [15], [Salvage],         [One-shot],   [Scrap ships at planet for half their build cost.],
  ),
)

== Movement and Arrival <movement>

The map is a grid of *sectors*, each named by its coordinates, such as
`[21,21]`. A sector may hold a *world*. The space around that world is its
*solar system*, and the space between sectors is *deep space*. Think of each
world as one step below its sector: a fleet crosses the map in deep space,
descends into the system to reach the world, and climbs back out to leave. A
full descent or climb costs one sector of movement, and a fleet covers as
many sectors a year as its slowest ship allows.

The map shows coordinates, not depth. A fleet in deep space and a starbase
in the system can both be at `[21,21]` without being together. They cannot
see or fight each other, and the fleet gets no repair help from the base.

You choose a mission, not a depth. *Hold Position keeps the fleet exactly
where it is*, in deep space or in the system, even partway through a climb or
a descent. Move Fleet followed by Hold Position is not an order to enter
orbit.

A fleet never moves across and down at once. It climbs out of orbit, crosses
the map in a straight line to the center of its target sector, and only then
heads down toward the world. A fleet leaving a world spends its first sector
of movement on the climb, so it covers one sector less in its first year.
Distance is measured in a straight line, so three sectors on a diagonal is
about 4.2 sectors of travel.

When a fleet arrives depends on its mission.

- *Sector missions* (Move Fleet, Patrol a Sector, Scout a Sector, Rendezvous
  at Sector) arrive the year the fleet enters the target sector.
- *World missions* continue down to the world. Guard/Blockade a World, View a
  World, Scout a Solar System, Salvage, Bombard, Invade, and Blitz arrive when
  the fleet reaches orbit. Seek Home and Colonize a World finish at the end of
  the first year the fleet is inside the system.
- *Guard a Starbase* takes up station when it reaches its base, in deep space
  or inside the system.
- *Join Another Fleet* flies to the host and merges into it, taking the host's
  position and orders. It is not a way to park an independent fleet at the
  host's coordinates.

A world mission often takes a year longer than a move to the same sector. The
*ETA* command shows the year a fleet will arrive; for a world mission, answer
yes to "Include time to enter system?" @appendix-combat gives the exact rule
for when a fleet counts as inside a system.

#pagebreak()

=== Where Missions Operate

This is the space where each mission does its work, not a list of every layer
it crosses in transit. The orders tables describe requirements and duration;
this table explains positioning.

#include "includes/mission_operating_space.typ"

=== Repairing at a Starbase

Crippled ships mend on their own, but a starbase speeds the work. Send a
damaged fleet to your starbase with Guard a Starbase (4) and leave it at rest
beside the base, inside your own system, for a whole turn: its older cripples
come back a turn early. A fleet that only shares the sector, or that arrives
during that turn, gains nothing. Unarmed fleets cannot guard a base. A fleet on Guard a Starbase may merge into the base's escort when it arrives, so check your fleet list afterward. The exact conditions are in @starbase-recovery.

== Mission Details

=== One-Shot Missions

*Mission 1: Move Fleet.* A simple transit order. The fleet travels to the destination sector at the speed of its slowest ship, then stops and reverts to Hold Position. You must issue new orders if you want it to do anything else.

*Mission 2: Seek Home.* The fleet travels to the nearest planet you own. It keeps that planet for as long as you own it, even if another becomes nearer on the way. If you lose the planet while the fleet is en route, the fleet turns at once for the nearest planet you still own, measured from where it is; with none left, it stops and holds where it is. On arrival, it reverts to Hold Position. The game picks the planet; you enter no coordinates. When you order a single fleet that is already in the system of its nearest planet, the game asks "Do you wish this fleet to seek the next closest system?" Yes, the default, sends it to the next closest planet; no keeps it on the planet it is at. If that planet is the only one you own, the game says so and gives no order. In a group order each fleet seeks its own nearest planet without asking.

*Mission 9: View a World.* A quick look. The fleet dives into the target system, scans for owner and production data, and climbs straight back out into deep space. While it is in the system, the owner's fleets may spot and attack it. It reverts to Hold Position after reporting.

*Mission 12: Colonize a World.* Requires at least one ETAC (Environmental Transformation And Colonization ship). If the planet is unowned, it is terraformed and claimed. The new colony starts with one garrison army, no treasury, and very low current production. It does not receive revenue or growth until later maintenance turns, and then develops faster when the empire-wide tax rate is low. The ETAC is not consumed --- it survives and can colonize additional planets. If the planet is already owned, the ETAC aborts, reports the owner's identity and production potential, writes that partial result into your Total Planet Database, and waits for new orders.

*Mission 15: Salvage.* The fleet travels to one of your planets and scraps its ships for half their build cost, rounded up ship by ship, returned to that planet's treasury: 23 points for a battleship, 8 for a cruiser, 3 for a destroyer, 3 for a transport (loaded or empty), 10 for an ETAC, and 8 for a scout. Armies aboard loaded transports land on the planet, and the fleet is disbanded. If you lose the planet while the fleet is en route, the fleet salvages at the nearest planet you still own instead; with none left, it stops and holds where it is. The game offers the planet nearest the fleet as the default target; if you own only one planet, it takes that planet without asking. A group salvages at one planet.

=== Persistent Standing Missions

*Mission 0: Hold Position.* The default idle state. The fleet stays at its current location and takes defensive action based on ROE if hostile fleets approach.

*Mission 3: Patrol a Sector.* The fleet watches a sector, looking ten times as hard as an ordinary fleet once it arrives. It identifies everything it spots, sends a report, and attacks what the rules in Who Fights Whom (see @appendix-combat) allow, as far as your ROE permits. The mission remains active until you assign a new one.

*Mission 4: Guard a Starbase.* The fleet escorts a starbase and fights alongside it: an attack on either one is a battle against both, and the base joins any battle its guard is drawn into as a defender. A guard stays with its base and does not close on the contacts it spots, but it attacks, or joins an attack by your other fleets there, when the ROE odds favor the fight.

*Mission 5: Guard/Blockade a World.* A strategy of denial. The blockade watches the world five times as hard as an ordinary fleet. It attacks any fleet that comes to bombard, invade, or blitz the world, or to colonize it while it is unowned, and any fleet of an empire you have declared an enemy, including ships leaving the world. Neutral fleets come and go. Blockading another player's world takes your fleet into his solar system, so his fleets there will attack it. It remains active until you assign a new mission.

*Missions 10 and 11: Scout a Sector / Scout a Solar System.* Both require at least one Scout ship, and a fleet consisting of a single Scout is the least likely to be detected. Scout a Sector is a stealthy watch over the sector: a scout on station looks five times as hard as an ordinary fleet and identifies what it spots, and with ROE 0 it never attacks. Scout a Solar System is an active spy run where the Scout penetrates the system to report on ground batteries, armies, current production, stardock contents, and orbiting fleets; entering the system means the owner's fleets there may attack it. Both are persistent missions: the fleet remains on station and generates a new report each maintenance turn for as long as it stays at its assigned sector. If you identify an *In Civil Disorder* fleet in a system that still belongs to that disorder state, the Total Planet Database marks that world's owner as `ICD` even before a full scout/view report is earned.

*Missions 13 and 14: Fleet Coordination.* Mission 13 (Join Another Fleet) causes the fleet to chase a specific host fleet and merge with it when they meet. If the host is destroyed before they rendezvous, the joining fleet abandons the mission. Mission 14 (Rendezvous at Sector) sends multiple fleets to a sector where the fleet with the lowest Fleet ID becomes the host of the combined force. The rendezvous point remains active so additional fleets can keep merging there.

=== Hostile Missions

Bombard a World (6), Invade a World (7), and Blitz a World (8) send a fleet
into orbit around another empire's world to attack it. What each one does to
the world, and which to choose, is in @attacking-worlds; the exact rules are
in @appendix-combat. Here is what you need to know about getting there.

*The fleet attacks the year it reaches orbit.* A fleet already in orbit that
is told to attack the world attacks at the next maintenance.

*It must first win the orbit.* The owner's starbase and any fleet of his with
warships there fight your fleet the week it arrives, whether they had spotted
it or not. If your fleet's ROE will not take them on, it turns for home
without a shot; if it loses, it is driven off. Only a fleet that holds the
orbit attacks. Bring enough to win that fight, or clear the orbit first.

*Bombard repeats; Invade and Blitz are single assaults.* A bombarding fleet
stays in orbit and fires again every year until you change its orders. An
invading or blitzing fleet makes one assault and then, win or lose, its
survivors hold position in orbit. Order any further attack yourself.

*A fleet in orbit on a hostile mission is very easy to see.* It has entered
the owner's solar system, where every fleet is easier to spot, and a fleet at
its target on an attack mission stands out above all. Expect his fleets to
find it and attack it, and set its ROE to match.

Only destroyers, cruisers, and battleships fire at a world; scouts,
transports, and ETACs carry no guns. Invade needs loaded transports and
warships. Blitz needs loaded transports. It can go in without warships, but
then it brings no fire of its own, and if the owner has a starbase or a
warship fleet in orbit, the transports turn for home.

#pagebreak()

// ─── 8. Interface and Commands ──────────────────────────────────────────

= Interface and Commands

Three keys on the *Main Menu* open your command menus: General Command (*G*) for autopilot, diplomacy, messages, reports, and the full galaxy map (*M*); Planet Command (*P*) for economy and production; and Fleet Command (*F*) for movement and missions. Other keys act at once: *I* looks up a planet, *T* opens the Total Planet Database, *B* and *D* give brief and detailed empire reports, and *V* shows the part of the map around you.

== Visual Themes

The `nc-game` client supports several visual themes. In a local session, open
*C>olor Theme* from the Main Menu or First Time Menu to choose one. Your last
local choice is remembered for that empire in that campaign.

In BBS door mode, the game keeps the classic *A>nsi color ON/OFF* toggle.
Each session starts from the `mag16` theme so ANSI16 terminals stay
stable. Press *A* to switch between that view and monochrome for the current
session. Saved local theme preferences do not apply in door mode.

== General Command

General Command handles empire-wide business.

*Autopilot (A)* minds your empire when you cannot. It switches itself on after
you miss three turns, and your next return switches it off. Turn it on by hand
before a long absence and it stays on until you turn it off. While it runs, it
gathers your idle fleets at your best worlds and sets their warships to guard
them; fleets on any other mission are left alone. @autopilot gives the details.

*Diplomacy (E)* sets each rival empire to Neutral or Enemy, and the choice
decides where your fleets fight. Your fleets attack an Enemy's fleets wherever
they meet them, as far as their ROE allows. A Neutral's fleets are left alone
unless they enter one of your solar systems, come to attack or colonize a world you blockade, race your fleet to colonize the same unowned world, or guard a world your fleet has come to attack or colonize. His declaration
against you changes nothing about how your fleets behave. There are no allies:
no other empire ever joins your battles, and a battle never changes anyone's
diplomacy.

*Messages (C)* lets you write to other empires. In the message editor,
*^E* sends, *^X* cancels, and *^G* shows help; *?* types a question mark.
Messages reach their recipients at the next maintenance. Your *Sent* folder
shows *Queued* until delivery, then the year sent. *Enter* reads a message;
*D* asks before cancelling queued mail or removing a delivered copy from your
view. Removing your sent copy does not affect the recipient's copy. *D* on
the recipient list also opens your queued messages.

You may queue up to three messages to each opponent per turn. The inbox has
three folders: *R* for Reports, *M* for received Messages, and *S* for Sent.
*Y* filters the selected folder by year. *D* removes one selected item after
confirmation. General Command's *Delete ALL reports* clears only Reports;
it leaves Messages and Sent alone. All reports and messages remain in the
game database: removal only hides them from your view. Cancelled mail also
stays stored, but is never delivered.

New reports and messages from the latest maintenance appear in the scrolling
login review. You can read and remove them there before reaching the main menu.

The empire rankings table shows each joined empire in one of three
states: *Active*, *MIA*, or *Defeated*. *MIA* means inactivity autopilot is
currently running because that player missed three consecutive turns.
*Defeated* means the empire has been eliminated from active command.

== Defeat, Recovery, and Victory <victory>

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
final review, you may no longer enter the campaign. At the next maintenance,
your planetless empire becomes a computer-controlled rogue empire. Its
remaining warships patrol; this does not restore your ability to play or
take away your final review.

*Rogue empires* are run by the computer. A seat no player has claimed stands
"In Civil Disorder": its empire grows nothing and gives no orders. A defeated
empire also falls into civil disorder, and once it holds no world it turns
rogue. A rogue that still holds worlds gathers its fleets at its best world
and guards it hard. Rogue empires are never eliminated. A rogue empire that loses
its last planet sends its warships on patrol, marks every player's empire an
enemy, and disbands its other fleets and starbases. Without a planet it does
not contend for the throne, and taking its last planet delivers no final blow.

When you eliminate another empire, Fleet Command reports that you delivered the
final blow. This is separate from overall game victory. You become Emperor,
and the game ends, when no other empire remains in contention: every rival is
Defeated, and no rogue empire still holds a world. At that point every still-playable empire receives a game-over report
naming the victor. The winner may continue to log in in *survey mode* to
inspect the final state of the galaxy, but may not issue orders or submit
turns. All other players receive one final review-only pass and are then
blocked from further play.

== Planet Command

Planet Command controls your economy and ground operations. Tax (*T*) sets the rate for every planet you own, not just the selected planet. From the main Planet Command menu, Scorch Earth (*S*) sets or rescinds the order that wears down your own industry to deny it to an invader (see @scorch-earth). Build (*B*) spends production points on ships, defenses, or starbases. Commission (*C*) assigns newly built ships from stardock into active fleets. *Mass Commission* (*M*) commissions every ship and starbase currently waiting in stardock. Load and Unload (*L* / *U*) move armies between the planet surface and troop transports.

The *Planet List* shows your worlds one row each. Highlight a world and act on it: Build (*B*), Display Queue (*D*), Abort Builds (*A*), Mass Commission (*M*), Commission (*C*), Load / Unload armies (*L* / *U*), and Scorch Earth (*X*). On that screen, *S* is reserved for *Sort*, while *I* or *Enter* opens planet information for the selected row. Owned-planet information includes the planet's *Budget* as well as its *Treasury*.

== Fleet Command

Fleet Command controls your ships in space. Mission (*O*) assigns missions 0--15. ROE (*C*) changes a fleet's rules of engagement, ID, or speed. A speed change takes 1 up to the fleet's maximum and touches nothing else; a fleet on Hold, or already at its target, keeps speed 0. Merge (*M*) combines fleets that are in the same sector. Transfer (*T*) moves individual ships between fleets.

The *Fleet List* lets you work on many fleets at once through its *Sel* column. Press *Space* on a fleet row to check or uncheck it. When one or more
fleets are checked, *O* assigns one mission to the checked set, *C* changes
their shared ROE or speed, *M* merges the checked fleets using the lowest
Fleet ID as the host, and *T* opens ship transfer for a checked pair. Row-based
commands such as *Review*, *ETA*, *Detach*, *Load*, and *Unload* still use the
highlighted fleet.

Crippled ships show in the Ships cell of the Fleet List, in *Review*, and in
*Detach* and *Transfer*, always in the notation of @ship-notation. *Review*
adds the short legend under the composition of a damaged fleet. Your
maintenance report says when ships have recovered.

*Detach* (*D*) and *Transfer* (*T*) stage ships one class at a time. Type a class
code (`BB`, `CA`, `DD`, `TT*`, `TT`, `SC`, or `ETAC`), then a quantity. A plain
quantity such as `3` takes healthy ships first and takes crippled ships only
when the class has too few healthy ones. A quantity followed by a bang, such as
`3!`, takes exactly three crippled ships; the game refuses it when the fleet has
fewer, and it never fills the gap with healthy ships. When the selected class
has crippled ships left to stage, `1! = 1 crippled` appears above the quantity
prompt. Among crippled ships, those
crippled most recently go first, then those with the most recovery time left.
The *Staged* line shows the ships that will move, in the same notation: `CA3!1`
is three cruisers, one of them crippled. Moving crippled ships never repairs
them or restarts their recovery, and each keeps the time it has left.

In *Detach*, type *D* to finish staging early. When only one ship remains on the
donor, confirmation opens automatically. Press *Enter* or *Y* to detach the
staged ships, *N* to return to editing, or *Q* to cancel. While editing, *X*
clears the staged selection. If the donor needs a lower speed after detaching,
the game asks for it before moving the ships.

*Transfer* uses the same staging and confirmation flow. Type *T* or press
*Enter* at an empty class prompt to finish staging early. Confirmation opens
automatically when only one ship remains on the source. Review the staged
ships, source remainder, and destination after transfer; then press *Enter*
or *Y* to transfer, *N* to keep editing, or *Q* to cancel. *X* clears staging
while editing.

== Building and Commissioning <building>

Each planet keeps a build queue, and your *budget* is the only limit on it: you may order any number of units of any kind, so long as the budget covers them. During maintenance, a planet processes as many queued build points as its current per-turn build capacity allows. Small orders may finish in one maintenance turn, while larger ones can stay queued across multiple years until the remaining cost reaches zero. As enough points are applied to complete individual units, ships and starbases move to *Stardock* --- a holding area on the planet where they sit idle and vulnerable until commissioned --- even if other units from the same order remain queued. Ships are commissioned into numbered fleets, while starbases are commissioned individually and managed through their own Starbase Command submenu. Armies and ground batteries, by contrast, deploy directly to the planet surface and do not pass through stardock.

Aborting the queue releases the points still unpaid. Points already spent on a part-built unit are not returned.

A planet without a starbase can spend up to its Production, rounded to the nearest point, in a single turn. A planet with an orbiting starbase can spend up to *5x* its Production, drawing from its treasury (see @treasury). The build screen shows *BUDGET* --- your treasury capped by build capacity, minus what you have already committed this turn.

#admonition("NOTE")[A world holds at most 255 armies and 255 ground batteries. An order that would pass either limit waits in the queue and spends nothing until there is room.]

#admonition("WARNING")[Troop transports are built empty. You must manually load armies onto them before sending them into battle.]

#admonition("WARNING")[Stardock contents are a prime target for enemy bombardment. Commission your ships promptly or risk losing them before they ever see combat.]

#admonition("NOTE")[The starmap can be exported as a TXT file, a CSV grid, and a CSV details sheet for offline planning. Local Rust-client play can export it directly from the in-game starmap view, and BBS operators can hand the exported files to players out of band if they want offline planning aids.]

#pagebreak()

// ─── 9. Strategy ────────────────────────────────────────────────────────

= Strategy

The rules are the same for every Star Master. What separates the Emperor from
the defeated is judgment: where to send the first ETAC, when to raise taxes,
which fights to take and which to walk away from. This chapter gives you a
starting doctrine. Your rivals will teach you the rest.

== Early Game: The Land Grab

The opening turns are a race for territory. Every unowned world you claim is
one a rival cannot, and the sooner a colony is founded, the more years it has
to grow.

*Build ETACs first, and escorts to go with them.* An ETAC costs 20, colonizes
a world, and survives to colonize the next. One ETAC can found a chain of
colonies if you keep it alive, so give each one a warship or two for company
and keep the fleet at ROE 0: its job is to arrive, not to fight. The escort
shields the ETAC if someone attacks. Your two starting colony fleets already
have a cruiser each; send them out on the first turn.

*Choose colonies by Potential and distance.* The Total Planet Database lists
the Potential of every world you know. A world of Potential 100 will one day
pay five times what a world of Potential 20 pays, but a close world pays
sooner than a far one, and an ETAC moves at speed 3, which slows every fleet
it travels with. Take the best worlds within a few sectors first, then reach
farther. Your destroyers, at speed 6, are pickets for early contact: a View a
World (9) order reports a world's owner and production. For a quiet look at a
world's defenses, build a scout; a lone scout is the hardest fleet to see.

*Garrison what you take.* A world with no armies falls to the first army that
lands. Armies cost 2 points each, and a handful on every colony turns a cheap
grab into a real assault. Add ground batteries, at 20 each, to the worlds you
cannot afford to lose: each one fires like a battleship and must be destroyed
before an invader can land.

*Keep taxes modest.* Your empire starts at 50%, and that is a sound rate while
you expand: your colonies grow faster, and any starbase you build gives its
full growth bonus at 50% or below. Resist the urge to tax higher for cash.
Above 65%, a yearly penalty eats into Production, and no starbase removes it.

== Rules of Engagement by Job

ROE decides which fights a fleet starts and when it leaves one. Set it for
the job, not the ship. The thresholds are in @appendix-combat.

- *0* for scouts, colony fleets, and anything carrying cargo you cannot
  afford to lose. The fleet never starts a fight. A fleet with no warships is
  0 whatever you set.
- *2 to 4* for raiders that should pick only easy fights: fast destroyers
  hunting lone transports and stragglers.
- *6*, the default, for most fleets. They fight at even odds or better.
- *7 to 9* for defenders and blockades that must hold their station at a
  cost. A fleet at 6 breaks off when the odds turn against it; a guard at 8
  stays even when outgunned two to one.
- *10* for a last stand. The fleet accepts any odds against a hostile fleet and never breaks off while it has a warship left.

A low ROE does not keep a fleet safe. Any fleet that is attacked fights at
least the first exchange. A low ROE keeps a fleet from starting fights it
should not; it does not stop an enemy from starting one.

== Mid-Game: Consolidation

When the nearby unowned worlds are gone, the game turns to fortification,
intelligence, and the first wars.

*Build starbases on young worlds you will keep.* A starbase speeds a world's
growth at modest tax rates and lets it spend five times its Production in one
year from its savings. Both bonuses pay most on a world with a long way to
grow and a treasury to draw on. A world already at its Potential gains only
the build capacity and the defense. Build the base where you mean to keep it:
it crawls one sector a year.

*Raise taxes as worlds mature.* A world near its Potential has little growth
left to lose. Move the rate toward 65% as most of your worlds get there: held steady, 65% gives a mature world its best revenue over the years. Tax higher only in an emergency: you get cash now and smaller worlds later. One rate
covers every world, so time the change for the empire as a whole.

*Scout before you invade.* Never attack a world blind. Scout a Solar System
(11) reports its batteries, armies, production, stardock, and the fleets in
orbit; View a World (9) reports only its owner and production. Count the
batteries and the armies, then bring what the count demands: the batteries
must be down before an invasion can land, and you want twice the defender's
armies to be confident of the landing (see @attacking-worlds).

*Choose your enemies.* Diplomacy knows only Neutral and Enemy. Declare an
empire Enemy and your fleets attack its fleets wherever they meet, as far as
their ROE allows. Leave it
Neutral and your fleets let its fleets pass, except in your own solar systems
and at worlds you are guarding or attacking. There are no alliances. Nobody
ever joins your battles, and any pact you make rests on the other empire's
word alone. You cannot fight everyone at once, so fight one rival at a time
and keep the rest Neutral until you are ready.

== Late Game: Total War

Once the raw worlds are gone, the only way to grow is to take worlds from
others.

*Mix your fleets.* A fleet with at least two of destroyers, cruisers, and
battleships adds one to its die. Cruisers and battleships cripple the enemy
line; destroyers destroy what is crippled. Every warship buys the same
firepower per point, but the heavier ships last longer, and an empire that builds nothing but destroyers tends to fall behind rivals with heavier, mixed fleets. Build destroyers for speed and for the kill, not as your whole navy
(see @cripple-or-destroy).

*Match the attack to the prize.* Bombard a world you mean to ruin or whose
batteries you must break; it destroys what you might capture. Invade a world
you want intact, once its batteries are down. Blitz a lightly defended world
when speed matters more than losses; it keeps everything, surviving batteries
included. A captured world pays nothing for two years, so take worlds early
rather than late, and do not count on their revenue to fund the next
campaign.

*Deny what you cannot take.* A blockade attacks an Enemy's fleets as they come
and go, and a bombarding fleet grinds a world's production down year by year.
If you cannot hold a world, scorch it. Scorch Earth wears your own world down
while the order stands, but a conqueror gets only half its production and
goods, and an empty stardock. Use it as a last resort, when the world is lost
whatever you do (see @scorch-earth).

You become Emperor when every rival is Defeated and no rogue empire holds a
world. Every world you keep and every fleet you bring home whole is a year
closer to the throne.

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

// Appendices are lettered, and references to them read "Appendix A".
#counter(heading).update(0)
#set heading(numbering: "A.1.", supplement: [Appendix])

= Economy Formula Reference <appendix-economy>

This appendix collects the exact economy formulas in one place. In every formula, `tax_rate` is the same empire-wide setting for all owned planets.

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

Moving a base out of orbit cancels every build order of the world beneath it
that relies on the base: when a queued unit, or the whole queue, costs more
than the world's Production and no other starbase of yours rests in orbit
there.

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
treasury. This applies only when the colony is founded, never on capture,
recapture, or recovery, and the tax rate does not change it. The maintenance
that founds the colony credits it no growth and no revenue; both begin the next
year, growth first and then revenue on the grown Production. The treasury
counts whole points only, so a very small colony, or one taxed at a low rate,
can sit at zero budget for a year or more.

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

// ─── Appendix B. Combat Rules and Tables ───────────────────

= Combat Rules and Tables <appendix-combat>

Battles resolve automatically during maintenance. This appendix gives the exact
rules and tables behind them.
Battle reports are short, but the fighting is not random brawling. Combat is
simultaneous and rule-driven. Both sides fire in the same round. Losses are
then applied. This keeps battle outcomes clear, severe, and consistent.

== The Rules of Battle

Each round, both sides fire at the same instant, and nobody fires first. Each side rolls one ten-sided die, and the roll decides how much of its total *Attack Strength (AS)* lands on the enemy: a quarter on 0--2, half on 3--5, and all of it on 6--9. A bigger fleet wins by bringing more AS, not by rolling on a better table. Rounds repeat until one side is destroyed or breaks off.

Ships take damage in two steps, from "Nominal" to "Crippled" and then destroyed. Under suppression fire, all nominal ships in the current target group (the combat line first, auxiliaries afterward) must be crippled before any crippled ship in it is destroyed; execution fire and critical hits destroy ships directly (see Tactical Roles and Split Fire). A crippled ship fights at half strength and keeps its speed. Recovery is automatic at the end of the second maintenance turn after the damage turn, with no order, payment, or shipyard. It fights crippled during those two turns and is whole for the next. A fleet that rests in orbit of your world beside your starbase for a whole turn gets its older cripples back one turn sooner. The bonus does not apply to fresh damage or to fleets merely sharing the sector. The fleet screens show how many ships of each kind are crippled (see @ship-notation). Ordinary fleet fire targets the combat line first --- destroyers, cruisers, battleships, and starbases. Scouts, transports, and ETACs are protected from it as long as any combat-line ships remain, so your non-combat vessels survive as long as your warships hold the line, except against a critical hit.

Each damage step costs the target's full *Defense Strength (DS)*: once to cripple a ship and again to finish it. Hits too few for the next step are not wasted. They carry over to the next round of the same battle, so even a small force wears down a big ship if it stays in the fight. Leftover hits last only as long as the battle. When it ends, whether by destruction, withdrawal, or the round limit, they are lost, and the next battle starts from zero, even against the same enemy fleet. A fleet that breaks off forfeits the hits it has built up against the enemy.

Three bonuses add to your die roll. A fleet that mixes at least two of
destroyers, cruisers, and battleships gets a *combined arms bonus* of +1, so
mix your composition when you can. A starbase fighting on your side adds
another +1. An attack that catches the enemy unprepared adds a third +1 to
the attacker's first round (see *Surprise* under Who Fights Whom). The
bonuses never add more than +2, and a roll pushed past 9
counts as 9. An unmodified 9 is a critical hit that destroys one more enemy
ship outright. In a draw, the defender wins. The exact ROE thresholds, combat values,
effectiveness table, and assault formulas follow later in this appendix.

== Who Fights Whom

Fleets meet only when they share a sector *and* a depth. Each sector has two
layers: deep space, and the solar system around its world (see @movement). A
fleet at least halfway down to a world is in its system. A fleet in orbit
cannot see a fleet passing overhead in deep space, and neither can fight the
other. Movement is 2.5D: the map is a flat board with each world one step
below its sector, and a full descent or climb costs one sector of movement.
Seek Home and Colonize a World finish at the end of the first year the fleet
is at least halfway down; every other world mission acts only from full orbit.

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
target on a bombard, invade, or blitz mission is twenty times easier again (a
hundred times, counting the solar system),
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
time it identifies a target as on every later week (see Rules of Engagement
Thresholds below).
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

*Surprise.* Your empire remembers, in each sector and layer where it has a
fleet or a starbase, which opposing empires it has spotted there. Whenever any
of your fleets or starbases there spots an alien unit, its empire goes into
that memory, and so does the empire of any fleet you fight there. All of your
fleets and starbases in that sector and layer share it. The memory lasts,
from week to week and year to year, for as long as you keep a fleet or a
starbase in that sector and layer; when the last one leaves or is destroyed it
is gone, and an opponent that leaves and comes back does not erase it. It is
local readiness, not knowledge of the whole empire: what a scout learned in
another sector does nothing here.

An attack is a *surprise* when the defending empire's memory there does not
include the attacker's empire: none of its fleets or starbases there has
spotted the attacker, and the two have not fought there. The attacking side
then adds 1 to its die in the first round of the battle. The bonus counts
within the +2 limit, an unmodified 9 is still the only critical hit, and a
pursuit round never gets it. Only the attacker can surprise: a defender
that was hidden earns nothing for it. Spotting is not the same as
identifying, and a fleet that spots an enemy and declines to attack, because
of its ROE or because the odds are against it, still remembers. The battle
report says "We attacked by surprise." or "The enemy attacked by surprise."
Staying unseen therefore keeps your chance of a surprise, and watching a
system keeps your defenders ready.

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
- *Execution Fire (Destroyers and Starbases):* These units use *focus fire* to eliminate specific targets. Their hits bypass the crippled state and allocate directly to the *destroyed* pool, paying 2x DS for a healthy ship and 1x DS for a crippled one to blow ships up one by one.

This creates a deadly pair. Heavy ships soften the line. Destroyers and
starbases finish it. Each pool carries its own leftover hits: suppression hits
never pay for an execution kill, and execution hits never pay for a cripple.

#admonition("NOTE")[All planetary return fire from ground batteries is treated as *Execution Fire*. Fortified worlds do not "soften" an attacker; they destroy him.]

== Fleet Limits

A fleet can contain as few as one ship and as many as *3,000 ships* of mixed types. A fleet always moves at the speed of its slowest member. A fleet sent to guard a starbase joins the escort already beside it when it arrives, unless the two together would pass 3,000 ships.

== Rules of Engagement (ROE)

You assign an ROE level (0--10) to control when your fleet attacks. ROE 0
never attacks, but a fleet on ROE 0 that is attacked still fights the first
exchange. Any higher ROE attacks a hostile fleet when the force ratio of the
two sides, your fleets in the sector and layer against the enemy's, meets the
ROE threshold (see Who Fights Whom); seeing the enemy first changes nothing.
ROE also decides when your fleet breaks off a battle: after the compulsory
first exchange it checks the same odds after every exchange and leaves as
soon as they fail (see Withdrawal and Retreat).

The full ROE threshold table follows below.

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

*Winning the orbit.* Before a fleet on a bombard, invade, or blitz mission can
strike, it must get past whatever the owner keeps in orbit. His starbase and
his fleets there with warships fight it the week it reaches orbit, spotted or
not, and your other fleets in that orbit join it under their own ROE. If its
ROE will not take them on with the side it has, it turns for home without a
shot; if it loses, it is driven off, and the batteries do not fire on it as it
goes. Only a fleet that holds the orbit attacks, in that same week. Each fleet
attacks on its own as it arrives, so two of your fleets at one world strike
one after the other, and the second need not fight defenders the first has
beaten. A fleet told to attack the world it already orbits attacks at the next
maintenance. A fleet of loaded transports may attempt a blitz without
warships, but then it brings no orbital fire of its own; the owner's ships
without warships do not stop it, but his starbase or any fleet of his with
warships in orbit does, and the transports turn for home.

#admonition("WARNING")[Planetary defenses use *focus fire*. Hits from ground batteries bypass the crippled state and directly destroy ships, making them far more lethal than dispersed ship-to-ship fire. A commander who brings a large fleet to orbit a fortified world will find his ships being picked off one by one by concentrated ground fire.]

Each bombardment turn resolves three rounds of fire. In rounds 1 and 2, your ships trade fire with the planet's batteries. While batteries stand at the start of a round, your hits land on the batteries only, and hits beyond what destroys them are wasted. Ships in the stardock are exposed from the next round, the first that begins with no batteries standing. Armies, stored goods, and factories are shielded in these rounds. In round 3, if all batteries have been destroyed, your remaining firepower breaks through to the stardock, then armies, stored goods, and factories. If batteries still stand, round 3 is another suppression exchange and the planet's vulnerable assets survive another turn.

Planetary targets have defense strength just like ships. Destroying a ground battery takes 20 hits, an army 10 hits, and a ship in the stardock as many hits as its own DS. A lone cruiser cannot level a fortified world in a single volley.

This means a well-defended planet takes sustained bombardment over multiple turns before you touch its production. Build batteries to buy time; bring heavy fleets to break through faster.

@missions explains when a hostile fleet arrives and attacks, and @attacking-worlds what each attack is for. The exact bombardment weights, batteries-only return-fire rule, and combat tables follow below.

=== The Landing Battle

Armies that land fight on the same table as fleets, army against army, with no force-ratio columns. Each round the *defenders fire first*. Each side rolls one `d10` for its whole army: 0--2 deals 25%, 3--5 deals 50%, and 6--9 deals 100% of its armies' strength. The attackers' losses come off, and the attackers who survive fire back the same way. Rounds repeat until one side has no armies left.

Damage short of a whole army carries into the next round of the same landing and is lost when the landing ends. There is no defender bonus, no homeworld bonus, and no critical hit; firing first is the defender's advantage. The attackers fire only with what survives, so both sides are never destroyed together. Your reports show whole armies lost and remaining; the carried fractions stay hidden.

Capture odds follow the ratio of landed armies to defending armies, roughly the same at any scale. Equal numbers take a world about one time in five. At 1.5 to 1 the odds are about even, and at 2 to 1 the attackers win nine times in ten. Overwhelming numbers still win every time. The Landing Battle table below gives the figures.

=== Captured Worlds

A world falls when its last defending army is destroyed, and your surviving armies become its garrison. It passes to you with what the fight left standing: its stored goods, production, docked ships, and queued build work. There is no capture bonus, no loss for the kind of attack, and no forgiveness of debt. Your capture report lists the goods, the docked ships, and the queued work you inherit.

A defender can set Scorch Earth to deny a world to the invader. If the world falls with the order set, its production and positive goods are halved, its stardock is emptied, the order is lifted, and production is raised to at least 1.0. A world with no potential stays at 0, debt is untouched, and the build queue is not burned. The capture report says when the defenders had scorched the world. A failed assault leaves the order set.

=== Failed Assaults

Failure is not defeat. A fleet whose landing fails, or whose Invade is blocked because batteries survived suppression, keeps its survivors in orbit on Hold and receives a failure report. A blocked Invade lands nothing and its armies stay aboard. The fleet does not blockade the world on its own, and like any fleet in orbit it is open to the ordinary encounter rules. A fleet beaten in the orbital battle before it can strike turns for home under the withdrawal rules above; the world's batteries fire on no fleet except one that strikes.


== Starbase Recovery <starbase-recovery>

Suppose your world and commissioned starbase are at `[21,21]`. A fleet on Move
to Sector (1) stops in deep space there. Hold (0) leaves it in deep space;
matching coordinates does not put it beside the base inside the system.

For a damaged fleet with warships, use Guard a Starbase (4) and select your base.
Let the fleet reach the base inside your owned system. Keep the fleet and base
at rest there for the whole following turn. You may leave Guard a Starbase in
force, or change to Hold after it reaches the base. Guard fleets may merge
with an existing escort; check your fleet list after maintenance.

The bonus is automatic, not a Guard a Starbase reward: any qualifying fleet
resting there can receive it. Unarmed fleets cannot use Guard a Starbase. Seek
Home (2) brings a fleet into its nearest owned system and finishes on Hold,
but the world chosen may not have a base. Guard/Blockade a World (5) is another
way for a combat fleet to enter a particular owned system. View (9) is not a
parking order; after scanning it climbs back out.

You must keep the world, fleet, and the same assisting base throughout the
resting turn. The base must remain at rest inside the system too. Arriving or
merging into position during that turn does not grant an immediate bonus.
Ships crippled during that turn receive no extra recovery then. Older
cripples can recover one turn sooner; a starbase cannot speed its own recovery.

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
- Surprise (the defending empire had not spotted the attacker in that sector
  and layer): `+1` to the attacker, in the first exchange only
- Total modifier: at most `+2`; a modified roll above `9` counts as `9`
- Pursuit exchange: the same modifiers, except surprise, which it never gets

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
  first, then the hulls with the most time left. Detach and Transfer
  take healthy hulls first for a plain quantity and crippled hulls, newest damage
  first, for `n!`.

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
the crippled state and allocate directly to the destroyed pool, paying `2x DS` for a healthy ship and `1x DS` for a crippled one.

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

= Preservation and Original Sources <appendix-preservation>

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
main body stays player-facing. The appendices collect formulas, tables,
list-screen codes, and interface details.

#pagebreak()

// ─── Appendix D. Interface Reference ───────────────────────────────────

= Interface Reference <appendix-table-ui>

This appendix covers the list-screen filters, the fleet notation, and the
details of autopilot.

== Table Filters

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

== Reading Fleet Compositions <ship-notation>

Wherever the game lists ships in one line, it uses one notation, and always the same order: scouts, battleships, cruisers, destroyers, loaded transports, empty transports, ETACs, starbases. A planet's stardock and build queue read the same way, with armies (`AR`) and ground batteries (`GB`) after the starbases. Each token is a ship code, the total number of ships of that kind, and, when some of them are crippled, a `!` and the number crippled. The total always includes the crippled ships. A single ship still shows its count, a kind the fleet lacks is left out, and `!0` is never written.

#figure(
  table(
    columns: (auto, 1fr),
    align: (left, left),
    inset: 6pt,
    table.header([Token], [Meaning]),
    [`SC1`], [One scout.],
    [`BB2!1`], [Two battleships; one is crippled.],
    [`CA12!3`], [Twelve cruisers in all; three of them are crippled.],
    [`DD8`], [Eight destroyers, none crippled.],
    [`TT*4!1`], [Four loaded transports; one is crippled.],
    [`TT5`], [Five empty transports.],
    [`ETAC2`], [Two ETACs.],
    [`SB1!1`], [One starbase, crippled.],
  ),
)

A fleet of `BB2!1 CA12!3 DD8 TT*4!1 TT5 ETAC2` therefore holds nine transports, five empty and four loaded: the star marks cargo, the bang marks damage, and the two never mix. The short legend, repeated in the in-game help, is: `! = crippled; * = loaded`.

Loaded and empty transports are damaged separately, so `TT*4!1 TT5!2` says exactly which transports are crippled. Loading an army uses your healthy empty transports first and a crippled one only when no healthy one is left; unloading or landing puts the armies aboard crippled transports ashore first. A crippled transport keeps its recovery time either way.

In the Fleet List the Ships cell is narrow. When it cannot hold the whole composition, it ends in `+`. Review shows the full composition, including any crippled ships omitted from the table.

== Autopilot <autopilot>

Autopilot switches on after three missed turns. Your first real return that
year --- logging into `nc-game`, entering through `nc-door`, or submitting a
valid turn file --- turns that inactivity autopilot back off. Manual autopilot
stays on until you change it yourself.

Before giving orders, autopilot combines stationary Hold and Guard/Blockade
fleets in orbit of your own worlds, up to 3000 ships per fleet. A stationary
Guard a Starbase fleet can take those fleets in when its base is at rest; it is
never absorbed by this automatic merge. While autopilot runs, each year it
sends every fleet on Hold Position to the planet of yours that is best for its
distance and production. Warships already in orbit there take Guard/Blockade,
raising an ROE of 0 to 6. Any fleet of yours on Guard/Blockade that sits still
in orbit with a starbase of yours that has no escort escorts that starbase,
whether or not autopilot gave it the order. Fleets with any other order are
left alone. Under autopilot, a fleet without warships that is on Guard/Blockade
a World or Guard a Starbase stays on it; without autopilot, maintenance sends
such a fleet home.
