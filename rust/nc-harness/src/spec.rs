use std::path::PathBuf;

use nc_data::{DiplomaticRelation, Order};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioBaseline {
    BuilderCompatible,
    JoinableNewGame,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioMetadata {
    pub label: Option<String>,
    pub player_count: u8,
    pub year: u16,
    pub seed: u64,
    pub baseline: ScenarioBaseline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseSpec {
    pub record_index_1_based: usize,
    pub handle: Option<String>,
    pub empire_name: Option<String>,
    pub homeworld_name: Option<String>,
    pub tax_rate: Option<u8>,
    /// Hand the empire to the autopilot AI, as the in-game toggle does.
    pub autopilot: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiplomacySpec {
    pub from_empire_raw: u8,
    pub to_empire_raw: u8,
    pub relation: DiplomaticRelation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StardockSlotSpec {
    pub slot_0_based: usize,
    pub kind_raw: u8,
    pub count: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommissionSpec {
    pub slot_0_based: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlanetSpec {
    pub record_index_1_based: usize,
    pub coords: Option<[u8; 2]>,
    pub owner_empire_raw: Option<u8>,
    pub name: Option<String>,
    pub potential_production: Option<u16>,
    pub present_production: Option<u16>,
    pub stored_production: Option<u32>,
    pub economy_marker: Option<u8>,
    pub armies: Option<u8>,
    pub ground_batteries: Option<u8>,
    pub stardock: Vec<StardockSlotSpec>,
    pub commissions: Vec<CommissionSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FleetShipsSpec {
    pub battleships: u16,
    pub cruisers: u16,
    pub destroyers: u16,
    pub scouts: u16,
    pub transports: u16,
    pub loaded_armies: u16,
    pub etacs: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetOrderSpec {
    pub kind: Order,
    pub speed: u8,
    pub target: [u8; 2],
    pub aux0: Option<u8>,
    pub aux1: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FleetSpec {
    pub record_index_1_based: usize,
    pub owner_empire_raw: Option<u8>,
    pub coords: Option<[u8; 2]>,
    pub ships: Option<FleetShipsSpec>,
    pub rules_of_engagement: Option<u8>,
    pub current_speed: Option<u8>,
    pub invasion_armies: Option<u8>,
    pub order: Option<FleetOrderSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnFileSpec {
    pub path: PathBuf,
    /// 1-based maintenance turn the file is submitted before; turn 1 files
    /// apply while the scenario is built.
    pub turn: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedMailSpec {
    pub sender_empire_raw: u8,
    pub recipient_empire_raw: u8,
    pub year: Option<u16>,
    pub subject: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewBlockSpec {
    pub player_record_index_1_based: Option<usize>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioSpec {
    pub metadata: ScenarioMetadata,
    pub houses: Vec<HouseSpec>,
    pub diplomacy: Vec<DiplomacySpec>,
    pub planets: Vec<PlanetSpec>,
    pub fleets: Vec<FleetSpec>,
    pub turn_files: Vec<TurnFileSpec>,
    pub queued_mail: Vec<QueuedMailSpec>,
    pub results_blocks: Vec<ReviewBlockSpec>,
    pub message_blocks: Vec<ReviewBlockSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatScenarioSpec {
    pub scenario: ScenarioSpec,
    pub maintenance_turns: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShipDimensionKind {
    Battleships,
    Cruisers,
    Destroyers,
    Scouts,
    Transports,
    LoadedArmies,
    Etacs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetStatField {
    Armies,
    GroundBatteries,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SweepDimension {
    FleetShips {
        fleet_record_index_1_based: usize,
        kind: ShipDimensionKind,
        values: Vec<u16>,
    },
    FleetRoe {
        fleet_record_index_1_based: usize,
        values: Vec<u8>,
    },
    PlanetStat {
        planet_record_index_1_based: usize,
        field: PlanetStatField,
        values: Vec<u16>,
    },
    DiplomaticRelation {
        from_empire_raw: u8,
        to_empire_raw: u8,
        values: Vec<DiplomaticRelation>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatSweepSpec {
    pub scenario_path: PathBuf,
    pub maintenance_turns: Option<u16>,
    pub seed: u64,
    pub max_cases: usize,
    pub dimensions: Vec<SweepDimension>,
}
