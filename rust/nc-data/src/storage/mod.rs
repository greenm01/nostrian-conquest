use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension};

use crate::{CoreGameData, QueuedPlayerMail, ReportBlockRow};

mod hex;
mod intel;
mod mail;
mod metadata;
mod planet_owned_since;
mod planet_scorch_orders;
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
const RUNTIME_SCHEMA_VERSION: i64 = 13;
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
    Directory(crate::GameDirectoryError),
    InvalidState(String),
    SchemaVersionMismatch {
        expected: i64,
        found: Option<i64>,
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
    pub planet_scorch_orders: std::collections::BTreeSet<usize>,
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
            Self::Directory(source) => write!(f, "{source}"),
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
        }
    }
}

impl std::error::Error for CampaignStoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Sql(source) => Some(source),
            Self::Parse(source) => Some(source),
            Self::Directory(source) => Some(source),
            Self::InvalidState(_) => None,
            Self::SchemaVersionMismatch { .. } => None,
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

impl From<crate::GameDirectoryError> for CampaignStoreError {
    fn from(value: crate::GameDirectoryError) -> Self {
        Self::Directory(value)
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
                 inactivity_purge_after_turns INTEGER NOT NULL,
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
            CREATE TABLE IF NOT EXISTS planet_scorch_orders (
                 snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                 planet_record_index INTEGER NOT NULL,
                 PRIMARY KEY(snapshot_id, planet_record_index)
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
             );
             CREATE TABLE IF NOT EXISTS snapshot_players (
                 snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                 record_index INTEGER NOT NULL,
                 occupied_flag INTEGER NOT NULL,
                 owner_mode_raw INTEGER NOT NULL,
                 handle_raw_hex TEXT NOT NULL,
                 legacy_status_name_max_len_raw INTEGER NOT NULL,
                 legacy_status_name_len_raw INTEGER NOT NULL,
                 name_block_raw_hex TEXT NOT NULL,
                 fleet_chain_head_raw INTEGER NOT NULL,
                 fleet_chain_tail_raw INTEGER NOT NULL,
                 starbase_count_raw INTEGER NOT NULL,
                 starbase_presence_flag_raw INTEGER NOT NULL,
                 ipbm_count_raw INTEGER NOT NULL,
                 homeworld_planet_index_raw INTEGER NOT NULL,
                 last_run_year_raw INTEGER NOT NULL,
                 planet_count_raw INTEGER NOT NULL,
                 tax_rate INTEGER NOT NULL,
                 production_score_raw INTEGER NOT NULL,
                 review_state_raw_hex TEXT NOT NULL,
                 diplomacy_raw_hex TEXT NOT NULL,
                 autopilot_flag INTEGER NOT NULL,
                 PRIMARY KEY(snapshot_id, record_index)
             );
             CREATE TABLE IF NOT EXISTS snapshot_planets (
                 snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                 record_index INTEGER NOT NULL,
                 coords_x INTEGER NOT NULL,
                 coords_y INTEGER NOT NULL,
                 potential_production_raw_hex TEXT NOT NULL,
                 factories_raw_hex TEXT NOT NULL,
                 stored_production_points INTEGER NOT NULL,
                 economy_marker_raw INTEGER NOT NULL,
                 name_len_raw INTEGER NOT NULL,
                 name_buffer_raw_hex TEXT NOT NULL,
                 name_suffix_raw_hex TEXT NOT NULL,
                 build_queue_raw_hex TEXT NOT NULL,
                 infrastructure_raw_hex TEXT NOT NULL,
                 population_raw_hex TEXT NOT NULL,
                 armies_raw INTEGER NOT NULL,
                 ground_batteries_raw INTEGER NOT NULL,
                 ownership_status_raw INTEGER NOT NULL,
                 owner_empire_slot_raw INTEGER NOT NULL,
                 tail_raw_hex TEXT NOT NULL,
                 PRIMARY KEY(snapshot_id, record_index)
             );
             CREATE TABLE IF NOT EXISTS snapshot_fleets (
                 snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                 record_index INTEGER NOT NULL,
                 local_slot_word_raw INTEGER NOT NULL,
                 owner_empire_raw INTEGER NOT NULL,
                 next_fleet_link_word_raw INTEGER NOT NULL,
                 fleet_id_word_raw INTEGER NOT NULL,
                 previous_fleet_id_raw INTEGER NOT NULL,
                 invasion_army_count_raw INTEGER NOT NULL,
                 max_speed INTEGER NOT NULL,
                 current_speed INTEGER NOT NULL,
                 current_x INTEGER NOT NULL,
                 current_y INTEGER NOT NULL,
                 tuple_a_raw_hex TEXT NOT NULL,
                 tuple_b_raw_hex TEXT NOT NULL,
                 tuple_c_raw_hex TEXT NOT NULL,
                 target_x INTEGER NOT NULL,
                 target_y INTEGER NOT NULL,
                 standing_order_code_raw INTEGER NOT NULL,
                 world_target_flag_raw INTEGER NOT NULL,
                 mission_id_raw INTEGER NOT NULL,
                 rules_of_engagement INTEGER NOT NULL,
                 battleship_count INTEGER NOT NULL,
                 cruiser_count INTEGER NOT NULL,
                 destroyer_count INTEGER NOT NULL,
                 empty_transport_count INTEGER NOT NULL,
                 loaded_transport_count INTEGER NOT NULL,
                 etac_count INTEGER NOT NULL,
                 scout_count INTEGER NOT NULL,
                 strength_cache_raw INTEGER NOT NULL,
                 PRIMARY KEY(snapshot_id, record_index)
             );
             CREATE TABLE IF NOT EXISTS snapshot_bases (
                 snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                 record_index INTEGER NOT NULL,
                 escort_fleet_slot_raw INTEGER NOT NULL,
                 base_id_raw INTEGER NOT NULL,
                 owner_empire_raw INTEGER NOT NULL,
                 next_base_link_raw INTEGER NOT NULL,
                 record_number_raw INTEGER NOT NULL,
                 previous_base_link_raw INTEGER NOT NULL,
                 current_x INTEGER NOT NULL,
                 current_y INTEGER NOT NULL,
                 x_fraction_raw_hex TEXT NOT NULL,
                 y_fraction_raw_hex TEXT NOT NULL,
                 depth_raw_hex TEXT NOT NULL,
                 moving_flag_raw INTEGER NOT NULL,
                 target_x INTEGER NOT NULL,
                 target_y INTEGER NOT NULL,
                 target_depth_raw INTEGER NOT NULL,
                 PRIMARY KEY(snapshot_id, record_index)
             );
             CREATE TABLE IF NOT EXISTS snapshot_ipbms (
                 snapshot_id INTEGER NOT NULL REFERENCES snapshots(id) ON DELETE CASCADE,
                 record_index INTEGER NOT NULL,
                 prefix_raw_hex TEXT NOT NULL,
                 tuple_a_raw_hex TEXT NOT NULL,
                 tuple_b_raw_hex TEXT NOT NULL,
                 tuple_c_raw_hex TEXT NOT NULL,
                 trailing_control_raw_hex TEXT NOT NULL,
                 PRIMARY KEY(snapshot_id, record_index)
             );
             CREATE TABLE IF NOT EXISTS snapshot_setup (
                 snapshot_id INTEGER PRIMARY KEY REFERENCES snapshots(id) ON DELETE CASCADE,
                 version_tag_raw_hex TEXT NOT NULL,
                 option_prefix_raw_hex TEXT NOT NULL,
                 snoop_enabled INTEGER NOT NULL,
                 max_time_between_keys_minutes_raw INTEGER NOT NULL,
                 byte_514_raw INTEGER NOT NULL,
                 remote_timeout_enabled INTEGER NOT NULL,
                 local_timeout_enabled INTEGER NOT NULL,
                 minimum_time_granted_minutes_raw INTEGER NOT NULL,
                 purge_after_turns_raw INTEGER NOT NULL,
                 byte_519_raw INTEGER NOT NULL,
                 autopilot_inactive_turns_raw INTEGER NOT NULL,
                 byte_521_raw INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS snapshot_conquest (
                 snapshot_id INTEGER PRIMARY KEY REFERENCES snapshots(id) ON DELETE CASCADE,
                 game_year INTEGER NOT NULL,
                 player_count INTEGER NOT NULL,
                 maintenance_schedule_raw_hex TEXT NOT NULL,
                 control_word_0a_raw INTEGER NOT NULL,
                 control_word_0c_raw INTEGER NOT NULL,
                 control_word_0e_raw INTEGER NOT NULL,
                 control_word_10_raw INTEGER NOT NULL,
                 control_word_12_raw INTEGER NOT NULL,
                 control_word_14_raw INTEGER NOT NULL,
                 control_word_16_raw INTEGER NOT NULL,
                 control_word_18_raw INTEGER NOT NULL,
                 control_word_1a_raw INTEGER NOT NULL,
                 control_word_1c_raw INTEGER NOT NULL,
                 control_word_1e_raw INTEGER NOT NULL,
                 control_word_20_raw INTEGER NOT NULL,
                 control_word_22_raw INTEGER NOT NULL,
                 control_word_24_raw INTEGER NOT NULL,
                 control_word_26_raw INTEGER NOT NULL,
                 control_word_28_raw INTEGER NOT NULL,
                 control_word_2a_raw INTEGER NOT NULL,
                 control_word_2c_raw INTEGER NOT NULL,
                 control_word_2e_raw INTEGER NOT NULL,
                 control_word_30_raw INTEGER NOT NULL,
                 control_word_32_raw INTEGER NOT NULL,
                 control_word_34_raw INTEGER NOT NULL,
                 control_word_36_raw INTEGER NOT NULL,
                 control_word_38_raw INTEGER NOT NULL,
                 control_word_3a_raw INTEGER NOT NULL,
                 control_byte_3c_raw INTEGER NOT NULL,
                 control_byte_3d_raw INTEGER NOT NULL,
                 control_byte_3e_raw INTEGER NOT NULL,
                 control_byte_3f_raw INTEGER NOT NULL,
                 control_byte_40_raw INTEGER NOT NULL,
                 control_byte_41_raw INTEGER NOT NULL,
                 control_byte_42_raw INTEGER NOT NULL,
                 control_byte_43_raw INTEGER NOT NULL,
                 control_byte_44_raw INTEGER NOT NULL,
                 control_byte_45_raw INTEGER NOT NULL,
                 control_byte_46_raw INTEGER NOT NULL,
                 control_byte_47_raw INTEGER NOT NULL,
                 control_byte_48_raw INTEGER NOT NULL,
                 control_byte_49_raw INTEGER NOT NULL,
                 control_byte_4a_raw INTEGER NOT NULL,
                 control_byte_4b_raw INTEGER NOT NULL,
                 control_byte_4c_raw INTEGER NOT NULL,
                 control_byte_4d_raw INTEGER NOT NULL,
                 control_byte_4e_raw INTEGER NOT NULL,
                 control_byte_4f_raw INTEGER NOT NULL,
                 control_byte_50_raw INTEGER NOT NULL,
                 control_byte_51_raw INTEGER NOT NULL,
                 control_byte_52_raw INTEGER NOT NULL,
                 control_byte_53_raw INTEGER NOT NULL,
                 control_byte_54_raw INTEGER NOT NULL
             );",
        )?;
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
