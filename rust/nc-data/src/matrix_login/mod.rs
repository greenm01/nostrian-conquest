//! Matrix sign-in codes. `nc-matrix` issues a code and link secret to the
//! verified sender of a request; `nc-bridge` redeems either one, inside the
//! transaction that creates the browser session. Both use the bridge's
//! `.web-sessions.db`, and only hashes are stored.
mod requests;

use crate::{AccountIdentity, access_code};
pub use requests::{
    INVITE_LIMIT, REQUEST_COOLDOWN_SECS, Request, may_invite, record_invite, request, take_requests,
};
use rusqlite::{Connection, OptionalExtension, params};

pub const LIFETIME_SECS: i64 = 600;
/// A user's next request is ignored until this many seconds have passed.
pub const REQUEST_INTERVAL_SECS: i64 = 30;
/// Failed redemptions allowed site-wide per lifetime window. A wrong code
/// cannot be attributed to any one pending code, so the limit is global.
pub const FAILURE_LIMIT: u32 = 100;
const PENDING_LIMIT: u32 = 4096;

#[derive(Debug, PartialEq, Eq)]
pub struct Issued {
    /// Eight characters for typing on the games page.
    pub code: String,
    /// 64 hex characters for the sign-in link's URL fragment.
    pub link_secret: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Issue {
    Issued(Issued),
    /// The user asked within `REQUEST_INTERVAL_SECS`; the earlier code stands.
    TooSoon,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Redeem {
    SignedIn {
        account: AccountIdentity,
        display_name: String,
    },
    Invalid,
    Throttled,
}

pub fn ensure(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS matrix_login_codes(account TEXT PRIMARY KEY,
            display_name TEXT NOT NULL,code_hash TEXT NOT NULL UNIQUE,
            link_hash TEXT NOT NULL UNIQUE,issued INTEGER NOT NULL,expires INTEGER NOT NULL);
        CREATE TABLE IF NOT EXISTS matrix_login_failures(at INTEGER NOT NULL);",
    )?;
    requests::ensure(conn)
}

/// Replaces any earlier code for this user. `account` must be a Matrix identity.
pub fn issue(
    conn: &Connection,
    account: &AccountIdentity,
    display_name: &str,
    now: i64,
) -> Result<Issue, String> {
    if !matches!(account, AccountIdentity::Matrix(_)) {
        return Err("Matrix sign-in codes require a Matrix identity".into());
    }
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM matrix_login_codes WHERE expires<=?1", [now])
        .map_err(|e| e.to_string())?;
    let last: Option<i64> = tx
        .query_row(
            "SELECT issued FROM matrix_login_codes WHERE account=?1",
            [account],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if last.is_some_and(|at| now - at < REQUEST_INTERVAL_SECS) {
        return Ok(Issue::TooSoon);
    }
    let pending: u32 = tx
        .query_row("SELECT COUNT(*) FROM matrix_login_codes", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if last.is_none() && pending >= PENDING_LIMIT {
        return Err("too many pending Matrix sign-ins".into());
    }
    let issued = loop {
        let code = access_code::generate();
        let taken: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM matrix_login_codes WHERE code_hash=?1 AND account<>?2)",
                params![access_code::hash(&code), account],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if !taken {
            let link_secret = (0..32)
                .map(|_| format!("{:02x}", rand::random::<u8>()))
                .collect();
            break Issued { code, link_secret };
        }
    };
    tx.execute(
        "INSERT OR REPLACE INTO matrix_login_codes VALUES (?1,?2,?3,?4,?5,?6)",
        params![
            account,
            display_name,
            access_code::hash(&issued.code),
            link_hash(&issued.link_secret),
            now,
            now + LIFETIME_SECS
        ],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(Issue::Issued(issued))
}

/// Accepts a typed code or a link secret and consumes it. The caller owns the
/// transaction, so a failed session write leaves the code unused; commit it for
/// every returned outcome so failed attempts count toward the limit.
pub fn redeem(conn: &Connection, secret: &str, now: i64) -> rusqlite::Result<Redeem> {
    conn.execute(
        "DELETE FROM matrix_login_failures WHERE at<=?1",
        [now - LIFETIME_SECS],
    )?;
    let failures: u32 = conn.query_row("SELECT COUNT(*) FROM matrix_login_failures", [], |r| {
        r.get(0)
    })?;
    if failures >= FAILURE_LIMIT {
        return Ok(Redeem::Throttled);
    }
    let secret = secret.trim();
    let (column, hash) = if secret.len() == 64 {
        ("link_hash", link_hash(secret))
    } else {
        ("code_hash", access_code::hash(secret))
    };
    let found = conn
        .query_row(
            &format!(
                "DELETE FROM matrix_login_codes WHERE {column}=?1 AND expires>?2
                RETURNING account,display_name"
            ),
            params![hash, now],
            |r| Ok((r.get::<_, AccountIdentity>(0)?, r.get::<_, String>(1)?)),
        )
        .optional()?;
    match found {
        Some((account, display_name)) => Ok(Redeem::SignedIn {
            account,
            display_name,
        }),
        None => {
            conn.execute("INSERT INTO matrix_login_failures VALUES (?1)", [now])?;
            Ok(Redeem::Invalid)
        }
    }
}

fn link_hash(secret: &str) -> String {
    blake3::hash(secret.as_bytes()).to_hex().to_string()
}
