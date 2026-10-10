use crate::CoreGameData;

/// A merge must not lose ships: refuse it when the scout word could not hold
/// both fleets' scouts. The fleets then stay separate.
pub(super) fn merge_fits_host(game_data: &CoreGameData, host_idx: usize, donor_idx: usize) -> bool {
    let host_scouts = game_data.fleets.records[host_idx].scout_count();
    let donor_scouts = game_data.fleets.records[donor_idx].scout_count();
    host_scouts.checked_add(donor_scouts).is_some()
}

pub(super) fn merge_one_fleet_into_host(
    game_data: &mut CoreGameData,
    host_idx: usize,
    donor_idx: usize,
) {
    let bb = game_data.fleets.records[donor_idx].battleship_count();
    let ca = game_data.fleets.records[donor_idx].cruiser_count();
    let dd = game_data.fleets.records[donor_idx].destroyer_count();
    let empty_tt = game_data.fleets.records[donor_idx].empty_transport_count();
    let loaded_tt = game_data.fleets.records[donor_idx].loaded_transport_count();
    let et = game_data.fleets.records[donor_idx].etac_count();
    let sc = game_data.fleets.records[donor_idx].scout_count();
    let strength_cache = game_data.fleets.records[donor_idx].strength_cache_raw();

    let donor_roe = game_data.fleets.records[donor_idx].rules_of_engagement();
    let donor_is_combat = game_data.fleets.records[donor_idx].has_any_combat_ships();
    let host_is_combat_before = game_data.fleets.records[host_idx].has_any_combat_ships();

    let host = &mut game_data.fleets.records[host_idx];
    host.set_battleship_count(host.battleship_count().saturating_add(bb));
    host.set_cruiser_count(host.cruiser_count().saturating_add(ca));
    host.set_destroyer_count(host.destroyer_count().saturating_add(dd));
    host.set_empty_transport_count(host.empty_transport_count().saturating_add(empty_tt));
    host.set_loaded_transport_count(host.loaded_transport_count().saturating_add(loaded_tt));
    host.set_etac_count(host.etac_count().saturating_add(et));
    host.set_scout_count(host.scout_count().saturating_add(sc));
    // ECMAINT's Join merge adds the 0x34 word too.
    host.set_strength_cache_raw(host.strength_cache_raw().wrapping_add(strength_cache));
    host.recompute_max_speed_from_composition();

    if !host_is_combat_before && donor_is_combat {
        // Support-only host absorbing a combat fleet assumes the combat ROE.
        host.set_rules_of_engagement(donor_roe);
    }
}
