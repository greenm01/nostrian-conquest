mod common;

use common::production::{
    owned_planet_with_present_production, player_with_empire_name, single_planet_game,
};
use nc_data::{CoreGameData, GameStateBuilder, Order, PlanetRecord, ProductionItemKind};
use nc_engine::{build_seeded_initialized_game, run_maintenance_turn};

fn autopilot_game(planet: PlanetRecord) -> CoreGameData {
    let mut player = player_with_empire_name("Auto", 50, 0);
    player.set_player_mode_raw(0x01);
    player.set_autopilot_flag(1);
    single_planet_game(player, planet)
}

fn queued_points(planet: &PlanetRecord, kind: ProductionItemKind) -> u32 {
    (0..10)
        .filter(|&slot| ProductionItemKind::from_raw(planet.build_kind_raw(slot)) == kind)
        .map(|slot| u32::from(planet.build_count_raw(slot)))
        .sum()
}

fn stardock_starbases(planet: &PlanetRecord) -> u32 {
    (0..nc_data::STARDOCK_SLOT_COUNT)
        .filter(|&slot| {
            planet.stardock_item_kind_current_known(slot) == ProductionItemKind::Starbase
        })
        .map(|slot| u32::from(planet.stardock_count_raw(slot)))
        .sum()
}

fn active_bases_at(game: &CoreGameData, owner: u8, coords: [u8; 2]) -> usize {
    game.bases
        .records
        .iter()
        .filter(|base| {
            base.owner_empire_raw() == owner
                && base.coords_raw() == coords
                && base.active_flag_raw() != 0
        })
        .count()
}

#[test]
fn autopilot_queues_no_more_than_the_planet_budget() {
    let planet = owned_planet_with_present_production(1, 100, 100, 0, 0, 0);
    let mut game = autopilot_game(planet);

    run_maintenance_turn(&mut game).expect("maintenance should succeed");

    let planet = &game.planets.records[0];
    let queued: u32 = (0..10).map(|slot| u32::from(planet.build_count_raw(slot))).sum();
    assert_eq!(planet.stored_production_points(), 50);
    assert!(queued > 0, "autopilot should queue defenses");
    assert!(
        queued <= planet.stored_production_points(),
        "queued {queued} points against a treasury of {}",
        planet.stored_production_points()
    );
}

#[test]
fn autopilot_keeps_partially_built_orders() {
    let mut planet = owned_planet_with_present_production(1, 45, 40, 500, 200, 200);
    planet.set_build_count_raw(0, 100);
    planet.set_build_kind_raw(0, 1);
    let mut game = autopilot_game(planet);

    run_maintenance_turn(&mut game).expect("maintenance should succeed");

    let planet = &game.planets.records[0];
    assert_eq!(queued_points(planet, ProductionItemKind::Destroyer), 60);
    assert_eq!(planet.stardock_count_raw(0), 8);
}

#[test]
fn autopilot_finishes_a_multi_turn_starbase_and_commissions_it() {
    let mut planet = owned_planet_with_present_production(1, 45, 40, 500, 200, 200);
    planet.set_coords_raw([7, 7]);
    planet.set_build_count_raw(0, 50);
    planet.set_build_kind_raw(0, 9);
    let mut game = autopilot_game(planet);

    run_maintenance_turn(&mut game).expect("first maintenance should succeed");
    // 40 of 50 points paid: the starbase is not finished yet.
    assert_eq!(
        queued_points(&game.planets.records[0], ProductionItemKind::Starbase),
        10
    );
    assert_eq!(stardock_starbases(&game.planets.records[0]), 0);
    assert_eq!(active_bases_at(&game, 1, [7, 7]), 0);

    run_maintenance_turn(&mut game).expect("second maintenance should succeed");
    let planet = &game.planets.records[0];
    assert_eq!(queued_points(planet, ProductionItemKind::Starbase), 0);
    assert_eq!(stardock_starbases(planet), 0);
    assert_eq!(active_bases_at(&game, 1, [7, 7]), 1);
}

#[test]
fn autopilot_homeworld_builds_and_commissions_exactly_one_starbase() {
    let mut game = build_seeded_initialized_game(4, 3000, 1515).expect("seeded game should build");
    assert_eq!(game.player.records[0].autopilot_flag(), 1);
    let coords = game.planets.records[0].coords_raw();

    for _ in 0..6 {
        run_maintenance_turn(&mut game).expect("maintenance should succeed");
    }

    let homeworld = &game.planets.records[0];
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
    assert_eq!(game.player.records[0].autopilot_flag(), 1);
    game.planets.records[0].set_coords_raw([250, 250]);
    game.planets.records[4] = game.planets.records[0].clone();
    game.planets.records[4].set_coords_raw([10, 1]);
    let fleet = &mut game.fleets.records[0];
    fleet.set_current_location_coords_raw([1, 1]);
    fleet.set_standing_order_kind(Order::HoldPosition);
    fleet.set_standing_order_target_coords_raw([1, 1]);

    run_maintenance_turn(&mut game).expect("maintenance should succeed");

    let fleet = &game.fleets.records[0];
    assert_eq!(fleet.standing_order_kind(), Order::SeekHome);
    assert_eq!(fleet.standing_order_target_coords_raw(), [10, 1]);
}
