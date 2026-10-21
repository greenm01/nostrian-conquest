//! A world with no starbase may pay for one from its treasury outside its
//! build capacity (decision y95fnqkm, amended for t207).

mod common;

use common::production::{
    commissioned_starbase, configured_conquest, owned_planet_with_present_production,
    player_with_empire_name, zeroed_setup,
};
use nc_data::{
    Coords, CoreGameData, PlanetTurnAction, PlanetTurnBlock, PlayerMode, ProductionItemKind,
    Starbase, TurnSubmission, stored_production_points,
};
use nc_engine::run_maintenance_turn;

const WORLD: [u8; 2] = [8, 8];

/// Production 20 with 80 points stored, at no tax so the treasury holds still.
fn colony_game(bases: Vec<Starbase>) -> CoreGameData {
    let mut player = player_with_empire_name("Alpha", 0, 0);
    player.mode = PlayerMode::Human;
    let mut colony = owned_planet_with_present_production(1, 100, 20, 80, 3, 1);
    colony.sector = Coords::from_array(WORLD);
    CoreGameData {
        players: vec![player],
        planets: vec![colony],
        fleets: vec![],
        bases,
        ipbms: vec![],
        setup: zeroed_setup(),
        conquest: configured_conquest(1),
        sightings: Default::default(),
    }
}

fn order(game: &mut CoreGameData, kind: ProductionItemKind, points: u32) -> bool {
    let submission = TurnSubmission {
        player_record_index_1_based: 1,
        year: game.conquest.game_year,
        tax_rate: None,
        diplomacy: vec![],
        planets: vec![PlanetTurnBlock {
            planet_record_index_1_based: 1,
            actions: vec![PlanetTurnAction::Build {
                points_remaining: points,
                kind: kind.code(),
            }],
        }],
        fleets: vec![],
        messages: vec![],
    };
    submission.apply_to(game, &mut Vec::new()).is_ok()
}

#[test]
fn a_world_without_a_base_may_order_one_from_its_treasury() {
    let mut game = colony_game(vec![]);
    assert_eq!(game.planet_build_points_left(1).unwrap(), 20);
    assert_eq!(
        game.planet_spend_left(1)
            .unwrap()
            .points_for(ProductionItemKind::Starbase),
        70
    );
    assert!(!order(&mut game, ProductionItemKind::Cruiser, 30));
    assert!(order(&mut game, ProductionItemKind::Starbase, 50));
    // The base leaves the capacity whole; the treasury holds 30 more.
    assert_eq!(game.planet_build_points_left(1).unwrap(), 20);
    assert!(!order(&mut game, ProductionItemKind::Starbase, 50));
    assert!(order(&mut game, ProductionItemKind::Destroyer, 20));
    assert_eq!(game.planet_build_points_left(1).unwrap(), 0);
}

#[test]
fn maintenance_pays_the_first_base_past_the_capacity() {
    let mut game = colony_game(vec![]);
    assert!(order(&mut game, ProductionItemKind::Starbase, 50));
    assert!(order(&mut game, ProductionItemKind::Destroyer, 20));
    run_maintenance_turn(&mut game).expect("maintenance should succeed");
    let world = &game.planets[0];
    assert_eq!(world.stardock.starbases, 1);
    assert_eq!(world.stardock.destroyers, 4);
    assert!(world.build_queue.iter().all(|slot| slot.points_owed == 0));
    assert_eq!(stored_production_points(world), 10);
}

#[test]
fn a_short_treasury_pays_the_first_base_over_several_years() {
    let mut game = colony_game(vec![]);
    game.planets[0].stored_goods = nc_data::Real48::from_f64(30.0);
    assert!(!order(&mut game, ProductionItemKind::Starbase, 50));
    // A base already queued from an earlier year keeps its allowance.
    game.append_planet_build_units(1, ProductionItemKind::Starbase, 1)
        .expect("order");
    run_maintenance_turn(&mut game).expect("maintenance should succeed");
    assert_eq!(game.planets[0].build_queue[0].points_owed, 20);
    assert_eq!(stored_production_points(&game.planets[0]), 0);
}

#[test]
fn a_second_base_keeps_the_standard_restrictions() {
    let mut docked = colony_game(vec![]);
    docked.planets[0].stardock.starbases = 1;
    assert_eq!(
        docked
            .planet_spend_left(1)
            .unwrap()
            .points_for(ProductionItemKind::Starbase),
        20
    );

    // A base in orbit lifts the capacity to five times production instead.
    let based = colony_game(vec![commissioned_starbase(1, WORLD)]);
    let limits = based.planet_spend_limits(1).unwrap();
    assert_eq!((limits.capacity, limits.first_base), (100, 0));
    assert_eq!(
        based
            .planet_spend_left(1)
            .unwrap()
            .points_for(ProductionItemKind::Starbase),
        80
    );
}

#[test]
fn a_base_draws_only_on_its_own_worlds_treasury() {
    // A rich second world does not lend its savings to a poor one.
    let mut game = colony_game(vec![]);
    game.planets[0].stored_goods = nc_data::Real48::from_f64(30.0);
    let mut rich = owned_planet_with_present_production(1, 100, 20, 1000, 3, 1);
    rich.sector = Coords::new(2, 2);
    game.planets.push(rich);

    assert_eq!(
        game.planet_spend_left(1)
            .unwrap()
            .points_for(ProductionItemKind::Starbase),
        30
    );
    assert!(!order(&mut game, ProductionItemKind::Starbase, 50));

    game.append_planet_build_units(1, ProductionItemKind::Starbase, 1)
        .expect("order");
    run_maintenance_turn(&mut game).expect("maintenance should succeed");
    assert_eq!(game.planets[0].build_queue[0].points_owed, 20);
    assert_eq!(stored_production_points(&game.planets[0]), 0);
    assert_eq!(stored_production_points(&game.planets[1]), 1000);
}
