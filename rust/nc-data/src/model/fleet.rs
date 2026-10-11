use crate::Real48;

use super::{Coords, CrippledShips, EmpireId, FleetId, RecordNumber};

/// A fleet's standing order, FLEETS.DAT 0x1F.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Order {
    #[default]
    HoldPosition,
    MoveOnly,
    SeekHome,
    PatrolSector,
    GuardStarbase,
    GuardBlockadeWorld,
    BombardWorld,
    InvadeWorld,
    BlitzWorld,
    ViewWorld,
    ScoutSector,
    ScoutSolarSystem,
    ColonizeWorld,
    JoinAnotherFleet,
    RendezvousSector,
    Salvage,
    Unknown(u8),
}

impl Order {
    pub fn from_code(raw: u8) -> Self {
        match raw {
            0 => Self::HoldPosition,
            1 => Self::MoveOnly,
            2 => Self::SeekHome,
            3 => Self::PatrolSector,
            4 => Self::GuardStarbase,
            5 => Self::GuardBlockadeWorld,
            6 => Self::BombardWorld,
            7 => Self::InvadeWorld,
            8 => Self::BlitzWorld,
            9 => Self::ViewWorld,
            10 => Self::ScoutSector,
            11 => Self::ScoutSolarSystem,
            12 => Self::ColonizeWorld,
            13 => Self::JoinAnotherFleet,
            14 => Self::RendezvousSector,
            15 => Self::Salvage,
            other => Self::Unknown(other),
        }
    }

    pub fn code(self) -> u8 {
        match self {
            Self::HoldPosition => 0,
            Self::MoveOnly => 1,
            Self::SeekHome => 2,
            Self::PatrolSector => 3,
            Self::GuardStarbase => 4,
            Self::GuardBlockadeWorld => 5,
            Self::BombardWorld => 6,
            Self::InvadeWorld => 7,
            Self::BlitzWorld => 8,
            Self::ViewWorld => 9,
            Self::ScoutSector => 10,
            Self::ScoutSolarSystem => 11,
            Self::ColonizeWorld => 12,
            Self::JoinAnotherFleet => 13,
            Self::RendezvousSector => 14,
            Self::Salvage => 15,
            Self::Unknown(raw) => raw,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::HoldPosition => "hold",
            Self::MoveOnly => "move",
            Self::SeekHome => "seek_home",
            Self::PatrolSector => "patrol",
            Self::GuardStarbase => "guard_starbase",
            Self::GuardBlockadeWorld => "guard_blockade",
            Self::BombardWorld => "bombard",
            Self::InvadeWorld => "invade",
            Self::BlitzWorld => "blitz",
            Self::ViewWorld => "view",
            Self::ScoutSector => "scout_sector",
            Self::ScoutSolarSystem => "scout_system",
            Self::ColonizeWorld => "colonize",
            Self::JoinAnotherFleet => "join_fleet",
            Self::RendezvousSector => "rendezvous",
            Self::Salvage => "salvage",
            Self::Unknown(_) => "unknown",
        }
    }

    /// ECGAME writes `0x22` as 1 for every order whose target is a world
    /// or fleet and 0 for orders that target a bare sector.
    pub fn targets_world(self) -> bool {
        !matches!(
            self,
            Self::MoveOnly
                | Self::PatrolSector
                | Self::ScoutSector
                | Self::RendezvousSector
                | Self::Unknown(_)
        )
    }

    pub fn display_label(self) -> &'static str {
        match self {
            Self::HoldPosition => "Hold position",
            Self::MoveOnly => "Move fleet",
            Self::SeekHome => "Seek home",
            Self::PatrolSector => "Patrol sector",
            Self::GuardStarbase => "Guard starbase",
            Self::GuardBlockadeWorld => "Guard/blockade world",
            Self::BombardWorld => "Bombard world",
            Self::InvadeWorld => "Invade world",
            Self::BlitzWorld => "Blitz world",
            Self::ViewWorld => "View world",
            Self::ScoutSector => "Scout sector",
            Self::ScoutSolarSystem => "Scout solar system",
            Self::ColonizeWorld => "Colonize world",
            Self::JoinAnotherFleet => "Join another fleet",
            Self::RendezvousSector => "Rendezvous at sector",
            Self::Salvage => "Salvage",
            Self::Unknown(_) => "Unknown order",
        }
    }
}

