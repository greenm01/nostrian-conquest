use rusqlite::{OptionalExtension, params};

use super::ai_seats::{AiSeat, load_ai_seats_conn, save_ai_seats_tx};
use super::{CampaignStore, CampaignStoreError};
use crate::bbs_config::validate_reservations as validate_shared_reservations;
use crate::{DEFAULT_INACTIVITY_AUTOPILOT_AFTER_TURNS, SeatReservation};

pub const DEFAULT_CAMPAIGN_THEME_KEY: &str = "tokyo_night";
pub const DEFAULT_MAINTENANCE_INTERVAL_MINUTES: u32 = 24 * 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CampaignSettings {
    pub slug: String,
    pub game_name: String,
    pub default_theme_key: String,
    pub snoop_enabled: bool,
    pub session_max_idle_minutes: u8,
    pub session_minimum_time_minutes: u8,
    pub session_local_timeout: bool,
    pub session_remote_timeout: bool,
    pub inactivity_autopilot_after_turns: u8,
    pub maintenance_enabled: bool,
    pub maintenance_interval_minutes: u32,
    pub maintenance_next_due_unix_seconds: Option<u64>,
    pub reservations: Vec<SeatReservation>,
    /// Seats a learned agent plays (t165).
    pub ai_seats: Vec<AiSeat>,
}

impl CampaignSettings {
    pub fn new(slug: impl Into<String>, game_name: impl Into<String>) -> Self {
        Self {
            slug: slug.into(),
            game_name: game_name.into(),
            default_theme_key: DEFAULT_CAMPAIGN_THEME_KEY.to_string(),
            snoop_enabled: true,
            session_max_idle_minutes: 10,
            session_minimum_time_minutes: 0,
            session_local_timeout: false,
            session_remote_timeout: true,
            inactivity_autopilot_after_turns: DEFAULT_INACTIVITY_AUTOPILOT_AFTER_TURNS,
            maintenance_enabled: false,
            maintenance_interval_minutes: DEFAULT_MAINTENANCE_INTERVAL_MINUTES,
            maintenance_next_due_unix_seconds: None,
            reservations: Vec::new(),
            ai_seats: Vec::new(),
        }
    }

    pub fn validate(self) -> Result<Self, CampaignStoreError> {
        let slug = self.slug.trim();
        if slug.is_empty() {
            return Err(CampaignStoreError::InvalidState(
                "campaign slug must not be blank".to_string(),
            ));
        }
        if self.game_name.trim().is_empty() {
            return Err(CampaignStoreError::InvalidState(
                "campaign game_name must not be blank".to_string(),
            ));
        }
        if self.default_theme_key.trim().is_empty() {
            return Err(CampaignStoreError::InvalidState(
                "campaign default_theme_key must not be blank".to_string(),
            ));
        }
        if self.session_max_idle_minutes > 120 {
            return Err(CampaignStoreError::InvalidState(format!(
                "session_max_idle_minutes must be <= 120, got {}",
                self.session_max_idle_minutes
            )));
        }
        if self.session_minimum_time_minutes > 120 {
            return Err(CampaignStoreError::InvalidState(format!(
                "session_minimum_time_minutes must be <= 120, got {}",
                self.session_minimum_time_minutes
            )));
        }
        if self.inactivity_autopilot_after_turns > 100 {
            return Err(CampaignStoreError::InvalidState(format!(
                "inactivity_autopilot_after_turns must be <= 100, got {}",
                self.inactivity_autopilot_after_turns
            )));
        }
        if self.maintenance_interval_minutes == 0 {
            return Err(CampaignStoreError::InvalidState(
                "maintenance_interval_minutes must be >= 1".to_string(),
            ));
        }
        validate_reservations(&self.reservations)?;
        for (idx, seat) in self.ai_seats.iter().enumerate() {
            let player = seat.player_record_index_1_based;
            if player == 0 {
                return Err(CampaignStoreError::InvalidState(
                    "AI seat player must be at least 1".to_string(),
                ));
            }
            if self.ai_seats[..idx]
                .iter()
                .any(|other| other.player_record_index_1_based == player)
            {
                return Err(CampaignStoreError::InvalidState(format!(
                    "seat {player} is listed as an AI seat twice"
                )));
            }
            if self.reservation_for_player(player).is_some() {
                return Err(CampaignStoreError::InvalidState(format!(
                    "seat {player} is reserved for a player and cannot be an AI seat"
                )));
            }
        }
        Ok(self)
    }

