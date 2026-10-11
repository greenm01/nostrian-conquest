use std::collections::BTreeMap;

use nc_compat::{DatabaseDat, merge_player_intel_from_compat};
use nc_data::{
    Coords, PlanetIntelEvent, PlanetIntelSource, Real48,
    build_player_starmap_projection_from_snapshots, latest_planet_intel_grants_for_viewer,
    merge_player_intel_from_runtime, set_armies_aboard, set_civil_disorder, set_planet_name,
    set_troop_transports,
};
use nc_engine::{GameStateBuilder, run_maintenance_turn, visible_hazard_intel_from_snapshots};

#[test]
fn sqlite_style_intel_persists_when_compat_database_row_disappears() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");
    game_data.planets[4].sector = Coords::new(4, 2);
    game_data.planets[4].owner.0 = 2;
    game_data.planets[4].years_under_owner = 2;

    let planet_count = game_data.planets.len();
    let mut observed_database =
        DatabaseDat::new_zeroed((game_data.conquest.player_count as usize) * planet_count);
    let record = observed_database.record_mut(4, 0, planet_count);
    record.set_planet_name("Prime");
    record.raw[0x15] = 2;
    record.raw[0x1c] = 100;
    record.raw[0x23] = 9;
    record.raw[0x25] = 3;

    let known = merge_player_intel_from_compat(&game_data, &observed_database, 1, 3000, None);
    let known_world = known.get(&5).expect("known world should be stored");
    assert_eq!(known_world.known_name.as_deref(), Some("Prime"));
    assert_eq!(known_world.known_owner_empire_id, Some(2));
    assert_eq!(known_world.known_potential_production, Some(100));
    assert_eq!(known_world.known_armies, Some(9));
    assert_eq!(known_world.known_ground_batteries, Some(3));
    assert_eq!(known_world.last_intel_year, Some(3000));

    let blank_database =
        DatabaseDat::new_zeroed((game_data.conquest.player_count as usize) * planet_count);
    let persisted =
        merge_player_intel_from_compat(&game_data, &blank_database, 1, 3001, Some(&known));
    let persisted_world = persisted
        .get(&5)
        .expect("persisted world should remain in sqlite intel");
    assert_eq!(persisted_world.known_name.as_deref(), Some("Prime"));
    assert_eq!(persisted_world.known_owner_empire_id, Some(2));
    assert_eq!(persisted_world.known_potential_production, Some(100));
    assert_eq!(persisted_world.known_armies, Some(9));
    assert_eq!(persisted_world.known_ground_batteries, Some(3));
    assert_eq!(persisted_world.last_intel_year, Some(3000));

    let projection = build_player_starmap_projection_from_snapshots(&game_data, &persisted, 1);
    let world = projection
        .worlds
        .iter()
        .find(|world| world.coords == [4, 2])
        .expect("projection world should exist");
    assert_eq!(world.known_name.as_deref(), Some("Prime"));
    assert_eq!(world.known_owner_empire_id, Some(2));

    let hazards = visible_hazard_intel_from_snapshots(&game_data, &persisted, 1);
    assert!(hazards.foreign_worlds.contains(&[4, 2]));
    assert!(hazards.hostile_homeworlds.contains(&[4, 2]));
}

#[test]
fn runtime_projection_keeps_owned_worlds_visible_without_snapshot_rows() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");
    game_data.planets[0].sector = Coords::new(5, 2);
    game_data.planets[0].owner.0 = 1;
    set_planet_name(&mut game_data.planets[0], "Home");
    game_data
        .set_guard_starbase(1, 1, [5, 2], 1, 1)
        .expect("guard starbase should seed");

    let projection =
        build_player_starmap_projection_from_snapshots(&game_data, &BTreeMap::new(), 1);
    let home = projection
        .worlds
        .iter()
        .find(|world| world.coords == [5, 2])
        .expect("owned world should exist");

    assert_eq!(home.known_owner_empire_id, Some(1));
    assert_eq!(home.known_name.as_deref(), Some("Home"));
    assert_eq!(
        home.known_potential_production,
        Some(u16::from(game_data.planets[0].potential_production))
    );
    assert_eq!(home.known_starbase_count, Some(1));
}

