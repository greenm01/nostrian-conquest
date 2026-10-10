//! ECGAME's choice of target for Seek Home and Salvage.
//!
//! ECGAME (`0xf5f0` in the unpacked image) lists the worlds the fleet's
//! empire owns in PLANETS.DAT order and takes the one with the shortest
//! travel distance (`0x1d8ac`), keeping the first on a tie. With
//! `skip_current_system` it leaves out the world in the fleet's sector when
//! the fleet is in that system (depth at least 0.5); Seek Home uses this for
//! "seek the next closest system".

use super::*;
use crate::Real48;

impl CoreGameData {
    /// The owned world ECGAME picks for this fleet's Seek Home or Salvage
    /// order, or `None` when its empire owns no world.
    pub fn nearest_owned_world(
        &self,
        fleet_record_index_1_based: usize,
        skip_current_system: bool,
    ) -> Option<[u8; 2]> {
        let fleet = self.fleets.records.get(fleet_record_index_1_based - 1)?;
        let here = fleet.current_location_coords_raw();
        let in_system = fleet.depth() >= Real48::HALF;
        let candidates = self
            .planets
            .records
            .iter()
            .filter(|planet| planet.owner_empire_slot_raw() == fleet.owner_empire_raw())
            .map(|planet| planet.coords_raw())
            .filter(|coords| !(skip_current_system && in_system && *coords == here))
            .collect::<Vec<_>>();
        if candidates.len() <= 1 {
            return candidates.first().copied();
        }
        let mut best = candidates[0];
        let mut best_distance = travel_distance(fleet, best, Real48::ONE);
        for &coords in &candidates[1..] {
            let distance = travel_distance(fleet, coords, Real48::ONE);
            if distance < best_distance {
                best = coords;
                best_distance = distance;
            }
        }
        Some(best)
    }

    /// ECGAME's single-fleet Seek Home flow.
    pub fn seek_home_target(&self, fleet_record_index_1_based: usize) -> SeekHomeTarget {
        let Some(nearest) = self.nearest_owned_world(fleet_record_index_1_based, false) else {
            return SeekHomeTarget::NoOwnedWorld;
        };
        let fleet = &self.fleets.records[fleet_record_index_1_based - 1];
        if !fleet_is_in_system(fleet, nearest) {
            return SeekHomeTarget::World(nearest);
        }
        match self.nearest_owned_world(fleet_record_index_1_based, true) {
            Some(next) => SeekHomeTarget::NextClosest(next),
            None => SeekHomeTarget::AlreadyInOnlySystem,
        }
    }
}

impl CoreGameData {
    /// ECGAME's Seek Home targets for one order given to these fleets: a lone
    /// fleet follows [`Self::seek_home_target`]; in a group each fleet heads
    /// for its own nearest world.
    pub fn seek_home_targets(
        &self,
        fleet_records_1_based: &[usize],
    ) -> Result<Vec<[u8; 2]>, SeekHomeTarget> {
        if let [record] = fleet_records_1_based {
            return match self.seek_home_target(*record) {
                SeekHomeTarget::World(target) | SeekHomeTarget::NextClosest(target) => {
                    Ok(vec![target])
                }
                other => Err(other),
            };
        }
        fleet_records_1_based
            .iter()
            .map(|&record| {
                self.nearest_owned_world(record, false)
                    .ok_or(SeekHomeTarget::NoOwnedWorld)
            })
            .collect()
    }
}

/// ECGAME's zero-ETA test: the fleet is in the target sector at the depth
/// layer of a world target (`Round(Z) == 1`).
fn fleet_is_in_system(fleet: &FleetRecord, coords: [u8; 2]) -> bool {
    fleet.current_location_coords_raw() == coords && fleet.depth().round() == 1
}

/// ECGAME's travel distance from the fleet to the target cell at depth
/// `target_depth` (`0x1d8ac`), in 6-byte reals.
fn travel_distance(fleet: &FleetRecord, target: [u8; 2], target_depth: Real48) -> Real48 {
    let here = fleet.current_location_coords_raw();
    let depth = fleet.depth();
    if here == target && depth.round() == target_depth.round() {
        return Real48::ZERO;
    }
    let to_centre = target_depth == Real48::ONE;
    let dx = axis_distance(here[0], fleet.x_fraction(), target[0], to_centre);
    let dy = axis_distance(here[1], fleet.y_fraction(), target[1], to_centre);
    let flat = (dx * dx + dy * dy).sqrt();
    if here == target {
        (depth - target_depth).abs() + flat
    } else {
        flat + depth + target_depth
    }
}

fn axis_distance(from: u8, fraction: Real48, to: u8, to_centre: bool) -> Real48 {
    let distance = match to.cmp(&from) {
        std::cmp::Ordering::Greater => Real48::from_int(i32::from(to - from)) - fraction,
        std::cmp::Ordering::Less => {
            Real48::from_int(i32::from(from)) + fraction
                - Real48::from_int(i32::from(to))
                - Real48::ONE
        }
        std::cmp::Ordering::Equal => return (Real48::HALF - fraction).abs(),
    };
    if to_centre {
        distance + Real48::HALF
    } else {
        distance
    }
}
