use nc_data::{CoreGameData, GameStateBuilder, InvalidPlayerStateEvent, Order};
use nc_engine::{run_maintenance_turn, validate_maintenance_state};
use std::path::Path;

fn load_fixture(name: &str) -> CoreGameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
        .join("v1.5");
    CoreGameData::load(&dir).unwrap_or_else(|e| panic!("Failed to load fixture {}: {}", name, e))
}

#[test]
fn invalid_bombard_order_is_canceled_before_execution() {
    let mut game_data = load_fixture("ecmaint-post");
    let owner_empire_raw = game_data.fleets.records[0].owner_empire_raw();
    for planet in &mut game_data.planets.records {
        planet.set_owner_empire_slot_raw(0);
        planet.set_ownership_status_raw(0);
    }
    let homeworld = &mut game_data.planets.records[0];
    homeworld.set_owner_empire_slot_raw(owner_empire_raw);
    homeworld.set_ownership_status_raw(2);
    homeworld.set_coords_raw([10, 10]);
    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw([15, 13]);
    fleet.set_standing_order_kind(Order::BombardWorld);
    fleet.set_standing_order_target_coords_raw([15, 13]);
    fleet.set_current_speed(3);
    fleet.raw[0x19] = 0x80;
    fleet.set_destroyer_count(0);
    fleet.set_cruiser_count(0);
    fleet.set_battleship_count(0);
    fleet.set_scout_count(1);
    fleet.set_troop_transport_count(0);
    fleet.set_army_count(0);
    fleet.set_etac_count(0);

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
    let fleet = &game_data.fleets.records[0];
    assert_eq!(fleet.standing_order_kind(), Order::SeekHome);
    assert!(fleet.current_speed() > 0);
    assert_eq!(fleet.standing_order_target_coords_raw(), [10, 10]);
}

#[test]
fn invalid_colonize_order_without_etac_is_canceled() {
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let fleet = &mut game_data.fleets.records[0];
    fleet.set_etac_count(0);

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
    assert_eq!(
        game_data.fleets.records[0].standing_order_kind(),
        Order::HoldPosition
    );
    assert_eq!(game_data.fleets.records[0].current_speed(), 0);
}

#[test]
fn invalid_planet_build_input_is_cleared_before_processing() {
    let mut game_data = load_fixture("ecmaint-post");
    let planet_idx = game_data
        .planets
        .records
        .iter()
        .position(|planet| planet.owner_empire_slot_raw() == 1)
        .expect("fixture should have owned planet");
    let planet = &mut game_data.planets.records[planet_idx];
    planet.set_build_count_raw(0, 12);
    planet.set_build_kind_raw(0, 0xfe);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert!(events.invalid_player_state_events.iter().any(|event| {
        matches!(
            event,
            InvalidPlayerStateEvent::PlanetInput { planet_idx: idx, .. } if *idx == planet_idx
        )
    }));
    let planet = &game_data.planets.records[planet_idx];
    assert_eq!(planet.build_count_raw(0), 0);
    assert_eq!(planet.build_kind_raw(0), 0);
}

#[test]
fn invalid_tax_rate_is_clamped_before_economy_processing() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.player.records[0].set_tax_rate_raw(255);

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
    assert_eq!(game_data.player.records[0].tax_rate(), 100);
}

#[test]
fn non_combat_fleet_roe_is_reset_to_zero() {
    let mut game_data = load_fixture("ecmaint-post");
    let fleet = &mut game_data.fleets.records[0];
    fleet.set_destroyer_count(0);
    fleet.set_cruiser_count(0);
    fleet.set_battleship_count(0);
    fleet.set_scout_count(1);
    fleet.set_troop_transport_count(0);
    fleet.set_army_count(0);
    fleet.set_etac_count(0);
    fleet.set_rules_of_engagement(6);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert!(events.invalid_player_state_events.iter().any(|event| {
        matches!(
            event,
            InvalidPlayerStateEvent::FleetInput { fleet_idx: 0, .. }
        )
    }));
    assert_eq!(game_data.fleets.records[0].rules_of_engagement(), 0);
}

#[test]
fn mixed_fleet_roe_is_not_reset_to_zero() {
    let mut game_data = load_fixture("ecmaint-post");
    let fleet = &mut game_data.fleets.records[0];
    fleet.set_destroyer_count(1);
    fleet.set_cruiser_count(0);
    fleet.set_battleship_count(0);
    fleet.set_scout_count(1);
    fleet.set_troop_transport_count(0);
    fleet.set_army_count(0);
    fleet.set_etac_count(0);
    fleet.set_rules_of_engagement(6);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert!(!events.invalid_player_state_events.iter().any(|event| {
        matches!(
            event,
            InvalidPlayerStateEvent::FleetInput { fleet_idx: 0, .. }
        )
    }));
    assert_eq!(game_data.fleets.records[0].rules_of_engagement(), 6);
}

#[test]
fn fleet_speed_is_clamped_to_current_maximum() {
    let mut game_data = load_fixture("ecmaint-post");
    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_speed(fleet.max_speed().saturating_add(3));

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert!(events.invalid_player_state_events.iter().any(|event| {
        matches!(
            event,
            InvalidPlayerStateEvent::FleetInput { fleet_idx: 0, .. }
        )
    }));
    assert_eq!(
        game_data.fleets.records[0].current_speed(),
        game_data.fleets.records[0].max_speed()
    );
}

#[test]
fn invalid_diplomacy_bytes_are_reset_before_maintenance() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.player.records[0].raw[0x54] = 0x01;
    game_data.player.records[0].raw[0x55] = 0xfe;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert!(events.invalid_player_state_events.iter().any(|event| {
        matches!(
            event,
            InvalidPlayerStateEvent::DiplomacyInput { player_idx: 0, .. }
        )
    }));
    assert_eq!(game_data.player.records[0].raw[0x54], 0x00);
    assert_eq!(game_data.player.records[0].raw[0x55], 0x00);
}

#[test]
fn maintenance_survives_deterministic_invalid_input_matrix() {
    for order_code in 16..=31 {
        let mut game_data = load_fixture("ecmaint-post");
        let fleet = &mut game_data.fleets.records[0];
        fleet.set_current_location_coords_raw([15, 13]);
        fleet.set_standing_order_code_raw(order_code);
        fleet.set_standing_order_target_coords_raw([15, 13]);
        fleet.set_current_speed(3);
        fleet.set_mission_aux_bytes([0xfe, 0xfe]);
        fleet.set_troop_transport_count(1);
        fleet.set_army_count(4);
        fleet.set_rules_of_engagement(42);
        game_data.planets.records[0].set_build_count_raw(0, 9);
        game_data.planets.records[0].set_build_kind_raw(0, 0xfe);
        game_data.planets.records[0].set_stardock_count_raw(0, 2);
        game_data.player.records[0].set_tax_rate_raw(255);

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
    let start_year = game_data.conquest.game_year();
    game_data.planets.records[0].set_owner_empire_slot_raw(99);

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
    assert_eq!(game_data.conquest.game_year(), start_year);
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
    let planet = &mut game_data.planets.records[10];
    planet.set_coords_raw(far_world);
    planet.set_owner_empire_slot_raw(1);
    planet.set_ownership_status_raw(2);
    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw([250, 1]);
    fleet.set_standing_order_kind(Order::SeekHome);
    fleet.set_standing_order_target_coords_raw([16, 13]);
    fleet.set_current_speed(3);

    run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(
        game_data.fleets.records[0].standing_order_target_coords_raw(),
        far_world
    );
}
