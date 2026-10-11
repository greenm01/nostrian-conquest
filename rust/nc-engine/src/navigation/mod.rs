mod eta;
mod geometry;
mod intel;
pub(crate) mod motion;
mod route;

use std::collections::{BTreeMap, HashSet};

use nc_data::{Coords, CoreGameData, Order, PlanetIntelSnapshot};

pub(crate) use geometry::{sector_distance_sq, visible_hazard_intel_is_empty};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteStep {
    pub coords: [u8; 2],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedRoute {
    pub steps: Vec<RouteStep>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FleetEtaEstimate {
    Arrived,
    Stopped,
    Unreachable,
    Years(u16),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VisibleHazardIntel {
    pub foreign_worlds: HashSet<[u8; 2]>,
    pub foreign_starbases: HashSet<[u8; 2]>,
    pub foreign_fleets: HashSet<[u8; 2]>,
    pub hostile_blockades: HashSet<[u8; 2]>,
    pub hostile_homeworlds: HashSet<[u8; 2]>,
}

pub fn plan_route(game_data: &CoreGameData, fleet_idx: usize) -> Option<PlannedRoute> {
    route::plan_route(game_data, fleet_idx)
}

pub fn visible_hazard_intel_from_snapshots(
    game_data: &CoreGameData,
    snapshots: &BTreeMap<usize, PlanetIntelSnapshot>,
    viewer_empire: u8,
) -> VisibleHazardIntel {
    intel::visible_hazard_intel_from_snapshots(game_data, snapshots, viewer_empire)
}

pub fn plan_route_with_intel(
    game_data: &CoreGameData,
    fleet_idx: usize,
    intel: &VisibleHazardIntel,
) -> Option<PlannedRoute> {
    route::plan_route_with_intel(game_data, fleet_idx, intel)
}

pub fn next_path_step(route: &PlannedRoute, max_steps: usize) -> Option<[u8; 2]> {
    route::next_path_step(route, max_steps)
}

pub fn estimate_fleet_eta(game_data: &CoreGameData, fleet_idx: usize) -> FleetEtaEstimate {
    eta::estimate_fleet_eta(game_data, fleet_idx)
}

pub fn estimate_fleet_eta_to_destination(
    game_data: &CoreGameData,
    fleet_idx: usize,
    destination: [u8; 2],
    include_system: bool,
    use_max_speed_if_stopped: bool,
) -> FleetEtaEstimate {
    eta::estimate_fleet_eta_to_destination(
        game_data,
        fleet_idx,
        destination,
        include_system,
        use_max_speed_if_stopped,
    )
}

pub fn estimate_direct_eta(
    current: [u8; 2],
    target: [u8; 2],
    speed: u8,
    include_system: bool,
) -> u16 {
    eta::estimate_direct_eta(current, target, speed, include_system)
}

pub(crate) fn plan_route_to_destination(
    game_data: &CoreGameData,
    fleet_idx: usize,
    destination: [u8; 2],
) -> Option<PlannedRoute> {
    let mut game_data = game_data.clone();
    let fleet = game_data.fleets.get_mut(fleet_idx)?;
    fleet.order = Order::MoveOnly;
    fleet.target = Coords::from_array(destination);
    route::plan_route(&game_data, fleet_idx)
}
