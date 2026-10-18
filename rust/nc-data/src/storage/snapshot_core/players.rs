use rusqlite::{Connection, Row, Transaction, named_params, params};

use super::{CampaignStoreError, insert_row, position_of};
use crate::model::{DiplomaticRelation, Player, PlayerMode, RecordNumber};

/// One row per PLAYER.DAT record; `snapshot_player_diplomacy` holds each
/// empire's stance toward empires 1 to 25, by their number.
pub(super) const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS snapshot_players (
        snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
        record_index INTEGER NOT NULL,
        mode INTEGER NOT NULL,
        handle TEXT NOT NULL,
        page_lines INTEGER NOT NULL,
        empire_name TEXT NOT NULL,
        messages_first INTEGER NOT NULL,
        messages_last INTEGER NOT NULL,
        results_first INTEGER NOT NULL,
        results_last INTEGER NOT NULL,
        fleets_first INTEGER NOT NULL,
        fleets_last INTEGER NOT NULL,
        bases_first INTEGER NOT NULL,
        bases_last INTEGER NOT NULL,
        ipbms_first INTEGER NOT NULL,
        ipbms_last INTEGER NOT NULL,
        homeworld INTEGER NOT NULL,
        last_year_on INTEGER NOT NULL,
        planet_count INTEGER NOT NULL,
        tax_rate INTEGER NOT NULL,
        production INTEGER NOT NULL,
        autopilot INTEGER NOT NULL,
        PRIMARY KEY(snapshot_id, record_index)
    );
    CREATE TABLE IF NOT EXISTS snapshot_player_diplomacy (
        snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
        record_index INTEGER NOT NULL,
        toward_empire INTEGER NOT NULL,
        relation INTEGER NOT NULL,
        PRIMARY KEY(snapshot_id, record_index, toward_empire)
    );";

pub(super) fn write<'a>(
    tx: &Transaction<'_>,
    snapshot_id: i64,
    records: impl IntoIterator<Item = (usize, &'a Player)> + Clone,
) -> Result<(), CampaignStoreError> {
    write_rows(tx, snapshot_id, records.clone())?;
    write_diplomacy(tx, snapshot_id, records)
}

pub(super) fn write_rows<'a>(
    tx: &Transaction<'_>,
    snapshot_id: i64,
    records: impl IntoIterator<Item = (usize, &'a Player)>,
) -> Result<(), CampaignStoreError> {
    for (index, player) in records {
        let record_index = index + 1;
        insert_row(
            tx,
            "snapshot_players",
            named_params! {
                ":snapshot_id": snapshot_id,
                ":record_index": record_index,
                ":mode": player.mode.code(),
                ":handle": player.handle,
                ":page_lines": player.page_lines,
                ":empire_name": player.empire_name,
                ":messages_first": player.messages_first,
                ":messages_last": player.messages_last,
                ":results_first": player.results_first,
                ":results_last": player.results_last,
                ":fleets_first": player.fleets_first.0,
                ":fleets_last": player.fleets_last.0,
                ":bases_first": player.bases_first.0,
                ":bases_last": player.bases_last.0,
                ":ipbms_first": player.ipbms_first.0,
                ":ipbms_last": player.ipbms_last.0,
                ":homeworld": player.homeworld,
                ":last_year_on": player.last_year_on,
                ":planet_count": player.planet_count,
                ":tax_rate": player.tax_rate,
                ":production": player.production,
                ":autopilot": player.autopilot,
            },
        )?;
    }
    Ok(())
}

pub(super) fn write_diplomacy<'a>(
    tx: &Transaction<'_>,
    snapshot_id: i64,
    records: impl IntoIterator<Item = (usize, &'a Player)>,
) -> Result<(), CampaignStoreError> {
    for (index, player) in records {
        let record_index = index + 1;
        for (slot, relation) in player.diplomacy.iter().enumerate() {
            insert_row(
                tx,
                "snapshot_player_diplomacy",
                named_params! {
                    ":snapshot_id": snapshot_id,
                    ":record_index": record_index,
                    ":toward_empire": slot + 1,
                    ":relation": relation.code(),
                },
            )?;
        }
    }
    Ok(())
}

pub(super) fn load(conn: &Connection, snapshot_id: i64) -> Result<Vec<Player>, CampaignStoreError> {
    let mut stmt = conn
        .prepare("SELECT * FROM snapshot_players WHERE snapshot_id = ?1 ORDER BY record_index")?;
    let mut records = stmt
        .query_map(params![snapshot_id], player_from_row)?
        .collect::<Result<Vec<_>, _>>()?;

    let mut stmt = conn.prepare(
        "SELECT record_index, toward_empire, relation
         FROM snapshot_player_diplomacy
         WHERE snapshot_id = ?1",
    )?;
    let rows = stmt.query_map(params![snapshot_id], |row| {
        Ok((
            row.get::<_, usize>("record_index")?,
            row.get::<_, usize>("toward_empire")?,
            row.get::<_, u8>("relation")?,
        ))
    })?;
    for row in rows {
        let (record_index, toward_empire, relation) = row?;
        let record = position_of(record_index, records.len(), "player record")?;
        let slot = position_of(toward_empire, 25, "diplomacy empire")?;
        records[record].diplomacy[slot] = DiplomaticRelation::from_code(relation);
    }
    Ok(records)
}

fn player_from_row(row: &Row<'_>) -> rusqlite::Result<Player> {
    Ok(Player {
        mode: PlayerMode::from_code(row.get("mode")?),
        handle: row.get("handle")?,
        page_lines: row.get("page_lines")?,
        empire_name: row.get("empire_name")?,
        messages_first: row.get("messages_first")?,
        messages_last: row.get("messages_last")?,
        results_first: row.get("results_first")?,
        results_last: row.get("results_last")?,
        fleets_first: RecordNumber(row.get("fleets_first")?),
        fleets_last: RecordNumber(row.get("fleets_last")?),
        bases_first: RecordNumber(row.get("bases_first")?),
        bases_last: RecordNumber(row.get("bases_last")?),
        ipbms_first: RecordNumber(row.get("ipbms_first")?),
        ipbms_last: RecordNumber(row.get("ipbms_last")?),
        homeworld: row.get("homeworld")?,
        last_year_on: row.get("last_year_on")?,
        planet_count: row.get("planet_count")?,
        tax_rate: row.get("tax_rate")?,
        production: row.get("production")?,
        diplomacy: [DiplomaticRelation::Neutral; 25],
        autopilot: row.get("autopilot")?,
    })
}
