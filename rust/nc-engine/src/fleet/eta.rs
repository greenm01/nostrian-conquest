use nc_data::{CoreGameData, Order, map_size_for_player_count};

use crate::{FleetEtaEstimate, estimate_fleet_eta_to_destination};

pub fn fleet_target_eta_estimate(
    game_data: &CoreGameData,
    fleet_record_index_1_based: usize,
    mission_code: u8,
    destination: [u8; 2],
) -> FleetEtaEstimate {
    estimate_fleet_eta_to_destination(
        game_data,
        fleet_record_index_1_based.saturating_sub(1),
        destination,
        Order::from_raw(mission_code).targets_world(),
        true,
    )
}

pub fn fleet_target_eta_message(
    game_data: &CoreGameData,
    fleet_record_index_1_based: usize,
    fleet_number: u16,
    mission_code: u8,
    destination: [u8; 2],
) -> String {
    match fleet_target_eta_estimate(
        game_data,
        fleet_record_index_1_based,
        mission_code,
        destination,
    ) {
        FleetEtaEstimate::Arrived => format!(
            "Fleet {fleet_number} reaches [{},{}] in 0 year(s), arriving in {}.",
            destination[0],
            destination[1],
            game_data.conquest.game_year(),
        ),
        FleetEtaEstimate::Years(years) => format!(
            "Fleet {fleet_number} reaches [{},{}] in {years} year(s), arriving in {}.",
            destination[0],
            destination[1],
            game_data.conquest.game_year() + years,
        ),
        FleetEtaEstimate::Stopped => format!(
            "Fleet {fleet_number} is stopped and cannot reach [{},{}].",
            destination[0], destination[1]
        ),
        FleetEtaEstimate::Unreachable => {
            format!("No route found to [{},{}].", destination[0], destination[1])
        }
    }
}

pub fn fleet_target_eta_confirmation_message(
    game_data: &CoreGameData,
    fleet_record_index_1_based: usize,
    fleet_number: u16,
    mission_code: u8,
    destination: [u8; 2],
) -> String {
    match fleet_target_eta_estimate(
        game_data,
        fleet_record_index_1_based,
        mission_code,
        destination,
    ) {
        FleetEtaEstimate::Arrived => format!(
            "Fleet {fleet_number} reaches ({:02},{:02}) in 0 year(s), arriving in {}.",
            destination[0],
            destination[1],
            game_data.conquest.game_year(),
        ),
        FleetEtaEstimate::Years(years) => format!(
            "Fleet {fleet_number} reaches ({:02},{:02}) in {years} year(s), arriving in {}.",
            destination[0],
            destination[1],
            game_data.conquest.game_year() + years,
        ),
        FleetEtaEstimate::Stopped => format!(
            "Fleet {fleet_number} is stopped and cannot reach ({:02},{:02}).",
            destination[0], destination[1]
        ),
        FleetEtaEstimate::Unreachable => format!(
            "No route found for Fleet {fleet_number} to ({:02},{:02}).",
            destination[0], destination[1]
        ),
    }
}

pub fn fleet_eta_label(game_data: &CoreGameData, fleet_idx: usize) -> String {
    match fleet_display_eta_estimate(game_data, fleet_idx) {
        FleetEtaEstimate::Arrived => game_data.conquest.game_year().to_string(),
        FleetEtaEstimate::Stopped => "STOP".to_string(),
        FleetEtaEstimate::Unreachable => "N/A".to_string(),
        FleetEtaEstimate::Years(years) => game_data
            .conquest
            .game_year()
            .saturating_add(years)
            .to_string(),
    }
}

pub fn fleet_list_eta_label(game_data: &CoreGameData, fleet_idx: usize) -> String {
    match fleet_display_eta_estimate(game_data, fleet_idx) {
        FleetEtaEstimate::Arrived => "0".to_string(),
        FleetEtaEstimate::Stopped => "S".to_string(),
        FleetEtaEstimate::Unreachable => "X".to_string(),
        FleetEtaEstimate::Years(years) => years.to_string(),
    }
}

pub fn fleet_eta_estimate_sort_key(estimate: FleetEtaEstimate) -> (u8, u16) {
    match estimate {
        FleetEtaEstimate::Arrived => (0, 0),
        FleetEtaEstimate::Years(years) => (1, years),
        FleetEtaEstimate::Stopped => (2, 0),
        FleetEtaEstimate::Unreachable => (3, 0),
    }
}

fn fleet_display_eta_estimate(game_data: &CoreGameData, fleet_idx: usize) -> FleetEtaEstimate {
    let Some(fleet) = game_data.fleets.records.get(fleet_idx) else {
        return FleetEtaEstimate::Unreachable;
    };
    if fleet.standing_order_kind() == Order::HoldPosition {
        return FleetEtaEstimate::Arrived;
    }
    let current = fleet.current_location_coords_raw();
    let target = fleet.standing_order_target_coords_raw();
    if current == target {
        return FleetEtaEstimate::Arrived;
    }
    let map_size = map_size_for_player_count(game_data.conquest.player_count());
    if target[0] == 0 || target[1] == 0 || target[0] > map_size || target[1] > map_size {
        return FleetEtaEstimate::Unreachable;
    }
    estimate_fleet_eta_to_destination(game_data, fleet_idx, target, false, true)
}