#[test]
fn generated_database_defaults_to_classic_unknown_sentinels() {
    let game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");
    let database = DatabaseDat::generate_from_planets_and_year(
        &game_data
            .planets
            .iter()
            .map(|planet| planet.name.clone())
            .collect::<Vec<_>>(),
        game_data.conquest.game_year,
        game_data.conquest.player_count as usize,
        None,
    );
    let record = database.record(0, 0, game_data.planets.len());
    assert_eq!(record.planet_name_bytes(), b"UNKNOWN");
    for offset in [
        0x15usize, 0x1c, 0x1d, 0x1e, 0x1f, 0x20, 0x23, 0x24, 0x25, 0x26,
    ] {
        assert_eq!(
            record.raw[offset], 0xff,
            "offset {offset:#x} should use unknown sentinel"
        );
    }
}

#[test]
fn runtime_intel_merges_scouted_worlds_without_compat_database() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3003)
        .build_initialized_baseline()
        .expect("baseline should build");
    game_data.planets[4].sector = Coords::new(9, 2);
    game_data.planets[4].owner.0 = 4;
    game_data.planets[4].years_under_owner = 2;
    set_planet_name(&mut game_data.planets[4], "Helios Prime");
    game_data.planets[4].potential_production = 136;
    game_data.planets[4].production = Real48::from_f64(136.0);
    game_data.planets[4].stored_goods = Real48::from_f64(35.0);
    game_data.planets[4].armies = 10;
    game_data.planets[4].ground_batteries = 6;
    game_data
        .set_guard_starbase(1, 4, [9, 2], 1, 1)
        .expect("guard starbase should seed");

    let grants = BTreeMap::from([(
        5usize,
        PlanetIntelEvent {
            planet_idx: 4,
            viewer_empire: 1,
            source: PlanetIntelSource::ScoutSolarSystem,
            source_fleet_idx: None,
            observed_snapshot: None,
            stardate_week: Some(22),
        },
    )]);
    let known =
        merge_player_intel_from_runtime(&game_data, 1, 3004, Some(&BTreeMap::new()), Some(&grants));
    let known_world = known.get(&5).expect("scouted world should be stored");

    assert_eq!(known_world.known_name.as_deref(), Some("Helios Prime"));
    assert_eq!(known_world.known_owner_empire_id, Some(4));
    assert_eq!(known_world.known_potential_production, Some(136));
    assert_eq!(known_world.known_current_production, Some(136));
    assert_eq!(known_world.known_stored_points, Some(35));
    assert_eq!(known_world.known_armies, Some(10));
    assert_eq!(known_world.known_ground_batteries, Some(6));
    assert_eq!(known_world.known_starbase_count, Some(1));
    assert_eq!(known_world.last_intel_year, Some(3003));
}

#[test]
fn blocked_colonization_intel_reveals_owner_and_potential() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3003)
        .build_initialized_baseline()
        .expect("baseline should build");
    game_data.planets[4].sector = Coords::new(9, 2);
    game_data.planets[4].owner.0 = 4;
    game_data.planets[4].years_under_owner = 2;
    set_planet_name(&mut game_data.planets[4], "Helios Prime");
    game_data.planets[4].potential_production = 136;

    let grants = BTreeMap::from([(
        5usize,
        PlanetIntelEvent {
            planet_idx: 4,
            viewer_empire: 1,
            source: PlanetIntelSource::ColonizeBlockedByOwner,
            source_fleet_idx: Some(3),
            observed_snapshot: None,
            stardate_week: Some(22),
        },
    )]);
    let known =
        merge_player_intel_from_runtime(&game_data, 1, 3004, Some(&BTreeMap::new()), Some(&grants));
    let known_world = known
        .get(&5)
        .expect("blocked-colonization world should be stored");

    assert_eq!(known_world.known_name.as_deref(), Some("Helios Prime"));
    assert_eq!(known_world.known_owner_empire_id, Some(4));
    assert_eq!(known_world.known_potential_production, Some(136));
    assert_eq!(known_world.known_armies, None);
    assert_eq!(known_world.known_ground_batteries, None);
    assert_eq!(known_world.last_intel_year, Some(3003));
}

