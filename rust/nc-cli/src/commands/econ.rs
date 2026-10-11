use std::fs;
use std::path::Path;

use nc_data::{CoreGameData, Order};

use crate::commands::runtime::{
    export_runtime_snapshot_in_place, with_runtime_game_mut_and_export,
};
use crate::support::paths::display_repo_path;
use crate::workspace::copy_init_files;

/// Apply the econ scenario to an already-initialized game directory.
///
/// Fixture-specific constants for this scenario:
/// - Fleet 3 (empire=1, slot=3, index=2): BombardWorld (0x06) order, speed=3/3,
///   target (15,13), BB=50, CA=50
/// - Planet 14 (index=13): set via direct raw byte assignment (Dust Bowl-type seeded world
///   at (15,13), owned by empire 2, armies=142, batteries=15)
///
/// Fleet 2 mutations are identical to the fleet-battle scenario's fleet 2 changes.
/// Planet 14 raw bytes are identical to the fleet-battle, bombard, and invade pre-fixtures.
///
/// All record indices and constants here are scenario-specific; the general mutators live in
/// nc-data and accept parameters.
pub(crate) fn apply_econ_scenario(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    set_econ(
        dir, 0x0f, 0x0d, // target_x, target_y (15,13)
        0, 50, 50, // bb=0, ca=50, dd=50 (BB not set in original econ)
        0x0f, 0x0d, 142, 15, // p14_x, p14_y, p14_armies, p14_batteries
    )?;
    println!("Applied scenario: econ");
    Ok(())
}

/// Set up an econ scenario with parameterized coordinates and ship counts.
///
/// Parameters:
/// - `target_x`, `target_y`: coordinates for fleet order and planet-14
/// - `bb`, `ca`, `dd`: ship counts for fleet 3 (bombardment fleet)
/// - `p14_x`, `p14_y`: Planet 14 coordinates
/// - `p14_armies`, `p14_batteries`: Planet 14 defenses
pub(crate) fn set_econ(
    dir: &Path,
    target_x: u8,
    target_y: u8,
    bb: u16,
    ca: u16,
    dd: u16,
    p14_x: u8,
    p14_y: u8,
    p14_armies: u8,
    p14_batteries: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    with_runtime_game_mut_and_export(dir, |data| {
        {
            let f = &mut data.fleets[2];
            f.set_max_speed(3);
            f.set_current_speed(3);
            f.set_standing_order_kind(Order::BombardWorld);
            f.set_standing_order_target_coords_raw([target_x, target_y]);
            f.set_battleship_count(bb);
            f.set_cruiser_count(ca);
            f.set_destroyer_count(dd);
        }
        {
            let p = data.planets.get_mut(13).ok_or("planet record 14 missing")?;
            p.set_as_owned_target_world(
                [p14_x, p14_y],
                [0x64, 0x87],
                [0x00, 0x00, 0x00, 0x00, 0x48, 0x87],
                0x04,
                0x0b,
                *b"TargetPrimeet",
                [0x05, 0x1d, 0x0b, 0x11, 0x25, 0x1c, 0x05],
                p14_armies,
                p14_batteries,
                0x02,
                0x02,
            );
        }
        Ok(())
    })?;

    println!(
        "  FLEET[3]: order=BombardWorld tgt=({}, {}) speed=3/3 BB={} CA={} DD={}",
        target_x, target_y, bb, ca, dd
    );
    println!(
        "  PLANET[14]: ({}, {}) empire=2 armies={} batteries={}",
        p14_x, p14_y, p14_armies, p14_batteries
    );
    Ok(())
}

/// Initialize an econ scenario directory from a source baseline.
pub(crate) fn init_econ(
    source: &Path,
    target: &Path,
    target_x: u8,
    target_y: u8,
    bb: u16,
    ca: u16,
    dd: u16,
    p14_x: u8,
    p14_y: u8,
    p14_armies: u8,
    p14_batteries: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    copy_init_files(source, target)?;
    set_econ(
        target,
        target_x,
        target_y,
        bb,
        ca,
        dd,
        p14_x,
        p14_y,
        p14_armies,
        p14_batteries,
    )?;
    export_runtime_snapshot_in_place(target)?;
    println!("Econ directory initialized at {}", target.display());
    Ok(())
}

