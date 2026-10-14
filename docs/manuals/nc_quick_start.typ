// Nostrian Conquest — Quick Start Guide
// Typst source — generates US Letter PDF

#set document(
  title: "Nostrian Conquest — Quick Start Guide",
  author: "Mason A. Green",
  date: datetime(year: 2026, month: 10, day: 13),
)

#set page(
  paper: "us-letter",
  margin: (x: 0.9in, y: 0.9in),
  header: context {
    if counter(page).get().first() > 1 {
      text(font: "IBM Plex Sans", size: 8.5pt, fill: luma(100))[
        Nostrian Conquest #h(1fr) Quick Start Guide
      ]
    }
  },
  footer: context {
    text(font: "IBM Plex Sans", size: 8.5pt, fill: luma(100))[
      Player Quick Start
      #h(1fr)
      Page #counter(page).get().first() of #counter(page).final().first()
    ]
  },
)

#set text(
  font: "IBM Plex Serif",
  size: 10pt,
)

#show raw: set text(font: "IBM Plex Mono", size: 9pt)
#show heading: set text(font: "IBM Plex Sans")
#show table.cell: set par(justify: false)

#set par(
  justify: true,
  leading: 0.6em,
)

#set heading(numbering: "1.")

#show table.cell.where(y: 0): strong

// Admonition helper
#let admonition(kind, body) = {
  block(
    width: 100%,
    inset: 8pt,
    stroke: 0.5pt + luma(140),
    radius: 2pt,
    fill: luma(248),
  )[
    #text(font: "IBM Plex Sans", weight: "bold")[#kind:] #body
  ]
}

// ─── Header Banner ────────────────────────────────────────────────────────

#align(center)[
  #v(0.5em)
  #text(font: "IBM Plex Sans", size: 20pt, weight: "bold")[NOSTRIAN CONQUEST] \
  #v(0.2em)
  #text(font: "IBM Plex Sans", size: 13pt, style: "italic", fill: luma(60))[Quick Start Player's Guide] \
  #v(0.2em)
  #text(font: "IBM Plex Sans", size: 9pt, fill: luma(100))[
    Version 1.0.0-beta.3 • Revision date: October 13, 2026
  ]
  #v(0.8em)
  #line(length: 100%, stroke: 0.5pt + luma(180))
]

#v(0.5em)

#block[
  #set text(style: "italic", size: 9.5pt)
  Beyond the mapped frontiers of the old Nostrian dominion lies a small galaxy of
  contested solar systems. The old masters are gone. Their stations are silent,
  their patrols vanished, and their subjects left with fleets, industry, and
  enough knowledge to build empires.
]

= Your First Year

Your goal is to dominate the galaxy. The game ends when one empire becomes Emperor and no serious contender remains.

Nostrian Conquest is asynchronous and turn-based. Log in on your own schedule, review reports, and issue fleet orders before maintenance. Players need not be online together. The pace leaves time to plan and negotiate.

There is no End Turn button. Maintenance resolves orders and advances the year on your sysop’s schedule. Reports use week/year dates: `14/3002` means week 14 of year 3002.

== What You Start With

A standard new empire has one homeworld at Production 100, a treasury of 35 points, 10 armies, and 4 ground batteries. It also has four fleets:

- *Fleets 1 and 2:* each has `CA1 ETAC1` — a cruiser and a reusable colony ship.
- *Fleets 3 and 4:* each has `DD1` — a destroyer for early exploration and defense.

ETACs claim unowned worlds with *Colonize*, mission 12. Troop transports (`TT`) carry armies for conquest. Check your reports; an existing campaign may differ from this opening.

== First-Turn Checklist

