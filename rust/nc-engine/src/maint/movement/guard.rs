//! Guard Starbase in the year's ticks: the arrival test (ECMAINT
//! `0b5d:0b3e`) and the arrival action (`0b5d:0e47`). A guard arrives when
//! it shares its base's sector and depth layer while under way.

use super::YearMovement;
use super::starbase::merge_fits;
use crate::CoreGameData;
use nc_data::{Mission, MissionEvent, MissionOutcome, MissionRetargetEvent, Order, Real48};

/// 1 inside a world's system (depth 0.5 or more), else 0.
fn layer(depth: Real48) -> u8 {
    u8::from(depth.round() == 1)
}

impl YearMovement<'_> {
    /// Guard fleet `fleet_idx`'s dispatch after its step this tick.
    pub(super) fn dispatch_guard(
        &mut self,
        game_data: &mut CoreGameData,
        fleet_idx: usize,
        tick: u8,
    ) {
        let fleet = &game_data.fleets[fleet_idx];
        let owner = fleet.owner.0;
        let base_id = fleet.mission_target;
        let sector = fleet.sector.to_array();
        let Some(base_idx) = game_data.bases.iter().position(|base| {
            base.base_id.0 != 0 && base.owner.0 == owner && base.base_id.0 == base_id
        }) else {
            // The base is gone: the guard holds where it is (`0b5d:0c17`).
            let fleet = &mut game_data.fleets[fleet_idx];
            let previous_target_coords = fleet.target.to_array();
            fleet.order = Order::HoldPosition;
            fleet.speed = 0;
            self.events
                .guard_retargets
                .push(MissionRetargetEvent::Abandoned {
                    fleet_idx,
                    reporting_fleet_number: Some(fleet.fleet_number.0 as u8),
                    owner_empire: owner,
                    mission: Mission::GuardStarbase,
                    previous_target_coords,
                    coords: sector,
                });
            return;
        };
        let base = &game_data.bases[base_idx];
        let fleet_layer = layer(fleet.depth);
        let base_layer = layer(base.depth);
        if sector == base.sector.to_array()
            && fleet_layer == 1
            && base_layer == 0
            && fleet.speed == 0
        {
            // At rest in orbit under a base in deep space: its target is
            // the orbit it sits in, so it would never move. It climbs to
            // the base and arrives there. Classic leaves it in orbit (t040).
            let fleet = &mut game_data.fleets[fleet_idx];
            fleet.target_depth = base_layer;
            return;
        }
        if sector != base.sector.to_array() || fleet_layer != base_layer || fleet.speed == 0 {
            return;
        }
        self.tracks[fleet_idx].resolve(fleet);
        game_data.fleets[fleet_idx].speed = 0;

        // Another guard of the base already at rest here takes this one in.
        let partner = (0..game_data.fleets.len()).find(|&other| {
            let guard = &game_data.fleets[other];
            other != fleet_idx
                && self.in_play(game_data, other)
                && guard.owner.0 == owner
                && guard.sector.to_array() == sector
                && layer(guard.depth) == fleet_layer
                && guard.order == Order::GuardStarbase
                && guard.mission_target == base_id
                && guard.speed == 0
        });
        let mut event = MissionEvent {
            fleet_idx,
            owner_empire: owner,
            kind: Mission::GuardStarbase,
            outcome: MissionOutcome::Arrived,
            abort_reason: None,
            planet_idx: None,
            location_coords: Some(sector),
            target_coords: Some(sector),
            stardate_week: Some(tick),
        };
        match partner {
            Some(host) if merge_fits(game_data, host, fleet_idx) => {
                self.merge_guard(game_data, host, fleet_idx, tick, true);
                return;
            }
            Some(_) => {
                // The merge would pass the ship limit: the guard gives up.
                game_data.fleets[fleet_idx].order = Order::HoldPosition;
                event.outcome = MissionOutcome::Failed;
            }
            None => {
                // It becomes the base's escort and takes the base's depth.
                // Its target layer becomes the base's too, so it arrives
                // once. Classic keeps the target layer, and a guard whose
                // layer differs from the base's arrives again every tick
                // (t040).
                let base_depth = game_data.bases[base_idx].depth;
                let fleet = &mut game_data.fleets[fleet_idx];
                fleet.depth = base_depth;
                fleet.target_depth = base_layer;
                let slot = fleet.fleet_number.0;
                game_data.bases[base_idx].escort.0 = slot;
                self.tracks[fleet_idx].resolve(&game_data.fleets[fleet_idx]);
            }
        }
        self.events.mission_events.push(event);
    }
}
