use nc_compat::{DatabaseDat, merge_player_intel_from_compat};
use nc_data::Order;
use nc_engine::{
    FleetEtaEstimate, GameStateBuilder, VisibleHazardIntel, estimate_fleet_eta,
    estimate_fleet_eta_to_destination, plan_route, plan_route_with_intel, run_maintenance_turn,
    run_maintenance_turn_with_visible_hazards, visible_hazard_intel_from_snapshots,
};

#[test]
fn pathfinder_prefers_direct_route_when_space_is_safe() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    let fleet = &mut game_data.fleets[0];
    fleet.set_current_location_coords_raw([2, 2]);
    fleet.set_standing_order_kind(Order::MoveOnly);
    fleet.set_standing_order_target_coords_raw([6, 2]);
    fleet.set_current_speed(3);

    let route = plan_route(&game_data, 0).expect("route should exist");
    assert_eq!(route.steps.first().map(|step| step.coords), Some([2, 2]));
    assert_eq!(route.steps.last().map(|step| step.coords), Some([6, 2]));
    assert!(route.steps.iter().all(|step| step.coords[1] == 2));
}

#[test]
fn pathfinder_avoids_foreign_guarded_system_when_alternate_route_exists() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    let fleet = &mut game_data.fleets[0];
    fleet.set_current_location_coords_raw([2, 2]);
    fleet.set_standing_order_kind(Order::MoveOnly);
    fleet.set_standing_order_target_coords_raw([6, 2]);
    fleet.set_current_speed(3);

    let mut intel = VisibleHazardIntel::default();
    intel.foreign_worlds.insert([4, 2]);
    intel.foreign_starbases.insert([4, 2]);

    let route = plan_route_with_intel(&game_data, 0, &intel).expect("route should exist");
    assert!(!route.steps.iter().any(|step| step.coords == [4, 2]));
}

#[test]
fn maintenance_preserves_fog_of_war_and_uses_direct_route_without_visible_intel() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    let fleet = &mut game_data.fleets[0];
    fleet.set_current_location_coords_raw([2, 2]);
    fleet.set_standing_order_kind(Order::MoveOnly);
    fleet.set_standing_order_target_coords_raw([6, 2]);
    fleet.set_current_speed(3);

    let foreign_world = &mut game_data.planets[4];
    foreign_world.set_coords_raw([4, 2]);
    foreign_world.set_owner_empire_slot_raw(2);
    foreign_world.set_ownership_status_raw(2);
    foreign_world.set_ground_batteries_raw(5);

    run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(game_data.fleets[0].current_location_coords_raw(), [4, 2]);
}

#[test]
fn visible_hazard_intel_derives_known_foreign_worlds_from_database_view() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");
    game_data.planets[4].set_coords_raw([4, 2]);

    let planet_count = game_data.planets.len();
    let mut database =
        DatabaseDat::new_zeroed((game_data.conquest.player_count() as usize) * planet_count);
    let record = database.record_mut(4, 0, planet_count);
    record.set_planet_name("Prime");
    record.raw[0x15] = 2;
    record.raw[0x1c] = 100;

    let snapshots = merge_player_intel_from_compat(
        &game_data,
        &database,
        1,
        game_data.conquest.game_year(),
        None,
    );
    let intel = visible_hazard_intel_from_snapshots(&game_data, &snapshots, 1);
    assert!(intel.foreign_worlds.contains(&[4, 2]));
    assert!(intel.hostile_homeworlds.contains(&[4, 2]));
}

