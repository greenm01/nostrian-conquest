use std::path::Path;

use nc_data::{CoreGameData, IPBM_RECORD_SIZE};

use crate::commands::runtime::{
    export_runtime_snapshot_in_place, load_runtime_game_data, with_runtime_game_mut,
};
use crate::support::paths::display_repo_path;
use crate::workspace::copy_init_files;

pub(crate) fn print_ipbm_report(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let data = load_runtime_game_data(dir)?;
    let ipbm_bytes = data.ipbm.to_bytes();

    println!("IPBM Report");
    println!("  dir={}", dir.display());
    println!(
        "  player[1].ipbm_chain_head={}",
        data.player.records[0].ipbm_chain_head_raw()
    );
    println!("  file_record_count={}", data.ipbm.records.len());
    println!("  file_size={}", ipbm_bytes.len());
    println!(
        "  expected_size_from_player1={}",
        data.player.records[0].ipbm_chain_head_raw() as usize * IPBM_RECORD_SIZE
    );

    for (idx, record) in data.ipbm.records.iter().enumerate() {
        println!(
            "  record {}: primary={:#06x} owner={} gate={:#06x} follow_on={:#06x} sector={:?} target={:?} target_depth={}",
            idx + 1,
            record.ipbm_number_raw(),
            record.owner_empire_raw(),
            record.next_ipbm_link_raw(),
            record.record_number_raw(),
            record.coords_raw(),
            record.target_coords_raw(),
            record.target_depth_raw()
        );
        println!("    x_fraction={:02x?}", record.x_fraction().to_bytes());
        println!("    y_fraction={:02x?}", record.y_fraction().to_bytes());
        println!("    depth={:02x?}", record.depth().to_bytes());
    }

    Ok(())
}

pub(crate) fn set_ipbm_zero_records(
    dir: &Path,
    count: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    with_runtime_game_mut(dir, |data| {
        data.set_ipbm_zero_records(count);
        Ok(())
    })?;

    println!("IPBM zero records written");
    println!("  player[1].ipbm_chain_head = {}", count);
    println!("  IPBM.DAT size = {}", count as usize * IPBM_RECORD_SIZE);
    Ok(())
}

pub(crate) fn set_ipbm_record_prefix(
    dir: &Path,
    record_index_1_based: usize,
    primary: u16,
    owner: u8,
    gate: u16,
    follow_on: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    with_runtime_game_mut(dir, |data| {
        data.set_ipbm_record_prefix(record_index_1_based, primary, owner, gate, follow_on)?;
        Ok(())
    })?;

    println!("IPBM record {} updated", record_index_1_based);
    println!(
        "  primary={:#06x} owner={} gate={:#06x} follow_on={:#06x}",
        primary, owner, gate, follow_on
    );
    Ok(())
}

pub(crate) fn validate_ipbm(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let data = CoreGameData::load(dir)?;
    let ipbm_bytes = data.ipbm.to_bytes();

    let errors = data.ipbm_count_length_errors_current_known();

    if errors.is_empty() {
        let expected_count = data.player1_ipbm_count_current_known();
        let actual_count = data.ipbm.records.len();
        println!("Valid IPBM count/length state");
        println!("  player[1].ipbm_chain_head = {}", expected_count);
        println!("  IPBM.DAT size = {}", ipbm_bytes.len());
        println!("  record_count = {}", actual_count);
        Ok(())
    } else {
        Err(errors.join("\n").into())
    }
}

pub(crate) fn init_ipbm_zero_records(
    source: &Path,
    target: &Path,
    count: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    copy_init_files(source, target)?;
    set_ipbm_zero_records(target, count)?;
    export_runtime_snapshot_in_place(target)?;
    println!("IPBM directory initialized at {}", target.display());
    Ok(())
}

pub(crate) fn apply_ipbm_scenario(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    with_runtime_game_mut(dir, |data| {
        data.set_ipbm_zero_records(0);
        Ok(())
    })
}

pub(crate) fn validate_ipbm_data(data: &CoreGameData) -> Result<(), Box<dyn std::error::Error>> {
    let errors = data.ipbm_count_length_errors_current_known();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n").into())
    }
}

pub(crate) fn init_ipbm_batch(
    source: &Path,
    target_root: &Path,
    counts: &[u16],
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(target_root)?;
    let mut manifest = String::new();
    manifest.push_str("IPBM batch\n");
    manifest.push_str(&format!("source={}\n", display_repo_path(source)));
    manifest.push_str(&format!("target_root={}\n", target_root.display()));
    manifest.push('\n');

    for count in counts {
        let name = format!("count-{:02}", count);
        let scenario_dir = target_root.join(&name);
        init_ipbm_zero_records(source, &scenario_dir, *count)?;
        manifest.push_str(&format!("{name}\n"));
        manifest.push_str(&format!("  count={}\n", count));
        manifest.push_str(&format!("  dir={}\n", scenario_dir.display()));
        manifest.push_str(&format!(
            "  validate=nc-cli ipbm-validate {}\n\n",
            scenario_dir.display()
        ));
    }

    std::fs::write(target_root.join("IPBM_BATCH.txt"), manifest)?;
    println!(
        "Initialized {} IPBM directories under {}",
        counts.len(),
        target_root.display()
    );
    Ok(())
}