/// Initialize a batch of econ scenario directories.
///
/// Spec format: x:y:bb:ca:dd:p14x:p14y:p14a:p14b
pub(crate) fn init_econ_batch(
    source: &Path,
    target_root: &Path,
    specs: &[(u8, u8, u16, u16, u16, u8, u8, u8, u8)],
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(target_root)?;
    let mut manifest = String::new();
    manifest.push_str("Econ batch\n");
    manifest.push_str(&format!("source={}\n", display_repo_path(source)));
    manifest.push_str(&format!("target_root={}\n", target_root.display()));
    manifest.push('\n');

    for &(x, y, bb, ca, dd, p14x, p14y, p14a, p14b) in specs {
        let name = format!(
            "x{:02}-y{:02}-bb{}-ca{}-dd{}-p14x{:02}-p14y{:02}-p14a{}-p14b{}",
            x, y, bb, ca, dd, p14x, p14y, p14a, p14b
        );
        let scenario_dir = target_root.join(&name);
        init_econ(
            source,
            &scenario_dir,
            x,
            y,
            bb,
            ca,
            dd,
            p14x,
            p14y,
            p14a,
            p14b,
        )?;
        manifest.push_str(&format!("{name}\n"));
        manifest.push_str(&format!("  target=[{}, {}]\n", x, y));
        manifest.push_str(&format!("  BB={} CA={} DD={}\n", bb, ca, dd));
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

    fs::write(target_root.join("ECON_BATCH.txt"), manifest)?;
    println!(
        "Initialized {} Econ directories under {}",
        specs.len(),
        target_root.display()
    );
    Ok(())
}

pub(crate) fn validate_econ_data(data: &CoreGameData) -> Result<(), Box<dyn std::error::Error>> {
    let mut errors = Vec::new();

    // Fleet 2 (empire=1, slot=3): BombardWorld, speed=3/3, target (15,13), BB=50, CA=50
    match data.fleets.get(2) {
        None => errors.push("missing fleet record 3".to_string()),
        Some(f) => {
            if f.max_speed() != 3 {
                errors.push(format!(
                    "FLEET[3].max_speed expected 3, got {}",
                    f.max_speed()
                ));
            }
            if f.current_speed() != 3 {
                errors.push(format!(
                    "FLEET[3].current_speed expected 3, got {}",
                    f.current_speed()
                ));
            }
            if f.standing_order_code_raw() != 0x06 {
                errors.push(format!(
                    "FLEET[3].order expected 0x06 (BombardWorld), got {:#04x}",
                    f.standing_order_code_raw()
                ));
            }
            if f.standing_order_target_coords_raw() != [0x0f, 0x0d] {
                errors.push(format!(
                    "FLEET[3].target expected (15,13), got {:?}",
                    f.standing_order_target_coords_raw()
                ));
            }
            if f.cruiser_count() != 50 {
                errors.push(format!(
                    "FLEET[3].ca expected 50, got {}",
                    f.cruiser_count()
                ));
            }
            if f.destroyer_count() != 50 {
                errors.push(format!(
                    "FLEET[3].dd expected 50, got {}",
                    f.destroyer_count()
                ));
            }
        }
    }

    // Planet 14 (index 13): Dust Bowl-type world at (15,13), owned empire=2, armies=142, batteries=15
    match data.planets.get(13) {
        None => errors.push("missing planet record 14".to_string()),
        Some(p) => {
            if p.coords_raw() != [0x0f, 0x0d] {
                errors.push(format!(
                    "PLANET[14].coords expected (15,13), got {:?}",
                    p.coords_raw()
                ));
            }
            if p.army_count_raw() != 142 {
                errors.push(format!(
                    "PLANET[14].armies expected 142, got {}",
                    p.army_count_raw()
                ));
            }
            if p.ground_batteries_raw() != 15 {
                errors.push(format!(
                    "PLANET[14].batteries expected 15, got {}",
                    p.ground_batteries_raw()
                ));
            }
            if p.ownership_status_raw() != 2 {
                errors.push(format!(
                    "PLANET[14].ownership_status expected 2, got {}",
                    p.ownership_status_raw()
                ));
            }
            if p.owner_empire_slot_raw() != 2 {
                errors.push(format!(
                    "PLANET[14].owner_empire expected 2, got {}",
                    p.owner_empire_slot_raw()
                ));
            }
        }
    }

    if errors.is_empty() {
        println!("Valid econ scenario");
        println!("  FLEET[3]: order=0x06 (BombardWorld) tgt=(15,13) speed=3/3 CA=50 DD=50");
        println!("  PLANET[14]: (15,13) empire=2 armies=142 batteries=15");
        Ok(())
    } else {
        Err(errors.join("\n").into())
    }
}
