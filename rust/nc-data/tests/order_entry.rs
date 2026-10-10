//! What ECGAME writes when a player gives a fleet an order (t034): ROE 0 for
//! fleets without combat ships, and the nearest owned world as the Seek Home
//! and Salvage target.

use std::path::PathBuf;

use nc_data::{
    CoreGameData, FleetOrderValidationError, GameStateMutationError, Order, Real48, SeekHomeTarget,
    TurnSubmission,
};

fn ecmaint_post() -> CoreGameData {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/ecmaint-post/v1.5");
    CoreGameData::load(&dir).expect("load ecmaint-post fixture")
}

/// Fleet record 3 (empire 1, one destroyer) at rest at `coords`.
fn park_fleet_3(data: &mut CoreGameData, coords: [u8; 2]) {
    let fleet = &mut data.fleets.records[2];
    fleet.set_current_location_coords_raw(coords);
    fleet.set_at_rest_motion();
}

/// Give empire 1 planet `record` and move it to `coords`.
fn own_world(data: &mut CoreGameData, record: usize, coords: [u8; 2]) {
    let planet = &mut data.planets.records[record - 1];
    planet.set_coords_raw(coords);
    planet.set_owner_empire_slot_raw(1);
}

fn not_nearest(result: Result<[u8; 2], GameStateMutationError>) -> bool {
    matches!(
        result,
        Err(GameStateMutationError::InvalidFleetOrder {
            reason: FleetOrderValidationError::NotNearestOwnedWorld,
            ..
        })
    )
}

#[test]
fn any_order_clears_roe_for_a_fleet_without_combat_ships() {
    let mut data = ecmaint_post();
    let fleet = &mut data.fleets.records[2];
    fleet.set_destroyer_count(0);
    fleet.set_scout_count(1);
    fleet.recompute_max_speed_from_composition();
    assert_eq!(fleet.rules_of_engagement(), 6);
    data.set_fleet_order(3, 1, Order::ScoutSector.to_raw(), [10, 10], None, None)
        .unwrap();
    assert_eq!(data.fleets.records[2].rules_of_engagement(), 0);

    let fleet = &mut data.fleets.records[3];
    fleet.set_destroyer_count(0);
    fleet.set_empty_transport_count(2);
    fleet.recompute_max_speed_from_composition();
    data.set_fleet_order(4, 1, Order::MoveOnly.to_raw(), [10, 10], None, None)
        .unwrap();
    assert_eq!(data.fleets.records[3].rules_of_engagement(), 0);

    // A cruiser escort keeps the ROE (taskA: CA + ETAC on Colonize).
    data.set_fleet_order(1, 1, Order::MoveOnly.to_raw(), [10, 10], None, None)
        .unwrap();
    assert_eq!(data.fleets.records[0].rules_of_engagement(), 6);
}

#[test]
fn seek_home_heads_for_the_only_owned_world() {
    // taskA: a fleet at (10,10) of empire 1, whose only world is (16,13).
    let mut data = ecmaint_post();
    park_fleet_3(&mut data, [10, 10]);
    assert_eq!(data.nearest_owned_world(3, false), Some([16, 13]));
    assert_eq!(data.seek_home_target(3), SeekHomeTarget::World([16, 13]));
    data.set_fleet_order(3, 1, Order::SeekHome.to_raw(), [16, 13], None, None)
        .unwrap();
    assert!(not_nearest(data.set_fleet_order(
        3,
        1,
        Order::SeekHome.to_raw(),
        [10, 10],
        None,
        None
    )));
}

#[test]
fn nearest_world_keeps_the_first_in_planet_order_on_a_tie() {
    let mut data = ecmaint_post();
    park_fleet_3(&mut data, [10, 10]);
    own_world(&mut data, 1, [8, 10]);
    own_world(&mut data, 2, [12, 10]);
    assert_eq!(data.nearest_owned_world(3, false), Some([8, 10]));

    own_world(&mut data, 1, [12, 10]);
    own_world(&mut data, 2, [8, 10]);
    assert_eq!(data.nearest_owned_world(3, false), Some([12, 10]));
}

#[test]
fn nearest_world_measures_from_the_fleet_position_inside_its_sector() {
    // Both worlds are two sectors away, but the fleet sits at x + 0.9, so the
    // eastern one is closer in ECGAME's travel distance.
    let mut data = ecmaint_post();
    park_fleet_3(&mut data, [10, 10]);
    data.fleets.records[2].set_x_fraction(Real48::from_int(9) / Real48::from_int(10));
    own_world(&mut data, 1, [8, 10]);
    own_world(&mut data, 2, [12, 10]);
    assert_eq!(data.nearest_owned_world(3, false), Some([12, 10]));
}

