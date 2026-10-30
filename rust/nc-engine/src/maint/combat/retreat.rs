//! Where a fleet goes when it leaves a battle.
//!
//! A fleet that lost the field, withdrew under its ROE, or lost the ships
//! its mission needs gets the Seek Home order as the order path writes it:
//! the target is classic's nearest friendly world
//! ([`nearest_friendly_world`], measured from the fleet's own position and
//! depth, leaving out the world of the system the fleet is in), the
//! world-target flag is 1, so the first leg climbs out of the layer the
//! battle was fought in and the last leg descends into the home world's
//! orbit, and the speed is the fleet's maximum after its losses
//! (`distribute_fleet_losses` recomputes it from the ships left). ROE is
//! left as it is. That is a mechanical matter, not a rule: classic zeroes
//! ROE only on scatter, and ec4x never does, so a retreat is not where ROE
//! changes. With no other owned world the fleet holds where it is, position
//! and depth untouched. A fleet with no ships left holds at rest and is
//! culled when the next maintenance starts.
//!
//! A fleet that broke off is also escaping: it is in an escape episode from
//! the battle's sector and layer ([`nc_data::EscapeEpisode`]), which notes
//! the empire that caught it in a pursuit exchange, so that empire gets no
//! second one while the episode lasts.

use crate::{CoreGameData, Order};

use super::fleet_battle::mission_lost_its_ships;
use super::state::{Member, TaskForce};
use nc_data::{
    Coords, EscapeEpisode, Real48, has_any_force, set_fleet_at_rest, set_mission_target,
};

/// Where fleet `fleet_idx` goes when it abandons its mission: ECMAINT's
/// nearest friendly world (`24a3:0876`, flag 1), which is ECGAME's picker
/// ([`CoreGameData::nearest_owned_world`]) told to leave out the world of
/// the system the fleet is in. Every abort and retreat in ECMAINT asks it
/// this way. `None` when the empire owns no other world.
pub(crate) fn nearest_friendly_world(
    game_data: &CoreGameData,
    fleet_idx: usize,
) -> Option<[u8; 2]> {
    game_data.nearest_owned_world(fleet_idx + 1, true)
}

pub(super) fn set_fleet_to_hold_current_position(fleet: &mut nc_data::Fleet) {
    let coords = fleet.sector.to_array();
    fleet.speed = 0;
    fleet.order = Order::HoldPosition;
    fleet.target = Coords::from_array(coords);
    set_fleet_at_rest(fleet);
}

/// Seek Home to `retreat_target` as the order path writes it: world target,
/// no mission id, full speed.
fn apply_retreat_order(fleet: &mut nc_data::Fleet, retreat_target: [u8; 2]) {
    fleet.order = Order::SeekHome;
    fleet.target = Coords::from_array(retreat_target);
    set_mission_target(fleet, 1, 0);
    fleet.speed = fleet.max_speed;
}

/// Hold where the fleet is, leaving its position and depth as they are. The
/// target is its own sector and layer, `TZ = Round(Z)`, as in every Hold
/// ECMAINT leaves a fleet on.
pub(crate) fn hold_in_place(fleet: &mut nc_data::Fleet) {
    let coords = fleet.sector.to_array();
    fleet.speed = 0;
    fleet.order = Order::HoldPosition;
    fleet.target = Coords::from_array(coords);
    set_mission_target(fleet, u8::from(fleet.depth.round() == 1), 0);
}

/// Take fleet `fleet_idx` off its order for `retreat_target`: Seek Home
/// there, or hold in place. A Guard Starbase fleet lets go of its base
/// first, as before each of ECMAINT's aborts and retreats (`1b05:25ab`
/// scatter, `1b05:3151` retreat; [`CoreGameData::release_starbase_escort`]).
pub(crate) fn abort_mission_to_seek_home_or_hold(
    game_data: &mut CoreGameData,
    fleet_idx: usize,
    retreat_target: Option<[u8; 2]>,
) {
    game_data.release_starbase_escort(fleet_idx);
    let fleet = &mut game_data.fleets[fleet_idx];
    if let Some(retreat_target) = retreat_target {
        apply_retreat_order(fleet, retreat_target);
    } else {
        hold_in_place(fleet);
    }
}

