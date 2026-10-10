use std::fs;
use std::path::Path;

use crate::commands::runtime::{
    export_runtime_snapshot_in_place, load_runtime_game_data, with_runtime_game_mut,
};
use crate::support::paths::display_repo_path;
use crate::workspace::copy_init_files;

pub(crate) fn apply_guard_starbase_scenario(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    set_guard_starbase_onebase(dir, 0x10, 0x0D)?;
    println!("Applied scenario: guard-starbase");
    Ok(())
}

pub(crate) fn set_guard_starbase_onebase(
    dir: &Path,
    target_x: u8,
    target_y: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    with_runtime_game_mut(dir, |data| {
        data.set_guard_starbase(1, 1, [target_x, target_y], 1, 1)
            .map_err(|err| err.to_string().into())
    })?;

    println!("  PLAYER[1].starbase_count_raw = 1");
    println!("  FLEET[1].order = 0x04, aux = [01, 01]");
    println!(
        "  BASES.DAT = structured single-base record at ({}, {}) for empire 1",
        target_x, target_y
    );
    Ok(())
}

pub(crate) fn validate_guard_starbase_scenario(
    dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = load_runtime_game_data(dir)?;
    let errors = data.guard_starbase_linkage_errors_current_known(1, 1);

    if errors.is_empty() {
        let linkage = data.guard_starbase_linkage_summary_current_known(1, 1)?;
        let fleet = &data.fleets.records[0];
        let player1 = &data.player.records[0];
        let base = &data.bases.records[0];
        println!("Valid guard-starbase scenario");
        println!("  PLAYER[1].starbase_count_raw = 1");
        println!(
            "  linkage keys: player[44]={} fleet[00]={} fleet[05]={} base[07]={}",
            player1.starbase_count_raw(),
            fleet.local_slot_word_raw(),
            fleet.fleet_id_word_raw(),
            base.chain_word_raw()
        );
        println!(
            "  one-base guard-starbase linkage holds at coords {:?}",
            base.coords_raw()
        );
        println!(
            "  selected_base: present={} id={:?} summary={:?} owner={:?}",
            linkage.selected_base_present,
            linkage.selected_base_id,
            linkage.selected_base_summary_word,
            linkage.selected_base_owner_empire
        );
        Ok(())
    } else {
        Err(errors.join("\n").into())
    }
}

pub(crate) fn print_guard_starbase_report(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let data = load_runtime_game_data(dir)?;
    let player1 = data
        .player
        .records
        .first()
        .ok_or("PLAYER.DAT missing record 1")?;
    let fleet1 = data
        .fleets
        .records
        .first()
        .ok_or("FLEETS.DAT missing record 1")?;

    println!("Guard Starbase Report");
    println!("  dir={}", dir.display());
    println!(
        "  player[1].fleet_chain_head_raw={}",
        player1.fleet_chain_head_raw()
    );
    println!(
        "  player[1].starbase_count_raw={}",
        player1.starbase_count_raw()
    );
    println!(
        "  fleet[1].local_slot_word_raw={}",
        fleet1.local_slot_word_raw()
    );
    println!(
        "  fleet[1].next_fleet_link_word_raw={}",
        fleet1.next_fleet_link_word_raw()
    );
    println!(
        "  fleet[1].fleet_id_word_raw={}",
        fleet1.fleet_id_word_raw()
    );
    println!(
        "  fleet[1].order={:#04x} target={:?} guard_index={}",
        fleet1.standing_order_code_raw(),
        fleet1.standing_order_target_coords_raw(),
        fleet1.guard_starbase_index_raw()
    );
    if let Ok(linkage) = data.guard_starbase_linkage_summary_current_known(1, 1) {
        println!(
            "  selected_base.present={} selected_base.id={:?} selected_base.summary={:?} selected_base.chain={:?} selected_base.owner={:?}",
            linkage.selected_base_present,
            linkage.selected_base_id,
            linkage.selected_base_summary_word,
            linkage.selected_base_chain_word,
            linkage.selected_base_owner_empire
        );
    }

    if let Some(base1) = data.bases.records.first() {
        println!("  base_count={}", data.bases.records.len());
        println!(
            "  base[1].slot={} summary_word={} id={} link={:#06x} chain={:#06x} coords={:?} trailing={:?} owner={}",
            base1.local_slot_raw(),
            base1.summary_word_raw(),
            base1.base_id_raw(),
            base1.link_word_raw(),
            base1.chain_word_raw(),
            base1.coords_raw(),
            base1.trailing_coords_raw(),
            base1.owner_empire_raw()
        );
    } else {
        println!("  base_count=0");
    }

    match validate_guard_starbase_scenario(dir) {
        Ok(()) => println!("  verdict=valid one-base guard-starbase linkage"),
        Err(err) => println!("  verdict=invalid: {err}"),
    }

    Ok(())
}

pub(crate) fn init_guard_starbase_onebase(
    source: &Path,
    target: &Path,
    target_x: u8,
    target_y: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    copy_init_files(source, target)?;
    set_guard_starbase_onebase(target, target_x, target_y)?;
    export_runtime_snapshot_in_place(target)?;
    println!(
        "Guard Starbase directory initialized at {}",
        target.display()
    );
    Ok(())
}

pub(crate) fn init_guard_starbase_batch(
    source: &Path,
    target_root: &Path,
    coords: &[[u8; 2]],
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(target_root)?;
    let mut manifest = String::new();
    manifest.push_str("Guard Starbase batch\n");
    manifest.push_str(&format!("source={}\n", display_repo_path(source)));
    manifest.push_str(&format!("target_root={}\n", target_root.display()));
    manifest.push('\n');

    for [x, y] in coords {
        let name = format!("x{:02}-y{:02}", x, y);
        let scenario_dir = target_root.join(&name);
        init_guard_starbase_onebase(source, &scenario_dir, *x, *y)?;
        manifest.push_str(&format!("{name}\n"));
        manifest.push_str(&format!("  target=[{}, {}]\n", x, y));
        manifest.push_str(&format!("  dir={}\n", scenario_dir.display()));
        manifest.push_str(&format!(
            "  validate=nc-cli compliance-report {}\n\n",
            scenario_dir.display()
        ));
    }

    fs::write(target_root.join("GUARD_STARBASES.txt"), manifest)?;
    println!(
        "Initialized {} Guard Starbase directories under {}",
        coords.len(),
        target_root.display()
    );
    Ok(())
}
