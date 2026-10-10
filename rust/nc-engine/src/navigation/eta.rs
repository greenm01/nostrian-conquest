use nc_data::{CoreGameData, Order, Real48};

use super::motion::{ArrivalRule, Course, Motion, run_year, target_depth};
use super::{FleetEtaEstimate, plan_route, plan_route_to_destination};

/// Years after which a replayed trip counts as never arriving.
const ETA_YEAR_LIMIT: u16 = 1000;

pub(super) fn estimate_fleet_eta(game_data: &CoreGameData, fleet_idx: usize) -> FleetEtaEstimate {
    let Some(fleet) = game_data.fleets.records.get(fleet_idx) else {
        return FleetEtaEstimate::Unreachable;
    };
    let order = fleet.standing_order_kind();
    let max_speed = if order == Order::HoldPosition {
        0
    } else {
        fleet.max_speed()
    };
    estimate_eta(
        plan_route(game_data, fleet_idx).is_some(),
        Motion::of_fleet(fleet),
        fleet.current_speed(),
        max_speed,
        order,
        fleet.standing_order_target_coords_raw(),
        target_depth(fleet.world_target_flag_raw()),
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
    if fleet.current_location_coords_raw() == destination {
        return FleetEtaEstimate::Arrived;
    }
    let speed = if fleet.current_speed() == 0 && use_max_speed_if_stopped {
        fleet.max_speed().max(1)
    } else {
        fleet.current_speed()
    };
    let max_speed = if use_max_speed_if_stopped {
        fleet.max_speed()
    } else {
        0
    };
    estimate_eta(
        plan_route_to_destination(game_data, fleet_idx, destination).is_some(),
        Motion::of_fleet(fleet),
        speed,
        max_speed,
        Order::MoveOnly,
        destination,
        target_depth(u8::from(include_system)),
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
    match estimate_eta(
        true,
        Motion::at_rest(current),
        speed,
        speed,
        Order::MoveOnly,
        target,
        target_depth(u8::from(include_system)),
    ) {
        FleetEtaEstimate::Years(years) => years,
        _ => 0,
    }
}

/// Replays the maintenance integrator year by year until the order's arrival
/// test passes. A fleet that already passes it has arrived.
fn estimate_eta(
    reachable: bool,
    motion: Motion,
    speed: u8,
    max_speed: u8,
    order: Order,
    target: [u8; 2],
    depth: Real48,
) -> FleetEtaEstimate {
    if ArrivalRule::for_order(order).passes(motion, target, depth, speed, false) {
        return FleetEtaEstimate::Arrived;
    }
    if speed == 0 && max_speed == 0 {
        return FleetEtaEstimate::Stopped;
    }
    if !reachable {
        return FleetEtaEstimate::Unreachable;
    }
    let mut course = Course::direct(target);
    let (mut motion, mut speed) = (motion, speed);
    for years in 1..=ETA_YEAR_LIMIT {
        let year = run_year(motion, speed, max_speed, order, target, depth, &mut course);
        if year.resolved_tick.is_some() {
            return FleetEtaEstimate::Years(years);
        }
        (motion, speed) = (year.motion, year.speed);
    }
    FleetEtaEstimate::Unreachable
}
