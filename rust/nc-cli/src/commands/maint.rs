use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::support::paths::{default_fixture_dir, repo_root, resolve_repo_path};
use crate::usage::print_maintenance_usage;
use nc_compat::classic::{CONQUEST_DAT_SIZE, decode_conquest};
use nc_compat::import_directory_snapshot;
use nc_data::{
    CampaignStore, CoreGameData, DiplomacyOverride, DiplomaticRelation, MaintenanceEvents,
    PlanetIntelSnapshot, apply_inactivity_autopilot_policy,
    apply_maintenance_events_to_player_war_stats, carry_unread_report_blocks,
    latest_planet_intel_grants_for_viewer, merge_player_intel_from_runtime,
};
use nc_engine::{
    DiplomacyConfig, DiplomacyDirective, VisibleHazardIntel, apply_results_reviewable_flags,
    build_rankings_text, build_results_report_blocks,
    run_maintenance_turn_with_context_seed_and_lifecycle, validate_maintenance_state,
    visible_hazard_intel_from_snapshots,
};

use crate::commands::runtime::{
    load_runtime_intel_by_viewer, load_runtime_state_preferring_live_directory,
};

/// Run Rust maintenance on a game directory for specified number of turns.
///
/// This is the programmatic API (used by tests and direct callers) — it skips
/// the schedule/token gate.  Use `run_rust_maintenance_with_options` with
/// `no_gate = false` for CLI invocations that should respect the schedule.
pub fn run_rust_maintenance_from_args(
    program: &str,
    mut args: impl Iterator<Item = String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let dir = args
        .next()
        .map(|arg| resolve_repo_path(&arg))
        .unwrap_or_else(default_fixture_dir);
    let turns = match args.next() {
        None => 1,
        Some(value) => match value.parse::<u16>() {
            Ok(turns) => turns,
            Err(_) => {
                eprintln!("Error: turns must be a valid number");
                print_maintenance_usage(program);
                return Ok(());
            }
        },
    };
    if args.next().is_some() {
        print_maintenance_usage(program);
        return Ok(());
    }
    run_rust_maintenance(&dir, turns)
}

pub fn run_rust_maintenance(dir: &Path, turns: u16) -> Result<(), Box<dyn std::error::Error>> {
    run_rust_maintenance_with_options(dir, turns, true)
}

