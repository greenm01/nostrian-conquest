use super::super::{FleetMergeEvent, Mission};
use super::helpers::{merge_fits_host, merge_one_fleet_into_host};
use crate::{CoreGameData, Order, maint::FleetRemovalRemapInfo};
use nc_data::PlayerMode;

/// Merge co-located friendly fleets for players flagged for combat consolidation.
///
/// **Trigger:** only players whose `PLAYER.DAT raw[0x00] == 0xff` have their
/// fleets merged. This byte is a combat-engagement flag set by ECGAME when the
/// player has declared war or been flagged as a rogue aggressor. Values
/// `0x00`, `0x01`, `0x02`, etc. leave fleets untouched.
///
/// Confirmed by black-box oracle testing (econ/fleet-battle/invade fixtures):
/// - Setting player 1 raw[0x00] to `0x00` prevents the merge entirely.
/// - Only `0xff` triggers co-location merging.
///
/// **Merge rules (confirmed from econ-pre/post fixture pair):**
/// - All fleets belonging to the flagged player at the same coordinates are
///   merged into the lowest-indexed fleet at that location (the survivor).
/// - Ship counts are summed.
/// - Surviving fleet's ROE is set to 10 (maximum aggression).
/// - Surviving fleet's next_fleet_id (raw[0x03]) and prev_fleet_id (raw[0x07])
///   chain links are cleared to 0x00.
/// - Merged fleet records are deleted from the array.
/// - Fleet ID links and player first/last fleet IDs are remapped afterward.
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
        if game_data.players[player_idx].mode != PlayerMode::Rogue {
            continue;
        }

        let owner = (player_idx + 1) as u8;
        let player_fleet_indices: Vec<usize> = (0..fleet_count)
            .filter(|&i| game_data.fleets[i].owner.0 == owner)
            .collect();

        let mut coord_to_survivor: std::collections::HashMap<[u8; 2], usize> =
            std::collections::HashMap::new();

        for &fi in &player_fleet_indices {
            let coords = game_data.fleets[fi].sector.to_array();
            if let Some(&survivor_idx) = coord_to_survivor.get(&coords) {
                if !merge_fits_host(game_data, survivor_idx, fi) {
                    continue;
                }
                to_remove[fi] = true;

                let merging_order = game_data.fleets[fi].order;
                let merge_kind = match merging_order {
                    Order::JoinAnotherFleet => Some(Mission::JoinAnotherFleet),
                    Order::RendezvousSector => Some(Mission::RendezvousSector),
                    _ => None,
                };
                if let Some(kind) = merge_kind {
                    merge_events.push(FleetMergeEvent {
                        fleet_idx: fi,
                        owner_empire_raw: owner,
                        kind,
                        host_fleet_id_raw: game_data.fleets[survivor_idx].record.0,
                        absorbed_fleet_id_raw: game_data.fleets[fi].record.0,
                        host_fleet_number: game_data.fleets[survivor_idx].fleet_number.0 as u8,
                        absorbed_fleet_number: game_data.fleets[fi].fleet_number.0 as u8,
                        coords,
                        survivor_side: false,
                        stardate_week: None,
                    });
                    if kind == Mission::RendezvousSector {
                        merge_events.push(FleetMergeEvent {
                            fleet_idx: survivor_idx,
                            owner_empire_raw: owner,
                            kind,
                            host_fleet_id_raw: game_data.fleets[survivor_idx].record.0,
                            absorbed_fleet_id_raw: game_data.fleets[fi].record.0,
                            host_fleet_number: game_data.fleets[survivor_idx].fleet_number.0 as u8,
                            absorbed_fleet_number: game_data.fleets[fi].fleet_number.0 as u8,
                            coords,
                            survivor_side: true,
                            stardate_week: None,
                        });
                    }
                }

                merge_one_fleet_into_host(game_data, survivor_idx, fi);
            } else {
                coord_to_survivor.insert(coords, fi);
            }
        }

        for (&coords, &fi) in &coord_to_survivor {
            let had_merges = player_fleet_indices.iter().any(|&other| {
                to_remove[other] && game_data.fleets[other].sector.to_array() == coords
            });

            if had_merges {
                game_data.fleets[fi].next.0 = 0x0000;
                game_data.fleets[fi].prev.0 = 0x00;
                game_data.fleets[fi].rules_of_engagement = 10;
                *player_merged = true;
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