/// Ships in a fleet, FLEETS.DAT 0x26..0x33.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ShipCounts {
    /// 0x26
    pub battleships: u16,
    /// 0x28
    pub cruisers: u16,
    /// 0x2A
    pub destroyers: u16,
    /// 0x2C: troop transports without an army.
    pub empty_transports: u16,
    /// 0x2E: troop transports carrying one army each.
    pub loaded_transports: u16,
    /// 0x30
    pub etacs: u16,
    /// 0x32
    pub scouts: u16,
}

/// One FLEETS.DAT record. Records form one doubly linked chain per empire,
/// headed by the player's `fleets_first` and `fleets_last`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Fleet {
    /// 0x00: the fleet's number within its empire ("Fleet 1").
    pub fleet_number: FleetId,
    /// 0x02: owning empire.
    pub owner: EmpireId,
    /// 0x03: next fleet in the owner's chain.
    pub next: RecordNumber,
    /// 0x05: this record's own number.
    pub record: RecordNumber,
    /// 0x07: previous fleet in the chain.
    pub prev: RecordNumber,
    /// 0x09: the slowest ship type present.
    pub max_speed: u8,
    /// 0x0A: current speed.
    pub speed: u8,
    /// 0x0B: sector.
    pub sector: Coords,
    /// 0x0D: in-sector x fraction; 0.5 is the center.
    pub x_fraction: Real48,
    /// 0x13: in-sector y fraction.
    pub y_fraction: Real48,
    /// 0x19: depth, 0 in deep space and 1 in orbit.
    pub depth: Real48,
    /// 0x1F: standing order.
    pub order: Order,
    /// 0x20: target sector.
    pub target: Coords,
    /// 0x22: target depth, 1 for a world and 0 for a bare sector.
    pub target_depth: u8,
    /// 0x23: the Join host's fleet number or the guarded starbase's id.
    pub mission_target: u16,
    /// 0x25: rules of engagement.
    pub rules_of_engagement: u8,
    /// 0x26: ships aboard.
    pub ships: ShipCounts,
    /// The ships among `ships` that battle has crippled, and how long each
    /// needs to recover. Not a classic field: an export writes whole hulls.
    pub crippled: CrippledShips,
    /// The escape the fleet is making from a battle, if any. Not a classic
    /// field: an export drops it.
    pub escape: Option<EscapeEpisode>,
}

/// A fleet's escape from a battle: it broke off in `sector` and `layer`
/// and is leaving. Each opposing empire catches it in at most one pursuit
/// exchange while the episode lasts, and an empire that has had its
/// exchange cannot attack the fleet again there. The episode ends when the
/// fleet leaves the sector or layer or receives a new order, and it
/// outlives the year.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EscapeEpisode {
    pub sector: Coords,
    /// 1 inside the world's system, 0 in deep space.
    pub layer: u8,
    /// Bit `empire - 1` is set once that empire has had its pursuit
    /// exchange.
    pub pursuers: u32,
}

impl EscapeEpisode {
    pub fn pursued_by(&self, empire: u8) -> bool {
        empire_bit(empire).is_some_and(|bit| self.pursuers & bit != 0)
    }

    pub fn record_pursuit(&mut self, empire: u8) {
        if let Some(bit) = empire_bit(empire) {
            self.pursuers |= bit;
        }
    }
}

pub(super) fn empire_bit(empire: u8) -> Option<u32> {
    (1..=32).contains(&empire).then(|| 1 << (empire - 1))
}
