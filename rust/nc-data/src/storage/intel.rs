use std::collections::BTreeMap;

use rusqlite::{Connection, Row, params};

use super::{CampaignStore, CampaignStoreError, IntelTier, PlanetIntelSnapshot};
use crate::{CoreGameData, merge_player_intel_from_runtime};

impl CampaignStore {
    pub fn latest_planet_intel_for_viewer(
        &self,
        viewer_empire_id: u8,
    ) -> Result<Vec<PlanetIntelSnapshot>, CampaignStoreError> {
        let mut conn = self.connection()?;
        let Some((snapshot_id, _)) = super::metadata::latest_snapshot_id_and_year(&mut conn)?
        else {
            return Ok(Vec::new());
        };
        load_planet_intel_rows_for_viewer(&mut conn, snapshot_id, viewer_empire_id)
    }

    pub fn load_snapshot_planet_intel_by_viewer(
        &self,
        snapshot_id: i64,
        player_count: u8,
    ) -> Result<Vec<BTreeMap<usize, PlanetIntelSnapshot>>, CampaignStoreError> {
        let mut conn = self.connection()?;
        load_planet_intel_by_viewer(&mut conn, snapshot_id, player_count)
    }
}

pub(super) fn load_planet_intel_by_viewer(
    conn: &Connection,
    snapshot_id: i64,
    player_count: u8,
) -> Result<Vec<BTreeMap<usize, PlanetIntelSnapshot>>, CampaignStoreError> {
    (1..=player_count)
        .map(|viewer_empire_id| {
            Ok(
                load_planet_intel_rows_for_viewer(conn, snapshot_id, viewer_empire_id)?
                    .into_iter()
                    .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                    .collect(),
            )
        })
        .collect()
}

fn planet_intel_snapshot_from_row(
    row: &Row<'_>,
    index_offset: usize,
) -> rusqlite::Result<PlanetIntelSnapshot> {
    Ok(PlanetIntelSnapshot {
        planet_record_index_1_based: row.get::<_, i64>(index_offset)? as usize,
        intel_tier: IntelTier::from_str(&row.get::<_, String>(index_offset + 1)?),
        compat_is_orbit_seed: row.get::<_, i64>(index_offset + 2)? != 0,
        last_intel_year: row
            .get::<_, Option<i64>>(index_offset + 3)?
            .map(|value| value as u16),
        seen_year: row
            .get::<_, Option<i64>>(index_offset + 4)?
            .map(|value| value as u16),
        scout_year: row
            .get::<_, Option<i64>>(index_offset + 5)?
            .map(|value| value as u16),
        known_name: row.get(index_offset + 6)?,
        known_owner_empire_id: row
            .get::<_, Option<i64>>(index_offset + 7)?
            .map(|value| value as u8),
        known_potential_production: row
            .get::<_, Option<i64>>(index_offset + 8)?
            .map(|value| value as u16),
        known_armies: row
            .get::<_, Option<i64>>(index_offset + 9)?
            .map(|value| value as u8),
        known_ground_batteries: row
            .get::<_, Option<i64>>(index_offset + 10)?
            .map(|value| value as u8),
        known_starbase_count: row
            .get::<_, Option<i64>>(index_offset + 11)?
            .map(|value| value as u8),
        known_current_production: row
            .get::<_, Option<i64>>(index_offset + 12)?
            .map(|value| value as u8),
        known_stored_points: row
            .get::<_, Option<i64>>(index_offset + 13)?
            .map(|value| value as u16),
        known_docked_summary: row.get(index_offset + 14)?,
        known_orbit_summary: row.get(index_offset + 15)?,
        compat_word_1e: row
            .get::<_, Option<i64>>(index_offset + 16)?
            .map(|value| value as u16),
    })
}

