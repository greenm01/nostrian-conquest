use rusqlite::{Connection, Row, Transaction, named_params, params};

use super::{CampaignStoreError, insert_row, real48_at, real48_column};
use crate::model::{Coords, EmpireId, FleetId, RecordNumber, Starbase, StarbaseId};
use crate::{BaseDat, BaseRecord};

/// One row per BASES.DAT record.
pub(super) const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS snapshot_bases (
        snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
        record_index INTEGER NOT NULL,
        escort INTEGER NOT NULL,
        base_id INTEGER NOT NULL,
        owner INTEGER NOT NULL,
        next_record INTEGER NOT NULL,
        record_number INTEGER NOT NULL,
        prev_record INTEGER NOT NULL,
        sector_x INTEGER NOT NULL,
        sector_y INTEGER NOT NULL,
        x_fraction_real48 INTEGER NOT NULL,
        y_fraction_real48 INTEGER NOT NULL,
        depth_real48 INTEGER NOT NULL,
        moving INTEGER NOT NULL,
        target_x INTEGER NOT NULL,
        target_y INTEGER NOT NULL,
        target_depth INTEGER NOT NULL,
        PRIMARY KEY(snapshot_id, record_index)
    );";

pub(super) fn write(
    tx: &Transaction<'_>,
    snapshot_id: i64,
    data: &BaseDat,
) -> Result<(), CampaignStoreError> {
    for (index, record) in data.records.iter().enumerate() {
        let base = &record.base;
        insert_row(
            tx,
            "snapshot_bases",
            named_params! {
                ":snapshot_id": snapshot_id,
                ":record_index": index + 1,
                ":escort": base.escort.0,
                ":base_id": base.base_id.0,
                ":owner": base.owner.0,
                ":next_record": base.next.0,
                ":record_number": base.record.0,
                ":prev_record": base.prev.0,
                ":sector_x": base.sector.x,
                ":sector_y": base.sector.y,
                ":x_fraction_real48": real48_column(base.x_fraction),
                ":y_fraction_real48": real48_column(base.y_fraction),
                ":depth_real48": real48_column(base.depth),
                ":moving": base.moving,
                ":target_x": base.target.x,
                ":target_y": base.target.y,
                ":target_depth": base.target_depth,
            },
        )?;
    }
    Ok(())
}

pub(super) fn load(conn: &mut Connection, snapshot_id: i64) -> Result<BaseDat, CampaignStoreError> {
    let mut stmt =
        conn.prepare("SELECT * FROM snapshot_bases WHERE snapshot_id = ?1 ORDER BY record_index")?;
    let records = stmt
        .query_map(params![snapshot_id], base_from_row)?
        .map(|base| base.map(|base| BaseRecord { base }))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(BaseDat { records })
}

fn base_from_row(row: &Row<'_>) -> rusqlite::Result<Starbase> {
    Ok(Starbase {
        escort: FleetId(row.get("escort")?),
        base_id: StarbaseId(row.get("base_id")?),
        owner: EmpireId(row.get("owner")?),
        next: RecordNumber(row.get("next_record")?),
        record: RecordNumber(row.get("record_number")?),
        prev: RecordNumber(row.get("prev_record")?),
        sector: Coords::new(row.get("sector_x")?, row.get("sector_y")?),
        x_fraction: real48_at(row, "x_fraction_real48")?,
        y_fraction: real48_at(row, "y_fraction_real48")?,
        depth: real48_at(row, "depth_real48")?,
        moving: row.get("moving")?,
        target: Coords::new(row.get("target_x")?, row.get("target_y")?),
        target_depth: row.get("target_depth")?,
    })
}