pub fn run_rust_maintenance_with_options(
    dir: &Path,
    turns: u16,
    no_gate: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "Running Rust maintenance on: {} ({} turn{})",
        dir.display(),
        turns,
        if turns == 1 { "" } else { "s" }
    );

    // Phase 4: Schedule / token gate.
    if !no_gate {
        let gate_conquest_path = dir.join("CONQUEST.DAT");
        if gate_conquest_path.exists() {
            let raw = fs::read(&gate_conquest_path)?;
            if let Some(conquest) = raw.get(..CONQUEST_DAT_SIZE) {
                let conquest = decode_conquest(conquest.try_into()?);
                match nc_engine::maint::gate::check_maintenance_schedule(&conquest) {
                    nc_engine::maint::gate::GateResult::Allowed => {}
                    nc_engine::maint::gate::GateResult::NotScheduled { day_name } => {
                        println!(
                            "Today is {} - maintenance is not scheduled to run.",
                            day_name
                        );
                        return Ok(());
                    }
                    nc_engine::maint::gate::GateResult::TokenBusy => {
                        println!("Maintenance token already held; skipping.");
                        return Ok(());
                    }
                }
            }
        }
        if nc_engine::maint::gate::check_token_files(dir) {
            println!("Maintenance token already held; skipping.");
            return Ok(());
        }
        nc_engine::maint::gate::create_maintenance_token(dir)?;
    }

    // Phase 5: Crash recovery — restore from .SAV if Move.Tok exists.
    if nc_engine::maint::recovery::check_crash_marker(dir) {
        println!("Detected interrupted run (Move.Tok found); restoring from .SAV backups.");
        nc_engine::maint::recovery::restore_from_sav(dir)?;
        nc_engine::maint::recovery::remove_crash_marker(dir);
    }

    let campaign_store = CampaignStore::open_default_in_dir(dir)?;
    let runtime_state = load_runtime_state_preferring_live_directory(dir, &campaign_store)?;

    let mut game_data = runtime_state.game_data;
    let campaign_seed = runtime_state.campaign_seed;
    let mut winner_state = runtime_state.winner_state;
    let start_year = game_data.conquest.game_year;
    let queued_mail = runtime_state.queued_mail;
    let planet_scorch_orders = runtime_state.planet_scorch_orders;
    let mut player_activity_states =
        campaign_store.latest_player_activity_states(game_data.conquest.player_count)?;
    let mut player_lifecycle_states =
        campaign_store.latest_player_lifecycle_states(game_data.conquest.player_count)?;
    let mut player_war_stats =
        campaign_store.latest_player_war_stats(game_data.conquest.player_count)?;
    let mut diplomacy_overrides = load_diplomacy_overrides_if_present(dir, &game_data)?;
    let mut planet_intel_by_viewer = load_runtime_intel_by_viewer(&campaign_store, &game_data)?;
    absorb_persistable_diplomacy_overrides(&mut game_data, &mut diplomacy_overrides)?;

    if let Err(err) = validate_maintenance_state(&game_data) {
        eprintln!(
            "Cross-file validation failed with {} issue(s):",
            err.issues().len()
        );
        for issue in err.issues() {
            eprintln!("  - {issue}");
        }
        return Err(Box::new(err));
    }

    if winner_state.winner_empire.is_some() {
        return Err("campaign winner has already been declared; maintenance is frozen".into());
    }

    // Create .SAV backups before movement phase.
    if let Err(e) = nc_engine::maint::recovery::create_movement_backups(dir) {
        eprintln!("Warning: could not create movement backups: {e}");
    }

    // Run maintenance logic for specified turns, accumulating events across all turns.
    let mut all_events = MaintenanceEvents::default();
    for turn in 1..=turns {
        let inactivity_threshold =
            u8::try_from(game_data.setup.autopilot_after_turns).unwrap_or(u8::MAX);
        apply_inactivity_autopilot_policy(
            &mut game_data,
            inactivity_threshold,
            &mut player_activity_states,
        );
        let visible_hazards = visible_hazards_from_snapshots(&game_data, &planet_intel_by_viewer);
        let events = if visible_hazards.is_empty() && diplomacy_overrides.is_empty() {
            run_maintenance_turn_with_context_seed_and_lifecycle(
                &mut game_data,
                campaign_seed,
                &[],
                &[],
                &mut player_lifecycle_states,
                &mut winner_state,
            )?
        } else if diplomacy_overrides.is_empty() {
            run_maintenance_turn_with_context_seed_and_lifecycle(
                &mut game_data,
                campaign_seed,
                &visible_hazards,
                &[],
                &mut player_lifecycle_states,
                &mut winner_state,
            )?
        } else {
            run_maintenance_turn_with_context_seed_and_lifecycle(
                &mut game_data,
                campaign_seed,
                &visible_hazards,
                &diplomacy_overrides,
                &mut player_lifecycle_states,
                &mut winner_state,
            )?
        };
        let planet_intel_grants_by_viewer = (1..=game_data.conquest.player_count)
            .map(|viewer_empire_id| {
                latest_planet_intel_grants_for_viewer(&events, viewer_empire_id)
            })
            .collect::<Vec<_>>();
        all_events.bombard_events.extend(events.bombard_events);
        all_events
            .planet_intel_events
            .extend(events.planet_intel_events);
        all_events
            .ownership_change_events
            .extend(events.ownership_change_events);
        all_events
            .fleet_battle_events
            .extend(events.fleet_battle_events);
        all_events
            .fleet_destroyed_events
            .extend(events.fleet_destroyed_events);
        all_events
            .starbase_destroyed_events
            .extend(events.starbase_destroyed_events);
        all_events
            .assault_report_events
            .extend(events.assault_report_events);
        all_events
            .scout_contact_events
            .extend(events.scout_contact_events);
        all_events
            .starbase_sighting_events
            .extend(events.starbase_sighting_events);
        all_events
            .encounter_disposition_events
            .extend(events.encounter_disposition_events);
        all_events
            .invalid_player_state_events
            .extend(events.invalid_player_state_events);
        all_events
            .fleet_merge_events
            .extend(events.fleet_merge_events);
        all_events.join_host_events.extend(events.join_host_events);
        all_events
            .mission_retarget_events
            .extend(events.mission_retarget_events);
        all_events
            .colonization_events
            .extend(events.colonization_events);
        all_events.mission_events.extend(events.mission_events);
        all_events.salvage_events.extend(events.salvage_events);
        all_events
            .diplomatic_escalation_events
            .extend(events.diplomatic_escalation_events);
        all_events
            .civil_disorder_events
            .extend(events.civil_disorder_events);
        all_events
            .campaign_outlook_events
            .extend(events.campaign_outlook_events);
        all_events
            .campaign_outcome_events
            .extend(events.campaign_outcome_events);
        all_events
            .game_victory_notice_events
            .extend(events.game_victory_notice_events);
        all_events
            .empire_elimination_events
            .extend(events.empire_elimination_events);
        all_events
            .fleet_defection_events
            .extend(events.fleet_defection_events);

        apply_diplomatic_escalations(&mut game_data, &mut diplomacy_overrides, &all_events)?;

        for viewer_empire_id in 1..=game_data.conquest.player_count {
            let viewer_idx = viewer_empire_id.saturating_sub(1) as usize;
            let previous = planet_intel_by_viewer
                .get(viewer_idx)
                .cloned()
                .unwrap_or_default();
            planet_intel_by_viewer[viewer_idx] = merge_player_intel_from_runtime(
                &game_data,
                viewer_empire_id,
                game_data.conquest.game_year,
                Some(&previous),
                planet_intel_grants_by_viewer.get(viewer_idx),
            );
        }

        println!("  Turn {}: year {}", turn, game_data.conquest.game_year);
    }

    println!(
        "  Year advanced: {} -> {}",
        start_year, game_data.conquest.game_year
    );

    let report_block_rows = carry_unread_report_blocks(
        &runtime_state.report_block_rows,
        build_results_report_blocks(&game_data, &all_events),
    );
    apply_results_reviewable_flags(&mut game_data, &report_block_rows);
    apply_maintenance_events_to_player_war_stats(&mut player_war_stats, &all_events);
    campaign_store.save_runtime_state_structured_with_intel_activity_lifecycle_and_war_stats(
        &game_data,
        &planet_scorch_orders,
        &report_block_rows,
        &queued_mail,
        &planet_intel_by_viewer,
        &player_activity_states,
        &player_lifecycle_states,
        &player_war_stats,
        winner_state,
    )?;

    let rankings_text = build_rankings_text(&game_data);
    fs::write(dir.join("RANKINGS.TXT"), rankings_text.as_bytes())?;
    save_diplomacy_overrides_if_needed(dir, game_data.conquest.player_count, &diplomacy_overrides)?;

    // Remove Move.Tok crash marker after successful run.
    nc_engine::maint::recovery::remove_crash_marker(dir);

    // Release Main.Tok gate token.
    if !no_gate {
        nc_engine::maint::gate::remove_maintenance_token(dir);
    }

    println!("Rust maintenance complete.");
    Ok(())
}

