use super::{Mission, MissionRetargetEvent};
use crate::navigation::sector_distance_sq;
use crate::{CoreGameData, Order};

pub(super) fn refresh_seek_home_targets(game_data: &mut CoreGameData) -> Vec<MissionRetargetEvent> {
    let owned_planets: Vec<(u8, [u8; 2])> = game_data
        .planets
        .records
        .iter()
        .map(|planet| (planet.owner_empire_slot_raw(), planet.coords_raw()))
        .collect();
    let mut events = Vec::new();
    for (fleet_idx, fleet) in game_data.fleets.records.iter_mut().enumerate() {
        if fleet.standing_order_kind() != Order::SeekHome {
            continue;
        }
        let previous_target_coords = fleet.standing_order_target_coords_raw();
        let current_coords = fleet.current_location_coords_raw();
        let owner_empire_raw = fleet.owner_empire_raw();
        let reporting_fleet_number = Some(fleet.local_slot_word_raw() as u8);
        let Some(new_target_coords) =
            nearest_owned_planet_target_from_list(&owned_planets, owner_empire_raw, current_coords)
        else {
            fleet.set_standing_order_kind(Order::HoldPosition);
            fleet.set_current_speed(0);
            fleet.set_standing_order_target_coords_raw(current_coords);
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
            fleet.set_standing_order_target_coords_raw(new_target_coords);
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

pub(super) fn refresh_join_host_targets(game_data: &mut CoreGameData) -> Vec<MissionRetargetEvent> {
    let current_host_viability: std::collections::HashMap<u16, bool> = game_data
        .fleets
        .records
        .iter()
        .map(|fleet| {
            let viable = fleet.destroyer_count() > 0
                || fleet.cruiser_count() > 0
                || fleet.battleship_count() > 0
                || fleet.scout_count() > 0
                || fleet.troop_transport_count() > 0
                || fleet.etac_count() > 0;
            (fleet.fleet_id_word_raw(), viable)
        })
        .collect();
    let current_fleet_coords: std::collections::HashMap<u16, [u8; 2]> = game_data
        .fleets
        .records
        .iter()
        .map(|fleet| {
            (
                fleet.fleet_id_word_raw(),
                fleet.current_location_coords_raw(),
            )
        })
        .collect();

    let mut events = Vec::new();
    for (fleet_idx, fleet) in game_data.fleets.records.iter_mut().enumerate() {
        if fleet.standing_order_kind() != Order::JoinAnotherFleet {
            continue;
        }

        let host_id = fleet.join_host_fleet_id_raw();
        if host_id == 0 || host_id == fleet.fleet_id_word_raw() {
            continue;
        }

        if !current_host_viability
            .get(&host_id)
            .copied()
            .unwrap_or(false)
        {
            continue;
        }

        if let Some(coords) = current_fleet_coords.get(&host_id).copied() {
            let previous_target_coords = fleet.standing_order_target_coords_raw();
            if coords != previous_target_coords {
                let current_coords = fleet.current_location_coords_raw();
                let reporting_fleet_number = Some(fleet.local_slot_word_raw() as u8);
                fleet.set_standing_order_target_coords_raw(coords);
                events.push(MissionRetargetEvent::Retargeted {
                    fleet_idx,
                    reporting_fleet_number,
                    owner_empire_raw: fleet.owner_empire_raw(),
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
        .records
        .iter()
        .filter(|base| base.base_id_raw() != 0 && base.owner_empire_raw() != 0)
        .map(|base| {
            (
                (base.owner_empire_raw(), base.base_id_raw()),
                base.coords_raw(),
            )
        })
        .collect();
    let active_base_coords: std::collections::HashSet<(u8, [u8; 2])> = game_data
        .bases
        .records
        .iter()
        .filter(|base| base.base_id_raw() != 0 && base.owner_empire_raw() != 0)
        .map(|base| (base.owner_empire_raw(), base.coords_raw()))
        .collect();
    let mut events = Vec::new();
    for (fleet_idx, fleet) in game_data.fleets.records.iter_mut().enumerate() {
        if fleet.standing_order_kind() != Order::GuardStarbase {
            continue;
        }
        let previous_target_coords = fleet.standing_order_target_coords_raw();
        let current_coords = fleet.current_location_coords_raw();
        let owner_empire_raw = fleet.owner_empire_raw();
        let reporting_fleet_number = Some(fleet.local_slot_word_raw() as u8);
        let base_id = fleet.guard_starbase_index_raw();
        let new_target_coords = active_bases_by_id
            .get(&(owner_empire_raw, base_id))
            .copied()
            .or_else(|| {
                active_base_coords
                    .contains(&(owner_empire_raw, previous_target_coords))
                    .then_some(previous_target_coords)
            });
        let Some(new_target_coords) = new_target_coords else {
            fleet.set_standing_order_kind(Order::HoldPosition);
            fleet.set_current_speed(0);
            fleet.set_standing_order_target_coords_raw(current_coords);
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
            fleet.set_standing_order_target_coords_raw(new_target_coords);
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
