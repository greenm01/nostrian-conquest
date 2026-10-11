use super::{Mission, MissionRetargetEvent};
use crate::{CoreGameData, Order};
use nc_data::{Coords, troop_transports};

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
                    owner_empire: fleet.owner.0,
                    mission: Mission::JoinAnotherFleet,
                    current_coords,
                    in_system: fleet.depth.round() == 1,
                    previous_target_coords,
                    new_target_coords: coords,
                    stardate_week: None,
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
        let owner_empire = fleet.owner.0;
        let reporting_fleet_number = Some(fleet.fleet_number.0 as u8);
        let base_id = fleet.mission_target;
        let new_target_coords = active_bases_by_id
            .get(&(owner_empire, base_id))
            .copied()
            .or_else(|| {
                active_base_coords
                    .contains(&(owner_empire, previous_target_coords))
                    .then_some(previous_target_coords)
            });
        let Some(new_target_coords) = new_target_coords else {
            fleet.order = Order::HoldPosition;
            fleet.speed = 0;
            fleet.target = Coords::from_array(current_coords);
            events.push(MissionRetargetEvent::Abandoned {
                fleet_idx,
                reporting_fleet_number,
                owner_empire,
                mission: Mission::GuardStarbase,
                previous_target_coords,
                coords: current_coords,
                in_system: fleet.depth.round() == 1,
                stardate_week: None,
            });
            continue;
        };
        if new_target_coords != previous_target_coords {
            fleet.target = Coords::from_array(new_target_coords);
            events.push(MissionRetargetEvent::Retargeted {
                fleet_idx,
                reporting_fleet_number,
                owner_empire,
                mission: Mission::GuardStarbase,
                current_coords,
                in_system: fleet.depth.round() == 1,
                previous_target_coords,
                new_target_coords,
                stardate_week: None,
            });
        }
    }
    events
}
