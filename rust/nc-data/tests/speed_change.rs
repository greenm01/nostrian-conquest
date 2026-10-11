//! ECGAME's speed change (`0x11c77`, t038): only the maximum speed (0x09,
//! recomputed from the ships) and the current speed (0x0a) change.

use std::path::PathBuf;

use nc_compat::load_classic_core_files;
use nc_data::{
    Coords, CoreGameData, GameStateMutationError, Order, TurnSubmission, set_mission_target,
};

fn ecmaint_post() -> CoreGameData {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/ecmaint-post/v1.5");
    load_classic_core_files(&dir).expect("load ecmaint-post fixture")
}

/// Fleet record 3 (one destroyer) under way on Move to (10,10).
fn moving_fleet_3(data: &mut CoreGameData) {
    let fleet = &mut data.fleets[2];
    fleet.order = Order::MoveOnly;
    fleet.target = Coords::new(10, 10);
    fleet.speed = 6;
    // Bytes no speed change may touch, including the mission id high byte.
    set_mission_target(fleet, 0, 0x0103);
}

#[test]
fn a_speed_change_writes_only_the_speed_bytes() {
    let mut data = ecmaint_post();
    moving_fleet_3(&mut data);
    let before = data.fleets[2].to_bytes();
    assert_eq!(data.set_fleet_speed(1, 3, 2), Ok(2));
    let after = data.fleets[2].to_bytes();
    let changed = (0..before.len())
        .filter(|&offset| before[offset] != after[offset])
        .collect::<Vec<_>>();
    assert_eq!(changed, vec![0x0a]);
    assert_eq!(data.fleets[2].mission_target, 0x0103);
}

#[test]
fn a_speed_change_recomputes_the_maximum_speed() {
    let mut data = ecmaint_post();
    moving_fleet_3(&mut data);
    data.fleets[2].max_speed = 2;
    assert_eq!(data.set_fleet_speed(1, 3, 6), Ok(6));
    assert_eq!(data.fleets[2].max_speed, 6);
}

#[test]
fn a_fleet_on_hold_or_at_its_target_keeps_speed_zero() {
    let mut data = ecmaint_post();
    // ecmaint-post fleet 1 guards its home world (16,13) at rest: arrived.
    assert_eq!(data.set_fleet_speed(1, 1, 2), Ok(0));
    assert_eq!(data.fleets[0].speed, 0);

    moving_fleet_3(&mut data);
    data.fleets[2].order = Order::HoldPosition;
    assert_eq!(data.set_fleet_speed(1, 3, 2), Ok(0));
}

#[test]
fn a_speed_change_takes_one_to_the_maximum() {
    let mut data = ecmaint_post();
    moving_fleet_3(&mut data);
    for speed in [0, 7] {
        assert!(matches!(
            data.set_fleet_speed(1, 3, speed),
            Err(GameStateMutationError::InvalidFleetSpeed { max: 6, .. })
        ));
    }
    assert!(data.set_fleet_speed(2, 3, 2).is_err());
}

#[test]
fn turn_files_change_speed_with_a_speed_node() {
    let mut data = ecmaint_post();
    moving_fleet_3(&mut data);
    let kdl = format!(
        "turn player=1 year={}\nfleet record=3 {{\n  speed value=4\n}}\n",
        data.conquest.game_year
    );
    let submission = TurnSubmission::parse_kdl_str(&kdl).unwrap();
    assert!(submission.to_kdl_string().contains("speed value=4"));
    submission.apply_to(&mut data, &mut Vec::new()).unwrap();
    assert_eq!(data.fleets[2].speed, 4);
    assert_eq!(data.fleets[2].mission_target, 0x0103);
}
