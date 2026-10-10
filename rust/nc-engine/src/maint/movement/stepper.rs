use crate::navigation::{
    advance_exact_position, decode_movement_fraction, encode_movement_fraction,
    plan_route_with_intel, rounded_coords_from_exact, visible_hazard_intel_is_empty,
};
use crate::{CoreGameData, Order, VisibleHazardIntel};
use nc_data::fleet_motion_state::{
    clear_exact_position, decode_exact_position, reset_motion_state_for_new_orders,
    reset_motion_state_for_stationary_arrival, store_exact_position,
};

pub(super) fn set_fleet_to_local_hold(fleet: &mut nc_data::FleetRecord) {
    let coords = fleet.current_location_coords_raw();
    fleet.set_current_speed(0);
    fleet.set_standing_order_kind(Order::HoldPosition);
    fleet.set_standing_order_target_coords_raw(coords);
    reset_motion_state_for_new_orders(fleet);
}

fn order_persists_on_arrival(order: Order) -> bool {
    // Note: Scout missions (ScoutSector, ScoutSolarSystem) are intentionally
    // excluded here. Although the manual defines them as persistent, the engine
    // reverts them to HoldPosition during the arrival phase to "clean" the
    // fleet's context for combat resolution (avoiding forced System Entry
    // hostility). The order is restored at the end of maintenance in mod.rs;
    // this "Revert-then-Restore" cycle is what implements persistent stealth.
    matches!(
        order,
        Order::PatrolSector
            | Order::GuardStarbase
            | Order::GuardBlockadeWorld
            | Order::JoinAnotherFleet
            | Order::RendezvousSector
            | Order::BombardWorld
            | Order::InvadeWorld
            | Order::BlitzWorld
            | Order::ViewWorld
    )
}

fn order_stops_on_arrival(order: Order) -> bool {
    matches!(
        order,
        Order::PatrolSector | Order::GuardStarbase | Order::GuardBlockadeWorld | Order::ViewWorld
    )
}

fn order_keeps_speed_on_arrival(order: Order) -> bool {
    matches!(order, Order::JoinAnotherFleet)
}

fn apply_standing_arrival_state(fleet: &mut nc_data::FleetRecord, order: Order) {
    fleet.set_current_speed(0);

    match order {
        Order::PatrolSector | Order::GuardBlockadeWorld | Order::ViewWorld => {
            reset_motion_state_for_stationary_arrival(fleet);
            fleet.set_tuple_c_payload_raw([0x81, 0x00, 0x00, 0x00, 0x00]);
        }
        Order::GuardStarbase => {
            // Controlled classic probes converge on a distinct guarded-arrival payload here.
            // This is still treated as a compatibility shape rather than a decoded semantic
            // model for the remaining tuple-a bytes.
            fleet.set_extended_tuple_a_payload_raw([0x7b, 0x00, 0x84, 0xd8, 0x89, 0x1d]);
            clear_exact_position(fleet);
            let mut extended = fleet.extended_tuple_c_payload_raw();
            extended[0] = 0x00;
            fleet.set_extended_tuple_c_payload_raw(extended);
        }
        _ => {}
    }
}

