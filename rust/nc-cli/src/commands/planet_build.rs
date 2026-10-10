use std::fs;
use std::path::Path;

use crate::commands::runtime::{
    export_runtime_snapshot_in_place, load_runtime_game_data, with_runtime_game_mut,
};
use crate::support::paths::display_repo_path;
use crate::workspace::copy_init_files;

pub(crate) fn set_planet_build(
    dir: &Path,
    record_index_1_based: usize,
    slot_raw: u8,
    kind_raw: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    with_runtime_game_mut(dir, |data| {
        data.set_planet_build(record_index_1_based, slot_raw, kind_raw)
            .map_err(|err| err.to_string().into())
    })?;

    println!(
        "Planet record {} updated: build_slot={:#04x} build_kind={:#04x}",
        record_index_1_based, slot_raw, kind_raw
    );
    Ok(())
}

pub(crate) fn set_planet_owner(
    dir: &Path,
    record_index_1_based: usize,
    owner_slot: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    with_runtime_game_mut(dir, |data| {
        let record = data
            .planets
            .records
            .get_mut(record_index_1_based - 1)
            .ok_or_else(|| format!("planet record index out of range: {record_index_1_based}"))?;
        record.set_owner_empire_slot_raw(owner_slot);
        record.set_ownership_status_raw(if owner_slot == 0 { 0 } else { 2 });
        Ok(())
    })?;

    println!(
        "Planet record {} owner set to {}",
        record_index_1_based, owner_slot
    );
    Ok(())
}

pub(crate) fn set_planet_name(
    dir: &Path,
    record_index_1_based: usize,
    name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    with_runtime_game_mut(dir, |data| {
        let record = data
            .planets
            .records
            .get_mut(record_index_1_based - 1)
            .ok_or_else(|| format!("planet record index out of range: {record_index_1_based}"))?;
        record.set_planet_name(name);
        Ok(())
    })?;

    println!(
        "Planet record {} name set to '{}'",
        record_index_1_based, name
    );
    Ok(())
}

pub(crate) fn set_planet_stats(
    dir: &Path,
    record_index_1_based: usize,
    armies: u8,
    batteries: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    with_runtime_game_mut(dir, |data| {
        let record = data
            .planets
            .records
            .get_mut(record_index_1_based - 1)
            .ok_or_else(|| format!("planet record index out of range: {record_index_1_based}"))?;
        record.set_army_count_raw(armies);
        record.set_ground_batteries_raw(batteries);
        Ok(())
    })?;

    println!(
        "Planet record {} stats set: armies={}, batteries={}",
        record_index_1_based, armies, batteries
    );
    Ok(())
}

pub(crate) fn set_planet_potential(
    dir: &Path,
    record_index_1_based: usize,
    p1: u8,
    p2: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    with_runtime_game_mut(dir, |data| {
        let record = data
            .planets
            .records
            .get_mut(record_index_1_based - 1)
            .ok_or_else(|| format!("planet record index out of range: {record_index_1_based}"))?;
        // Raw bytes: potential production, then present production's exponent.
        record.raw[0x02] = p1;
        record.raw[0x03] = p2;
        Ok(())
    })?;

    println!(
        "Planet record {} potential set to ({:#04x}, {:#04x})",
        record_index_1_based, p1, p2
    );
    Ok(())
}

pub(crate) fn set_planet_present(
    dir: &Path,
    record_index_1_based: usize,
    points: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    with_runtime_game_mut(dir, |data| {
        let record = data
            .planets
            .records
            .get_mut(record_index_1_based - 1)
            .ok_or_else(|| format!("planet record index out of range: {record_index_1_based}"))?;
        if !record.set_present_production_points(points) {
            return Err(format!("planet present production out of range: {points}").into());
        }
        Ok(())
    })?;

    println!(
        "Planet record {} present production points set to {}",
        record_index_1_based, points
    );
    Ok(())
}

