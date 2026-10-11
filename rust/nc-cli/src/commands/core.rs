use std::fs;
use std::path::Path;

use nc_compat::classic::{FLEET_RECORD_SIZE, PLANET_RECORD_SIZE, PLAYER_RECORD_SIZE};
use nc_compat::{
    core_file_diff_counts, core_file_diff_offsets, core_file_exact_match_errors,
    current_known_baseline_diff_counts, current_known_baseline_diff_offsets,
    ensure_classic_auxiliary_files, load_classic_core_files, save_classic_core_files,
};
use nc_data::{order_summary, planet_derived_summary};

use crate::commands::runtime::{
    load_runtime_game_data, with_runtime_game_mut_and_export, with_runtime_game_mut_and_export_core,
};
use crate::support::paths::{display_repo_path, post_maint_fixture_dir};
use crate::workspace::{
    copy_current_known_core_files, copy_top_level_files, generate_database_dat,
};

pub(crate) fn print_core_report(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let data = load_runtime_game_data(dir)?;
    let starbase_total = data.player1_starbase_count_current_known();
    let owned_base_total = data.player1_owned_base_record_count_current_known();
    let ipbm_total = data.player1_ipbm_count_current_known();
    let player_owned_planet_counts = data.player_owned_planet_counts_current_known();
    let player_owned_base_counts = data.player_owned_base_record_counts_current_known();
    let player_homeworld_seed_coords = data.player_homeworld_seed_coords_current_known();
    let player_fleet_chain_heads = data.player_fleet_chain_heads_current_known();
    let initialized_fleet_block_head_ids = data.current_known_initialized_fleet_block_head_ids();
    let campaign_contenders = data.campaign_contenders();
    let sole_contender = data.sole_contender();
    let campaign_outlook = data.campaign_outlook();
    let campaign_outcome = data.campaign_outcome();

    println!("Core State Report");
    println!("  dir={}", dir.display());
    println!("  player_record_count={}", data.players.len());
    println!("  planet_record_count={}", data.planets.len());
    println!("  fleet_record_count={}", data.fleets.len());
    println!("  base_record_count={}", data.bases.len());
    println!("  ipbm_record_count={}", data.ipbms.len());
    println!("  conquest.game_year={}", data.conquest.game_year);
    println!(
        "  initialized_fleet_blocks={}",
        data.looks_like_initialized_fleet_blocks_current_known()
    );
    println!(
        "  initialized_fleet_payloads={}",
        data.current_known_initialized_fleet_payload_errors()
            .is_empty()
    );
    println!(
        "  initialized_fleet_missions={}",
        data.current_known_initialized_fleet_mission_errors()
            .is_empty()
    );
    println!(
        "  initialized_homeworld_alignment={}",
        data.current_known_initialized_homeworld_alignment_errors()
            .is_empty()
    );
    println!(
        "  initialized_planet_ownership={}",
        data.current_known_initialized_planet_ownership_errors()
            .is_empty()
    );
    println!(
        "  homeworld_seed_payloads={}",
        data.current_known_homeworld_seed_payload_errors()
            .is_empty()
    );
    println!(
        "  unowned_planet_payloads={}",
        data.current_known_unowned_planet_payload_errors()
            .is_empty()
    );
    println!(
        "  empty_auxiliary_state={}",
        data.current_known_empty_auxiliary_state_errors().is_empty()
    );
    println!(
        "  setup_baseline={}",
        data.current_known_setup_baseline_errors().is_empty()
    );
    println!(
        "  conquest_baseline={}",
        data.current_known_conquest_baseline_errors().is_empty()
    );
    println!(
        "  initialized_fleet_block_head_ids={:?}",
        initialized_fleet_block_head_ids
    );
    println!("  player1_starbase_count={}", starbase_total);
    println!("  player1_owned_base_record_count={}", owned_base_total);
    println!("  player1_ipbm_count={}", ipbm_total);
    println!("  campaign_outlook={}", campaign_outlook.as_str());
    println!("  campaign_outcome={}", campaign_outcome.as_str());
    println!("  campaign_contenders={:?}", campaign_contenders);
    println!("  sole_contender={:?}", sole_contender);

    for (idx, record) in data.players.iter().enumerate() {
        let campaign_state = data
            .empire_campaign_state((idx + 1) as u8)
            .map(|state| state.as_str())
            .unwrap_or("unknown");
        println!(
            "  player {:02}: campaign_state={} owned_planet_count={} homeworld_seed_coords={:?} owned_base_count={} first_base_record_raw={} ipbm_count_word_raw={} fleet_chain_head_raw={}",
            idx + 1,
            campaign_state,
            player_owned_planet_counts[idx],
            player_homeworld_seed_coords.get(idx).copied().flatten(),
            player_owned_base_counts[idx],
            record.bases_first.0,
            record.ipbms_first.0,
            player_fleet_chain_heads[idx]
        );
    }

    Ok(())
}

