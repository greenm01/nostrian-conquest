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
    /// The campaign's revision after this change; changes commit in its
    /// order.
    pub revision: i64,
    /// How long the change waited for other sessions' changes to finish.
    pub waited: std::time::Duration,
    pub campaign_seed: u64,
    pub state: RuntimeChange,
}

/// How a change is saved.
#[derive(Clone, Copy)]
enum Write {
    /// Only the rows the change altered, in place (t204, phase 2).
    Rows,
    /// The whole year rewritten as a new snapshot, as every save did before.
    Year,
}

impl CampaignStore {
    /// Apply `apply` to the latest state and save the result, all in one
    /// `BEGIN IMMEDIATE` transaction: a concurrent change waits for this one
    /// and then sees it, so neither is refused or lost. Only the rows the
    /// change altered are written, in the current year's snapshot.
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
        self.change_written(loaded_year, apply, Write::Rows)
    }

    /// [`CampaignStore::change`], saved by rewriting the whole year as a new
    /// snapshot: the tests' reference for what a change must leave stored.
    #[doc(hidden)]
    pub fn change_rewriting_the_year<T, E>(
        &self,
        loaded_year: u16,
        apply: impl FnOnce(&mut RuntimeChange) -> Result<T, E>,
    ) -> Result<(T, CommittedChange), E>
    where
        E: From<CampaignStoreError>,
    {
        self.change_written(loaded_year, apply, Write::Year)
    }

    fn change_written<T, E>(
        &self,
        loaded_year: u16,
        apply: impl FnOnce(&mut RuntimeChange) -> Result<T, E>,
        write: Write,
    ) -> Result<(T, CommittedChange), E>
    where
        E: From<CampaignStoreError>,
    {
        let mut conn = self.write_connection()?;
        let waiting = std::time::Instant::now();
        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(CampaignStoreError::Sql)?;
        let waited = waiting.elapsed();
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
        let before = RuntimeChange {
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
        let mut state = before.clone();
        let value = apply(&mut state)?;
        if state.game_data.conquest.game_year != year {
            return Err(CampaignStoreError::InvalidState(
                "a player change cannot change the game year".to_string(),
            )
            .into());
        }
        let snapshot_id = match write {
            Write::Rows => {
                write_changed_rows(&tx, snapshot_id, year, &before, &state)?;
                super::metadata::bump_revision_tx(&tx)?;
                snapshot_id
            }
            Write::Year => super::runtime::save_runtime_state_internal_tx(
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
            )?,
        };
        let campaign_seed = super::metadata::load_campaign_seed_tx(&tx)?
            .ok_or_else(|| CampaignStoreError::InvalidState("campaign seed missing".into()))?;
        let revision = super::metadata::load_revision(&tx)?;
        tx.commit().map_err(CampaignStoreError::Sql)?;
        Ok((
            value,
            CommittedChange {
                snapshot_id,
                revision,
                waited,
                campaign_seed,
                state,
            },
        ))
    }
}

/// Write, into snapshot `snapshot_id`, what a full save of `after` would
/// store and `before` (what the snapshot holds) does not: the changed
/// records and the intel, owned-since, activity and lifecycle rows derived
/// from them. War stats come from the previous snapshot in a full save,
/// which is this one, so they never change here.
fn write_changed_rows(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    year: u16,
    before: &RuntimeChange,
    after: &RuntimeChange,
) -> Result<(), CampaignStoreError> {
    super::snapshot_core::write_changed_core_rows(
        tx,
        snapshot_id,
        &before.game_data,
        &after.game_data,
    )?;
    super::report_blocks::write_changed_report_block_rows(
        tx,
        snapshot_id,
        &before.report_block_rows,
        &after.report_block_rows,
    )?;
    super::mail::write_changed_queued_mail_rows(
        tx,
        snapshot_id,
        &before.queued_mail,
        &after.queued_mail,
    )?;
    super::intel::write_changed_planet_intel_rows(tx, snapshot_id, &after.game_data, year)?;
    super::planet_owned_since::write_changed_owned_planet_year_rows(
        tx,
        snapshot_id,
        &after.game_data,
        year,
    )?;
    super::player_activity::write_changed_player_activity_rows(
        tx,
        snapshot_id,
        &after.game_data,
        &after.player_activity_states,
    )?;
    super::player_lifecycle::write_changed_player_lifecycle_rows(
        tx,
        snapshot_id,
        after.game_data.conquest.player_count,
        &after.player_lifecycle_states,
    )?;
    if after.winner_state != before.winner_state {
        super::metadata::persist_winner_state_tx(tx, after.winner_state)?;
    }
    Ok(())
}
