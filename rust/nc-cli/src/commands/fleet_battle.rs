use std::fs;
use std::path::Path;

use nc_data::{Coords, CoreGameData, EmpireId, Order, Planet, Real48};

use crate::commands::runtime::{
    export_runtime_snapshot_in_place, with_runtime_game_mut_and_export,
};
use crate::support::paths::display_repo_path;
use crate::workspace::copy_init_files;

/// Apply the fleet-battle scenario to an already-initialized game directory.
///
/// Fixture-specific constants for this scenario:
/// - Fleet 1 (empire=1, slot=1, index=0): moved to (10,10) with RoE=100, BB=50, CA=50, DD=50
/// - Fleet 3 (empire=1, slot=3, index=2): BombardWorld (0x06) order, speed=3/3, target (15,13),
///   CA=50, DD=50 (in addition to existing CA=1 for a total of 50 cruisers)
/// - Fleet 5 (empire=2, slot=1, index=4): Patrol (0x03) order at (10,10), speed=0/6,
///   SC=0, BB=100, CA=0
/// - Fleet 9 (empire=3, slot=1, index=8): MoveOnly (0x01) to (10,10), speed=3/6,
///   loc=(9,10), SC=0, BB=1, CA=0
///
/// The preserved pre-fixture holds 10 in byte 0x24 of fleets 5 and 9, written
/// by an earlier tool as a scout count. Classic reads it as the high byte of
/// the mission id (0x0a00), which Patrol and Move ignore, so the fleets carry
/// no scouts. The scenario writes that mission id to recreate the fixture.
/// - Planet 14 (index=13): set via direct raw byte assignment (Dust Bowl-type seeded world
///   at (15,13), owned by empire 2, armies=142, batteries=15)
///
/// All record indices and constants here are scenario-specific; the general mutators live in
/// nc-data and accept parameters.
pub(crate) fn apply_fleet_battle_scenario(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    set_fleet_battle(
        dir, 0x0a, 0x0a, // battle_coords (10,10)
        100, 50, 50, 50, // f0_roe, f0_bb, f0_ca, f0_dd
        50, 50, // f2_ca, f2_dd
        0, 100, 0, // f4_sc, f4_bb, f4_ca
        9, 0x0a, 0, 1, 0, // f8_loc_x, f8_loc_y, f8_sc, f8_bb, f8_ca
        0x0f, 0x0d, 142, 15, // p14_x, p14_y, p14_armies, p14_batteries
    )?;
    println!("Applied scenario: fleet-battle");
    Ok(())
}