pub(crate) fn print_current_known_baseline_diff(
    dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = load_runtime_game_data(dir)?;
    let diffs = current_known_baseline_diff_counts(&data);

    println!("Current-known Baseline Diff");
    println!("  dir={}", dir.display());
    for diff in diffs {
        println!("  {}: differing_bytes={}", diff.name, diff.differing_bytes);
    }

    Ok(())
}

pub(crate) fn print_canonical_current_known_baseline_diff(
    dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = load_runtime_game_data(dir)?;
    let baseline = load_classic_core_files(&post_maint_fixture_dir())?;
    let diffs = core_file_diff_counts(&data, &baseline);

    println!("Canonical Current-known Baseline Diff");
    println!("  dir={}", dir.display());
    for diff in diffs {
        println!("  {}: differing_bytes={}", diff.name, diff.differing_bytes);
    }

    Ok(())
}

pub(crate) fn print_current_known_baseline_diff_offsets(
    dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = load_runtime_game_data(dir)?;
    let diffs = current_known_baseline_diff_offsets(&data);

    println!("Current-known Baseline Diff Offsets");
    println!("  dir={}", dir.display());
    for diff in diffs {
        println!(
            "  {}: differing_offsets={:?}",
            diff.name, diff.differing_offsets
        );
    }

    Ok(())
}

pub(crate) fn print_canonical_current_known_baseline_diff_offsets(
    dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = load_runtime_game_data(dir)?;
    let baseline = load_classic_core_files(&post_maint_fixture_dir())?;
    let diffs = core_file_diff_offsets(&data, &baseline);

    println!("Canonical Current-known Baseline Diff Offsets");
    println!("  dir={}", dir.display());
    for diff in diffs {
        println!(
            "  {}: differing_offsets={:?}",
            diff.name, diff.differing_offsets
        );
    }

    Ok(())
}

pub(crate) fn print_canonical_transition_clusters(
    dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = load_runtime_game_data(dir)?;
    let baseline = load_classic_core_files(&post_maint_fixture_dir())?;
    let diffs = core_file_diff_offsets(&data, &baseline);

    println!("Canonical Transition Clusters");
    println!("  dir={}", dir.display());
    for diff in diffs {
        match diff.name {
            "PLAYER.DAT" => {
                print_record_clusters(diff.name, PLAYER_RECORD_SIZE, &diff.differing_offsets)
            }
            "PLANETS.DAT" => {
                print_record_clusters(diff.name, PLANET_RECORD_SIZE, &diff.differing_offsets)
            }
            "FLEETS.DAT" => {
                print_record_clusters(diff.name, FLEET_RECORD_SIZE, &diff.differing_offsets)
            }
            _ => println!(
                "  {}: differing_offsets={:?}",
                diff.name, diff.differing_offsets
            ),
        }
    }

    Ok(())
}

