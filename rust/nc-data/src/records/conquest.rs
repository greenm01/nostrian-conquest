use crate::CONQUEST_DAT_SIZE;
use crate::support::{ParseError, copy_array, expect_size};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConquestDat {
    pub raw: [u8; CONQUEST_DAT_SIZE],
}

impl ConquestDat {
    const INACTIVE_PRODUCTION_SLOT_OFFSETS: [usize; 3] = [0x0C, 0x0E, 0x10];

    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        expect_size(data, CONQUEST_DAT_SIZE, "CONQUEST.DAT")?;
        Ok(Self {
            raw: copy_array(data),
        })
    }

    pub fn control_header(&self) -> &[u8] {
        &self.raw[..0x55]
    }
    pub fn set_control_header_bytes(&mut self, value: &[u8; 0x55]) {
        self.raw[..0x55].copy_from_slice(value);
    }
    pub fn game_year(&self) -> u16 {
        u16::from_le_bytes([self.raw[0], self.raw[1]])
    }
    pub fn player_count(&self) -> u8 {
        self.raw[2]
    }
    pub fn player_config_word(&self) -> u16 {
        u16::from_le_bytes([self.raw[2], self.raw[3]])
    }
    pub fn maintenance_schedule_bytes(&self) -> [u8; 7] {
        self.raw[3..10]
            .try_into()
            .expect("maintenance schedule should be 7 bytes")
    }
    /// Sunday through Saturday; ECMAINT runs on a day whose byte is nonzero.
    pub fn maintenance_schedule_enabled(&self) -> [bool; 7] {
        self.maintenance_schedule_bytes().map(|byte| byte != 0)
    }
    pub fn set_maintenance_schedule_bytes(&mut self, bytes: [u8; 7]) {
        self.raw[3..10].copy_from_slice(&bytes);
    }
    /// Writes 1 for an enabled day and 0 otherwise, as ECUTIL does.
    pub fn set_maintenance_schedule_enabled(&mut self, enabled: [bool; 7]) {
        for (idx, enabled) in enabled.into_iter().enumerate() {
            self.raw[3 + idx] = u8::from(enabled);
        }
    }
    pub fn header_words(&self) -> Vec<u16> {
        self.control_header()
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect()
    }

    /// Set the game year (offset 0x00..0x01, little-endian u16).
    pub fn set_game_year(&mut self, year: u16) {
        self.raw[0..2].copy_from_slice(&year.to_le_bytes());
    }

    /// Set the player count (offset 0x02).
    pub fn set_player_count(&mut self, count: u8) {
        self.raw[2] = count;
    }

    pub fn inactive_production_slot_raw(&self, slot: usize) -> Option<u16> {
        Self::INACTIVE_PRODUCTION_SLOT_OFFSETS
            .get(slot)
            .copied()
            .map(|offset| self.raw_word(offset))
    }

    pub fn set_inactive_production_slot_raw(&mut self, slot: usize, value: u16) -> bool {
        if let Some(offset) = Self::INACTIVE_PRODUCTION_SLOT_OFFSETS.get(slot).copied() {
            self.set_raw_word(offset, value);
            true
        } else {
            false
        }
    }

    pub fn control_word_12_raw(&self) -> u16 {
        self.raw_word(0x12)
    }
    pub fn set_control_word_12_raw(&mut self, value: u16) {
        self.set_raw_word(0x12, value);
    }
    pub fn control_word_1a_raw(&self) -> u16 {
        self.raw_word(0x1A)
    }
    pub fn set_control_word_1a_raw(&mut self, value: u16) {
        self.set_raw_word(0x1A, value);
    }
    pub fn control_word_20_raw(&self) -> u16 {
        self.raw_word(0x20)
    }
    pub fn set_control_word_20_raw(&mut self, value: u16) {
        self.set_raw_word(0x20, value);
    }
    pub fn control_word_22_raw(&self) -> u16 {
        self.raw_word(0x22)
    }
    pub fn set_control_word_22_raw(&mut self, value: u16) {
        self.set_raw_word(0x22, value);
    }
    pub fn control_word_26_raw(&self) -> u16 {
        self.raw_word(0x26)
    }
    pub fn set_control_word_26_raw(&mut self, value: u16) {
        self.set_raw_word(0x26, value);
    }
    pub fn control_word_28_raw(&self) -> u16 {
        self.raw_word(0x28)
    }
    pub fn set_control_word_28_raw(&mut self, value: u16) {
        self.set_raw_word(0x28, value);
    }
    pub fn control_word_36_raw(&self) -> u16 {
        self.raw_word(0x36)
    }
    pub fn set_control_word_36_raw(&mut self, value: u16) {
        self.set_raw_word(0x36, value);
    }
    pub fn control_word_38_raw(&self) -> u16 {
        self.raw_word(0x38)
    }
    pub fn set_control_word_38_raw(&mut self, value: u16) {
        self.set_raw_word(0x38, value);
    }
    pub fn control_word_3a_raw(&self) -> u16 {
        self.raw_word(0x3A)
    }
    pub fn set_control_word_3a_raw(&mut self, value: u16) {
        self.set_raw_word(0x3A, value);
    }
    pub fn control_word_40_raw(&self) -> u16 {
        self.raw_word(0x40)
    }
    pub fn set_control_word_40_raw(&mut self, value: u16) {
        self.set_raw_word(0x40, value);
    }
    pub fn control_word_52_raw(&self) -> u16 {
        self.raw_word(0x52)
    }
    pub fn set_control_word_52_raw(&mut self, value: u16) {
        self.set_raw_word(0x52, value);
    }

    pub fn control_byte_3d_raw(&self) -> u8 {
        self.raw_byte(0x3D)
    }
    pub fn set_control_byte_3d_raw(&mut self, value: u8) {
        self.set_raw_byte(0x3D, value);
    }
    pub fn control_byte_44_raw(&self) -> u8 {
        self.raw_byte(0x44)
    }
    pub fn set_control_byte_44_raw(&mut self, value: u8) {
        self.set_raw_byte(0x44, value);
    }
    pub fn control_byte_47_raw(&self) -> u8 {
        self.raw_byte(0x47)
    }
    pub fn set_control_byte_47_raw(&mut self, value: u8) {
        self.set_raw_byte(0x47, value);
    }
    pub fn control_byte_48_raw(&self) -> u8 {
        self.raw_byte(0x48)
    }
    pub fn set_control_byte_48_raw(&mut self, value: u8) {
        self.set_raw_byte(0x48, value);
    }
    pub fn control_byte_4a_raw(&self) -> u8 {
        self.raw_byte(0x4A)
    }
    pub fn set_control_byte_4a_raw(&mut self, value: u8) {
        self.set_raw_byte(0x4A, value);
    }
    pub fn control_byte_4b_raw(&self) -> u8 {
        self.raw_byte(0x4B)
    }
    pub fn set_control_byte_4b_raw(&mut self, value: u8) {
        self.set_raw_byte(0x4B, value);
    }
    pub fn control_byte_54_raw(&self) -> u8 {
        self.raw_byte(0x54)
    }
    pub fn set_control_byte_54_raw(&mut self, value: u8) {
        self.set_raw_byte(0x54, value);
    }

    pub fn clear_control_byte_if_equal(&mut self, offset: usize, value: u8) {
        self.clear_raw_byte_if_equal(offset, value);
    }

    pub fn raw_byte(&self, offset: usize) -> u8 {
        self.raw[offset]
    }

    pub fn set_raw_byte(&mut self, offset: usize, value: u8) {
        self.raw[offset] = value;
    }

    pub fn raw_word(&self, offset: usize) -> u16 {
        u16::from_le_bytes([self.raw[offset], self.raw[offset + 1]])
    }

    pub fn set_raw_word(&mut self, offset: usize, value: u16) {
        self.raw[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    pub fn clear_raw_byte_if_equal(&mut self, offset: usize, value: u8) {
        if self.raw_byte(offset) == value {
            self.set_raw_byte(offset, 0);
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.raw.to_vec()
    }
}
