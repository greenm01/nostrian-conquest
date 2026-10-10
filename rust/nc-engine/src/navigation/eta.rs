use nc_data::fleet_motion_state::decode_exact_position;
use nc_data::{CoreGameData, FleetRecord};

use super::{
    FleetEtaEstimate, PlannedRoute, advance_exact_position, plan_route, plan_route_to_destination,
    rounded_coords_from_exact,
};

pub(super) fn estimate_fleet_eta(game_data: &CoreGameData, fleet_idx: usize) -> FleetEtaEstimate {
    let Some(fleet) = game_data.fleets.records.get(fleet_idx) else {
        return FleetEtaEstimate::Unreachable;
    };
    estimate_eta_for_route(
        plan_route(game_data, fleet_idx),
        fleet.current_location_coords_raw(),
        fleet.standing_order_target_coords_raw(),
        fleet.current_speed(),
        decode_movement_sub_acc(fleet),
        decode_exact_position(fleet),
        false,
    )
}

pub(super) fn estimate_fleet_eta_to_destination(
    game_data: &CoreGameData,
    fleet_idx: usize,
    destination: [u8; 2],
    include_system: bool,
    use_max_speed_if_stopped: bool,
) -> FleetEtaEstimate {
    let Some(fleet) = game_data.fleets.records.get(fleet_idx) else {
        return FleetEtaEstimate::Unreachable;
    };
    let current = fleet.current_location_coords_raw();
    let speed = if fleet.current_speed() == 0 && use_max_speed_if_stopped {
        fleet.max_speed().max(1)
    } else {
        fleet.current_speed()
    };
    let sub_acc = if fleet.current_speed() == 0 && use_max_speed_if_stopped {
        0
    } else {
        decode_movement_sub_acc(fleet)
    };
    estimate_eta_for_route(
        plan_route_to_destination(game_data, fleet_idx, destination),
        current,
        destination,
        speed,
        sub_acc,
        decode_exact_position(fleet),
        include_system,
    )
}

pub(super) fn estimate_direct_eta(
    current: [u8; 2],
    target: [u8; 2],
    speed: u8,
    include_system: bool,
) -> u16 {
    if current == target || speed == 0 {
        return 0;
    }
    // Matches the classic ECGAME travel-time display (exact accumulator), e.g.
    // the preserved starbase move "Travel time is 16 years".
    simulate_eta_years(
        [f64::from(current[0]), f64::from(current[1])],
        target,
        speed,
        0,
        include_system,
        false,
    )
}

fn estimate_eta_for_route(
    route: Option<PlannedRoute>,
    current: [u8; 2],
    target: [u8; 2],
    speed: u8,
    sub_acc_prev: u32,
    exact_position: Option<[f64; 2]>,
    include_system: bool,
) -> FleetEtaEstimate {
    if current == target {
        return FleetEtaEstimate::Arrived;
    }
    if speed == 0 {
        return FleetEtaEstimate::Stopped;
    }
    let Some(route) = route else {
        return FleetEtaEstimate::Unreachable;
    };
    if route.steps.len() <= 1 {
        return FleetEtaEstimate::Arrived;
    }
    let exact_current = exact_position.unwrap_or([f64::from(current[0]), f64::from(current[1])]);
    FleetEtaEstimate::Years(simulate_eta_years(
        exact_current,
        target,
        speed,
        sub_acc_prev,
        include_system,
        true,
    ))
}

/// `fleet_fraction_round_trip` replays the fleet stepper, which stores the
/// accumulator in the lossy classic fraction byte between turns.
fn simulate_eta_years(
    mut exact_position: [f64; 2],
    target: [u8; 2],
    speed: u8,
    sub_acc_prev: u32,
    include_system: bool,
    fleet_fraction_round_trip: bool,
) -> u16 {
    let carry = |sub_acc: u32| {
        if fleet_fraction_round_trip {
            round_trip_movement_fraction(sub_acc)
        } else {
            sub_acc
        }
    };
    let mut years = 0u16;
    let mut sub_acc = sub_acc_prev;

    while rounded_coords_from_exact(exact_position, target) != target {
        years = years.saturating_add(1);
        let sub_acc_new = sub_acc + u32::from(speed) * 8;
        let int_move = (sub_acc_new / 9) as f64;
        sub_acc = carry(sub_acc_new % 9);
        exact_position = advance_exact_position(exact_position, target, int_move, None, false);
    }

    if include_system {
        let mut remaining_system_distance = 1.0;
        while remaining_system_distance > 0.0 {
            years = years.saturating_add(1);
            let sub_acc_new = sub_acc + u32::from(speed) * 8;
            let int_move = (sub_acc_new / 9) as f64;
            sub_acc = carry(sub_acc_new % 9);
            remaining_system_distance -= int_move;
        }
    }

    years
}

fn round_trip_movement_fraction(sub_acc: u32) -> u32 {
    decode_movement_fraction(encode_movement_fraction(sub_acc))
}

fn decode_movement_sub_acc(fleet: &FleetRecord) -> u32 {
    if fleet.current_speed() == 0 || fleet.movement_state_flag_raw() == 0x80 {
        return 0;
    }
    decode_movement_fraction(fleet.movement_fraction_raw())
}

/// Encode the 0..=8 movement sub-unit accumulator as the classic FLEETS.DAT
/// raw[0x0F] fraction byte: `(sub_acc - 9) * 2 / 3` as i8.
pub(crate) fn encode_movement_fraction(sub_acc: u32) -> u8 {
    ((sub_acc as i32 - 9) * 2 / 3) as i8 as u8
}

/// Decode the classic raw[0x0F] fraction byte (`9 + raw * 3 / 2`). The
/// classic encoding is lossy, and malformed imported bytes are clamped to
/// the 0..=8 accumulator range instead of underflowing.
pub(crate) fn decode_movement_fraction(raw: u8) -> u32 {
    (9 + i32::from(raw as i8) * 3 / 2).clamp(0, 8) as u32
}