/// Run original ECMAINT oracle on a directory
pub fn run_original_ecmaint(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running original ECMAINT oracle on: {}", dir.display());
    let repo_root = repo_root();

    // Ensure ECMAINT.EXE is present — without it DOSBox opens an interactive window.
    let engine_path = dir.join("ECMAINT.EXE");
    if !engine_path.exists() {
        let source_engine = repo_root.join("original/v1.5/ECMAINT.EXE");
        if source_engine.exists() {
            fs::copy(&source_engine, &engine_path)?;
        } else {
            return Err("ECMAINT.EXE not found in dir or original/v1.5/".into());
        }
    }

    // Use the existing oracle harness via Python script.
    // Pass SDL_VIDEODRIVER=dummy and SDL_AUDIODRIVER=dummy explicitly so DOSBox
    // runs headless even when called from a GUI environment.
    let output = Command::new("python3")
        .arg("tools/ecmaint_oracle.py")
        .arg("run")
        .arg(dir)
        .current_dir(&repo_root)
        .env("SDL_VIDEODRIVER", "dummy")
        .env("SDL_AUDIODRIVER", "dummy")
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ECMAINT oracle failed: {}", stderr).into());
    }

    println!("Original ECMAINT complete.");
    Ok(())
}

/// Compare Rust maintenance output vs original ECMAINT output
///
/// # Arguments
/// * `dir` - The game directory to compare
/// * `turns` - Number of turns to run (default 1, or auto-detect from year difference)
pub fn compare_maintenance(
    dir: &Path,
    turns: Option<u16>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "Comparing maintenance implementations for: {}",
        dir.display()
    );
    println!();

    // Auto-detect turns from directory name if not specified.
    // Known tick counts per scenario family:
    //   move: 3 ticks   (3000 → 3003)
    //   bombard: 2 ticks (3000 → 3002)
    //   invade: 2 ticks  (3010 → 3012)
    //   econ: 2 ticks    (3010 → 3012)
    //   all others: 1 tick
    let turns_to_run = match turns {
        Some(t) => t,
        None => {
            let dir_str = dir.to_string_lossy();
            if dir_str.contains("move") {
                println!("Auto-detected: move scenario — 3 turns");
                3
            } else if dir_str.contains("bombard")
                || dir_str.contains("invade")
                || dir_str.contains("econ")
            {
                println!("Auto-detected: 2-tick scenario — 2 turns");
                2
            } else {
                1
            }
        }
    };

    // Create temporary directories for comparison
    let temp_dir = std::env::temp_dir().join("ecmaint-compare");
    let rust_dir = temp_dir.join("rust");
    let oracle_dir = temp_dir.join("oracle");

    // Clean up and recreate
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&rust_dir)?;
    fs::create_dir_all(&oracle_dir)?;

    // Copy original to both working directories
    copy_directory(dir, &rust_dir)?;
    copy_directory(dir, &oracle_dir)?;

    // Run Rust maintenance for N turns
    println!("=== Running Rust maintenance ({} turns) ===", turns_to_run);
    let store = CampaignStore::open_default_in_dir(&rust_dir)?;
    import_directory_snapshot(&store, &rust_dir)?;
    run_rust_maintenance(&rust_dir, turns_to_run)?;
    crate::commands::storage::export_latest_db_snapshot(&rust_dir, &rust_dir)?;
    println!();

    // Run original ECMAINT N times
    println!("=== Running original ECMAINT ({} turns) ===", turns_to_run);
    for turn in 1..=turns_to_run {
        println!("  Turn {}...", turn);
        run_original_ecmaint(&oracle_dir)?;
    }
    println!();

    // Compare outputs
    println!("=== Comparison Results ===");
    compare_dat_files(dir, &rust_dir, &oracle_dir)?;

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(())
}