+ *Keep a map beside you.* Print your player map or track known systems in a spreadsheet. Mark coordinates, owners, production potential, and fleet destinations. Update it from each turn’s reports. Main-menu `G`, then `M` opens the galaxy map; ask your sysop for a printable map or CSV export if available.
+ *Read your reports.* From the main menu, press `G`, then `R` for the Inbox. It has Reports, Messages, and Sent folders. Return with `Q`.
+ *Inspect your worlds.* Main-menu `P` opens Planet Command. `T` sets *one tax rate for every planet you own*. You cannot set separate rates for individual worlds. Around 50–65% is a useful opening range: lower taxes favor growth; higher taxes collect more now. Above 65%, production suffers a penalty.
+ *Queue useful builds.* In Planet Command, press `B`, then `S` to specify orders. Builds advance during maintenance and may take several turns. Commission finished ships from stardock with Planet Command `C` or `M` before using them.
+ *Explore and colonize.* Main-menu `F` opens Fleet Command. Use `O` to choose a fleet, mission, and target. Send a destroyer on *View* (9) to inspect a world, and a cruiser-and-ETAC fleet on *Colonize* (12) to claim an unowned world. Scout missions 10 and 11 require a Scout (`SC`).
+ *Check travel time, then log off.* Use the fleet ETA command (`E`) and include system-entry time for a world mission. Check targets and standing orders. Return to the main menu and press `Q` when finished.

#pagebreak()

= Ships, Travel, and Growth

== Know Your Ships

#table(
  columns: (auto, auto, auto, 1fr),
  align: (left, right, right, left),
  inset: 5pt,
  table.header([Code], [Cost], [Max. speed], [Role]),
  [`DD`], [5], [6], [Destroyer: fast combat ship and raider.],
  [`CA`], [15], [5], [Cruiser: balanced fighter and escort.],
  [`BB`], [45], [4], [Battleship: heavy firepower and durability.],
  [`SC`], [15], [6], [Scout: unarmed reconnaissance ship.],
  [`TT`], [5], [5], [Troop transport: carries one army; built empty.],
  [`ETAC`], [20], [3], [Reusable colony ship; carries no armies.],
  [`SB`], [50], [1], [Starbase: separate unit, with orbital defense and economic benefits.],
)

Speed is sectors per year. A fleet's maximum speed is set by its slowest ship; you can order it to travel more slowly. Leaving orbit costs one sector of movement, and entering a destination's orbit costs another. Check ETA rather than assuming a nearby world is one turn away.

== Read the Ship Notation

- `CA12` means twelve cruisers, none crippled.
- `CA12!3` means twelve cruisers in all, three crippled.
- `TT*4!1` means four loaded transports, one crippled. `TT2` means two empty transports.
- A trailing `+` in a narrow list means more ships; Review shows the full fleet.

The legend is `! = crippled; * = loaded`.

*Detach and Transfer:* enter a ship code such as `CA`, then a quantity. `3` selects three ships, healthy first. `3!` selects exactly three crippled ships. These are two prompts: do not enter `CA3!` as one command. Check the staged ships before confirming.

== Recovery Is Automatic

Crippled ships fight at half strength but keep their speed. They recover automatically at the end of the second turn after they were damaged. No repair order or payment is needed.

A full turn resting inside your owned system beside your starbase can shorten recovery by one turn. The next section explains how to get there; matching map coordinates alone is not enough.

== Grow Before You Fight

Colonization can take several turns. Once a colony is established, its ETAC can move on to another unowned world.

New colonies start at Production 3, or their Potential if lower, and begin growing and earning revenue the following maintenance. The same tax rate applies to your homeworld and every colony. Lowering it favors colony growth over immediate tax revenue across your whole empire. It does not change any world's Potential Production.

A commissioned starbase at rest in orbit boosts growth at taxes below 65% and lets the world spend up to five times its production from saved treasury. More bases add defense, not more economic bonuses. Armies cost 2 points and batteries 20. They deploy to the surface when their builds complete, not when you place the order.

#pagebreak()

= Sectors, Systems, and Starbases

A *sector* is a square on the map. A *world* is the planet in that square.
The *system* is the inner space around the world; *deep space* is the outer
travel layer. A fleet and a starbase can share coordinates but occupy different
layers. They cannot fight together or share recovery assistance across layers.

Travel between sectors takes place in deep space. A world mission takes the
fleet down into the system. Reaching orbit is the end of that descent. You
choose a mission, not a depth number. *Hold keeps the position the fleet already
has* — in deep space, inside a system, or partway between.

== Where Missions Operate

This table shows where a mission does its work, not every layer crossed on the
way. Deep space and the system can have the same map coordinates.

