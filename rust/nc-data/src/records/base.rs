use crate::support::{ParseError, copy_array};
use crate::{BASE_RECORD_SIZE, Real48};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseRecord {
    pub raw: [u8; BASE_RECORD_SIZE],
}

impl BaseRecord {
    pub fn new_zeroed() -> Self {
        Self {
            raw: [0; BASE_RECORD_SIZE],
        }
    }
    pub fn local_slot_raw(&self) -> u8 {
        self.raw[0x00]
    }
    pub fn set_local_slot_raw(&mut self, value: u8) {
        self.raw[0x00] = value;
    }
    pub fn active_flag_raw(&self) -> u8 {
        self.raw[0x02]
    }
    pub fn summary_word_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x02], self.raw[0x03]])
    }
    pub fn set_summary_word_raw(&mut self, value: u16) {
        self.raw[0x02..0x04].copy_from_slice(&value.to_le_bytes());
    }
    pub fn set_active_flag_raw(&mut self, value: u8) {
        self.raw[0x02] = value;
    }
    pub fn base_id_raw(&self) -> u8 {
        self.raw[0x04]
    }
    pub fn set_base_id_raw(&mut self, value: u8) {
        self.raw[0x04] = value;
    }
    pub fn link_word_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x05], self.raw[0x06]])
    }
    pub fn set_link_word_raw(&mut self, value: u16) {
        self.raw[0x05..0x07].copy_from_slice(&value.to_le_bytes());
    }
    pub fn chain_word_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x07], self.raw[0x08]])
    }
    pub fn set_chain_word_raw(&mut self, value: u16) {
        self.raw[0x07..0x09].copy_from_slice(&value.to_le_bytes());
    }
    pub fn coords_raw(&self) -> [u8; 2] {
        [self.raw[0x0B], self.raw[0x0C]]
    }
    pub fn set_coords_raw(&mut self, coords: [u8; 2]) {
        self.raw[0x0B..0x0D].copy_from_slice(&coords);
    }
    pub fn tuple_a_payload_raw(&self) -> [u8; 5] {
        copy_array(&self.raw[0x0D..0x12])
    }
    pub fn set_tuple_a_payload_raw(&mut self, payload: [u8; 5]) {
        self.raw[0x0D..0x12].copy_from_slice(&payload);
    }
    pub fn tuple_b_payload_raw(&self) -> [u8; 5] {
        copy_array(&self.raw[0x13..0x18])
    }
    pub fn set_tuple_b_payload_raw(&mut self, payload: [u8; 5]) {
        self.raw[0x13..0x18].copy_from_slice(&payload);
    }
    pub fn tuple_c_payload_raw(&self) -> [u8; 5] {
        copy_array(&self.raw[0x19..0x1E])
    }
    pub fn set_tuple_c_payload_raw(&mut self, payload: [u8; 5]) {
        self.raw[0x19..0x1E].copy_from_slice(&payload);
    }
    /// 0x19..=0x1E: depth into the system, a 6-byte real (1.0 in orbit).
    pub fn depth(&self) -> Real48 {
        Real48::from_bytes(copy_array(&self.raw[0x19..=0x1E]))
    }
    pub fn trailing_coords_raw(&self) -> [u8; 2] {
        copy_array(&self.raw[0x20..0x22])
    }
    pub fn set_trailing_coords_raw(&mut self, coords: [u8; 2]) {
        self.raw[0x20..0x22].copy_from_slice(&coords);
    }
    pub fn owner_empire_raw(&self) -> u8 {
        self.raw[0x22]
    }
    pub fn set_owner_empire_raw(&mut self, value: u8) {
        self.raw[0x22] = value;
    }
    pub fn from_raw(raw: [u8; BASE_RECORD_SIZE]) -> Self {
        Self { raw }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseDat {
    pub records: Vec<BaseRecord>,
}

impl BaseDat {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        if data.len() % BASE_RECORD_SIZE != 0 {
            return Err(ParseError::WrongRecordMultiple {
                file_type: "BASES.DAT",
                record_size: BASE_RECORD_SIZE,
                actual: data.len(),
            });
        }
        Ok(Self {
            records: data
                .chunks_exact(BASE_RECORD_SIZE)
                .map(|chunk| BaseRecord {
                    raw: copy_array(chunk),
                })
                .collect(),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.records
            .iter()
            .flat_map(|record| record.raw)
            .collect::<Vec<_>>()
    }
}
