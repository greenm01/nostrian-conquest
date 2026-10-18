use rusqlite::{OptionalExtension, params};

use super::{CampaignStoreError, Connection, WinnerState};

pub(super) fn latest_snapshot_id_and_year(
    conn: &Connection,
) -> Result<Option<(i64, u16)>, CampaignStoreError> {
    conn.query_row(
        "SELECT id, game_year FROM snapshots ORDER BY game_year DESC LIMIT 1",
        [],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)? as u16)),
    )
    .optional()
    .map_err(CampaignStoreError::Sql)
}

pub(super) fn load_campaign_seed(conn: &Connection) -> Result<Option<u64>, CampaignStoreError> {
    conn.query_row(
        "SELECT int_value FROM campaign_metadata WHERE key = 'campaign_seed'",
        [],
        |row| row.get::<_, i64>(0),
    )
    .optional()
    .map(|value| value.map(|seed| seed as u64))
    .map_err(CampaignStoreError::Sql)
}

pub(super) fn load_runtime_schema_version(
    conn: &Connection,
) -> Result<Option<i64>, CampaignStoreError> {
    conn.query_row(
        "SELECT int_value FROM campaign_metadata WHERE key = 'runtime_schema_version'",
        [],
        |row| row.get::<_, i64>(0),
    )
    .optional()
    .map_err(CampaignStoreError::Sql)
}

pub(super) fn load_campaign_seed_tx(
    tx: &rusqlite::Transaction<'_>,
) -> Result<Option<u64>, CampaignStoreError> {
    tx.query_row(
        "SELECT int_value FROM campaign_metadata WHERE key = 'campaign_seed'",
        [],
        |row| row.get::<_, i64>(0),
    )
    .optional()
    .map(|value| value.map(|seed| seed as u64))
    .map_err(CampaignStoreError::Sql)
}

pub(super) fn load_winner_state(conn: &Connection) -> Result<WinnerState, CampaignStoreError> {
    let winner_empire = conn
        .query_row(
            "SELECT int_value FROM campaign_metadata WHERE key = 'winner_empire_raw'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map(|value| value.map(|v| v as u8))
        .map_err(CampaignStoreError::Sql)?;
    let winner_declared_year = conn
        .query_row(
            "SELECT int_value FROM campaign_metadata WHERE key = 'winner_declared_year'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map(|value| value.map(|v| v as u16))
        .map_err(CampaignStoreError::Sql)?;
    Ok(WinnerState {
        winner_empire,
        winner_declared_year,
    })
}

pub(super) fn load_winner_state_tx(
    tx: &rusqlite::Transaction<'_>,
) -> Result<WinnerState, CampaignStoreError> {
    let winner_empire = tx
        .query_row(
            "SELECT int_value FROM campaign_metadata WHERE key = 'winner_empire_raw'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map(|value| value.map(|v| v as u8))
        .map_err(CampaignStoreError::Sql)?;
    let winner_declared_year = tx
        .query_row(
            "SELECT int_value FROM campaign_metadata WHERE key = 'winner_declared_year'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map(|value| value.map(|v| v as u16))
        .map_err(CampaignStoreError::Sql)?;
    Ok(WinnerState {
        winner_empire,
        winner_declared_year,
    })
}

pub(super) fn persist_campaign_seed(
    conn: &Connection,
    seed: u64,
) -> Result<(), CampaignStoreError> {
    conn.execute(
        "INSERT INTO campaign_metadata(key, int_value)
         VALUES ('campaign_seed', ?1)
         ON CONFLICT(key) DO UPDATE SET int_value = excluded.int_value",
        params![seed as i64],
    )?;
    Ok(())
}

pub(super) fn persist_runtime_schema_version(
    conn: &Connection,
    version: i64,
) -> Result<(), CampaignStoreError> {
    conn.execute(
        "INSERT INTO campaign_metadata(key, int_value)
         VALUES ('runtime_schema_version', ?1)
         ON CONFLICT(key) DO UPDATE SET int_value = excluded.int_value",
        params![version],
    )?;
    Ok(())
}

pub(super) fn persist_winner_state_tx(
    tx: &rusqlite::Transaction<'_>,
    winner_state: WinnerState,
) -> Result<(), CampaignStoreError> {
    tx.execute(
        "DELETE FROM campaign_metadata
         WHERE key IN ('winner_empire_raw', 'winner_declared_year')",
        [],
    )?;
    if let Some(winner_empire) = winner_state.winner_empire {
        tx.execute(
            "INSERT INTO campaign_metadata(key, int_value)
             VALUES ('winner_empire_raw', ?1)",
            params![i64::from(winner_empire)],
        )?;
    }
    if let Some(winner_declared_year) = winner_state.winner_declared_year {
        tx.execute(
            "INSERT INTO campaign_metadata(key, int_value)
             VALUES ('winner_declared_year', ?1)",
            params![i64::from(winner_declared_year)],
        )?;
    }
    Ok(())
}

pub(super) fn persist_campaign_seed_tx(
    tx: &rusqlite::Transaction<'_>,
    seed: u64,
) -> Result<(), CampaignStoreError> {
    tx.execute(
        "INSERT INTO campaign_metadata(key, int_value)
         VALUES ('campaign_seed', ?1)
         ON CONFLICT(key) DO UPDATE SET int_value = excluded.int_value",
        params![seed as i64],
    )?;
    Ok(())
}

/// The campaign's revision: every save raises it, so a reader can tell
/// whether the campaign changed since it loaded (t204).
pub(super) fn load_revision(conn: &Connection) -> Result<i64, CampaignStoreError> {
    Ok(conn
        .query_row(
            "SELECT int_value FROM campaign_metadata WHERE key = 'revision'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
        .unwrap_or(0))
}

pub(super) fn bump_revision_tx(tx: &rusqlite::Transaction<'_>) -> Result<i64, CampaignStoreError> {
    Ok(tx.query_row(
        "INSERT INTO campaign_metadata(key, int_value) VALUES ('revision', 1)
         ON CONFLICT(key) DO UPDATE SET int_value = int_value + 1
         RETURNING int_value",
        [],
        |row| row.get(0),
    )?)
}
