//! ECGAME's choice of target for Seek Home and Salvage, and ECMAINT's picks
//! for fleet orders.
//!
//! ECGAME (`0xf5f0` in the unpacked image) lists the worlds the fleet's
//! empire owns in PLANETS.DAT order and takes the one with the shortest
//! travel distance (`0x1d8ac`), keeping the first on a tie. With
//! `skip_current_system` it leaves out the world in the fleet's sector when
//! the fleet is in that system (depth at least 0.5); Seek Home uses this for
//! "seek the next closest system". ECMAINT's picker (`24a3:0876`) is the
//! same list and measure; flag 1 is the call above, flag 2 divides each
//! distance by the world's present production first
//! ([`CoreGameData::nearest_productive_world`]).

use super::*;
use crate::{Planet, Real48};

impl CoreGameData {
    /// The owned world ECGAME picks for this fleet's Seek Home or Salvage
    /// order, or `None` when its empire owns no world.
    pub fn nearest_owned_world(
        &self,
        fleet_record_index_1_based: usize,
        skip_current_system: bool,
    ) -> Option<[u8; 2]> {
        self.pick_owned_world(fleet_record_index_1_based, skip_current_system, false)
    }

    /// ECMAINT's picker with flag 2, which the rogue and autopilot orders
    /// use (`24a3:26c2`): every world the fleet's empire owns, the one in
    /// the fleet's own system included, ranked by travel distance divided by
    /// present production, a production below 1.0 counting as 1.0. The
    /// earlier world wins a tie and a lone world is taken unmeasured.
    /// ECMAINT also raises the stored production of each candidate below 1.0
    /// to 1.0; that is a classic bug and nothing is written here.
    pub fn nearest_productive_world(&self, fleet_record_index_1_based: usize) -> Option<[u8; 2]> {
        self.pick_owned_world(fleet_record_index_1_based, false, true)
    }

    fn pick_owned_world(
        &self,
        fleet_record_index_1_based: usize,
        skip_current_system: bool,
        per_production: bool,
    ) -> Option<[u8; 2]> {
        let fleet = self.fleets.get(fleet_record_index_1_based - 1)?;
        let here = fleet.sector.to_array();
        let in_system = fleet.depth >= Real48::HALF;
        let mut candidates = self
            .planets
            .iter()
            .filter(|planet| planet.owner.0 == fleet.owner.0)
            .filter(|planet| {
                !(skip_current_system && in_system && planet.sector.to_array() == here)
            });
        let first = candidates.next()?;
        let measure = |planet: &Planet| {
            let distance = travel_distance(fleet, planet.sector.to_array(), Real48::ONE);
            if per_production {
                distance / planet.production.max(Real48::ONE)
            } else {
                distance
            }
        };
        let Some(second) = candidates.next() else {
            return Some(first.sector.to_array());
        };
        let mut best = first;
        let mut best_measure = measure(first);
        for planet in std::iter::once(second).chain(candidates) {
            let planet_measure = measure(planet);
            if planet_measure < best_measure {
                best = planet;
                best_measure = planet_measure;
            }
        }
        Some(best.sector.to_array())
    }

    /// ECGAME's single-fleet Seek Home flow.
    pub fn seek_home_target(&self, fleet_record_index_1_based: usize) -> SeekHomeTarget {
        let Some(nearest) = self.nearest_owned_world(fleet_record_index_1_based, false) else {
            return SeekHomeTarget::NoOwnedWorld;
        };
        let fleet = &self.fleets[fleet_record_index_1_based - 1];
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
fn fleet_is_in_system(fleet: &Fleet, coords: [u8; 2]) -> bool {
    fleet.sector.to_array() == coords && fleet.depth.round() == 1
}

/// ECGAME's travel distance from the fleet to the target cell at depth
/// `target_depth` (`0x1d8ac`), in 6-byte reals.
fn travel_distance(fleet: &Fleet, target: [u8; 2], target_depth: Real48) -> Real48 {
    let here = fleet.sector.to_array();
    let depth = fleet.depth;
    if here == target && depth.round() == target_depth.round() {
        return Real48::ZERO;
    }
    let to_centre = target_depth == Real48::ONE;
    let dx = axis_distance(here[0], fleet.x_fraction, target[0], to_centre);
    let dy = axis_distance(here[1], fleet.y_fraction, target[1], to_centre);
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
