use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension};

use crate::{CoreGameData, QueuedPlayerMail, ReportBlockRow};

mod hex;
mod intel;
mod mail;
mod metadata;
mod planet_owned_since;
mod player_activity;
mod player_lifecycle;
mod player_war_stats;
mod report_blocks;
mod runtime;
mod seat_bind_source;
mod settings;
mod snapshot_core;

pub use player_activity::PlayerActivityState;
pub use player_lifecycle::{PlayerLifecycleState, TerminalOutcome, WinnerState};
pub use player_war_stats::PlayerWarStatsState;
pub use seat_bind_source::SeatBindSource;
pub use settings::{
    CampaignSettings, DEFAULT_CAMPAIGN_THEME_KEY, DEFAULT_MAINTENANCE_INTERVAL_MINUTES,
};

pub const DEFAULT_CAMPAIGN_DB_NAME: &str = "ncgame.db";
/// What a player is told when a save is refused because maintenance has
/// written a newer year since the session loaded its state. Short enough for
/// one line of the 80-column menu notice area.
pub const STALE_SESSION_MESSAGE: &str =
    "Maintenance has run; your last change was not saved. Log in again.";
const RUNTIME_SCHEMA_VERSION: i64 = 15;
const LEGACY_RECORD_TABLES: [&str; 7] = [
    "player_record_fields",
    "planet_record_fields",
    "fleet_record_fields",
    "base_record_fields",
    "ipbm_record_fields",
    "setup_record_fields",
    "conquest_record_fields",
];

#[derive(Debug)]
pub enum CampaignStoreError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Sql(rusqlite::Error),
    Parse(crate::ParseError),
    InvalidState(String),
    SchemaVersionMismatch {
        expected: i64,
        found: Option<i64>,
    },
    /// A save was refused because a snapshot newer than the year the caller
    /// loaded exists, so the write would land in a year nobody reads anymore.
    StaleSnapshot {
        loaded_year: u16,
        latest_year: u16,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntelTier {
    Owned,
    Full,
    Partial,
    Unknown,
}

impl IntelTier {
    /// Infer intel tier from a `PlayerStarmapWorld` as seen by `viewer_empire_id`.
    ///
    /// Owned  — the viewer owns this world.
    /// Full   — armies or ground batteries are known (Scout System level).
    /// Partial — name, owner, or potential production is known (View/ETAC level).
    /// Unknown — no intel at all.
    pub fn infer_from_world(viewer_empire_id: u8, world: &crate::PlayerStarmapWorld) -> IntelTier {
        Self::infer_from_fields(
            viewer_empire_id,
            world.known_owner_empire_id,
            world.known_name.as_deref(),
            world.known_potential_production,
            world.known_armies,
            world.known_ground_batteries,
        )
    }

    /// Infer intel tier from raw constituent fields.
    ///
    /// Use this when constructing a `PlayerStarmapWorld` before the struct is
    /// complete. Use `infer_from_world` when you already have the struct.
    pub fn infer_from_fields(
        viewer_empire_id: u8,
        known_owner_empire_id: Option<u8>,
        known_name: Option<&str>,
        known_potential_production: Option<u16>,
        known_armies: Option<u8>,
        known_ground_batteries: Option<u8>,
    ) -> IntelTier {
        if known_owner_empire_id == Some(viewer_empire_id) {
            IntelTier::Owned
        } else if known_armies.is_some() || known_ground_batteries.is_some() {
            IntelTier::Full
        } else if known_name.is_some()
            || known_owner_empire_id.is_some()
            || known_potential_production.is_some()
        {
            IntelTier::Partial
        } else {
            IntelTier::Unknown
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanetIntelSnapshot {
    pub planet_record_index_1_based: usize,
    pub intel_tier: IntelTier,
    pub compat_is_orbit_seed: bool,
    pub last_intel_year: Option<u16>,
    pub seen_year: Option<u16>,
    pub scout_year: Option<u16>,
    pub known_name: Option<String>,
    pub known_owner_empire_id: Option<u8>,
    pub known_potential_production: Option<u16>,
    pub known_armies: Option<u8>,
    pub known_ground_batteries: Option<u8>,
    pub known_starbase_count: Option<u8>,
    pub known_current_production: Option<u8>,
    pub known_stored_points: Option<u16>,
    pub known_docked_summary: Option<String>,
    pub known_orbit_summary: Option<String>,
    pub compat_word_1e: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CampaignStore {
    path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CampaignRuntimeState {
    pub snapshot_id: i64,
    pub game_year: u16,
    pub campaign_seed: u64,
    pub winner_state: WinnerState,
    pub game_data: CoreGameData,
    /// Structured report blocks. This is the authoritative runtime review
    /// state; callers should not rely on classic RESULTS.DAT byte payloads.
    pub report_block_rows: Vec<ReportBlockRow>,
    pub queued_mail: Vec<QueuedPlayerMail>,
}

impl std::fmt::Display for CampaignStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "{}: {}", path.display(), source),
            Self::Sql(source) => write!(f, "{source}"),
            Self::Parse(source) => write!(f, "{source}"),
            Self::InvalidState(message) => write!(f, "{message}"),
            Self::SchemaVersionMismatch { expected, found } => match found {
                Some(found) => write!(
                    f,
                    "runtime sqlite schema version {found} is unsupported; expected {expected}. recreate or refresh ncgame.db"
                ),
                None => write!(
                    f,
                    "runtime sqlite schema is missing or legacy; expected version {expected}. recreate or refresh ncgame.db"
                ),
            },
            Self::StaleSnapshot { .. } => f.write_str(STALE_SESSION_MESSAGE),
        }
    }
}

impl std::error::Error for CampaignStoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Sql(source) => Some(source),
            Self::Parse(source) => Some(source),
            Self::InvalidState(_) => None,
            Self::SchemaVersionMismatch { .. } | Self::StaleSnapshot { .. } => None,
        }
    }
}

