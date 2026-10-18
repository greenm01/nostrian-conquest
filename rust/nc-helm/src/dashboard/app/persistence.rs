use nc_data::{PlayerAccessMode, ReportBlockRow, RuntimeChange, player_access_mode};

use super::state::DashApp;

impl DashApp {
    /// Apply one change of the player's (t204). With a campaign store it
    /// runs on the campaign's latest state inside the store transaction, so
    /// other players' changes are kept, and the dashboard then holds the
    /// committed state. Without one (hosted play) it edits the dashboard's
    /// copy; the staged turn carries the order to the host. When `apply`
    /// fails, nothing changes.
    pub(crate) fn change<T>(
        &mut self,
        apply: impl FnOnce(&mut RuntimeChange) -> Result<T, Box<dyn std::error::Error>>,
    ) -> Result<T, Box<dyn std::error::Error>> {
        let Some(store) = self.campaign_store.clone() else {
            let mut state = RuntimeChange {
                game_data: self.game_data.clone(),
                report_block_rows: self.report_block_rows.clone(),
                queued_mail: self.queued_mail.clone(),
                player_activity_states: self.player_activity_states.clone(),
                player_lifecycle_states: self.player_lifecycle_states.clone(),
                winner_state: self.winner_state,
            };
            let value = apply(&mut state)?;
            self.adopt(state);
            return Ok(value);
        };
        let access_mode = player_access_mode(
            self.player_record_index_1_based,
            &self.player_lifecycle_states,
            self.winner_state,
        );
        if !matches!(access_mode, PlayerAccessMode::Normal) {
            return Err("campaign state is read-only in the current access mode".into());
        }
        let (value, committed) = store.change(self.game_data.conquest.game_year, apply)?;
        self.adopt(committed.state);
        self.owned_planet_years =
            store.latest_owned_planet_years_for_empire(self.player_record_index_1_based as u8)?;
        self.planet_intel_snapshots =
            store.latest_planet_intel_for_viewer(self.player_record_index_1_based as u8)?;
        self.player_war_stats = store
            .latest_player_war_stats(self.game_data.conquest.player_count)?
            .get(self.player_record_index_1_based.saturating_sub(1))
            .copied()
            .unwrap_or_else(|| {
                nc_data::PlayerWarStatsState::for_player(self.player_record_index_1_based)
            });
        Ok(value)
    }

    fn adopt(&mut self, state: RuntimeChange) {
        self.game_data = state.game_data;
        self.report_block_rows = state.report_block_rows;
        self.queued_mail = state.queued_mail;
        self.player_activity_states = state.player_activity_states;
        self.player_lifecycle_states = state.player_lifecycle_states;
        self.winner_state = state.winner_state;
        self.game_data_revision += 1;
    }
}

/// A change the player's input refused: its message is shown and nothing is
/// saved.
#[derive(Debug)]
pub(crate) struct Refused(pub String);

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Refused {}

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