pub(crate) fn print_canonical_transition_details(
    dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = load_runtime_game_data(dir)?;
    let baseline = load_classic_core_files(&post_maint_fixture_dir())?;
    let diffs = core_file_diff_offsets(&data, &baseline);

    println!("Canonical Transition Details");
    println!("  dir={}", dir.display());

    for diff in diffs {
        match diff.name {
            "PLAYER.DAT" => {
                println!("  PLAYER.DAT:");
                for record_index in
                    unique_record_indexes(&diff.differing_offsets, PLAYER_RECORD_SIZE)
                {
                    let current = &data.players[record_index - 1];
                    let canonical = &baseline.players[record_index - 1];
                    println!(
                        "    record {} current: first_base_record_raw={} ipbm_chain_head_raw={} fleet_chain_head_raw={}",
                        record_index,
                        current.bases_first.0,
                        current.ipbms_first.0,
                        current.fleets_first.0
                    );
                    println!(
                        "    record {} canonical: first_base_record_raw={} ipbm_chain_head_raw={} fleet_chain_head_raw={}",
                        record_index,
                        canonical.bases_first.0,
                        canonical.ipbms_first.0,
                        canonical.fleets_first.0
                    );
                }
            }
            "PLANETS.DAT" => {
                println!("  PLANETS.DAT:");
                for record_index in
                    unique_record_indexes(&diff.differing_offsets, PLANET_RECORD_SIZE)
                {
                    let current = &data.planets[record_index - 1];
                    let canonical = &baseline.planets[record_index - 1];
                    println!(
                        "    record {} current: {}",
                        record_index,
                        planet_derived_summary(current)
                    );
                    println!(
                        "    record {} canonical: {}",
                        record_index,
                        planet_derived_summary(canonical)
                    );
                }
            }
            "FLEETS.DAT" => {
                println!("  FLEETS.DAT:");
                for record_index in
                    unique_record_indexes(&diff.differing_offsets, FLEET_RECORD_SIZE)
                {
                    let current = &data.fleets[record_index - 1];
                    let canonical = &baseline.fleets[record_index - 1];
                    println!(
                        "    record {} current: loc={:?} target={:?} order={} aux={:?}",
                        record_index,
                        current.sector.to_array(),
                        current.target.to_array(),
                        order_summary(current),
                        [current.target_depth, current.mission_target as u8]
                    );
                    println!(
                        "    record {} canonical: loc={:?} target={:?} order={} aux={:?}",
                        record_index,
                        canonical.sector.to_array(),
                        canonical.target.to_array(),
                        order_summary(canonical),
                        [canonical.target_depth, canonical.mission_target as u8]
                    );
                }
            }
            _ => {
                if !diff.differing_offsets.is_empty() {
                    println!(
                        "  {}: differing_offsets={:?}",
                        diff.name, diff.differing_offsets
                    );
                }
            }
        }
    }

    Ok(())
}

fn print_record_clusters(name: &str, record_size: usize, offsets: &[usize]) {
    println!("  {}:", name);
    if offsets.is_empty() {
        println!("    differing_offsets=[]");
        return;
    }

    let mut current_record: Option<usize> = None;
    let mut current_offsets: Vec<usize> = Vec::new();
    for offset in offsets {
        let record_index = offset / record_size + 1;
        let record_offset = offset % record_size;
        if current_record == Some(record_index) {
            current_offsets.push(record_offset);
        } else {
            if let Some(record_index) = current_record {
                println!("    record {} -> {:?}", record_index, current_offsets);
            }
            current_record = Some(record_index);
            current_offsets = vec![record_offset];
        }
    }

    if let Some(record_index) = current_record {
        println!("    record {} -> {:?}", record_index, current_offsets);
    }
}

fn unique_record_indexes(offsets: &[usize], record_size: usize) -> Vec<usize> {
    let mut result = Vec::new();
    let mut current = None;
    for offset in offsets {
        let record_index = offset / record_size + 1;
        if current != Some(record_index) {
            result.push(record_index);
            current = Some(record_index);
        }
    }
    result
}

pub(crate) fn validate_core_state(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let data = load_runtime_game_data(dir)?;
    let errors = data.current_known_core_state_errors();

    if errors.is_empty() {
        println!("Valid core state");
        println!("  base_record_count = {}", data.bases.len());
        println!(
            "  player1_starbase_count = {}",
            data.player1_starbase_count_current_known()
        );
        println!(
            "  player1_owned_base_record_count = {}",
            data.player1_owned_base_record_count_current_known()
        );
        println!("  ipbm_record_count = {}", data.ipbms.len());
        println!(
            "  player1_ipbm_count = {}",
            data.player1_ipbm_count_current_known()
        );
        println!(
            "  initialized_fleet_blocks = {}",
            data.looks_like_initialized_fleet_blocks_current_known()
        );
        println!(
            "  initialized_fleet_payloads = {}",
            data.current_known_initialized_fleet_payload_errors()
                .is_empty()
        );
        println!(
            "  initialized_fleet_missions = {}",
            data.current_known_initialized_fleet_mission_errors()
                .is_empty()
        );
        println!(
            "  initialized_homeworld_alignment = {}",
            data.current_known_initialized_homeworld_alignment_errors()
                .is_empty()
        );
        println!(
            "  initialized_planet_ownership = {}",
            data.current_known_initialized_planet_ownership_errors()
                .is_empty()
        );
        println!(
            "  homeworld_seed_payloads = {}",
            data.current_known_homeworld_seed_payload_errors()
                .is_empty()
        );
        println!(
            "  unowned_planet_payloads = {}",
            data.current_known_unowned_planet_payload_errors()
                .is_empty()
        );
        println!(
            "  empty_auxiliary_state = {}",
            data.current_known_empty_auxiliary_state_errors().is_empty()
        );
        println!(
            "  setup_baseline = {}",
            data.current_known_setup_baseline_errors().is_empty()
        );
        println!(
            "  conquest_baseline = {}",
            data.current_known_conquest_baseline_errors().is_empty()
        );
        println!(
            "  player_inputs = {}",
            data.current_known_player_input_errors().is_empty()
        );
        Ok(())
    } else {
        Err(errors.join("\n").into())
    }
}

