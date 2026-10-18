use rusqlite::{Connection, Result as SqliteResult, params};
use std::path::Path;

const INIT_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS player_roster (
    npub TEXT PRIMARY KEY,
    handle TEXT,
    first_seen_at INTEGER NOT NULL,
    last_seen_at INTEGER NOT NULL,
    games_joined INTEGER NOT NULL DEFAULT 0,
    games_completed INTEGER NOT NULL DEFAULT 0,
    games_abandoned INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS player_roster_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    npub TEXT NOT NULL REFERENCES player_roster(npub),
    game_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    seat INTEGER,
    created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_player_roster_events_npub
    ON player_roster_events(npub, created_at);
"#;

pub struct RosterStore {
    conn: Connection,
}

impl RosterStore {
    pub fn open(path: &Path) -> SqliteResult<Self> {
        let mut conn = crate::sqlite::open_for_writes(path)?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute_batch(INIT_SQL)?;
        tx.commit()?;
        Ok(RosterStore { conn })
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}

pub struct RosterEntry {
    pub npub: String,
    pub handle: Option<String>,
    pub first_seen_at: i64,
    pub last_seen_at: i64,
    pub games_joined: i64,
    pub games_completed: i64,
    pub games_abandoned: i64,
}

pub struct RosterEvent {
    pub npub: String,
    pub game_id: String,
    pub event_type: String,
    pub seat: Option<u32>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandleOwnership {
    Available,
    OwnedBySelf,
    Taken,
}

pub fn normalize_handle(handle: &str) -> Option<String> {
    let trimmed = handle.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_ascii_lowercase())
    }
}

pub fn resolve_handle_ownership(
    conn: &Connection,
    npub: &str,
    handle: &str,
) -> SqliteResult<HandleOwnership> {
    let Some(normalized) = normalize_handle(handle) else {
        return Ok(HandleOwnership::Available);
    };

    let mut stmt = conn.prepare(
        "SELECT npub
         FROM player_roster
         WHERE handle IS NOT NULL
           AND lower(trim(handle)) = ?1
         LIMIT 1",
    )?;
    let mut rows = stmt.query(params![normalized])?;
    if let Some(row) = rows.next()? {
        let owner_npub: String = row.get(0)?;
        if owner_npub == npub {
            Ok(HandleOwnership::OwnedBySelf)
        } else {
            Ok(HandleOwnership::Taken)
        }
    } else {
        Ok(HandleOwnership::Available)
    }
}

/// Record that a player was seen (sent an invite request). Updates first/last seen timestamps
/// and handle; does not add an event row.
pub fn upsert_player_seen(
    conn: &Connection,
    npub: &str,
    handle: Option<&str>,
    _game_id: &str,
) -> SqliteResult<()> {
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "INSERT INTO player_roster (npub, handle, first_seen_at, last_seen_at)
         VALUES (?1, ?2, ?3, ?3)
         ON CONFLICT(npub) DO UPDATE SET
             last_seen_at = excluded.last_seen_at,
             handle = COALESCE(excluded.handle, player_roster.handle)",
        params![npub, handle, now],
    )?;
    Ok(())
}

/// Record that a player joined a game (claimed a seat). Increments games_joined.
pub fn record_player_joined(
    conn: &Connection,
    npub: &str,
    game_id: &str,
    seat: u32,
) -> SqliteResult<()> {
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "UPDATE player_roster SET games_joined = games_joined + 1, last_seen_at = ?2
         WHERE npub = ?1",
        params![npub, now],
    )?;
    conn.execute(
        "INSERT INTO player_roster_events (npub, game_id, event_type, seat, created_at)
         VALUES (?1, ?2, 'joined', ?3, ?4)",
        params![npub, game_id, seat, now],
    )?;
    Ok(())
}

/// Record that a player was ejected for inactivity (sandbox MIA). Increments games_abandoned.
pub fn record_player_abandoned(
    conn: &Connection,
    npub: &str,
    game_id: &str,
    seat: u32,
) -> SqliteResult<()> {
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "UPDATE player_roster SET games_abandoned = games_abandoned + 1, last_seen_at = ?2
         WHERE npub = ?1",
        params![npub, now],
    )?;
    conn.execute(
        "INSERT INTO player_roster_events (npub, game_id, event_type, seat, created_at)
         VALUES (?1, ?2, 'abandoned', ?3, ?4)",
        params![npub, game_id, seat, now],
    )?;
    Ok(())
}

pub fn list_roster(conn: &Connection) -> SqliteResult<Vec<RosterEntry>> {
    let mut stmt = conn.prepare(
        "SELECT npub, handle, first_seen_at, last_seen_at,
                games_joined, games_completed, games_abandoned
         FROM player_roster ORDER BY last_seen_at DESC",
    )?;
    let entries = stmt.query_map([], |row| {
        Ok(RosterEntry {
            npub: row.get(0)?,
            handle: row.get(1)?,
            first_seen_at: row.get(2)?,
            last_seen_at: row.get(3)?,
            games_joined: row.get(4)?,
            games_completed: row.get(5)?,
            games_abandoned: row.get(6)?,
        })
    })?;
    entries.collect()
}

pub fn get_roster_entry(conn: &Connection, npub: &str) -> SqliteResult<Option<RosterEntry>> {
    let mut stmt = conn.prepare(
        "SELECT npub, handle, first_seen_at, last_seen_at,
                games_joined, games_completed, games_abandoned
         FROM player_roster WHERE npub = ?1",
    )?;
    let mut rows = stmt.query(params![npub])?;
    if let Some(row) = rows.next()? {
        Ok(Some(RosterEntry {
            npub: row.get(0)?,
            handle: row.get(1)?,
            first_seen_at: row.get(2)?,
            last_seen_at: row.get(3)?,
            games_joined: row.get(4)?,
            games_completed: row.get(5)?,
            games_abandoned: row.get(6)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn list_roster_events_for_npub(
    conn: &Connection,
    npub: &str,
) -> SqliteResult<Vec<RosterEvent>> {
    let mut stmt = conn.prepare(
        "SELECT npub, game_id, event_type, seat, created_at
         FROM player_roster_events WHERE npub = ?1 ORDER BY created_at DESC",
    )?;
    let events = stmt.query_map(params![npub], |row| {
        Ok(RosterEvent {
            npub: row.get(0)?,
            game_id: row.get(1)?,
            event_type: row.get(2)?,
            seat: row.get(3)?,
            created_at: row.get(4)?,
        })
    })?;
    events.collect()
}
