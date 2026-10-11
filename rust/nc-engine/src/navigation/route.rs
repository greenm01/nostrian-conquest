use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use nc_data::{CoreGameData, Fleet, Order, map_size_for_player_count};

use super::{PlannedRoute, RouteStep, VisibleHazardIntel};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FrontierNode {
    estimated_total_cost: u32,
    cost_so_far: u32,
    coords: [u8; 2],
}

impl Ord for FrontierNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .estimated_total_cost
            .cmp(&self.estimated_total_cost)
            .then_with(|| other.cost_so_far.cmp(&self.cost_so_far))
            .then_with(|| other.coords.cmp(&self.coords))
    }
}

impl PartialOrd for FrontierNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

const BASE_STEP_COST: u32 = 10;
const FOREIGN_WORLD_COST: u32 = 70;
const FOREIGN_FLEET_COST: u32 = 55;
const STARBASE_COST: u32 = 90;
const BLOCKADE_COST: u32 = 80;
const HOMEWORLD_COST: u32 = 45;
const HARD_BLOCK_COST: u32 = u32::MAX / 4;

pub(super) fn plan_route(game_data: &CoreGameData, fleet_idx: usize) -> Option<PlannedRoute> {
    let intel = VisibleHazardIntel::default();
    plan_route_with_intel(game_data, fleet_idx, &intel)
}

pub(super) fn plan_route_with_intel(
    game_data: &CoreGameData,
    fleet_idx: usize,
    intel: &VisibleHazardIntel,
) -> Option<PlannedRoute> {
    let fleet = game_data.fleets.get(fleet_idx)?;
    let order = fleet.standing_order_kind();
    if !order_uses_pathfinding(order) {
        return None;
    }

    let start = fleet.current_location_coords_raw();
    let goal = fleet.standing_order_target_coords_raw();
    if start == goal {
        return Some(PlannedRoute {
            steps: vec![RouteStep { coords: start }],
        });
    }

    let map_size = map_size_for_player_count(game_data.conquest.player_count());
    let mut frontier = BinaryHeap::new();
    frontier.push(FrontierNode {
        estimated_total_cost: heuristic_cost(start, goal),
        cost_so_far: 0,
        coords: start,
    });

    let mut came_from: HashMap<[u8; 2], [u8; 2]> = HashMap::new();
    let mut best_cost: HashMap<[u8; 2], u32> = HashMap::new();
    best_cost.insert(start, 0);

    while let Some(current) = frontier.pop() {
        if current.coords == goal {
            return Some(reconstruct_route(start, goal, &came_from));
        }

        for next in neighbors(current.coords, map_size) {
            let step_cost = sector_cost(fleet, order, goal, next, intel);
            if step_cost >= HARD_BLOCK_COST {
                continue;
            }

            let new_cost = current.cost_so_far.saturating_add(step_cost);
            let is_better = best_cost
                .get(&next)
                .map(|existing| new_cost < *existing)
                .unwrap_or(true);
            if is_better {
                best_cost.insert(next, new_cost);
                came_from.insert(next, current.coords);
                frontier.push(FrontierNode {
                    estimated_total_cost: new_cost.saturating_add(heuristic_cost(next, goal)),
                    cost_so_far: new_cost,
                    coords: next,
                });
            }
        }
    }

    None
}

pub(super) fn next_path_step(route: &PlannedRoute, max_steps: usize) -> Option<[u8; 2]> {
    if route.steps.len() <= 1 {
        return route.steps.last().map(|step| step.coords);
    }
    let idx = max_steps.min(route.steps.len() - 1);
    route.steps.get(idx).map(|step| step.coords)
}

fn order_uses_pathfinding(order: Order) -> bool {
    matches!(
        order,
        Order::MoveOnly
            | Order::SeekHome
            | Order::PatrolSector
            | Order::ViewWorld
            | Order::ScoutSector
            | Order::ScoutSolarSystem
            | Order::ColonizeWorld
            | Order::JoinAnotherFleet
            | Order::RendezvousSector
    )
}

fn heuristic_cost(from: [u8; 2], to: [u8; 2]) -> u32 {
    let dx = from[0].abs_diff(to[0]) as u32;
    let dy = from[1].abs_diff(to[1]) as u32;
    (dx + dy) * BASE_STEP_COST
}

/// Adjacent sectors inside the 1-based `1..=map_size` sector grid.
fn neighbors(coords: [u8; 2], map_size: u8) -> impl Iterator<Item = [u8; 2]> {
    let x = coords[0] as i16;
    let y = coords[1] as i16;
    let max = map_size as i16;
    let mut out = Vec::with_capacity(8);
    for dx in -1..=1 {
        for dy in -1..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let nx = x + dx;
            let ny = y + dy;
            if nx >= 1 && nx <= max && ny >= 1 && ny <= max {
                out.push([nx as u8, ny as u8]);
            }
        }
    }
    out.into_iter()
}

fn sector_cost(
    _fleet: &Fleet,
    order: Order,
    final_goal: [u8; 2],
    coords: [u8; 2],
    intel: &VisibleHazardIntel,
) -> u32 {
    if coords == final_goal {
        return BASE_STEP_COST;
    }

    let hostile_targeting = matches!(
        order,
        Order::BombardWorld
            | Order::InvadeWorld
            | Order::BlitzWorld
            | Order::GuardBlockadeWorld
            | Order::GuardStarbase
    );

    let mut cost = BASE_STEP_COST;

    if intel.foreign_worlds.contains(&coords) {
        if hostile_targeting && coords == final_goal {
            return BASE_STEP_COST;
        }
        cost = cost.saturating_add(FOREIGN_WORLD_COST);
    }
    if intel.foreign_starbases.contains(&coords) {
        if hostile_targeting && coords == final_goal {
            return BASE_STEP_COST;
        }
        cost = cost.saturating_add(STARBASE_COST);
    }
    if intel.foreign_fleets.contains(&coords) {
        if hostile_targeting && coords == final_goal {
            return BASE_STEP_COST;
        }
        cost = cost.saturating_add(FOREIGN_FLEET_COST);
    }
    if intel.hostile_blockades.contains(&coords) {
        cost = cost.saturating_add(BLOCKADE_COST);
    }
    if intel.hostile_homeworlds.contains(&coords) {
        cost = cost.saturating_add(HOMEWORLD_COST);
    }

    cost
}

fn reconstruct_route(
    start: [u8; 2],
    goal: [u8; 2],
    came_from: &HashMap<[u8; 2], [u8; 2]>,
) -> PlannedRoute {
    let mut coords = vec![goal];
    let mut cursor = goal;
    while cursor != start {
        let Some(prev) = came_from.get(&cursor) else {
            break;
        };
        cursor = *prev;
        coords.push(cursor);
    }
    coords.reverse();
    PlannedRoute {
        steps: coords
            .into_iter()
            .map(|coords| RouteStep { coords })
            .collect(),
    }
}
