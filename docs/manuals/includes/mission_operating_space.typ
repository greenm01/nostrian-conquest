// Shared by the quick-start guide and full player manual.
#block[
  #set text(size: 9pt)
  #table(
    columns: (1.35fr, 0.8fr, 2fr),
    align: (left, left, left),
    inset: 5pt,
    table.header([Mission], [Operates in], [Notes]),
    [Move Fleet (1), Patrol a Sector (3), Scout a Sector (10), Rendezvous at Sector (14)],
      [Deep space], [These orders do not take you down to the world.],
    [Seek Home (2), Colonize a World (12)],
      [System], [Finish on Hold inside the system. Seek Home chooses an owned world; Colonize requires an unowned world.],
    [Guard/Blockade a World (5), Bombard a World (6), Invade a World (7), Blitz a World (8)],
      [System], [Continue to the world to guard or attack it.],
    [Scout a Solar System (11), Salvage (15)],
      [System], [Scout the world, or scrap ships at an owned world.],
    [Hold Position (0)],
      [Either], [Keeps the current position. Does not enter or leave a system.],
    [Guard a Starbase (4)],
      [Either], [Joins the base in its layer and follows it. A base in deep space gives no recovery bonus.],
    [View a World (9)],
      [Both in sequence], [Enters the system to scan, then climbs back into deep space.],
    [Join Another Fleet (13)],
      [Special], [Approaches the host's sector as a system mission, then merges into the host and takes its position and orders.],
  )
]
