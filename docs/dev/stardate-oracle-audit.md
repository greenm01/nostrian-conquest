# ECMAINT Stardate Audit

`tools/ncmaint_stardate_audit.py` gives empire 1's first fleet one order on
the hazard-free move map, runs the same directory through classic `ECMAINT`
and `nc-cli maint-rust` year by year, and compares the fleet's sector, order
and reports: the week, the layer (Sector or System) and the kind. Classic
saves only at year end, so the report stardates are the only view of its
weeks. Run it on a machine with nix:

```
nix shell nixpkgs#dosbox-x -c python3 tools/ncmaint_stardate_audit.py --markdown /tmp/stardate-audit.md
```

Takeaways (2026-10-30, all 29 case-years match):

- Rust moves fleets week for week as classic does for every order probed:
  each report carries classic's week and layer.
- A View World fleet's Move report carries the week it climbed out of the
  system, not the week it viewed (fixed in 9436cdde).
- The probe must set the order's world flag as ECGAME does (mission aux 0 =
  `Order::targets_world`). With it clear, classic's world orders stop in the
  sector and report from there.
- Not covered: colonizing (the ETAC cases never land within the years run),
  combat, and the NC routing extensions in `ec-movement-spec.md`.

Results:

| case | year | match | classic | rust |
|---|---|---|---|---|
| move | 1 | yes | (15, 12) move; — | (15, 12) move; — |
| move | 2 | yes | (12, 9) hold; 52:Sector:Move | (12, 9) hold; 52:Sector:Move |
| move | 3 | yes | (12, 9) hold; — | (12, 9) hold; — |
| move-scout | 1 | yes | (12, 13) move; — | (12, 13) move; — |
| move-scout | 2 | yes | (10, 13) hold; 16:Sector:Move | (10, 13) hold; 16:Sector:Move |
| seek-home | 1 | yes | (13, 10) seek_home; — | (13, 10) seek_home; — |
| seek-home | 2 | yes | (16, 13) seek_home; — | (16, 13) seek_home; — |
| seek-home | 3 | yes | (16, 13) hold; 29:System:Home | (16, 13) hold; 29:System:Home |
| patrol | 1 | yes | (15, 12) patrol; — | (15, 12) patrol; — |
| patrol | 2 | yes | (13, 10) patrol; 27:Sector:Patrol | (13, 10) patrol; 27:Sector:Patrol |
| patrol | 3 | yes | (13, 10) patrol; — | (13, 10) patrol; — |
| guard-blockade-home | 1 | yes | (14, 12) guard_blockade; — | (14, 12) guard_blockade; — |
| guard-blockade-home | 2 | yes | (16, 13) guard_blockade; — | (16, 13) guard_blockade; — |
| guard-blockade-home | 3 | yes | (16, 13) guard_blockade; 3:System:Guard/Blockade World | (16, 13) guard_blockade; 3:System:Guard/Blockade World |
| view-world | 1 | yes | (14, 13) view; — | (14, 13) view; — |
| view-world | 2 | yes | (12, 12) view; — | (12, 12) view; — |
| view-world-scout | 1 | yes | (12, 11) view; — | (12, 11) view; — |
| view-world-scout | 2 | yes | (12, 11) hold; 16:System:Viewing 25:Sector:Move | (12, 11) hold; 16:System:Viewing 25:Sector:Move |
| scout-sector | 1 | yes | (13, 10) scout_sector; — | (13, 10) scout_sector; — |
| scout-sector | 2 | yes | (12, 9) scout_sector; 10:Sector:Scouting | (12, 9) scout_sector; 10:Sector:Scouting |
| scout-system | 1 | yes | (12, 11) scout_system; — | (12, 11) scout_system; — |
| scout-system | 2 | yes | (12, 11) scout_system; 16:System:Scouting | (12, 11) scout_system; 16:System:Scouting |
| colonize | 1 | yes | (16, 13) colonize; — | (16, 13) colonize; — |
| colonize | 2 | yes | (15, 13) colonize; — | (15, 13) colonize; — |
| colonize-far | 1 | yes | (16, 13) colonize; — | (16, 13) colonize; — |
| colonize-far | 2 | yes | (15, 13) colonize; — | (15, 13) colonize; — |
| colonize-far | 3 | yes | (14, 13) colonize; — | (14, 13) colonize; — |
| rendezvous | 1 | yes | (15, 12) rendezvous; — | (15, 12) rendezvous; — |
| rendezvous | 2 | yes | (13, 10) rendezvous; 27:Sector:Rendezvous | (13, 10) rendezvous; 27:Sector:Rendezvous |