/// Set up a fleet-battle scenario with parameterized coordinates and ship counts.
///
/// Parameters:
/// - `battle_x`, `battle_y`: coordinates where fleets gather for battle
/// - `f0_roe`, `f0_bb`, `f0_ca`, `f0_dd`: Fleet 0 (empire 1, slot 1) RoE and ship counts
/// - `f2_ca`, `f2_dd`: Fleet 2 (empire 1, slot 3) ship counts for bombard
/// - `f4_sc`, `f4_bb`, `f4_ca`: Fleet 4 (empire 2, slot 1) ship counts
/// - `f8_loc_x`, `f8_loc_y`: Fleet 8 (empire 3, slot 1) location (usually near battle)
/// - `f8_sc`, `f8_bb`, `f8_ca`: Fleet 8 ship counts
/// - `p14_x`, `p14_y`: Planet 14 coordinates
/// - `p14_armies`, `p14_batteries`: Planet 14 defenses
pub(crate) fn set_fleet_battle(
    dir: &Path,
    battle_x: u8,
    battle_y: u8,
    f0_roe: u8,
    f0_bb: u16,
    f0_ca: u16,
    f0_dd: u16,
    f2_ca: u16,
    f2_dd: u16,
    f4_sc: u8,
    f4_bb: u16,
    f4_ca: u16,
    f8_loc_x: u8,
    f8_loc_y: u8,
    f8_sc: u8,
    f8_bb: u16,
    f8_ca: u16,
    p14_x: u8,
    p14_y: u8,
    p14_armies: u8,
    p14_batteries: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    with_runtime_game_mut_and_export(dir, |data| {
        {
            let f = &mut data.fleets[0];
            f.sector = Coords::new(battle_x, battle_y);
            f.target = Coords::new(battle_x, battle_y);
            f.rules_of_engagement = f0_roe;
            f.ships.battleships = f0_bb;
            f.ships.cruisers = f0_ca;
            f.ships.destroyers = f0_dd;
        }
        {
            let f = &mut data.fleets[2];
            f.max_speed = 3;
            f.speed = 3;
            f.order = Order::BombardWorld;
            f.target = Coords::new(p14_x, p14_y);
            f.ships.cruisers = f2_ca;
            f.ships.destroyers = f2_dd;
        }
        {
            let f = &mut data.fleets[4];
            f.max_speed = 6;
            f.sector = Coords::new(battle_x, battle_y);
            f.order = Order::PatrolSector;
            f.target = Coords::new(battle_x, battle_y);
            f.ships.scouts = u16::from(f4_sc);
            f.mission_target = 0x0a00;
            f.ships.battleships = f4_bb;
            f.ships.cruisers = f4_ca;
        }
        {
            let f = &mut data.fleets[8];
            f.max_speed = 6;
            f.speed = 3;
            f.sector = Coords::new(f8_loc_x, f8_loc_y);
            f.order = Order::MoveOnly;
            f.target = Coords::new(battle_x, battle_y);
            f.ships.scouts = u16::from(f8_sc);
            f.mission_target = 0x0a00;
            f.ships.battleships = f8_bb;
            f.ships.cruisers = f8_ca;
        }
        {
            let p = data.planets.get_mut(13).ok_or("planet record 14 missing")?;
            *p = Planet {
                sector: Coords::new(p14_x, p14_y),
                potential_production: 100,
                production: Real48::from_int(100),
                stored_goods: Real48::from_int(66),
                name: "TargetPrime".to_string(),
                armies: u16::from(p14_armies),
                ground_batteries: u16::from(p14_batteries),
                years_under_owner: 0x02,
                owner: EmpireId(0x02),
                ..Planet::default()
            };
        }
        Ok(())
    })?;

    println!(
        "  FLEET[1]: loc=({}, {}) tgt=({}, {}) RoE={} BB={} CA={} DD={}",
        battle_x, battle_y, battle_x, battle_y, f0_roe, f0_bb, f0_ca, f0_dd
    );
    println!(
        "  FLEET[3]: order=BombardWorld tgt=({}, {}) speed=3/3 CA={} DD={}",
        p14_x, p14_y, f2_ca, f2_dd
    );
    println!(
        "  FLEET[5]: order=PatrolSector loc=({}, {}) speed=0/6 SC={} BB={} CA={}",
        battle_x, battle_y, f4_sc, f4_bb, f4_ca
    );
    println!(
        "  FLEET[9]: order=MoveOnly loc=({}, {}) tgt=({}, {}) speed=3/6 SC={} BB={} CA={}",
        f8_loc_x, f8_loc_y, battle_x, battle_y, f8_sc, f8_bb, f8_ca
    );
    println!(
        "  PLANET[14]: ({}, {}) empire=2 armies={} batteries={}",
        p14_x, p14_y, p14_armies, p14_batteries
    );
    Ok(())
}

/// Initialize a fleet-battle scenario directory from a source baseline.
pub(crate) fn init_fleet_battle(
    source: &Path,
    target: &Path,
    battle_x: u8,
    battle_y: u8,
    f0_roe: u8,
    f0_bb: u16,
    f0_ca: u16,
    f0_dd: u16,
    f2_ca: u16,
    f2_dd: u16,
    f4_sc: u8,
    f4_bb: u16,
    f4_ca: u16,
    f8_loc_x: u8,
    f8_loc_y: u8,
    f8_sc: u8,
    f8_bb: u16,
    f8_ca: u16,
    p14_x: u8,
    p14_y: u8,
    p14_armies: u8,
    p14_batteries: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    copy_init_files(source, target)?;
    set_fleet_battle(
        target,
        battle_x,
        battle_y,
        f0_roe,
        f0_bb,
        f0_ca,
        f0_dd,
        f2_ca,
        f2_dd,
        f4_sc,
        f4_bb,
        f4_ca,
        f8_loc_x,
        f8_loc_y,
        f8_sc,
        f8_bb,
        f8_ca,
        p14_x,
        p14_y,
        p14_armies,
        p14_batteries,
    )?;
    export_runtime_snapshot_in_place(target)?;
    println!("Fleet-battle directory initialized at {}", target.display());
    Ok(())
}