pub(crate) fn set_planet_stored(
    dir: &Path,
    record_index_1_based: usize,
    points: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    with_runtime_game_mut(dir, |data| {
        let record = data
            .planets
            .records
            .get_mut(record_index_1_based - 1)
            .ok_or_else(|| format!("planet record index out of range: {record_index_1_based}"))?;
        record.set_stored_production_points(points);
        Ok(())
    })?;

    println!(
        "Planet record {} treasury set to {}",
        record_index_1_based, points
    );
    Ok(())
}

pub(crate) fn set_planet_stardock_slot(
    dir: &Path,
    record_index_1_based: usize,
    slot: usize,
    kind_raw: u8,
    count: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    if nc_data::stardock_slot_for_kind(kind_raw) != Some(slot) {
        return Err(format!("stardock slot {slot} does not hold kind {kind_raw:#04x}").into());
    }

    with_runtime_game_mut(dir, |data| {
        let record = data
            .planets
            .records
            .get_mut(record_index_1_based - 1)
            .ok_or_else(|| format!("planet record index out of range: {record_index_1_based}"))?;
        record.set_stardock_count_raw(slot, count);
        Ok(())
    })?;

    println!(
        "Planet record {} stardock slot {} set: kind={:#04x} count={}",
        record_index_1_based, slot, kind_raw, count
    );
    Ok(())
}

pub(crate) fn init_planet_original(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    with_runtime_game_mut(dir, |data| {
        // Homeworlds
        let hw_specs = [
            (
                5,
                [0x07, 0x04],
                4,
                "Not Named Yet",
                [0x64, 0x87],
                0x8648,
                12,
            ),
            (
                8,
                [0x10, 0x05],
                3,
                "Not Named Yet",
                [0x64, 0x87],
                0x8648,
                12,
            ),
            (
                13,
                [0x06, 0x0c],
                2,
                "Not Named Yet",
                [0x64, 0x87],
                0x8648,
                12,
            ),
            (16, [0x10, 0x0d], 1, "Dust Bowl", [0x64, 0x87], 0x8748, 4),
        ];

        for (idx, coords, owner, name, potential, fact_word, tax) in hw_specs {
            let record = &mut data.planets.records[idx - 1];
            record.set_coords_raw(coords);
            record.set_owner_empire_slot_raw(owner);
            record.set_ownership_status_raw(2);
            if idx == 16 {
                let mut buf = [0u8; 13];
                buf[..13].copy_from_slice(b"Dust Bowl Yet");
                record.set_planet_name_buffer(9, &buf);
            } else {
                record.set_planet_name(name);
            }
            // Raw bytes of the sample file: potential production, present
            // production (100), and stored goods.
            record.raw[0x02..0x04].copy_from_slice(&potential);
            record.raw[0x08..0x0A].copy_from_slice(&u16::to_le_bytes(fact_word));
            record.set_army_count_raw(if idx == 16 { 142 } else { 10 });
            record.set_ground_batteries_raw(if idx == 16 { 15 } else { 4 });
            record.raw[0x0E] = tax;
        }

        // Unowned planets
        let unowned_specs = [
            (1, [0x06, 0x01], [0x93, 0x00]),
            (2, [0x10, 0x01], [0x24, 0x00]),
            (3, [0x0f, 0x03], [0x36, 0x00]),
            (4, [0x05, 0x04], [0x3d, 0x00]),
            (6, [0x0f, 0x04], [0x1e, 0x00]),
            (7, [0x03, 0x05], [0x3e, 0x00]),
            (9, [0x11, 0x09], [0x48, 0x00]),
            (10, [0x05, 0x0b], [0x46, 0x00]),
            (11, [0x06, 0x0b], [0x3f, 0x00]),
            (12, [0x10, 0x0b], [0x66, 0x00]),
            (14, [0x0c, 0x0c], [0x23, 0x00]),
            (15, [0x0f, 0x0d], [0x41, 0x00]),
            (17, [0x11, 0x0f], [0x2f, 0x00]),
            (18, [0x04, 0x10], [0x4f, 0x00]),
            (19, [0x10, 0x10], [0x35, 0x00]),
            (20, [0x09, 0x12], [0x86, 0x00]),
        ];

        for (idx, coords, potential) in unowned_specs {
            let record = &mut data.planets.records[idx - 1];
            record.set_coords_raw(coords);
            record.set_owner_empire_slot_raw(0);
            record.set_ownership_status_raw(0);
            record.set_status_or_name_prefix_raw("Unowned");
            record.raw[0x02..0x04].copy_from_slice(&potential);
            record.raw[0x04..0x0A].fill(0);
            record.set_army_count_raw(0);
            record.set_ground_batteries_raw(0);
        }

        Ok(())
    })?;
    println!("Planet topology initialized to original sample state");
    Ok(())
}

