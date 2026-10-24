//! One maintenance run against a campaign store.
//!
//! `nc-sysop maint`, hosted maintenance and `nc-cli maint-rust` all advance
//! a campaign through this: load the latest snapshot, run one year per
//! `run_turn`, and save the reports, mail, intel, activity, lifecycle, war
//! stats and winner state the years produced. The paths therefore write the
//! same snapshot for the same game.

use std::collections::BTreeMap;

use nc_data::{
    CampaignStore, CoreGameData, DiplomacyOverride, MaintenanceEvents, PlanetIntelSnapshot,
    PlayerActivityState, PlayerLifecycleState, PlayerWarStatsState, QueuedPlayerMail,
    ReportBlockRow, WinnerState, apply_inactivity_autopilot_policy,
    apply_maintenance_events_to_player_war_stats, carry_unread_report_blocks,
    latest_planet_intel_grants_for_viewer, merge_player_intel_from_runtime,
};

use super::{
    apply_results_reviewable_flags, build_results_report_blocks,
    run_maintenance_turn_with_context_seed_and_lifecycle,
};
use crate::{VisibleHazardIntel, visible_hazard_intel_from_snapshots};

/// A campaign loaded for maintenance. The public fields are the state a
/// caller may change between years: hosted maintenance applies staged
/// orders and mail before a year and recycles sandbox seats after one.
pub struct CampaignMaintenance {
    pub game_data: CoreGameData,
    pub queued_mail: Vec<QueuedPlayerMail>,
    pub player_activity_states: Vec<PlayerActivityState>,
    pub player_lifecycle_states: Vec<PlayerLifecycleState>,
    pub winner_state: WinnerState,
    start_year: u16,
    start_revision: i64,
    campaign_seed: u64,
    previous_report_block_rows: Vec<ReportBlockRow>,
    planet_intel_by_viewer: Vec<BTreeMap<usize, PlanetIntelSnapshot>>,
    player_war_stats: Vec<PlayerWarStatsState>,
    events: MaintenanceEvents,
    last_year_events: MaintenanceEvents,
}

impl CampaignMaintenance {
    /// Load the latest snapshot. A store without one, or a campaign whose
    /// winner has been declared, is refused.
    pub fn load(store: &CampaignStore) -> Result<Self, Box<dyn std::error::Error>> {
        let runtime_state = store.load_latest_runtime_state()?.ok_or(
            "campaign store has no snapshots; initialize the campaign with nc-sysop first",
        )?;
        if runtime_state.winner_state.winner_empire.is_some() {
            return Err("campaign winner has already been declared; maintenance is frozen".into());
        }
        let player_count = runtime_state.game_data.conquest.player_count;
        Ok(Self {
            start_year: runtime_state.game_data.conquest.game_year,
            start_revision: runtime_state.revision,
            campaign_seed: runtime_state.campaign_seed,
            planet_intel_by_viewer: store
                .load_snapshot_planet_intel_by_viewer(runtime_state.snapshot_id, player_count)?,
            player_activity_states: store.latest_player_activity_states(player_count)?,
            player_lifecycle_states: store.latest_player_lifecycle_states(player_count)?,
            player_war_stats: store.latest_player_war_stats(player_count)?,
            winner_state: runtime_state.winner_state,
            game_data: runtime_state.game_data,
            queued_mail: runtime_state.queued_mail,
            previous_report_block_rows: runtime_state.report_block_rows,
            events: MaintenanceEvents::default(),
            last_year_events: MaintenanceEvents::default(),
        })
    }

    /// The year the snapshot was loaded at.
    pub fn start_year(&self) -> u16 {
        self.start_year
    }

    /// Every year's events run so far, merged.
    pub fn events(&self) -> &MaintenanceEvents {
        &self.events
    }

    /// The events of the last year run, alone: what an AI seat learns from
    /// before its next turn.
    pub fn last_year_events(&self) -> &MaintenanceEvents {
        &self.last_year_events
    }

    /// Advance one year: put inactive empires on autopilot, run the
    /// engine's yearly pass with each empire's known hazards, and fold the
    /// year's intel grants and events into the run.
    pub fn run_turn(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.run_turn_with_diplomacy_overrides(&[])
    }

    /// `run_turn` under diplomacy directives the stored relations cannot
    /// hold, which a caller keeps between years.
    pub fn run_turn_with_diplomacy_overrides(
        &mut self,
        diplomacy_overrides: &[DiplomacyOverride],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let inactivity_threshold =
            u8::try_from(self.game_data.setup.autopilot_after_turns).unwrap_or(u8::MAX);
        let autopilot_engaged_empires = apply_inactivity_autopilot_policy(
            &mut self.game_data,
            inactivity_threshold,
            &mut self.player_activity_states,
        );
        let visible_hazards =
            visible_hazards_from_snapshots(&self.game_data, &self.planet_intel_by_viewer);
        let mut events = run_maintenance_turn_with_context_seed_and_lifecycle(
            &mut self.game_data,
            self.campaign_seed,
            &visible_hazards,
            diplomacy_overrides,
            &mut self.player_lifecycle_states,
            &mut self.winner_state,
        )?;
        events.autopilot_engaged_empires = autopilot_engaged_empires;
        for viewer_empire_id in 1..=self.game_data.conquest.player_count {
            let viewer_idx = usize::from(viewer_empire_id) - 1;
            let grants = latest_planet_intel_grants_for_viewer(&events, viewer_empire_id);
            let previous = std::mem::take(&mut self.planet_intel_by_viewer[viewer_idx]);
            self.planet_intel_by_viewer[viewer_idx] = merge_player_intel_from_runtime(
                &self.game_data,
                viewer_empire_id,
                self.game_data.conquest.game_year,
                Some(&previous),
                Some(&grants),
            );
        }
        self.last_year_events = events.clone();
        self.events.extend(events);
        Ok(())
    }

    /// Compose the run's reports behind last year's unread ones, tally war
    /// stats, and save the snapshot.
    pub fn save(mut self, store: &CampaignStore) -> Result<(), Box<dyn std::error::Error>> {
        let report_block_rows = carry_unread_report_blocks(
            &self.previous_report_block_rows,
            build_results_report_blocks(&self.game_data, &self.events),
        );
        apply_results_reviewable_flags(&mut self.game_data, &report_block_rows);
        apply_maintenance_events_to_player_war_stats(&mut self.player_war_stats, &self.events);
        store
            .expecting_revision(self.start_revision)
            .save_year_with_intel_activity_lifecycle_and_war_stats(
                self.start_year,
                &self.game_data,
                &report_block_rows,
                &self.queued_mail,
                &self.planet_intel_by_viewer,
                &self.player_activity_states,
                &self.player_lifecycle_states,
                &self.player_war_stats,
                self.winner_state,
            )?;
        Ok(())
    }
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
