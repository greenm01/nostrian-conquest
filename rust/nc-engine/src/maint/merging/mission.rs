use super::super::{FleetMergeEvent, JoinMissionHostEvent, Mission};
use super::helpers::{merge_fits_host, merge_one_fleet_into_host};
use crate::navigation::motion::target_depth;
use crate::{CoreGameData, Order, maint::FleetRemovalRemapInfo};
use nc_data::{Coords, has_any_force, troop_transports};

pub(super) fn process_mission_fleet_merging(
    game_data: &mut CoreGameData,
) -> Result<(Vec<FleetMergeEvent>, FleetRemovalRemapInfo), Box<dyn std::error::Error>> {
    let fleet_count = game_data.fleets.len();
    if fleet_count == 0 {
        return Ok((Vec::new(), FleetRemovalRemapInfo::default()));
    }

    let mut to_remove = vec![false; fleet_count];
    let mut merge_events = Vec::new();

    // ECMAINT finds the host by owner and fleet number (the 0x00 word).
    let fleet_lookup: std::collections::HashMap<(u8, u16), usize> = game_data
        .fleets
        .iter()
        .enumerate()
        .map(|(idx, fleet)| ((fleet.owner.0, fleet.fleet_number.0), idx))
        .collect();

    for joiner_idx in 0..fleet_count {
        if to_remove[joiner_idx] {
            continue;
        }
        let joiner = &game_data.fleets[joiner_idx];
        if joiner.order != Order::JoinAnotherFleet {
            continue;
        }
        let host_number = joiner.mission_target;
        let joiner_owner = joiner.owner.0;
        let joiner_fleet_id = joiner.record.0;
        let joiner_coords = joiner.sector.to_array();
        let Some(&host_idx) = fleet_lookup.get(&(joiner_owner, host_number)) else {
            continue;
        };
        // A host destroyed this year keeps its empty record until next
        // turn's cull; joining it would revive its number and its order.
        if host_idx == joiner_idx
            || to_remove[host_idx]
            || !has_any_force(&game_data.fleets[host_idx])
        {
            continue;
        }
        let same_coords = game_data.fleets[host_idx].sector.to_array() == joiner_coords;
        // The join resolves when the joiner settles at its target depth in
        // the host's sector, as ECMAINT's join arrival test requires.
        let joiner = &game_data.fleets[joiner_idx];
        let settled = joiner.depth == target_depth(joiner.target_depth);
        if !same_coords || !settled || !merge_fits_host(game_data, host_idx, joiner_idx) {
            continue;
        }

        merge_one_fleet_into_host(game_data, host_idx, joiner_idx);
        to_remove[joiner_idx] = true;
        merge_events.push(FleetMergeEvent {
            fleet_idx: joiner_idx,
            owner_empire: joiner_owner,
            kind: Mission::JoinAnotherFleet,
            host_fleet_id: game_data.fleets[host_idx].record.0,
            absorbed_fleet_id: joiner_fleet_id,
            host_fleet_number: game_data.fleets[host_idx].fleet_number.0 as u8,
            absorbed_fleet_number: game_data.fleets[joiner_idx].fleet_number.0 as u8,
            coords: joiner_coords,
            survivor_side: false,
            stardate_week: None,
        });
    }

    let mut rendezvous_groups: std::collections::BTreeMap<(u8, [u8; 2]), Vec<usize>> =
        std::collections::BTreeMap::new();
    for (idx, fleet) in game_data.fleets.iter().enumerate() {
        if to_remove[idx] || fleet.order != Order::RendezvousSector {
            continue;
        }
        let coords = fleet.sector.to_array();
        if fleet.target.to_array() != coords {
            continue;
        }
        rendezvous_groups
            .entry((fleet.owner.0, coords))
            .or_default()
            .push(idx);
    }

    for ((_owner, coords), mut group) in rendezvous_groups {
        if group.len() < 2 {
            continue;
        }
        group.sort_by_key(|idx| game_data.fleets[*idx].record.0);
        let survivor_idx = group[0];
        for &absorbed_idx in group.iter().skip(1) {
            if to_remove[absorbed_idx] || !merge_fits_host(game_data, survivor_idx, absorbed_idx) {
                continue;
            }
            let absorbed_id = game_data.fleets[absorbed_idx].record.0;
            let owner_empire = game_data.fleets[absorbed_idx].owner.0;
            merge_one_fleet_into_host(game_data, survivor_idx, absorbed_idx);
            to_remove[absorbed_idx] = true;
            merge_events.push(FleetMergeEvent {
                fleet_idx: absorbed_idx,
                owner_empire,
                kind: Mission::RendezvousSector,
                host_fleet_id: game_data.fleets[survivor_idx].record.0,
                absorbed_fleet_id: absorbed_id,
                host_fleet_number: game_data.fleets[survivor_idx].fleet_number.0 as u8,
                absorbed_fleet_number: game_data.fleets[absorbed_idx].fleet_number.0 as u8,
                coords,
                survivor_side: false,
                stardate_week: None,
            });
            merge_events.push(FleetMergeEvent {
                fleet_idx: survivor_idx,
                owner_empire,
                kind: Mission::RendezvousSector,
                host_fleet_id: game_data.fleets[survivor_idx].record.0,
                absorbed_fleet_id: absorbed_id,
                host_fleet_number: game_data.fleets[survivor_idx].fleet_number.0 as u8,
                absorbed_fleet_number: game_data.fleets[absorbed_idx].fleet_number.0 as u8,
                coords,
                survivor_side: true,
                stardate_week: None,
            });
        }
    }

    let remap_info = if to_remove.iter().any(|remove| *remove) {
        super::super::apply_fleet_removal_remap(game_data, &to_remove)
    } else {
        FleetRemovalRemapInfo::default()
    };

    Ok((merge_events, remap_info))
}