pub(crate) fn apply_planet_build_scenario(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    set_planet_build(dir, 15, 0x03, 0x01)?;
    println!("Applied scenario: planet-build");
    Ok(())
}

pub(crate) fn print_planet_build_report(
    dir: &Path,
    record_index_1_based: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = load_runtime_game_data(dir)?;
    let record = data
        .planets
        .records
        .get(record_index_1_based - 1)
        .ok_or_else(|| format!("planet record index out of range: {record_index_1_based}"))?;
    println!("Planet Build Report");
    println!("  dir={}", dir.display());
    println!("  record={}", record_index_1_based);
    println!("  build_slot={:#04x}", record.build_count_raw(0));
    println!("  build_kind={:#04x}", record.build_kind_raw(0));
    println!("  stardock_count={:#04x}", record.stardock_count_raw(0));
    println!("  stardock_kind={:#04x}", record.stardock_kind_raw(0));
    println!("  owner={:#04x}", record.owner_empire_slot_raw());
    println!("  coords={:?}", record.coords_raw());
    println!("  summary={}", record.derived_summary());
    Ok(())
}

pub(crate) fn init_planet_build_scenario(
    source: &Path,
    target: &Path,
    record_index_1_based: usize,
    slot_raw: u8,
    kind_raw: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    copy_init_files(source, target)?;
    set_planet_build(target, record_index_1_based, slot_raw, kind_raw)?;
    export_runtime_snapshot_in_place(target)?;
    println!("Planet-build directory initialized at {}", target.display());
    Ok(())
}

pub(crate) fn init_planet_build_batch(
    source: &Path,
    target_root: &Path,
    specs: &[(usize, u8, u8)],
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(target_root)?;
    let mut manifest = String::new();
    manifest.push_str("Planet-build batch\n");
    manifest.push_str(&format!("source={}\n", display_repo_path(source)));
    manifest.push_str(&format!("target_root={}\n", target_root.display()));
    manifest.push('\n');

    for (record_index, slot_raw, kind_raw) in specs {
        let name = format!("p{:02}-s{:02x}-k{:02x}", record_index, slot_raw, kind_raw);
        let scenario_dir = target_root.join(&name);
        init_planet_build_scenario(source, &scenario_dir, *record_index, *slot_raw, *kind_raw)?;
        manifest.push_str(&format!("{name}\n"));
        manifest.push_str(&format!(
            "  spec={}:{:#04x}:{:#04x}\n",
            record_index, slot_raw, kind_raw
        ));
        manifest.push_str(&format!("  dir={}\n", scenario_dir.display()));
        manifest.push_str(&format!(
            "  validate=nc-cli validate {} planet-build\n\n",
            scenario_dir.display()
        ));
    }

    fs::write(target_root.join("PLANET_BUILDS.txt"), manifest)?;
    println!(
        "Initialized {} planet-build directories under {}",
        specs.len(),
        target_root.display()
    );
    Ok(())
}
