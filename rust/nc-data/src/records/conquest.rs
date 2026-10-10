use crate::CONQUEST_DAT_SIZE;
use crate::classic::{decode_conquest, encode_conquest, parse_conquest_dat};
use crate::model::Conquest;
use crate::records::player::PlayerRecord;
use crate::support::ParseError;

/// A [`Conquest`] behind the accessor API call sites still use.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConquestDat {
    pub conquest: Conquest,
}

impl ConquestDat {
    pub fn new_zeroed() -> Self {
        Self::default()
    }

    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        Ok(Self {
            conquest: parse_conquest_dat(data)?,
        })
    }

    pub fn from_raw(raw: [u8; CONQUEST_DAT_SIZE]) -> Self {
        Self {
            conquest: decode_conquest(&raw),
        }
    }

    pub fn game_year(&self) -> u16 {
        self.conquest.game_year
    }
    pub fn set_game_year(&mut self, year: u16) {
        self.conquest.game_year = year;
    }
    pub fn player_count(&self) -> u8 {
        self.conquest.player_count
    }
    pub fn set_player_count(&mut self, count: u8) {
        self.conquest.player_count = count;
    }
    /// The schedule as the seven bytes ECUTIL writes, Sunday first.
    pub fn maintenance_schedule_bytes(&self) -> [u8; 7] {
        self.conquest.maintenance_days.0.map(u8::from)
    }
    pub fn maintenance_schedule_enabled(&self) -> [bool; 7] {
        self.conquest.maintenance_days.0
    }
    pub fn set_maintenance_schedule_bytes(&mut self, bytes: [u8; 7]) {
        self.conquest.maintenance_days.0 = bytes.map(|byte| byte != 0);
    }
    pub fn set_maintenance_schedule_enabled(&mut self, enabled: [bool; 7]) {
        self.conquest.maintenance_days.0 = enabled;
    }
    /// Each empire's production at the start of the most recent pass.
    pub fn previous_production(&self) -> &[u16; 25] {
        &self.conquest.previous_production
    }
    /// Each empire's planet count at the start of the most recent pass.
    pub fn previous_planet_count(&self) -> &[u8; 25] {
        &self.conquest.previous_planet_count
    }
    /// Copies each empire's production and planet count into the
    /// previous-year arrays, as ECMAINT does at the start of a pass
    /// (0x27A5). Slots without a player record read 0.
    pub fn snapshot_previous_year(&mut self, players: &[PlayerRecord]) {
        for slot in 0..25 {
            let player = players.get(slot);
            self.conquest.previous_production[slot] =
                player.map_or(0, PlayerRecord::production_score_raw);
            self.conquest.previous_planet_count[slot] =
                player.map_or(0, PlayerRecord::planet_count_raw);
        }
    }
    pub fn to_bytes(&self) -> Vec<u8> {
        encode_conquest(&self.conquest).to_vec()
    }
}
