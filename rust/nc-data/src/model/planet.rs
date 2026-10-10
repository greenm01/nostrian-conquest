use crate::Real48;

use super::{Coords, EmpireId};

/// Units ordered this year, PLANETS.DAT 0x24: ten words in classic's kind
/// order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BuildQueue {
    /// 0x24, kind 1
    pub destroyers: u16,
    /// 0x26, kind 2
    pub cruisers: u16,
    /// 0x28, kind 3
    pub battleships: u16,
    /// 0x2A, kind 4
    pub scouts: u16,
    /// 0x2C, kind 5
    pub transports: u16,
    /// 0x2E, kind 6
    pub etacs: u16,
    /// 0x30, kind 7
    pub starbases: u16,
    /// 0x32, kind 8: ECGAME refuses it ("reserved for future use").
    pub reserved: u16,
    /// 0x34, kind 9
    pub armies: u16,
    /// 0x36, kind 10
    pub ground_batteries: u16,
}

impl BuildQueue {
    pub const fn to_words(self) -> [u16; 10] {
        [
            self.destroyers,
            self.cruisers,
            self.battleships,
            self.scouts,
            self.transports,
            self.etacs,
            self.starbases,
            self.reserved,
            self.armies,
            self.ground_batteries,
        ]
    }

    pub const fn from_words(words: [u16; 10]) -> Self {
        Self {
            destroyers: words[0],
            cruisers: words[1],
            battleships: words[2],
            scouts: words[3],
            transports: words[4],
            etacs: words[5],
            starbases: words[6],
            reserved: words[7],
            armies: words[8],
            ground_batteries: words[9],
        }
    }
}

/// Built units waiting to be commissioned, PLANETS.DAT 0x38: eight u32
/// counts in classic's kind order. The yearly pass clamps each at 65535.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stardock {
    /// 0x38
    pub destroyers: u32,
    /// 0x3C
    pub cruisers: u32,
    /// 0x40
    pub battleships: u32,
    /// 0x44
    pub scouts: u32,
    /// 0x48
    pub transports: u32,
    /// 0x4C
    pub etacs: u32,
    /// 0x50
    pub starbases: u32,
    /// 0x54: nothing builds one; the bombard report names the slot IPBMs.
    pub ipbms: u32,
}

impl Stardock {
    pub const fn to_counts(self) -> [u32; 8] {
        [
            self.destroyers,
            self.cruisers,
            self.battleships,
            self.scouts,
            self.transports,
            self.etacs,
            self.starbases,
            self.ipbms,
        ]
    }

    pub const fn from_counts(counts: [u32; 8]) -> Self {
        Self {
            destroyers: counts[0],
            cruisers: counts[1],
            battleships: counts[2],
            scouts: counts[3],
            transports: counts[4],
            etacs: counts[5],
            starbases: counts[6],
            ipbms: counts[7],
        }
    }
}

/// One PLANETS.DAT record, in map order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Planet {
    /// 0x00: sector.
    pub sector: Coords,
    /// 0x02: ceiling for present production.
    pub potential_production: u8,
    /// 0x03: present production ("factories"), P.
    pub production: Real48,
    /// 0x09: stored production points, G.
    pub stored_goods: Real48,
    /// 0x0F: `String[20]`; "Unowned" or "Not Named Yet" until renamed.
    pub name: String,
    /// 0x24: units ordered this year.
    pub build_queue: BuildQueue,
    /// 0x38: built units waiting to be commissioned.
    pub stardock: Stardock,
    /// 0x58: armies on the surface.
    pub armies: u16,
    /// 0x5A: ground batteries.
    pub ground_batteries: u16,
    /// 0x5C: years under the current owner, capped at 2: 0 just captured
    /// (no income), 1 recently captured (half), 2 fully functional.
    pub years_under_owner: u8,
    /// 0x5D: owning empire, 0 for none.
    pub owner: EmpireId,
    /// 0x60: the Scorch-Earth order.
    pub scorch_earth: bool,
}
