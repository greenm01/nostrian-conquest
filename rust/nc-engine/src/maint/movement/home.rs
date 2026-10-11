//! Seek Home and Salvage watch their world: each tick, in the fleet's
//! dispatch after its step and before its arrival test (ECMAINT `0b5d:01f9`
//! Seek Home, `0b5d:4a44` Salvage), the order's target world must still be
//! the fleet's empire's. A fleet whose world is lost, or never was its
//! empire's, turns for the nearest friendly world.

use super::YearMovement;
use crate::maint::combat::nearest_friendly_world;
use crate::{CoreGameData, Order};
use nc_data::{Coords, Mission, MissionRetargetEvent};

impl YearMovement<'_> {
    /// Check the target world of fleet `fleet_idx`, if it is on Seek Home or
    /// Salvage. Returns whether the order changed, which ends the tick
    /// without an arrival test.
    ///
    /// The picker is classic's flag 1, measured from where the fleet has just
    /// stepped. Only the target sector changes (Salvage also refills a zero
    /// speed); the order and its target depth stay. With no other world the
    /// fleet holds where it is, speed 0 and target depth the layer it is
    /// nearer, its depth untouched. Classic snaps a Salvage fleet's depth to
    /// that layer; NC does not.
    pub(super) fn check_home_world(
        &mut self,
        game_data: &mut CoreGameData,
        fleet_idx: usize,
    ) -> bool {
        let fleet = &game_data.fleets[fleet_idx];
        let mission = match fleet.order {
            Order::SeekHome => Mission::SeekHome,
            Order::Salvage => Mission::Salvage,
            _ => return false,
        };
        let (owner, target, sector) = (
            fleet.owner.0,
            fleet.target.to_array(),
            fleet.sector.to_array(),
        );
        if game_data
            .planets
            .iter()
            .any(|planet| planet.sector.to_array() == target && planet.owner.0 == owner)
        {
            return false;
        }

        let reporting_fleet_number = Some(fleet.fleet_number.0 as u8);
        let in_system = fleet.depth.round() == 1;
        let new_target = nearest_friendly_world(game_data, fleet_idx);
        let fleet = &mut game_data.fleets[fleet_idx];
        let event = match new_target {
            Some(new_target) => {
                fleet.target = Coords::from_array(new_target);
                if mission == Mission::Salvage && fleet.speed == 0 {
                    fleet.speed = fleet.max_speed;
                }
                MissionRetargetEvent::Retargeted {
                    fleet_idx,
                    reporting_fleet_number,
                    owner_empire: owner,
                    mission,
                    current_coords: sector,
                    in_system,
                    previous_target_coords: target,
                    new_target_coords: new_target,
                    stardate_week: None,
                }
            }
            None => {
                fleet.order = Order::HoldPosition;
                fleet.speed = 0;
                fleet.target = fleet.sector;
                fleet.target_depth = fleet.depth.round() as u8;
                fleet.escape = None;
                MissionRetargetEvent::Abandoned {
                    fleet_idx,
                    reporting_fleet_number,
                    owner_empire: owner,
                    mission,
                    previous_target_coords: target,
                    coords: sector,
                    in_system,
                    stardate_week: None,
                }
            }
        };
        self.events.retargets.push(event);
        true
    }
}
