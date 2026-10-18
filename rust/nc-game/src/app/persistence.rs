use std::collections::BTreeMap;

use nc_data::{
    CampaignStoreError, CommittedChange, PlayerAccessMode, ReportBlockRow, RuntimeChange,
    STALE_SESSION_MESSAGE, player_access_mode,
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

    /// Apply one change of this player's to the campaign's latest state and
    /// save it, in one store transaction (t204): other players' changes made
    /// since this session loaded are kept, and the session then holds the
    /// committed state, theirs included. `apply` edits that state only; it
    /// must not prompt. When it fails, nothing is saved and the session's
    /// state is unchanged.
    ///
    /// If maintenance has since written a newer year the store refuses; the
    /// session is then marked stale and never saves again (the refusal is
    /// reported once the current action finishes).
    pub(crate) fn change<T>(
        &mut self,
        apply: impl FnOnce(&mut RuntimeChange) -> Result<T, Box<dyn std::error::Error>>,
    ) -> Result<T, Box<dyn std::error::Error>> {
        if !matches!(self.player_access_mode, PlayerAccessMode::Normal) {
            return Err("campaign state is read-only in the current access mode".into());
        }
        self.change_in_any_access_mode(apply)
    }

    /// [`App::change`] for the one change a session in a read-only access
    /// mode makes: recording that a finished empire reviewed its outcome.
    pub(crate) fn change_in_any_access_mode<T>(
        &mut self,
        apply: impl FnOnce(&mut RuntimeChange) -> Result<T, Box<dyn std::error::Error>>,
    ) -> Result<T, Box<dyn std::error::Error>> {
        if self.session_stale {
            self.stale_notice_due = true;
            return Err(STALE_SESSION_MESSAGE.into());
        }
        let changed = self
            .planet
            .campaign_store
            .change(self.game_data.conquest.game_year, apply);
        match changed {
            Ok((value, committed)) => {
                self.adopt_committed_change(committed)?;
                Ok(value)
            }
            Err(err) => {
                match err.downcast_ref::<CampaignStoreError>() {
                    Some(CampaignStoreError::HostingPaused) => {
                        self.concurrent_notice_due = Some(err.to_string());
                    }
                    Some(CampaignStoreError::StaleSnapshot { .. }) => {
                        tracing::warn!(
                            player = self.player.record_index_1_based,
                            error = ?err,
                            "save refused: maintenance has run since this session loaded"
                        );
                        self.session_stale = true;
                        self.stale_notice_due = true;
                    }
                    _ => {}
                }
                Err(err)
            }
        }
    }

    fn adopt_committed_change(
        &mut self,
        committed: CommittedChange,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.snapshot_id = committed.snapshot_id;
        self.campaign_seed = committed.campaign_seed;
        let state = committed.state;
        self.game_data = state.game_data;
        self.report_block_rows = state.report_block_rows;
        self.queued_mail = state.queued_mail;
        self.winner_state = state.winner_state;
        self.bind_player_record_index_1_based(self.player.record_index_1_based)
    }

    /// Once an action that tried to save has finished, tell the player in the
    /// menu notice line that the save was refused. Errors from the save are
    /// reported by each caller in its own prompt, so this runs after them.
    pub(crate) fn show_stale_session_notice_if_due(&mut self) {
        if let Some(notice) = self.concurrent_notice_due.take() {
            if self
                .reload_runtime_state_and_bind_player_record_index_1_based(
                    self.player.record_index_1_based,
                )
                .is_ok()
            {
                self.show_command_menu_notice(CommandMenu::Main, notice);
            } else {
                self.session_stale = true;
                self.stale_notice_due = true;
            }
        }
        if std::mem::take(&mut self.stale_notice_due) {
            self.show_command_menu_notice(CommandMenu::Main, STALE_SESSION_MESSAGE);
        }
    }

    pub(crate) fn has_active_report_blocks(&self) -> bool {
        has_visible_runtime_reports(
            self.player.record_index_1_based as u8,
            &self.report_block_rows,
        )
    }
}

/// Add a report for `viewer_empire_id` after every stored report.
pub(crate) fn append_report_block(
    rows: &mut Vec<ReportBlockRow>,
    viewer_empire_id: u8,
    text: impl Into<String>,
) {
    let next_index = rows
        .iter()
        .map(|row| row.block_index)
        .max()
        .map(|idx| idx + 1)
        .unwrap_or(0);
    rows.push(ReportBlockRow {
        viewer_empire_id,
        block_index: next_index,
        decoded_text: text.into(),
        raw_bytes: None,
        recipient_deleted: false,
    });
}
