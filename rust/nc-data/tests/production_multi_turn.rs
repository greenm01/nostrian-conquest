mod common;

use common::production::{
    commissioned_starbase, configured_conquest, joinable_single_player_game,
    owned_planet_with_present_production, player_with_empire_name, zeroed_setup,
};
use nc_data::{
    Coords, CoreGameData, PlayerMode, ProductionItemKind, present_production_points,
    stardock_slot_kind, stored_production_points,
};
use nc_engine::run_maintenance_turn;

fn multi_planet_game(
    player: nc_data::Player,
    planets: Vec<nc_data::Planet>,
    bases: Vec<nc_data::Starbase>,
) -> CoreGameData {
    CoreGameData {
        players: vec![player],
        planets,
        fleets: vec![],
        bases,
        ipbms: vec![],
        setup: zeroed_setup(),
        conquest: configured_conquest(1),
        sightings: Default::default(),
    }
}

fn run_turns(game: &mut CoreGameData, count: usize) {
    for _ in 0..count {
        run_maintenance_turn(game).expect("maintenance should succeed");
    }
}

#[test]
fn joined_homeworld_spending_carries_forward_across_turns() {
    let mut game = joinable_single_player_game();
    game.join_player(1, "Codex Dominion")
        .expect("join player should succeed");

    let homeworld_idx = game.players[0].homeworld as usize - 1;
    assert_eq!(stored_production_points(&game.planets[homeworld_idx]), 50);
    game.append_planet_build_order(homeworld_idx + 1, 50, 1)
        .expect("build order should queue");

    run_maintenance_turn(&mut game).expect("maintenance should succeed");
    let homeworld = &game.planets[homeworld_idx];
    assert_eq!(stored_production_points(homeworld), 50);
    assert_eq!(homeworld.build_queue[0].points_owed, 0);
    assert_eq!(homeworld.build_queue[0].kind.code(), 0);
    assert_eq!(
        stardock_slot_kind(homeworld, 0),
        ProductionItemKind::Destroyer
    );

    run_maintenance_turn(&mut game).expect("second maintenance should succeed");
    assert_eq!(stored_production_points(&game.planets[homeworld_idx]), 100);
}

#[test]
fn partial_build_consumes_only_yearly_spend_before_revenue_credit() {
    let mut player = player_with_empire_name("Alpha", 50, 0);
    player.mode = PlayerMode::Human;

    let mut planet = owned_planet_with_present_production(1, 100, 100, 200, 10, 4);
    planet.build_queue[0].points_owed = 150;
    planet.build_queue[0].kind = ProductionItemKind::Destroyer;

    let mut game = multi_planet_game(player, vec![planet], vec![]);
    run_maintenance_turn(&mut game).expect("maintenance should succeed");

    let planet = &game.planets[0];
    assert_eq!(planet.build_queue[0].points_owed, 50);
    assert_eq!(planet.build_queue[0].kind.code(), 1);
    assert_eq!(stored_production_points(planet), 150);
    assert_eq!(stardock_slot_kind(planet, 0).code(), 1);
    assert_eq!(planet.stardock.destroyers, 20);

    run_maintenance_turn(&mut game).expect("second maintenance should succeed");
    let planet = &game.planets[0];
    assert_eq!(planet.build_queue[0].points_owed, 0);
    assert_eq!(planet.build_queue[0].kind.code(), 0);
    assert_eq!(stored_production_points(planet), 150);
    assert_eq!(stardock_slot_kind(planet, 0).code(), 1);
    assert_eq!(planet.stardock.destroyers, 30);
}

