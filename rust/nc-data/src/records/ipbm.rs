use crate::IPBM_RECORD_SIZE;
use crate::classic::{decode_ipbm, encode_ipbm, parse_ipbm_dat};
use crate::model::{Coords, Ipbm};
use crate::real48::Real48;
use crate::support::ParseError;

/// An [`Ipbm`] behind the accessor API call sites still use.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IpbmRecord {
    pub ipbm: Ipbm,
}

impl IpbmRecord {
    pub fn new_zeroed() -> Self {
        Self::default()
    }
    pub fn from_raw(raw: [u8; IPBM_RECORD_SIZE]) -> Self {
        Self {
            ipbm: decode_ipbm(&raw),
        }
    }
    /// The classic IPBM.DAT encoding of this record.
    pub fn to_bytes(&self) -> [u8; IPBM_RECORD_SIZE] {
        encode_ipbm(&self.ipbm)
    }
    /// The missile's number within its empire.
    pub fn ipbm_number_raw(&self) -> u16 {
        self.ipbm.ipbm_number
    }
    pub fn set_ipbm_number_raw(&mut self, value: u16) {
        self.ipbm.ipbm_number = value;
    }
    pub fn owner_empire_raw(&self) -> u8 {
        self.ipbm.owner.0
    }
    pub fn set_owner_empire_raw(&mut self, value: u8) {
        self.ipbm.owner.0 = value;
    }
    /// Record number of the next IPBM in the owner's chain; 0 ends it.
    pub fn next_ipbm_link_raw(&self) -> u16 {
        self.ipbm.next.0
    }
    pub fn set_next_ipbm_link_raw(&mut self, value: u16) {
        self.ipbm.next.0 = value;
    }
    /// This record's own 1-based record number.
    pub fn record_number_raw(&self) -> u16 {
        self.ipbm.record.0
    }
    pub fn set_record_number_raw(&mut self, value: u16) {
        self.ipbm.record.0 = value;
    }
    /// Record number of the previous IPBM in the chain; 0 at the head.
    pub fn previous_ipbm_link_raw(&self) -> u16 {
        self.ipbm.prev.0
    }
    pub fn set_previous_ipbm_link_raw(&mut self, value: u16) {
        self.ipbm.prev.0 = value;
    }
    pub fn coords_raw(&self) -> [u8; 2] {
        self.ipbm.sector.to_array()
    }
    pub fn set_coords_raw(&mut self, coords: [u8; 2]) {
        self.ipbm.sector = Coords::from_array(coords);
    }
    pub fn x_fraction(&self) -> Real48 {
        self.ipbm.x_fraction
    }
    pub fn set_x_fraction(&mut self, value: Real48) {
        self.ipbm.x_fraction = value;
    }
    pub fn y_fraction(&self) -> Real48 {
        self.ipbm.y_fraction
    }
    pub fn set_y_fraction(&mut self, value: Real48) {
        self.ipbm.y_fraction = value;
    }
    pub fn depth(&self) -> Real48 {
        self.ipbm.depth
    }
    pub fn set_depth(&mut self, value: Real48) {
        self.ipbm.depth = value;
    }
    pub fn target_coords_raw(&self) -> [u8; 2] {
        self.ipbm.target.to_array()
    }
    pub fn set_target_coords_raw(&mut self, coords: [u8; 2]) {
        self.ipbm.target = Coords::from_array(coords);
    }
    pub fn target_depth_raw(&self) -> u8 {
        self.ipbm.target_depth
    }
    pub fn set_target_depth_raw(&mut self, value: u8) {
        self.ipbm.target_depth = value;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpbmDat {
    pub records: Vec<IpbmRecord>,
}

impl IpbmDat {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        Ok(Self {
            records: parse_ipbm_dat(data)?
                .into_iter()
                .map(|ipbm| IpbmRecord { ipbm })
                .collect(),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.records
            .iter()
            .flat_map(|record| encode_ipbm(&record.ipbm))
            .collect()
    }
}
