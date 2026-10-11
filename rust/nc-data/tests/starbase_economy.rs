//! Which starbases count for a world's economy: ECMAINT's growth bonus
//! (`1852:011b`) and ECGAME's 5x build budget.

mod common;

use common::production::{
    commissioned_starbase, configured_conquest, owned_planet_with_present_production,
    player_with_empire_name, zeroed_setup,
};
use nc_data::{Coords, CoreGameData, PlayerMode, Real48, Starbase, present_production_points};
use nc_engine::run_maintenance_turn;

const WORLD: [u8; 2] = [8, 8];

fn colony_game(bases: Vec<Starbase>) -> CoreGameData {
    let mut player = player_with_empire_name("Alpha", 50, 0);
    player.mode = PlayerMode::Human;
    let mut colony = owned_planet_with_present_production(1, 100, 50, 0, 3, 1);
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

fn deep_space_base() -> Starbase {
    let mut base = commissioned_starbase(1, WORLD);
    base.depth = Real48::ZERO;
    base.target_depth = 0;
    base
}

fn present_after_one_year(bases: Vec<Starbase>) -> Option<u16> {
    let mut game = colony_game(bases);
    run_maintenance_turn(&mut game).expect("maintenance should succeed");
    Some(present_production_points(&game.planets[0]))
}

#[test]
fn a_base_at_rest_in_orbit_speeds_growth() {
    let plain = present_after_one_year(vec![]);
    // From 50 at 50% tax: 56.25, and 59.375 with a base.
    assert_eq!(plain, Some(56));
    assert_eq!(
        present_after_one_year(vec![commissioned_starbase(1, WORLD)]),
        Some(59)
    );
}

#[test]
fn a_base_in_deep_space_gives_no_growth_bonus() {
    assert_eq!(present_after_one_year(vec![deep_space_base()]), Some(56));
}

#[test]
fn a_base_at_depth_one_half_counts_as_in_orbit() {
    let mut base = commissioned_starbase(1, WORLD);
    base.depth = Real48::HALF;
    assert_eq!(present_after_one_year(vec![base]), Some(59));
}

#[test]
fn a_moving_base_gives_no_growth_bonus_even_when_it_ends_the_year_at_rest() {
    // ECMAINT settles the bonus before the year's ticks; this base's flag
    // clears on tick 1 when it finds itself at its target.
    let mut base = commissioned_starbase(1, WORLD);
    base.moving = true;
    let mut game = colony_game(vec![base]);
    run_maintenance_turn(&mut game).expect("maintenance should succeed");
    assert!(!game.bases[0].moving);
    assert_eq!(present_production_points(&game.planets[0]), 56);
}

#[test]
fn build_budget_takes_five_times_production_only_from_a_base_bound_for_orbit() {
    let budget = |base: Starbase| {
        let mut game = colony_game(vec![base]);
        game.planets[0].stored_goods = Real48::from_f64(1000.0);
        game.planet_build_budget(1).expect("budget")
    };

    assert_eq!(budget(commissioned_starbase(1, WORLD)), 250);
    assert_eq!(budget(deep_space_base()), 50);

    let mut leaving = commissioned_starbase(1, WORLD);
    leaving.moving = true;
    leaving.target = Coords::new(9, 8);
    leaving.target_depth = 0;
    assert_eq!(budget(leaving), 50);

    let mut descending = deep_space_base();
    descending.depth = Real48::HALF;
    descending.moving = true;
    descending.target_depth = 1;
    assert_eq!(budget(descending), 250);
}
