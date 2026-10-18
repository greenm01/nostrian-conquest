use rusqlite::{Connection, params};

use super::hex::{decode_hex, encode_hex};
use super::{CampaignStore, CampaignStoreError, ReportBlockRow};

impl CampaignStore {
    pub fn load_snapshot_report_block_rows(
        &self,
        snapshot_id: i64,
        include_deleted: bool,
    ) -> Result<Vec<ReportBlockRow>, CampaignStoreError> {
        let mut conn = self.connection()?;
        load_report_block_rows_filtered(&mut conn, snapshot_id, include_deleted)
    }

    /// Check whether any active (non-deleted) report blocks exist.
    pub fn has_active_report_blocks(&self, snapshot_id: i64) -> Result<bool, CampaignStoreError> {
        let conn = self.connection()?;
        let exists: bool = conn.query_row(
            "SELECT EXISTS(
                 SELECT 1 FROM report_blocks
                 WHERE snapshot_id = ?1 AND recipient_deleted = 0
             )",
            params![snapshot_id],
            |row| row.get(0),
        )?;
        Ok(exists)
    }

    /// Load the active (non-deleted) report block rows for the latest snapshot.
    pub fn load_latest_report_block_rows(&self) -> Result<Vec<ReportBlockRow>, CampaignStoreError> {
        let mut conn = self.connection()?;
        let Some((snapshot_id, _)) = super::metadata::latest_snapshot_id_and_year(&mut conn)?
        else {
            return Ok(Vec::new());
        };
        load_report_block_rows(&mut conn, snapshot_id)
    }
}

pub(super) fn write_report_block_rows<'a>(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    rows: impl IntoIterator<Item = &'a ReportBlockRow>,
) -> Result<(), CampaignStoreError> {
    let mut stmt = tx.prepare(
        "INSERT INTO report_blocks(snapshot_id, viewer_empire_id, block_index, decoded_text, raw_hex, recipient_deleted)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    for row in rows {
        stmt.execute(params![
            snapshot_id,
            i64::from(row.viewer_empire_id),
            row.block_index as i64,
            &row.decoded_text,
            row.raw_bytes.as_ref().map(|bytes| encode_hex(bytes)),
            i64::from(u8::from(row.recipient_deleted)),
        ])?;
    }
    Ok(())
}

pub(super) fn load_report_block_rows(
    conn: &Connection,
    snapshot_id: i64,
) -> Result<Vec<ReportBlockRow>, CampaignStoreError> {
    load_report_block_rows_filtered(conn, snapshot_id, false)
}

pub(super) fn load_report_block_rows_filtered(
    conn: &Connection,
    snapshot_id: i64,
    include_deleted: bool,
) -> Result<Vec<ReportBlockRow>, CampaignStoreError> {
    let sql = if include_deleted {
        "SELECT viewer_empire_id, block_index, decoded_text, raw_hex, recipient_deleted
         FROM report_blocks WHERE snapshot_id = ?1
         ORDER BY viewer_empire_id, block_index"
    } else {
        "SELECT viewer_empire_id, block_index, decoded_text, raw_hex, recipient_deleted
         FROM report_blocks WHERE snapshot_id = ?1 AND recipient_deleted = 0
         ORDER BY viewer_empire_id, block_index"
    };
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![snapshot_id], |row| {
        Ok(ReportBlockRow {
            viewer_empire_id: row.get::<_, i64>(0)? as u8,
            block_index: row.get::<_, i64>(1)? as usize,
            decoded_text: row.get(2)?,
            raw_bytes: row
                .get::<_, Option<String>>(3)?
                .map(|hex| decode_hex(&hex))
                .transpose()
                .map_err(|err| {
                    rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, err)),
                    )
                })?,
            recipient_deleted: row.get::<_, i64>(4)? != 0,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Rewrite, in snapshot `snapshot_id`, only the report rows that differ
/// between `before` (what the snapshot holds) and `after`, each row named by
/// its viewer and block index (t204).
pub(super) fn write_changed_report_block_rows(
    tx: &rusqlite::Transaction<'_>,
    snapshot_id: i64,
    before: &[ReportBlockRow],
    after: &[ReportBlockRow],
) -> Result<(), CampaignStoreError> {
    let key = |row: &ReportBlockRow| (row.viewer_empire_id, row.block_index);
    let before: std::collections::BTreeMap<_, _> =
        before.iter().map(|row| (key(row), row)).collect();
    let after_keys: std::collections::BTreeSet<_> = after.iter().map(key).collect();
    let changed: Vec<&ReportBlockRow> = after
        .iter()
        .filter(|row| before.get(&key(row)) != Some(row))
        .collect();
    let stale = before
        .keys()
        .filter(|row_key| !after_keys.contains(*row_key))
        .copied()
        .chain(
            changed
                .iter()
                .map(|row| key(row))
                .filter(|row_key| before.contains_key(row_key)),
        );
    for (viewer_empire_id, block_index) in stale.collect::<Vec<_>>() {
        tx.execute(
            "DELETE FROM report_blocks
             WHERE snapshot_id = ?1 AND viewer_empire_id = ?2 AND block_index = ?3",
            params![snapshot_id, i64::from(viewer_empire_id), block_index as i64],
        )?;
    }
    write_report_block_rows(tx, snapshot_id, changed)
}
