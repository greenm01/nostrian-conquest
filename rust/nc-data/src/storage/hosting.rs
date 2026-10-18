//! Gates and receipts shared by managed doors and their maintenance runner.
use super::{CampaignStore, CampaignStoreError};
use rusqlite::{OptionalExtension, params};

/// Whether players' changes are refused: the host paused the campaign, or a
/// maintenance run outside the host is under way (t205).
pub(super) fn writes_blocked(conn: &rusqlite::Connection) -> Result<bool, CampaignStoreError> {
    Ok(conn.query_row(
        "SELECT COALESCE(MAX(int_value), 0) FROM campaign_metadata
             WHERE key IN ('hosting_write_blocked', 'maintenance_running')",
        [],
        |r| r.get::<_, i64>(0),
    )? != 0)
}

fn set_flag(store: &CampaignStore, key: &str, on: bool) -> Result<(), CampaignStoreError> {
    store.connection()?.execute(
        "INSERT INTO campaign_metadata(key,int_value) VALUES (?1,?2)
         ON CONFLICT(key) DO UPDATE SET int_value=excluded.int_value",
        params![key, i64::from(on)],
    )?;
    Ok(())
}

/// A maintenance run by `nc-sysop maint` or `nc-cli maint` (t205). While it
/// lasts, players' changes are refused as paused, so the year it runs cannot
/// change under it and its save is never refused. It holds an OS lock on
/// `<campaign>.maintenance.lock`, which ends with the process, so only one
/// run goes at a time; a run that finds the block left by one that crashed
/// takes it over. Dropping the run lifts the block.
pub struct MaintenanceRun {
    store: CampaignStore,
    took_over: bool,
    _lock: std::fs::File,
}

impl MaintenanceRun {
    /// Whether the run found the block of an earlier run that stopped
    /// before lifting it, and took it over.
    pub fn took_over(&self) -> bool {
        self.took_over
    }

    /// The store to save the run's years through: its saves pass the block.
    pub fn store(&self) -> &CampaignStore {
        &self.store
    }
}

impl Drop for MaintenanceRun {
    fn drop(&mut self) {
        if let Err(err) = set_flag(&self.store, "maintenance_running", false) {
            eprintln!(
                "error: could not lift the maintenance block on {}: {err}",
                self.store.path.display()
            );
        }
    }
}

impl CampaignStore {
    pub fn set_hosting_write_blocked(&self, blocked: bool) -> Result<(), CampaignStoreError> {
        set_flag(self, "hosting_write_blocked", blocked)
    }

    /// Start a maintenance run outside the host; see [`MaintenanceRun`].
    pub fn begin_maintenance(&self) -> Result<MaintenanceRun, CampaignStoreError> {
        let lock_path = self.path.with_extension("maintenance.lock");
        let io = |source| CampaignStoreError::Io {
            path: lock_path.clone(),
            source,
        };
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&lock_path)
            .map_err(io)?;
        match lock.try_lock() {
            Ok(()) => {}
            Err(std::fs::TryLockError::WouldBlock) => {
                return Err(CampaignStoreError::InvalidState(
                    "maintenance is already running for this campaign".into(),
                ));
            }
            Err(std::fs::TryLockError::Error(err)) => return Err(io(err)),
        }
        let took_over = self.maintenance_was_interrupted()?;
        set_flag(self, "maintenance_running", true)?;
        let mut store = self.clone();
        store.in_maintenance_run = true;
        Ok(MaintenanceRun {
            store,
            took_over,
            _lock: lock,
        })
    }

    fn maintenance_was_interrupted(&self) -> Result<bool, CampaignStoreError> {
        Ok(self
            .connection()?
            .query_row(
                "SELECT int_value FROM campaign_metadata WHERE key='maintenance_running'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .optional()?
            .unwrap_or(0)
            != 0)
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
