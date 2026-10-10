use super::VisibleHazardIntel;

pub(crate) fn visible_hazard_intel_is_empty(intel: &VisibleHazardIntel) -> bool {
    intel.foreign_worlds.is_empty()
        && intel.foreign_starbases.is_empty()
        && intel.foreign_fleets.is_empty()
        && intel.hostile_blockades.is_empty()
        && intel.hostile_homeworlds.is_empty()
}

/// Squared sector distance, wide enough for any pair of map coordinates.
pub(crate) fn sector_distance_sq(from: [u8; 2], to: [u8; 2]) -> u32 {
    let dx = u32::from(from[0].abs_diff(to[0]));
    let dy = u32::from(from[1].abs_diff(to[1]));
    dx * dx + dy * dy
}
