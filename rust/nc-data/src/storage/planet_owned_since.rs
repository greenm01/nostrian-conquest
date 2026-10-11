use std::collections::BTreeMap;

use rusqlite::{Connection, params};

use super::{CampaignStore, CampaignStoreError};
use crate::CoreGameData;

impl CampaignStore {
    pub fn latest_owned_planet_years_for_empire(
        &self,
        owner_empire_id: u8,
    ) -> Result<BTreeMap<usize, u16>, CampaignStoreError> {
        let mut conn = self.connection()?;
        let Some((snapshot_id, _)) = super::metadata::latest_snapshot_id_and_year(&mut conn)?
        else {
            return Ok(BTreeMap::new());
        };
        load_owned_planet_years_for_empire(&conn, snapshot_id, owner_empire_id)
    }
}

pub(super) fn load_owned_planet_years_for_empire(
    conn: &Connection,
    snapshot_id: i64,
    owner_empire_id: u8,
) -> Result<BTreeMap<usize, u16>, CampaignStoreError> {
    let mut rows = load_owned_planet_year_rows(conn, snapshot_id, Some(owner_empire_id))?;
    if rows.is_empty() {
        rows = derive_owned_planet_year_rows(conn, snapshot_id, Some(owner_empire_id))?;
    }
    Ok(rows
        .into_iter()
        .filter_map(|((empire_id, planet_record_index), acquired_year)| {
            (empire_id == owner_empire_id).then_some((planet_record_index, acquired_year))
        })
        .collect())
}

pub(super) fn write_owned_planet_year_rows(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    game_data: &CoreGameData,
    year: u16,
    previous_snapshot_id: Option<i64>,
) -> Result<(), CampaignStoreError> {
    let previous_rows = if let Some(previous_snapshot_id) = previous_snapshot_id {
        let rows = load_owned_planet_year_rows(tx, previous_snapshot_id, None)?;
        if rows.is_empty() {
            derive_owned_planet_year_rows(tx, previous_snapshot_id, None)?
        } else {
            rows
        }
    } else {
        BTreeMap::new()
    };

    let mut stmt = tx.prepare(
        "INSERT INTO planet_owned_since(
             snapshot_id, owner_empire_id, planet_record_index, acquired_year
         ) VALUES (?1, ?2, ?3, ?4)",
    )?;
    for (planet_idx, planet) in game_data.planets.iter().enumerate() {
        let owner_empire_id = planet.owner.0;
        if owner_empire_id == 0 {
            continue;
        }
        let planet_record_index = planet_idx + 1;
        let acquired_year = previous_rows
            .get(&(owner_empire_id, planet_record_index))
            .copied()
            .unwrap_or(year);
        stmt.execute(params![
            snapshot_id,
            i64::from(owner_empire_id),
            planet_record_index as i64,
            i64::from(acquired_year),
        ])?;
    }
    Ok(())
}

fn load_owned_planet_year_rows(
    conn: &Connection,
    snapshot_id: i64,
    owner_empire_id: Option<u8>,
) -> Result<BTreeMap<(u8, usize), u16>, CampaignStoreError> {
    let sql = if owner_empire_id.is_some() {
        "SELECT owner_empire_id, planet_record_index, acquired_year
         FROM planet_owned_since
         WHERE snapshot_id = ?1 AND owner_empire_id = ?2
         ORDER BY owner_empire_id, planet_record_index"
    } else {
        "SELECT owner_empire_id, planet_record_index, acquired_year
         FROM planet_owned_since
         WHERE snapshot_id = ?1
         ORDER BY owner_empire_id, planet_record_index"
    };
    let mut stmt = conn.prepare(sql)?;
    let rows = if let Some(owner_empire_id) = owner_empire_id {
        stmt.query_map(params![snapshot_id, i64::from(owner_empire_id)], |row| {
            Ok((
                (row.get::<_, i64>(0)? as u8, row.get::<_, i64>(1)? as usize),
                row.get::<_, i64>(2)? as u16,
            ))
        })?
        .collect::<Result<BTreeMap<_, _>, _>>()?
    } else {
        stmt.query_map(params![snapshot_id], |row| {
            Ok((
                (row.get::<_, i64>(0)? as u8, row.get::<_, i64>(1)? as usize),
                row.get::<_, i64>(2)? as u16,
            ))
        })?
        .collect::<Result<BTreeMap<_, _>, _>>()?
    };
    Ok(rows)
}

fn derive_owned_planet_year_rows(
    conn: &Connection,
    snapshot_id: i64,
    owner_empire_id: Option<u8>,
) -> Result<BTreeMap<(u8, usize), u16>, CampaignStoreError> {
    let sql = if owner_empire_id.is_some() {
        "SELECT snapshots.game_year, snapshot_planets.record_index, snapshot_planets.owner
         FROM snapshot_planets
         JOIN snapshots ON snapshots.id = snapshot_planets.snapshot_id
         WHERE snapshot_planets.snapshot_id <= ?1 AND snapshot_planets.owner IN (0, ?2)
         ORDER BY snapshots.game_year, snapshot_planets.snapshot_id, snapshot_planets.record_index"
    } else {
        "SELECT snapshots.game_year, snapshot_planets.record_index, snapshot_planets.owner
         FROM snapshot_planets
         JOIN snapshots ON snapshots.id = snapshot_planets.snapshot_id
         WHERE snapshot_planets.snapshot_id <= ?1
         ORDER BY snapshots.game_year, snapshot_planets.snapshot_id, snapshot_planets.record_index"
    };
    let mut stmt = conn.prepare(sql)?;
    let mut active = BTreeMap::<usize, (u8, u16)>::new();
    if let Some(owner_empire_id) = owner_empire_id {
        let rows = stmt.query_map(params![snapshot_id, i64::from(owner_empire_id)], |row| {
            Ok((
                row.get::<_, i64>(0)? as u16,
                row.get::<_, i64>(1)? as usize,
                row.get::<_, i64>(2)? as u8,
            ))
        })?;
        for row in rows {
            let (game_year, planet_record_index, current_owner) = row?;
            match active.get(&planet_record_index).copied() {
                Some((existing_owner, acquired_year)) if current_owner == existing_owner => {
                    active.insert(planet_record_index, (existing_owner, acquired_year));
                }
                _ if current_owner == 0 => {
                    active.remove(&planet_record_index);
                }
                _ => {
                    active.insert(planet_record_index, (current_owner, game_year));
                }
            }
        }
    } else {
        let rows = stmt.query_map(params![snapshot_id], |row| {
            Ok((
                row.get::<_, i64>(0)? as u16,
                row.get::<_, i64>(1)? as usize,
                row.get::<_, i64>(2)? as u8,
            ))
        })?;
        for row in rows {
            let (game_year, planet_record_index, current_owner) = row?;
            match active.get(&planet_record_index).copied() {
                Some((existing_owner, acquired_year)) if current_owner == existing_owner => {
                    active.insert(planet_record_index, (existing_owner, acquired_year));
                }
                _ if current_owner == 0 => {
                    active.remove(&planet_record_index);
                }
                _ => {
                    active.insert(planet_record_index, (current_owner, game_year));
                }
            }
        }
    }

    Ok(active
        .into_iter()
        .map(|(planet_record_index, (empire_id, acquired_year))| {
            ((empire_id, planet_record_index), acquired_year)
        })
        .collect())
}
