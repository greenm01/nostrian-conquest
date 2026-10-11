use nc_compat::load_classic_core_files;
use nc_data::{
    Coords, CoreGameData, DiplomaticRelation, GameStateBuilder, InvalidPlayerStateEvent, Order,
    ProductionItemKind, Real48, set_armies_aboard, set_diplomatic_relation_toward,
    set_mission_target, set_troop_transports,
};
use nc_engine::{run_maintenance_turn, validate_maintenance_state};
use std::path::Path;

fn load_fixture(name: &str) -> CoreGameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
        .join("v1.5");
    load_classic_core_files(&dir)
        .unwrap_or_else(|e| panic!("Failed to load fixture {}: {}", name, e))
}

#[test]
fn invalid_bombard_order_is_canceled_before_execution() {
    let mut game_data = load_fixture("ecmaint-post");
    let owner_empire_raw = game_data.fleets[0].owner.0;
    for planet in &mut game_data.planets {
        planet.owner.0 = 0;
        planet.years_under_owner = 0;
    }
    let homeworld = &mut game_data.planets[0];
    homeworld.owner.0 = owner_empire_raw;
    homeworld.years_under_owner = 2;
    homeworld.sector = Coords::new(10, 10);
    let fleet = &mut game_data.fleets[0];
    fleet.sector = Coords::new(15, 13);
    fleet.order = Order::BombardWorld;
    fleet.target = Coords::new(15, 13);
    fleet.speed = 3;
    fleet.depth = Real48::HALF;
    fleet.ships.destroyers = 0;
    fleet.ships.cruisers = 0;
    fleet.ships.battleships = 0;
    fleet.ships.scouts = 1;
    set_troop_transports(fleet, 0);
    set_armies_aboard(fleet, 0);
    fleet.ships.etacs = 0;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert!(events.bombard_events.is_empty());
    assert!(events.invalid_player_state_events.iter().any(|event| {
        matches!(
            event,
            InvalidPlayerStateEvent::FleetMission {
                fleet_idx: 0,
                order_code_raw: 6,
                ..
            }
        )
    }));
    let fleet = &game_data.fleets[0];
    assert_eq!(fleet.order, Order::SeekHome);
    assert!(fleet.speed > 0);
    assert_eq!(fleet.target.to_array(), [10, 10]);
}

#[test]
fn invalid_colonize_order_without_etac_is_canceled() {
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let fleet = &mut game_data.fleets[0];
    fleet.ships.etacs = 0;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert!(events.colonization_events.is_empty());
    assert!(events.invalid_player_state_events.iter().any(|event| {
        matches!(
            event,
            InvalidPlayerStateEvent::FleetMission {
                fleet_idx: 0,
                order_code_raw: 12,
                ..
            }
        )
    }));
    // The fleet already sits in orbit at home, so Seek Home resolves on the
    // first tick and the fleet holds there.
    assert_eq!(game_data.fleets[0].order, Order::HoldPosition);
    assert_eq!(game_data.fleets[0].speed, 0);
}

#[test]
fn invalid_planet_build_input_is_cleared_before_processing() {
    let mut game_data = load_fixture("ecmaint-post");
    let planet_idx = game_data
        .planets
        .iter()
        .position(|planet| planet.owner.0 == 1)
        .expect("fixture should have owned planet");
    let planet = &mut game_data.planets[planet_idx];
    planet.build_queue[0].points_owed = 12;
    planet.build_queue[0].kind = ProductionItemKind::from_raw(0xfe);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert!(events.invalid_player_state_events.iter().any(|event| {
        matches!(
            event,
            InvalidPlayerStateEvent::PlanetInput { planet_idx: idx, .. } if *idx == planet_idx
        )
    }));
    let planet = &game_data.planets[planet_idx];
    assert_eq!(planet.build_queue[0].points_owed, 0);
    assert_eq!(planet.build_queue[0].kind.to_raw(), 0);
}

#[test]
fn invalid_tax_rate_is_clamped_before_economy_processing() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.players[0].tax_rate = 255;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert!(events.invalid_player_state_events.iter().any(|event| {
        matches!(
            event,
            InvalidPlayerStateEvent::PlayerTaxRate {
                player_idx: 0,
                tax_rate: 255,
                ..
            }
        )
    }));
    assert_eq!(game_data.players[0].tax_rate, 100);
}

#[test]
fn non_combat_fleet_roe_is_reset_to_zero() {
    let mut game_data = load_fixture("ecmaint-post");
    let fleet = &mut game_data.fleets[0];
    fleet.ships.destroyers = 0;
    fleet.ships.cruisers = 0;
    fleet.ships.battleships = 0;
    fleet.ships.scouts = 1;
    set_troop_transports(fleet, 0);
    set_armies_aboard(fleet, 0);
    fleet.ships.etacs = 0;
    fleet.rules_of_engagement = 6;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert!(events.invalid_player_state_events.iter().any(|event| {
        matches!(
            event,
            InvalidPlayerStateEvent::FleetInput { fleet_idx: 0, .. }
        )
    }));
    assert_eq!(game_data.fleets[0].rules_of_engagement, 0);
}

