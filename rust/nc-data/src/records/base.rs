use crate::BASE_RECORD_SIZE;
use crate::real48::Real48;
use crate::support::{ParseError, copy_array};

/// One 35-byte BASES.DAT record. The layout is mapped in
/// `docs/dev/ec-starbase-spec.md`.
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
    /// 0x00..=0x01: the escort, the local slot word (FLEETS 0x00) of the
    /// Guard Starbase fleet that rides along with this base; 0 for none.
    pub fn escort_fleet_slot_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x00], self.raw[0x01]])
    }
    pub fn set_escort_fleet_slot_raw(&mut self, value: u16) {
        self.raw[0x00..0x02].copy_from_slice(&value.to_le_bytes());
    }
    /// 0x02..=0x03: the starbase id the player sees ("Starbase 1"). Guard
    /// Starbase fleets name it in FLEETS 0x23. Rust zeroes the record of a
    /// destroyed base, so a live base has a non-zero id.
    pub fn base_id_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x02], self.raw[0x03]])
    }
    pub fn set_base_id_raw(&mut self, value: u16) {
        self.raw[0x02..0x04].copy_from_slice(&value.to_le_bytes());
    }
    /// 0x04: owning empire.
    pub fn owner_empire_raw(&self) -> u8 {
        self.raw[0x04]
    }
    pub fn set_owner_empire_raw(&mut self, value: u8) {
        self.raw[0x04] = value;
    }
    /// 0x05..=0x06: record number of the next base in the owner's chain; 0
    /// ends the chain.
    pub fn next_base_link_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x05], self.raw[0x06]])
    }
    pub fn set_next_base_link_raw(&mut self, value: u16) {
        self.raw[0x05..0x07].copy_from_slice(&value.to_le_bytes());
    }
    /// 0x07..=0x08: this record's own 1-based record number.
    pub fn record_number_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x07], self.raw[0x08]])
    }
    pub fn set_record_number_raw(&mut self, value: u16) {
        self.raw[0x07..0x09].copy_from_slice(&value.to_le_bytes());
    }
    /// 0x09..=0x0A: record number of the previous base in the owner's chain;
    /// 0 at the head.
    pub fn previous_base_link_raw(&self) -> u16 {
        u16::from_le_bytes([self.raw[0x09], self.raw[0x0A]])
    }
    pub fn set_previous_base_link_raw(&mut self, value: u16) {
        self.raw[0x09..0x0B].copy_from_slice(&value.to_le_bytes());
    }
    /// 0x0B..=0x0C: current sector x, y.
    pub fn coords_raw(&self) -> [u8; 2] {
        [self.raw[0x0B], self.raw[0x0C]]
    }
    pub fn set_coords_raw(&mut self, coords: [u8; 2]) {
        self.raw[0x0B..0x0D].copy_from_slice(&coords);
    }
    /// 0x0D..=0x12: X offset inside the sector, a 6-byte real (0.5 is the
    /// sector centre).
    pub fn x_fraction(&self) -> Real48 {
        Real48::from_bytes(copy_array(&self.raw[0x0D..=0x12]))
    }
    pub fn set_x_fraction(&mut self, value: Real48) {
        self.raw[0x0D..=0x12].copy_from_slice(&value.to_bytes());
    }
    /// 0x13..=0x18: Y offset inside the sector, a 6-byte real.
    pub fn y_fraction(&self) -> Real48 {
        Real48::from_bytes(copy_array(&self.raw[0x13..=0x18]))
    }
    pub fn set_y_fraction(&mut self, value: Real48) {
        self.raw[0x13..=0x18].copy_from_slice(&value.to_bytes());
    }
    /// 0x19..=0x1E: depth, a 6-byte real: 0.0 in deep space, 1.0 in orbit.
    pub fn depth(&self) -> Real48 {
        Real48::from_bytes(copy_array(&self.raw[0x19..=0x1E]))
    }
    pub fn set_depth(&mut self, value: Real48) {
        self.raw[0x19..=0x1E].copy_from_slice(&value.to_bytes());
    }
    /// At rest in orbit at the sector centre: offsets 0.5, depth 1.0.
    pub fn set_at_rest_motion(&mut self) {
        self.set_x_fraction(Real48::HALF);
        self.set_y_fraction(Real48::HALF);
        self.set_depth(Real48::ONE);
    }
    /// 0x1F: 1 while the base is under way to its target, 0 once it has
    /// arrived or been halted.
    pub fn moving_flag_raw(&self) -> u8 {
        self.raw[0x1F]
    }
    pub fn set_moving_flag_raw(&mut self, value: u8) {
        self.raw[0x1F] = value;
    }
    /// 0x20..=0x21: target sector x, y.
    pub fn target_coords_raw(&self) -> [u8; 2] {
        [self.raw[0x20], self.raw[0x21]]
    }
    pub fn set_target_coords_raw(&mut self, coords: [u8; 2]) {
        self.raw[0x20..0x22].copy_from_slice(&coords);
    }
    /// 0x22: target depth layer: 1 in orbit, 0 in deep space.
    pub fn target_depth_raw(&self) -> u8 {
        self.raw[0x22]
    }
    pub fn set_target_depth_raw(&mut self, value: u8) {
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