    pub fn reservation_for_alias(&self, alias: &str) -> Option<&SeatReservation> {
        let alias = alias.trim();
        self.reservations
            .iter()
            .find(|reservation| reservation.alias.eq_ignore_ascii_case(alias))
    }

    pub fn reservation_for_player(
        &self,
        player_record_index_1_based: usize,
    ) -> Option<&SeatReservation> {
        self.reservations.iter().find(|reservation| {
            reservation.player_record_index_1_based == player_record_index_1_based
        })
    }

    pub fn validate_reservations_for_player_count(
        &self,
        player_count: usize,
    ) -> Result<(), CampaignStoreError> {
        for reservation in &self.reservations {
            if reservation.player_record_index_1_based > player_count {
                return Err(CampaignStoreError::InvalidState(format!(
                    "reservation player {} exceeds player count {}",
                    reservation.player_record_index_1_based, player_count
                )));
            }
        }
        for seat in &self.ai_seats {
            if seat.player_record_index_1_based > player_count {
                return Err(CampaignStoreError::InvalidState(format!(
                    "AI seat {} exceeds player count {}",
                    seat.player_record_index_1_based, player_count
                )));
            }
        }
        Ok(())
    }

    pub fn maintenance_due_at(&self, now_unix_seconds: u64) -> bool {
        self.maintenance_enabled
            && self
                .maintenance_next_due_unix_seconds
                .map(|due| due <= now_unix_seconds)
                .unwrap_or(false)
    }
}

impl CampaignStore {
    pub fn campaign_settings(&self) -> Result<Option<CampaignSettings>, CampaignStoreError> {
        let conn = self.connection()?;
        let Some(mut settings) = conn
            .query_row(
                "SELECT slug,
                        game_name,
                        default_theme_key,
                        snoop_enabled,
                        session_max_idle_minutes,
                        session_minimum_time_minutes,
                        session_local_timeout,
                        session_remote_timeout,
                        inactivity_autopilot_after_turns,
                        maintenance_enabled,
                        maintenance_interval_minutes,
                        maintenance_next_due_unix_seconds
                 FROM campaign_settings
                 WHERE singleton = 1",
                [],
                |row| {
                    Ok(CampaignSettings {
                        slug: row.get(0)?,
                        game_name: row.get(1)?,
                        default_theme_key: row.get(2)?,
                        snoop_enabled: row.get::<_, i64>(3)? != 0,
                        session_max_idle_minutes: row.get::<_, i64>(4)? as u8,
                        session_minimum_time_minutes: row.get::<_, i64>(5)? as u8,
                        session_local_timeout: row.get::<_, i64>(6)? != 0,
                        session_remote_timeout: row.get::<_, i64>(7)? != 0,
                        inactivity_autopilot_after_turns: row.get::<_, i64>(8)? as u8,
                        maintenance_enabled: row.get::<_, i64>(9)? != 0,
                        maintenance_interval_minutes: row.get::<_, i64>(10)? as u32,
                        maintenance_next_due_unix_seconds: row
                            .get::<_, Option<i64>>(11)?
                            .map(|value| value as u64),
                        reservations: Vec::new(),
                        ai_seats: Vec::new(),
                    })
                },
            )
            .optional()?
        else {
            return Ok(None);
        };
        settings.reservations = load_reservations_conn(&conn)?;
        settings.ai_seats = load_ai_seats_conn(&conn)?;
        Ok(Some(settings.validate()?))
    }

    pub fn load_campaign_settings(&self) -> Result<CampaignSettings, CampaignStoreError> {
        self.campaign_settings()?.ok_or_else(|| {
            CampaignStoreError::InvalidState(
                "campaign settings are missing from ncgame.db".to_string(),
            )
        })
    }

