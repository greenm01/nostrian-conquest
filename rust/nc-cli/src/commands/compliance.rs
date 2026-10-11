use std::fs;
use std::path::Path;

use crate::commands::runtime::load_runtime_game_data;

pub(crate) fn print_compliance_report(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let data = load_runtime_game_data(dir)?;
    let status = data.current_known_compliance_status();
    let summary = data.current_known_key_word_summary();
    let preflight_errors = data.ecmaint_preflight_errors();

    println!("Compliance Report");
    println!("  dir={}", dir.display());
    println!();
    println!(
        "{} ecmaint-preflight{}",
        if preflight_errors.is_empty() {
            "OK  "
        } else {
            "FAIL"
        },
        render_errors_suffix(&preflight_errors)
    );
    println!(
        "{} guard-starbase-linkage{}",
        if status.guard_starbase {
            "OK  "
        } else {
            "FAIL"
        },
        render_errors_suffix(
            &data.guard_starbase_linkage_errors_for_guarding_fleets_current_known(1)
        )
    );
    println!(
        "{} ipbm-count-length{}",
        if status.ipbm { "OK  " } else { "FAIL" },
        render_errors_suffix(&data.ipbm_count_errors_current_known())
    );
    println!();
    println!(
        "Key words: player.first_base_record={} player.ipbm_count={}",
        summary.player_first_base_record, summary.player_ipbm_count
    );
    println!(
        "  guarding_fleet_count={}",
        data.guarding_fleet_record_indexes_current_known().len()
    );
    for linkage in data.guard_starbase_linkage_summaries_for_guarding_fleets_current_known(1) {
        println!(
            "  guarding_fleet[{}]: guard_index={} target={:?} selected_base.present={} selected_base.id={:?} selected_base.escort={:?} selected_base.owner={:?}",
            linkage.fleet_record_index_1_based,
            linkage.guard_index,
            linkage.target_coords,
            linkage.selected_base_present,
            linkage.selected_base_id,
            linkage.selected_base_escort_fleet_slot,
            linkage.selected_base_owner_empire
        );
    }
    if let (Some(local_slot), Some(fleet_id), Some(guard_index), Some(target)) = (
        summary.fleet1_local_slot,
        summary.fleet1_id,
        summary.fleet1_guard_index,
        summary.fleet1_target,
    ) {
        println!(
            "  fleet1.local_slot={} fleet1.id={} fleet1.guard={} target={:?}",
            local_slot, fleet_id, guard_index, target
        );
    }
    if let (Some(escort), Some(base_id), Some(record_number), Some(coords)) = (
        summary.base1_escort_fleet_slot,
        summary.base1_id,
        summary.base1_record_number,
        summary.base1_coords,
    ) {
        println!(
            "  base1.escort={} base1.id={} base1.record={} coords={:?}",
            escort, base_id, record_number, coords
        );
    } else {
        println!("  base1=<none>");
    }
    println!("  ipbm.record_count={}", summary.ipbm_record_count);
    Ok(())
}

pub(crate) fn print_compliance_batch_report(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("Compliance Batch Report");
    println!("  root={}", root.display());
    let mut dirs = fs::read_dir(root)?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            entry.file_type().ok().filter(|ty| ty.is_dir())?;
            Some(entry.path())
        })
        .collect::<Vec<_>>();
    dirs.sort();

    for dir in dirs {
        print!(
            "{}: ",
            dir.file_name().unwrap_or_default().to_string_lossy()
        );
        let status = load_runtime_game_data(&dir)
            .ok()
            .map(|data| data.current_known_compliance_status());
        println!(
            "fleet-order={} planet-build={} guard-starbase={} ipbm={}",
            if status.as_ref().map(|s| s.fleet_order).unwrap_or(false) {
                "ok"
            } else {
                "fail"
            },
            if status.as_ref().map(|s| s.planet_build).unwrap_or(false) {
                "ok"
            } else {
                "fail"
            },
            if status.as_ref().map(|s| s.guard_starbase).unwrap_or(false) {
                "ok"
            } else {
                "fail"
            },
            if status.as_ref().map(|s| s.ipbm).unwrap_or(false) {
                "ok"
            } else {
                "fail"
            }
        );
    }

    Ok(())
}

fn render_errors_suffix(errors: &[String]) -> String {
    if errors.is_empty() {
        String::new()
    } else {
        format!(": {}", errors.join("; "))
    }
}
