use std::cmp::Reverse;

use nc_data::{CoreGameData, EmpirePlanetEconomyRow, Fleet, troop_transports};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmyTransportMode {
    Load,
    Unload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanetTransportFleetCandidate {
    pub fleet_record_index_1_based: usize,
    pub fleet_number: u16,
    pub troop_transports: u16,
    pub loaded_armies: u16,
    pub available_qty: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanetTransportPlanetCandidate {
    pub planet_record_index_1_based: usize,
    pub planet_name: String,
    pub coords: [u8; 2],
    pub planet_armies: u8,
    pub transport_capacity: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetTransportSelectionError {
    NotOwnedFleet,
    NotAtOwnedWorld,
    NoTroopTransports,
    TroopTransportsFull,
    NoPlanetArmies,
    TroopTransportsEmpty,
    NoPlanetCapacity,
    NothingAvailableToLoad,
    NothingAvailableToUnload,
}

impl PlanetTransportSelectionError {
    pub fn message(self) -> &'static str {
        match self {
            Self::NotOwnedFleet => "Enter one of your fleet numbers.",
            Self::NotAtOwnedWorld => "That fleet is not at one of your worlds.",
            Self::NoTroopTransports => "That fleet has no troop transports.",
            Self::TroopTransportsFull => "That fleet's troop transports are already full.",
            Self::NoPlanetArmies => "That world has no armies available to load.",
            Self::TroopTransportsEmpty => "That fleet's troop transports are already empty.",
            Self::NoPlanetCapacity => "That world has no room to receive unloaded armies.",
            Self::NothingAvailableToLoad => {
                "That fleet cannot load any armies from that world right now."
            }
            Self::NothingAvailableToUnload => {
                "That fleet cannot unload any armies to that world right now."
            }
        }
    }
}

pub fn transport_available_qty(
    mode: ArmyTransportMode,
    fleet: &Fleet,
    planet: &EmpirePlanetEconomyRow,
) -> u16 {
    match mode {
        ArmyTransportMode::Load => troop_transports(fleet)
            .saturating_sub(fleet.ships.loaded_transports)
            .min(u16::from(planet.armies)),
        ArmyTransportMode::Unload => fleet
            .ships
            .loaded_transports
            .min(u16::from(u8::MAX.saturating_sub(planet.armies))),
    }
}

pub fn default_fleet_transport_fleet_number(
    game_data: &CoreGameData,
    owner_empire_id: u8,
    mode: ArmyTransportMode,
    owned_planet_rows: &[EmpirePlanetEconomyRow],
) -> Option<u16> {
    game_data
        .fleets
        .iter()
        .filter(|fleet| fleet.owner.0 == owner_empire_id)
        .filter_map(|fleet| {
            let planet = owned_planet_row_for_fleet(owned_planet_rows, fleet)?;
            let ranking_qty = match mode {
                ArmyTransportMode::Load => {
                    troop_transports(fleet).saturating_sub(fleet.ships.loaded_transports)
                }
                ArmyTransportMode::Unload => fleet.ships.loaded_transports,
            };
            if ranking_qty == 0 || transport_available_qty(mode, fleet, planet) == 0 {
                return None;
            }
            Some((ranking_qty, Reverse(fleet.fleet_number.0)))
        })
        .max()
        .map(|(_, fleet_number)| fleet_number.0)
}

pub fn transport_fleet_candidates_for_planet(
    game_data: &CoreGameData,
    owner_empire_id: u8,
    mode: ArmyTransportMode,
    planet: &EmpirePlanetEconomyRow,
) -> Vec<PlanetTransportFleetCandidate> {
    game_data
        .fleets
        .iter()
        .enumerate()
        .filter(|(_, fleet)| {
            fleet.owner.0 == owner_empire_id
                && fleet.sector.to_array() == planet.coords
                && troop_transports(fleet) > 0
        })
        .map(|(idx, fleet)| PlanetTransportFleetCandidate {
            fleet_record_index_1_based: idx + 1,
            fleet_number: fleet.fleet_number.0,
            troop_transports: troop_transports(fleet),
            loaded_armies: fleet.ships.loaded_transports,
            available_qty: transport_available_qty(mode, fleet, planet),
        })
        .collect()
}

pub fn transport_planet_candidates(
    game_data: &CoreGameData,
    owner_empire_id: u8,
    mode: ArmyTransportMode,
    owned_planet_rows: &[EmpirePlanetEconomyRow],
) -> Vec<PlanetTransportPlanetCandidate> {
    owned_planet_rows
        .iter()
        .filter_map(|row| {
            if mode == ArmyTransportMode::Load && row.armies == 0 {
                return None;
            }
            let fleets =
                transport_fleet_candidates_for_planet(game_data, owner_empire_id, mode, row)
                    .into_iter()
                    .filter(|fleet| fleet.available_qty > 0)
                    .collect::<Vec<_>>();
            if fleets.is_empty() {
                return None;
            }
            Some(PlanetTransportPlanetCandidate {
                planet_record_index_1_based: row.planet_record_index_1_based,
                planet_name: row.planet_name.clone(),
                coords: row.coords,
                planet_armies: row.armies,
                transport_capacity: fleets.iter().map(|fleet| fleet.available_qty).sum(),
            })
        })
        .collect()
}

pub fn resolve_planet_transport_fleet_selection(
    game_data: &CoreGameData,
    owner_empire_id: u8,
    mode: ArmyTransportMode,
    fleet_number: u16,
    owned_planet_rows: &[EmpirePlanetEconomyRow],
) -> Result<(PlanetTransportFleetCandidate, EmpirePlanetEconomyRow), PlanetTransportSelectionError>
{
    let (fleet_record_index_1_based, fleet) = game_data
        .fleets
        .iter()
        .enumerate()
        .find(|(_, fleet)| fleet.owner.0 == owner_empire_id && fleet.fleet_number.0 == fleet_number)
        .map(|(idx, fleet)| (idx + 1, fleet))
        .ok_or(PlanetTransportSelectionError::NotOwnedFleet)?;

    let planet = owned_planet_row_for_fleet(owned_planet_rows, fleet)
        .cloned()
        .ok_or(PlanetTransportSelectionError::NotAtOwnedWorld)?;

    if troop_transports(fleet) == 0 {
        return Err(PlanetTransportSelectionError::NoTroopTransports);
    }

    match mode {
        ArmyTransportMode::Load => {
            if troop_transports(fleet).saturating_sub(fleet.ships.loaded_transports) == 0 {
                return Err(PlanetTransportSelectionError::TroopTransportsFull);
            }
            if planet.armies == 0 {
                return Err(PlanetTransportSelectionError::NoPlanetArmies);
            }
        }
        ArmyTransportMode::Unload => {
            if fleet.ships.loaded_transports == 0 {
                return Err(PlanetTransportSelectionError::TroopTransportsEmpty);
            }
            if planet.armies == u8::MAX {
                return Err(PlanetTransportSelectionError::NoPlanetCapacity);
            }
        }
    }

    let available_qty = transport_available_qty(mode, fleet, &planet);
    if available_qty == 0 {
        return Err(match mode {
            ArmyTransportMode::Load => PlanetTransportSelectionError::NothingAvailableToLoad,
            ArmyTransportMode::Unload => PlanetTransportSelectionError::NothingAvailableToUnload,
        });
    }

    Ok((
        PlanetTransportFleetCandidate {
            fleet_record_index_1_based,
            fleet_number: fleet.fleet_number.0,
            troop_transports: troop_transports(fleet),
            loaded_armies: fleet.ships.loaded_transports,
            available_qty,
        },
        planet,
    ))
}

fn owned_planet_row_for_fleet<'a>(
    owned_planet_rows: &'a [EmpirePlanetEconomyRow],
    fleet: &Fleet,
) -> Option<&'a EmpirePlanetEconomyRow> {
    owned_planet_rows
        .iter()
        .find(|row| row.coords == fleet.sector.to_array())
}