#[test]
fn civil_disorder_contact_intel_only_reveals_owner() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3003)
        .build_initialized_baseline()
        .expect("baseline should build");
    set_civil_disorder(&mut game_data.players[1]);
    // The orders pass reads last year's count; this empire still owns a world.
    game_data.players[1].planet_count = 1;
    game_data.planets[4].sector = Coords::new(9, 2);
    game_data.planets[4].owner.0 = 2;
    game_data.planets[4].years_under_owner = 2;
    set_planet_name(&mut game_data.planets[4], "Not Named Yet");
    game_data.planets[4].potential_production = 100;

    let grants = BTreeMap::from([(
        5usize,
        PlanetIntelEvent {
            planet_idx: 4,
            viewer_empire: 1,
            source: PlanetIntelSource::CivilDisorderContact,
            source_fleet_idx: Some(3),
            observed_snapshot: None,
            stardate_week: Some(2),
        },
    )]);
    let known =
        merge_player_intel_from_runtime(&game_data, 1, 3004, Some(&BTreeMap::new()), Some(&grants));
    let known_world = known.get(&5).expect("ICD contact world should be stored");

    assert_eq!(known_world.known_owner_empire_id, Some(2));
    assert_eq!(known_world.known_name, None);
    assert_eq!(known_world.known_potential_production, None);
    assert_eq!(known_world.known_armies, None);
    assert_eq!(known_world.known_ground_batteries, None);
    assert_eq!(known_world.last_intel_year, Some(3003));
}

#[test]
fn identified_civil_disorder_contact_emits_owner_only_intel() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3003)
        .build_initialized_baseline()
        .expect("baseline should build");
    for fleet in &mut game_data.fleets {
        let coords = fleet.sector.to_array();
        fleet.speed = 0;
        fleet.order = nc_data::Order::HoldPosition;
        fleet.target = Coords::from_array(coords);
        fleet.ships.destroyers = 0;
        fleet.ships.cruisers = 0;
        fleet.ships.battleships = 0;
        fleet.ships.scouts = 0;
        set_troop_transports(fleet, 0);
        fleet.ships.etacs = 0;
        set_armies_aboard(fleet, 0);
        fleet.rules_of_engagement = 0;
    }

    set_civil_disorder(&mut game_data.players[1]);
    // The orders pass reads last year's count; this empire still owns a world.
    game_data.players[1].planet_count = 1;
    game_data.planets[4].sector = Coords::new(9, 2);
    game_data.planets[4].owner.0 = 2;
    game_data.planets[4].years_under_owner = 2;
    set_planet_name(&mut game_data.planets[4], "Not Named Yet");

    let scout = &mut game_data.fleets[0];
    scout.owner.0 = 1;
    scout.sector = Coords::new(9, 2);
    scout.order = nc_data::Order::ScoutSector;
    scout.target = Coords::new(9, 2);
    scout.ships.scouts = 1;

    let enemy = &mut game_data.fleets[1];
    enemy.owner.0 = 2;
    enemy.sector = Coords::new(9, 2);
    enemy.order = nc_data::Order::HoldPosition;
    enemy.ships.destroyers = 1;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");
    let grants = latest_planet_intel_grants_for_viewer(&events, 1);
    let known =
        merge_player_intel_from_runtime(&game_data, 1, 3004, Some(&BTreeMap::new()), Some(&grants));
    let known_world = known.get(&5).expect("ICD contact world should be stored");

    assert!(events.scout_contact_events.iter().any(|event| {
        event.viewer_empire == 1 && event.target_empire == 2 && event.coords == [9, 2]
    }));
    assert!(events.planet_intel_events.iter().any(|event| {
        event.viewer_empire == 1
            && event.planet_idx == 4
            && event.source == PlanetIntelSource::CivilDisorderContact
    }));
    assert_eq!(known_world.known_owner_empire_id, Some(2));
    assert_eq!(known_world.known_name, None);
    assert_eq!(known_world.known_potential_production, None);
}
