//! One player's change, applied to the latest state inside one write
//! transaction (t204), so players of one campaign can play at the same time.

use super::{
    CampaignStore, CampaignStoreError, PlayerActivityState, PlayerLifecycleState, WinnerState,
};
use crate::{CoreGameData, QueuedPlayerMail, ReportBlockRow};

/// The runtime state a change edits: the latest stored year, loaded inside
/// the change's transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeChange {
    pub game_data: CoreGameData,
    pub report_block_rows: Vec<ReportBlockRow>,
    pub queued_mail: Vec<QueuedPlayerMail>,
    pub player_activity_states: Vec<PlayerActivityState>,
    pub player_lifecycle_states: Vec<PlayerLifecycleState>,
    pub winner_state: WinnerState,
}

/// The state a change committed, and the snapshot it was saved as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedChange {
    pub snapshot_id: i64,
    pub campaign_seed: u64,
    pub state: RuntimeChange,
}

impl CampaignStore {
    /// Apply `apply` to the latest state and save the result, all in one
    /// `BEGIN IMMEDIATE` transaction: a concurrent change waits for this one
    /// and then sees it, so neither is refused or lost.
    ///
    /// `loaded_year` is the year the caller's session loaded. When
    /// maintenance has since written a newer year, nothing runs and the
    /// change fails with [`CampaignStoreError::StaleSnapshot`]; a paused
    /// campaign fails with [`CampaignStoreError::HostingPaused`]. When
    /// `apply` fails, nothing is written. `apply` must not change the year.
    /// Planet intel is kept, with each empire's own worlds brought up to date.
    pub fn change<T, E>(
        &self,
        loaded_year: u16,
        apply: impl FnOnce(&mut RuntimeChange) -> Result<T, E>,
    ) -> Result<(T, CommittedChange), E>
    where
        E: From<CampaignStoreError>,
    {
        let mut conn = self.write_connection()?;
        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(CampaignStoreError::Sql)?;
        super::runtime::refuse_if_superseded(&tx, loaded_year)?;
        if super::hosting::writes_blocked(&tx)? {
            return Err(CampaignStoreError::HostingPaused.into());
        }
        let Some((snapshot_id, year)) = super::metadata::latest_snapshot_id_and_year(&tx)? else {
            return Err(CampaignStoreError::InvalidState(
                "campaign store has no runtime snapshots".to_string(),
            )
            .into());
        };
        let game_data = super::snapshot_core::load_snapshot_game_data(&tx, snapshot_id)?;
        let player_count = game_data.conquest.player_count;
        let mut state = RuntimeChange {
            report_block_rows: super::report_blocks::load_report_block_rows_filtered(
                &tx,
                snapshot_id,
                true,
            )?,
            queued_mail: super::mail::load_queued_mail_rows(&tx, snapshot_id)?,
            player_activity_states: super::player_activity::load_player_activity_rows(
                &tx,
                snapshot_id,
                player_count,
            )?,
            player_lifecycle_states: super::player_lifecycle::load_player_lifecycle_rows(
                &tx,
                snapshot_id,
                player_count,
            )?,
            winner_state: super::metadata::load_winner_state(&tx)?,
            game_data,
        };
        let value = apply(&mut state)?;
        if state.game_data.conquest.game_year != year {
            return Err(CampaignStoreError::InvalidState(
                "a player change cannot change the game year".to_string(),
            )
            .into());
        }
        let snapshot_id = super::runtime::save_runtime_state_internal_tx(
            &tx,
            &state.game_data,
            year,
            &state.report_block_rows,
            &state.queued_mail,
            None,
            None,
            Some(&state.player_activity_states),
            Some(&state.player_lifecycle_states),
            None,
            Some(state.winner_state),
        )?;
        let campaign_seed = super::metadata::load_campaign_seed_tx(&tx)?
            .ok_or_else(|| CampaignStoreError::InvalidState("campaign seed missing".into()))?;
        tx.commit().map_err(CampaignStoreError::Sql)?;
        Ok((
            value,
            CommittedChange {
                snapshot_id,
                campaign_seed,
                state,
            },
        ))
    }
}
