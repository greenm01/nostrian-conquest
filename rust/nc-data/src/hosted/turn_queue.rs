use rusqlite::{Connection, Result as SqliteResult, params};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnSubmission {
    pub id: String,
    pub game_id: String,
    pub turn: u32,
    pub player_pubkey: String,
    pub commands: String,
    pub status: TurnSubmissionStatus,
    pub submitted_at: i64,
    pub processed_at: Option<i64>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnSubmissionStatus {
    Pending,
    Accepted,
    Rejected,
    Superseded,
}

impl TurnSubmissionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TurnSubmissionStatus::Pending => "pending",
            TurnSubmissionStatus::Accepted => "accepted",
            TurnSubmissionStatus::Rejected => "rejected",
            TurnSubmissionStatus::Superseded => "superseded",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(TurnSubmissionStatus::Pending),
            "accepted" => Some(TurnSubmissionStatus::Accepted),
            "rejected" => Some(TurnSubmissionStatus::Rejected),
            "superseded" => Some(TurnSubmissionStatus::Superseded),
            _ => None,
        }
    }
}

pub fn enqueue_turn(
    conn: &Connection,
    id: &str,
    game_id: &str,
    turn: u32,
    player_pubkey: &str,
    commands: &str,
) -> SqliteResult<()> {
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "INSERT INTO turn_queue (id, game_id, turn, player_pubkey, commands, status, submitted_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'pending', ?6)",
        params![id, game_id, turn, player_pubkey, commands, now],
    )?;
    Ok(())
}

pub fn get_pending_turn(
    conn: &Connection,
    game_id: &str,
    turn: u32,
    player_pubkey: &str,
) -> SqliteResult<Option<TurnSubmission>> {
    let mut stmt = conn.prepare(
        "SELECT id, game_id, turn, player_pubkey, commands, status, submitted_at,
                processed_at, error_message
         FROM turn_queue WHERE game_id = ?1 AND turn = ?2 AND player_pubkey = ?3 AND status = 'pending'"
    )?;

    let mut rows = stmt.query(params![game_id, turn, player_pubkey])?;
    rows.next()?.map(read_turn_submission).transpose()
}

pub fn get_turn_submission(conn: &Connection, id: &str) -> SqliteResult<Option<TurnSubmission>> {
    let mut stmt = conn.prepare(
        "SELECT id, game_id, turn, player_pubkey, commands, status, submitted_at,
                processed_at, error_message
         FROM turn_queue WHERE id = ?1",
    )?;

    let mut rows = stmt.query(params![id])?;
    rows.next()?.map(read_turn_submission).transpose()
}

fn read_turn_submission(row: &rusqlite::Row<'_>) -> SqliteResult<TurnSubmission> {
    Ok(TurnSubmission {
        id: row.get(0)?,
        game_id: row.get(1)?,
        turn: row.get(2)?,
        player_pubkey: row.get(3)?,
        commands: row.get(4)?,
        status: TurnSubmissionStatus::from_str(&row.get::<_, String>(5)?)
            .unwrap_or(TurnSubmissionStatus::Pending),
        submitted_at: row.get(6)?,
        processed_at: row.get(7)?,
        error_message: row.get(8)?,
    })
}

pub fn list_pending_turns(
    conn: &Connection,
    game_id: &str,
    turn: u32,
) -> SqliteResult<Vec<TurnSubmission>> {
    let mut stmt = conn.prepare(
        "SELECT id, game_id, turn, player_pubkey, commands, status, submitted_at,
                processed_at, error_message
         FROM turn_queue WHERE game_id = ?1 AND turn = ?2 AND status = 'pending'
         ORDER BY submitted_at",
    )?;

    let turns = stmt.query_map(params![game_id, turn], read_turn_submission)?;

    turns.collect()
}

pub fn count_pending_turns(conn: &Connection, game_id: &str) -> SqliteResult<u32> {
    let mut stmt = conn.prepare(
        "SELECT COUNT(*) FROM turn_queue
         WHERE game_id = ?1 AND status = 'pending'",
    )?;
    stmt.query_row(params![game_id], |row| row.get(0))
}

pub fn accept_turn(conn: &Connection, id: &str) -> SqliteResult<()> {
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "UPDATE turn_queue SET status = 'accepted', processed_at = ?1 WHERE id = ?2",
        params![now, id],
    )?;
    Ok(())
}

pub fn reject_turn(conn: &Connection, id: &str, error_message: &str) -> SqliteResult<()> {
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "UPDATE turn_queue SET status = 'rejected', processed_at = ?1, error_message = ?2 WHERE id = ?3",
        params![now, error_message, id],
    )?;
    Ok(())
}

pub fn mark_superseded(
    conn: &Connection,
    game_id: &str,
    turn: u32,
    player_pubkey: &str,
) -> SqliteResult<()> {
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "UPDATE turn_queue SET status = 'superseded', processed_at = ?1
         WHERE game_id = ?2 AND turn = ?3 AND player_pubkey = ?4 AND status = 'pending'",
        params![now, game_id, turn, player_pubkey],
    )?;
    Ok(())
}
