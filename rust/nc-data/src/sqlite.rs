//! How the hosted services open their SQLite files (t205): the campaign
//! store, `hosted.db`, `roster.db` and `.web-sessions.db`. Several processes
//! (the bridge, the bots, the admin timers and one door per player) open the
//! same files at once.

use std::path::Path;
use std::time::Duration;

use rusqlite::Connection;

/// How long a connection waits for another process's write to finish.
/// SQLite's wait is not fair, so a busy file can keep a writer waiting well
/// past rusqlite's default 5 s.
pub const BUSY_TIMEOUT: Duration = Duration::from_secs(30);

/// A connection with foreign keys enforced and [`BUSY_TIMEOUT`]. It leaves
/// the file's journal mode as it is, so opening a campaign only to read it
/// changes nothing.
pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.busy_timeout(BUSY_TIMEOUT)?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    Ok(conn)
}

/// [`open`], with the file put in WAL mode, so readers never block a writer
/// nor it them (the mode is stored in the file), and `synchronous=NORMAL`,
/// which WAL keeps safe against corruption.
pub fn open_for_writes(path: &Path) -> rusqlite::Result<Connection> {
    let conn = open(path)?;
    conn.query_row("PRAGMA journal_mode = WAL", [], |row| {
        row.get::<_, String>(0)
    })?;
    conn.execute_batch("PRAGMA synchronous = NORMAL;")?;
    Ok(conn)
}

/// Add `column` to `table` with `definition` unless it is already there.
/// Run inside the caller's write transaction, so two processes upgrading the
/// same file cannot both add it.
pub fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> rusqlite::Result<()> {
    let present: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM pragma_table_info(?1) WHERE name = ?2)",
        [table, column],
        |row| row.get(0),
    )?;
    if !present {
        conn.execute_batch(&format!(
            "ALTER TABLE {table} ADD COLUMN {column} {definition}"
        ))?;
    }
    Ok(())
}
