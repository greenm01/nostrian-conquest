mod arrivals;
mod salvage;
mod stepper;

use super::{
    ColonizationEvent, FleetRemovalRemapInfo, MovementEvents, hostile_order_ready_for_execution,
};
use crate::{CoreGameData, Order, VisibleHazardIntel};
use arrivals::handle_fleet_arrival;
use nc_data::fleet_motion_state::{decode_exact_position, reset_motion_state_for_new_orders};
use salvage::queue_salvage_resolution;
use stepper::{process_single_fleet_movement, set_fleet_to_local_hold};

/// Process fleet movement for all fleets with active movement.
///
/// Based on docs/dev/archive/RE_NOTES.md section "Fleet Movement: Speed and Distance":
/// - Distance per turn = speed / 1.5 (approximately)
/// - Any order kind with speed > 0 and target ≠ current position triggers movement
/// - Fleet coordinates live in the classic fleet record layout at offsets 0x0B..0x0C (x, y)
///
/// Returns the movement side-effect events, indexed against the fleet list
/// before salvaged fleets are removed, plus the remap for that removal.
pub(super) fn process_fleet_movement(
    game_data: &mut CoreGameData,
    visible_hazards_by_empire: &[VisibleHazardIntel],
) -> Result<(MovementEvents, FleetRemovalRemapInfo), Box<dyn std::error::Error>> {
    let fleet_count = game_data.fleets.records.len();
    let mut movement_events = MovementEvents {
        fleet_travel_speeds: vec![0; fleet_count],
        ..MovementEvents::default()
    };
    let mut to_remove = vec![false; fleet_count];

    for i in 0..fleet_count {
        let (target_x, target_y, current_x, current_y, speed, order_kind, owner_empire, ready) = {
            let fleet = &game_data.fleets.records[i];
            (
                fleet.standing_order_target_coords_raw()[0],
                fleet.standing_order_target_coords_raw()[1],
                fleet.current_location_coords_raw()[0],
                fleet.current_location_coords_raw()[1],
                fleet.current_speed(),
                fleet.standing_order_kind(),
                fleet.owner_empire_raw(),
                fleet.transit_ready_flag_raw(),
            )
        };
        let has_unresolved_exact_transit = target_x == current_x
            && target_y == current_y
            && decode_exact_position(&game_data.fleets.records[i]).is_some();
        if is_stale_off_target_hostile(
            order_kind,
            speed,
            ready,
            [current_x, current_y],
            [target_x, target_y],
        ) {
            rearm_stale_off_target_hostile_fleet(&mut game_data.fleets.records[i]);
        }
        if ready == 0x80
            && matches!(
                order_kind,
                Order::BombardWorld | Order::InvadeWorld | Order::BlitzWorld
            )
            && !hostile_order_ready_for_execution(&game_data.fleets.records[i], order_kind)
            && (speed > 0 || has_unresolved_exact_transit)
        {
            clear_stale_hostile_ready_state(&mut game_data.fleets.records[i]);
        }
        // ColonizeWorld on-station: fleet already at target (or arrived with speed still set).
        // Queue colonization and reset to HoldPosition immediately; do not enter the
        // should_move gate which would skip the fleet with no side-effects.
        if matches!(order_kind, Order::ColonizeWorld)
            && target_x == current_x
            && target_y == current_y
        {
            set_fleet_to_local_hold(&mut game_data.fleets.records[i]);
            movement_events.colonization_events.push(ColonizationEvent {
                fleet_idx: i,
                coords: [target_x, target_y],
                owner_empire,
            });
            continue;
        }
        if matches!(order_kind, Order::Salvage) && target_x == current_x && target_y == current_y {
            let planet_idx = game_data
                .planets
                .records
                .iter()
                .position(|planet| planet.coords_raw() == [target_x, target_y]);
            queue_salvage_resolution(
                game_data,
                &mut movement_events,
                &mut to_remove,
                i,
                owner_empire,
                planet_idx,
                [target_x, target_y],
            )?;
            continue;
        }
        // A fleet moves when it has a non-HoldPosition order, speed > 0,
        // and either has not yet reached its visible target sector or is still
        // carrying unresolved exact in-transit state after rounding into it.
        // order_code 0x00 = HoldPosition — fleet stays put even if speed > 0
        // and target != current.
        // Note: BombardWorld/InvadeWorld fleets also move to their target before executing;
        // they are allowed here — arrival handling decides whether the order persists and
        // whether movement state stops or stays armed for the next phase.
        let order_code = game_data.fleets.records[i].standing_order_code_raw();
        let should_move = speed > 0
            && order_code != 0x00
            && ((target_x != current_x || target_y != current_y) || has_unresolved_exact_transit);

        if should_move {
            movement_events.fleet_travel_speeds[i] = speed;
            let arrived = process_single_fleet_movement(game_data, i, visible_hazards_by_empire)?;

            if arrived {
                handle_fleet_arrival(
                    game_data,
                    &mut movement_events,
                    &mut to_remove,
                    i,
                    order_kind,
                    owner_empire,
                    [target_x, target_y],
                )?;
            }
        }
    }

    let remap_info = if to_remove.iter().any(|remove| *remove) {
        super::apply_fleet_removal_remap(game_data, &to_remove)
    } else {
        FleetRemovalRemapInfo::default()
    };

    Ok((movement_events, remap_info))
}

pub(super) fn set_view_world_completion_hold(fleet: &mut nc_data::FleetRecord) {
    arrivals::set_view_world_completion_hold(fleet);
}

fn is_stale_off_target_hostile(
    order: Order,
    speed: u8,
    ready: u8,
    current: [u8; 2],
    target: [u8; 2],
) -> bool {
    matches!(
        order,
        Order::BombardWorld | Order::InvadeWorld | Order::BlitzWorld
    ) && speed == 0
        && ready == 0x80
        && current != target
}

fn rearm_stale_off_target_hostile_fleet(fleet: &mut nc_data::FleetRecord) {
    let speed = fleet.max_speed().max(1);
    reset_motion_state_for_new_orders(fleet);
    fleet.set_current_speed(speed);
    fleet.set_tuple_c_payload_raw([0x81, 0x00, 0x00, 0x00, 0x00]);
}

fn clear_stale_hostile_ready_state(fleet: &mut nc_data::FleetRecord) {
    fleet.set_transit_ready_flag_raw(0x00);
}
