use crate::BASE_RECORD_SIZE;
use crate::classic::{decode_base, encode_base, parse_bases_dat};
use crate::model::{Coords, Starbase};
use crate::real48::Real48;
use crate::support::ParseError;

/// A [`Starbase`] behind the accessor API call sites still use. The layout
/// is mapped in `docs/dev/ec-starbase-spec.md`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BaseRecord {
    pub base: Starbase,
}

impl BaseRecord {
    pub fn new_zeroed() -> Self {
        Self::default()
    }
    pub fn from_raw(raw: [u8; BASE_RECORD_SIZE]) -> Self {
        Self {
            base: decode_base(&raw),
        }
    }
    /// The classic BASES.DAT encoding of this record.
    pub fn to_bytes(&self) -> [u8; BASE_RECORD_SIZE] {
        encode_base(&self.base)
    }
    /// The escort: the fleet number of the Guard Starbase fleet that rides
    /// along with this base; 0 for none.
    pub fn escort_fleet_slot_raw(&self) -> u16 {
        self.base.escort.0
    }
    pub fn set_escort_fleet_slot_raw(&mut self, value: u16) {
        self.base.escort.0 = value;
    }
    /// The starbase id the player sees ("Starbase 1"). Guard Starbase
    /// fleets name it as their mission target. Rust zeroes the record of a
    /// destroyed base, so a live base has a non-zero id.
    pub fn base_id_raw(&self) -> u16 {
        self.base.base_id.0
    }
    pub fn set_base_id_raw(&mut self, value: u16) {
        self.base.base_id.0 = value;
    }
    pub fn owner_empire_raw(&self) -> u8 {
        self.base.owner.0
    }
    pub fn set_owner_empire_raw(&mut self, value: u8) {
        self.base.owner.0 = value;
    }
    /// Record number of the next base in the owner's chain; 0 ends the
    /// chain.
    pub fn next_base_link_raw(&self) -> u16 {
        self.base.next.0
    }
    pub fn set_next_base_link_raw(&mut self, value: u16) {
        self.base.next.0 = value;
    }
    /// This record's own 1-based record number.
    pub fn record_number_raw(&self) -> u16 {
        self.base.record.0
    }
    pub fn set_record_number_raw(&mut self, value: u16) {
        self.base.record.0 = value;
    }
    /// Record number of the previous base in the owner's chain; 0 at the
    /// head.
    pub fn previous_base_link_raw(&self) -> u16 {
        self.base.prev.0
    }
    pub fn set_previous_base_link_raw(&mut self, value: u16) {
        self.base.prev.0 = value;
    }
    pub fn coords_raw(&self) -> [u8; 2] {
        self.base.sector.to_array()
    }
    pub fn set_coords_raw(&mut self, coords: [u8; 2]) {
        self.base.sector = Coords::from_array(coords);
    }
    /// X offset inside the sector (0.5 is the sector centre).
    pub fn x_fraction(&self) -> Real48 {
        self.base.x_fraction
    }
    pub fn set_x_fraction(&mut self, value: Real48) {
        self.base.x_fraction = value;
    }
    pub fn y_fraction(&self) -> Real48 {
        self.base.y_fraction
    }
    pub fn set_y_fraction(&mut self, value: Real48) {
        self.base.y_fraction = value;
    }
    /// Depth: 0.0 in deep space, 1.0 in orbit.
    pub fn depth(&self) -> Real48 {
        self.base.depth
    }
    pub fn set_depth(&mut self, value: Real48) {
        self.base.depth = value;
    }
    /// At rest in orbit at the sector centre: offsets 0.5, depth 1.0.
    pub fn set_at_rest_motion(&mut self) {
        self.set_x_fraction(Real48::HALF);
        self.set_y_fraction(Real48::HALF);
        self.set_depth(Real48::ONE);
    }
    /// 1 while the base is under way to its target, 0 once it has arrived
    /// or been halted.
    pub fn moving_flag_raw(&self) -> u8 {
        u8::from(self.base.moving)
    }
    pub fn set_moving_flag_raw(&mut self, value: u8) {
        self.base.moving = value != 0;
    }
    pub fn target_coords_raw(&self) -> [u8; 2] {
        self.base.target.to_array()
    }
    pub fn set_target_coords_raw(&mut self, coords: [u8; 2]) {
        self.base.target = Coords::from_array(coords);
    }
    /// Target depth layer: 1 in orbit, 0 in deep space.
    pub fn target_depth_raw(&self) -> u8 {
        self.base.target_depth
    }
    pub fn set_target_depth_raw(&mut self, value: u8) {
        self.base.target_depth = value;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseDat {
    pub records: Vec<BaseRecord>,
}

impl BaseDat {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        Ok(Self {
            records: parse_bases_dat(data)?
                .into_iter()
                .map(|base| BaseRecord { base })
                .collect(),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.records
            .iter()
            .flat_map(|record| encode_base(&record.base))
            .collect()
    }
}