/// Initialize a batch of fleet-battle scenario directories.
///
/// Spec format: battle_x:battle_y:f0_roe:f0_bb:f0_ca:f0_dd:f2_ca:f2_dd:f4_sc:f4_bb:f4_ca:f8_loc_x:f8_loc_y:f8_sc:f8_bb:f8_ca:p14_x:p14_y:p14_armies:p14_batteries
pub(crate) fn init_fleet_battle_batch(
    source: &Path,
    target_root: &Path,
    specs: &[(
        u8,
        u8,
        u8,
        u16,
        u16,
        u16,
        u16,
        u16,
        u8,
        u16,
        u16,
        u8,
        u8,
        u8,
        u16,
        u16,
        u8,
        u8,
        u8,
        u8,
    )],
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(target_root)?;
    let mut manifest = String::new();
    manifest.push_str("Fleet-battle batch\n");
    manifest.push_str(&format!("source={}\n", display_repo_path(source)));
    manifest.push_str(&format!("target_root={}\n", target_root.display()));
    manifest.push('\n');

    for &(
        bx,
        by,
        f0r,
        f0bb,
        f0ca,
        f0dd,
        f2ca,
        f2dd,
        f4sc,
        f4bb,
        f4ca,
        f8lx,
        f8ly,
        f8sc,
        f8bb,
        f8ca,
        p14x,
        p14y,
        p14a,
        p14b,
    ) in specs
    {
        let name = format!(
            "bx{:02}-by{:02}-f0r{}-f0bb{}-f0ca{}-f0dd{}-f2ca{}-f2dd{}-f4sc{}-f4bb{}-f4ca{}-f8lx{:02}-f8ly{:02}-f8sc{}-f8bb{}-f8ca{}-p14x{:02}-p14y{:02}-p14a{}-p14b{}",
            bx,
            by,
            f0r,
            f0bb,
            f0ca,
            f0dd,
            f2ca,
            f2dd,
            f4sc,
            f4bb,
            f4ca,
            f8lx,
            f8ly,
            f8sc,
            f8bb,
            f8ca,
            p14x,
            p14y,
            p14a,
            p14b
        );
        let scenario_dir = target_root.join(&name);
        init_fleet_battle(
            source,
            &scenario_dir,
            bx,
            by,
            f0r,
            f0bb,
            f0ca,
            f0dd,
            f2ca,
            f2dd,
            f4sc,
            f4bb,
            f4ca,
            f8lx,
            f8ly,
            f8sc,
            f8bb,
            f8ca,
            p14x,
            p14y,
            p14a,
            p14b,
        )?;
        manifest.push_str(&format!("{name}\n"));
        manifest.push_str(&format!("  battle=[{}, {}]\n", bx, by));
        manifest.push_str(&format!(
            "  F0: RoE={} BB={} CA={} DD={}\n",
            f0r, f0bb, f0ca, f0dd
        ));
        manifest.push_str(&format!(
            "  F2: CA={} DD={} (bombards planet)\n",
            f2ca, f2dd
        ));
        manifest.push_str(&format!("  F4: SC={} BB={} CA={}\n", f4sc, f4bb, f4ca));
        manifest.push_str(&format!(
            "  F8: loc=[{}, {}] SC={} BB={} CA={}\n",
            f8lx, f8ly, f8sc, f8bb, f8ca
        ));
        manifest.push_str(&format!(
            "  Planet14: [{}, {}] armies={} batteries={}\n",
            p14x, p14y, p14a, p14b
        ));
        manifest.push_str(&format!("  dir={}\n", scenario_dir.display()));
        manifest.push_str(&format!(
            "  validate=nc-cli compliance-report {}\n\n",
            scenario_dir.display()
        ));
    }

    fs::write(target_root.join("FLEET_BATTLE_BATCH.txt"), manifest)?;
    println!(
        "Initialized {} fleet-battle directories under {}",
        specs.len(),
        target_root.display()
    );
    Ok(())
}