pub(super) fn load_planet_intel_rows_for_viewer(
    conn: &Connection,
    snapshot_id: i64,
    viewer_empire_id: u8,
) -> Result<Vec<PlanetIntelSnapshot>, CampaignStoreError> {
    let mut stmt = conn.prepare(
        "SELECT planet_record_index, intel_tier, compat_is_orbit_seed, last_intel_year,
                seen_year, scout_year,
                known_name, known_owner_empire_id, known_potential_production,
                known_armies, known_ground_batteries, known_starbase_count,
                known_current_production, known_stored_points,
                known_docked_summary, known_orbit_summary, compat_word_1e
         FROM planet_intel
         WHERE snapshot_id = ?1 AND viewer_empire_id = ?2
         ORDER BY planet_record_index",
    )?;
    let rows = stmt.query_map(params![snapshot_id, i64::from(viewer_empire_id)], |row| {
        planet_intel_snapshot_from_row(row, 0)
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub(super) fn load_intel_rows(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
) -> Result<BTreeMap<(u8, usize), PlanetIntelSnapshot>, CampaignStoreError> {
    let mut stmt = tx.prepare(
        "SELECT viewer_empire_id, planet_record_index, intel_tier, compat_is_orbit_seed, last_intel_year,
                seen_year, scout_year,
                known_name, known_owner_empire_id, known_potential_production,
                known_armies, known_ground_batteries, known_starbase_count,
                known_current_production, known_stored_points,
                known_docked_summary, known_orbit_summary, compat_word_1e
         FROM planet_intel
         WHERE snapshot_id = ?1",
    )?;
    let rows = stmt.query_map(params![snapshot_id], |row| {
        Ok((
            (row.get::<_, i64>(0)? as u8, row.get::<_, i64>(1)? as usize),
            planet_intel_snapshot_from_row(row, 1)?,
        ))
    })?;
    Ok(rows.collect::<Result<BTreeMap<_, _>, _>>()?)
}

pub(super) fn write_planet_intel_rows(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    game_data: &CoreGameData,
    year: u16,
    current_rows_override: Option<&[BTreeMap<usize, PlanetIntelSnapshot>]>,
    previous: &BTreeMap<(u8, usize), PlanetIntelSnapshot>,
) -> Result<(), CampaignStoreError> {
    let player_count = game_data.conquest.player_count;
    let mut stmt = tx.prepare(
        "INSERT INTO planet_intel(
             snapshot_id, viewer_empire_id, planet_record_index, intel_tier, last_intel_year,
             compat_is_orbit_seed, seen_year, scout_year,
             known_name, known_owner_empire_id, known_potential_production,
             known_armies, known_ground_batteries, known_starbase_count,
             known_current_production, known_stored_points,
             known_docked_summary, known_orbit_summary, compat_word_1e
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
    )?;
    for viewer_empire_id in 1..=player_count {
        let previous_rows = previous
            .iter()
            .filter_map(|((empire_id, planet_record_index), snapshot)| {
                (*empire_id == viewer_empire_id).then_some((*planet_record_index, snapshot.clone()))
            })
            .collect::<BTreeMap<_, _>>();
        let merged_rows = current_rows_override
            .and_then(|rows| rows.get(viewer_empire_id.saturating_sub(1) as usize))
            .cloned()
            .unwrap_or_else(|| {
                merge_player_intel_from_runtime(
                    game_data,
                    viewer_empire_id,
                    year,
                    Some(&previous_rows),
                    None,
                )
            });
        for (planet_record_index_1_based, snapshot) in merged_rows {
            stmt.execute(params![
                snapshot_id,
                i64::from(viewer_empire_id),
                planet_record_index_1_based as i64,
                snapshot.intel_tier.as_str(),
                snapshot.last_intel_year.map(i64::from),
                i64::from(u8::from(snapshot.compat_is_orbit_seed)),
                snapshot.seen_year.map(i64::from),
                snapshot.scout_year.map(i64::from),
                snapshot.known_name,
                snapshot.known_owner_empire_id.map(i64::from),
                snapshot.known_potential_production.map(i64::from),
                snapshot.known_armies.map(i64::from),
                snapshot.known_ground_batteries.map(i64::from),
                snapshot.known_starbase_count.map(i64::from),
                snapshot.known_current_production.map(i64::from),
                snapshot.known_stored_points.map(i64::from),
                snapshot.known_docked_summary,
                snapshot.known_orbit_summary,
                snapshot.compat_word_1e.map(i64::from),
            ])?;
        }
    }
    Ok(())
}