impl From<rusqlite::Error> for CampaignStoreError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Sql(value)
    }
}

impl From<crate::ParseError> for CampaignStoreError {
    fn from(value: crate::ParseError) -> Self {
        Self::Parse(value)
    }
}

impl IntelTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Owned => "owned",
            Self::Full => "full",
            Self::Partial => "partial",
            Self::Unknown => "unknown",
        }
    }

    pub(super) fn from_str(value: &str) -> Self {
        match value {
            "owned" => Self::Owned,
            "full" => Self::Full,
            "partial" => Self::Partial,
            _ => Self::Unknown,
        }
    }
}

impl CampaignStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, CampaignStoreError> {
        let store = Self {
            path: path.as_ref().to_path_buf(),
        };
        store.initialize()?;
        Ok(store)
    }

    pub fn open_default_in_dir(dir: &Path) -> Result<Self, CampaignStoreError> {
        Self::open(dir.join(DEFAULT_CAMPAIGN_DB_NAME))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn has_snapshots(&self) -> Result<bool, CampaignStoreError> {
        let mut conn = self.connection()?;
        Ok(metadata::latest_snapshot_id_and_year(&mut conn)?.is_some())
    }

    pub fn latest_snapshot_metadata(&self) -> Result<Option<(i64, u16)>, CampaignStoreError> {
        let mut conn = self.connection()?;
        metadata::latest_snapshot_id_and_year(&mut conn)
    }

    pub fn player_theme_preference(
        &self,
        player_record_index_1_based: usize,
    ) -> Result<Option<String>, CampaignStoreError> {
        let conn = self.connection()?;
        let mut stmt = conn.prepare(
            "SELECT theme_key
             FROM player_client_preferences
             WHERE player_record_index = ?1",
        )?;
        let mut rows = stmt.query([player_record_index_1_based as i64])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        Ok(Some(row.get(0)?))
    }

    pub fn set_player_theme_preference(
        &self,
        player_record_index_1_based: usize,
        theme_key: &str,
    ) -> Result<(), CampaignStoreError> {
        let conn = self.connection()?;
        conn.execute(
            "INSERT INTO player_client_preferences (player_record_index, theme_key)
             VALUES (?1, ?2)
             ON CONFLICT(player_record_index)
             DO UPDATE SET theme_key = excluded.theme_key",
            (player_record_index_1_based as i64, theme_key),
        )?;
        Ok(())
    }

    fn initialize(&self) -> Result<(), CampaignStoreError> {
        let conn = self.connection()?;
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS snapshots (
                 id INTEGER PRIMARY KEY,
                 game_year INTEGER NOT NULL UNIQUE
             );
             CREATE TABLE IF NOT EXISTS campaign_metadata (
                 key TEXT PRIMARY KEY,
                 int_value INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS player_client_preferences (
                 player_record_index INTEGER PRIMARY KEY,
                 theme_key TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS campaign_settings (
                 singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
                 slug TEXT NOT NULL,
                 game_name TEXT NOT NULL,
                 default_theme_key TEXT NOT NULL,
                 snoop_enabled INTEGER NOT NULL,
                 session_max_idle_minutes INTEGER NOT NULL,
                 session_minimum_time_minutes INTEGER NOT NULL,
                 session_local_timeout INTEGER NOT NULL,
                 session_remote_timeout INTEGER NOT NULL,
                 inactivity_autopilot_after_turns INTEGER NOT NULL,
                 maintenance_enabled INTEGER NOT NULL,
                 maintenance_interval_minutes INTEGER NOT NULL,
                 maintenance_next_due_unix_seconds INTEGER
             );
             CREATE TABLE IF NOT EXISTS seat_reservations (
                 player_record_index INTEGER PRIMARY KEY,
                 alias TEXT NOT NULL UNIQUE
             );
             CREATE TABLE IF NOT EXISTS seat_bind_sources (
                 player_record_index INTEGER PRIMARY KEY,
                 bind_source TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS planet_intel (
                 snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                 viewer_empire_id INTEGER NOT NULL,
                 planet_record_index INTEGER NOT NULL,
                 intel_tier TEXT NOT NULL,
                 compat_is_orbit_seed INTEGER NOT NULL DEFAULT 0,
                 last_intel_year INTEGER,
                 seen_year INTEGER,
                 scout_year INTEGER,
                 known_name TEXT,
                 known_owner_empire_id INTEGER,
                 known_potential_production INTEGER,
                 known_armies INTEGER,
                 known_ground_batteries INTEGER,
                 known_starbase_count INTEGER,
                 known_current_production INTEGER,
                 known_stored_points INTEGER,
                 known_docked_summary TEXT,
                 known_orbit_summary TEXT,
                 compat_word_1e INTEGER,
                 PRIMARY KEY(snapshot_id, viewer_empire_id, planet_record_index)
             );
             CREATE TABLE IF NOT EXISTS queued_mail (
                 snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                 queue_index INTEGER NOT NULL,
                 sender_empire_id INTEGER NOT NULL,
                 recipient_empire_id INTEGER NOT NULL,
                 year INTEGER NOT NULL,
                 subject TEXT NOT NULL,
                 body TEXT NOT NULL,
                 recipient_deleted INTEGER NOT NULL DEFAULT 0,
                 PRIMARY KEY(snapshot_id, queue_index)
             );
             CREATE TABLE IF NOT EXISTS report_blocks (
                 snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                 viewer_empire_id INTEGER NOT NULL DEFAULT 0,
                 block_index INTEGER NOT NULL,
                 decoded_text TEXT NOT NULL,
                 raw_hex TEXT,
                 recipient_deleted INTEGER NOT NULL DEFAULT 0,
                 PRIMARY KEY(snapshot_id, viewer_empire_id, block_index)
             );
             CREATE TABLE IF NOT EXISTS planet_owned_since (
                 snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                 owner_empire_id INTEGER NOT NULL,
                 planet_record_index INTEGER NOT NULL,
                 acquired_year INTEGER NOT NULL,
                 PRIMARY KEY(snapshot_id, owner_empire_id, planet_record_index)
             );
             CREATE TABLE IF NOT EXISTS player_activity (
                 snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                 player_record_index INTEGER NOT NULL,
                 last_participation_year INTEGER NOT NULL,
                 inactivity_autopilot_pending_clear INTEGER NOT NULL DEFAULT 0,
                 PRIMARY KEY(snapshot_id, player_record_index)
             );
             CREATE TABLE IF NOT EXISTS player_lifecycle (
                 snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                 player_record_index INTEGER NOT NULL,
                 recovery_window_turns_remaining INTEGER NOT NULL DEFAULT 0,
                 terminal_outcome TEXT NOT NULL DEFAULT 'none',
                 terminal_review_consumed INTEGER NOT NULL DEFAULT 0,
                 PRIMARY KEY(snapshot_id, player_record_index)
             );
             CREATE TABLE IF NOT EXISTS player_war_stats (
                 snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                 player_record_index INTEGER NOT NULL,
                 colonies_established INTEGER NOT NULL,
                 worlds_taken INTEGER NOT NULL,
                 worlds_lost INTEGER NOT NULL,
                 bombardments_launched INTEGER NOT NULL,
                 bombardments_suffered INTEGER NOT NULL,
                 invade_attempts INTEGER NOT NULL,
                 invade_successes INTEGER NOT NULL,
                 blitz_attempts INTEGER NOT NULL,
                 blitz_successes INTEGER NOT NULL,
                 attacks_repelled INTEGER NOT NULL,
                 units_lost_destroyers INTEGER NOT NULL,
                 units_lost_cruisers INTEGER NOT NULL,
                 units_lost_battleships INTEGER NOT NULL,
                 units_lost_scouts INTEGER NOT NULL,
                 units_lost_transports INTEGER NOT NULL,
                 units_lost_etacs INTEGER NOT NULL,
                 units_lost_starbases INTEGER NOT NULL,
                 units_lost_armies INTEGER NOT NULL,
                 units_lost_ground_batteries INTEGER NOT NULL,
                 enemy_destroyed_destroyers INTEGER NOT NULL,
                 enemy_destroyed_cruisers INTEGER NOT NULL,
                 enemy_destroyed_battleships INTEGER NOT NULL,
                 enemy_destroyed_scouts INTEGER NOT NULL,
                 enemy_destroyed_transports INTEGER NOT NULL,
                 enemy_destroyed_etacs INTEGER NOT NULL,
                 enemy_destroyed_starbases INTEGER NOT NULL,
                 enemy_destroyed_armies INTEGER NOT NULL,
                 enemy_destroyed_ground_batteries INTEGER NOT NULL,
                 PRIMARY KEY(snapshot_id, player_record_index)
             );",
        )?;
        snapshot_core::create_tables(&conn)?;
        ensure_column(&conn, "planet_intel", "known_docked_summary", "TEXT")?;
        ensure_column(&conn, "planet_intel", "known_orbit_summary", "TEXT")?;
        ensure_column(&conn, "planet_intel", "known_starbase_count", "INTEGER")?;
        let mut conn = conn;
        let schema_version = metadata::load_runtime_schema_version(&mut conn)?;
        match schema_version {
            Some(found) if found == RUNTIME_SCHEMA_VERSION => {}
            Some(found) => {
                return Err(CampaignStoreError::SchemaVersionMismatch {
                    expected: RUNTIME_SCHEMA_VERSION,
                    found: Some(found),
                });
            }
            None => {
                if legacy_record_schema_present(&conn)? {
                    return Err(CampaignStoreError::SchemaVersionMismatch {
                        expected: RUNTIME_SCHEMA_VERSION,
                        found: None,
                    });
                }
                metadata::persist_runtime_schema_version(&mut conn, RUNTIME_SCHEMA_VERSION)?;
            }
        }
        Ok(())
    }

    fn connection(&self) -> Result<Connection, CampaignStoreError> {
        Connection::open(&self.path).map_err(CampaignStoreError::Sql)
    }
}

fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    sql_type: &str,
) -> Result<(), CampaignStoreError> {
    let pragma = format!("PRAGMA table_info({table})");
    let mut stmt = conn.prepare(&pragma)?;
    let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
    let has_column = columns
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .any(|name| name == column);
    if !has_column {
        let sql = format!("ALTER TABLE {table} ADD COLUMN {column} {sql_type}");
        conn.execute(&sql, [])?;
    }
    Ok(())
}

fn legacy_record_schema_present(conn: &Connection) -> Result<bool, CampaignStoreError> {
    for table in LEGACY_RECORD_TABLES {
        let exists: Option<i64> = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1",
                [table],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_some() {
            return Ok(true);
        }
    }
    Ok(false)
}