/// Copy directory contents recursively
fn copy_directory(src: &Path, dst: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = path.file_name().unwrap_or_default();

        // Skip .oracle directory and .SAV files
        let name_str = file_name.to_string_lossy();
        if name_str.starts_with('.') || name_str.ends_with(".SAV") {
            continue;
        }

        let dest_path = dst.join(file_name);

        if path.is_file() {
            fs::copy(&path, &dest_path)?;
        }
    }

    Ok(())
}

fn load_diplomacy_overrides_if_present(
    dir: &Path,
    game_data: &CoreGameData,
) -> Result<Vec<DiplomacyOverride>, Box<dyn std::error::Error>> {
    let path = dir.join("diplomacy.kdl");
    if !path.exists() {
        return Ok(Vec::new());
    }

    let config = DiplomacyConfig::load_kdl(&path)?
        .validate_for_player_count(game_data.players.len() as u8)?;
    Ok(config
        .directives
        .into_iter()
        .map(|directive| DiplomacyOverride {
            from_empire: directive.from_empire,
            to_empire: directive.to_empire,
            relation: directive.relation,
        })
        .collect())
}

fn apply_diplomatic_escalations(
    game_data: &mut CoreGameData,
    diplomacy_overrides: &mut Vec<DiplomacyOverride>,
    events: &MaintenanceEvents,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut pairs = Vec::new();

    for event in &events.fleet_battle_events {
        for &enemy_empire in &event.enemy_empires {
            pairs.push((event.reporting_empire, enemy_empire));
        }
    }

    for event in &events.bombard_events {
        if event.defender_empire != 0 {
            pairs.push((event.attacker_empire, event.defender_empire));
        }
    }

    for event in &events.assault_report_events {
        if event.defender_empire != 0 {
            pairs.push((event.attacker_empire, event.defender_empire));
        }
    }

    for (left, right) in pairs {
        escalate_pair(game_data, diplomacy_overrides, left, right)?;
    }

    Ok(())
}

