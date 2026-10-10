//! Where a fleet goes when it leaves a battle.
//!
//! A fleet that lost the field, withdrew under its ROE, or lost the ships
//! its mission needs gets the Seek Home order as the order path writes it:
//! the target is the nearest owned world by ECGAME's travel-distance picker
//! ([`CoreGameData::nearest_owned_world`], measured from the fleet's own
//! position and depth), the world-target flag is 1, so the first leg climbs
//! out of the layer the battle was fought in and the last leg descends into
//! the home world's orbit, and the speed is the fleet's maximum after its
//! losses (`distribute_fleet_losses` recomputes it from the ships left).
//! ROE is left as it is. That is a mechanical matter, not a rule: classic
//! zeroes ROE only on scatter, and ec4x never does, so a retreat is not
//! where ROE changes. With no owned world the fleet holds where it is,
//! position and depth untouched. A fleet with no ships left holds at rest
//! and is culled when the next maintenance starts.

use crate::{CoreGameData, Order};

use super::state::TaskForce;

pub(crate) fn nearest_owned_planet(
    game_data: &CoreGameData,
    empire: u8,
    from: [u8; 2],
) -> Option<[u8; 2]> {
    game_data
        .planets
        .records
        .iter()
        .filter(|p| p.owner_empire_slot_raw() == empire)
        .min_by_key(|p| {
            let [x, y] = p.coords_raw();
            let dx = (x as i32 - from[0] as i32).unsigned_abs();
            let dy = (y as i32 - from[1] as i32).unsigned_abs();
            dx + dy
        })
        .map(|p| p.coords_raw())
}

pub(super) fn set_fleet_to_hold_current_position(fleet: &mut nc_data::FleetRecord) {
    let coords = fleet.current_location_coords_raw();
    fleet.set_current_speed(0);
    fleet.set_standing_order_kind(Order::HoldPosition);
    fleet.set_standing_order_target_coords_raw(coords);
    fleet.set_at_rest_motion();
}

/// Seek Home to `retreat_target` as the order path writes it: world target,
/// no mission id, full speed.
fn apply_retreat_order(fleet: &mut nc_data::FleetRecord, retreat_target: [u8; 2]) {
    fleet.set_standing_order_kind(Order::SeekHome);
    fleet.set_standing_order_target_coords_raw(retreat_target);
    fleet.set_mission_target(1, 0);
    fleet.set_current_speed(fleet.max_speed());
}

/// Hold where the fleet is, leaving its position and depth as they are.
fn hold_in_place(fleet: &mut nc_data::FleetRecord) {
    let coords = fleet.current_location_coords_raw();
    fleet.set_current_speed(0);
    fleet.set_standing_order_kind(Order::HoldPosition);
    fleet.set_standing_order_target_coords_raw(coords);
    fleet.set_mission_target(0, 0);
}

pub(crate) fn abort_mission_to_seek_home_or_hold(
    fleet: &mut nc_data::FleetRecord,
    retreat_target: Option<[u8; 2]>,
) {
    if let Some(retreat_target) = retreat_target {
        apply_retreat_order(fleet, retreat_target);
    } else {
        hold_in_place(fleet);
    }
}

/// Send one fleet home from the battle: Seek Home to its nearest owned
/// world, or hold in place when its empire owns none.
pub(super) fn retreat_fleet(game_data: &mut CoreGameData, fleet_idx: usize) {
    let retreat_target = game_data.nearest_owned_world(fleet_idx + 1, false);
    abort_mission_to_seek_home_or_hold(&mut game_data.fleets.records[fleet_idx], retreat_target);
}

/// Send a task force's fleets home after their losses are on the records:
/// each fleet with ships retreats on its own; one with none holds at rest
/// until it is culled.
pub(super) fn retreat_task_force(game_data: &mut CoreGameData, task_force: &TaskForce) {
    for &idx in &task_force.fleet_indices {
        let fleet = &mut game_data.fleets.records[idx];
        if fleet.destroyer_count() == 0
            && fleet.cruiser_count() == 0
            && fleet.battleship_count() == 0
            && fleet.scout_count() == 0
            && fleet.troop_transport_count() == 0
            && fleet.etac_count() == 0
        {
            set_fleet_to_hold_current_position(fleet);
            fleet.set_rules_of_engagement(0);
            continue;
        }

        retreat_fleet(game_data, idx);
    }
}

pub(super) fn dominant_empire_after_battle(
    task_forces: &[TaskForce],
    winner_empire: Option<u8>,
) -> Option<u8> {
    if winner_empire.is_some() {
        return winner_empire;
    }

    let mut surviving_empires = task_forces
        .iter()
        .filter(|tf| tf.engaged_in_battle && tf.state.has_units() && !tf.withdrew_under_roe)
        .map(|tf| tf.empire)
        .collect::<Vec<_>>();
    surviving_empires.sort_unstable();
    surviving_empires.dedup();
    if surviving_empires.len() == 1 {
        Some(surviving_empires[0])
    } else {
        None
    }
}
