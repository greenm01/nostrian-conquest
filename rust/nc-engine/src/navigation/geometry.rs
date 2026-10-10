use super::{PlannedRoute, VisibleHazardIntel};

pub(crate) fn visible_hazard_intel_is_empty(intel: &VisibleHazardIntel) -> bool {
    intel.foreign_worlds.is_empty()
        && intel.foreign_starbases.is_empty()
        && intel.foreign_fleets.is_empty()
        && intel.hostile_blockades.is_empty()
        && intel.hostile_homeworlds.is_empty()
}

pub(crate) fn advance_exact_position(
    exact_start: [f64; 2],
    target: [u8; 2],
    travel_distance: f64,
    route: Option<&PlannedRoute>,
    use_route_geometry: bool,
) -> [f64; 2] {
    if travel_distance <= 0.0 {
        return exact_start;
    }

    if use_route_geometry {
        if let Some(route) = route {
            return advance_along_route(exact_start, route, travel_distance);
        }
    }

    advance_toward(
        exact_start,
        [f64::from(target[0]), f64::from(target[1])],
        travel_distance,
    )
}

pub(crate) fn rounded_coords_from_exact(exact: [f64; 2], target: [u8; 2]) -> [u8; 2] {
    let rounded_x = exact[0].round().clamp(0.0, f64::from(u8::MAX)) as u8;
    let rounded_y = exact[1].round().clamp(0.0, f64::from(u8::MAX)) as u8;
    if rounded_x == target[0] && rounded_y == target[1] {
        target
    } else {
        [rounded_x, rounded_y]
    }
}

/// Movement orders complete at the exact path endpoint, not when the rounded
/// visible sector first matches the target.
pub(crate) fn exact_position_reached_target(exact: [f64; 2], target: [u8; 2]) -> bool {
    (exact[0] - f64::from(target[0])).abs() <= f64::EPSILON
        && (exact[1] - f64::from(target[1])).abs() <= f64::EPSILON
}

/// Squared sector distance, wide enough for any pair of map coordinates.
pub(crate) fn sector_distance_sq(from: [u8; 2], to: [u8; 2]) -> u32 {
    let dx = u32::from(from[0].abs_diff(to[0]));
    let dy = u32::from(from[1].abs_diff(to[1]));
    dx * dx + dy * dy
}

fn advance_along_route(
    exact_start: [f64; 2],
    route: &PlannedRoute,
    mut travel_distance: f64,
) -> [f64; 2] {
    let mut current = exact_start;

    for step in route.steps.iter().skip(1) {
        let waypoint = [f64::from(step.coords[0]), f64::from(step.coords[1])];
        let dx = waypoint[0] - current[0];
        let dy = waypoint[1] - current[1];
        let segment_distance = (dx * dx + dy * dy).sqrt();
        if segment_distance <= f64::EPSILON {
            current = waypoint;
            continue;
        }

        if travel_distance < segment_distance {
            return [
                current[0] + dx * (travel_distance / segment_distance),
                current[1] + dy * (travel_distance / segment_distance),
            ];
        }

        current = waypoint;
        travel_distance -= segment_distance;
    }

    current
}

fn advance_toward(from: [f64; 2], to: [f64; 2], travel_distance: f64) -> [f64; 2] {
    let dx = to[0] - from[0];
    let dy = to[1] - from[1];
    let distance = (dx * dx + dy * dy).sqrt();
    if distance <= f64::EPSILON || travel_distance >= distance {
        return to;
    }

    [
        from[0] + dx * (travel_distance / distance),
        from[1] + dy * (travel_distance / distance),
    ]
}