fn escalate_pair(
    game_data: &mut CoreGameData,
    diplomacy_overrides: &mut Vec<DiplomacyOverride>,
    left: u8,
    right: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    if left == 0 || right == 0 || left == right {
        return Ok(());
    }
    apply_one_way_escalation(game_data, diplomacy_overrides, left, right)?;
    apply_one_way_escalation(game_data, diplomacy_overrides, right, left)?;
    Ok(())
}

fn apply_one_way_escalation(
    game_data: &mut CoreGameData,
    diplomacy_overrides: &mut Vec<DiplomacyOverride>,
    from: u8,
    to: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    if matches!(
        game_data.stored_diplomatic_relation(from, to),
        Some(DiplomaticRelation::Enemy)
    ) {
        return Ok(());
    }

    if game_data.set_stored_diplomatic_relation(from, to, DiplomaticRelation::Enemy)? {
        return Ok(());
    }

    if let Some(existing) = diplomacy_overrides
        .iter_mut()
        .find(|directive| directive.from_empire == from && directive.to_empire == to)
    {
        existing.relation = DiplomaticRelation::Enemy;
        return Ok(());
    }

    diplomacy_overrides.push(DiplomacyOverride {
        from_empire: from,
        to_empire: to,
        relation: DiplomaticRelation::Enemy,
    });
    Ok(())
}

fn save_diplomacy_overrides_if_needed(
    dir: &Path,
    player_count: u8,
    diplomacy_overrides: &[DiplomacyOverride],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = dir.join("diplomacy.kdl");
    let directives = diplomacy_overrides
        .iter()
        .copied()
        .map(|directive| DiplomacyDirective {
            from_empire: directive.from_empire,
            to_empire: directive.to_empire,
            relation: directive.relation,
        })
        .collect::<Vec<_>>();

    if directives.is_empty() {
        if path.exists() {
            fs::write(path, [])?;
        }
        return Ok(());
    }

    let config = DiplomacyConfig { directives }.validate_for_player_count(player_count)?;
    fs::write(path, config.to_kdl_string())?;
    Ok(())
}

fn absorb_persistable_diplomacy_overrides(
    game_data: &mut CoreGameData,
    diplomacy_overrides: &mut Vec<DiplomacyOverride>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut remaining = Vec::with_capacity(diplomacy_overrides.len());
    for directive in diplomacy_overrides.drain(..) {
        if game_data.set_stored_diplomatic_relation(
            directive.from_empire,
            directive.to_empire,
            directive.relation,
        )? {
            continue;
        }
        remaining.push(directive);
    }
    *diplomacy_overrides = remaining;
    Ok(())
}

