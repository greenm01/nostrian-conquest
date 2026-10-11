mod common;

use common::production::{
    owned_planet_with_present_production, player_with_empire_name, single_planet_game,
};
use nc_data::{
    Coords, CoreGameData, GameStateBuilder, Order, Planet, PlayerMode, ProductionItemKind,
    stardock_slot_kind, stored_production_points,
};
use nc_engine::{build_seeded_initialized_game, run_maintenance_turn};

fn autopilot_game(planet: Planet) -> CoreGameData {
    let mut player = player_with_empire_name("Auto", 50, 0);
    player.mode = PlayerMode::Human;
    player.autopilot = true;
    single_planet_game(player, planet)
}

fn queued_points(planet: &Planet, kind: ProductionItemKind) -> u32 {
    (0..10)
        .filter(|&slot| planet.build_queue[slot].kind == kind)
        .map(|slot| u32::from(planet.build_queue[slot].points_owed))
        .sum()
}

fn stardock_starbases(planet: &Planet) -> u32 {
    (0..nc_data::STARDOCK_SLOT_COUNT)
        .filter(|&slot| stardock_slot_kind(planet, slot) == ProductionItemKind::Starbase)
        .map(|slot| u32::from(u16::try_from(planet.stardock[slot]).unwrap_or(u16::MAX)))
        .sum()
}

fn active_bases_at(game: &CoreGameData, owner: u8, coords: [u8; 2]) -> usize {
    game.bases
        .iter()
        .filter(|base| {
            base.owner.0 == owner && base.sector.to_array() == coords && base.base_id.0 != 0
        })
        .count()
}

#[test]
fn autopilot_queues_no_more_than_the_planet_budget() {
    let planet = owned_planet_with_present_production(1, 100, 100, 0, 0, 0);
    let mut game = autopilot_game(planet);

    run_maintenance_turn(&mut game).expect("maintenance should succeed");

    let planet = &game.planets[0];
    let queued: u32 = (0..10)
        .map(|slot| u32::from(planet.build_queue[slot].points_owed))
        .sum();
    assert_eq!(stored_production_points(planet), 50);
    assert!(queued > 0, "autopilot should queue defenses");
    assert!(
        queued <= stored_production_points(planet),
        "queued {queued} points against a treasury of {}",
        stored_production_points(planet)
    );
}

#[test]
fn autopilot_keeps_partially_built_orders() {
    let mut planet = owned_planet_with_present_production(1, 45, 40, 500, 200, 200);
    planet.build_queue[0].points_owed = 100;
    planet.build_queue[0].kind = ProductionItemKind::Destroyer;
    let mut game = autopilot_game(planet);

    run_maintenance_turn(&mut game).expect("maintenance should succeed");

    let planet = &game.planets[0];
    assert_eq!(queued_points(planet, ProductionItemKind::Destroyer), 60);
    assert_eq!(planet.stardock.destroyers, 8);
}

#[test]
fn autopilot_finishes_a_multi_turn_starbase_and_commissions_it() {
    let mut planet = owned_planet_with_present_production(1, 45, 40, 500, 200, 200);
    planet.sector = Coords::new(7, 7);
    planet.build_queue[0].points_owed = 50;
    planet.build_queue[0].kind = ProductionItemKind::Starbase;
    let mut game = autopilot_game(planet);

    run_maintenance_turn(&mut game).expect("first maintenance should succeed");
    // 40 of 50 points paid: the starbase is not finished yet.
    assert_eq!(
        queued_points(&game.planets[0], ProductionItemKind::Starbase),
        10
    );
    assert_eq!(stardock_starbases(&game.planets[0]), 0);
    assert_eq!(active_bases_at(&game, 1, [7, 7]), 0);

    run_maintenance_turn(&mut game).expect("second maintenance should succeed");
    let planet = &game.planets[0];
    assert_eq!(queued_points(planet, ProductionItemKind::Starbase), 0);
    assert_eq!(stardock_starbases(planet), 0);
    assert_eq!(active_bases_at(&game, 1, [7, 7]), 1);
}

#[test]
fn autopilot_homeworld_builds_and_commissions_exactly_one_starbase() {
    let mut game = build_seeded_initialized_game(4, 3000, 1515).expect("seeded game should build");
    assert!(game.players[0].autopilot);
    let coords = game.planets[0].sector.to_array();

    for _ in 0..6 {
        run_maintenance_turn(&mut game).expect("maintenance should succeed");
    }

    let homeworld = &game.planets[0];
    assert_eq!(stardock_starbases(homeworld), 0);
    assert_eq!(queued_points(homeworld, ProductionItemKind::Starbase), 0);
    assert_eq!(active_bases_at(&game, 1, coords), 1);
}

#[test]
fn autopilot_recall_picks_the_nearest_world_across_the_whole_map() {
    // Squared distances past about 181 sectors overflow i16.
    let mut game = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");
    assert!(game.players[0].autopilot);
    game.planets[0].sector = Coords::new(250, 250);
    game.planets[4] = game.planets[0].clone();
    game.planets[4].sector = Coords::new(10, 1);
    let fleet = &mut game.fleets[0];
    fleet.sector = Coords::new(1, 1);
    fleet.order = Order::HoldPosition;
    fleet.target = Coords::new(1, 1);

    run_maintenance_turn(&mut game).expect("maintenance should succeed");

    let fleet = &game.fleets[0];
    assert_eq!(fleet.order, Order::SeekHome);
    assert_eq!(fleet.target.to_array(), [10, 1]);
}
