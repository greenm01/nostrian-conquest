use nc_data::hosted::{
    GameTier, HostedStore, RecruitingMode, RosterStore, SeatStatus, get_seat_by_number,
    record_player_abandoned, set_claimed_year,
};
use nc_data::{
    CoreGameData, DEFAULT_INACTIVITY_AUTOPILOT_AFTER_TURNS, PlanetIntelSnapshot,
    apply_inactivity_autopilot_policy, merge_player_intel_from_runtime,
    reset_player_slot_to_baseline,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

const SANDBOX_MAX_OCCUPANT_TURNS: u16 = 10;

pub fn run(args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    if args.is_empty() || args.iter().any(|arg| matches!(*arg, "--help" | "-h")) {
        print_usage();
        return Ok(());
    }

    let mut game_dir = None;
    let mut turns = 1;

    let mut i = 0;
    while i < args.len() {
        match args[i] {
            _ if args[i].starts_with("--") => {
                return Err(format!("unknown flag: {}", args[i]).into());
            }
            _ => {
                if game_dir.is_none() {
                    game_dir = Some(PathBuf::from(args[i]));
                } else if let Ok(n) = args[i].parse::<u32>() {
                    turns = n;
                } else {
                    return Err(format!("unexpected argument: {}", args[i]).into());
                }
                i += 1;
            }
        }
    }

    let game_dir = game_dir.ok_or("missing game directory argument")?;
    let db_path = game_dir.join("hosted.db");

    if !db_path.exists() {
        return Err(format!("game not found at {}", game_dir.display()).into());
    }

    let store = HostedStore::open(&db_path)?;
    let game_id = game_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("game")
        .to_string();

    run_maintenance(&store, &game_dir, &game_id, turns)
}

fn run_maintenance(
    store: &HostedStore,
    game_dir: &PathBuf,
    game_id: &str,
    turns: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let settings = nc_data::hosted::get_settings(store.connection(), game_id)?;
    let is_sandbox = settings.game_tier == GameTier::Sandbox;

    println!("Running maintenance for game {} ({} turns)", game_id, turns);
    println!("  Tier:                {}", settings.game_tier.as_str());
    println!("  Maintenance enabled: {}", settings.maintenance_enabled);
    println!(
        "  Interval: {} minutes",
        settings.maintenance_interval_minutes
    );

    let pending_turns = nc_data::hosted::list_pending_turns(store.connection(), game_id, 0)?;
    println!("  Pending turns: {}", pending_turns.len());

    let campaign_store = crate::support::runtime::open_existing_campaign_store(game_dir)?;
    let mut game_data = campaign_store.load_latest_runtime_game_data()?;
    let mut player_activity_states =
        campaign_store.latest_player_activity_states(game_data.conquest.player_count())?;

    let current_turn = (game_data.conquest.game_year() - 3000) as u32;

    for turn_num in 0..turns {
        let turn: u32 = current_turn + turn_num + 1;
        println!("  Processing turn {}...", turn);

        let turn_submissions: Vec<_> = pending_turns
            .iter()
            .filter(|t| t.turn == turn as u32)
            .collect();

        if turn_submissions.is_empty() {
            println!("    No orders submitted for turn {}", turn);
        } else {
            println!("    Applying {} turn submissions", turn_submissions.len());
            for submission in &turn_submissions {
                let short_key = if submission.player_pubkey.len() >= 8 {
                    &submission.player_pubkey[..8]
                } else {
                    &submission.player_pubkey
                };
                println!(
                    "      - Player {}: {} commands",
                    short_key,
                    submission.commands.len()
                );
            }
        }

        // Apply inactivity autopilot for Sandbox games before running maintenance.
        if is_sandbox {
            apply_inactivity_autopilot_policy(
                &mut game_data,
                DEFAULT_INACTIVITY_AUTOPILOT_AFTER_TURNS,
                &mut player_activity_states,
            );
        }

        match nc_engine::run_maintenance_turn(&mut game_data) {
            Ok(_events) => {
                println!("    Turn {} complete", turn);
            }
            Err(e) => {
                println!("    ERROR processing turn {}: {}", turn, e);
            }
        }

        if is_sandbox {
            recycle_sandbox_seats(
                store,
                game_dir,
                game_id,
                &mut game_data,
                &mut player_activity_states,
            )?;
        }
    }

    // Save the snapshot with updated activity states and fresh intel.
    let player_count = game_data.conquest.player_count();
    let year = game_data.conquest.game_year();
    let intel_by_viewer: Vec<BTreeMap<usize, PlanetIntelSnapshot>> = (1..=player_count)
        .map(|viewer_id| merge_player_intel_from_runtime(&game_data, viewer_id, year, None, None))
        .collect();
    campaign_store.save_runtime_state_structured_with_intel_and_activity(
        &game_data,
        &BTreeSet::new(),
        &[],
        &[],
        &intel_by_viewer,
        &player_activity_states,
    )?;
    println!("Updated campaign store snapshot.");

    println!("Maintenance complete for {} turns", turns);

    Ok(())
}

fn recycle_sandbox_seats(
    store: &HostedStore,
    game_dir: &PathBuf,
    game_id: &str,
    game_data: &mut CoreGameData,
    player_activity_states: &mut [nc_data::PlayerActivityState],
) -> Result<(), Box<dyn std::error::Error>> {
    let current_year = game_data.conquest.game_year();
    let mut recycled_any = false;

    for state in player_activity_states.iter_mut() {
        let seat_number = state.player_record_index_1_based as u32;
        let Some(seat) = get_seat_by_number(store.connection(), game_id, seat_number)? else {
            continue;
        };
        if seat.status != SeatStatus::Claimed {
            continue;
        }

        let claimed_year = match seat.claimed_year {
            Some(year) => year,
            None => {
                set_claimed_year(store.connection(), game_id, seat_number, current_year)?;
                continue;
            }
        };
        let exceeded_tenure =
            current_year.saturating_sub(claimed_year) >= SANDBOX_MAX_OCCUPANT_TURNS;
        let eject_for_mia = state.inactivity_autopilot_pending_clear;
        if !eject_for_mia && !exceeded_tenure {
            continue;
        }

        if eject_for_mia {
            println!(
                "  Ejecting MIA player from seat {} (3-turn inactivity)",
                seat_number
            );
        } else {
            println!(
                "  Recycling sandbox seat {} after {} turns",
                seat_number, SANDBOX_MAX_OCCUPANT_TURNS
            );
        }

        let player_npub = seat.player_pubkey.clone();
        if let Err(e) = reset_player_slot_to_baseline(game_data, state.player_record_index_1_based)
        {
            println!("    ERROR resetting player slot {}: {}", seat_number, e);
            continue;
        }

        if let Err(e) = nc_data::hosted::reset_seat(store.connection(), game_id, seat_number) {
            println!("    ERROR resetting hosted seat {}: {}", seat_number, e);
            continue;
        }

        if eject_for_mia {
            if let Some(ref npub) = player_npub {
                let roster_db = game_dir.parent().map(|p| p.join("roster.db"));
                if let Some(roster_db) = roster_db {
                    if let Ok(rs) = RosterStore::open(&roster_db) {
                        let _ =
                            record_player_abandoned(rs.connection(), npub, game_id, seat_number);
                    }
                }
            }
            state.inactivity_autopilot_pending_clear = false;
        }

        recycled_any = true;
        println!("    Seat {} reset and opened for replacement.", seat_number);
    }

    if recycled_any {
        let mut settings = nc_data::hosted::get_settings(store.connection(), game_id)?;
        if settings.recruiting == RecruitingMode::None {
            settings.recruiting = RecruitingMode::ReplacementPlayers;
            nc_data::hosted::update_settings(store.connection(), game_id, &settings)?;
        }
        nc_data::hosted::mark_catalog_dirty(store.connection(), game_id)?;
    }

    Ok(())
}

fn print_usage() {
    println!("Usage: nc-host maint <dir> [turns]");
    println!();
    println!("Arguments:");
    println!("  <dir>     Game directory path");
    println!("  [turns]   Number of turns to process (default: 1)");
}
