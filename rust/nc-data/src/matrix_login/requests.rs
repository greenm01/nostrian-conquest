//! Sign-in requests from the games page. Anyone can type any Matrix ID, so the
//! bridge only queues a request; `nc-matrix` delivers it under the limits here.
use crate::AccountIdentity;
use rusqlite::{Connection, OptionalExtension, params};

/// One site request per Matrix ID in this window.
pub const REQUEST_COOLDOWN_SECS: i64 = 600;
/// New private rooms the bot may open for site requests per hour.
pub const INVITE_LIMIT: u32 = 20;
/// A request the bot has not picked up by then lapses.
const REQUEST_LIFETIME_SECS: i64 = 300;
const QUEUE_LIMIT: u32 = 1024;

#[derive(Debug, PartialEq, Eq)]
pub enum Request {
    Queued,
    /// This Matrix ID was sent a request within `REQUEST_COOLDOWN_SECS`.
    TooSoon,
    /// Too many requests are waiting for the bot.
    Busy,
}

pub(super) fn ensure(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS matrix_login_requests(account TEXT PRIMARY KEY,
            requested INTEGER NOT NULL,pending INTEGER NOT NULL);
        CREATE TABLE IF NOT EXISTS matrix_login_invites(at INTEGER NOT NULL);",
    )
}

/// Queues a sign-in message for `account`, which must be a Matrix identity.
pub fn request(conn: &Connection, account: &AccountIdentity, now: i64) -> Result<Request, String> {
    if !matches!(account, AccountIdentity::Matrix(_)) {
        return Err("Matrix sign-in requests require a Matrix identity".into());
    }
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM matrix_login_requests WHERE requested<=?1",
        [now - REQUEST_COOLDOWN_SECS],
    )
    .map_err(|e| e.to_string())?;
    let recent = tx
        .query_row(
            "SELECT 1 FROM matrix_login_requests WHERE account=?1",
            [account],
            |_| Ok(()),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if recent.is_some() {
        return Ok(Request::TooSoon);
    }
    let waiting: u32 = tx
        .query_row(
            "SELECT COUNT(*) FROM matrix_login_requests WHERE pending=1",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if waiting >= QUEUE_LIMIT {
        return Ok(Request::Busy);
    }
    tx.execute(
        "INSERT INTO matrix_login_requests VALUES (?1,?2,1)",
        params![account, now],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(Request::Queued)
}

/// Claims waiting requests for delivery. Each is returned once; its row stays
/// until the cooldown ends so the same ID cannot be requested again sooner.
pub fn take_requests(conn: &Connection, now: i64) -> rusqlite::Result<Vec<AccountIdentity>> {
    let tx = conn.unchecked_transaction()?;
    let accounts = tx
        .prepare(
            "UPDATE matrix_login_requests SET pending=0 WHERE pending=1 RETURNING account,requested",
        )?
        .query_map([], |r| Ok((r.get::<_, AccountIdentity>(0)?, r.get::<_, i64>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    tx.commit()?;
    Ok(accounts
        .into_iter()
        .filter(|(_, at)| now - at < REQUEST_LIFETIME_SECS)
        .map(|(account, _)| account)
        .collect())
}

/// Whether the bot may open another private room for a site request.
pub fn may_invite(conn: &Connection, now: i64) -> rusqlite::Result<bool> {
    conn.execute(
        "DELETE FROM matrix_login_invites WHERE at<=?1",
        [now - 3600],
    )?;
    let count: u32 = conn.query_row("SELECT COUNT(*) FROM matrix_login_invites", [], |r| {
        r.get(0)
    })?;
    Ok(count < INVITE_LIMIT)
}

pub fn record_invite(conn: &Connection, now: i64) -> rusqlite::Result<()> {
    conn.execute("INSERT INTO matrix_login_invites VALUES (?1)", [now])
        .map(drop)
}