pub(crate) fn validate_fleet_battle_data(
    data: &CoreGameData,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut errors = Vec::new();

    // Fleet 0 (empire=1, slot=1): at (10,10), RoE=100, BB=50, CA=50, DD=50
    match data.fleets.first() {
        None => errors.push("missing fleet record 1".to_string()),
        Some(f) => {
            if f.sector.to_array() != [0x0a, 0x0a] {
                errors.push(format!(
                    "FLEET[1].location expected (10,10), got {:?}",
                    f.sector.to_array()
                ));
            }
            if f.target.to_array() != [0x0a, 0x0a] {
                errors.push(format!(
                    "FLEET[1].target expected (10,10), got {:?}",
                    f.target.to_array()
                ));
            }
            if f.rules_of_engagement != 0x64 {
                errors.push(format!(
                    "FLEET[1].roe expected 100 (0x64), got {}",
                    f.rules_of_engagement
                ));
            }
            if f.ships.battleships != 50 {
                errors.push(format!(
                    "FLEET[1].bb expected 50, got {}",
                    f.ships.battleships
                ));
            }
            if f.ships.cruisers != 50 {
                errors.push(format!("FLEET[1].ca expected 50, got {}", f.ships.cruisers));
            }
            if f.ships.destroyers != 50 {
                errors.push(format!(
                    "FLEET[1].dd expected 50, got {}",
                    f.ships.destroyers
                ));
            }
        }
    }

    // Fleet 2 (empire=1, slot=3): BombardWorld, speed=3/3, target (15,13), CA=50, DD=50
    match data.fleets.get(2) {
        None => errors.push("missing fleet record 3".to_string()),
        Some(f) => {
            if f.max_speed != 3 {
                errors.push(format!(
                    "FLEET[3].max_speed expected 3, got {}",
                    f.max_speed
                ));
            }
            if f.speed != 3 {
                errors.push(format!(
                    "FLEET[3].current_speed expected 3, got {}",
                    f.speed
                ));
            }
            if f.order.code() != 0x06 {
                errors.push(format!(
                    "FLEET[3].order expected 0x06 (BombardWorld), got {:#04x}",
                    f.order.code()
                ));
            }
            if f.target.to_array() != [0x0f, 0x0d] {
                errors.push(format!(
                    "FLEET[3].target expected (15,13), got {:?}",
                    f.target.to_array()
                ));
            }
            if f.ships.cruisers != 50 {
                errors.push(format!("FLEET[3].ca expected 50, got {}", f.ships.cruisers));
            }
            if f.ships.destroyers != 50 {
                errors.push(format!(
                    "FLEET[3].dd expected 50, got {}",
                    f.ships.destroyers
                ));
            }
        }
    }

    // Fleet 4 (empire=2, slot=1): Patrol at (10,10), speed=0/6, SC=0, BB=100, CA=0
    match data.fleets.get(4) {
        None => errors.push("missing fleet record 5".to_string()),
        Some(f) => {
            if f.max_speed != 6 {
                errors.push(format!(
                    "FLEET[5].max_speed expected 6, got {}",
                    f.max_speed
                ));
            }
            if f.sector.to_array() != [0x0a, 0x0a] {
                errors.push(format!(
                    "FLEET[5].location expected (10,10), got {:?}",
                    f.sector.to_array()
                ));
            }
            if f.order.code() != 0x03 {
                errors.push(format!(
                    "FLEET[5].order expected 0x03 (PatrolSector), got {:#04x}",
                    f.order.code()
                ));
            }
            if f.target.to_array() != [0x0a, 0x0a] {
                errors.push(format!(
                    "FLEET[5].target expected (10,10), got {:?}",
                    f.target.to_array()
                ));
            }
            if f.ships.scouts != 0 {
                errors.push(format!("FLEET[5].sc expected 0, got {}", f.ships.scouts));
            }
            if f.ships.battleships != 100 {
                errors.push(format!(
                    "FLEET[5].bb expected 100, got {}",
                    f.ships.battleships
                ));
            }
            if f.ships.cruisers != 0 {
                errors.push(format!("FLEET[5].ca expected 0, got {}", f.ships.cruisers));
            }
        }
    }

    // Fleet 8 (empire=3, slot=1): MoveOnly to (10,10), speed=3/6, loc=(9,10), SC=0, BB=1, CA=0
    match data.fleets.get(8) {
        None => errors.push("missing fleet record 9".to_string()),
        Some(f) => {
            if f.max_speed != 6 {
                errors.push(format!(
                    "FLEET[9].max_speed expected 6, got {}",
                    f.max_speed
                ));
            }
            if f.speed != 3 {
                errors.push(format!(
                    "FLEET[9].current_speed expected 3, got {}",
                    f.speed
                ));
            }
            if f.sector.to_array() != [0x09, 0x0a] {
                errors.push(format!(
                    "FLEET[9].location expected (9,10), got {:?}",
                    f.sector.to_array()
                ));
            }
            if f.order.code() != 0x01 {
                errors.push(format!(
                    "FLEET[9].order expected 0x01 (MoveOnly), got {:#04x}",
                    f.order.code()
                ));
            }
            if f.target.to_array() != [0x0a, 0x0a] {
                errors.push(format!(
                    "FLEET[9].target expected (10,10), got {:?}",
                    f.target.to_array()
                ));
            }
            if f.ships.scouts != 0 {
                errors.push(format!("FLEET[9].sc expected 0, got {}", f.ships.scouts));
            }
            if f.ships.battleships != 1 {
                errors.push(format!(
                    "FLEET[9].bb expected 1, got {}",
                    f.ships.battleships
                ));
            }
            if f.ships.cruisers != 0 {
                errors.push(format!("FLEET[9].ca expected 0, got {}", f.ships.cruisers));
            }
        }
    }

    // Planet 14 (index 13): Dust Bowl-type world at (15,13), owned empire=2, armies=142, batteries=15
    match data.planets.get(13) {
        None => errors.push("missing planet record 14".to_string()),
        Some(p) => {
            if p.sector.to_array() != [0x0f, 0x0d] {
                errors.push(format!(
                    "PLANET[14].coords expected (15,13), got {:?}",
                    p.sector.to_array()
                ));
            }
            if p.armies != 142 {
                errors.push(format!(
                    "PLANET[14].armies expected 142, got {}",
                    u8::try_from(p.armies).unwrap_or(u8::MAX)
                ));
            }
            if p.ground_batteries != 15 {
                errors.push(format!(
                    "PLANET[14].batteries expected 15, got {}",
                    u8::try_from(p.ground_batteries).unwrap_or(u8::MAX)
                ));
            }
            if p.years_under_owner != 2 {
                errors.push(format!(
                    "PLANET[14].ownership_status expected 2, got {}",
                    p.years_under_owner
                ));
            }
            if p.owner.0 != 2 {
                errors.push(format!(
                    "PLANET[14].owner_empire expected 2, got {}",
                    p.owner.0
                ));
            }
        }
    }

    if errors.is_empty() {
        println!("Valid fleet-battle scenario");
        println!("  FLEET[1]: loc=(10,10) tgt=(10,10) RoE=100 BB=50 CA=50 DD=50");
        println!("  FLEET[3]: order=0x06 (BombardWorld) tgt=(15,13) speed=3/3 CA=50 DD=50");
        println!("  FLEET[5]: order=0x03 (PatrolSector) loc=(10,10) speed=0/6 SC=0 BB=100");
        println!("  FLEET[9]: order=0x01 (MoveOnly) loc=(9,10) tgt=(10,10) speed=3/6 SC=0 BB=1");
        println!("  PLANET[14]: (15,13) empire=2 armies=142 batteries=15");
        Ok(())
    } else {
        Err(errors.join("\n").into())
    }
}
