use rusqlite::{Connection, Result as SqliteResult, params};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameSettings {
    pub maintenance_enabled: bool,
    pub maintenance_interval_minutes: u32,
    pub maintenance_next_due_unix_seconds: Option<i64>,
    pub lobby_visibility: LobbyVisibility,
    pub recruiting: RecruitingMode,
    pub catalog_state: CatalogState,
    pub host_alias: Option<String>,
    pub summary: Option<String>,
    pub game_tier: GameTier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameTier {
    Sandbox,
    League,
}

impl GameTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            GameTier::Sandbox => "sandbox",
            GameTier::League => "league",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "sandbox" => Some(GameTier::Sandbox),
            "league" => Some(GameTier::League),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LobbyVisibility {
    Public,
    Private,
}

impl LobbyVisibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            LobbyVisibility::Public => "public",
            LobbyVisibility::Private => "private",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "public" => Some(LobbyVisibility::Public),
            "private" => Some(LobbyVisibility::Private),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecruitingMode {
    None,
    NewPlayers,
    ReplacementPlayers,
}

impl RecruitingMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            RecruitingMode::None => "none",
            RecruitingMode::NewPlayers => "new_players",
            RecruitingMode::ReplacementPlayers => "replacement_players",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "none" => Some(RecruitingMode::None),
            "new_players" => Some(RecruitingMode::NewPlayers),
            "replacement_players" => Some(RecruitingMode::ReplacementPlayers),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogState {
    Listed,
    Retired,
}

impl CatalogState {
    pub fn as_str(&self) -> &'static str {
        match self {
            CatalogState::Listed => "listed",
            CatalogState::Retired => "retired",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "listed" => Some(CatalogState::Listed),
            "retired" => Some(CatalogState::Retired),
            _ => None,
        }
    }
}

pub fn get_settings(conn: &Connection, game_id: &str) -> SqliteResult<GameSettings> {
    let mut stmt = conn.prepare(
        "SELECT maintenance_enabled, maintenance_interval_minutes, maintenance_next_due_unix_seconds,
                lobby_visibility, recruiting, catalog_state, host_alias, summary, game_tier
         FROM game_metadata WHERE id = ?1"
    )?;

    stmt.query_row(params![game_id], |row| {
        Ok(GameSettings {
            maintenance_enabled: row.get::<_, i32>(0)? != 0,
            maintenance_interval_minutes: row.get(1)?,
            maintenance_next_due_unix_seconds: row.get(2)?,
            lobby_visibility: LobbyVisibility::from_str(&row.get::<_, String>(3)?)
                .unwrap_or(LobbyVisibility::Private),
            recruiting: RecruitingMode::from_str(&row.get::<_, String>(4)?)
                .unwrap_or(RecruitingMode::None),
            catalog_state: row
                .get::<_, Option<String>>(5)?
                .as_deref()
                .and_then(CatalogState::from_str)
                .unwrap_or(CatalogState::Listed),
            host_alias: row.get(6)?,
            summary: row.get(7)?,
            game_tier: row
                .get::<_, Option<String>>(8)?
                .as_deref()
                .and_then(GameTier::from_str)
                .unwrap_or(GameTier::League),
        })
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameMetadata {
    pub id: String,
    pub name: String,
    pub status: String,
    pub created_at: i64,
    pub current_year: u32,
    pub current_turn: u32,
    pub players: u32,
}

pub fn get_game_metadata(conn: &Connection, game_id: &str) -> SqliteResult<GameMetadata> {
    let mut stmt = conn.prepare(
        "SELECT id, name, status, created_at, current_year, current_turn, players
         FROM game_metadata WHERE id = ?1",
    )?;

    stmt.query_row(params![game_id], |row| {
        Ok(GameMetadata {
            id: row.get(0)?,
            name: row.get(1)?,
            status: row.get(2)?,
            created_at: row.get(3)?,
            current_year: row.get(4)?,
            current_turn: row.get(5)?,
            players: row.get(6)?,
        })
    })
}

pub fn update_settings(
    conn: &Connection,
    game_id: &str,
    settings: &GameSettings,
) -> SqliteResult<()> {
    conn.execute(
        "UPDATE game_metadata SET
            maintenance_enabled = ?1,
            maintenance_interval_minutes = ?2,
            maintenance_next_due_unix_seconds = ?3,
            lobby_visibility = ?4,
            recruiting = ?5,
            catalog_state = ?6,
            host_alias = ?7,
            summary = ?8,
            updated_at = ?9,
            game_tier = ?10
         WHERE id = ?11",
        params![
            settings.maintenance_enabled as i32,
            settings.maintenance_interval_minutes,
            settings.maintenance_next_due_unix_seconds,
            settings.lobby_visibility.as_str(),
            settings.recruiting.as_str(),
            settings.catalog_state.as_str(),
            settings.host_alias,
            settings.summary,
            chrono::Utc::now().timestamp(),
            settings.game_tier.as_str(),
            game_id,
        ],
    )?;
    Ok(())
}

/// Record the year and turn maintenance advanced the game to.
pub fn set_game_clock(conn: &Connection, game_id: &str, year: u16, turn: u32) -> SqliteResult<()> {
    conn.execute(
        "UPDATE game_metadata SET current_year = ?1, current_turn = ?2, updated_at = ?3
         WHERE id = ?4",
        params![year, turn, chrono::Utc::now().timestamp(), game_id],
    )?;
    Ok(())
}

pub fn mark_catalog_dirty(conn: &Connection, game_id: &str) -> SqliteResult<()> {
    let now = chrono::Utc::now().timestamp();
    conn.execute(
        "UPDATE game_metadata SET catalog_dirty_since = ?1 WHERE id = ?2",
        params![now, game_id],
    )?;
    Ok(())
}

pub fn get_catalog_dirty_since(conn: &Connection, game_id: &str) -> SqliteResult<Option<i64>> {
    let mut stmt = conn.prepare("SELECT catalog_dirty_since FROM game_metadata WHERE id = ?1")?;
    let result: Option<i64> = stmt.query_row(params![game_id], |row| row.get(0)).ok();
    Ok(result)
}

pub fn clear_catalog_dirty(conn: &Connection, game_id: &str) -> SqliteResult<()> {
    conn.execute(
        "UPDATE game_metadata SET catalog_dirty_since = NULL WHERE id = ?1",
        params![game_id],
    )?;
    Ok(())
}
