mod common;

use common::production::joinable_single_player_game;
use nc_data::{ColonizationResolvedEvent, CoreGameData, Order};
use nc_engine::run_maintenance_turn;

fn colonization_probe() -> (CoreGameData, usize) {
    let mut game = joinable_single_player_game();
    game.join_player(1, "Codex Dominion")
        .expect("join player should succeed");

    let target_idx = game
        .planets
        .iter()
        .position(|planet| planet.owner_empire_slot_raw() == 0)
        .expect("baseline should have an unowned planet");
    let target_coords = game.planets[target_idx].coords_raw();

    let fleet_idx = game
        .fleets
        .iter()
        .position(|fleet| fleet.owner_empire_raw() == 1 && fleet.etac_count() > 0)
        .expect("joined player should have an ETAC fleet");
    let staging_coords = if target_coords[0] > 1 {
        [target_coords[0] - 1, target_coords[1]]
    } else {
        [target_coords[0] + 1, target_coords[1]]
    };
    game.fleets[fleet_idx].set_current_location_coords_raw(staging_coords);
    game.set_fleet_order(
        fleet_idx + 1,
        3,
        Order::ColonizeWorld.to_raw(),
        target_coords,
        None,
        None,
    )
    .expect("colonize order should apply");

    (game, target_idx)
}

#[test]
fn newly_colonized_active_planets_skip_same_turn_economics() {
    let (mut game, target_idx) = colonization_probe();

    let events = run_maintenance_turn(&mut game).expect("maintenance should succeed");
    assert!(events.colonization_events.iter().any(|event| {
        matches!(
            event,
            ColonizationResolvedEvent::Succeeded {
                planet_idx,
                colonizer_empire_raw: 1,
                ..
            } if *planet_idx == target_idx
        )
    }));

    let colony = &game.planets[target_idx];
    assert_eq!(colony.owner_empire_slot_raw(), 1);
    assert_eq!(colony.stored_production_points(), 0);
    assert_eq!(colony.present_production_points().unwrap_or(0), 1);
}
