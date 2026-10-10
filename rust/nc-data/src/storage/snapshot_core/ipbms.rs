use rusqlite::{Connection, Row, Transaction, named_params, params};

use super::{CampaignStoreError, insert_row, real48_at, real48_column};
use crate::model::{Coords, EmpireId, Ipbm, RecordNumber};
use crate::{IpbmDat, IpbmRecord};

/// One row per IPBM.DAT record.
pub(super) const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS snapshot_ipbms (
        snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
        record_index INTEGER NOT NULL,
        ipbm_number INTEGER NOT NULL,
        owner INTEGER NOT NULL,
        next_record INTEGER NOT NULL,
        record_number INTEGER NOT NULL,
        prev_record INTEGER NOT NULL,
        sector_x INTEGER NOT NULL,
        sector_y INTEGER NOT NULL,
        x_fraction_real48 INTEGER NOT NULL,
        y_fraction_real48 INTEGER NOT NULL,
        depth_real48 INTEGER NOT NULL,
        target_x INTEGER NOT NULL,
        target_y INTEGER NOT NULL,
        target_depth INTEGER NOT NULL,
        PRIMARY KEY(snapshot_id, record_index)
    );";

pub(super) fn write(
    tx: &Transaction<'_>,
    snapshot_id: i64,
    data: &IpbmDat,
) -> Result<(), CampaignStoreError> {
    for (index, record) in data.records.iter().enumerate() {
        let ipbm = &record.ipbm;
        insert_row(
            tx,
            "snapshot_ipbms",
            named_params! {
                ":snapshot_id": snapshot_id,
                ":record_index": index + 1,
                ":ipbm_number": ipbm.ipbm_number,
                ":owner": ipbm.owner.0,
                ":next_record": ipbm.next.0,
                ":record_number": ipbm.record.0,
                ":prev_record": ipbm.prev.0,
                ":sector_x": ipbm.sector.x,
                ":sector_y": ipbm.sector.y,
                ":x_fraction_real48": real48_column(ipbm.x_fraction),
                ":y_fraction_real48": real48_column(ipbm.y_fraction),
                ":depth_real48": real48_column(ipbm.depth),
                ":target_x": ipbm.target.x,
                ":target_y": ipbm.target.y,
                ":target_depth": ipbm.target_depth,
            },
        )?;
    }
    Ok(())
}

pub(super) fn load(conn: &mut Connection, snapshot_id: i64) -> Result<IpbmDat, CampaignStoreError> {
    let mut stmt =
        conn.prepare("SELECT * FROM snapshot_ipbms WHERE snapshot_id = ?1 ORDER BY record_index")?;
    let records = stmt
        .query_map(params![snapshot_id], ipbm_from_row)?
        .map(|ipbm| ipbm.map(|ipbm| IpbmRecord { ipbm }))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(IpbmDat { records })
}

fn ipbm_from_row(row: &Row<'_>) -> rusqlite::Result<Ipbm> {
    Ok(Ipbm {
        ipbm_number: row.get("ipbm_number")?,
        owner: EmpireId(row.get("owner")?),
        next: RecordNumber(row.get("next_record")?),
        record: RecordNumber(row.get("record_number")?),
        prev: RecordNumber(row.get("prev_record")?),
        sector: Coords::new(row.get("sector_x")?, row.get("sector_y")?),
        x_fraction: real48_at(row, "x_fraction_real48")?,
        y_fraction: real48_at(row, "y_fraction_real48")?,
        depth: real48_at(row, "depth_real48")?,
        target: Coords::new(row.get("target_x")?, row.get("target_y")?),
        target_depth: row.get("target_depth")?,
    })
}
