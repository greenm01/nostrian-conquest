//! World orders watch their target world's owner each tick, in the fleet's
//! dispatch after its step and before its arrival test, as ECMAINT's order
//! dispatcher does (`0b5d:5581`; assaults `ec-battle-spec.md` §2.1).
//!
//! - Bombard, Invade and Blitz at a world the fleet's empire owns guard it
//!   (warships and ROE above 0) or move to it.
//! - View at such a world moves to its sector instead.
//! - Colonize at such a world holds where it is.
//! - Colonize at a world another empire owns gives up once it is in the
//!   system and climbs back out to the sector.
//!
//! Scout System checks its world on arrival (`arrivals.rs`).

use super::super::ColonizationEvent;
use super::YearMovement;
use crate::{CoreGameData, Order};
use nc_data::{Mission, MissionRetargetEvent, has_combat_ships};

impl YearMovement<'_> {
    /// Check fleet `fleet_idx`'s target world. Returns whether the order
    /// changed, which ends the tick without an arrival test.
    pub(super) fn check_target_world(
        &mut self,
        game_data: &mut CoreGameData,
        fleet_idx: usize,
    ) -> bool {
        let fleet = &game_data.fleets[fleet_idx];
        let (owner, target, sector) = (
            fleet.owner.0,
            fleet.target.to_array(),
            fleet.sector.to_array(),
        );
        let Some(world_owner) = game_data
            .planets
            .iter()
            .find(|planet| planet.sector.to_array() == target)
            .map(|planet| planet.owner.0)
        else {
            return false;
        };
        let in_system = fleet.depth.round() == 1;
        let ours = world_owner == owner;
        let (mission, follow_on) = match fleet.order {
            Order::BombardWorld | Order::InvadeWorld | Order::BlitzWorld if ours => {
                let guards = has_combat_ships(fleet) && fleet.rules_of_engagement > 0;
                let mission = match fleet.order {
                    Order::BombardWorld => Mission::BombardWorld,
                    Order::InvadeWorld => Mission::InvadeWorld,
                    _ => Mission::BlitzWorld,
                };
                let follow_on = if guards {
                    Order::GuardBlockadeWorld
                } else {
                    Order::MoveOnly
                };
                (mission, follow_on)
            }
            Order::ViewWorld if ours => (Mission::ViewWorld, Order::MoveOnly),
            Order::ColonizeWorld if ours => (Mission::ColonizeWorld, Order::HoldPosition),
            Order::ColonizeWorld if world_owner != 0 && sector == target && in_system => {
                // The colony attempt resolves after the year as blocked, in
                // this week; the fleet climbs out meanwhile.
                self.events.colonization_events.push(ColonizationEvent {
                    fleet_idx,
                    coords: target,
                    owner_empire: owner,
                    stardate_week: None,
                });
                let fleet = &mut game_data.fleets[fleet_idx];
                fleet.order = Order::MoveOnly;
                fleet.target_depth = 0;
                return true;
            }
            _ => return false,
        };
        let fleet = &mut game_data.fleets[fleet_idx];
        fleet.order = follow_on;
        match follow_on {
            Order::HoldPosition => {
                // Classic's Colonize holds before it moves this tick.
                self.tracks[fleet_idx].undo_step(fleet);
                fleet.speed = 0;
                fleet.target = fleet.sector;
            }
            Order::MoveOnly if mission == Mission::ViewWorld => fleet.target_depth = 0,
            _ => {
                fleet.target_depth = 1;
                if fleet.speed == 0 {
                    fleet.speed = fleet.max_speed;
                }
            }
        }
        self.events
            .retargets
            .push(MissionRetargetEvent::TargetWorldOwned {
                fleet_idx,
                reporting_fleet_number: Some(fleet.fleet_number.0 as u8),
                owner_empire: owner,
                mission,
                target_coords: target,
                coords: sector,
                in_system,
                follow_on,
                stardate_week: None,
            });
        true
    }
}