/// Retarget or abandon join missions after one fleet-record compaction.
///
/// Join hosts are named by (owner, fleet number); fleet numbers survive a
/// compaction, so a joiner whose host was removed has had its host number
/// zeroed by the remap, and `remap_info` remembers what it was. Merge events
/// are keyed by the absorbed fleet's number, which ECMAINT's merge
/// (`2895:4a6b`) uses to point the absorbed fleet's joiners at the host.
pub(super) fn process_join_host_updates(
    game_data: &mut CoreGameData,
    merge_events: &[FleetMergeEvent],
    destroyed_join_host_fleet_numbers: &std::collections::HashMap<u16, u8>,
    remap_info: &FleetRemovalRemapInfo,
) -> Vec<JoinMissionHostEvent> {
    // (owner, absorbed fleet number) -> the merge that absorbed it.
    let mut absorbed_by = std::collections::HashMap::new();
    for event in merge_events {
        if event.absorbed_fleet_id != 0 && event.absorbed_fleet_id != event.host_fleet_id {
            absorbed_by.insert(
                (event.owner_empire, u16::from(event.absorbed_fleet_number)),
                event,
            );
        }
    }

    let current_host_viability: std::collections::HashMap<(u8, u16), bool> = game_data
        .fleets
        .iter()
        .map(|fleet| {
            let viable = fleet.ships.destroyers > 0
                || fleet.ships.cruisers > 0
                || fleet.ships.battleships > 0
                || fleet.ships.scouts > 0
                || troop_transports(fleet) > 0
                || fleet.ships.etacs > 0;
            ((fleet.owner.0, fleet.fleet_number.0), viable)
        })
        .collect();
    let current_fleet_coords: std::collections::HashMap<(u8, u16), [u8; 2]> = game_data
        .fleets
        .iter()
        .map(|fleet| {
            (
                (fleet.owner.0, fleet.fleet_number.0),
                fleet.sector.to_array(),
            )
        })
        .collect();

    let mut events = Vec::new();
    for (fleet_idx, fleet) in game_data.fleets.iter_mut().enumerate() {
        if fleet.order != Order::JoinAnotherFleet {
            continue;
        }

        let owner = fleet.owner.0;
        let host_number = fleet.mission_target;
        if host_number == fleet.fleet_number.0 {
            continue;
        }

        // A zero host number means the host fleet was removed (culled at
        // turn start or destroyed in a prior step) and the remap zeroed the
        // reference. Follow it into an absorbing host, else abandon.
        if host_number == 0 {
            let absorbing_merge = remap_info
                .prior_join_host_numbers
                .get(&fleet.record.0)
                .and_then(|prior_host_number| absorbed_by.get(&(owner, *prior_host_number)));
            if let Some(merge) = absorbing_merge {
                let new_host_number = u16::from(merge.host_fleet_number);
                if let Some(coords) = current_fleet_coords.get(&(owner, new_host_number)) {
                    fleet.mission_target = new_host_number;
                    fleet.target = Coords::from_array(*coords);
                    events.push(JoinMissionHostEvent::Retargeted {
                        fleet_idx,
                        owner_empire: owner,
                        previous_host_fleet_number: Some(merge.absorbed_fleet_number),
                        new_host_fleet_number: Some(merge.host_fleet_number),
                        coords: fleet.sector.to_array(),
                    });
                    continue;
                }
            }
            let coords = fleet.sector.to_array();
            fleet.order = Order::HoldPosition;
            fleet.speed = 0;
            fleet.target = Coords::from_array(coords);
            // join_host_fleet_id is already 0 — no need to set it again.
            events.push(JoinMissionHostEvent::HostDestroyed {
                fleet_idx,
                owner_empire: owner,
                destroyed_host_fleet_number: destroyed_join_host_fleet_numbers
                    .get(&fleet.record.0)
                    .copied(),
                coords,
            });
            continue;
        }

        // A non-zero host number survived the compaction, so it was not absorbed.
        let host_viable = current_host_viability.get(&(owner, host_number)).copied();
        if host_viable != Some(true) {
            let coords = fleet.sector.to_array();
            fleet.order = Order::HoldPosition;
            fleet.speed = 0;
            fleet.target = Coords::from_array(coords);
            fleet.mission_target = 0;
            events.push(JoinMissionHostEvent::HostDestroyed {
                fleet_idx,
                owner_empire: owner,
                destroyed_host_fleet_number: host_viable.is_some().then_some(host_number as u8),
                coords,
            });
        }
    }

    events
}