#[test]
fn mixed_fleet_roe_is_not_reset_to_zero() {
    let mut game_data = load_fixture("ecmaint-post");
    let fleet = &mut game_data.fleets[0];
    fleet.ships.destroyers = 1;
    fleet.ships.cruisers = 0;
    fleet.ships.battleships = 0;
    fleet.ships.scouts = 1;
    set_troop_transports(fleet, 0);
    set_armies_aboard(fleet, 0);
    fleet.ships.etacs = 0;
    fleet.rules_of_engagement = 6;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert!(!events.invalid_player_state_events.iter().any(|event| {
        matches!(
            event,
            InvalidPlayerStateEvent::FleetInput { fleet_idx: 0, .. }
        )
    }));
    assert_eq!(game_data.fleets[0].rules_of_engagement, 6);
}

#[test]
fn fleet_speed_is_clamped_to_current_maximum() {
    let mut game_data = load_fixture("ecmaint-post");
    let fleet = &mut game_data.fleets[0];
    fleet.speed = fleet.max_speed.saturating_add(3);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert!(events.invalid_player_state_events.iter().any(|event| {
        matches!(
            event,
            InvalidPlayerStateEvent::FleetInput { fleet_idx: 0, .. }
        )
    }));
    assert_eq!(game_data.fleets[0].speed, game_data.fleets[0].max_speed);
}

#[test]
fn invalid_diplomacy_bytes_are_reset_before_maintenance() {
    let mut game_data = load_fixture("ecmaint-post");
    set_diplomatic_relation_toward(&mut game_data.players[0], 1, DiplomaticRelation::Enemy);
    set_diplomatic_relation_toward(
        &mut game_data.players[0],
        2,
        DiplomaticRelation::from_raw(0xfe),
    );

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert!(events.invalid_player_state_events.iter().any(|event| {
        matches!(
            event,
            InvalidPlayerStateEvent::DiplomacyInput { player_idx: 0, .. }
        )
    }));
    assert_eq!(
        game_data.players[0].diplomacy[0],
        DiplomaticRelation::Neutral
    );
    assert_eq!(
        game_data.players[0].diplomacy[1],
        DiplomaticRelation::Neutral
    );
}

#[test]
fn maintenance_survives_deterministic_invalid_input_matrix() {
    for order_code in 16..=31 {
        let mut game_data = load_fixture("ecmaint-post");
        let fleet = &mut game_data.fleets[0];
        fleet.sector = Coords::new(15, 13);
        fleet.order = Order::from_raw(order_code);
        fleet.target = Coords::new(15, 13);
        fleet.speed = 3;
        set_mission_target(fleet, 0xfe, 0xfe);
        set_troop_transports(fleet, 1);
        set_armies_aboard(fleet, 4);
        fleet.rules_of_engagement = 42;
        game_data.planets[0].build_queue[0].points_owed = 9;
        game_data.planets[0].build_queue[0].kind = ProductionItemKind::from_raw(0xfe);
        game_data.planets[0].stardock.destroyers = 2;
        game_data.players[0].tax_rate = 255;

        let result = run_maintenance_turn(&mut game_data);
        assert!(
            result.is_ok(),
            "order code {order_code:#04x} should not panic"
        );
    }
}

#[test]
fn maintenance_rejects_structurally_invalid_state_before_simulation() {
    let mut game_data = load_fixture("ecmaint-post");
    let start_year = game_data.conquest.game_year;
    game_data.planets[0].owner.0 = 99;

    let preflight = validate_maintenance_state(&game_data)
        .expect_err("structural validation should reject an out-of-range owner");
    assert!(
        preflight
            .issues()
            .iter()
            .any(|issue| issue.contains("owner"))
    );

    let err = run_maintenance_turn(&mut game_data)
        .expect_err("maintenance should fail before mutating invalid state");
    assert!(
        err.to_string().contains("maintenance preflight failed"),
        "unexpected error: {err}"
    );
    assert_eq!(game_data.conquest.game_year, start_year);
}

#[test]
fn seek_home_picks_nearest_world_across_distant_coordinates() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");
    // Squared distances past i16 range (dx = 234 to the homeworld at (16,13)).
    let far_world = [200, 1];
    let planet = &mut game_data.planets[10];
    planet.sector = Coords::from_array(far_world);
    planet.owner.0 = 1;
    planet.years_under_owner = 2;
    let fleet = &mut game_data.fleets[0];
    fleet.sector = Coords::new(250, 1);
    fleet.order = Order::SeekHome;
    fleet.target = Coords::new(16, 13);
    fleet.speed = 3;

    run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(game_data.fleets[0].target.to_array(), far_world);
}
