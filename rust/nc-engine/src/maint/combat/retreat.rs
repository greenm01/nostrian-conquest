use crate::{CoreGameData, Order};
use nc_data::fleet_motion_state::clear_exact_position;

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
    fleet.set_tuple_c_payload_raw([0x81, 0x00, 0x00, 0x00, 0x00]);
    clear_exact_position(fleet);
}

fn apply_retreat_order(fleet: &mut nc_data::FleetRecord, retreat_target: [u8; 2]) {
    fleet.set_standing_order_kind(Order::SeekHome);
    fleet.set_standing_order_target_coords_raw(retreat_target);
    fleet.set_current_speed(fleet.max_speed());
    fleet.set_extended_tuple_a_payload_raw([0x7f, 0xc0, 0x00, 0xff, 0xff, 0x7f]);
    fleet.set_extended_tuple_c_payload_raw([0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    fleet.set_mission_aux_bytes([0, 0]);
    fleet.set_rules_of_engagement(0);
}

pub(crate) fn abort_mission_to_seek_home_or_hold(
    fleet: &mut nc_data::FleetRecord,
    retreat_target: Option<[u8; 2]>,
) {
    if let Some(retreat_target) = retreat_target {
        apply_retreat_order(fleet, retreat_target);
    } else {
        set_fleet_to_hold_current_position(fleet);
        fleet.set_mission_aux_bytes([0, 0]);
    }
}

pub(super) fn retreat_task_force(game_data: &mut CoreGameData, task_force: &TaskForce) {
    let retreat_target = nearest_owned_planet(game_data, task_force.empire, task_force.coords)
        .unwrap_or(task_force.coords);

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

        apply_retreat_order(fleet, retreat_target);
    }
}

pub(super) fn apply_roe_retreat_to_task_force(
    game_data: &mut CoreGameData,
    fleet_indices: &[usize],
    retreat_target: [u8; 2],
) {
    for &idx in fleet_indices {
        let fleet = &mut game_data.fleets.records[idx];
        if fleet.destroyer_count() == 0
            && fleet.cruiser_count() == 0
            && fleet.battleship_count() == 0
            && fleet.scout_count() == 0
            && fleet.troop_transport_count() == 0
            && fleet.etac_count() == 0
        {
            continue;
        }
        apply_retreat_order(fleet, retreat_target);
    }
}

pub(super) fn clear_empty_withdrawn_fleets(game_data: &mut CoreGameData, fleet_indices: &[usize]) {
    for &idx in fleet_indices {
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
        }
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
