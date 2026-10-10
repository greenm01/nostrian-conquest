use crate::CoreGameData;

/// A merge must not lose ships: refuse it when the one-byte scout count
/// could not hold both fleets' scouts. The fleets then stay separate.
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
    let tt = game_data.fleets.records[donor_idx].troop_transport_count();
    let army = game_data.fleets.records[donor_idx].army_count();
    let et = game_data.fleets.records[donor_idx].etac_count();
    let sc = game_data.fleets.records[donor_idx].scout_count();

    let donor_roe = game_data.fleets.records[donor_idx].rules_of_engagement();
    let donor_is_combat = game_data.fleets.records[donor_idx].has_any_combat_ships();
    let host_is_combat_before = game_data.fleets.records[host_idx].has_any_combat_ships();

    let host = &mut game_data.fleets.records[host_idx];
    host.set_battleship_count(host.battleship_count().saturating_add(bb));
    host.set_cruiser_count(host.cruiser_count().saturating_add(ca));
    host.set_destroyer_count(host.destroyer_count().saturating_add(dd));
    host.set_troop_transport_count(host.troop_transport_count().saturating_add(tt));
    host.set_army_count(host.army_count().saturating_add(army));
    host.set_etac_count(host.etac_count().saturating_add(et));
    host.set_scout_count(host.scout_count().saturating_add(sc));
    host.recompute_max_speed_from_composition();

    if !host_is_combat_before && donor_is_combat {
        // Support-only host absorbing a combat fleet assumes the combat ROE.
        host.set_rules_of_engagement(donor_roe);
    }
}