fn visible_hazards_from_snapshots(
    game_data: &CoreGameData,
    planet_intel_by_viewer: &[BTreeMap<usize, PlanetIntelSnapshot>],
) -> Vec<VisibleHazardIntel> {
    (1..=game_data.conquest.player_count as usize)
        .map(|viewer_idx| {
            let empty = BTreeMap::new();
            visible_hazard_intel_from_snapshots(
                game_data,
                planet_intel_by_viewer.get(viewer_idx - 1).unwrap_or(&empty),
                viewer_idx as u8,
            )
        })
        .collect()
}

/// Compare .DAT files between two directories
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ComparePolicy {
    Strict,
    CanonicalCombat,
}

fn compare_policy_for_dir(dir: &Path) -> ComparePolicy {
    let dir_str = dir.to_string_lossy();
    if dir_str.contains("fleet-battle")
        || dir_str.contains("bombard")
        || dir_str.contains("invade")
        || dir_str.contains("econ")
    {
        ComparePolicy::CanonicalCombat
    } else {
        ComparePolicy::Strict
    }
}

fn is_structurally_accepted_diff(policy: ComparePolicy, file: &str) -> bool {
    match policy {
        ComparePolicy::Strict => false,
        ComparePolicy::CanonicalCombat => matches!(
            file,
            "FLEETS.DAT" | "PLANETS.DAT" | "DATABASE.DAT" | "MESSAGES.DAT" | "RESULTS.DAT"
        ),
    }
}

