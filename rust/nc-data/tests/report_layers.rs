//! Report headers name the layer a fleet is in, as classic ECMAINT does
//! (2026-10-30, `tools/ncmaint_stardate_audit.py`, mission 4 with the
//! sweep's `--deep-base`).

mod common;

use common::move_scenario::*;
use nc_data::{Coords, Order, Real48, Starbase};

#[test]
fn a_guard_joining_a_starbase_in_deep_space_reports_from_the_sector() {
    let mut game_data = scenario();
    let mut base = Starbase::default();
    base.base_id.0 = 1;
    base.owner.0 = 1;
    base.record.0 = 1;
    base.sector = Coords::from_array(EMPTY_SECTOR);
    base.x_fraction = Real48::HALF;
    base.y_fraction = Real48::HALF;
    base.depth = Real48::ZERO;
    base.target = base.sector;
    base.target_depth = 0;
    game_data.bases = vec![base];
    game_data.players[0].bases_first.0 = 1;
    let [x, y] = EMPTY_SECTOR;
    place_fleet(
        &mut game_data,
        [x + 1, y],
        Real48::ZERO,
        Order::GuardStarbase,
        EMPTY_SECTOR,
        3,
    );
    game_data.fleets[0].mission_target = 1;

    let events = run(&mut game_data);

    let text = report_text(&game_data, &events);
    assert!(
        text.contains("From your 1st Fleet, located in Sector(13,13):"),
        "{text}"
    );
    assert!(
        text.contains("Guard Starbase mission report: We have arrived at Starbase 1"),
        "{text}"
    );
}