#[test]
fn multiple_build_slots_share_one_planet_spend_budget_per_turn() {
    let mut player = player_with_empire_name("Alpha", 50, 0);
    player.mode = PlayerMode::Human;

    let mut planet = owned_planet_with_present_production(1, 100, 50, 200, 10, 4);
    planet.build_queue[0].points_owed = 25;
    planet.build_queue[0].kind = ProductionItemKind::Destroyer;
    planet.build_queue[1].points_owed = 40;
    planet.build_queue[1].kind = ProductionItemKind::Etac;

    let mut game = multi_planet_game(player, vec![planet], vec![]);
    run_maintenance_turn(&mut game).expect("maintenance should succeed");

    let planet = &game.planets[0];
    assert_eq!(planet.build_queue[0].points_owed, 0);
    assert_eq!(planet.build_queue[0].kind.code(), 0);
    assert_eq!(planet.build_queue[1].points_owed, 15);
    assert_eq!(planet.build_queue[1].kind.code(), 6);
    assert_eq!(stored_production_points(planet), 178);
    assert_eq!(stardock_slot_kind(planet, 0).code(), 1);
    assert_eq!(planet.stardock.destroyers, 5);
    assert_eq!(stardock_slot_kind(planet, 5).code(), 6);
    assert_eq!(planet.stardock.etacs, 1);
}

#[test]
fn capped_surface_build_keeps_full_stored_reserve() {
    let mut player = player_with_empire_name("Alpha", 50, 0);
    player.mode = PlayerMode::Human;

    let mut planet = owned_planet_with_present_production(1, 100, 100, 120, u8::MAX, 4);
    planet.build_queue[0].points_owed = 100;
    planet.build_queue[0].kind = ProductionItemKind::Army;

    let mut game = multi_planet_game(player, vec![planet], vec![]);
    run_maintenance_turn(&mut game).expect("maintenance should succeed");

    let planet = &game.planets[0];
    assert_eq!(planet.build_queue[0].points_owed, 100);
    assert_eq!(planet.build_queue[0].kind.code(), 8);
    assert_eq!(u8::try_from(planet.armies).unwrap_or(u8::MAX), u8::MAX);
    assert_eq!(stored_production_points(planet), 170);
}

#[test]
fn multi_turn_empire_economy_tracks_growth_and_revenue_across_owned_planets() {
    let mut player = player_with_empire_name("Alpha", 50, 0);
    player.mode = PlayerMode::Human;

    let homeworld = owned_planet_with_present_production(1, 100, 100, 0, 10, 4);
    let colony = owned_planet_with_present_production(1, 100, 25, 0, 1, 0);
    let mut game = multi_planet_game(player, vec![homeworld, colony], vec![]);

    // Empire Revenue forecasts the next pass: the colony grows 25 -> 34.375
    // and pays 17.19, so 50 + 17.
    assert_eq!(game.empire_available_production_points(1), 67);

    run_maintenance_turn(&mut game).expect("maintenance should succeed");
    assert_eq!(stored_production_points(&game.planets[0]), 50);
    assert_eq!(stored_production_points(&game.planets[1]), 17);
    assert_eq!(present_production_points(&game.planets[1]), 34);
    // Next: the colony grows 34.375 -> 42.578 and pays 21.29, so 50 + 21.
    assert_eq!(game.empire_available_production_points(1), 71);

    run_maintenance_turn(&mut game).expect("second maintenance should succeed");
    assert_eq!(stored_production_points(&game.planets[0]), 100);
    // 17.1875 + 21.2890625 = 38.4765625 carried as a fraction.
    assert_eq!(stored_production_points(&game.planets[1]), 38);
    assert_eq!(present_production_points(&game.planets[1]), 43);
    assert_eq!(game.empire_available_production_points(1), 75);
}

