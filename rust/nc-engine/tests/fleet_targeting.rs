use std::collections::BTreeSet;

use nc_data::GameStateBuilder;
use nc_engine::default_host_fleet_target;

#[test]
fn default_host_fleet_target_ranks_distant_fleets_without_overflow() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");
    // dx = 199 from the anchor squares past i16 range.
    game_data.fleets.records[0].set_current_location_coords_raw([200, 1]);

    let target = default_host_fleet_target(&game_data, 1, [1, 1], &BTreeSet::new())
        .expect("empire 1 has fleets");

    assert_eq!(target.coords, [16, 13]);
}
