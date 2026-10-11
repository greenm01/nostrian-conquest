// Shared by the quick-start guide and full player manual.
#block[
  #set text(size: 9pt)
  #table(
    columns: (1.35fr, 0.8fr, 2fr),
    align: (left, left, left),
    inset: 5pt,
    table.header([Mission], [Operates in], [Notes]),
    [Move (1), Patrol (3), Scout Sector (10), Rendezvous (14)],
      [Deep space], [These orders do not take you down to the world.],
    [Seek Home (2), Colonize (12)],
      [System], [Finish on Hold inside the system. Seek Home chooses an owned world; Colonize requires an unowned world.],
    [Guard/Blockade World (5), Bombard (6), Invade (7), Blitz (8)],
      [System], [Continue to the world to guard or attack it.],
    [Scout System (11), Salvage (15)],
      [System], [Scout the world, or scrap ships at an owned world.],
    [Hold (0)],
      [Either], [Keeps the current position. Does not enter or leave a system.],
    [Guard Starbase (4)],
      [Either], [Joins the base in its layer and follows it. A base in deep space gives no recovery bonus.],
    [View (9)],
      [Both in sequence], [Enters the system to scan, then climbs back into deep space.],
    [Join Fleet (13)],
      [Special], [Approaches the host's sector as a system mission, then merges into the host and takes its position and orders.],
  )
]
