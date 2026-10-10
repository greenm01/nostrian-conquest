use std::path::PathBuf;

use nc_data::SeatReservation;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuntimeSetupOverrides {
    pub snoop_enabled: Option<bool>,
    pub session_max_idle_minutes: Option<u8>,
    pub session_minimum_time_minutes: Option<u8>,
    pub session_local_timeout: Option<bool>,
    pub session_remote_timeout: Option<bool>,
    pub inactivity_autopilot_after_turns: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeConfig {
    pub game_name: String,
    pub theme: Option<PathBuf>,
    pub reservations: Vec<SeatReservation>,
    pub setup_overrides: RuntimeSetupOverrides,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            game_name: "Nostrian Conquest".to_string(),
            theme: None,
            reservations: Vec::new(),
            setup_overrides: RuntimeSetupOverrides::default(),
        }
    }
}

impl RuntimeConfig {
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
    ) -> Result<(), String> {
        for reservation in &self.reservations {
            if reservation.player_record_index_1_based > player_count {
                return Err(format!(
                    "reservation player {} exceeds player count {}",
                    reservation.player_record_index_1_based, player_count
                ));
            }
        }
        Ok(())
    }

    pub fn idle_timeout_secs(&self) -> Option<u64> {
        self.setup_overrides
            .session_max_idle_minutes
            .map(u64::from)
            .filter(|minutes| *minutes > 0)
            .map(|minutes| minutes.saturating_mul(60))
    }
}
