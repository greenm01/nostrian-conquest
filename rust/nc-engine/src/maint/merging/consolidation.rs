use super::super::{FleetMergeEvent, Mission};
use super::helpers::{merge_fits, merge_one_fleet_into_host};
use crate::{CoreGameData, Order, maint::FleetRemovalRemapInfo};
use nc_data::{Fleet, PlayerMode, Real48};

/// A fleet that may host: a rogue empire's whatever its order and speed, an
/// autopilot empire's only on Hold, Guard/Blockade, or Guard Starbase at
/// rest. A Guard Starbase fleet hosts only when its base rests at its target.
fn host_may_merge(game_data: &CoreGameData, fleet: &Fleet, rogue: bool) -> bool {
    if !rogue
        && (fleet.speed != 0
            || !matches!(
                fleet.order,
                Order::HoldPosition | Order::GuardBlockadeWorld | Order::GuardStarbase
            ))
    {
        return false;
    }
    // A missing base never inherits another fleet's eligibility. Classic
    // left this test in a stale local; the owner rejected that bug (t099).
    fleet.order != Order::GuardStarbase || guarded_base_rests(game_data, fleet).unwrap_or(false)
}

/// Whether the base a Guard Starbase fleet names sits at its target, in the
/// layer the target names; `None` when the empire has no such base.
fn guarded_base_rests(game_data: &CoreGameData, fleet: &Fleet) -> Option<bool> {
    let base = game_data.bases.iter().find(|base| {
        base.owner == fleet.owner && base.base_id.0 != 0 && base.base_id.0 == fleet.mission_target
    })?;
    Some(base.sector == base.target && base.depth.round() == i32::from(base.target_depth))
}

/// A fleet that may be taken in: a rogue empire's whatever its order and
/// speed, an autopilot empire's only on Hold or Guard/Blockade at rest. A
/// Guard Starbase fleet on autopilot is never taken in.
fn donor_may_merge(fleet: &Fleet, rogue: bool) -> bool {
    rogue
        || (fleet.speed == 0
            && matches!(fleet.order, Order::HoldPosition | Order::GuardBlockadeWorld))
}

/// ECMAINT's merge pass (`24a3:216f`) for the empires it plays: rogue
/// empires and human empires on autopilot. It runs in the orders pass,
/// before the empire's fleet orders (rules in `docs/dev/ec-movement-spec.md`,
/// "Rogue And Autopilot Orders"):
///
/// - An empire with no world (PLAYER `0x50`) merges nothing.
/// - For each world of the empire, in planet order, the fleets in orbit of it
///   (same sector, depth 0.5 or more) are scanned in fleet order. Each one
///   that may host takes in, in fleet order, every other fleet there that
///   may be taken in and leaves the result at 3000 ships or fewer
///   (`2895:3057`). A fleet taken in is gone for the rest of the pass.
/// - Fleets elsewhere never merge, whoever the empire is.
pub(super) fn process_fleet_merging(
    game_data: &mut CoreGameData,
) -> Result<(Vec<FleetMergeEvent>, FleetRemovalRemapInfo), Box<dyn std::error::Error>> {
    let fleet_count = game_data.fleets.len();
    if fleet_count == 0 {
        return Ok((Vec::new(), FleetRemovalRemapInfo::default()));
    }

    let mut to_remove = vec![false; fleet_count];
    let mut merge_events = Vec::new();
    let mut players_with_merges = vec![false; game_data.players.len()];

    for (player_idx, player_merged) in players_with_merges.iter_mut().enumerate() {
        let player = &game_data.players[player_idx];
        if !player.played_by_ecmaint() || player.planet_count == 0 {
            continue;
        }
        let rogue = player.mode == PlayerMode::Rogue;
        let owner = (player_idx + 1) as u8;
        let worlds: Vec<[u8; 2]> = game_data
            .planets
            .iter()
            .filter(|planet| planet.owner.0 == owner)
            .map(|planet| planet.sector.to_array())
            .collect();

        for world in worlds {
            let in_orbit: Vec<usize> = (0..fleet_count)
                .filter(|&idx| {
                    let fleet = &game_data.fleets[idx];
                    fleet.owner.0 == owner
                        && fleet.sector.to_array() == world
                        && fleet.depth >= Real48::HALF
                })
                .collect();
            for &host in &in_orbit {
                if to_remove[host] || !host_may_merge(game_data, &game_data.fleets[host], rogue) {
                    continue;
                }
                for &donor in &in_orbit {
                    if donor == host
                        || to_remove[donor]
                        || !donor_may_merge(&game_data.fleets[donor], rogue)
                        || !merge_fits(game_data, host, donor)
                    {
                        continue;
                    }
                    record_merge(game_data, host, donor, world, &mut merge_events);
                    merge_one_fleet_into_host(game_data, host, donor);
                    to_remove[donor] = true;
                    *player_merged = true;
                }
            }
        }
    }
    let remap_info = super::super::apply_fleet_removal_remap(game_data, &to_remove);

    for (player_idx, had_merge) in players_with_merges.into_iter().enumerate() {
        if had_merge && game_data.players[player_idx].mode == PlayerMode::Rogue {
            game_data.players[player_idx].tax_rate = 0x41;
        }
    }

    Ok((merge_events, remap_info))
}

/// A merged-away Join or Rendezvous fleet is reported; any other is not.
fn record_merge(
    game_data: &CoreGameData,
    host: usize,
    donor: usize,
    coords: [u8; 2],
    merge_events: &mut Vec<FleetMergeEvent>,
) {
    let kind = match game_data.fleets[donor].order {
        Order::JoinAnotherFleet => Mission::JoinAnotherFleet,
        Order::RendezvousSector => Mission::RendezvousSector,
        _ => return,
    };
    let event = |fleet_idx, survivor_side| FleetMergeEvent {
        fleet_idx,
        owner_empire: game_data.fleets[donor].owner.0,
        kind,
        host_fleet_id: game_data.fleets[host].record.0,
        absorbed_fleet_id: game_data.fleets[donor].record.0,
        host_fleet_number: game_data.fleets[host].fleet_number.0 as u8,
        absorbed_fleet_number: game_data.fleets[donor].fleet_number.0 as u8,
        coords,
        survivor_side,
        stardate_week: None,
    };
    merge_events.push(event(donor, false));
    if kind == Mission::RendezvousSector {
        merge_events.push(event(host, true));
    }
}
