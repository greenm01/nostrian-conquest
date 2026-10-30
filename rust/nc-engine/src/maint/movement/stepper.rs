use crate::navigation::motion::{
    ArrivalRule, Course, Motion, TICKS_PER_YEAR, advance, target_depth,
};
use crate::navigation::{plan_route_with_intel, visible_hazard_intel_is_empty};
use crate::{CoreGameData, Order, VisibleHazardIntel};
use nc_data::Real48;

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

/// What a fleet's movement step did in one tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::maint) enum TickEnd {
    /// Still under way, or not moving.
    Underway,
    /// The order's arrival test passed and the order resolved.
    Resolved,
    /// A hostile order's strike test passed. The fleet stops in orbit
    /// (depth 1.0, speed 0) on its order; the strike it makes now decides
    /// what the order becomes.
    Strike,
}

/// A fleet's order as its movement sees it: the order, target, and target
/// depth flag. A change of any of them (an aborted mission, a retreat)
/// starts a new course.
type OrderKey = (u8, [u8; 2], u8);

fn order_key(fleet: &nc_data::Fleet) -> OrderKey {
    (
        fleet.order.code(),
        fleet.target.to_array(),
        fleet.target_depth,
    )
}

/// One fleet's movement through the year, a tick at a time (ECMAINT
/// `0824:01c0` for the step, `0b5d:5581` for the arrival test).
#[derive(Debug, Clone, Default)]
pub(super) struct FleetTrack {
    /// The fleet's position in full precision; `None` reads it from the
    /// record (after something else moved the fleet).
    motion: Option<Motion>,
    /// Motion and speed at the start of this tick, if the fleet stepped.
    before: Option<(Motion, u8)>,
    course: Option<(OrderKey, Course)>,
    /// The order resolved this year; the fleet stops until it changes.
    resolved: Option<OrderKey>,
    /// A resolved View World climbing back out of the system (ECMAINT turns
    /// the order into a Move out at full speed, `0b5d:219c`).
    viewing_out: Option<OrderKey>,
    /// The week a View World fleet finished climbing out, until taken.
    pub(super) climbed_out: Option<u8>,
    pub(super) travel_speed: u8,
}

impl FleetTrack {
    /// The order classic would see: a View fleet climbing out is on a Move
    /// to depth 0.
    pub(super) fn effective_order(&self, fleet: &nc_data::Fleet) -> (Order, u8) {
        if self.viewing_out == Some(order_key(fleet)) {
            (Order::MoveOnly, 0)
        } else {
            (fleet.order, fleet.target_depth)
        }
    }

    /// Whether the fleet stepped this tick.
    pub(super) fn stepped(&self) -> bool {
        self.before.is_some()
    }

    /// The order resolved this tick; the fleet stops until it changes.
    pub(super) fn resolve(&mut self, fleet: &nc_data::Fleet) {
        self.resolved = Some(order_key(fleet));
        self.motion = None;
    }

    /// Forget the cached position; the record was changed elsewhere.
    pub(super) fn reload(&mut self) {
        self.motion = None;
    }

    /// Undo this tick's step (an attacker loses the tick's movement).
    pub(super) fn undo_step(&mut self, fleet: &mut nc_data::Fleet) {
        if let Some((motion, _)) = self.before {
            motion.write_to(fleet);
            self.motion = Some(motion);
        }
    }

    /// The year is over: a View fleet still climbing out stops; its viewing
    /// report turns the order into the Move out (`complete_view_world`).
    pub(super) fn finish(&self, fleet: &mut nc_data::Fleet) {
        if self.viewing_out == Some(order_key(fleet)) {
            fleet.speed = 0;
        }
    }
}

