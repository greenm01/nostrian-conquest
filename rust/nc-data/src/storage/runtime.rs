use rusqlite::{OptionalExtension, params};
use std::collections::BTreeMap;

use super::{
    CampaignRuntimeState, CampaignStore, CampaignStoreError, PlanetIntelSnapshot,
    PlayerActivityState, PlayerLifecycleState, PlayerWarStatsState,
};
use crate::{
    CoreGameData, QueuedPlayerMail, ReportBlockRow, WinnerState, derive_campaign_seed_from_runtime,
    generate_campaign_seed,
};

impl CampaignStore {
    pub fn load_latest_runtime_game_data(&self) -> Result<CoreGameData, CampaignStoreError> {
        let Some(state) = self.load_latest_runtime_state()? else {
            return Err(CampaignStoreError::InvalidState(
                "campaign store has no runtime snapshots".to_string(),
            ));
        };
        Ok(state.game_data)
    }

    pub fn load_snapshot_game_data(
        &self,
        snapshot_id: i64,
    ) -> Result<CoreGameData, CampaignStoreError> {
        let mut conn = self.connection()?;
        super::snapshot_core::load_snapshot_game_data(&mut conn, snapshot_id)
    }

    /// Includes hidden reports so a later save preserves their stored history.
    pub fn load_latest_runtime_state(
        &self,
    ) -> Result<Option<CampaignRuntimeState>, CampaignStoreError> {
        let mut conn = self.connection()?;
        let Some((snapshot_id, game_year)) =
            super::metadata::latest_snapshot_id_and_year(&mut conn)?
        else {
            return Ok(None);
        };
        let game_data = super::snapshot_core::load_snapshot_game_data(&mut conn, snapshot_id)?;
        let report_block_rows =
            super::report_blocks::load_report_block_rows_filtered(&mut conn, snapshot_id, true)?;
        let queued_mail = super::mail::load_queued_mail_rows(&mut conn, snapshot_id)?;
        let winner_state = super::metadata::load_winner_state(&mut conn)?;
        let stored_campaign_seed = super::metadata::load_campaign_seed(&mut conn)?;
        let campaign_seed = stored_campaign_seed.unwrap_or_else(|| {
            derive_campaign_seed_from_runtime(&game_data, &report_block_rows, &queued_mail)
        });
        if stored_campaign_seed.is_none() {
            super::metadata::persist_campaign_seed(&mut conn, campaign_seed)?;
        }
        Ok(Some(CampaignRuntimeState {
            snapshot_id,
            game_year,
            campaign_seed,
            winner_state,
            game_data,
            report_block_rows,
            queued_mail,
        }))
    }

