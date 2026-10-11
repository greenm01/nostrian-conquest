use crate::Real48;

use super::{Coords, EmpireId};

/// A unit kind as the build queue stores it, 1 to 9; any other byte is
/// `Unknown`, and an empty slot holds `Unknown(0)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductionItemKind {
    Destroyer,
    Cruiser,
    Battleship,
    Scout,
    Transport,
    Etac,
    GroundBattery,
    Army,
    Starbase,
    Unknown(u8),
}

impl ProductionItemKind {
    pub fn from_code(raw: u8) -> Self {
        match raw {
            1 => Self::Destroyer,
            2 => Self::Cruiser,
            3 => Self::Battleship,
            4 => Self::Scout,
            5 => Self::Transport,
            6 => Self::Etac,
            7 => Self::GroundBattery,
            8 => Self::Army,
            9 => Self::Starbase,
            other => Self::Unknown(other),
        }
    }

    pub fn code(self) -> u8 {
        match self {
            Self::Destroyer => 1,
            Self::Cruiser => 2,
            Self::Battleship => 3,
            Self::Scout => 4,
            Self::Transport => 5,
            Self::Etac => 6,
            Self::GroundBattery => 7,
            Self::Army => 8,
            Self::Starbase => 9,
            Self::Unknown(raw) => raw,
        }
    }

    pub fn requires_stardock(self) -> bool {
        matches!(
            self,
            Self::Destroyer
                | Self::Cruiser
                | Self::Battleship
                | Self::Scout
                | Self::Transport
                | Self::Etac
                | Self::Starbase
        )
    }

    pub fn build_cost(self) -> Option<u32> {
        match self {
            Self::Destroyer => Some(5),
            Self::Cruiser => Some(15),
            Self::Battleship => Some(45),
            Self::Scout => Some(15),
            Self::Transport => Some(5),
            Self::Etac => Some(20),
            Self::GroundBattery => Some(20),
            Self::Army => Some(2),
            Self::Starbase => Some(50),
            Self::Unknown(_) => None,
        }
    }
}

/// One slot of Rust's build queue: production points still owed on an
/// order and the unit kind ordered. The yearly pass pays each slot from the
/// planet's budget, so an order can span years.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildSlot {
    /// PLANETS.DAT 0x24 + slot.
    pub points_owed: u8,
    /// PLANETS.DAT 0x2E + slot.
    pub kind: ProductionItemKind,
}

impl BuildSlot {
    /// Nothing owed and kind byte 0.
    pub const EMPTY: Self = Self {
        points_owed: 0,
        kind: ProductionItemKind::Unknown(0),
    };
}

impl Default for BuildSlot {
    fn default() -> Self {
        Self::EMPTY
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

/// A stardock count by slot, 0 to 7 in classic's kind order.
impl std::ops::Index<usize> for Stardock {
    type Output = u32;

    fn index(&self, slot: usize) -> &u32 {
        match slot {
            0 => &self.destroyers,
            1 => &self.cruisers,
            2 => &self.battleships,
            3 => &self.scouts,
            4 => &self.transports,
            5 => &self.etacs,
            6 => &self.starbases,
            7 => &self.ipbms,
            _ => panic!("stardock slot {slot} out of range"),
        }
    }
}

impl std::ops::IndexMut<usize> for Stardock {
    fn index_mut(&mut self, slot: usize) -> &mut u32 {
        match slot {
            0 => &mut self.destroyers,
            1 => &mut self.cruisers,
            2 => &mut self.battleships,
            3 => &mut self.scouts,
            4 => &mut self.transports,
            5 => &mut self.etacs,
            6 => &mut self.starbases,
            7 => &mut self.ipbms,
            _ => panic!("stardock slot {slot} out of range"),
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
    /// 0x24: Rust's ten build slots. Classic reads these bytes as ten u16
    /// counts of units ordered, by kind; translating between the two is
    /// t056's job, so until then the codec carries the slot bytes.
    pub build_queue: [BuildSlot; 10],
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
