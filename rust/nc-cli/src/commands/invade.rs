use std::fs;
use std::path::Path;

use nc_data::ShipCategory::{Battleships, Cruisers, Destroyers, EmptyTransports, Scouts};
use nc_data::{
    Composition, Coords, CoreGameData, EmpireId, Order, Planet, Real48, set_troop_transports,
    troop_transports,
};

use crate::commands::runtime::{
    export_runtime_snapshot_in_place, with_runtime_game_mut_and_export,
};
use crate::support::paths::display_repo_path;
use crate::workspace::copy_init_files;

/// Apply the invade scenario to an already-initialized game directory.
///
/// Fixture-specific constants for this scenario:
/// - Fleet 3 (empire=1, slot=3, index=2): InvadeWorld order, speed=3/3,
///   target (15,13), byte 0x08 = 100 (an old tool's "invasion armies", which
///   classic reads as the previous-fleet word's high byte), SC=0, BB=100,
///   CA=50, DD=50, TT=50
///   (the preserved pre-fixture holds 100 in byte 0x24, which classic reads as
///   the mission id's high byte, not scouts)
/// - Planet 14 (index=13): set via set_as_owned_target_world (Dust Bowl-type seeded world
///   at (15,13), owned by empire 2, armies=142, batteries=15)
///
/// All record indices and constants here are scenario-specific; the general mutators live in
/// nc-data and accept parameters.
pub(crate) fn apply_invade_scenario(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    set_invade_onefleet(dir, 0x0f, 0x0d, 0, 100, 50, 50, 50, 100)?;
    println!("Applied scenario: invade");
    Ok(())
}

/// Set up a single invade fleet order plus planet seed for parameterized use.
///
/// Parameters:
/// - `target_x`, `target_y`: coordinates for fleet order and planet-14
/// - `sc`, `bb`, `ca`, `dd`, `tt`: ship counts for fleet 3
/// - `invasion_armies`: armies loaded on fleet 3 for invasion
pub(crate) fn set_invade_onefleet(
    dir: &Path,
    target_x: u8,
    target_y: u8,
    sc: u8,
    bb: u16,
    ca: u16,
    dd: u16,
    tt: u16,
    invasion_armies: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    with_runtime_game_mut_and_export(dir, |data| {
        {
            let f = &mut data.fleets[2];
            // The preserved fixture holds the old "invasion armies" byte at
            // 0x08, which classic reads as the high byte of the previous-fleet
            // word.
            f.prev.0 = (f.prev.0 & 0x00FF) | (u16::from(invasion_armies) << 8);
            f.max_speed = 3;
            f.speed = 3;
            f.order = Order::InvadeWorld;
            f.target = Coords::new(target_x, target_y);
            f.ships.scouts = u16::from(sc);
            f.ships.battleships = bb;
            f.ships.cruisers = ca;
            f.ships.destroyers = dd;
            set_troop_transports(f, tt);
        }
        {
            let p = data.planets.get_mut(13).ok_or("planet record 14 missing")?;
            *p = Planet {
                sector: Coords::new(target_x, target_y),
                potential_production: 100,
                production: Real48::from_int(100),
                stored_goods: Real48::from_int(66),
                name: "TargetPrime".to_string(),
                armies: 0x8e,
                ground_batteries: 0x0f,
                years_under_owner: 0x02,
                owner: EmpireId(0x02),
                ..Planet::default()
            };
        }
        Ok(())
    })?;

    println!(
        "  FLEET[3].order=InvadeWorld tgt=({}, {}) army={} {}",
        target_x,
        target_y,
        invasion_armies,
        invade_ships(sc, bb, ca, dd, tt)
    );
    println!(
        "  PLANET[14]: Dust Bowl target world at ({}, {})",
        target_x, target_y
    );
    Ok(())
}

pub(crate) fn init_invade(
    source: &Path,
    target: &Path,
    target_x: u8,
    target_y: u8,
    sc: u8,
    bb: u16,
    ca: u16,
    dd: u16,
    tt: u16,
    invasion_armies: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    copy_init_files(source, target)?;
    set_invade_onefleet(
        target,
        target_x,
        target_y,
        sc,
        bb,
        ca,
        dd,
        tt,
        invasion_armies,
    )?;
    export_runtime_snapshot_in_place(target)?;
    println!("Invade directory initialized at {}", target.display());
    Ok(())
}

