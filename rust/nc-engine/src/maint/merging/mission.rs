use super::super::{FleetMergeEvent, JoinMissionHostEvent, Mission};
use super::helpers::{merge_fits_host, merge_one_fleet_into_host};
use crate::navigation::motion::target_depth;
use crate::{CoreGameData, Order, maint::FleetRemovalRemapInfo};

pub(super) fn process_mission_fleet_merging(
    game_data: &mut CoreGameData,
) -> Result<(Vec<FleetMergeEvent>, FleetRemovalRemapInfo), Box<dyn std::error::Error>> {
    let fleet_count = game_data.fleets.records.len();
    if fleet_count == 0 {
        return Ok((Vec::new(), FleetRemovalRemapInfo::default()));
    }

    let mut to_remove = vec![false; fleet_count];
    let mut merge_events = Vec::new();

    let fleet_lookup: std::collections::HashMap<u8, usize> = game_data
        .fleets
        .records
        .iter()
        .enumerate()
        .map(|(idx, fleet)| (fleet.fleet_id(), idx))
        .collect();

    for joiner_idx in 0..fleet_count {
        if to_remove[joiner_idx] {
            continue;
        }
        let joiner = &game_data.fleets.records[joiner_idx];
        if joiner.standing_order_kind() != Order::JoinAnotherFleet {
            continue;
        }
        let host_id = joiner.join_host_fleet_id_raw();
        let joiner_owner = joiner.owner_empire_raw();
        let joiner_fleet_id = joiner.fleet_id();
        let joiner_coords = joiner.current_location_coords_raw();
        let Some(&host_idx) = fleet_lookup.get(&host_id) else {
            continue;
        };
        if host_idx == joiner_idx || to_remove[host_idx] {
            continue;
        }
        let same_owner = game_data.fleets.records[host_idx].owner_empire_raw() == joiner_owner;
        let same_coords =
            game_data.fleets.records[host_idx].current_location_coords_raw() == joiner_coords;
        // The join resolves when the joiner settles at its target depth in
        // the host's sector, as ECMAINT's join arrival test requires.
        let joiner = &game_data.fleets.records[joiner_idx];
        let settled = joiner.depth() == target_depth(joiner.world_target_flag_raw());
        if !same_owner
            || !same_coords
            || !settled
            || !merge_fits_host(game_data, host_idx, joiner_idx)
        {
            continue;
        }

        merge_one_fleet_into_host(game_data, host_idx, joiner_idx);
        to_remove[joiner_idx] = true;
        merge_events.push(FleetMergeEvent {
            fleet_idx: joiner_idx,
            owner_empire_raw: joiner_owner,
            kind: Mission::JoinAnotherFleet,
            host_fleet_id_raw: game_data.fleets.records[host_idx].fleet_id(),
            absorbed_fleet_id_raw: joiner_fleet_id,
            host_fleet_number: game_data.fleets.records[host_idx].local_slot_word_raw() as u8,
            absorbed_fleet_number: game_data.fleets.records[joiner_idx].local_slot_word_raw() as u8,
            coords: joiner_coords,
            survivor_side: false,
            stardate_week: None,
        });
    }

    let mut rendezvous_groups: std::collections::BTreeMap<(u8, [u8; 2]), Vec<usize>> =
        std::collections::BTreeMap::new();
    for (idx, fleet) in game_data.fleets.records.iter().enumerate() {
        if to_remove[idx] || fleet.standing_order_kind() != Order::RendezvousSector {
            continue;
        }
        let coords = fleet.current_location_coords_raw();
        if fleet.standing_order_target_coords_raw() != coords {
            continue;
        }
        rendezvous_groups
            .entry((fleet.owner_empire_raw(), coords))
            .or_default()
            .push(idx);
    }

    for ((_owner, coords), mut group) in rendezvous_groups {
        if group.len() < 2 {
            continue;
        }
        group.sort_by_key(|idx| game_data.fleets.records[*idx].fleet_id());
        let survivor_idx = group[0];
        for &absorbed_idx in group.iter().skip(1) {
            if to_remove[absorbed_idx] || !merge_fits_host(game_data, survivor_idx, absorbed_idx) {
                continue;
            }
            let absorbed_id = game_data.fleets.records[absorbed_idx].fleet_id();
            let owner_empire_raw = game_data.fleets.records[absorbed_idx].owner_empire_raw();
            merge_one_fleet_into_host(game_data, survivor_idx, absorbed_idx);
            to_remove[absorbed_idx] = true;
            merge_events.push(FleetMergeEvent {
                fleet_idx: absorbed_idx,
                owner_empire_raw,
                kind: Mission::RendezvousSector,
                host_fleet_id_raw: game_data.fleets.records[survivor_idx].fleet_id(),
                absorbed_fleet_id_raw: absorbed_id,
                host_fleet_number: game_data.fleets.records[survivor_idx].local_slot_word_raw()
                    as u8,
                absorbed_fleet_number: game_data.fleets.records[absorbed_idx].local_slot_word_raw()
                    as u8,
                coords,
                survivor_side: false,
                stardate_week: None,
            });
            merge_events.push(FleetMergeEvent {
                fleet_idx: survivor_idx,
                owner_empire_raw,
                kind: Mission::RendezvousSector,
                host_fleet_id_raw: game_data.fleets.records[survivor_idx].fleet_id(),
                absorbed_fleet_id_raw: absorbed_id,
                host_fleet_number: game_data.fleets.records[survivor_idx].local_slot_word_raw()
                    as u8,
                absorbed_fleet_number: game_data.fleets.records[absorbed_idx].local_slot_word_raw()
                    as u8,
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
/// `merge_events` carry fleet ids from before that compaction, while the
/// fleets' join-host ids were already remapped by it; `remap_info` bridges
/// the two id spaces.
pub(super) fn process_join_host_updates(
    game_data: &mut CoreGameData,
    merge_events: &[FleetMergeEvent],
    destroyed_join_host_fleet_numbers: &std::collections::HashMap<u8, u8>,
    remap_info: &FleetRemovalRemapInfo,
) -> Vec<JoinMissionHostEvent> {
    // Pre-compaction id of each absorbed fleet -> the merge that absorbed it.
    let mut absorbed_by = std::collections::HashMap::new();
    for event in merge_events {
        if event.absorbed_fleet_id_raw != 0
            && event.absorbed_fleet_id_raw != event.host_fleet_id_raw
        {
            absorbed_by.insert(event.absorbed_fleet_id_raw, event);
        }
    }

    let current_fleet_ids: std::collections::HashSet<u8> = game_data
        .fleets
        .records
        .iter()
        .map(|fleet| fleet.fleet_id())
        .collect();
    let current_host_viability: std::collections::HashMap<u8, bool> = game_data
        .fleets
        .records
        .iter()
        .map(|fleet| {
            let viable = fleet.destroyer_count() > 0
                || fleet.cruiser_count() > 0
                || fleet.battleship_count() > 0
                || fleet.scout_count() > 0
                || fleet.troop_transport_count() > 0
                || fleet.etac_count() > 0;
            (fleet.fleet_id(), viable)
        })
        .collect();
    let current_fleet_coords: std::collections::HashMap<u8, [u8; 2]> = game_data
        .fleets
        .records
        .iter()
        .map(|fleet| (fleet.fleet_id(), fleet.current_location_coords_raw()))
        .collect();
    let current_fleet_numbers: std::collections::HashMap<u8, u8> = game_data
        .fleets
        .records
        .iter()
        .map(|fleet| (fleet.fleet_id(), fleet.local_slot_word_raw() as u8))
        .collect();

    let mut events = Vec::new();
    for (fleet_idx, fleet) in game_data.fleets.records.iter_mut().enumerate() {
        if fleet.standing_order_kind() != Order::JoinAnotherFleet {
            continue;
        }

        let host_id = fleet.join_host_fleet_id_raw();
        if host_id == fleet.fleet_id() {
            continue;
        }

        // host_id == 0 means the host fleet was removed (culled at turn start
        // or destroyed in a prior step) and the remap zeroed the reference.
        // Treat it the same as a missing/non-viable host — abandon the mission.
        if host_id == 0 {
            let absorbing_merge = remap_info
                .prior_join_host_ids
                .get(&fleet.fleet_id())
                .and_then(|prior_host_id| absorbed_by.get(prior_host_id));
            if let Some(merge) = absorbing_merge {
                if let Some(new_host_id) =
                    remap_info.fleet_indices.fleet_id(merge.host_fleet_id_raw)
                {
                    fleet.set_join_host_fleet_id_raw(new_host_id);
                    if let Some(coords) = current_fleet_coords.get(&new_host_id).copied() {
                        fleet.set_standing_order_target_coords_raw(coords);
                    }
                    events.push(JoinMissionHostEvent::Retargeted {
                        fleet_idx,
                        owner_empire_raw: fleet.owner_empire_raw(),
                        previous_host_fleet_number: Some(merge.absorbed_fleet_number),
                        new_host_fleet_number: Some(merge.host_fleet_number),
                        coords: fleet.current_location_coords_raw(),
                    });
                    continue;
                }
            }
            let coords = fleet.current_location_coords_raw();
            fleet.set_standing_order_kind(Order::HoldPosition);
            fleet.set_current_speed(0);
            fleet.set_standing_order_target_coords_raw(coords);
            // join_host_fleet_id_raw is already 0 — no need to set it again.
            events.push(JoinMissionHostEvent::HostDestroyed {
                fleet_idx,
                owner_empire_raw: fleet.owner_empire_raw(),
                destroyed_host_fleet_number: destroyed_join_host_fleet_numbers
                    .get(&fleet.fleet_id())
                    .copied(),
                coords,
            });
            continue;
        }

        // A non-zero host id survived the compaction, so it was not absorbed.
        let host_exists = current_fleet_ids.contains(&host_id);
        let host_viable = current_host_viability
            .get(&host_id)
            .copied()
            .unwrap_or(false);
        if !host_exists || !host_viable {
            let coords = fleet.current_location_coords_raw();
            fleet.set_standing_order_kind(Order::HoldPosition);
            fleet.set_current_speed(0);
            fleet.set_standing_order_target_coords_raw(coords);
            fleet.set_join_host_fleet_id_raw(0);
            events.push(JoinMissionHostEvent::HostDestroyed {
                fleet_idx,
                owner_empire_raw: fleet.owner_empire_raw(),
                destroyed_host_fleet_number: current_fleet_numbers.get(&host_id).copied(),
                coords,
            });
        }
    }

    events
}
