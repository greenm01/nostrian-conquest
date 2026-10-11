mod common;

use common::production::joinable_single_player_game;
use nc_data::{
    ColonizationResolvedEvent, Coords, CoreGameData, Order, present_production_points,
    stored_production_points,
};
use nc_engine::run_maintenance_turn;

fn colonization_probe(potential: u8) -> (CoreGameData, usize) {
    let mut game = joinable_single_player_game();
    game.join_player(1, "Codex Dominion")
        .expect("join player should succeed");

    let target_idx = game
        .planets
        .iter()
        .position(|planet| planet.owner.0 == 0)
        .expect("baseline should have an unowned planet");
    game.planets[target_idx].potential_production = potential;
    let target_coords = game.planets[target_idx].sector.to_array();

    let fleet_idx = game
        .fleets
        .iter()
        .position(|fleet| fleet.owner.0 == 1 && fleet.ships.etacs > 0)
        .expect("joined player should have an ETAC fleet");
    let staging_coords = if target_coords[0] > 1 {
        [target_coords[0] - 1, target_coords[1]]
    } else {
        [target_coords[0] + 1, target_coords[1]]
    };
    game.fleets[fleet_idx].sector = Coords::from_array(staging_coords);
    game.set_fleet_order(
        fleet_idx + 1,
        3,
        Order::ColonizeWorld.code(),
        target_coords,
        None,
        None,
    )
    .expect("colonize order should apply");

    (game, target_idx)
}

#[test]
fn newly_colonized_active_planets_skip_same_turn_economics() {
    let (mut game, target_idx) = colonization_probe(40);

    let events = run_maintenance_turn(&mut game).expect("maintenance should succeed");
    assert!(events.colonization_events.iter().any(|event| {
        matches!(
            event,
            ColonizationResolvedEvent::Succeeded {
                planet_idx,
                colonizer_empire: 1,
                ..
            } if *planet_idx == target_idx
        )
    }));

    let colony = &game.planets[target_idx];
    assert_eq!(colony.owner.0, 1);
    assert_eq!(stored_production_points(colony), 0);
    // The colony head start: production 3, not 1.
    assert_eq!(present_production_points(colony), 3);
}

#[test]
fn a_small_world_starts_at_its_potential_when_that_is_under_the_head_start() {
    let (mut game, target_idx) = colonization_probe(2);
    run_maintenance_turn(&mut game).expect("maintenance should succeed");
    assert_eq!(game.planets[target_idx].owner.0, 1);
    assert_eq!(present_production_points(&game.planets[target_idx]), 2);
}
