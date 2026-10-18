//! The crippled hulls of fleets and starbases: one row per class and
//! turns-left value that has any, in a child table of each record's table.
//! `turns_left` 1 to `RECOVERY_TURNS` is the hulls still recovering; 0 is the
//! hulls crippled during the current pass.

use rusqlite::{Connection, Transaction, named_params, params};

use super::{CampaignStoreError, insert_row, position_of};
use crate::model::{CrippledHulls, Fleet, HullClass, RECOVERY_TURNS, Starbase};

/// `class` is [`HullClass::code`]; a base's one hull has no class column.
pub(super) const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS snapshot_fleet_crippled (
        snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
        record_index INTEGER NOT NULL,
        class INTEGER NOT NULL,
        turns_left INTEGER NOT NULL,
        hulls INTEGER NOT NULL,
        PRIMARY KEY(snapshot_id, record_index, class, turns_left)
    );
    CREATE TABLE IF NOT EXISTS snapshot_base_crippled (
        snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
        record_index INTEGER NOT NULL,
        turns_left INTEGER NOT NULL,
        hulls INTEGER NOT NULL,
        PRIMARY KEY(snapshot_id, record_index, turns_left)
    );";

/// The non-empty counts of `hulls` as `(turns_left, hulls)`.
fn rows(hulls: &CrippledHulls) -> impl Iterator<Item = (usize, u32)> {
    std::iter::once((0, hulls.fresh))
        .chain(hulls.turns.into_iter().enumerate().map(|(i, n)| (i + 1, n)))
        .filter(|&(_, count)| count > 0)
}

fn set(hulls: &mut CrippledHulls, turns_left: usize, count: u32) -> Result<(), CampaignStoreError> {
    match turns_left {
        0 => hulls.fresh = count,
        1..=RECOVERY_TURNS => hulls.turns[turns_left - 1] = count,
        _ => {
            return Err(CampaignStoreError::InvalidState(format!(
                "crippled turns left {turns_left} is outside 0..={RECOVERY_TURNS}"
            )));
        }
    }
    Ok(())
}

pub(super) fn write_fleets<'a>(
    tx: &Transaction<'_>,
    snapshot_id: i64,
    records: impl IntoIterator<Item = (usize, &'a Fleet)>,
) -> Result<(), CampaignStoreError> {
    for (index, fleet) in records {
        for class in HullClass::ALL {
            for (turns_left, hulls) in rows(fleet.crippled.get(class)) {
                insert_row(
                    tx,
                    "snapshot_fleet_crippled",
                    named_params! {
                        ":snapshot_id": snapshot_id,
                        ":record_index": index + 1,
                        ":class": class.code(),
                        ":turns_left": turns_left,
                        ":hulls": hulls,
                    },
                )?;
            }
        }
    }
    Ok(())
}

pub(super) fn write_bases<'a>(
    tx: &Transaction<'_>,
    snapshot_id: i64,
    records: impl IntoIterator<Item = (usize, &'a Starbase)>,
) -> Result<(), CampaignStoreError> {
    for (index, base) in records {
        for (turns_left, hulls) in rows(&base.crippled) {
            insert_row(
                tx,
                "snapshot_base_crippled",
                named_params! {
                    ":snapshot_id": snapshot_id,
                    ":record_index": index + 1,
                    ":turns_left": turns_left,
                    ":hulls": hulls,
                },
            )?;
        }
    }
    Ok(())
}

pub(super) fn load_fleets(
    conn: &Connection,
    snapshot_id: i64,
    fleets: &mut [Fleet],
) -> Result<(), CampaignStoreError> {
    let mut stmt = conn.prepare(
        "SELECT record_index, class, turns_left, hulls
         FROM snapshot_fleet_crippled
         WHERE snapshot_id = ?1",
    )?;
    let rows = stmt.query_map(params![snapshot_id], |row| {
        Ok((
            row.get::<_, usize>("record_index")?,
            row.get::<_, u8>("class")?,
            row.get::<_, usize>("turns_left")?,
            row.get::<_, u32>("hulls")?,
        ))
    })?;
    for row in rows {
        let (record_index, class, turns_left, hulls) = row?;
        let record = position_of(record_index, fleets.len(), "fleet record")?;
        let class = HullClass::from_code(class).ok_or_else(|| {
            CampaignStoreError::InvalidState(format!("crippled ship class {class} is unknown"))
        })?;
        set(fleets[record].crippled.get_mut(class), turns_left, hulls)?;
    }
    Ok(())
}

pub(super) fn load_bases(
    conn: &Connection,
    snapshot_id: i64,
    bases: &mut [Starbase],
) -> Result<(), CampaignStoreError> {
    let mut stmt = conn.prepare(
        "SELECT record_index, turns_left, hulls
         FROM snapshot_base_crippled
         WHERE snapshot_id = ?1",
    )?;
    let rows = stmt.query_map(params![snapshot_id], |row| {
        Ok((
            row.get::<_, usize>("record_index")?,
            row.get::<_, usize>("turns_left")?,
            row.get::<_, u32>("hulls")?,
        ))
    })?;
    for row in rows {
        let (record_index, turns_left, hulls) = row?;
        let record = position_of(record_index, bases.len(), "starbase record")?;
        set(&mut bases[record].crippled, turns_left, hulls)?;
    }
    Ok(())
}