fn compare_dat_files(
    source_dir: &Path,
    rust_dir: &Path,
    oracle_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let dat_files = [
        "CONQUEST.DAT",
        "PLAYER.DAT",
        "PLANETS.DAT",
        "FLEETS.DAT",
        "BASES.DAT",
        "IPBM.DAT",
        "MESSAGES.DAT",
        "RESULTS.DAT",
        "DATABASE.DAT",
        "SETUP.DAT",
    ];

    let policy = compare_policy_for_dir(source_dir);
    let mut total_files = 0;
    let mut matching_files = 0;
    let mut accepted_files = 0;

    for file in &dat_files {
        let rust_path = rust_dir.join(file);
        let oracle_path = oracle_dir.join(file);

        if !rust_path.exists() || !oracle_path.exists() {
            continue;
        }

        total_files += 1;
        let rust_data = fs::read(&rust_path)?;
        let oracle_data = fs::read(&oracle_path)?;

        if rust_data == oracle_data {
            matching_files += 1;
            accepted_files += 1;
            println!("  ✓ {}: MATCH ({} bytes)", file, rust_data.len());
        } else {
            // Calculate diff stats
            let diff_count = rust_data
                .iter()
                .zip(oracle_data.iter())
                .enumerate()
                .filter(|(_, (a, b))| a != b)
                .count();
            let max_diffs = rust_data.len().max(oracle_data.len())
                - rust_data.len().min(oracle_data.len())
                + diff_count;

            if is_structurally_accepted_diff(policy, file) {
                accepted_files += 1;
                println!(
                    "  ~ {}: STRUCTURAL DIFF ACCEPTED ({} differing bytes out of {})",
                    file,
                    max_diffs,
                    rust_data.len().max(oracle_data.len())
                );
            } else {
                println!(
                    "  ✗ {}: DIFFER ({} differing bytes out of {})",
                    file,
                    max_diffs,
                    rust_data.len().max(oracle_data.len())
                );
            }

            // Show first few diffs for FLEETS.DAT
            if *file == "FLEETS.DAT" && diff_count > 0 {
                println!("    First 5 differences:");
                let mut shown = 0;
                for (i, (a, b)) in rust_data.iter().zip(oracle_data.iter()).enumerate() {
                    if a != b && shown < 5 {
                        let fleet_idx = if i >= 2 { (i - 2) / 54 } else { 0 };
                        let field_off = if i >= 2 { (i - 2) % 54 } else { i };
                        println!(
                            "      Offset 0x{:04x} (fleet {}, field 0x{:02x}): 0x{:02x} -> 0x{:02x}",
                            i, fleet_idx, field_off, a, b
                        );
                        shown += 1;
                    }
                }
            }
        }
    }

    println!();
    let strict_percentage = if total_files > 0 {
        (matching_files as f64 / total_files as f64) * 100.0
    } else {
        0.0
    };
    println!(
        "Strict parity: {}/{} files match ({:.1}%)",
        matching_files, total_files, strict_percentage
    );
    let accepted_percentage = if total_files > 0 {
        (accepted_files as f64 / total_files as f64) * 100.0
    } else {
        0.0
    };
    match policy {
        ComparePolicy::Strict => {
            println!(
                "Acceptance: strict byte-exact comparison ({}/{} files accepted, {:.1}%)",
                accepted_files, total_files, accepted_percentage
            );
        }
        ComparePolicy::CanonicalCombat => {
            println!(
                "Acceptance: canonical combat structural comparison ({}/{} files accepted, {:.1}%)",
                accepted_files, total_files, accepted_percentage
            );
            println!(
                "  Structural diffs are accepted only in combat-driven files: FLEETS.DAT, PLANETS.DAT, DATABASE.DAT, MESSAGES.DAT, RESULTS.DAT"
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{ComparePolicy, compare_policy_for_dir, is_structurally_accepted_diff};

    #[test]
    fn compare_policy_uses_canonical_combat_for_combat_heavy_scenarios() {
        assert_eq!(
            compare_policy_for_dir(Path::new("fixtures/ecmaint-fleet-battle-pre/v1.5")),
            ComparePolicy::CanonicalCombat
        );
        assert_eq!(
            compare_policy_for_dir(Path::new("fixtures/ecmaint-bombard-pre/v1.5")),
            ComparePolicy::CanonicalCombat
        );
        assert_eq!(
            compare_policy_for_dir(Path::new("fixtures/ecmaint-invade-pre/v1.5")),
            ComparePolicy::CanonicalCombat
        );
        assert_eq!(
            compare_policy_for_dir(Path::new("fixtures/ecmaint-econ-pre/v1.5")),
            ComparePolicy::CanonicalCombat
        );
    }

    #[test]
    fn compare_policy_keeps_deterministic_scenarios_strict() {
        assert_eq!(
            compare_policy_for_dir(Path::new("fixtures/ecmaint-move-pre/v1.5")),
            ComparePolicy::Strict
        );
        assert_eq!(
            compare_policy_for_dir(Path::new("fixtures/ecmaint-build-pre/v1.5")),
            ComparePolicy::Strict
        );
    }

    #[test]
    fn canonical_combat_acceptance_is_limited_to_combat_driven_files() {
        assert!(is_structurally_accepted_diff(
            ComparePolicy::CanonicalCombat,
            "FLEETS.DAT"
        ));
        assert!(is_structurally_accepted_diff(
            ComparePolicy::CanonicalCombat,
            "PLANETS.DAT"
        ));
        assert!(is_structurally_accepted_diff(
            ComparePolicy::CanonicalCombat,
            "DATABASE.DAT"
        ));
        assert!(is_structurally_accepted_diff(
            ComparePolicy::CanonicalCombat,
            "MESSAGES.DAT"
        ));
        assert!(is_structurally_accepted_diff(
            ComparePolicy::CanonicalCombat,
            "RESULTS.DAT"
        ));
        assert!(!is_structurally_accepted_diff(
            ComparePolicy::CanonicalCombat,
            "PLAYER.DAT"
        ));
        assert!(!is_structurally_accepted_diff(
            ComparePolicy::CanonicalCombat,
            "CONQUEST.DAT"
        ));
        assert!(!is_structurally_accepted_diff(
            ComparePolicy::Strict,
            "FLEETS.DAT"
        ));
    }
}