/// Send one fleet home from the battle: Seek Home to its nearest friendly
/// world, or hold in place when its empire owns no other.
pub(crate) fn retreat_fleet(game_data: &mut CoreGameData, fleet_idx: usize) {
    let retreat_target = nearest_friendly_world(game_data, fleet_idx);
    abort_mission_to_seek_home_or_hold(game_data, fleet_idx, retreat_target);
}

/// Whether fleet `idx`, beaten or breaking off before it reached its target,
/// keeps its order and carries on (owner, 2026-10-30): it is still en route
/// on a moving order and has the ships its mission needs. A fleet beaten at
/// its own target goes home.
pub(super) fn resumes_mission(game_data: &CoreGameData, idx: usize) -> bool {
    let fleet = &game_data.fleets[idx];
    has_any_force(fleet)
        && fleet.sector != fleet.target
        && !matches!(
            fleet.order,
            Order::HoldPosition | Order::SeekHome | Order::Unknown(_)
        )
        && !mission_lost_its_ships(game_data, idx, fleet.order)
}

/// Keep a resuming fleet on its order, at the full speed of the ships it has
/// left, as a fleet that held the field carries on.
fn carry_on(fleet: &mut nc_data::Fleet) {
    fleet.speed = fleet.max_speed;
}

/// Send the fleets still on a side at the end home after their losses are
/// on the records: each fleet with ships retreats on its own, unless it is
/// in `resuming`; one with none holds at rest until it is culled. Fleets that
/// broke off have escaped already.
pub(super) fn retreat_task_force(
    game_data: &mut CoreGameData,
    task_force: &TaskForce,
    resuming: &std::collections::HashSet<usize>,
) {
    for idx in task_force
        .members
        .iter()
        .filter(|member| member.fighting())
        .filter_map(Member::fleet_idx)
    {
        if !has_any_force(&game_data.fleets[idx]) {
            hold_until_culled(&mut game_data.fleets[idx]);
            continue;
        }
        if resuming.contains(&idx) {
            carry_on(&mut game_data.fleets[idx]);
            continue;
        }
        retreat_fleet(game_data, idx);
    }
}

/// Send a fleet that broke off home from the battle at `coords` and record
/// its escape: with ships it retreats and is in an escape episode from
/// `coords` and its layer, continuing the one it was already in (a fleet's
/// episode is always from the cell it is in, since leaving the cell ends
/// it), with `pursuer` noted as having had its pursuit exchange. A fleet
/// with nowhere to go holds where it is and is in no episode; one with no
/// ships holds at rest until it is culled. A fleet that `resumes` keeps its
/// order and is in the escape episode all the same.
pub(super) fn escape_fleet(
    game_data: &mut CoreGameData,
    idx: usize,
    coords: [u8; 2],
    pursuer: Option<u8>,
    resumes: bool,
) {
    if !has_any_force(&game_data.fleets[idx]) {
        hold_until_culled(&mut game_data.fleets[idx]);
        return;
    }
    let goes_on = if resumes {
        carry_on(&mut game_data.fleets[idx]);
        true
    } else {
        let home = nearest_friendly_world(game_data, idx);
        abort_mission_to_seek_home_or_hold(game_data, idx, home);
        home.is_some()
    };
    let fleet = &mut game_data.fleets[idx];
    fleet.escape = goes_on.then(|| {
        let mut episode = fleet.escape.unwrap_or(EscapeEpisode {
            sector: Coords::from_array(coords),
            layer: u8::from(fleet.depth >= Real48::HALF),
            pursuers: 0,
        });
        if let Some(pursuer) = pursuer {
            episode.record_pursuit(pursuer);
        }
        episode
    });
}

fn hold_until_culled(fleet: &mut nc_data::Fleet) {
    set_fleet_to_hold_current_position(fleet);
    fleet.rules_of_engagement = 0;
    fleet.escape = None;
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
        .filter(|tf| tf.engaged_in_battle && tf.state.has_units())
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
