use crate::Real48;

use super::{Coords, CrippledHulls, EmpireId, FleetId, RecordNumber, StarbaseId};

/// One BASES.DAT record. Records form one doubly linked chain per empire,
/// headed by the player's `bases_first` and `bases_last`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Starbase {
    /// 0x00: the Guard Starbase fleet riding with the base; 0 for none.
    pub escort: FleetId,
    /// 0x02: "Starbase 1".
    pub base_id: StarbaseId,
    /// 0x04: owning empire.
    pub owner: EmpireId,
    /// 0x05: next base in the owner's chain.
    pub next: RecordNumber,
    /// 0x07: this record's own number.
    pub record: RecordNumber,
    /// 0x09: previous base in the chain.
    pub prev: RecordNumber,
    /// 0x0B: sector.
    pub sector: Coords,
    /// 0x0D: in-sector x fraction; 0.5 is the center.
    pub x_fraction: Real48,
    /// 0x13: in-sector y fraction.
    pub y_fraction: Real48,
    /// 0x19: depth, 0 in deep space and 1 in orbit.
    pub depth: Real48,
    /// 0x1F: under way to its target.
    pub moving: bool,
    /// 0x20: target sector.
    pub target: Coords,
    /// 0x22: target depth, 1 orbit or 0 deep space.
    pub target_depth: u8,
    /// Whether battle has crippled the base (at most one hull), and how long
    /// it needs to recover. Not a classic field: an export writes it whole.
    pub crippled: CrippledHulls,
}
