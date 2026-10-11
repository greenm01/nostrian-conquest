use crate::CONQUEST_DAT_SIZE;
use crate::classic::{decode_conquest, encode_conquest};
use crate::model::Conquest;
use crate::model::Player;

impl Conquest {
    pub fn from_raw(raw: [u8; CONQUEST_DAT_SIZE]) -> Self {
        decode_conquest(&raw)
    }

    pub fn game_year(&self) -> u16 {
        self.game_year
    }
    pub fn set_game_year(&mut self, year: u16) {
        self.game_year = year;
    }
    pub fn player_count(&self) -> u8 {
        self.player_count
    }
    pub fn set_player_count(&mut self, count: u8) {
        self.player_count = count;
    }
    /// The schedule as the seven bytes ECUTIL writes, Sunday first.
    pub fn maintenance_schedule_bytes(&self) -> [u8; 7] {
        self.maintenance_days.0.map(u8::from)
    }
    /// Sunday through Saturday; ECMAINT runs on a day whose byte is nonzero.
    pub fn maintenance_schedule_enabled(&self) -> [bool; 7] {
        self.maintenance_days.0
    }
    pub fn set_maintenance_schedule_bytes(&mut self, bytes: [u8; 7]) {
        self.maintenance_days.0 = bytes.map(|byte| byte != 0);
    }
    /// Writes 1 for an enabled day and 0 otherwise, as ECUTIL does.
    pub fn set_maintenance_schedule_enabled(&mut self, enabled: [bool; 7]) {
        self.maintenance_days.0 = enabled;
    }
    /// Each empire's production at the start of the most recent pass.
    pub fn previous_production(&self) -> &[u16; 25] {
        &self.previous_production
    }
    /// Each empire's planet count at the start of the most recent pass.
    pub fn previous_planet_count(&self) -> &[u8; 25] {
        &self.previous_planet_count
    }
    /// Copies each empire's production and planet count into the
    /// previous-year arrays, as ECMAINT does at the start of a pass
    /// (0x27A5). Slots without a player record read 0.
    pub fn snapshot_previous_year(&mut self, players: &[Player]) {
        crate::rankings::snapshot_previous_year(self, players)
    }
    pub fn to_bytes(&self) -> Vec<u8> {
        encode_conquest(self).to_vec()
    }
}