#[test]
fn seek_home_from_a_home_system_takes_the_next_closest_world() {
    let mut data = ecmaint_post();
    park_fleet_3(&mut data, [16, 13]);
    assert_eq!(
        data.seek_home_target(3),
        SeekHomeTarget::AlreadyInOnlySystem
    );

    own_world(&mut data, 1, [11, 1]);
    own_world(&mut data, 14, [15, 13]);
    assert_eq!(data.nearest_owned_world(3, false), Some([16, 13]));
    assert_eq!(data.nearest_owned_world(3, true), Some([15, 13]));
    assert_eq!(
        data.seek_home_target(3),
        SeekHomeTarget::NextClosest([15, 13])
    );
    data.set_fleet_order(3, 1, Order::SeekHome.to_raw(), [15, 13], None, None)
        .unwrap();
    assert!(not_nearest(data.set_fleet_order(
        3,
        1,
        Order::SeekHome.to_raw(),
        [11, 1],
        None,
        None
    )));

    // Climbing out (depth below 0.5), the fleet no longer skips its system.
    data.fleets.records[2].set_depth(Real48::from_int(1) / Real48::from_int(4));
    assert_eq!(data.nearest_owned_world(3, true), Some([16, 13]));
    assert_eq!(data.seek_home_target(3), SeekHomeTarget::World([16, 13]));
}

#[test]
fn seek_home_without_an_owned_world_has_no_target() {
    let mut data = ecmaint_post();
    data.planets.records[14].set_owner_empire_slot_raw(0);
    assert_eq!(data.nearest_owned_world(3, false), None);
    assert_eq!(data.seek_home_target(3), SeekHomeTarget::NoOwnedWorld);
    assert_eq!(
        data.seek_home_targets(&[3, 4]),
        Err(SeekHomeTarget::NoOwnedWorld)
    );
}

#[test]
fn a_seek_home_group_heads_each_fleet_for_its_own_nearest_world() {
    let mut data = ecmaint_post();
    own_world(&mut data, 1, [11, 1]);
    park_fleet_3(&mut data, [12, 2]);
    let fleet_4 = &mut data.fleets.records[3];
    fleet_4.set_current_location_coords_raw([16, 12]);
    fleet_4.set_at_rest_motion();
    assert_eq!(data.seek_home_targets(&[3, 4]), Ok(vec![[11, 1], [16, 13]]));
}

fn turn_with_fleet_3_order(data: &CoreGameData, order: &str) -> String {
    format!(
        "turn player=1 year={}\nfleet record=3 {{\n  {order}\n}}\n",
        data.conquest.game_year()
    )
}

#[test]
fn turn_files_may_leave_the_seek_home_and_salvage_target_out() {
    let mut data = ecmaint_post();
    park_fleet_3(&mut data, [10, 10]);
    let kdl = turn_with_fleet_3_order(&data, r#"order speed=3 kind="seek_home""#);
    let submission = TurnSubmission::parse_kdl_str(&kdl).unwrap();
    assert!(!submission.to_kdl_string().contains(" x="));
    submission.apply_to(&mut data, &mut Vec::new()).unwrap();
    let fleet = &data.fleets.records[2];
    assert_eq!(fleet.standing_order_kind(), Order::SeekHome);
    assert_eq!(fleet.standing_order_target_coords_raw(), [16, 13]);

    let kdl = turn_with_fleet_3_order(&data, r#"order speed=3 kind="salvage""#);
    TurnSubmission::parse_kdl_str(&kdl)
        .unwrap()
        .apply_to(&mut data, &mut Vec::new())
        .unwrap();
    let fleet = &data.fleets.records[2];
    assert_eq!(fleet.standing_order_kind(), Order::Salvage);
    assert_eq!(fleet.standing_order_target_coords_raw(), [16, 13]);
}

#[test]
fn turn_files_need_both_coordinates_for_other_orders() {
    let data = ecmaint_post();
    for order in [
        r#"order speed=3 kind="move""#,
        r#"order speed=3 kind="seek_home" x=16"#,
    ] {
        let kdl = turn_with_fleet_3_order(&data, order);
        assert!(TurnSubmission::parse_kdl_str(&kdl).is_err(), "{order}");
    }
}
