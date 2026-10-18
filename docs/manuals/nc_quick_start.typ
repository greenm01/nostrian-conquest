// Nostrian Conquest — Quick Start Guide
// Typst source — generates US Letter PDF

#set document(
  title: "Nostrian Conquest — Quick Start Guide",
  author: "Mason A. Green",
  date: datetime(year: 2026, month: 10, day: 18),
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
    Version 1.0.0-beta.3 • Revision date: October 18, 2026
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

Nostrian Conquest is turn-based, and you play it on your own schedule. Log in when you like, read your reports, and give your fleet orders before maintenance. You do not have to be online at the same time as the other players. The pace gives you time to plan and negotiate.

There is no End Turn button. Maintenance runs on your sysop’s schedule. It carries out all orders and moves the game to the next year. Reports use week/year dates: `14/3002` means week 14 of year 3002.

== What You Start With

A standard new empire starts with one homeworld at Production 100, a treasury of 35 points, 10 armies, and 4 ground batteries. It also has four fleets:

- *Fleets 1 and 2:* each has `CA1 ETAC1`, one cruiser and one reusable colony ship.
- *Fleets 3 and 4:* each has `DD1`, one destroyer for early exploring and defense.

ETACs claim unowned worlds with the *Colonize* mission (12). Troop transports (`TT`) carry armies for conquest. Check your reports. A campaign already in progress may give you a different start.

== First-Turn Checklist

+ *Keep a map beside you.* Print your player map or track known systems in a spreadsheet. Mark coordinates, owners, production potential, and fleet destinations. Update it from each turn’s reports. From the main menu, press `G`, then `M` to open the galaxy map. Ask your sysop for a printable map or a CSV export if he offers one.
+ *Read your reports.* From the main menu, press `G`, then `R` to open the Inbox. It has three folders: Reports, Messages, and Sent. Press `Q` to go back.
+ *Inspect your worlds.* From the main menu, press `P` to open Planet Command. Press `T` to set the tax rate. *One rate applies to every planet you own.* You cannot set a separate rate for each world. A rate of 50–65% is a good start. Lower taxes help growth. Higher taxes collect more now. Above 65%, production takes a penalty.
+ *Queue useful builds.* In Planet Command, press `B`, then `S` to enter build orders. Builds advance during maintenance and may take several turns. Finished ships wait in stardock. Commission them with Planet Command `C` or `M` before you use them.
+ *Explore and colonize.* From the main menu, press `F` to open Fleet Command. Press `O` to choose a fleet, a mission, and a target. Send a destroyer on *View* (9) to look at a world. Send a cruiser-and-ETAC fleet on *Colonize* (12) to claim an unowned world. Scout missions 10 and 11 need a Scout (`SC`).
+ *Check travel time, then log off.* Use Fleet Command `E` (ETA). For a world mission, add the time the fleet needs to enter the system. Check your targets and standing orders. Go back to the main menu and press `Q` to quit.

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

Speed is sectors per year. A fleet moves no faster than its slowest ship. You can order it to move slower. Leaving orbit costs one sector of movement. Entering orbit at the destination costs one more. Check the ETA. Do not assume a nearby world is one turn away.

== Read the Ship Notation

- `CA12` means twelve cruisers, none crippled.
- `CA12!3` means twelve cruisers in all, three crippled.
- `TT*4!1` means four loaded transports, one crippled. `TT2` means two empty transports.
- A trailing `+` in a narrow list means the fleet has more ships than fit on the line. Review shows the full fleet.

The legend is `! = crippled; * = loaded`.

*Detach and Transfer:* enter a ship code such as `CA`, then a quantity. `3` picks three ships, healthy ones first. `3!` picks exactly three crippled ships. The code and the quantity are two separate prompts. Do not type `CA3!` as one entry. Check the staged ships before you confirm.

== Recovery Is Automatic

Crippled ships fight at half strength but keep their speed. They recover on their own at the end of the second turn after they were damaged. You need no repair order, and you pay nothing.

A fleet that rests a full turn inside a system you own, beside your starbase, recovers one turn sooner. "Reaching a Starbase for Recovery" explains how to get there. Matching map coordinates alone is not enough.

== Grow Before You Fight

Colonizing can take several turns. Once the colony is set up, its ETAC can move on to another unowned world.

A new colony starts at Production 3, or at its Potential if that is lower. It starts growing and earning revenue at the next maintenance. Your homeworld and every colony pay the same tax rate. A lower rate trades tax revenue now for colony growth across your whole empire. The rate does not change any world's Potential Production.