/// Step fleet `fleet_idx` one tick, before the encounter check. Returns
/// whether it moved.
pub(super) fn step_fleet(
    game_data: &mut CoreGameData,
    track: &mut FleetTrack,
    fleet_idx: usize,
    visible_hazards_by_empire: &[VisibleHazardIntel],
) -> bool {
    track.before = None;
    let fleet = &game_data.fleets[fleet_idx];
    let key = order_key(fleet);
    if track.resolved == Some(key) {
        return false;
    }
    let viewing_out = track.viewing_out == Some(key);
    let order = fleet.order;
    let target = fleet.target.to_array();
    let depth = if viewing_out {
        Real48::ZERO
    } else {
        target_depth(fleet.target_depth)
    };
    // A standing order already settled on station stays put; it resolved in
    // an earlier year (scouts report from station in the observation phase).
    // A guard always steps: its base may have moved.
    let on_station = !viewing_out
        && order != Order::GuardStarbase
        && (order_persists_on_arrival(order)
            || matches!(order, Order::ScoutSector | Order::ScoutSolarSystem))
        && ArrivalRule::for_order(order).passes(
            track.motion.unwrap_or_else(|| Motion::of_fleet(fleet)),
            target,
            depth,
            fleet.speed,
            false,
        );
    if on_station || (!viewing_out && matches!(order, Order::HoldPosition | Order::Unknown(_))) {
        return false;
    }
    if track.course.as_ref().map(|(k, _)| *k) != Some(key) {
        let course = if viewing_out {
            Course::direct(target)
        } else {
            fleet_course(game_data, fleet_idx, visible_hazards_by_empire, target)
        };
        track.course = Some((key, course));
    }
    let fleet = &mut game_data.fleets[fleet_idx];
    let motion = track.motion.unwrap_or_else(|| Motion::of_fleet(fleet));
    let speed = fleet.speed;
    let order = if viewing_out { Order::MoveOnly } else { order };
    let course = &mut track.course.as_mut().expect("course set above").1;
    let (motion_after, speed_after) =
        advance(motion, speed, fleet.max_speed, order, target, depth, course);
    track.before = Some((motion, speed));
    track.motion = Some(motion_after);
    track.travel_speed = track.travel_speed.max(speed).max(speed_after);
    motion_after.write_to(fleet);
    fleet.speed = speed_after;
    true
}

/// The arrival test after the encounter check, for a fleet that stepped
/// this tick.
pub(super) fn test_arrival(
    game_data: &mut CoreGameData,
    track: &mut FleetTrack,
    fleet_idx: usize,
    tick: u8,
) -> TickEnd {
    if track.before.is_none() {
        return TickEnd::Underway;
    }
    let fleet = &mut game_data.fleets[fleet_idx];
    let key = order_key(fleet);
    let motion = track.motion.unwrap_or_else(|| Motion::of_fleet(fleet));
    let order = fleet.order;
    let target = fleet.target.to_array();
    let last_tick = tick == TICKS_PER_YEAR;
    if track.viewing_out == Some(key) {
        if ArrivalRule::for_order(Order::MoveOnly).passes(
            motion,
            target,
            Real48::ZERO,
            fleet.speed,
            last_tick,
        ) {
            track.resolved = Some(key);
            track.climbed_out = Some(tick);
        }
        return TickEnd::Underway;
    }
    let depth = target_depth(fleet.target_depth);
    if !ArrivalRule::for_order(order).passes(motion, target, depth, fleet.speed, last_tick) {
        return TickEnd::Underway;
    }
    if order == Order::ViewWorld {
        // ECMAINT's View World action (0b5d:219c) turns the order into a
        // Move out of the system at full speed, and the fleet climbs for the
        // rest of the year. The order stays View until the viewing report
        // is finalized after the year (`complete_view_world`).
        track.viewing_out = Some(key);
        fleet.speed = fleet.max_speed;
        return TickEnd::Resolved;
    }
    track.resolved = Some(key);
    if is_hostile_order(order) {
        fleet.speed = 0;
        return TickEnd::Strike;
    }
    apply_resolution_state(fleet, order);
    track.motion = None;
    TickEnd::Resolved
}

fn fleet_course(
    game_data: &CoreGameData,
    fleet_idx: usize,
    visible_hazards_by_empire: &[VisibleHazardIntel],
    target: [u8; 2],
) -> Course {
    let owner = game_data.fleets[fleet_idx].owner.0;
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
fn apply_resolution_state(fleet: &mut nc_data::Fleet, order: Order) {
    if !order_persists_on_arrival(order) {
        fleet.speed = 0;
        fleet.order = Order::HoldPosition;
        return;
    }
    if !order_keeps_speed_on_arrival(order) {
        fleet.speed = 0;
    }
}