    /// Save a runtime snapshot for `game_data`'s year.
    ///
    /// Every `save_runtime_state_*` method takes `loaded_year`, the game year
    /// the caller loaded its state from (the year being written when it
    /// creates state from nothing). If a newer snapshot exists, nothing is
    /// written and the save fails with [`CampaignStoreError::StaleSnapshot`].
    /// Maintenance loads year Y and writes Y + 1, so it is never refused.
    pub fn save_runtime_state_structured(
        &self,
        loaded_year: u16,
        game_data: &CoreGameData,
        report_block_rows: &[ReportBlockRow],
        queued_mail: &[QueuedPlayerMail],
    ) -> Result<i64, CampaignStoreError> {
        self.save_runtime_state_internal(
            loaded_year,
            game_data,
            report_block_rows,
            queued_mail,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }

    pub fn save_runtime_state_structured_with_intel(
        &self,
        loaded_year: u16,
        game_data: &CoreGameData,
        report_block_rows: &[ReportBlockRow],
        queued_mail: &[QueuedPlayerMail],
        planet_intel_by_viewer: &[BTreeMap<usize, PlanetIntelSnapshot>],
    ) -> Result<i64, CampaignStoreError> {
        self.save_runtime_state_structured_with_intel_and_seed(
            loaded_year,
            game_data,
            report_block_rows,
            queued_mail,
            planet_intel_by_viewer,
            None,
        )
    }

    pub fn save_runtime_state_structured_with_intel_and_seed(
        &self,
        loaded_year: u16,
        game_data: &CoreGameData,
        report_block_rows: &[ReportBlockRow],
        queued_mail: &[QueuedPlayerMail],
        planet_intel_by_viewer: &[BTreeMap<usize, PlanetIntelSnapshot>],
        campaign_seed: Option<u64>,
    ) -> Result<i64, CampaignStoreError> {
        self.save_runtime_state_internal(
            loaded_year,
            game_data,
            report_block_rows,
            queued_mail,
            Some(planet_intel_by_viewer),
            campaign_seed,
            None,
            None,
            None,
            None,
        )
    }

    pub fn save_runtime_state_structured_with_intel_and_activity(
        &self,
        loaded_year: u16,
        game_data: &CoreGameData,
        report_block_rows: &[ReportBlockRow],
        queued_mail: &[QueuedPlayerMail],
        planet_intel_by_viewer: &[BTreeMap<usize, PlanetIntelSnapshot>],
        player_activity_states: &[PlayerActivityState],
    ) -> Result<i64, CampaignStoreError> {
        self.save_runtime_state_internal(
            loaded_year,
            game_data,
            report_block_rows,
            queued_mail,
            Some(planet_intel_by_viewer),
            None,
            Some(player_activity_states),
            None,
            None,
            None,
        )
    }

    pub fn save_runtime_state_structured_with_intel_activity_and_war_stats(
        &self,
        loaded_year: u16,
        game_data: &CoreGameData,
        report_block_rows: &[ReportBlockRow],
        queued_mail: &[QueuedPlayerMail],
        planet_intel_by_viewer: &[BTreeMap<usize, PlanetIntelSnapshot>],
        player_activity_states: &[PlayerActivityState],
        player_war_stats: &[PlayerWarStatsState],
    ) -> Result<i64, CampaignStoreError> {
        self.save_runtime_state_internal(
            loaded_year,
            game_data,
            report_block_rows,
            queued_mail,
            Some(planet_intel_by_viewer),
            None,
            Some(player_activity_states),
            None,
            Some(player_war_stats),
            None,
        )
    }

    pub fn save_runtime_state_structured_with_intel_activity_lifecycle_and_war_stats(
        &self,
        loaded_year: u16,
        game_data: &CoreGameData,
        report_block_rows: &[ReportBlockRow],
        queued_mail: &[QueuedPlayerMail],
        planet_intel_by_viewer: &[BTreeMap<usize, PlanetIntelSnapshot>],
        player_activity_states: &[PlayerActivityState],
        player_lifecycle_states: &[PlayerLifecycleState],
        player_war_stats: &[PlayerWarStatsState],
        winner_state: WinnerState,
    ) -> Result<i64, CampaignStoreError> {
        self.save_runtime_state_internal(
            loaded_year,
            game_data,
            report_block_rows,
            queued_mail,
            Some(planet_intel_by_viewer),
            None,
            Some(player_activity_states),
            Some(player_lifecycle_states),
            Some(player_war_stats),
            Some(winner_state),
        )
    }

    pub fn save_runtime_state_structured_with_intel_activity_and_lifecycle(
        &self,
        loaded_year: u16,
        game_data: &CoreGameData,
        report_block_rows: &[ReportBlockRow],
        queued_mail: &[QueuedPlayerMail],
        planet_intel_by_viewer: &[BTreeMap<usize, PlanetIntelSnapshot>],
        player_activity_states: &[PlayerActivityState],
        player_lifecycle_states: &[PlayerLifecycleState],
        winner_state: WinnerState,
    ) -> Result<i64, CampaignStoreError> {
        self.save_runtime_state_internal(
            loaded_year,
            game_data,
            report_block_rows,
            queued_mail,
            Some(planet_intel_by_viewer),
            None,
            Some(player_activity_states),
            Some(player_lifecycle_states),
            None,
            Some(winner_state),
        )
    }

    fn save_runtime_state_internal(
        &self,
        loaded_year: u16,
        game_data: &CoreGameData,
        report_block_rows: &[ReportBlockRow],
        queued_mail: &[QueuedPlayerMail],
        planet_intel_by_viewer_override: Option<&[BTreeMap<usize, PlanetIntelSnapshot>]>,
        campaign_seed_override: Option<u64>,
        player_activity_override: Option<&[PlayerActivityState]>,
        player_lifecycle_override: Option<&[PlayerLifecycleState]>,
        player_war_stats_override: Option<&[PlayerWarStatsState]>,
        winner_state_override: Option<WinnerState>,
    ) -> Result<i64, CampaignStoreError> {
        let year = game_data.conquest.game_year;
        let mut conn = self.connection()?;
        let tx = conn.transaction()?;
        refuse_if_superseded(&tx, loaded_year)?;
        let snapshot_id = save_runtime_state_internal_tx(
            &tx,
            game_data,
            year,
            report_block_rows,
            queued_mail,
            planet_intel_by_viewer_override,
            campaign_seed_override,
            player_activity_override,
            player_lifecycle_override,
            player_war_stats_override,
            winner_state_override,
        )?;
        tx.commit()?;
        Ok(snapshot_id)
    }
}

/// Refuse a save when a snapshot newer than `loaded_year` exists: maintenance
/// has written a later year since the caller loaded its state, and the store
/// reads the highest year, so the write would never be seen.
fn refuse_if_superseded(
    tx: &rusqlite::Transaction<'_>,
    loaded_year: u16,
) -> Result<(), CampaignStoreError> {
    let latest_year = tx
        .query_row("SELECT MAX(game_year) FROM snapshots", [], |row| {
            row.get::<_, Option<i64>>(0)
        })?
        .and_then(|year| u16::try_from(year).ok());
    match latest_year {
        Some(latest_year) if latest_year > loaded_year => Err(CampaignStoreError::StaleSnapshot {
            loaded_year,
            latest_year,
        }),
        _ => Ok(()),
    }
}

pub(super) fn save_runtime_state_internal_tx(
    tx: &rusqlite::Transaction<'_>,
    game_data: &CoreGameData,
    year: u16,
    report_block_rows: &[ReportBlockRow],
    queued_mail: &[QueuedPlayerMail],
    planet_intel_by_viewer_override: Option<&[BTreeMap<usize, PlanetIntelSnapshot>]>,
    campaign_seed_override: Option<u64>,
    player_activity_override: Option<&[PlayerActivityState]>,
    player_lifecycle_override: Option<&[PlayerLifecycleState]>,
    player_war_stats_override: Option<&[PlayerWarStatsState]>,
    winner_state_override: Option<WinnerState>,
) -> Result<i64, CampaignStoreError> {
    let campaign_seed = super::metadata::load_campaign_seed_tx(&tx)?
        .or(campaign_seed_override)
        .unwrap_or_else(generate_campaign_seed);
    super::metadata::persist_campaign_seed_tx(&tx, campaign_seed)?;
    let previous_snapshot_id = tx
        .query_row(
            "SELECT id FROM snapshots ORDER BY game_year DESC LIMIT 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()?;
    let previous_intel = if let Some(previous_snapshot_id) = previous_snapshot_id {
        super::intel::load_intel_rows(&tx, previous_snapshot_id)?
    } else {
        BTreeMap::new()
    };
    tx.execute(
        "DELETE FROM snapshots WHERE game_year = ?1",
        params![i64::from(year)],
    )?;
    tx.execute(
        "INSERT INTO snapshots(game_year) VALUES (?1)",
        params![i64::from(year)],
    )?;
    let snapshot_id = tx.last_insert_rowid();
    super::snapshot_core::write_snapshot_core_rows(&tx, snapshot_id, game_data)?;
    super::report_blocks::write_report_block_rows(&tx, snapshot_id, report_block_rows)?;
    super::mail::write_queued_mail_rows(&tx, snapshot_id, queued_mail)?;
    super::intel::write_planet_intel_rows(
        &tx,
        snapshot_id,
        game_data,
        year,
        planet_intel_by_viewer_override,
        &previous_intel,
    )?;
    super::planet_owned_since::write_owned_planet_year_rows(
        &tx,
        snapshot_id,
        game_data,
        year,
        previous_snapshot_id,
    )?;
    super::player_activity::write_player_activity_rows(
        &tx,
        snapshot_id,
        game_data,
        previous_snapshot_id,
        player_activity_override,
    )?;
    super::player_lifecycle::write_player_lifecycle_rows(
        &tx,
        snapshot_id,
        game_data.conquest.player_count,
        previous_snapshot_id,
        player_lifecycle_override,
    )?;
    super::player_war_stats::write_player_war_stats_rows(
        &tx,
        snapshot_id,
        game_data.conquest.player_count,
        previous_snapshot_id,
        player_war_stats_override,
    )?;
    let winner_state = winner_state_override.unwrap_or(super::metadata::load_winner_state_tx(&tx)?);
    super::metadata::persist_winner_state_tx(&tx, winner_state)?;
    Ok(snapshot_id)
}
