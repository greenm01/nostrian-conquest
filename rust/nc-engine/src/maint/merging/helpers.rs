use crate::CoreGameData;
use nc_data::{recompute_max_speed, starship_count};

/// A fleet cannot grow past this many ships by merging (`2895:3057`).
const MAX_MERGED_SHIPS: u32 = 3000;

/// Whether two fleets merged would stay within the ship limit.
pub(in crate::maint) fn merge_fits(game_data: &CoreGameData, host: usize, donor: usize) -> bool {
    let fleets = &game_data.fleets;
    starship_count(&fleets[host]) + starship_count(&fleets[donor]) <= MAX_MERGED_SHIPS
}

/// A merge must not lose ships: refuse it when the scout word could not hold
/// both fleets' scouts. The fleets then stay separate.
pub(super) fn merge_fits_host(game_data: &CoreGameData, host_idx: usize, donor_idx: usize) -> bool {
    let host_scouts = game_data.fleets[host_idx].ships.scouts;
    let donor_scouts = game_data.fleets[donor_idx].ships.scouts;
    host_scouts.checked_add(donor_scouts).is_some()
}

pub(super) fn merge_one_fleet_into_host(
    game_data: &mut CoreGameData,
    host_idx: usize,
    donor_idx: usize,
) {
    let bb = game_data.fleets[donor_idx].ships.battleships;
    let ca = game_data.fleets[donor_idx].ships.cruisers;
    let dd = game_data.fleets[donor_idx].ships.destroyers;
    let empty_tt = game_data.fleets[donor_idx].ships.empty_transports;
    let loaded_tt = game_data.fleets[donor_idx].ships.loaded_transports;
    let et = game_data.fleets[donor_idx].ships.etacs;
    let sc = game_data.fleets[donor_idx].ships.scouts;
    let donor_escape = game_data.fleets[donor_idx].escape;
    let donor_crippled = game_data.fleets[donor_idx].crippled;

    let host = &mut game_data.fleets[host_idx];
    host.ships.battleships = host.ships.battleships.saturating_add(bb);
    host.ships.cruisers = host.ships.cruisers.saturating_add(ca);
    host.ships.destroyers = host.ships.destroyers.saturating_add(dd);
    host.ships.empty_transports = host.ships.empty_transports.saturating_add(empty_tt);
    host.ships.loaded_transports = host.ships.loaded_transports.saturating_add(loaded_tt);
    host.ships.etacs = host.ships.etacs.saturating_add(et);
    host.ships.scouts = host.ships.scouts.saturating_add(sc);
    // Merging adds the crippled counts, this pass's included, and never
    // resets or speeds up a recovery.
    host.crippled.add(&donor_crippled);
    // 2895:4a6b leaves the host's ROE alone.
    recompute_max_speed(host);
    // A merge gives nobody a second pursuit: the host keeps every pursuer
    // either fleet's escape had recorded.
    if let Some(donor_escape) = donor_escape {
        match &mut host.escape {
            Some(escape) if escape.sector == donor_escape.sector => {
                escape.pursuers |= donor_escape.pursuers;
            }
            Some(_) => {}
            None => host.escape = Some(donor_escape),
        }
    }
}