#include "includes/mission_operating_space.typ"

== Reaching a Starbase for Recovery

Suppose your world and starbase are at `[21,21]`. *Move to Sector* gets a fleet
to deep space at `[21,21]`; switching to Hold leaves it there, without the bonus.
For a damaged fleet with warships, choose *Guard Starbase* (4) and select that
base. Let the fleet reach it inside the system. Then keep both fleet and base
at rest there for the whole following turn. Guard Starbase can stay in force;
Hold is not required. If you choose Hold, wait until the fleet has reached the base.

You must own the world and the base throughout that resting turn. Arriving
partway through the turn is not enough. Only ships damaged in an earlier turn
receive the bonus. Unarmed fleets cannot use Guard Starbase; *Seek Home* can
bring them inside an owned system, but chooses the nearest owned world, which
may not have a base.

#pagebreak()

= Combat and Your Next Login

== Choose Your Risks

*ROE* (Rules of Engagement) runs from 0 to 10. Set it with Fleet Command `C`. It controls willingness to fight a hostile force, not whom you declare an enemy. ROE 0 never volunteers to attack; ROE 6 requires at least equal attack strength; ROE 10 accepts any odds. The comparison uses participating sides' attack strength, including damaged ships at their reduced strength, rather than ship counts.

A fleet that is attacked must fight one exchange. After each exchange, its ROE decides whether it stays. Fleets without warships try to escape. Faster enemies can catch a retreat, so low ROE is not a promise of safety. Escorts protect auxiliaries from ordinary fire while the combat line holds, but critical hits can reach past it.

*Prepare before assaulting.* Troop transports are built empty. Use Fleet Command `L` at your world to load armies; each transport carries one. Scout the target and bring escorts. Defending forces in orbit can stop a planetary strike before it reaches the surface. Then choose the mission:

- *Bombard (6):* repeated attacks that can destroy batteries, docked ships, armies, goods, and production. It continues each year until you change orders.
- *Invade (7):* requires warships and loaded transports. Suppress batteries, soften the garrison, then land armies. Surviving batteries block the landing.
- *Blitz (8):* requires loaded transports. Escorting warships provide brief cover fire, then surviving batteries fire on your landing transports. Surviving batteries become yours if you capture the world.

Invade and Blitz do not damage industry or goods, but a defender’s Scorch Earth order can destroy assets before and during capture. Defending armies fire first in each ground round. Equal numbers are a poor gamble; bring a margin. Fleets that keep orbit after an assault finish on Hold, win or lose; those beaten in space withdraw. A captured world earns no revenue for the next two maintenance turns.

== Find the Right Menu

Keys depend on the menu. These paths start at the *main menu*; press `Q` to back out of a submenu first.

#table(
  columns: (1.15in, 1fr),
  align: (left, left),
  inset: 4pt,
  table.header([Keys], [Action]),
  [`P`, then `T`], [Set the empire-wide tax rate.],
  [`P`, then `B`], [Build ships and defenses.],
  [`F`, then `O`], [Assign a fleet mission.],
  [`F`, then `C`], [Change fleet ROE, ID, or speed.],
  [`G`, then `R`], [Review the Inbox: Reports, Messages, Sent.],
  [`G`, then `C`], [Send a message to another empire.],
  [`G`, then `M`], [Open the galaxy map.],
  [`G`, then `E`], [Declare or list enemies.],
  [`V`], [View a partial map.],
  [`B` or `D`], [Brief or detailed empire report.],
  [`H` or `?`], [Help for the current screen.],
  [`Q`], [Leave a submenu; from the main menu, quit.],
)

After maintenance, read reports, commission new ships, and check completed missions. Sent marks pending messages as Queued. General Command `D` clears reports from view, not from the game’s history.

For the full rules and tables, read *nc_player_manual.pdf*, shipped beside this guide.

#pagebreak()

#let manual_license_notice = [
  New text, layout, and explanatory material in this manual
  © 2026 Mason A. Green and are licensed under CC BY-NC-SA 4.0. Original
  historical Esterian Conquest references and other preserved 1992 source
  material are excluded from that grant and remain credited to their original
  authors and rights holders.
]

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
