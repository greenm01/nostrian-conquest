use std::collections::BTreeSet;

use nc_data::{Coords, GameStateBuilder};
use nc_engine::default_host_fleet_target;

#[test]
fn default_host_fleet_target_ranks_distant_fleets_without_overflow() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");
    // dx = 199 from the anchor squares past i16 range.
    game_data.fleets[0].sector = Coords::new(200, 1);

    let target = default_host_fleet_target(&game_data, 1, [1, 1], &BTreeSet::new())
        .expect("empire 1 has fleets");

    assert_eq!(target.coords, [16, 13]);
}

#[test]
fn a_fleet_on_salvage_is_never_the_default_join_host() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");
    let anchor = game_data.fleets[0].sector.to_array();
    let nearest = default_host_fleet_target(&game_data, 1, anchor, &BTreeSet::new())
        .expect("empire 1 has fleets")
        .fleet_record_index_1_based;
    game_data.fleets[nearest - 1].order = nc_data::Order::Salvage;

    let host = default_host_fleet_target(&game_data, 1, anchor, &BTreeSet::new())
        .expect("empire 1 has other fleets");
    assert_ne!(host.fleet_record_index_1_based, nearest);
}

#[test]
fn warships_are_not_offered_a_scout_fleet_as_their_default_join_host() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");
    let joiner = BTreeSet::from([1]);
    assert!(nc_data::has_combat_ships(&game_data.fleets[0]));
    let anchor = game_data.fleets[0].sector.to_array();
    let nearest = default_host_fleet_target(&game_data, 1, anchor, &joiner)
        .expect("empire 1 has fleets")
        .fleet_record_index_1_based;
    game_data.fleets[nearest - 1].ships = Default::default();
    game_data.fleets[nearest - 1].ships.scouts = 1;

    let host = default_host_fleet_target(&game_data, 1, anchor, &joiner)
        .expect("empire 1 has other fleets");
    assert_ne!(host.fleet_record_index_1_based, nearest);
}
