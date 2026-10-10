use std::path::PathBuf;

use crate::{BaseDat, ConquestDat, FleetDat, IpbmDat, ParseError, PlanetDat, PlayerDat, SetupDat};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreGameData {
    pub player: PlayerDat,
    pub planets: PlanetDat,
    pub fleets: FleetDat,
    pub bases: BaseDat,
    pub ipbm: IpbmDat,
    pub setup: SetupDat,
    pub conquest: ConquestDat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentKnownComplianceStatus {
    pub fleet_order: bool,
    pub planet_build: bool,
    pub guard_starbase: bool,
    pub ipbm: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentKnownKeyWordSummary {
    pub player_starbase_count: u16,
    pub player_ipbm_count: u16,
    pub fleet1_local_slot: Option<u16>,
    pub fleet1_id: Option<u16>,
    pub fleet1_guard_index: Option<u8>,
    pub fleet1_guard_enable: Option<u8>,
    pub fleet1_target: Option<[u8; 2]>,
    pub base1_summary: Option<u16>,
    pub base1_id: Option<u8>,
    pub base1_chain: Option<u16>,
    pub base1_coords: Option<[u8; 2]>,
    pub ipbm_record_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentKnownGuardStarbaseLinkageSummary {
    pub player_record_index_1_based: usize,
    pub fleet_record_index_1_based: usize,
    pub player_starbase_count: u16,
    pub fleet_order: u8,
    pub fleet_local_slot: u16,
    pub fleet_id: u16,
    pub guard_index: u8,
    pub guard_enable: u8,
    pub target_coords: [u8; 2],
    pub selected_base_present: bool,
    pub selected_base_summary_word: Option<u16>,
    pub selected_base_id: Option<u8>,
    pub selected_base_chain_word: Option<u16>,
    pub selected_base_coords: Option<[u8; 2]>,
    pub selected_base_trailing_coords: Option<[u8; 2]>,
    pub selected_base_owner_empire: Option<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EmpireUnitSummary {
    pub destroyers: u32,
    pub cruisers: u32,
    pub battleships: u32,
    pub scouts: u32,
    pub transports: u32,
    pub etacs: u32,
    pub starbases: u32,
    pub armies: u32,
    pub ground_batteries: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EmpireEconomySummary {
    pub owned_planets: usize,
    pub present_production: u16,
    pub potential_production: u16,
    pub total_available_points: u32,
    pub efficiency_percent: f64,
    pub rank_by_planets: usize,
    pub rank_by_present_production: usize,
    pub tax_rate: u8,
    pub max_fleets_and_bases: usize,
    pub current_fleets_and_bases: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmpireProductionRankingSort {
    Id,
    Production,
    NumberOfPlanets,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmpireProductionRankingRow {
    pub empire_id: u8,
    pub empire_name: String,
    pub planets_owned: usize,
    pub current_production: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmpirePlanetEconomyRow {
    pub planet_record_index_1_based: usize,
    pub coords: [u8; 2],
    pub planet_name: String,
    pub present_production: u16,
    pub potential_production: u16,
    pub stored_production_points: u32,
    pub yearly_tax_revenue: u32,
    pub yearly_growth_delta: u16,
    pub build_capacity: u16,
    pub has_friendly_starbase: bool,
    pub armies: u8,
    pub ground_batteries: u8,
    pub is_homeworld_seed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetOrderValidationError {
    UnknownOrderCode(u8),
    MissingCombatShips,
    MissingScoutShip,
    MissingEtac,
    MissingLoadedTroopTransports,
    MissingPlanetTarget,
    TargetOwnedByFleetEmpire,
    TargetNotOwnedByFleetEmpire,
    TargetAlreadyOwned,
    DuplicateFriendlyColonizeTarget {
        target: [u8; 2],
        conflicting_fleet_record_index_1_based: usize,
    },
    InvalidJoinHost,
    InvalidGuardStarbase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetPlayerInputValidationError {
    InvalidOrder(FleetOrderValidationError),
    LoadedArmiesExceedTransportCapacity { loaded_armies: u16, transports: u16 },
    SpeedExceedsMaximum { speed: u8, max: u8 },
    RulesOfEngagementOutOfRange { roe: u8 },
    NonCombatFleetMustUseZeroRoe { roe: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetPlayerInputValidationError {
    InvalidBuildKind(u8),
    InvalidBuildPointsForKind {
        kind_raw: u8,
        points_remaining_raw: u32,
    },
    MissingBuildKindForCount,
    MissingBuildCountForKind,
    InvalidStardockKind(u8),
    MissingStardockKindForCount,
    MissingStardockCountForKind,
    InvalidTaxRate(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerDiplomacyValidationError {
    TargetOutOfRange { target_empire_raw: u8 },
    SelfTarget { empire_raw: u8 },
    InvalidStoredRelationByte { target_empire_raw: u8, raw: u8 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreFileDiffCount {
    pub name: &'static str,
    pub differing_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreFileDiffOffsets {
    pub name: &'static str,
    pub differing_offsets: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampaignState {
    CivilDisorder,
    Rogue,
    Stable,
    MarginalExistence,
    DefectionRisk,
    Defeated,
}

impl CampaignState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CivilDisorder => "civil_disorder",
            Self::Rogue => "rogue",
            Self::Stable => "stable",
            Self::MarginalExistence => "marginal_existence",
            Self::DefectionRisk => "defection_risk",
            Self::Defeated => "defeated",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampaignOutlook {
    Contested,
    SoleContender(u8),
}

impl CampaignOutlook {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Contested => "contested",
            Self::SoleContender(_) => "sole_contender",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampaignOutcome {
    Ongoing,
    RecognizedEmperor(u8),
}

impl CampaignOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ongoing => "ongoing",
            Self::RecognizedEmperor(_) => "recognized_emperor",
        }
    }
}

#[derive(Debug)]
pub enum GameDirectoryError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: PathBuf,
        source: ParseError,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameStateMutationError {
    InvalidPlayerCount {
        player_count: u8,
    },
    MissingBaseRecord {
        index_1_based: usize,
    },
    MissingFleetRecord {
        index_1_based: usize,
    },
    MissingIpbmRecord {
        index_1_based: usize,
    },
    MissingPlanetRecord {
        index_1_based: usize,
    },
    MissingPlayerRecord {
        index_1_based: usize,
    },
    PlayerAlreadyJoined {
        index_1_based: usize,
    },
    PlayerSlotNotJoinable {
        index_1_based: usize,
    },
    PlanetBuildQueueFull {
        index_1_based: usize,
    },
    EmptyStardockSlot {
        planet_index_1_based: usize,
        slot_0_based: usize,
    },
    InvalidCommissionSelection,
    FleetNotAtPlanet {
        fleet_index_1_based: usize,
        planet_index_1_based: usize,
    },
    PlanetArmyShortage {
        planet_index_1_based: usize,
        requested: u16,
        available: u16,
    },
    FleetArmyShortage {
        fleet_index_1_based: usize,
        requested: u16,
        available: u16,
    },
    PlanetArmyCapacityExceeded {
        planet_index_1_based: usize,
        requested: u16,
        available: u16,
    },
    PlanetGroundBatteryCapacityExceeded {
        planet_index_1_based: usize,
        requested: u16,
        available: u16,
    },
    TransportCapacityExceeded {
        fleet_index_1_based: usize,
        requested: u16,
        available: u16,
    },
    BaseOwnershipMismatch {
        player_index_1_based: usize,
        base_record_index_1_based: usize,
    },
    FleetOwnershipMismatch {
        player_index_1_based: usize,
        fleet_index_1_based: usize,
    },
    PlanetOwnershipMismatch {
        player_index_1_based: usize,
        planet_index_1_based: usize,
    },
    FleetDetachSelectionEmpty {
        fleet_index_1_based: usize,
    },
    FleetDetachSelectionExceedsAvailable {
        fleet_index_1_based: usize,
        ship_kind: &'static str,
        requested: u16,
        available: u16,
    },
    FleetDetachLeavesFleetEmpty {
        fleet_index_1_based: usize,
    },
    InvalidFleetSpeed {
        fleet_index_1_based: usize,
        requested: u8,
        max: u8,
    },
    InvalidFleetLocalSlot {
        fleet_index_1_based: usize,
        requested: u16,
    },
    InvalidFleetMergeSelection {
        fleet_index_1_based: usize,
        host_fleet_index_1_based: usize,
    },
    InvalidFleetOrder {
        fleet_index_1_based: usize,
        reason: FleetOrderValidationError,
    },
    InvalidFleetPlayerInput {
        fleet_index_1_based: usize,
        reason: FleetPlayerInputValidationError,
    },
    InvalidPlanetPlayerInput {
        planet_index_1_based: usize,
        reason: PlanetPlayerInputValidationError,
    },
    InvalidPlayerTaxRate {
        player_index_1_based: usize,
        tax_rate: u8,
    },
    InvalidDiplomacyInput {
        player_index_1_based: usize,
        reason: PlayerDiplomacyValidationError,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommissionResult {
    Fleet { fleet_record_index_1_based: usize },
    Starbase { base_record_index_1_based: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CommissionFleetDraft {
    pub destroyers: u16,
    pub cruisers: u16,
    pub battleships: u16,
    pub scouts: u16,
    pub transports: u16,
    pub etacs: u16,
}

impl CommissionFleetDraft {
    pub fn total_ships(self) -> u32 {
        u32::from(self.destroyers)
            + u32::from(self.cruisers)
            + u32::from(self.battleships)
            + u32::from(self.scouts)
            + u32::from(self.transports)
            + u32::from(self.etacs)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AutoCommissionReport {
    pub ships_commissioned: u32,
    pub starbases_commissioned: usize,
    pub planets_used: usize,
    pub fleets_created: usize,
    pub entries: Vec<AutoCommissionEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutoCommissionEntry {
    Fleet(AutoCommissionFleetEntry),
    Starbase(AutoCommissionStarbaseEntry),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoCommissionFleetEntry {
    pub fleet_number: u16,
    pub planet_name: String,
    pub coords: [u8; 2],
    pub destroyers: u32,
    pub cruisers: u32,
    pub battleships: u32,
    pub scouts: u32,
    pub transports: u32,
    pub etacs: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoCommissionStarbaseEntry {
    pub starbase_number: u8,
    pub planet_name: String,
    pub coords: [u8; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FleetDetachSelection {
    pub battleships: u16,
    pub cruisers: u16,
    pub destroyers: u16,
    pub full_transports: u16,
    pub empty_transports: u16,
    pub scouts: u8,
    pub etacs: u16,
}

impl FleetDetachSelection {
    pub fn total_ships(self) -> u32 {
        u32::from(self.battleships)
            + u32::from(self.cruisers)
            + u32::from(self.destroyers)
            + u32::from(self.full_transports)
            + u32::from(self.empty_transports)
            + u32::from(self.scouts)
            + u32::from(self.etacs)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FleetDetachResult {
    pub donor_fleet_record_index_1_based: usize,
    pub new_fleet_record_index_1_based: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FleetTransferResult {
    pub donor_fleet_record_index_1_based: usize,
    pub host_fleet_record_index_1_based: usize,
}

impl std::fmt::Display for GameDirectoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "{}: {}", path.display(), source),
            Self::Parse { path, source } => write!(f, "{}: {}", path.display(), source),
        }
    }
}

impl std::error::Error for GameDirectoryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
        }
    }
}

impl std::fmt::Display for FleetOrderValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownOrderCode(order_code) => {
                write!(f, "unknown fleet order code {}", order_code)
            }
            Self::MissingCombatShips => write!(f, "mission requires combat ships"),
            Self::MissingScoutShip => write!(f, "mission requires a scout ship"),
            Self::MissingEtac => write!(f, "mission requires an ETAC"),
            Self::MissingLoadedTroopTransports => {
                write!(f, "mission requires loaded troop transports")
            }
            Self::MissingPlanetTarget => write!(f, "mission requires a planet target"),
            Self::TargetOwnedByFleetEmpire => {
                write!(f, "mission target is owned by the fleet empire")
            }
            Self::TargetNotOwnedByFleetEmpire => {
                write!(f, "mission target is not owned by the fleet empire")
            }
            Self::TargetAlreadyOwned => write!(f, "mission target is already owned"),
            Self::DuplicateFriendlyColonizeTarget {
                target,
                conflicting_fleet_record_index_1_based,
            } => write!(
                f,
                "friendly fleet {} already claims colonize target ({},{})",
                conflicting_fleet_record_index_1_based, target[0], target[1]
            ),
            Self::InvalidJoinHost => write!(f, "join mission host fleet is invalid"),
            Self::InvalidGuardStarbase => write!(f, "guard starbase target is invalid"),
        }
    }
}

impl std::fmt::Display for GameStateMutationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPlayerCount { player_count } => {
                write!(f, "player count must be in 1..=25, got {player_count}")
            }
            Self::MissingBaseRecord { index_1_based } => {
                write!(f, "missing base record {}", index_1_based)
            }
            Self::MissingFleetRecord { index_1_based } => {
                write!(f, "missing fleet record {}", index_1_based)
            }
            Self::MissingIpbmRecord { index_1_based } => {
                write!(f, "missing IPBM record {}", index_1_based)
            }
            Self::MissingPlanetRecord { index_1_based } => {
                write!(f, "missing planet record {}", index_1_based)
            }
            Self::MissingPlayerRecord { index_1_based } => {
                write!(f, "missing player record {}", index_1_based)
            }
            Self::PlayerAlreadyJoined { index_1_based } => {
                write!(f, "player {} is already joined", index_1_based)
            }
            Self::PlayerSlotNotJoinable { index_1_based } => {
                write!(f, "player {} is not an open joinable seat", index_1_based)
            }
            Self::PlanetBuildQueueFull { index_1_based } => {
                write!(f, "build queue full for planet record {}", index_1_based)
            }
            Self::EmptyStardockSlot {
                planet_index_1_based,
                slot_0_based,
            } => write!(
                f,
                "empty stardock slot {} on planet record {}",
                slot_0_based + 1,
                planet_index_1_based
            ),
            Self::InvalidCommissionSelection => write!(f, "invalid commission selection"),
            Self::FleetNotAtPlanet {
                fleet_index_1_based,
                planet_index_1_based,
            } => write!(
                f,
                "fleet {} is not at planet {}",
                fleet_index_1_based, planet_index_1_based
            ),
            Self::PlanetArmyShortage {
                planet_index_1_based,
                requested,
                available,
            } => write!(
                f,
                "planet {} has only {} armies available, requested {}",
                planet_index_1_based, available, requested
            ),
            Self::FleetArmyShortage {
                fleet_index_1_based,
                requested,
                available,
            } => write!(
                f,
                "fleet {} has only {} loaded armies available, requested {}",
                fleet_index_1_based, available, requested
            ),
            Self::PlanetArmyCapacityExceeded {
                planet_index_1_based,
                requested,
                available,
            } => write!(
                f,
                "planet {} can receive only {} more armies, requested {}",
                planet_index_1_based, available, requested
            ),
            Self::PlanetGroundBatteryCapacityExceeded {
                planet_index_1_based,
                requested,
                available,
            } => write!(
                f,
                "planet {} can receive only {} more batteries, requested {}",
                planet_index_1_based, available, requested
            ),
            Self::TransportCapacityExceeded {
                fleet_index_1_based,
                requested,
                available,
            } => write!(
                f,
                "fleet {} has only {} troop transport capacity available, requested {}",
                fleet_index_1_based, available, requested
            ),
            Self::BaseOwnershipMismatch {
                player_index_1_based,
                base_record_index_1_based,
            } => write!(
                f,
                "base {} is not owned by player {}",
                base_record_index_1_based, player_index_1_based
            ),
            Self::FleetOwnershipMismatch {
                player_index_1_based,
                fleet_index_1_based,
            } => write!(
                f,
                "fleet {} is not owned by player {}",
                fleet_index_1_based, player_index_1_based
            ),
            Self::PlanetOwnershipMismatch {
                player_index_1_based,
                planet_index_1_based,
            } => write!(
                f,
                "planet {} is not owned by player {}",
                planet_index_1_based, player_index_1_based
            ),
            Self::FleetDetachSelectionEmpty {
                fleet_index_1_based,
            } => write!(f, "fleet {} detach selection is empty", fleet_index_1_based),
            Self::FleetDetachSelectionExceedsAvailable {
                fleet_index_1_based,
                ship_kind,
                requested,
                available,
            } => write!(
                f,
                "fleet {} has only {} {} available, requested {}",
                fleet_index_1_based, available, ship_kind, requested
            ),
            Self::FleetDetachLeavesFleetEmpty {
                fleet_index_1_based,
            } => write!(
                f,
                "fleet {} must retain at least one ship after detach",
                fleet_index_1_based
            ),
            Self::InvalidFleetSpeed {
                fleet_index_1_based,
                requested,
                max,
            } => write!(
                f,
                "fleet {} speed {} exceeds maximum {}",
                fleet_index_1_based, requested, max
            ),
            Self::InvalidFleetLocalSlot {
                fleet_index_1_based,
                requested,
            } => write!(
                f,
                "fleet {} cannot use local slot {}",
                fleet_index_1_based, requested
            ),
            Self::InvalidFleetMergeSelection {
                fleet_index_1_based,
                host_fleet_index_1_based,
            } => write!(
                f,
                "fleet {} cannot merge into fleet {}",
                fleet_index_1_based, host_fleet_index_1_based
            ),
            Self::InvalidFleetOrder {
                fleet_index_1_based,
                reason,
            } => write!(
                f,
                "fleet {} has invalid order: {}",
                fleet_index_1_based, reason
            ),
            Self::InvalidFleetPlayerInput {
                fleet_index_1_based,
                reason,
            } => write!(
                f,
                "fleet {} has invalid player input: {:?}",
                fleet_index_1_based, reason
            ),
            Self::InvalidPlanetPlayerInput {
                planet_index_1_based,
                reason,
            } => write!(
                f,
                "planet {} has invalid player input: {:?}",
                planet_index_1_based, reason
            ),
            Self::InvalidPlayerTaxRate {
                player_index_1_based,
                tax_rate,
            } => write!(
                f,
                "player {} has invalid tax rate {}",
                player_index_1_based, tax_rate
            ),
            Self::InvalidDiplomacyInput {
                player_index_1_based,
                reason,
            } => write!(
                f,
                "player {} has invalid diplomacy input: {:?}",
                player_index_1_based, reason
            ),
        }
    }
}

impl std::error::Error for GameStateMutationError {}
