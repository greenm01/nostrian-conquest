//! How the hosted services open their SQLite files (t205): the campaign
//! store, `hosted.db`, `roster.db` and `.web-sessions.db`. Several processes
//! (the bridge, the bots, the admin timers and one door per player) open the
//! same files at once.

use std::path::Path;
use std::time::Duration;

use rusqlite::{Connection, Transaction, TransactionBehavior};

/// How long a connection waits for another process's write to finish.
pub const BUSY_TIMEOUT: Duration = Duration::from_secs(30);

/// How often a waiting connection tries the lock again. SQLite's own wait
/// backs off to 100 ms between tries, so while others keep taking the lock
/// one writer can wait seconds; trying every millisecond keeps the wait
/// close to the time the writers ahead of it take (t205 soak).
const BUSY_RETRY: Duration = Duration::from_millis(1);

fn wait_for_lock(attempts: i32) -> bool {
    if u128::try_from(attempts).unwrap_or(0) * BUSY_RETRY.as_micros() >= BUSY_TIMEOUT.as_micros() {
        return false;
    }
    std::thread::sleep(BUSY_RETRY);
    true
}

/// A connection with foreign keys enforced that waits up to [`BUSY_TIMEOUT`]
/// for another process's write. It leaves
/// the file's journal mode as it is, so opening a campaign only to read it
/// changes nothing.
pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.busy_handler(Some(wait_for_lock))?;
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

/// A write transaction on a shared connection, taking the write lock at once
/// (`BEGIN IMMEDIATE`), so a transaction that reads before it writes waits
/// for other writers instead of failing when it comes to write.
pub fn write_transaction(conn: &Connection) -> rusqlite::Result<Transaction<'_>> {
    Transaction::new_unchecked(conn, TransactionBehavior::Immediate)
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
