//! Defensive mode (owner, 2026-11-06): a world builds as many ground
//! batteries as its points buy, then armies with the rest, and takes no new
//! ship or starbase orders.

use nc_data::{CoreGameData, GameStateBuilder, GameStateMutationError, ProductionItemKind};
use nc_engine::run_maintenance_turn;

fn baseline() -> (CoreGameData, usize) {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");
    for player in &mut game_data.players {
        player.autopilot = false;
    }
    let home = game_data
        .planets
        .iter()
        .position(|planet| planet.owner.0 == 1)
        .expect("empire 1 has a home world")
        + 1;
    game_data
        .set_planet_defensive(home, true)
        .expect("home world exists");
    (game_data, home)
}

/// Batteries and armies the year's points buy: batteries first, then armies.
fn expected_purchase(game_data: &CoreGameData, record: usize) -> (u16, u16) {
    let points = game_data.planet_build_points_left(record).unwrap();
    let planet = &game_data.planets[record - 1];
    let batteries = (points / 20).min(255 - u32::from(planet.ground_batteries));
    let armies = ((points - batteries * 20) / 2).min(255 - u32::from(planet.armies));
    (batteries as u16, armies as u16)
}

#[test]
fn a_defensive_world_buys_batteries_then_armies_with_the_rest() {
    let (mut game_data, home) = baseline();
    let before = game_data.planets[home - 1].clone();
    let (batteries, armies) = expected_purchase(&game_data, home);
    assert!(batteries > 0, "the home world can afford a battery");

    run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let after = &game_data.planets[home - 1];
    assert_eq!(after.ground_batteries, before.ground_batteries + batteries);
    assert_eq!(after.armies, before.armies + armies);
}

#[test]
fn with_its_batteries_full_a_defensive_world_spends_everything_on_armies() {
    let (mut game_data, home) = baseline();
    game_data.planets[home - 1].ground_batteries = 255;
    game_data.planets[home - 1].armies = 0;
    let points = game_data.planet_build_points_left(home).unwrap();

    run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let after = &game_data.planets[home - 1];
    assert_eq!(after.ground_batteries, 255);
    assert_eq!(u32::from(after.armies), (points / 2).min(255));
}

#[test]
fn a_world_out_of_room_for_both_queues_nothing() {
    let (mut game_data, home) = baseline();
    game_data.planets[home - 1].ground_batteries = 255;
    game_data.planets[home - 1].armies = 255;

    run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert!(
        game_data.planets[home - 1]
            .build_queue
            .iter()
            .all(|slot| slot.points_owed == 0)
    );
}

#[test]
fn a_ship_queued_before_the_mode_keeps_its_claim_on_the_points() {
    let (mut game_data, home) = baseline();
    game_data.planets[home - 1].defensive = false;
    game_data
        .append_planet_build_order(home, 5, ProductionItemKind::Destroyer.code())
        .expect("a destroyer order");
    game_data.planets[home - 1].defensive = true;
    let docked = game_data.planets[home - 1].stardock.destroyers;
    let (batteries, _) = expected_purchase(&game_data, home);
    let before = game_data.planets[home - 1].ground_batteries;

    run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let after = &game_data.planets[home - 1];
    assert_eq!(after.stardock.destroyers, docked + 1);
    assert_eq!(after.ground_batteries, before + batteries);
}

#[test]
fn a_defensive_world_refuses_ship_and_starbase_orders() {
    let (mut game_data, home) = baseline();
    for kind in [ProductionItemKind::Destroyer, ProductionItemKind::Starbase] {
        let cost = kind.build_cost().unwrap();
        assert!(matches!(
            game_data.append_planet_build_order(home, cost, kind.code()),
            Err(GameStateMutationError::DefensiveWorldBuild { .. })
        ));
    }
    for kind in [ProductionItemKind::GroundBattery, ProductionItemKind::Army] {
        let cost = kind.build_cost().unwrap();
        game_data
            .append_planet_build_order(home, cost, kind.code())
            .expect("defenses are allowed");
    }
}
