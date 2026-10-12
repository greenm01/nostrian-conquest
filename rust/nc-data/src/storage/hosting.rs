//! Gates and receipts shared by managed doors and their maintenance runner.
use super::{CampaignStore, CampaignStoreError};
use rusqlite::{OptionalExtension, params};

pub(super) fn writes_blocked(conn: &rusqlite::Connection) -> Result<bool, CampaignStoreError> {
    Ok(conn
        .query_row(
            "SELECT int_value FROM campaign_metadata WHERE key='hosting_write_blocked'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .optional()?
        .unwrap_or(0)
        != 0)
}

impl CampaignStore {
    pub fn set_hosting_write_blocked(&self, blocked: bool) -> Result<(), CampaignStoreError> {
        self.connection()?.execute(
            "INSERT INTO campaign_metadata(key,int_value) VALUES ('hosting_write_blocked',?1)
             ON CONFLICT(key) DO UPDATE SET int_value=excluded.int_value",
            [i64::from(blocked)],
        )?;
        Ok(())
    }

    pub fn hosting_writes_blocked(&self) -> Result<bool, CampaignStoreError> {
        writes_blocked(&self.connection()?)
    }

    /// A successful save records the receipt in the same transaction as the
    /// new year. A repeated request cannot write a second snapshot.
    pub fn recording_maintenance(
        &self,
        request_id: &str,
        year: u16,
    ) -> Result<Self, CampaignStoreError> {
        self.connection()?.execute_batch(
            "CREATE TABLE IF NOT EXISTS maintenance_receipts (
             request_id TEXT PRIMARY KEY, expected_year INTEGER NOT NULL, end_year INTEGER NOT NULL);"
        )?;
        let mut store = self.clone();
        store.maintenance_receipt = Some((request_id.to_owned(), year));
        Ok(store)
    }

    pub fn maintenance_receipt(
        &self,
        request_id: &str,
    ) -> Result<Option<(u16, u16)>, CampaignStoreError> {
        let conn = self.connection()?;
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='maintenance_receipts')",
            [], |r| r.get(0),
        )?;
        if !exists {
            return Ok(None);
        }
        Ok(conn
            .query_row(
                "SELECT expected_year,end_year FROM maintenance_receipts WHERE request_id=?1",
                params![request_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?)
    }
}
