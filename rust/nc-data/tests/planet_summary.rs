use std::collections::BTreeSet;

use nc_data::{
    BaseRecord, CompactUnitSummaryStyle, GameStateBuilder, OwnedPlanetStatus,
    format_build_queue_summary, format_owned_orbit_summary, format_stardock_summary,
    owned_orbit_presence, owned_planet_status,
};

#[test]
fn compact_planet_summaries_support_all_dashboard_and_game_styles() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .build_initialized_baseline()
        .expect("baseline");
    let planet = &mut game_data.planets.records[0];
    planet.set_build_kind_raw(0, 3);
    planet.set_build_count_raw(0, 90);
    planet.set_stardock_kind_raw(0, 2);
    planet.set_stardock_count_raw(0, 5);

    assert_eq!(
        format_build_queue_summary(planet, CompactUnitSummaryStyle::JoinedCodes),
        "2BB"
    );
    assert_eq!(
        format_build_queue_summary(planet, CompactUnitSummaryStyle::DashedCodes),
        "2-BB"
    );
    assert_eq!(
        format_stardock_summary(planet, CompactUnitSummaryStyle::Words),
        "5 cruisers"
    );
}

#[test]
fn orbit_presence_summary_counts_fleets_and_starbases() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .build_initialized_baseline()
        .expect("baseline");
    let coords = [1, 1];

    game_data.fleets.records[0].set_owner_empire_raw(1);
    game_data.fleets.records[0].set_current_location_coords_raw(coords);
    game_data.fleets.records[0].set_destroyer_count(1);

    let mut base = BaseRecord::new_zeroed();
    base.set_coords_raw(coords);
    base.set_owner_empire_raw(1);
    base.set_base_id_raw(1);
    game_data.bases.records.push(base);

    let summary = owned_orbit_presence(&game_data, 1, coords);
    assert_eq!(summary.fleets, 1);
    assert_eq!(summary.starbases, 1);
    assert_eq!(format_owned_orbit_summary(summary), "1 fleet, 1 starbase");
}

#[test]
fn owned_planet_status_distinguishes_damage_and_scorch() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .build_initialized_baseline()
        .expect("baseline");
    let planet = &mut game_data.planets.records[1];
    planet.set_owner_empire_slot_raw(1);
    planet.set_ownership_status_raw(1);
    planet.set_potential_production_raw([10, 0]);
    planet.set_present_production_points(0);
    assert_eq!(
        owned_planet_status(&game_data, 1, 1, &BTreeSet::new()),
        OwnedPlanetStatus::FactoriesDestroyed
    );

    let mut scorch = BTreeSet::new();
    scorch.insert(2);
    assert_eq!(
        owned_planet_status(&game_data, 1, 1, &scorch),
        OwnedPlanetStatus::Scorched
    );
}