    pub fn save_campaign_settings(
        &self,
        settings: &CampaignSettings,
    ) -> Result<(), CampaignStoreError> {
        let settings = settings.clone().validate()?;
        let mut conn = self.connection()?;
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO campaign_settings (
                 singleton,
                 slug,
                 game_name,
                 default_theme_key,
                 snoop_enabled,
                 session_max_idle_minutes,
                 session_minimum_time_minutes,
                 session_local_timeout,
                 session_remote_timeout,
                 inactivity_autopilot_after_turns,
                 maintenance_enabled,
                 maintenance_interval_minutes,
                 maintenance_next_due_unix_seconds
             ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(singleton) DO UPDATE SET
                 slug = excluded.slug,
                 game_name = excluded.game_name,
                 default_theme_key = excluded.default_theme_key,
                 snoop_enabled = excluded.snoop_enabled,
                 session_max_idle_minutes = excluded.session_max_idle_minutes,
                 session_minimum_time_minutes = excluded.session_minimum_time_minutes,
                 session_local_timeout = excluded.session_local_timeout,
                 session_remote_timeout = excluded.session_remote_timeout,
                 inactivity_autopilot_after_turns = excluded.inactivity_autopilot_after_turns,
                 maintenance_enabled = excluded.maintenance_enabled,
                 maintenance_interval_minutes = excluded.maintenance_interval_minutes,
                 maintenance_next_due_unix_seconds = excluded.maintenance_next_due_unix_seconds",
            params![
                settings.slug,
                settings.game_name,
                settings.default_theme_key,
                i64::from(settings.snoop_enabled),
                i64::from(settings.session_max_idle_minutes),
                i64::from(settings.session_minimum_time_minutes),
                i64::from(settings.session_local_timeout),
                i64::from(settings.session_remote_timeout),
                i64::from(settings.inactivity_autopilot_after_turns),
                i64::from(settings.maintenance_enabled),
                i64::from(settings.maintenance_interval_minutes),
                settings
                    .maintenance_next_due_unix_seconds
                    .map(|value| value as i64),
            ],
        )?;
        tx.execute("DELETE FROM seat_reservations", [])?;
        for reservation in &settings.reservations {
            tx.execute(
                "INSERT INTO seat_reservations(player_record_index, alias)
                 VALUES (?1, ?2)",
                params![
                    reservation.player_record_index_1_based as i64,
                    reservation.alias.trim()
                ],
            )?;
        }
        save_ai_seats_tx(&tx, &settings.ai_seats)?;
        tx.commit()?;
        self.apply_runtime_policy_settings(&settings)?;
        Ok(())
    }

    fn apply_runtime_policy_settings(
        &self,
        settings: &CampaignSettings,
    ) -> Result<(), CampaignStoreError> {
        let Some(runtime_state) = self.load_latest_runtime_state()? else {
            return Ok(());
        };
        let mut game_data = runtime_state.game_data;
        let setup = &mut game_data.setup;
        let mut changed = false;

        if setup.snoop != settings.snoop_enabled {
            setup.snoop = settings.snoop_enabled;
            changed = true;
        }
        if u8::try_from(setup.max_key_gap_minutes).unwrap_or(u8::MAX)
            != settings.session_max_idle_minutes
        {
            setup.max_key_gap_minutes = u16::from(settings.session_max_idle_minutes);
            changed = true;
        }
        if setup.minimum_time_minutes != settings.session_minimum_time_minutes {
            setup.minimum_time_minutes = settings.session_minimum_time_minutes;
            changed = true;
        }
        if setup.local_timeout != settings.session_local_timeout {
            setup.local_timeout = settings.session_local_timeout;
            changed = true;
        }
        if setup.remote_timeout != settings.session_remote_timeout {
            setup.remote_timeout = settings.session_remote_timeout;
            changed = true;
        }
        if u8::try_from(setup.autopilot_after_turns).unwrap_or(u8::MAX)
            != settings.inactivity_autopilot_after_turns
        {
            setup.autopilot_after_turns = u16::from(settings.inactivity_autopilot_after_turns);
            changed = true;
        }
        if changed {
            self.save_year(
                runtime_state.game_year,
                &game_data,
                &runtime_state.report_block_rows,
                &runtime_state.queued_mail,
            )?;
        }
        Ok(())
    }
}

fn validate_reservations(reservations: &[SeatReservation]) -> Result<(), CampaignStoreError> {
    validate_shared_reservations(reservations).map_err(CampaignStoreError::InvalidState)
}

fn load_reservations_conn(
    conn: &rusqlite::Connection,
) -> Result<Vec<SeatReservation>, CampaignStoreError> {
    let mut stmt = conn.prepare(
        "SELECT player_record_index, alias
         FROM seat_reservations
         ORDER BY player_record_index ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(SeatReservation {
            player_record_index_1_based: row.get::<_, i64>(0)? as usize,
            alias: row.get(1)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}