/// Process movement for a single fleet using the ECMAINT movement formula.
///
/// Movement formula (confirmed from move-scenario fixture, speed=3, horizontal move):
/// - Uses a sub-grid of 9 sub-units per grid cell.
/// - Each turn: sub_acc += speed * 8; integer_move = sub_acc / 9; sub_acc %= 9.
/// - The fleet advances from its exact in-transit position toward its target
///   by integer_move movement units and only rounds when writing visible
///   sector coordinates.
/// - This is equivalent to distance_per_turn ≈ speed * 8/9.
///
/// The fractional accumulator is persisted in raw[0x0f] between turns.
/// Encoding (confirmed for speed=3): raw[0x0f] as i8 = (sub_acc - 9) * 2 / 3
/// (Generalised to: the sub_acc is always a multiple of 3 for speed=3 with denominator 9.)
///
/// When a fleet starts moving from rest (raw[0x0d] == 0x80):
/// - raw[0x0d] → 0x7f (transit tag byte)
/// - raw[0x0e] → 0xc0 (fixed constant during transit)
/// - raw[0x10..0x12] → [0xff, 0xff, 0x7f] (fixed constants during transit)
/// - raw[0x19] → 0x00 (clear departure flag)
///
/// On arrival (exact path reaches target):
/// - completion orders clear current_speed and fall back to HoldPosition
/// - standing guard/patrol orders keep their order but stop moving
/// - delayed hostile orders remain armed for the next ready-resolution tick
/// - hostile/one-shot arrivals still use the current tuple-c ready/completion stamp
///
/// Classic can round a fleet into the visible target sector before the hidden
/// in-transit path is actually exhausted. Completion is therefore keyed from the
/// exact path endpoint, not from the first rounded target-sector hit.
///
/// Confirmed from fleet-scenario fixture: fleet 0 ColonizeWorld, speed=3,
/// pos=(16,13) → (15,13) (arrived), all above changes observed.
/// Confirmed from move-scenario fixture: fleet 0 MoveOnly, speed=3,
/// pos=(16,13) → (24,13) after 3 turns, position and 0x0f encoding verified.
/// Confirmed from persistent mission probes: PatrolSector and GuardBlockadeWorld
/// keep their order but drop to speed=0 with a stationary tuple-c shape; GuardStarbase
/// also drops to speed=0 but still has partially unresolved arrival bytes.
///
/// Returns `true` if the fleet arrived at its target this turn.
pub(super) fn process_single_fleet_movement(
    game_data: &mut CoreGameData,
    fleet_idx: usize,
    visible_hazards_by_empire: &[VisibleHazardIntel],
) -> Result<bool, Box<dyn std::error::Error>> {
    let (
        current_x,
        current_y,
        target_x,
        target_y,
        speed,
        is_at_rest,
        raw_0f,
        owner_empire_raw,
        order,
    ) = {
        let fleet = &game_data.fleets.records[fleet_idx];
        (
            fleet.current_location_coords_raw()[0],
            fleet.current_location_coords_raw()[1],
            fleet.standing_order_target_coords_raw()[0],
            fleet.standing_order_target_coords_raw()[1],
            fleet.current_speed(),
            fleet.movement_state_flag_raw() == 0x80,
            fleet.movement_fraction_raw(),
            fleet.owner_empire_raw(),
            fleet.standing_order_kind(),
        )
    };

    if order == Order::GuardStarbase {
        // Classic clears the guard-starbase index byte on the first maintenance pass and
        // keeps later mission continuity keyed from the active base at the guarded target.
        game_data.fleets.records[fleet_idx].set_join_host_fleet_id_raw(0x00);
    }

    if speed == 0 {
        return Ok(false);
    }

    let exact_start = {
        let fleet = &game_data.fleets.records[fleet_idx];
        if is_at_rest {
            None
        } else {
            decode_exact_position(fleet)
        }
    };
    let dx_total = target_x as i32 - current_x as i32;
    let dy_total = target_y as i32 - current_y as i32;
    let has_unresolved_exact_transit = dx_total == 0 && dy_total == 0 && exact_start.is_some();

    if dx_total == 0 && dy_total == 0 && !has_unresolved_exact_transit {
        let fleet = &mut game_data.fleets.records[fleet_idx];
        set_fleet_to_local_hold(fleet);
        return Ok(true);
    }

    let sub_acc_prev: u32 = if is_at_rest {
        0
    } else {
        decode_movement_fraction(raw_0f)
    };

    let sub_acc_new = sub_acc_prev + (speed as u32) * 8;
    let sub_acc_after = sub_acc_new % 9;

    let int_move = (sub_acc_new / 9) as f64;
    let hazard_intel = visible_hazards_by_empire
        .get(owner_empire_raw.saturating_sub(1) as usize)
        .cloned()
        .unwrap_or_default();
    let exact_start = exact_start.unwrap_or([f64::from(current_x), f64::from(current_y)]);
    let use_route_geometry =
        !visible_hazard_intel_is_empty(&hazard_intel) && !has_unresolved_exact_transit;
    let route = if use_route_geometry {
        plan_route_with_intel(game_data, fleet_idx, &hazard_intel)
    } else {
        None
    };
    let exact_end = advance_exact_position(
        exact_start,
        [target_x, target_y],
        int_move,
        route.as_ref(),
        use_route_geometry,
    );
    let [new_x, new_y] = rounded_coords_from_exact(exact_end, [target_x, target_y]);

    game_data.fleets.records[fleet_idx].set_current_location_coords_raw([new_x, new_y]);

    // Classic completes the order in the year the fleet enters the target
    // sector, not when its exact path ends.
    if [new_x, new_y] == [target_x, target_y] {
        let arrival_order =
            Order::from_raw(game_data.fleets.records[fleet_idx].standing_order_code_raw());
        let preserves_order_on_arrival = order_persists_on_arrival(arrival_order);

        if !preserves_order_on_arrival {
            game_data.fleets.records[fleet_idx].set_current_speed(0);
            game_data.fleets.records[fleet_idx].set_standing_order_kind(Order::HoldPosition);
            game_data.fleets.records[fleet_idx]
                .set_extended_tuple_c_payload_raw([0x80, 0xb9, 0xff, 0xff, 0xff, 0x7f]);
        } else if order_stops_on_arrival(arrival_order) {
            apply_standing_arrival_state(&mut game_data.fleets.records[fleet_idx], arrival_order);
        } else {
            // Most persistent-on-arrival missions stay on-station once they reach the target
            // sector, but JoinAnotherFleet is a chase order: reaching the host's previous
            // position must not zero the fleet's speed or it will never continue pursuit after
            // retargeting on the next turn.
            if !order_keeps_speed_on_arrival(arrival_order) {
                game_data.fleets.records[fleet_idx].set_current_speed(0);
            }
            game_data.fleets.records[fleet_idx]
                .set_extended_tuple_c_payload_raw([0x80, 0xb9, 0xff, 0xff, 0xff, 0x7f]);
        }

        return Ok(true);
    }

    if is_at_rest {
        game_data.fleets.records[fleet_idx]
            .set_extended_tuple_a_payload_raw([0x7f, 0xc0, 0x00, 0xff, 0xff, 0x7f]);
        game_data.fleets.records[fleet_idx]
            .set_extended_tuple_c_payload_raw([0x00, 0x00, 0x00, 0x00, 0x00, 0x7f]);
    }

    store_exact_position(&mut game_data.fleets.records[fleet_idx], exact_end);

    game_data.fleets.records[fleet_idx]
        .set_movement_fraction_raw(encode_movement_fraction(sub_acc_after));

    Ok(false)
}
