mod common;

use common::production::{
    commissioned_starbase, configured_conquest, joinable_single_player_game,
    owned_planet_with_present_production, player_with_empire_name, zeroed_setup,
};
use nc_data::{
    BaseDat, CoreGameData, FleetDat, IpbmDat, PlanetDat, PlayerDat, ProductionItemKind,
    yearly_tax_revenue,
};
use nc_engine::run_maintenance_turn;

fn multi_planet_game(
    player: nc_data::PlayerRecord,
    planets: Vec<nc_data::PlanetRecord>,
    bases: Vec<nc_data::BaseRecord>,
) -> CoreGameData {
    CoreGameData {
        player: PlayerDat {
            records: vec![player],
        },
        planets: PlanetDat { records: planets },
        fleets: FleetDat { records: vec![] },
        bases: BaseDat { records: bases },
        ipbm: IpbmDat { records: vec![] },
        setup: zeroed_setup(),
        conquest: configured_conquest(1),
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

    let homeworld_idx = game.player.records[0].homeworld_planet_index_1_based_raw() as usize - 1;
    assert_eq!(
        game.planets.records[homeworld_idx].stored_production_points(),
        50
    );
    game.append_planet_build_order(homeworld_idx + 1, 50, 1)
        .expect("build order should queue");

    run_maintenance_turn(&mut game).expect("maintenance should succeed");
    let homeworld = &game.planets.records[homeworld_idx];
    assert_eq!(homeworld.stored_production_points(), 50);
    assert_eq!(homeworld.build_count_raw(0), 0);
    assert_eq!(homeworld.build_kind_raw(0), 0);
    assert_eq!(
        homeworld.stardock_item_kind_current_known(0),
        ProductionItemKind::Destroyer
    );

    run_maintenance_turn(&mut game).expect("second maintenance should succeed");
    assert_eq!(
        game.planets.records[homeworld_idx].stored_production_points(),
        100
    );
}

#[test]
fn partial_build_consumes_only_yearly_spend_before_revenue_credit() {
    let mut player = player_with_empire_name("Alpha", 50, 0);
    player.set_player_mode_raw(0x01);

    let mut planet = owned_planet_with_present_production(1, 100, 100, 200, 10, 4);
    planet.set_build_count_raw(0, 150);
    planet.set_build_kind_raw(0, 1);

    let mut game = multi_planet_game(player, vec![planet], vec![]);
    run_maintenance_turn(&mut game).expect("maintenance should succeed");

    let planet = &game.planets.records[0];
    assert_eq!(planet.build_count_raw(0), 50);
    assert_eq!(planet.build_kind_raw(0), 1);
    assert_eq!(planet.stored_production_points(), 150);
    assert_eq!(planet.stardock_kind_raw(0), 1);
    assert_eq!(planet.stardock_count_raw(0), 20);

    run_maintenance_turn(&mut game).expect("second maintenance should succeed");
    let planet = &game.planets.records[0];
    assert_eq!(planet.build_count_raw(0), 0);
    assert_eq!(planet.build_kind_raw(0), 0);
    assert_eq!(planet.stored_production_points(), 150);
    assert_eq!(planet.stardock_kind_raw(0), 1);
    assert_eq!(planet.stardock_count_raw(0), 30);
}

#[test]
fn multiple_build_slots_share_one_planet_spend_budget_per_turn() {
    let mut player = player_with_empire_name("Alpha", 50, 0);
    player.set_player_mode_raw(0x01);

    let mut planet = owned_planet_with_present_production(1, 100, 50, 200, 10, 4);
    planet.set_build_count_raw(0, 25);
    planet.set_build_kind_raw(0, 1);
    planet.set_build_count_raw(1, 40);
    planet.set_build_kind_raw(1, 6);

    let mut game = multi_planet_game(player, vec![planet], vec![]);
    run_maintenance_turn(&mut game).expect("maintenance should succeed");

    let planet = &game.planets.records[0];
    assert_eq!(planet.build_count_raw(0), 0);
    assert_eq!(planet.build_kind_raw(0), 0);
    assert_eq!(planet.build_count_raw(1), 15);
    assert_eq!(planet.build_kind_raw(1), 6);
    assert_eq!(planet.stored_production_points(), 178);
    assert_eq!(planet.stardock_kind_raw(0), 1);
    assert_eq!(planet.stardock_count_raw(0), 5);
    assert_eq!(planet.stardock_kind_raw(5), 6);
    assert_eq!(planet.stardock_count_raw(5), 1);
}

#[test]
fn capped_surface_build_keeps_full_stored_reserve() {
    let mut player = player_with_empire_name("Alpha", 50, 0);
    player.set_player_mode_raw(0x01);

    let mut planet = owned_planet_with_present_production(1, 100, 100, 120, u8::MAX, 4);
    planet.set_build_count_raw(0, 100);
    planet.set_build_kind_raw(0, 8);

    let mut game = multi_planet_game(player, vec![planet], vec![]);
    run_maintenance_turn(&mut game).expect("maintenance should succeed");

    let planet = &game.planets.records[0];
    assert_eq!(planet.build_count_raw(0), 100);
    assert_eq!(planet.build_kind_raw(0), 8);
    assert_eq!(planet.army_count_raw(), u8::MAX);
    assert_eq!(planet.stored_production_points(), 170);
}

#[test]
fn multi_turn_empire_economy_tracks_growth_and_revenue_across_owned_planets() {
    let mut player = player_with_empire_name("Alpha", 50, 0);
    player.set_player_mode_raw(0x01);

    let homeworld = owned_planet_with_present_production(1, 100, 100, 0, 10, 4);
    let colony = owned_planet_with_present_production(1, 100, 25, 0, 1, 0);
    let mut game = multi_planet_game(player, vec![homeworld, colony], vec![]);

    assert_eq!(game.empire_available_production_points(1), 62);

    run_maintenance_turn(&mut game).expect("maintenance should succeed");
    assert_eq!(game.planets.records[0].stored_production_points(), 50);
    assert_eq!(game.planets.records[1].stored_production_points(), 17);
    assert_eq!(
        game.planets.records[1].present_production_points(),
        Some(35)
    );
    assert_eq!(
        game.empire_available_production_points(1),
        yearly_tax_revenue(100, 50) + yearly_tax_revenue(35, 50)
    );
    assert_eq!(game.empire_available_production_points(1), 67);

    run_maintenance_turn(&mut game).expect("second maintenance should succeed");
    assert_eq!(game.planets.records[0].stored_production_points(), 100);
    assert_eq!(game.planets.records[1].stored_production_points(), 39);
    assert_eq!(
        game.planets.records[1].present_production_points(),
        Some(44)
    );
    assert_eq!(
        game.empire_available_production_points(1),
        yearly_tax_revenue(100, 50) + yearly_tax_revenue(44, 50)
    );
    assert_eq!(game.empire_available_production_points(1), 72);
}

#[test]
fn commissioned_starbase_changes_multi_turn_growth_and_build_capacity_but_not_revenue() {
    let mut player = player_with_empire_name("Alpha", 50, 0);
    player.set_player_mode_raw(0x01);

    let mut with_base_planet = owned_planet_with_present_production(1, 100, 50, 200, 3, 1);
    with_base_planet.set_coords_raw([8, 8]);
    with_base_planet.set_build_count_raw(0, 200);
    with_base_planet.set_build_kind_raw(0, 1);

    let mut without_base_planet = with_base_planet.clone();
    without_base_planet.set_coords_raw([9, 9]);

    let mut with_base = multi_planet_game(
        player.clone(),
        vec![with_base_planet],
        vec![commissioned_starbase(1, [8, 8])],
    );
    let mut without_base = multi_planet_game(player, vec![without_base_planet], vec![]);

    run_turns(&mut with_base, 1);
    run_turns(&mut without_base, 1);

    let with_base_planet = &with_base.planets.records[0];
    let without_base_planet = &without_base.planets.records[0];

    assert_eq!(with_base.empire_available_production_points(1), 30);
    assert_eq!(without_base.empire_available_production_points(1), 28);
    assert_eq!(with_base_planet.stored_production_points(), 30);
    assert_eq!(without_base_planet.stored_production_points(), 178);
    assert_eq!(with_base_planet.build_count_raw(0), 0);
    assert_eq!(without_base_planet.build_count_raw(0), 150);
    assert_eq!(with_base_planet.present_production_points(), Some(61));
    assert_eq!(without_base_planet.present_production_points(), Some(57));
}

#[test]
fn commissioned_starbases_do_not_change_multi_turn_high_tax_penalty_behavior() {
    let mut player = player_with_empire_name("Alpha", 70, 0);
    player.set_player_mode_raw(0x01);

    let mut with_base_planet = owned_planet_with_present_production(1, 100, 50, 0, 3, 1);
    with_base_planet.set_coords_raw([12, 12]);
    let without_base_planet = with_base_planet.clone();

    let mut with_base = multi_planet_game(
        player.clone(),
        vec![with_base_planet],
        vec![commissioned_starbase(1, [12, 12])],
    );
    let mut without_base = multi_planet_game(player, vec![without_base_planet], vec![]);

    run_turns(&mut with_base, 2);
    run_turns(&mut without_base, 2);

    assert_eq!(
        with_base.planets.records[0].present_production_points(),
        Some(56)
    );
    assert_eq!(
        with_base.planets.records[0].present_production_points(),
        without_base.planets.records[0].present_production_points()
    );
    assert_eq!(
        with_base.empire_available_production_points(1),
        without_base.empire_available_production_points(1)
    );
}

#[test]
fn build_spending_is_capped_by_planet_treasury() {
    let mut player = player_with_empire_name("Alpha", 50, 0);
    player.set_player_mode_raw(0x01);

    let mut broke = owned_planet_with_present_production(1, 100, 100, 0, 10, 4);
    broke.set_build_count_raw(0, 100);
    broke.set_build_kind_raw(0, 1);
    let mut short = owned_planet_with_present_production(1, 100, 100, 30, 10, 4);
    short.set_coords_raw([5, 5]);
    short.set_build_count_raw(0, 100);
    short.set_build_kind_raw(0, 1);

    let mut game = multi_planet_game(player, vec![broke, short], vec![]);
    run_maintenance_turn(&mut game).expect("maintenance should succeed");

    // Treasury 0: nothing is spent or delivered; revenue still lands.
    let broke = &game.planets.records[0];
    assert_eq!(broke.build_count_raw(0), 100);
    assert_eq!(broke.build_kind_raw(0), 1);
    assert_eq!(broke.stardock_count_raw(0), 0);
    assert_eq!(broke.stored_production_points(), 50);

    // Treasury 30 < capacity 100: spend 30 and deliver 6 destroyers.
    let short = &game.planets.records[1];
    assert_eq!(short.build_count_raw(0), 70);
    assert_eq!(short.stardock_kind_raw(0), 1);
    assert_eq!(short.stardock_count_raw(0), 6);
    assert_eq!(short.stored_production_points(), 50);
}

#[test]
fn rogue_empire_planets_collect_revenue_and_grow() {
    let mut player = player_with_empire_name("Rogue", 50, 0);
    player.set_player_mode_raw(0xff);

    let planet = owned_planet_with_present_production(1, 100, 50, 0, 200, 200);
    let mut game = multi_planet_game(player, vec![planet], vec![]);
    run_maintenance_turn(&mut game).expect("maintenance should succeed");

    let planet = &game.planets.records[0];
    assert_eq!(planet.present_production_points(), Some(57));
    assert_eq!(planet.stored_production_points(), 28);
}
