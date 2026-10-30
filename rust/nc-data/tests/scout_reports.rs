//! View World and Scout Solar System reports against classic ECMAINT
//! (2026-10-30, `tools/ncmaint_stardate_audit.py`, missions 9 and 11).

mod common;

use common::move_scenario::*;
use nc_data::{Order, Real48};

#[test]
fn viewing_an_unowned_world_reports_but_has_a_potential() {
    let mut game_data = scenario();
    place_fleet(
        &mut game_data,
        EMPTY_SECTOR,
        Real48::ZERO,
        Order::ViewWorld,
        NEAR_WORLD,
        3,
    );
    let potential = planet_at(&mut game_data, NEAR_WORLD).potential_production;

    let events = run(&mut game_data);

    let text = report_text(&game_data, &events);
    assert!(
        text.contains(&format!(
            "The world is unowned but has a potential of {potential} points."
        )),
        "{text}"
    );
}

#[test]
fn a_scout_on_station_over_an_unowned_world_reports_in_week_1_as_classic_does() {
    let mut game_data = scenario();
    set_ships(&mut game_data.fleets[0], 0, 0, 0, 1, 0);
    place_fleet(
        &mut game_data,
        NEAR_WORLD,
        Real48::ONE,
        Order::ScoutSolarSystem,
        NEAR_WORLD,
        0,
    );
    game_data.fleets[0].rules_of_engagement = 0;
    let potential = planet_at(&mut game_data, NEAR_WORLD).potential_production;
    let year = game_data.conquest.game_year;

    let events = run(&mut game_data);

    let text = report_text(&game_data, &events);
    assert!(
        text.contains(&format!("Stardate: 01/{year} Scouting mission report")),
        "{text}"
    );
    assert!(
        text.contains(&format!(
            "Scouting mission report: We are in extended orbit around the world in System(12,12). The world is unowned but has a potential value of {potential} points."
        )),
        "{text}"
    );
}
