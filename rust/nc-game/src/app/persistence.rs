use std::collections::BTreeMap;

use nc_data::{
    CampaignStoreError, PlayerAccessMode, ReportBlockRow, STALE_SESSION_MESSAGE, player_access_mode,
};

use super::state::App;
use crate::reports::has_visible_runtime_reports;
use crate::screen::CommandMenu;

impl App {
    pub(crate) fn bind_player_record_index_1_based(
        &mut self,
        player_record_index_1_based: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.startup_state.reserved_seat_alias = self
            .game_config
            .reservation_for_player(player_record_index_1_based)
            .map(|reservation| reservation.alias.clone());
        self.player = crate::model::PlayerContext::from_game_data(
            &self.game_data,
            player_record_index_1_based,
        )?;
        self.planet_intel_snapshots = self
            .planet
            .campaign_store
            .latest_planet_intel_for_viewer(player_record_index_1_based as u8)?
            .into_iter()
            .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
            .collect::<BTreeMap<_, _>>();
        self.player_activity_states = self
            .planet
            .campaign_store
            .latest_player_activity_states(self.game_data.conquest.player_count)?;
        self.player_lifecycle_states = self
            .planet
            .campaign_store
            .latest_player_lifecycle_states(self.game_data.conquest.player_count)?;
        self.player_access_mode = player_access_mode(
            player_record_index_1_based,
            &self.player_lifecycle_states,
            self.winner_state,
        );
        self.player
            .apply_access_mode_override(self.player_access_mode);
        self.owned_planet_years = self
            .planet
            .campaign_store
            .latest_owned_planet_years_for_empire(player_record_index_1_based as u8)?;
        self.planet.intel_snapshots = self.planet_intel_snapshots.clone();
        self.refresh_review_context()?;
        Ok(())
    }

    pub(crate) fn reload_runtime_state(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let runtime_state = self
            .planet
            .campaign_store
            .load_latest_runtime_state()?
            .ok_or(
                "campaign store has no snapshots; initialize the campaign with nc-sysop first",
            )?;
        self.snapshot_id = runtime_state.snapshot_id;
        self.campaign_seed = runtime_state.campaign_seed;
        self.winner_state = runtime_state.winner_state;
        self.game_data = runtime_state.game_data;
        self.report_block_rows = runtime_state.report_block_rows;
        self.queued_mail = runtime_state.queued_mail;
        self.player_activity_states = self
            .planet
            .campaign_store
            .latest_player_activity_states(self.game_data.conquest.player_count)?;
        self.player_lifecycle_states = self
            .planet
            .campaign_store
            .latest_player_lifecycle_states(self.game_data.conquest.player_count)?;
        Ok(())
    }

    pub(crate) fn reload_runtime_state_and_bind_player_record_index_1_based(
        &mut self,
        player_record_index_1_based: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.reload_runtime_state()?;
        self.bind_player_record_index_1_based(player_record_index_1_based)
    }

    pub(crate) fn append_report_block(&mut self, text: impl Into<String>) {
        let next_index = self
            .report_block_rows
            .iter()
            .map(|row| row.block_index)
            .max()
            .map(|idx| idx + 1)
            .unwrap_or(0);
        self.report_block_rows.push(ReportBlockRow {
            viewer_empire_id: self.player.record_index_1_based as u8,
            block_index: next_index,
            decoded_text: text.into(),
            raw_bytes: None,
            recipient_deleted: false,
        });
    }

    /// Save the in-memory state over the snapshot of the year this session
    /// loaded. If maintenance has since written a newer year the store
    /// refuses; the session is then marked stale and never saves again (the
    /// refusal is reported once the current action finishes).
    fn persist_runtime_snapshot(&mut self) -> Result<i64, Box<dyn std::error::Error>> {
        if self.session_stale {
            self.stale_notice_due = true;
            return Err(STALE_SESSION_MESSAGE.into());
        }
        let planet_intel_by_viewer = (1..=self.game_data.conquest.player_count)
            .map(|viewer_empire_id| {
                self.planet
                    .campaign_store
                    .latest_planet_intel_for_viewer(viewer_empire_id)
                    .map(|snapshots| {
                        snapshots
                            .into_iter()
                            .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                            .collect::<BTreeMap<_, _>>()
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let saved = self
            .planet
            .campaign_store
            .save_runtime_state_structured_with_intel_activity_and_lifecycle(
                self.game_data.conquest.game_year,
                &self.game_data,
                &self.report_block_rows,
                &self.queued_mail,
                &planet_intel_by_viewer,
                &self.player_activity_states,
                &self.player_lifecycle_states,
                self.winner_state,
            );
        match saved {
            Ok(snapshot_id) => Ok(snapshot_id),
            Err(err @ CampaignStoreError::StaleSnapshot { .. }) => {
                tracing::warn!(
                    player = self.player.record_index_1_based,
                    error = ?err,
                    "save refused: maintenance has run since this session loaded"
                );
                self.session_stale = true;
                self.stale_notice_due = true;
                Err(err.into())
            }
            Err(err) => Err(err.into()),
        }
    }

    /// Once an action that tried to save has finished, tell the player in the
    /// menu notice line that the save was refused. Errors from the save are
    /// reported by each caller in its own prompt, so this runs after them.
    pub(crate) fn show_stale_session_notice_if_due(&mut self) {
        if std::mem::take(&mut self.stale_notice_due) {
            self.show_command_menu_notice(CommandMenu::Main, STALE_SESSION_MESSAGE);
        }
    }

    pub(crate) fn save_game_data(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !matches!(self.player_access_mode, PlayerAccessMode::Normal) {
            return Err("campaign state is read-only in the current access mode".into());
        }
        self.snapshot_id = self.persist_runtime_snapshot()?;
        self.planet_intel_snapshots = self
            .planet
            .campaign_store
            .latest_planet_intel_for_viewer(self.player.record_index_1_based as u8)?
            .into_iter()
            .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
            .collect::<BTreeMap<_, _>>();
        self.owned_planet_years = self
            .planet
            .campaign_store
            .latest_owned_planet_years_for_empire(self.player.record_index_1_based as u8)?;
        self.planet.intel_snapshots = self.planet_intel_snapshots.clone();
        self.player_activity_states = self
            .planet
            .campaign_store
            .latest_player_activity_states(self.game_data.conquest.player_count)?;
        self.player_lifecycle_states = self
            .planet
            .campaign_store
            .latest_player_lifecycle_states(self.game_data.conquest.player_count)?;
        self.player_access_mode = player_access_mode(
            self.player.record_index_1_based,
            &self.player_lifecycle_states,
            self.winner_state,
        );
        Ok(())
    }

    pub(crate) fn save_terminal_access_state(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.snapshot_id = self.persist_runtime_snapshot()?;
        Ok(())
    }

    pub(crate) fn has_active_report_blocks(&self) -> bool {
        has_visible_runtime_reports(
            self.player.record_index_1_based as u8,
            &self.report_block_rows,
        )
    }
}
