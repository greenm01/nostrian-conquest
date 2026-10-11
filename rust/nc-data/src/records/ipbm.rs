use crate::IPBM_RECORD_SIZE;
use crate::classic::{decode_ipbm, encode_ipbm};
use crate::model::{Coords, Ipbm};
use crate::real48::Real48;

impl Ipbm {
    pub fn from_raw(raw: [u8; IPBM_RECORD_SIZE]) -> Self {
        decode_ipbm(&raw)
    }
    /// The classic IPBM.DAT encoding of this record.
    pub fn to_bytes(&self) -> [u8; IPBM_RECORD_SIZE] {
        encode_ipbm(self)
    }
    /// The missile's number within its empire.
    pub fn ipbm_number_raw(&self) -> u16 {
        self.ipbm_number
    }
    pub fn set_ipbm_number_raw(&mut self, value: u16) {
        self.ipbm_number = value;
    }
    pub fn owner_empire_raw(&self) -> u8 {
        self.owner.0
    }
    pub fn set_owner_empire_raw(&mut self, value: u8) {
        self.owner.0 = value;
    }
    /// Record number of the next IPBM in the owner's chain; 0 ends it.
    pub fn next_ipbm_link_raw(&self) -> u16 {
        self.next.0
    }
    pub fn set_next_ipbm_link_raw(&mut self, value: u16) {
        self.next.0 = value;
    }
    /// This record's own 1-based record number.
    pub fn record_number_raw(&self) -> u16 {
        self.record.0
    }
    pub fn set_record_number_raw(&mut self, value: u16) {
        self.record.0 = value;
    }
    /// Record number of the previous IPBM in the chain; 0 at the head.
    pub fn previous_ipbm_link_raw(&self) -> u16 {
        self.prev.0
    }
    pub fn set_previous_ipbm_link_raw(&mut self, value: u16) {
        self.prev.0 = value;
    }
    pub fn coords_raw(&self) -> [u8; 2] {
        self.sector.to_array()
    }
    pub fn set_coords_raw(&mut self, coords: [u8; 2]) {
        self.sector = Coords::from_array(coords);
    }
    pub fn x_fraction(&self) -> Real48 {
        self.x_fraction
    }
    pub fn set_x_fraction(&mut self, value: Real48) {
        self.x_fraction = value;
    }
    pub fn y_fraction(&self) -> Real48 {
        self.y_fraction
    }
    pub fn set_y_fraction(&mut self, value: Real48) {
        self.y_fraction = value;
    }
    pub fn depth(&self) -> Real48 {
        self.depth
    }
    pub fn set_depth(&mut self, value: Real48) {
        self.depth = value;
    }
    pub fn target_coords_raw(&self) -> [u8; 2] {
        self.target.to_array()
    }
    pub fn set_target_coords_raw(&mut self, coords: [u8; 2]) {
        self.target = Coords::from_array(coords);
    }
    pub fn target_depth_raw(&self) -> u8 {
        self.target_depth
    }
    pub fn set_target_depth_raw(&mut self, value: u8) {
        self.target_depth = value;
    }
}