A commissioned starbase at rest in orbit boosts growth when the tax rate is below 65%. It also lets the world spend up to five times its production from your saved treasury. Extra bases add defense, not more economic bonus. Armies cost 2 points and batteries cost 20. They reach the surface when their builds finish, not when you place the order.

#pagebreak()

= Sectors, Systems, and Starbases

A *sector* is a square on the map. A *world* is the planet in that square.
The *system* is the inner space around the world. *Deep space* is the outer
layer where fleets travel. A fleet and a starbase can share the same coordinates
but sit in different layers. Across layers, they cannot fight together, and the
base cannot help the fleet recover.

Fleets travel between sectors in deep space. A world mission takes the fleet
down into the system. The descent ends when the fleet reaches orbit. You choose
a mission, not a depth. *Hold keeps the fleet where it already is:* in deep
space, inside a system, or partway between.

== Where Missions Operate

This table shows where each mission does its work. It does not list every layer
the fleet crosses on the way. Deep space and the system can share the same map
coordinates.

#include "includes/mission_operating_space.typ"

== Reaching a Starbase for Recovery

Suppose your world and starbase are at `[21,21]`. *Move Fleet* brings a fleet
to deep space at `[21,21]`. If you switch it to Hold there, it stays in deep
space and gets no bonus. For a damaged fleet with warships, choose *Guard
Starbase* (4) and pick that base. Let the fleet reach the base inside the
system. Then keep both the fleet and the base at rest there for the whole next
turn. You can leave Guard Starbase in force. You do not need Hold. If you do
choose Hold, wait until the fleet has reached the base.

You must own the world and the base for the whole resting turn. A fleet that
arrives partway through the turn gets nothing. Only ships damaged in an earlier
turn get the bonus. Unarmed fleets cannot use Guard Starbase. *Seek Home* can
bring them inside a system you own, but it picks the nearest owned world, which
may not have a base.

#pagebreak()

= Combat and Your Next Login

== Choose Your Risks

*ROE* (Rules of Engagement) runs from 0 to 10. Set it with Fleet Command `C`. It sets how willing a fleet is to fight a hostile force. It does not decide whom you declare an enemy. ROE 0 never volunteers to attack. ROE 6 needs at least equal attack strength. ROE 10 fights at any odds. The game compares the attack strength of each side that takes part, not the ship counts. Damaged ships count at their reduced strength.

A fleet that is attacked must fight one exchange. After each exchange, its ROE decides whether it stays. Fleets without warships try to escape. A faster enemy can catch a fleet that retreats, so a low ROE does not promise safety. While the combat line holds, escorts shield the auxiliaries from ordinary fire. Critical hits can still reach past the line.

*Prepare before you assault.* Troop transports are built empty. At your own world, use Fleet Command `L` to load armies. Each transport carries one. Scout the target and bring escorts. Defenders in orbit can stop a planetary strike before it reaches the surface. Then choose the mission:

- *Bombard (6):* repeated attacks that can destroy batteries, docked ships, armies, goods, and production. The attack repeats each year until you change orders.
- *Invade (7):* needs warships and loaded transports. The fleet suppresses the batteries for the year, softens the garrison, then lands armies. If any batteries survive, no one lands. The fleet keeps its order and tries again next year. A world taken by invasion keeps only 20--40% of its production.
- *Blitz (8):* needs loaded transports. Escorting warships give brief cover fire. Then the surviving batteries fire on your landing transports. A blitz that lands in strength takes the world almost whole, batteries included.

Batteries are deadly. Plan on about three times their cost in warships to break a fortified world, and expect to lose ships doing it. A defender’s Scorch Earth order can destroy assets before and during capture. Defending armies fire first in each ground round. Equal numbers are a poor bet. Bring a margin. A fleet that keeps orbit after an assault ends on Hold, win or lose. A fleet beaten in space withdraws. A captured world earns no revenue for the next two maintenance turns.

== Find the Right Menu

Keys depend on the menu you are in. These paths start at the *main menu*. If you are in a submenu, press `Q` to back out first.

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

After maintenance, read your reports, commission new ships, and check which missions finished. The Sent folder marks a message that has not gone out yet as Queued. General Command `D` clears reports from view. It does not erase them from the game’s history.

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
  #v(3em)
  #block(width: 80%)[
    Questions, bug reports or ideas? Message the author on Nostr, or see the
    #link("https://play.nostrian-conquest.com/contact")[contact page] for
    Discord and Matrix.
    #v(0.5em)
    `npub1cecl2g9z776jvpudpqxajsrsgwuvq4vp7w26q74tx2438zy3q9vq8st7pp`
  ]
]