pub(crate) fn validate_current_known_baseline_exact(
    dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = load_runtime_game_data(dir)?;
    let baseline = load_classic_core_files(&post_maint_fixture_dir())?;
    let errors = core_file_exact_match_errors(
        &data,
        &baseline,
        "canonical current-known post-maint baseline",
    );

    if errors.is_empty() {
        println!("Exact canonical current-known baseline match");
        println!("  dir = {}", dir.display());
        Ok(())
    } else {
        Err(errors.join("\n").into())
    }
}

pub(crate) fn sync_core_counts(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let data = with_runtime_game_mut_and_export(dir, |data| {
        data.sync_player1_current_known_counts();
        Ok(data.clone())
    })?;

    println!("Core counts synchronized");
    println!(
        "  player1_starbase_count = {}",
        data.player1_starbase_count_current_known()
    );
    println!(
        "  player1_owned_base_record_count = {}",
        data.player1_owned_base_record_count_current_known()
    );
    println!(
        "  player1_ipbm_count = {}",
        data.player1_ipbm_count_current_known()
    );
    println!(
        "  initialized_fleet_blocks = {}",
        data.looks_like_initialized_fleet_blocks_current_known()
    );
    println!(
        "  initialized_fleet_payloads = {}",
        data.current_known_initialized_fleet_payload_errors()
            .is_empty()
    );
    println!(
        "  initialized_fleet_missions = {}",
        data.current_known_initialized_fleet_mission_errors()
            .is_empty()
    );
    println!(
        "  initialized_homeworld_alignment = {}",
        data.current_known_initialized_homeworld_alignment_errors()
            .is_empty()
    );
    println!(
        "  initialized_planet_ownership = {}",
        data.current_known_initialized_planet_ownership_errors()
            .is_empty()
    );
    println!(
        "  homeworld_seed_payloads = {}",
        data.current_known_homeworld_seed_payload_errors()
            .is_empty()
    );
    println!(
        "  unowned_planet_payloads = {}",
        data.current_known_unowned_planet_payload_errors()
            .is_empty()
    );
    println!(
        "  empty_auxiliary_state = {}",
        data.current_known_empty_auxiliary_state_errors().is_empty()
    );
    println!(
        "  setup_baseline = {}",
        data.current_known_setup_baseline_errors().is_empty()
    );
    println!(
        "  conquest_baseline = {}",
        data.current_known_conquest_baseline_errors().is_empty()
    );
    for (idx, owned_base_count) in data
        .player_owned_base_record_counts_current_known()
        .into_iter()
        .enumerate()
    {
        let fleet_chain_head = data.player_fleet_chain_heads_current_known()[idx];
        println!(
            "  player {:02}: owned_planet_count = {} homeworld_seed_coords = {:?} owned_base_count = {} first_base_record_raw = {} ipbm_count_word_raw = {} fleet_chain_head_raw = {}",
            idx + 1,
            data.player_owned_planet_counts_current_known()[idx],
            data.player_homeworld_seed_coords_current_known()
                .get(idx)
                .copied()
                .flatten(),
            owned_base_count,
            data.players[idx].bases_first.0,
            data.players[idx].ipbms_first.0,
            fleet_chain_head
        );
    }
    Ok(())
}