#[test]
fn visible_hazard_intel_marks_foreign_blockades_over_owned_worlds() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");
    game_data.planets[4].set_coords_raw([4, 2]);
    game_data.planets[4].set_owner_empire_slot_raw(1);
    game_data.planets[4].set_ownership_status_raw(2);

    let blockader = &mut game_data.fleets[4];
    blockader.set_owner_empire_raw(2);
    blockader.set_current_location_coords_raw([4, 2]);
    blockader.set_standing_order_kind(Order::GuardBlockadeWorld);
    blockader.set_standing_order_target_coords_raw([4, 2]);

    let planet_count = game_data.planets.len();
    let mut database =
        DatabaseDat::new_zeroed((game_data.conquest.player_count() as usize) * planet_count);
    let record = database.record_mut(4, 0, planet_count);
    record.set_planet_name("Home Rim");
    record.raw[0x15] = 1;

    let snapshots = merge_player_intel_from_compat(
        &game_data,
        &database,
        1,
        game_data.conquest.game_year(),
        None,
    );
    let intel = visible_hazard_intel_from_snapshots(&game_data, &snapshots, 1);
    assert!(intel.hostile_blockades.contains(&[4, 2]));
}

#[test]
fn maintenance_avoids_known_foreign_world_when_visible_hazard_intel_is_supplied() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    let fleet = &mut game_data.fleets[0];
    fleet.set_current_location_coords_raw([2, 2]);
    fleet.set_standing_order_kind(Order::MoveOnly);
    fleet.set_standing_order_target_coords_raw([6, 2]);
    fleet.set_current_speed(3);

    let mut hazards =
        vec![VisibleHazardIntel::default(); game_data.conquest.player_count() as usize];
    hazards[0].foreign_worlds.insert([4, 2]);

    run_maintenance_turn_with_visible_hazards(&mut game_data, &hazards)
        .expect("maintenance should succeed");

    assert_ne!(game_data.fleets[0].current_location_coords_raw(), [4, 2]);
}

#[test]
fn maintenance_avoids_owned_world_under_known_blockade() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    let fleet = &mut game_data.fleets[0];
    fleet.set_current_location_coords_raw([2, 2]);
    fleet.set_standing_order_kind(Order::MoveOnly);
    fleet.set_standing_order_target_coords_raw([6, 2]);
    fleet.set_current_speed(3);

    let mut hazards =
        vec![VisibleHazardIntel::default(); game_data.conquest.player_count() as usize];
    hazards[0].hostile_blockades.insert([4, 2]);

    run_maintenance_turn_with_visible_hazards(&mut game_data, &hazards)
        .expect("maintenance should succeed");

    assert_ne!(game_data.fleets[0].current_location_coords_raw(), [4, 2]);
}

#[test]
fn fleet_eta_uses_movement_subgrid_budget_instead_of_simple_div_ceil() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    let fleet = &mut game_data.fleets[0];
    fleet.set_current_location_coords_raw([2, 2]);
    fleet.set_standing_order_kind(Order::MoveOnly);
    fleet.set_standing_order_target_coords_raw([3, 2]);
    fleet.set_current_speed(1);
    fleet.set_mission_aux_bytes([0, 0]);

    assert_eq!(
        estimate_fleet_eta(&game_data, 0),
        FleetEtaEstimate::Years(2)
    );
}

#[test]
fn fleet_eta_restores_max_speed_unless_holding() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    let fleet = &mut game_data.fleets[0];
    fleet.set_current_location_coords_raw([2, 2]);
    fleet.set_standing_order_kind(Order::MoveOnly);
    fleet.set_standing_order_target_coords_raw([6, 2]);
    fleet.set_current_speed(0);
    fleet.set_mission_aux_bytes([0, 0]);

    // ECMAINT restores maximum speed to a stopped fleet whose order has not
    // arrived, after the first tick.
    assert_eq!(
        estimate_fleet_eta(&game_data, 0),
        FleetEtaEstimate::Years(2)
    );

    game_data.fleets[0].set_standing_order_kind(Order::HoldPosition);
    assert_eq!(estimate_fleet_eta(&game_data, 0), FleetEtaEstimate::Stopped);
}