pub(crate) fn init_invade_batch(
    source: &Path,
    target_root: &Path,
    specs: &[(u8, u8, u8, u16, u16, u16, u16, u8)],
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(target_root)?;
    let mut manifest = String::new();
    manifest.push_str("Invade batch\n");
    manifest.push_str(&format!("source={}\n", display_repo_path(source)));
    manifest.push_str(&format!("target_root={}\n", target_root.display()));
    manifest.push('\n');

    for &(x, y, sc, bb, ca, dd, tt, armies) in specs {
        let name = format!(
            "x{:02}-y{:02}-sc{}-bb{}-ca{}-dd{}-tt{}-a{}",
            x, y, sc, bb, ca, dd, tt, armies
        );
        let scenario_dir = target_root.join(&name);
        init_invade(source, &scenario_dir, x, y, sc, bb, ca, dd, tt, armies)?;
        manifest.push_str(&format!("{name}\n"));
        manifest.push_str(&format!("  target=[{}, {}]\n", x, y));
        manifest.push_str(&format!(
            "  {} armies={}\n",
            invade_ships(sc, bb, ca, dd, tt),
            armies
        ));
        manifest.push_str(&format!("  dir={}\n", scenario_dir.display()));
        manifest.push_str(&format!(
            "  validate=nc-cli compliance-report {}\n\n",
            scenario_dir.display()
        ));
    }

    fs::write(target_root.join("INVADE_BATCH.txt"), manifest)?;
    println!(
        "Initialized {} Invade directories under {}",
        specs.len(),
        target_root.display()
    );
    Ok(())
}

pub(crate) fn validate_invade_data(data: &CoreGameData) -> Result<(), Box<dyn std::error::Error>> {
    let mut errors = Vec::new();

    // Fleet 2 (empire=1, slot=3): InvadeWorld, speed=3/3, target (15,13),
    // army_count=100, SC=0, BB=100, CA=50, DD=50, TT=50
    match data.fleets.get(2) {
        None => errors.push("missing fleet record 3".to_string()),
        Some(f) => {
            if f.prev.0 != 0x6402 {
                errors.push(format!(
                    "FLEET[3].previous_fleet_id expected 0x6402, got {:#06x}",
                    f.prev.0
                ));
            }
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
            if f.order.code() != 0x07 {
                errors.push(format!(
                    "FLEET[3].order expected 0x07 (InvadeWorld), got {:#04x}",
                    f.order.code()
                ));
            }
            if f.target.to_array() != [0x0f, 0x0d] {
                errors.push(format!(
                    "FLEET[3].target expected (15,13), got {:?}",
                    f.target.to_array()
                ));
            }
            if f.ships.scouts != 0 {
                errors.push(format!("FLEET[3].sc expected 0, got {}", f.ships.scouts));
            }
            if f.ships.battleships != 100 {
                errors.push(format!(
                    "FLEET[3].bb expected 100, got {}",
                    f.ships.battleships
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
            if troop_transports(f) != 50 {
                errors.push(format!(
                    "FLEET[3].tt expected 50, got {}",
                    troop_transports(f)
                ));
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
        println!("Valid invade scenario");
        println!(
            "  FLEET[3]: order=0x07 (InvadeWorld) tgt=(15,13) speed=3/3 army=100 {}",
            invade_ships(0, 100, 50, 50, 50)
        );
        println!("  PLANET[14]: (15,13) empire=2 armies=142 batteries=15");
        Ok(())
    } else {
        Err(errors.join("\n").into())
    }
}

/// The scenario fleet's ships in the ship notation. Its transports are empty:
/// the invasion armies are a separate figure.
fn invade_ships(sc: u8, bb: u16, ca: u16, dd: u16, tt: u16) -> String {
    Composition::undamaged([
        (Scouts, sc.into()),
        (Battleships, bb.into()),
        (Cruisers, ca.into()),
        (Destroyers, dd.into()),
        (EmptyTransports, tt.into()),
    ])
    .notation()
}
