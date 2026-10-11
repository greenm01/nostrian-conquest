//! Local sighting memory: one row per viewing empire, sector, and layer
//! that remembers an opponent, with the opponents as a bit mask.

use rusqlite::{Connection, Transaction, named_params, params};

use super::{CampaignStoreError, insert_row};
use crate::model::{Coords, Sighting, SightingMemory};

pub(super) const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS snapshot_sightings (
        snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
        viewer INTEGER NOT NULL,
        sector_x INTEGER NOT NULL,
        sector_y INTEGER NOT NULL,
        layer INTEGER NOT NULL,
        opponents INTEGER NOT NULL,
        PRIMARY KEY(snapshot_id, viewer, sector_x, sector_y, layer)
    );";

pub(super) fn write(
    tx: &Transaction<'_>,
    snapshot_id: i64,
    sightings: &SightingMemory,
) -> Result<(), CampaignStoreError> {
    for sighting in sightings.iter() {
        insert_row(
            tx,
            "snapshot_sightings",
            named_params! {
                ":snapshot_id": snapshot_id,
                ":viewer": sighting.viewer,
                ":sector_x": sighting.sector.x,
                ":sector_y": sighting.sector.y,
                ":layer": sighting.layer,
                ":opponents": sighting.opponents,
            },
        )?;
    }
    Ok(())
}

pub(super) fn load(
    conn: &mut Connection,
    snapshot_id: i64,
) -> Result<SightingMemory, CampaignStoreError> {
    let mut stmt = conn.prepare(
        "SELECT viewer, sector_x, sector_y, layer, opponents
         FROM snapshot_sightings
         WHERE snapshot_id = ?1",
    )?;
    let rows = stmt.query_map(params![snapshot_id], |row| {
        Ok(Sighting {
            viewer: row.get("viewer")?,
            sector: Coords::new(row.get("sector_x")?, row.get("sector_y")?),
            layer: row.get("layer")?,
            opponents: row.get("opponents")?,
        })
    })?;
    let mut memory = SightingMemory::default();
    for row in rows {
        memory.set(row?);
    }
    Ok(memory)
}
