use super::super::{FleetMergeEvent, JoinMissionHostEvent, Mission};
use super::helpers::{merge_fits_host, merge_one_fleet_into_host};
use crate::navigation::motion::target_depth;
use crate::{CoreGameData, Order, maint::FleetRemovalRemapInfo};

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
        .map(|(idx, fleet)| ((fleet.owner_empire_raw(), fleet.local_slot_word_raw()), idx))
        .collect();

    for joiner_idx in 0..fleet_count {
        if to_remove[joiner_idx] {
            continue;
        }
        let joiner = &game_data.fleets[joiner_idx];
        if joiner.standing_order_kind() != Order::JoinAnotherFleet {
            continue;
        }
        let host_number = joiner.join_host_fleet_id_raw();
        let joiner_owner = joiner.owner_empire_raw();
        let joiner_fleet_id = joiner.fleet_id_word_raw();
        let joiner_coords = joiner.current_location_coords_raw();
        let Some(&host_idx) = fleet_lookup.get(&(joiner_owner, host_number)) else {
            continue;
        };
        if host_idx == joiner_idx || to_remove[host_idx] {
            continue;
        }
        let same_coords = game_data.fleets[host_idx].current_location_coords_raw() == joiner_coords;
        // The join resolves when the joiner settles at its target depth in
        // the host's sector, as ECMAINT's join arrival test requires.
        let joiner = &game_data.fleets[joiner_idx];
        let settled = joiner.depth() == target_depth(joiner.world_target_flag_raw());
        if !same_coords || !settled || !merge_fits_host(game_data, host_idx, joiner_idx) {
            continue;
        }

        merge_one_fleet_into_host(game_data, host_idx, joiner_idx);
        to_remove[joiner_idx] = true;
        merge_events.push(FleetMergeEvent {
            fleet_idx: joiner_idx,
            owner_empire_raw: joiner_owner,
            kind: Mission::JoinAnotherFleet,
            host_fleet_id_raw: game_data.fleets[host_idx].fleet_id_word_raw(),
            absorbed_fleet_id_raw: joiner_fleet_id,
            host_fleet_number: game_data.fleets[host_idx].local_slot_word_raw() as u8,
            absorbed_fleet_number: game_data.fleets[joiner_idx].local_slot_word_raw() as u8,
            coords: joiner_coords,
            survivor_side: false,
            stardate_week: None,
        });
    }

    let mut rendezvous_groups: std::collections::BTreeMap<(u8, [u8; 2]), Vec<usize>> =
        std::collections::BTreeMap::new();
    for (idx, fleet) in game_data.fleets.iter().enumerate() {
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
        group.sort_by_key(|idx| game_data.fleets[*idx].fleet_id_word_raw());
        let survivor_idx = group[0];
        for &absorbed_idx in group.iter().skip(1) {
            if to_remove[absorbed_idx] || !merge_fits_host(game_data, survivor_idx, absorbed_idx) {
                continue;
            }
            let absorbed_id = game_data.fleets[absorbed_idx].fleet_id_word_raw();
            let owner_empire_raw = game_data.fleets[absorbed_idx].owner_empire_raw();
            merge_one_fleet_into_host(game_data, survivor_idx, absorbed_idx);
            to_remove[absorbed_idx] = true;
            merge_events.push(FleetMergeEvent {
                fleet_idx: absorbed_idx,
                owner_empire_raw,
                kind: Mission::RendezvousSector,
                host_fleet_id_raw: game_data.fleets[survivor_idx].fleet_id_word_raw(),
                absorbed_fleet_id_raw: absorbed_id,
                host_fleet_number: game_data.fleets[survivor_idx].local_slot_word_raw() as u8,
                absorbed_fleet_number: game_data.fleets[absorbed_idx].local_slot_word_raw() as u8,
                coords,
                survivor_side: false,
                stardate_week: None,
            });
            merge_events.push(FleetMergeEvent {
                fleet_idx: survivor_idx,
                owner_empire_raw,
                kind: Mission::RendezvousSector,
                host_fleet_id_raw: game_data.fleets[survivor_idx].fleet_id_word_raw(),
                absorbed_fleet_id_raw: absorbed_id,
                host_fleet_number: game_data.fleets[survivor_idx].local_slot_word_raw() as u8,
                absorbed_fleet_number: game_data.fleets[absorbed_idx].local_slot_word_raw() as u8,
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
        if event.absorbed_fleet_id_raw != 0
            && event.absorbed_fleet_id_raw != event.host_fleet_id_raw
        {
            absorbed_by.insert(
                (
                    event.owner_empire_raw,
                    u16::from(event.absorbed_fleet_number),
                ),
                event,
            );
        }
    }

    let current_host_viability: std::collections::HashMap<(u8, u16), bool> = game_data
        .fleets
        .iter()
        .map(|fleet| {
            let viable = fleet.destroyer_count() > 0
                || fleet.cruiser_count() > 0
                || fleet.battleship_count() > 0
                || fleet.scout_count() > 0
                || fleet.troop_transport_count() > 0
                || fleet.etac_count() > 0;
            (
                (fleet.owner_empire_raw(), fleet.local_slot_word_raw()),
                viable,
            )
        })
        .collect();
    let current_fleet_coords: std::collections::HashMap<(u8, u16), [u8; 2]> = game_data
        .fleets
        .iter()
        .map(|fleet| {
            (
                (fleet.owner_empire_raw(), fleet.local_slot_word_raw()),
                fleet.current_location_coords_raw(),
            )
        })
        .collect();

    let mut events = Vec::new();
    for (fleet_idx, fleet) in game_data.fleets.iter_mut().enumerate() {
        if fleet.standing_order_kind() != Order::JoinAnotherFleet {
            continue;
        }

        let owner = fleet.owner_empire_raw();
        let host_number = fleet.join_host_fleet_id_raw();
        if host_number == fleet.local_slot_word_raw() {
            continue;
        }

        // A zero host number means the host fleet was removed (culled at
        // turn start or destroyed in a prior step) and the remap zeroed the
        // reference. Follow it into an absorbing host, else abandon.
        if host_number == 0 {
            let absorbing_merge = remap_info
                .prior_join_host_numbers
                .get(&fleet.fleet_id_word_raw())
                .and_then(|prior_host_number| absorbed_by.get(&(owner, *prior_host_number)));
            if let Some(merge) = absorbing_merge {
                let new_host_number = u16::from(merge.host_fleet_number);
                if let Some(coords) = current_fleet_coords.get(&(owner, new_host_number)) {
                    fleet.set_join_host_fleet_id_raw(new_host_number);
                    fleet.set_standing_order_target_coords_raw(*coords);
                    events.push(JoinMissionHostEvent::Retargeted {
                        fleet_idx,
                        owner_empire_raw: owner,
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
                owner_empire_raw: owner,
                destroyed_host_fleet_number: destroyed_join_host_fleet_numbers
                    .get(&fleet.fleet_id_word_raw())
                    .copied(),
                coords,
            });
            continue;
        }

        // A non-zero host number survived the compaction, so it was not absorbed.
        let host_viable = current_host_viability.get(&(owner, host_number)).copied();
        if host_viable != Some(true) {
            let coords = fleet.current_location_coords_raw();
            fleet.set_standing_order_kind(Order::HoldPosition);
            fleet.set_current_speed(0);
            fleet.set_standing_order_target_coords_raw(coords);
            fleet.set_join_host_fleet_id_raw(0);
            events.push(JoinMissionHostEvent::HostDestroyed {
                fleet_idx,
                owner_empire_raw: owner,
                destroyed_host_fleet_number: host_viable.is_some().then_some(host_number as u8),
                coords,
            });
        }
    }

    events
}
