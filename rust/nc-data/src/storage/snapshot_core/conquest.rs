use rusqlite::{Connection, Transaction, named_params, params};

use super::{CampaignStoreError, index_below, insert_row, position_of};
use crate::ConquestDat;
use crate::model::Conquest;

/// The CONQUEST.DAT game clock. `snapshot_conquest_maintenance_days` holds
/// the schedule by `weekday`, 0 for Sunday to 6 for Saturday;
/// `snapshot_conquest_previous_rankings` holds each empire's production and
/// planet count at the start of the most recent pass, by empire number.
pub(super) const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS snapshot_conquest (
        snapshot_id INTEGER PRIMARY KEY REFERENCES snapshots(id) ON DELETE CASCADE,
        game_year INTEGER NOT NULL,
        player_count INTEGER NOT NULL
    );
    CREATE TABLE IF NOT EXISTS snapshot_conquest_maintenance_days (
        snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
        weekday INTEGER NOT NULL,
        scheduled INTEGER NOT NULL,
        PRIMARY KEY(snapshot_id, weekday)
    );
    CREATE TABLE IF NOT EXISTS snapshot_conquest_previous_rankings (
        snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
        empire INTEGER NOT NULL,
        production INTEGER NOT NULL,
        planet_count INTEGER NOT NULL,
        PRIMARY KEY(snapshot_id, empire)
    );";

pub(super) fn write(
    tx: &Transaction<'_>,
    snapshot_id: i64,
    data: &ConquestDat,
) -> Result<(), CampaignStoreError> {
    let conquest = &data.conquest;
    insert_row(
        tx,
        "snapshot_conquest",
        named_params! {
            ":snapshot_id": snapshot_id,
            ":game_year": conquest.game_year,
            ":player_count": conquest.player_count,
        },
    )?;
    for (weekday, scheduled) in conquest.maintenance_days.0.iter().enumerate() {
        insert_row(
            tx,
            "snapshot_conquest_maintenance_days",
            named_params! {
                ":snapshot_id": snapshot_id,
                ":weekday": weekday,
                ":scheduled": scheduled,
            },
        )?;
    }
    for (slot, (production, planet_count)) in conquest
        .previous_production
        .iter()
        .zip(&conquest.previous_planet_count)
        .enumerate()
    {
        insert_row(
            tx,
            "snapshot_conquest_previous_rankings",
            named_params! {
                ":snapshot_id": snapshot_id,
                ":empire": slot + 1,
                ":production": production,
                ":planet_count": planet_count,
            },
        )?;
    }
    Ok(())
}

pub(super) fn load(
    conn: &mut Connection,
    snapshot_id: i64,
) -> Result<ConquestDat, CampaignStoreError> {
    let mut conquest = conn.query_row(
        "SELECT game_year, player_count FROM snapshot_conquest WHERE snapshot_id = ?1",
        params![snapshot_id],
        |row| {
            Ok(Conquest {
                game_year: row.get("game_year")?,
                player_count: row.get("player_count")?,
                ..Conquest::default()
            })
        },
    )?;

    let mut stmt = conn.prepare(
        "SELECT weekday, scheduled
         FROM snapshot_conquest_maintenance_days
         WHERE snapshot_id = ?1",
    )?;
    let rows = stmt.query_map(params![snapshot_id], |row| {
        Ok((
            row.get::<_, usize>("weekday")?,
            row.get::<_, bool>("scheduled")?,
        ))
    })?;
    for row in rows {
        let (weekday, scheduled) = row?;
        let days = &mut conquest.maintenance_days.0;
        days[index_below(weekday, days.len(), "maintenance weekday")?] = scheduled;
    }

    let mut stmt = conn.prepare(
        "SELECT empire, production, planet_count
         FROM snapshot_conquest_previous_rankings
         WHERE snapshot_id = ?1",
    )?;
    let rows = stmt.query_map(params![snapshot_id], |row| {
        Ok((
            row.get::<_, usize>("empire")?,
            row.get::<_, u16>("production")?,
            row.get::<_, u8>("planet_count")?,
        ))
    })?;
    for row in rows {
        let (empire, production, planet_count) = row?;
        let slot = position_of(empire, conquest.previous_production.len(), "ranking empire")?;
        conquest.previous_production[slot] = production;
        conquest.previous_planet_count[slot] = planet_count;
    }
    Ok(ConquestDat { conquest })
}
