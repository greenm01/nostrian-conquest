mod arrivals;
mod salvage;
mod stepper;

use super::{FleetRemovalRemapInfo, MovementEvents};
use crate::{CoreGameData, VisibleHazardIntel};
use arrivals::{handle_fleet_arrival, handle_hostile_target_entry};
use stepper::{YearEnd, is_hostile_order, process_single_fleet_movement};

/// Run one maintenance year of movement for every fleet.
///
/// Each fleet steps through ECMAINT's 52 ticks (`navigation::motion`) until
/// its order's arrival test passes. Resolved orders run their arrival
/// handling; hostile orders that pass their strike test are queued for the
/// planetary assault phase, which runs later in the year.
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
        let (target, order_kind, owner_empire) = {
            let fleet = &game_data.fleets.records[i];
            (
                fleet.standing_order_target_coords_raw(),
                fleet.standing_order_kind(),
                fleet.owner_empire_raw(),
            )
        };
        let movement = process_single_fleet_movement(game_data, i, visible_hazards_by_empire);
        movement_events.fleet_travel_speeds[i] = movement.travel_speed;
        if is_hostile_order(order_kind) && movement.entered_target {
            handle_hostile_target_entry(game_data, &mut movement_events, i, order_kind, target);
        }
        match movement.end {
            YearEnd::Underway => {}
            YearEnd::Strike => movement_events.hostile_strike_fleet_indices.push(i),
            YearEnd::Resolved => handle_fleet_arrival(
                game_data,
                &mut movement_events,
                &mut to_remove,
                i,
                order_kind,
                owner_empire,
                target,
            )?,
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
