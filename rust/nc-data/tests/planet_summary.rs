use nc_data::{
    CompactUnitSummaryStyle, Coords, GameStateBuilder, OwnedPlanetStatus, ProductionItemKind,
    Real48, Starbase, format_build_queue_summary, format_owned_orbit_summary,
    format_stardock_summary, owned_orbit_presence, owned_planet_status,
};

#[test]
fn compact_planet_summaries_support_the_notation_and_words() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .build_initialized_baseline()
        .expect("baseline");
    let planet = &mut game_data.planets[0];
    planet.build_queue[0].kind = ProductionItemKind::Battleship;
    planet.build_queue[0].points_owed = 90;
    planet.stardock.cruisers = 5;

    assert_eq!(
        format_build_queue_summary(planet, CompactUnitSummaryStyle::Notation),
        "BB2"
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

    game_data.fleets[0].owner.0 = 1;
    game_data.fleets[0].sector = Coords::from_array(coords);
    game_data.fleets[0].ships.destroyers = 1;

    let mut base = Starbase {
        sector: Coords::from_array(coords),
        ..Starbase::default()
    };
    base.owner.0 = 1;
    base.base_id.0 = 1;
    game_data.bases.push(base);

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
    let planet = &mut game_data.planets[1];
    planet.owner.0 = 1;
    planet.years_under_owner = 1;
    planet.potential_production = 10;
    planet.production = Real48::ZERO;
    assert_eq!(
        owned_planet_status(&game_data, 1, 1),
        OwnedPlanetStatus::FactoriesDestroyed
    );

    game_data.planets[1].scorch_earth = true;
    assert_eq!(
        owned_planet_status(&game_data, 1, 1),
        OwnedPlanetStatus::Scorched
    );
}
