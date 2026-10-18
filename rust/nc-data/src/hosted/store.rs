use rusqlite::{Connection, Result as SqliteResult};
use std::path::Path;

use super::schema::INIT_SQL;

pub struct HostedStore {
    conn: Connection,
}

impl HostedStore {
    pub fn open(path: &Path) -> SqliteResult<Self> {
        let mut conn = crate::sqlite::open_for_writes(path)?;
        initialize(&mut conn)?;
        Ok(HostedStore { conn })
    }

    pub fn create(path: &Path) -> SqliteResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                rusqlite::Error::InvalidPath(std::path::PathBuf::from(format!(
                    "failed to create parent directory: {}",
                    e
                )))
            })?;
        }
        let mut conn = crate::sqlite::open_for_writes(path)?;
        initialize(&mut conn)?;
        Ok(HostedStore { conn })
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}

/// Create the schema in one write transaction (t205), so processes opening
/// the file at once cannot race, and a failure is reported rather than
/// mistaken for a schema already there.
fn initialize(conn: &mut Connection) -> SqliteResult<()> {
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    tx.execute_batch(INIT_SQL)?;
    tx.commit()
}
