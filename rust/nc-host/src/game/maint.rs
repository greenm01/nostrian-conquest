//! Hosted maintenance: the campaign pipeline `nc-sysop maint` runs, with
//! the hosted steps around it. Before each year the pending turn
//! submissions go in; after each year a sandbox recycles its MIA and
//! long-held seats; after the run every claimed seat is sent its new state.

use std::path::Path;

use nc_data::hosted::{
    self, GameTier, HostedStore, RecruitingMode, RosterStore, SeatStatus, get_seat_by_number,
    record_player_abandoned, set_claimed_year,
};
use nc_data::{CoreGameData, PlayerActivityState, reset_player_slot_to_baseline};
use nc_engine::CampaignMaintenance;

use crate::game::sync::publish_state_to_claimed_seats;
use crate::game::turns::{apply_pending_turn_submissions, enqueue_turn_receipt, turn_for_year};
use crate::support::runtime::open_existing_campaign_store;

const SANDBOX_MAX_OCCUPANT_TURNS: u16 = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostedMaintenanceReport {
    pub start_year: u16,
    pub end_year: u16,
    pub applied_submissions: usize,
    pub rejected_submissions: usize,
    pub published_seats: usize,
}

/// Run `turns` years of maintenance for the hosted game in `game_dir`.
pub fn run_hosted_maintenance(
    game_dir: &Path,
    game_id: &str,
    turns: u32,
) -> Result<HostedMaintenanceReport, Box<dyn std::error::Error>> {
    let store = HostedStore::open(&game_dir.join("hosted.db"))?;
    let settings = hosted::get_settings(store.connection(), game_id)?;
    let campaign_store = open_existing_campaign_store(game_dir)?;
    let mut maintenance = CampaignMaintenance::load(&campaign_store)?;
    let start_year = maintenance.start_year();
    let mut ai = nc_ai_onnx::CampaignAi::load(
        &campaign_store,
        &maintenance,
        nc_ai_onnx::model_from_env().as_deref(),
    )?;
    let mut applied_submissions = 0;
    let mut rejected_submissions = 0;

    for _ in 0..turns {
        let submissions = apply_pending_turn_submissions(&store, game_id, &mut maintenance)?;
        applied_submissions += submissions.applied;
        for (queued, rejection) in submissions.rejections {
            tracing::warn!(
                "Rejected staged turn {} for {}: {}",
                queued.id,
                game_id,
                rejection.message
            );
            enqueue_turn_receipt(
                store.connection(),
                game_id,
                &queued.player_pubkey,
                &rejection.into_receipt(&queued),
            )?;
            rejected_submissions += 1;
        }
        if let Some(ai) = &ai {
            ai.play(&mut maintenance)?;
        }
        maintenance.run_turn()?;
        if let Some(ai) = &mut ai {
            ai.learn(&maintenance);
        }
        if settings.game_tier == GameTier::Sandbox {
            recycle_sandbox_seats(
                &store,
                game_dir,
                game_id,
                &mut maintenance.game_data,
                &mut maintenance.player_activity_states,
            )?;
        }
    }

    let end_year = maintenance.game_data.conquest.game_year;
    maintenance.save(&campaign_store)?;
    if let Some(ai) = &ai {
        ai.save(&campaign_store, end_year)?;
    }
    hosted::set_game_clock(
        store.connection(),
        game_id,
        end_year,
        turn_for_year(end_year),
    )?;
    let published_seats = publish_state_to_claimed_seats(&store, game_dir, game_id)?;

    Ok(HostedMaintenanceReport {
        start_year,
        end_year,
        applied_submissions,
        rejected_submissions,
        published_seats,
    })
}

/// One due year for the scheduler. The next due time moves one interval
/// past now whether or not the year ran, so a game that fails is retried
/// next interval rather than on every poll.
pub fn run_scheduled_maintenance(
    game_dir: &Path,
    game_id: &str,
) -> Result<HostedMaintenanceReport, Box<dyn std::error::Error>> {
    let result = run_hosted_maintenance(game_dir, game_id, 1);
    let store = HostedStore::open(&game_dir.join("hosted.db"))?;
    let mut settings = hosted::get_settings(store.connection(), game_id)?;
    if settings.maintenance_enabled {
        settings.maintenance_next_due_unix_seconds =
            Some(next_due_from_now(settings.maintenance_interval_minutes));
        hosted::update_settings(store.connection(), game_id, &settings)?;
    }
    result
}

pub fn next_due_from_now(interval_minutes: u32) -> i64 {
    chrono::Utc::now().timestamp() + i64::from(interval_minutes) * 60
}

/// Reset a sandbox seat whose player went MIA (three missed turns) or has
/// held it for ten turns, and reopen it for a replacement player.
fn recycle_sandbox_seats(
    store: &HostedStore,
    game_dir: &Path,
    game_id: &str,
    game_data: &mut CoreGameData,
    player_activity_states: &mut [PlayerActivityState],
) -> Result<(), Box<dyn std::error::Error>> {
    let current_year = game_data.conquest.game_year;
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
            tracing::info!(
                "Ejecting MIA player from sandbox seat {} in {} (3-turn inactivity)",
                seat_number,
                game_id
            );
        } else {
            tracing::info!(
                "Recycling sandbox seat {} in {} after {} turns",
                seat_number,
                game_id,
                SANDBOX_MAX_OCCUPANT_TURNS
            );
        }

        let player_npub = seat.player_pubkey.clone();
        if let Err(e) = reset_player_slot_to_baseline(game_data, state.player_record_index_1_based)
        {
            tracing::error!("Failed to reset player slot {}: {}", seat_number, e);
            continue;
        }

        if let Err(e) = hosted::reset_seat(store.connection(), game_id, seat_number) {
            tracing::error!("Failed to reset hosted seat {}: {}", seat_number, e);
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
    }

    if recycled_any {
        let mut settings = hosted::get_settings(store.connection(), game_id)?;
        if settings.recruiting == RecruitingMode::None {
            settings.recruiting = RecruitingMode::ReplacementPlayers;
            hosted::update_settings(store.connection(), game_id, &settings)?;
        }
        hosted::mark_catalog_dirty(store.connection(), game_id)?;
    }

    Ok(())
}
