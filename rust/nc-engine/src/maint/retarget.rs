use super::{Mission, MissionRetargetEvent};
use crate::navigation::sector_distance_sq;
use crate::{CoreGameData, Order};
use nc_data::{Coords, troop_transports};

pub(super) fn refresh_seek_home_targets(game_data: &mut CoreGameData) -> Vec<MissionRetargetEvent> {
    let owned_planets: Vec<(u8, [u8; 2])> = game_data
        .planets
        .iter()
        .map(|planet| (planet.owner.0, planet.sector.to_array()))
        .collect();
    let mut events = Vec::new();
    for (fleet_idx, fleet) in game_data.fleets.iter_mut().enumerate() {
        if fleet.order != Order::SeekHome {
            continue;
        }
        let previous_target_coords = fleet.target.to_array();
        let current_coords = fleet.sector.to_array();
        let owner_empire_raw = fleet.owner.0;
        let reporting_fleet_number = Some(fleet.fleet_number.0 as u8);
        let Some(new_target_coords) =
            nearest_owned_planet_target_from_list(&owned_planets, owner_empire_raw, current_coords)
        else {
            fleet.order = Order::HoldPosition;
            fleet.speed = 0;
            fleet.target = Coords::from_array(current_coords);
            events.push(MissionRetargetEvent::Abandoned {
                fleet_idx,
                reporting_fleet_number,
                owner_empire_raw,
                mission: Mission::SeekHome,
                previous_target_coords,
                coords: current_coords,
            });
            continue;
        };
        if new_target_coords != previous_target_coords {
            fleet.target = Coords::from_array(new_target_coords);
            events.push(MissionRetargetEvent::Retargeted {
                fleet_idx,
                reporting_fleet_number,
                owner_empire_raw,
                mission: Mission::SeekHome,
                current_coords,
                previous_target_coords,
                new_target_coords,
            });
        }
    }
    events
}

/// Join hosts are named by (owner, fleet number), the host's 0x00 word.
pub(super) fn refresh_join_host_targets(game_data: &mut CoreGameData) -> Vec<MissionRetargetEvent> {
    let current_host_viability: std::collections::HashMap<(u8, u16), bool> = game_data
        .fleets
        .iter()
        .map(|fleet| {
            let viable = fleet.ships.destroyers > 0
                || fleet.ships.cruisers > 0
                || fleet.ships.battleships > 0
                || fleet.ships.scouts > 0
                || troop_transports(fleet) > 0
                || fleet.ships.etacs > 0;
            ((fleet.owner.0, fleet.fleet_number.0), viable)
        })
        .collect();
    let current_fleet_coords: std::collections::HashMap<(u8, u16), [u8; 2]> = game_data
        .fleets
        .iter()
        .map(|fleet| {
            (
                (fleet.owner.0, fleet.fleet_number.0),
                fleet.sector.to_array(),
            )
        })
        .collect();

    let mut events = Vec::new();
    for (fleet_idx, fleet) in game_data.fleets.iter_mut().enumerate() {
        if fleet.order != Order::JoinAnotherFleet {
            continue;
        }

        let host_number = fleet.mission_target;
        if host_number == 0 || host_number == fleet.fleet_number.0 {
            continue;
        }
        let host_key = (fleet.owner.0, host_number);

        if !current_host_viability
            .get(&host_key)
            .copied()
            .unwrap_or(false)
        {
            continue;
        }

        if let Some(coords) = current_fleet_coords.get(&host_key).copied() {
            let previous_target_coords = fleet.target.to_array();
            if coords != previous_target_coords {
                let current_coords = fleet.sector.to_array();
                let reporting_fleet_number = Some(fleet.fleet_number.0 as u8);
                fleet.target = Coords::from_array(coords);
                events.push(MissionRetargetEvent::Retargeted {
                    fleet_idx,
                    reporting_fleet_number,
                    owner_empire_raw: fleet.owner.0,
                    mission: Mission::JoinAnotherFleet,
                    current_coords,
                    previous_target_coords,
                    new_target_coords: coords,
                });
            }
        }
    }
    events
}

pub(super) fn refresh_guard_starbase_targets(
    game_data: &mut CoreGameData,
) -> Vec<MissionRetargetEvent> {
    let active_bases_by_id: std::collections::HashMap<(u8, u16), [u8; 2]> = game_data
        .bases
        .iter()
        .filter(|base| base.base_id.0 != 0 && base.owner.0 != 0)
        .map(|base| ((base.owner.0, base.base_id.0), base.sector.to_array()))
        .collect();
    let active_base_coords: std::collections::HashSet<(u8, [u8; 2])> = game_data
        .bases
        .iter()
        .filter(|base| base.base_id.0 != 0 && base.owner.0 != 0)
        .map(|base| (base.owner.0, base.sector.to_array()))
        .collect();
    let mut events = Vec::new();
    for (fleet_idx, fleet) in game_data.fleets.iter_mut().enumerate() {
        if fleet.order != Order::GuardStarbase {
            continue;
        }
        let previous_target_coords = fleet.target.to_array();
        let current_coords = fleet.sector.to_array();
        let owner_empire_raw = fleet.owner.0;
        let reporting_fleet_number = Some(fleet.fleet_number.0 as u8);
        let base_id = fleet.mission_target;
        let new_target_coords = active_bases_by_id
            .get(&(owner_empire_raw, base_id))
            .copied()
            .or_else(|| {
                active_base_coords
                    .contains(&(owner_empire_raw, previous_target_coords))
                    .then_some(previous_target_coords)
            });
        let Some(new_target_coords) = new_target_coords else {
            fleet.order = Order::HoldPosition;
            fleet.speed = 0;
            fleet.target = Coords::from_array(current_coords);
            events.push(MissionRetargetEvent::Abandoned {
                fleet_idx,
                reporting_fleet_number,
                owner_empire_raw,
                mission: Mission::GuardStarbase,
                previous_target_coords,
                coords: current_coords,
            });
            continue;
        };
        if new_target_coords != previous_target_coords {
            fleet.target = Coords::from_array(new_target_coords);
            events.push(MissionRetargetEvent::Retargeted {
                fleet_idx,
                reporting_fleet_number,
                owner_empire_raw,
                mission: Mission::GuardStarbase,
                current_coords,
                previous_target_coords,
                new_target_coords,
            });
        }
    }
    events
}

pub(super) fn nearest_owned_planet_target_from_list(
    owned_planets: &[(u8, [u8; 2])],
    empire_raw: u8,
    from: [u8; 2],
) -> Option<[u8; 2]> {
    owned_planets
        .iter()
        .filter(|(owner, _)| *owner == empire_raw)
        .min_by_key(|(_, coords)| sector_distance_sq(from, *coords))
        .map(|(_, coords)| *coords)
}