#[test]
fn commissioned_starbase_changes_multi_turn_growth_and_build_capacity_but_not_revenue() {
    let mut player = player_with_empire_name("Alpha", 50, 0);
    player.mode = PlayerMode::Human;

    let mut with_base_planet = owned_planet_with_present_production(1, 100, 50, 200, 3, 1);
    with_base_planet.sector = Coords::new(8, 8);
    with_base_planet.build_queue[0].points_owed = 200;
    with_base_planet.build_queue[0].kind = ProductionItemKind::Destroyer;

    let mut without_base_planet = with_base_planet.clone();
    without_base_planet.sector = Coords::new(9, 9);

    let mut with_base = multi_planet_game(
        player.clone(),
        vec![with_base_planet],
        vec![commissioned_starbase(1, [8, 8])],
    );
    let mut without_base = multi_planet_game(player, vec![without_base_planet], vec![]);

    run_turns(&mut with_base, 1);
    run_turns(&mut without_base, 1);

    let with_base_planet = &with_base.planets[0];
    let without_base_planet = &without_base.planets[0];

    // From 50 at 50% tax: 59.375 with the base and 56.25 without. Revenue is
    // the next pass's: 59.375 grows to 66.99, 56.25 to 61.72.
    assert_eq!(with_base.empire_available_production_points(1), 33);
    assert_eq!(without_base.empire_available_production_points(1), 31);
    assert_eq!(stored_production_points(with_base_planet), 29);
    assert_eq!(stored_production_points(without_base_planet), 178);
    assert_eq!(with_base_planet.build_queue[0].points_owed, 0);
    assert_eq!(without_base_planet.build_queue[0].points_owed, 150);
    assert_eq!(present_production_points(with_base_planet), 59);
    assert_eq!(present_production_points(without_base_planet), 56);
}

#[test]
fn commissioned_starbases_do_not_change_multi_turn_high_tax_penalty_behavior() {
    let mut player = player_with_empire_name("Alpha", 70, 0);
    player.mode = PlayerMode::Human;

    let mut with_base_planet = owned_planet_with_present_production(1, 100, 50, 0, 3, 1);
    with_base_planet.sector = Coords::new(12, 12);
    let without_base_planet = with_base_planet.clone();

    let mut with_base = multi_planet_game(
        player.clone(),
        vec![with_base_planet],
        vec![commissioned_starbase(1, [12, 12])],
    );
    let mut without_base = multi_planet_game(player, vec![without_base_planet], vec![]);

    run_turns(&mut with_base, 2);
    run_turns(&mut without_base, 2);

    assert_eq!(present_production_points(&with_base.planets[0]), 56);
    assert_eq!(
        Some(present_production_points(&with_base.planets[0])),
        Some(present_production_points(&without_base.planets[0]))
    );
    assert_eq!(
        with_base.empire_available_production_points(1),
        without_base.empire_available_production_points(1)
    );
}

#[test]
fn build_spending_is_capped_by_planet_treasury() {
    let mut player = player_with_empire_name("Alpha", 50, 0);
    player.mode = PlayerMode::Human;

    let mut broke = owned_planet_with_present_production(1, 100, 100, 0, 10, 4);
    broke.build_queue[0].points_owed = 100;
    broke.build_queue[0].kind = ProductionItemKind::Destroyer;
    let mut short = owned_planet_with_present_production(1, 100, 100, 30, 10, 4);
    short.sector = Coords::new(5, 5);
    short.build_queue[0].points_owed = 100;
    short.build_queue[0].kind = ProductionItemKind::Destroyer;

    let mut game = multi_planet_game(player, vec![broke, short], vec![]);
    run_maintenance_turn(&mut game).expect("maintenance should succeed");

    // Treasury 0: nothing is spent or delivered; revenue still lands.
    let broke = &game.planets[0];
    assert_eq!(broke.build_queue[0].points_owed, 100);
    assert_eq!(broke.build_queue[0].kind.code(), 1);
    assert_eq!(broke.stardock.destroyers, 0);
    assert_eq!(stored_production_points(broke), 50);

    // Treasury 30 < capacity 100: spend 30 and deliver 6 destroyers.
    let short = &game.planets[1];
    assert_eq!(short.build_queue[0].points_owed, 70);
    assert_eq!(stardock_slot_kind(short, 0).code(), 1);
    assert_eq!(short.stardock.destroyers, 6);
    assert_eq!(stored_production_points(short), 50);
}

#[test]
fn rogue_empire_planets_collect_revenue_and_grow() {
    let mut player = player_with_empire_name("Rogue", 50, 0);
    player.mode = PlayerMode::Rogue;

    let planet = owned_planet_with_present_production(1, 100, 50, 0, 200, 200);
    let mut game = multi_planet_game(player, vec![planet], vec![]);
    run_maintenance_turn(&mut game).expect("maintenance should succeed");

    let planet = &game.planets[0];
    assert_eq!(present_production_points(planet), 56);
    assert_eq!(stored_production_points(planet), 28);
}