#[test]
fn fleet_eta_to_destination_can_use_max_speed_for_stopped_fleets() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    let fleet = &mut game_data.fleets[0];
    fleet.set_current_location_coords_raw([2, 2]);
    fleet.set_standing_order_kind(Order::HoldPosition);
    fleet.set_standing_order_target_coords_raw([2, 2]);
    fleet.set_current_speed(0);

    assert_eq!(
        estimate_fleet_eta_to_destination(&game_data, 0, [3, 2], false, true),
        FleetEtaEstimate::Years(1)
    );
}

#[test]
fn fleet_eta_matches_actual_arrival_turn_for_every_speed() {
    // Axis-aligned and diagonal moves: the ETA replays the same 6-byte-real
    // integrator and arrival test that maintenance runs.
    let offsets: [[u8; 2]; 11] = [
        [1, 0],
        [2, 0],
        [5, 0],
        [9, 0],
        [12, 0],
        [16, 0],
        [1, 1],
        [3, 2],
        [6, 8],
        [7, 3],
        [10, 9],
    ];
    for speed in 1..=9u8 {
        for offset in offsets {
            let mut game_data = GameStateBuilder::new()
                .with_player_count(4)
                .with_year(3000)
                .build_initialized_baseline()
                .expect("baseline should build");
            for player in &mut game_data.players {
                player.set_autopilot_flag(0);
            }
            let target = [1 + offset[0], 9 + offset[1]];
            let fleet = &mut game_data.fleets[0];
            fleet.set_max_speed(9);
            fleet.set_current_location_coords_raw([1, 9]);
            fleet.set_standing_order_kind(Order::MoveOnly);
            fleet.set_standing_order_target_coords_raw(target);
            fleet.set_current_speed(speed);

            let mut turns = 0u16;
            let mut predictions = Vec::new();
            while game_data.fleets[0].standing_order_kind() == Order::MoveOnly {
                let FleetEtaEstimate::Years(predicted) = estimate_fleet_eta(&game_data, 0) else {
                    panic!("speed {speed} target {target:?}: fleet should be under way");
                };
                predictions.push(predicted);
                run_maintenance_turn(&mut game_data).expect("maintenance should succeed");
                turns += 1;
                assert!(turns <= 40, "speed {speed} target {target:?}: no arrival");
            }
            assert_eq!(game_data.fleets[0].current_location_coords_raw(), target);
            // The estimate before each year matches the years still to go.
            let remaining: Vec<u16> = (1..=turns).rev().collect();
            assert_eq!(
                predictions, remaining,
                "speed {speed} target {target:?}: ETA vs actual arrival"
            );
        }
    }
}

#[test]
fn pathfinder_reaches_the_last_row_and_column_of_the_map() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    // Map coordinates are 1-based, so an 18x18 map includes sector (18,18).
    let fleet = &mut game_data.fleets[0];
    fleet.set_current_location_coords_raw([15, 15]);
    fleet.set_standing_order_kind(Order::MoveOnly);
    fleet.set_standing_order_target_coords_raw([18, 18]);
    fleet.set_current_speed(3);

    let route = plan_route(&game_data, 0).expect("corner route should exist");
    assert_eq!(route.steps.last().map(|step| step.coords), Some([18, 18]));
}

#[test]
fn pathfinder_never_detours_through_row_or_column_zero() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    let fleet = &mut game_data.fleets[0];
    fleet.set_current_location_coords_raw([1, 3]);
    fleet.set_standing_order_kind(Order::MoveOnly);
    fleet.set_standing_order_target_coords_raw([1, 7]);
    fleet.set_current_speed(3);

    let mut intel = VisibleHazardIntel::default();
    intel.foreign_worlds.insert([1, 5]);
    intel.foreign_worlds.insert([2, 5]);

    let route = plan_route_with_intel(&game_data, 0, &intel).expect("route should exist");
    assert!(
        route
            .steps
            .iter()
            .all(|step| step.coords[0] >= 1 && step.coords[1] >= 1),
        "{route:?}"
    );
}
