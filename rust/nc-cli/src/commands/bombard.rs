use std::fs;
use std::path::Path;

use nc_data::ShipCategory::{Cruisers, Destroyers};
use nc_data::{Composition, Coords, CoreGameData, Order};

use crate::commands::fleet_order::set_raw_fleet_order;
use crate::commands::runtime::{
    export_runtime_snapshot_in_place, with_runtime_game_mut_and_export,
};
use crate::support::paths::display_repo_path;
use crate::workspace::copy_init_files;

/// Apply the bombard scenario to an already-initialized game directory.
///
/// Fixture-specific constants for this scenario:
/// - Fleet 3 (player 1, slot 3): BombardWorld order (0x06), speed 3, max_speed 3,
///   targeting planet 14 at (15, 13), CA=3, DD=5
/// - Planet 14: seeded as an owned planet for empire 2 by cloning planet 13's raw layout
///   and changing only the x-coordinate to 15 (0x0F)
///
/// All record indices and constants here are scenario-specific; the general mutators
/// (set_fleet_order, etc.) live in nc-data and accept parameters.
pub(crate) fn apply_bombard_scenario(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    set_bombard_onefleet(dir, 0x0F, 0x0D, 3, 5)?;
    println!("Applied scenario: bombard");
    Ok(())
}

/// Set up a single bombard fleet order plus planet seed for parameterized use.
///
/// Parameters:
/// - `target_x`, `target_y`: coordinates for fleet order and planet-14 clone's x-coord override
/// - `ca`: cruiser count for fleet 3 (fixture default: 3)
/// - `dd`: destroyer count for fleet 3 (fixture default: 5)
///
/// Scenario-specific constants (fleet 3, planet 13 → 14 clone) live here; general
/// mutators live in nc-data.
pub(crate) fn set_bombard_onefleet(
    dir: &Path,
    target_x: u8,
    target_y: u8,
    ca: u16,
    dd: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    with_runtime_game_mut_and_export(dir, |data| {
        {
            let fleet3 = &mut data.fleets[2];
            fleet3.max_speed = 3;
            fleet3.speed = 3;
            fleet3.ships.cruisers = ca;
            fleet3.ships.destroyers = dd;
        }
        {
            let template = data
                .planets
                .get(12)
                .ok_or("planet record 13 missing")?
                .clone();
            let p14 = data.planets.get_mut(13).ok_or("planet record 14 missing")?;
            *p14 = template;
            p14.sector = Coords::new(target_x, target_y);
        }
        set_raw_fleet_order(
            data,
            3,
            3,
            Order::BombardWorld.code(),
            [target_x, target_y],
            None,
            None,
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    })?;

    println!(
        "  FLEET[3].order = 0x06 (BombardWorld), target = ({}, {}), {}",
        target_x,
        target_y,
        Composition::undamaged([(Cruisers, ca.into()), (Destroyers, dd.into())]).notation()
    );
    println!(
        "  PLANET[14]: homeworld clone empire=2, coords=({}, {})",
        target_x, target_y
    );
    Ok(())
}

pub(crate) fn init_bombard(
    source: &Path,
    target: &Path,
    target_x: u8,
    target_y: u8,
    ca: u16,
    dd: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    copy_init_files(source, target)?;
    set_bombard_onefleet(target, target_x, target_y, ca, dd)?;
    export_runtime_snapshot_in_place(target)?;
    println!("Bombard directory initialized at {}", target.display());
    Ok(())
}

pub(crate) fn init_bombard_batch(
    source: &Path,
    target_root: &Path,
    specs: &[(u8, u8, u16, u16)],
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(target_root)?;
    let mut manifest = String::new();
    manifest.push_str("Bombard batch\n");
    manifest.push_str(&format!("source={}\n", display_repo_path(source)));
    manifest.push_str(&format!("target_root={}\n", target_root.display()));
    manifest.push('\n');

    for &(target_x, target_y, ca, dd) in specs {
        let name = format!("x{:02}-y{:02}-ca{}-dd{}", target_x, target_y, ca, dd);
        let scenario_dir = target_root.join(&name);
        init_bombard(source, &scenario_dir, target_x, target_y, ca, dd)?;
        manifest.push_str(&format!("{name}\n"));
        manifest.push_str(&format!("  target=[{}, {}]\n", target_x, target_y));
        manifest.push_str(&format!("  CA={} DD={}\n", ca, dd));
        manifest.push_str(&format!("  dir={}\n", scenario_dir.display()));
        manifest.push_str(&format!(
            "  validate=nc-cli compliance-report {}\n\n",
            scenario_dir.display()
        ));
    }

    fs::write(target_root.join("BOMBARD_BATCH.txt"), manifest)?;
    println!(
        "Initialized {} Bombard directories under {}",
        specs.len(),
        target_root.display()
    );
    Ok(())
}

pub(crate) fn validate_bombard_data(data: &CoreGameData) -> Result<(), Box<dyn std::error::Error>> {
    let mut errors = Vec::new();

    // Fleet 3: BombardWorld order targeting (15,13), speed 3, max_speed 3
    match data.fleets.get(2) {
        None => errors.push("missing fleet record 3".to_string()),
        Some(fleet3) => {
            if fleet3.max_speed != 3 {
                errors.push(format!(
                    "FLEET[3].max_speed expected 3, got {}",
                    fleet3.max_speed
                ));
            }
            if fleet3.speed != 3 {
                errors.push(format!(
                    "FLEET[3].current_speed expected 3, got {}",
                    fleet3.speed
                ));
            }
            if fleet3.order.code() != 0x06 {
                errors.push(format!(
                    "FLEET[3].order expected 0x06 (bombard), got {:#04x}",
                    fleet3.order.code()
                ));
            }
            if fleet3.target.to_array() != [0x0F, 0x0D] {
                errors.push(format!(
                    "FLEET[3].target expected (15,13), got {:?}",
                    fleet3.target.to_array()
                ));
            }
        }
    }

    // Planet 14: owned by empire 2, at (15,13), with armies and batteries
    match data.planets.get(13) {
        None => errors.push("missing planet record 14".to_string()),
        Some(p) => {
            if p.sector.to_array() != [0x0F, 0x0D] {
                errors.push(format!(
                    "PLANET[14].coords expected (15,13), got {:?}",
                    p.sector.to_array()
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
                    "PLANET[14].owner_empire_slot expected 2, got {}",
                    p.owner.0
                ));
            }
            if p.armies != 10 {
                errors.push(format!(
                    "PLANET[14].army_count expected 10, got {}",
                    u8::try_from(p.armies).unwrap_or(u8::MAX)
                ));
            }
            if p.ground_batteries != 4 {
                errors.push(format!(
                    "PLANET[14].ground_batteries expected 4, got {}",
                    u8::try_from(p.ground_batteries).unwrap_or(u8::MAX)
                ));
            }
        }
    }

    if errors.is_empty() {
        println!("Valid bombard scenario");
        println!("  FLEET[3].max_speed = 3");
        println!("  FLEET[3].order = 0x06 (BombardWorld)");
        println!("  FLEET[3].target = (15, 13)");
        println!("  PLANET[14]: homeworld seed empire=2 armies=10 batteries=4");
        Ok(())
    } else {
        Err(errors.join("\n").into())
    }
}
