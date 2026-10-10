use crate::Real48;

use super::{Coords, EmpireId, RecordNumber};

/// One IPBM.DAT record: a fleet-shaped record for a feature no v1.5 program
/// completes. Nothing creates one; the model keeps it for file round trips.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ipbm {
    /// 0x00: the missile's number within its empire.
    pub ipbm_number: u16,
    /// 0x02: owning empire.
    pub owner: EmpireId,
    /// 0x03: next IPBM in the owner's chain.
    pub next: RecordNumber,
    /// 0x05: this record's own number.
    pub record: RecordNumber,
    /// 0x07: previous IPBM in the chain.
    pub prev: RecordNumber,
    /// 0x09: sector.
    pub sector: Coords,
    /// 0x0B: in-sector x fraction.
    pub x_fraction: Real48,
    /// 0x11: in-sector y fraction.
    pub y_fraction: Real48,
    /// 0x17: depth, 0 in deep space and 1 in orbit.
    pub depth: Real48,
    /// 0x1D: target sector.
    pub target: Coords,
    /// 0x1F: target depth.
    pub target_depth: u8,
}