pub(crate) fn sync_core_baseline(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let data = with_runtime_game_mut_and_export_core(dir, |data| {
        data.sync_current_known_baseline_controls_and_counts();
        Ok(data.clone())
    })?;

    println!("Core baseline synchronized");
    println!(
        "  player1_starbase_count = {}",
        data.player1_starbase_count_current_known()
    );
    println!(
        "  player1_owned_base_record_count = {}",
        data.player1_owned_base_record_count_current_known()
    );
    println!(
        "  player1_ipbm_count = {}",
        data.player1_ipbm_count_current_known()
    );
    println!(
        "  initialized_fleet_blocks = {}",
        data.looks_like_initialized_fleet_blocks_current_known()
    );
    println!(
        "  initialized_fleet_payloads = {}",
        data.current_known_initialized_fleet_payload_errors()
            .is_empty()
    );
    println!(
        "  initialized_fleet_missions = {}",
        data.current_known_initialized_fleet_mission_errors()
            .is_empty()
    );
    println!(
        "  initialized_homeworld_alignment = {}",
        data.current_known_initialized_homeworld_alignment_errors()
            .is_empty()
    );
    println!(
        "  initialized_planet_ownership = {}",
        data.current_known_initialized_planet_ownership_errors()
            .is_empty()
    );
    println!(
        "  homeworld_seed_payloads = {}",
        data.current_known_homeworld_seed_payload_errors()
            .is_empty()
    );
    println!(
        "  unowned_planet_payloads = {}",
        data.current_known_unowned_planet_payload_errors()
            .is_empty()
    );
    println!(
        "  empty_auxiliary_state = {}",
        data.current_known_empty_auxiliary_state_errors().is_empty()
    );
    println!(
        "  setup_baseline = {}",
        data.current_known_setup_baseline_errors().is_empty()
    );
    println!(
        "  conquest_baseline = {}",
        data.current_known_conquest_baseline_errors().is_empty()
    );
    Ok(())
}

pub(crate) fn sync_initialized_fleet_baseline(
    dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = with_runtime_game_mut_and_export(dir, |data| {
        data.sync_current_known_initialized_fleet_baseline();
        Ok(data.clone())
    })?;

    println!("Initialized fleet baseline synchronized");
    println!(
        "  initialized_fleet_blocks = {}",
        data.looks_like_initialized_fleet_blocks_current_known()
    );
    println!(
        "  initialized_fleet_payloads = {}",
        data.current_known_initialized_fleet_payload_errors()
            .is_empty()
    );
    println!(
        "  initialized_fleet_missions = {}",
        data.current_known_initialized_fleet_mission_errors()
            .is_empty()
    );
    println!(
        "  initialized_homeworld_alignment = {}",
        data.current_known_initialized_homeworld_alignment_errors()
            .is_empty()
    );
    Ok(())
}

pub(crate) fn sync_initialized_planet_payloads(
    dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = with_runtime_game_mut_and_export(dir, |data| {
        data.sync_current_known_initialized_planet_payloads();
        Ok(data.clone())
    })?;

    println!("Initialized planet payloads synchronized");
    println!(
        "  initialized_planet_ownership = {}",
        data.current_known_initialized_planet_ownership_errors()
            .is_empty()
    );
    println!(
        "  homeworld_seed_payloads = {}",
        data.current_known_homeworld_seed_payload_errors()
            .is_empty()
    );
    println!(
        "  unowned_planet_payloads = {}",
        data.current_known_unowned_planet_payload_errors()
            .is_empty()
    );
    Ok(())
}

pub(crate) fn sync_current_known_baseline(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let data = with_runtime_game_mut_and_export_core(dir, |data| {
        data.sync_current_known_initialized_post_maint_baseline();
        Ok(data.clone())
    })?;

    println!("Current-known baseline synchronized");
    println!(
        "  initialized_fleet_blocks = {}",
        data.looks_like_initialized_fleet_blocks_current_known()
    );
    println!(
        "  initialized_fleet_payloads = {}",
        data.current_known_initialized_fleet_payload_errors()
            .is_empty()
    );
    println!(
        "  initialized_fleet_missions = {}",
        data.current_known_initialized_fleet_mission_errors()
            .is_empty()
    );
    println!(
        "  initialized_homeworld_alignment = {}",
        data.current_known_initialized_homeworld_alignment_errors()
            .is_empty()
    );
    println!(
        "  initialized_planet_ownership = {}",
        data.current_known_initialized_planet_ownership_errors()
            .is_empty()
    );
    println!(
        "  homeworld_seed_payloads = {}",
        data.current_known_homeworld_seed_payload_errors()
            .is_empty()
    );
    println!(
        "  unowned_planet_payloads = {}",
        data.current_known_unowned_planet_payload_errors()
            .is_empty()
    );
    println!(
        "  empty_auxiliary_state = {}",
        data.current_known_empty_auxiliary_state_errors().is_empty()
    );
    println!(
        "  setup_baseline = {}",
        data.current_known_setup_baseline_errors().is_empty()
    );
    println!(
        "  conquest_baseline = {}",
        data.current_known_conquest_baseline_errors().is_empty()
    );
    Ok(())
}

