// Nostrian Conquest — Quick Start Guide
// Typst source — generates US Letter PDF

#set document(
  title: "Nostrian Conquest — Quick Start Guide",
  author: "Mason A. Green",
  date: datetime(year: 2026, month: 10, day: 11),
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
    Version 1.0.0-beta.3 • Revision date: October 11, 2026
  ]
  #v(0.8em)
  #line(length: 100%, stroke: 0.5pt + luma(180))
]

#v(0.5em)

= Your First Year

Your goal is to dominate the galaxy. Expand from one world, build your economy, and defeat your rivals. The game ends when one empire becomes Emperor and no serious contender remains.

Nostrian Conquest is an asynchronous, turn-based strategy game. Log in when it suits you, review reports, and issue fleet orders before the next maintenance. Players need not be online together. The deliberate pace leaves time to plan, negotiate, and weigh your next move.

There is no End Turn button. Scheduled maintenance resolves everyone's orders and advances the year. Ask your sysop for the schedule. Builds and the economy run first; movement and combat then unfold over 52 internal weeks. `Stardate 14/3002` means week 14 of year 3002.

== What You Start With

A standard new empire has one homeworld at Production 100, a treasury of 35 points, 10 armies, and 4 ground batteries. It also has four fleets:

- *Fleets 1 and 2:* each has `CA1 ETAC1` — a cruiser and a reusable colony ship.
- *Fleets 3 and 4:* each has `DD1` — a destroyer for early exploration and defense.

An ETAC does not carry an army. It claims unowned worlds with *Colonize*, mission 12. Troop transports (`TT`) carry armies for conquest. Check your own reports and fleet list; an existing campaign may differ from this opening.

== First-Turn Checklist

+ *Read your reports.* From the main menu, press `G`, then `R` for the Inbox. It has Reports, Messages, and Sent folders. Return with `Q`.
+ *Inspect your worlds.* Main-menu `P` opens Planet Command. `T` sets *one tax rate for every planet you own*. You cannot set separate rates for individual worlds. Around 50–65% is a useful opening range: lower taxes favor growth; higher taxes collect more now. Above 65%, production suffers a penalty.
+ *Queue useful builds.* In Planet Command, `B` opens Build Commands; `S` specifies orders. A cruiser costs 15 points, a destroyer 5, and an ETAC 20. Spend for your plan; your homeworld already has defenses. Builds advance during maintenance and may take several turns to finish. Commission finished ships from stardock with Planet Command `C` or `M` before using them.
+ *Explore and colonize.* Main-menu `F` opens Fleet Command. Use `O`, choose the fleet, then select its mission and target. Send a destroyer on *View*, mission 9, to inspect a nearby world. Send a cruiser-and-ETAC fleet on *Colonize*, mission 12, to an unowned world. Do not use an invasion order to found a colony. Scout missions 10 and 11 require a Scout (`SC`), not a destroyer.
+ *Check travel time, then log off.* Use the fleet ETA command (`E`) and include system-entry time for a world mission. Check targets and standing orders. Return to the main menu and press `Q` when finished.

#admonition("REMEMBER")[
  Colonization can take more than one year. The ETAC survives, so send it to another unowned world after the colony is established. Review the results after every maintenance.
]

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

Keep damaged ships at rest in orbit of your world beside your starbase for the whole following turn to recover them one turn sooner. The base must stay there too. The bonus applies to ships damaged in an earlier turn, not fresh damage. Sharing the sector alone is not enough.

== Grow Before You Fight

New colonies start at Production 3, or their Potential if lower, and begin growing and earning revenue the following maintenance. The same tax rate applies to your homeworld and every colony. Lowering it favors colony growth over immediate tax revenue across your whole empire. It does not change any world's Potential Production.

A commissioned starbase at rest in orbit boosts growth at taxes below 65% and lets the world spend up to five times its production from saved treasury. More bases add defense, not more economic bonuses. Armies cost 2 points and batteries 20. They deploy to the surface when their builds complete, not when you place the order.

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
