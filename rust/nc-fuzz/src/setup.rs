//! Drawing a small, contact-prone game and expressing it as a harness
//! scenario, so the fuzz game and its KDL replay start from the same state.

use nc_data::{DiplomaticRelation, map_size_for_player_count};
use nc_harness::{
    DiplomacySpec, FleetShipsSpec, FleetSpec, HouseSpec, PlanetSpec, ScenarioBaseline,
    ScenarioMetadata, ScenarioSpec,
};

use crate::choice::{ChoiceSource, pick_from};
use crate::policy::Policy;

pub const START_YEAR: u16 = 3000;
const MAX_PLAYERS: u32 = 4;
const MAP_SEEDS: u32 = 9_999;
/// Every empire starts with this many fleet records, in empire order.
const FLEETS_PER_EMPIRE: usize = 4;
/// Ship counts an armada draws from, edge values included.
const ARMADA_COUNTS: [u16; 5] = [0, 1, 3, 12, 200];

/// Run-size knobs, read from the environment so long runs need no rebuild.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FuzzConfig {
    /// Most maintenance turns one case plays (`NC_FUZZ_TURNS`).
    pub max_turns: u16,
}

impl FuzzConfig {
    pub const DEFAULT_MAX_TURNS: u16 = 8;

    pub fn from_env() -> Self {
        Self {
            max_turns: env_number("NC_FUZZ_TURNS").unwrap_or(Self::DEFAULT_MAX_TURNS),
        }
    }
}

/// Number of fuzz cases a driver runs (`NC_FUZZ_CASES`), or its default.
pub fn case_count_from_env(default: u32) -> u32 {
    env_number("NC_FUZZ_CASES").unwrap_or(default)
}

fn env_number<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::var(name).ok()?.trim().parse().ok()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseSetup {
    pub player_count: u8,
    pub map_seed: u64,
    /// One policy per empire, empire 1 first.
    pub policies: Vec<Policy>,
    /// Empire whose homeworld starts unowned, to reach collapse, recovery,
    /// and campaign-outcome paths within a short game.
    pub collapsed_empire: Option<u8>,
    /// Every fleet starts at the map center, forcing contact on turn 1.
    pub rally: bool,
    /// Every empire starts at war with every other.
    pub war: bool,
    /// Per empire, an edge-valued composition for its first fleet.
    pub armadas: Vec<Option<FleetShipsSpec>>,
    pub turns: u16,
}

impl CaseSetup {
    pub fn draw(choices: &mut dyn ChoiceSource, config: &FuzzConfig) -> Self {
        let player_count = choices.pick(2, MAX_PLAYERS) as u8;
        let map_seed = u64::from(choices.pick(0, MAP_SEEDS));
        let policies = (0..player_count)
            .map(|_| pick_from(choices, &Policy::ALL).expect("policies"))
            .collect();
        let collapsed_empire = choices
            .flip()
            .then(|| choices.pick(1, u32::from(player_count)) as u8);
        let rally = choices.flip();
        let war = choices.flip();
        let armadas = (0..player_count)
            .map(|_| choices.flip().then(|| draw_armada(choices)))
            .collect();
        let turns = choices.pick(1, u32::from(config.max_turns.max(1))) as u16;
        Self {
            player_count,
            map_seed,
            policies,
            collapsed_empire,
            rally,
            war,
            armadas,
            turns,
        }
    }

    pub fn policy(&self, empire_raw: u8) -> Policy {
        self.policies[usize::from(empire_raw) - 1]
    }

    pub fn summary(&self) -> String {
        format!(
            "setup: {} empires, map seed {}, policies [{}], collapsed {:?}, rally {}, war {}, armadas {}, {} turns",
            self.player_count,
            self.map_seed,
            self.policies
                .iter()
                .map(|policy| policy.name())
                .collect::<Vec<_>>()
                .join(", "),
            self.collapsed_empire,
            self.rally,
            self.war,
            self.armadas.iter().flatten().count(),
            self.turns
        )
    }

    pub fn scenario_spec(&self) -> ScenarioSpec {
        let empires = 1..=self.player_count;
        let center = map_size_for_player_count(self.player_count).div_ceil(2);
        let fleets = (0..usize::from(self.player_count) * FLEETS_PER_EMPIRE)
            .filter_map(|idx| {
                let armada = (idx % FLEETS_PER_EMPIRE == 0)
                    .then(|| self.armadas[idx / FLEETS_PER_EMPIRE].clone())
                    .flatten();
                (self.rally || armada.is_some()).then(|| FleetSpec {
                    record_index_1_based: idx + 1,
                    coords: self.rally.then_some([center, center]),
                    ships: armada,
                    ..FleetSpec::default()
                })
            })
            .collect();
        ScenarioSpec {
            metadata: ScenarioMetadata {
                label: Some("nc-fuzz".to_string()),
                player_count: self.player_count,
                year: START_YEAR,
                seed: self.map_seed,
                baseline: ScenarioBaseline::BuilderCompatible,
            },
            houses: empires
                .clone()
                .map(|empire_raw| HouseSpec {
                    record_index_1_based: usize::from(empire_raw),
                    handle: None,
                    empire_name: None,
                    homeworld_name: None,
                    tax_rate: None,
                    autopilot: self.policy(empire_raw) == Policy::Autopilot,
                })
                .collect(),
            diplomacy: if self.war {
                empires
                    .clone()
                    .flat_map(|from| {
                        empires
                            .clone()
                            .filter(move |&to| to != from)
                            .map(move |to| DiplomacySpec {
                                from_empire_raw: from,
                                to_empire_raw: to,
                                relation: DiplomaticRelation::Enemy,
                            })
                    })
                    .collect()
            } else {
                Vec::new()
            },
            planets: self
                .collapsed_empire
                .map(|empire_raw| PlanetSpec {
                    record_index_1_based: usize::from(empire_raw),
                    owner_empire_raw: Some(0),
                    ..PlanetSpec::default()
                })
                .into_iter()
                .collect(),
            fleets,
            turn_files: Vec::new(),
            queued_mail: Vec::new(),
            results_blocks: Vec::new(),
            message_blocks: Vec::new(),
        }
    }
}

fn draw_armada(choices: &mut dyn ChoiceSource) -> FleetShipsSpec {
    let mut count = || pick_from(choices, &ARMADA_COUNTS).expect("armada counts");
    let mut ships = FleetShipsSpec {
        destroyers: count(),
        cruisers: count(),
        battleships: count(),
        scouts: count() as u8,
        transports: count(),
        loaded_armies: 0,
        etacs: count(),
    };
    if ships.destroyers
        + ships.cruisers
        + ships.battleships
        + u16::from(ships.scouts)
        + ships.transports
        + ships.etacs
        == 0
    {
        ships.destroyers = 1;
    }
    ships.loaded_armies = count().min(ships.transports);
    ships
}
