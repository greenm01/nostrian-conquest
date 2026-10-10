use crate::navigation::motion::{ArrivalRule, Course, FleetYear, Motion, run_year, target_depth};
use crate::navigation::{plan_route_with_intel, visible_hazard_intel_is_empty};
use crate::{CoreGameData, Order, VisibleHazardIntel};
use nc_data::Real48;

pub(super) fn set_fleet_to_local_hold(fleet: &mut nc_data::FleetRecord) {
    let coords = fleet.current_location_coords_raw();
    fleet.set_current_speed(0);
    fleet.set_standing_order_kind(Order::HoldPosition);
    fleet.set_standing_order_target_coords_raw(coords);
}

fn order_persists_on_arrival(order: Order) -> bool {
    // Note: Scout missions (ScoutSector, ScoutSolarSystem) are intentionally
    // excluded here. Although the manual defines them as persistent, the engine
    // reverts them to HoldPosition during the arrival phase to "clean" the
    // fleet's context for combat resolution (avoiding forced System Entry
    // hostility). The order is restored at the end of maintenance in mod.rs;
    // this "Revert-then-Restore" cycle is what implements persistent stealth.
    matches!(
        order,
        Order::PatrolSector
            | Order::GuardStarbase
            | Order::GuardBlockadeWorld
            | Order::JoinAnotherFleet
            | Order::RendezvousSector
            | Order::ViewWorld
    )
}

fn order_keeps_speed_on_arrival(order: Order) -> bool {
    matches!(order, Order::JoinAnotherFleet)
}

pub(super) fn is_hostile_order(order: Order) -> bool {
    matches!(
        order,
        Order::BombardWorld | Order::InvadeWorld | Order::BlitzWorld
    )
}

/// How a fleet's year of movement ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum YearEnd {
    /// Still under way (or idle) at year end.
    Underway,
    /// The order's arrival test passed and the order resolved.
    Resolved,
    /// A hostile order's strike test passed. The record keeps the motion and
    /// speed from one tick before the strike, which is where a failed attack
    /// leaves it to retry next year.
    Strike,
}

pub(super) struct FleetMovement {
    pub end: YearEnd,
    /// The fleet entered its target sector from outside this year.
    pub entered_target: bool,
    /// Speed the fleet travelled at, 0 when it did not move.
    pub travel_speed: u8,
}

/// Run one year (52 ticks) of a fleet's movement and arrival test.
pub(super) fn process_single_fleet_movement(
    game_data: &mut CoreGameData,
    fleet_idx: usize,
    visible_hazards_by_empire: &[VisibleHazardIntel],
) -> FleetMovement {
    let fleet = &game_data.fleets.records[fleet_idx];
    let order = fleet.standing_order_kind();
    let start = fleet.current_location_coords_raw();
    let target = fleet.standing_order_target_coords_raw();
    let depth = target_depth(fleet.world_target_flag_raw());
    // A standing order already settled on station stays put; it resolved in
    // an earlier year (scouts report from station in the observation phase).
    let on_station = (order_persists_on_arrival(order)
        || matches!(order, Order::ScoutSector | Order::ScoutSolarSystem))
        && ArrivalRule::for_order(order).passes(
            Motion::of_fleet(fleet),
            target,
            depth,
            fleet.current_speed(),
            false,
        );
    if on_station || matches!(order, Order::HoldPosition | Order::Unknown(_)) {
        return FleetMovement {
            end: YearEnd::Underway,
            entered_target: false,
            travel_speed: 0,
        };
    }
    let mut course = fleet_course(game_data, fleet_idx, visible_hazards_by_empire, target);
    let fleet = &game_data.fleets.records[fleet_idx];
    let year = run_year(
        Motion::of_fleet(fleet),
        fleet.current_speed(),
        fleet.max_speed(),
        order,
        target,
        depth,
        &mut course,
    );
    let travel_speed = year.speed.max(fleet.current_speed());
    let entered_target = start != target && year.motion.sector() == target;
    let fleet = &mut game_data.fleets.records[fleet_idx];
    let end = match year.resolved_tick {
        None => {
            year.motion.write_to(fleet);
            fleet.set_current_speed(year.speed);
            YearEnd::Underway
        }
        Some(_) if is_hostile_order(order) => {
            let (motion, speed) = year.before_resolution;
            motion.write_to(fleet);
            fleet.set_current_speed(speed);
            YearEnd::Strike
        }
        Some(_) => {
            year.motion.write_to(fleet);
            apply_resolution_state(fleet, order, &year);
            YearEnd::Resolved
        }
    };
    FleetMovement {
        end,
        entered_target,
        travel_speed,
    }
}

fn fleet_course(
    game_data: &CoreGameData,
    fleet_idx: usize,
    visible_hazards_by_empire: &[VisibleHazardIntel],
    target: [u8; 2],
) -> Course {
    let owner = game_data.fleets.records[fleet_idx].owner_empire_raw();
    let Some(intel) = visible_hazards_by_empire.get(usize::from(owner.saturating_sub(1))) else {
        return Course::direct(target);
    };
    if visible_hazard_intel_is_empty(intel) {
        return Course::direct(target);
    }
    match plan_route_with_intel(game_data, fleet_idx, intel) {
        Some(route) => Course::through(
            route.steps.iter().skip(1).map(|step| step.coords).collect(),
            target,
        ),
        None => Course::direct(target),
    }
}

/// What a resolved order leaves on the fleet besides its arrival position.
fn apply_resolution_state(fleet: &mut nc_data::FleetRecord, order: Order, year: &FleetYear) {
    fleet.set_current_speed(year.speed);
    if !order_persists_on_arrival(order) {
        fleet.set_current_speed(0);
        fleet.set_standing_order_kind(Order::HoldPosition);
        return;
    }
    if !order_keeps_speed_on_arrival(order) {
        fleet.set_current_speed(0);
    }
    if order == Order::GuardStarbase {
        // ECMAINT's guard-starbase action settles the fleet in orbit.
        fleet.set_depth(Real48::ONE);
    }
}