pub(crate) fn sync_canonical_current_known_baseline(
    dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let baseline_dir = post_maint_fixture_dir();
    copy_current_known_core_files(&baseline_dir, dir)?;
    let data = load_runtime_game_data(dir)?;

    println!("Canonical current-known baseline synchronized");
    println!("  dir = {}", dir.display());
    println!(
        "  exact_canonical_current_known_baseline = {}",
        core_file_exact_match_errors(
            &data,
            &load_classic_core_files(&baseline_dir)?,
            "canonical current-known post-maint baseline",
        )
        .is_empty()
    );
    Ok(())
}

pub(crate) fn init_current_known_baseline(
    source: &Path,
    target: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    copy_top_level_files(source, target)?;
    seed_missing_current_known_core_files(target)?;
    // Keep this path as an explicit classic-directory compatibility initializer.
    // The canonical transition reports compare the direct projected file shape
    // against the preserved post-maint baseline, and routing this through the
    // runtime bridge changes those drift reports.
    let mut data = load_classic_core_files(target)?;
    data.sync_current_known_initialized_post_maint_baseline();
    save_classic_core_files(target, &data)?;

    println!(
        "Current-known baseline directory initialized at {}",
        target.display()
    );
    println!("  source snapshot: {}", display_repo_path(source));
    println!(
        "  initialized_fleet_blocks = {}",
        data.looks_like_initialized_fleet_blocks_current_known()
    );
    println!(
        "  initialized_fleet_payloads = {}",
        data.current_known_initialized_fleet_payload_errors()
            .is_empty()
    );
    println!(
        "  initialized_fleet_missions = {}",
        data.current_known_initialized_fleet_mission_errors()
            .is_empty()
    );
    println!(
        "  initialized_planet_ownership = {}",
        data.current_known_initialized_planet_ownership_errors()
            .is_empty()
    );
    println!(
        "  homeworld_seed_payloads = {}",
        data.current_known_homeworld_seed_payload_errors()
            .is_empty()
    );
    println!(
        "  unowned_planet_payloads = {}",
        data.current_known_unowned_planet_payload_errors()
            .is_empty()
    );
    println!(
        "  setup_baseline = {}",
        data.current_known_setup_baseline_errors().is_empty()
    );
    println!(
        "  conquest_baseline = {}",
        data.current_known_conquest_baseline_errors().is_empty()
    );

    Ok(())
}

pub(crate) fn init_canonical_current_known_baseline(
    source: &Path,
    target: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    copy_top_level_files(source, target)?;
    copy_current_known_core_files(&post_maint_fixture_dir(), target)?;

    // Generate DATABASE.DAT from the copied PLANETS.DAT + CONQUEST.DAT
    generate_database_dat(target)?;

    // Ensure auxiliary files exist
    ensure_classic_auxiliary_files(target)?;

    let baseline = load_classic_core_files(&post_maint_fixture_dir())?;
    let data = load_classic_core_files(target)?;

    println!(
        "Canonical current-known baseline directory initialized at {}",
        target.display()
    );
    println!("  source snapshot: {}", display_repo_path(source));
    println!(
        "  exact_canonical_current_known_baseline = {}",
        core_file_exact_match_errors(
            &data,
            &baseline,
            "canonical current-known post-maint baseline",
        )
        .is_empty()
    );

    Ok(())
}

pub(crate) fn set_player_tax_rate(
    dir: &Path,
    player_record_index_1_based: usize,
    tax_rate: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    with_runtime_game_mut_and_export(dir, |data| {
        data.set_player_tax_rate(player_record_index_1_based, tax_rate)?;
        Ok(())
    })?;

    println!(
        "Player {} tax rate set to {}%",
        player_record_index_1_based, tax_rate
    );
    Ok(())
}

fn seed_missing_current_known_core_files(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    for name in ["BASES.DAT", "IPBM.DAT"] {
        let path = dir.join(name);
        if !path.exists() {
            fs::write(path, [])?;
        }
    }
    Ok(())
}
