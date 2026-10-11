use nc_data::{
    Coords, FleetMergeEvent, GameStateBuilder, MaintenanceEvents, Mission, MissionEvent,
    MissionOutcome, Order,
};
use nc_engine::{build_results_report_blocks, run_maintenance_turn};

fn viewer_report_text(
    game_data: &nc_data::CoreGameData,
    events: &MaintenanceEvents,
    viewer: u8,
) -> String {
    build_results_report_blocks(game_data, events)
        .into_iter()
        .filter(|row| row.viewer_empire_id == viewer)
        .map(|row| row.decoded_text)
        .collect::<Vec<_>>()
        .join(" ")
        .replace('\n', " ")
}

fn baseline_game() -> nc_data::CoreGameData {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");
    for player in &mut game_data.players {
        player.autopilot = false;
    }
    game_data
}

#[test]
fn salvage_success_report_names_the_salvaged_fleet() {
    let mut game_data = baseline_game();
    let target = game_data.planets[0].sector.to_array();
    let fleet = &mut game_data.fleets[2];
    assert_eq!(fleet.fleet_number.0, 3);
    fleet.sector = Coords::new(target[0] + 1, target[1]);
    fleet.order = Order::Salvage;
    fleet.target = Coords::from_array(target);
    // Fast enough to reach orbit within the year (climb, one sector, dive).
    fleet.speed = 6;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let text = viewer_report_text(&game_data, &events, 1);
    assert!(text.contains("Salvage mission report"), "{text}");
    assert!(
        text.contains(&format!(
            "From your 3rd Fleet, located in System({},{})",
            target[0], target[1]
        )),
        "{text}"
    );
}

#[test]
fn absorbed_side_merge_index_does_not_hide_another_fleets_rendezvous_arrival() {
    let game_data = baseline_game();
    let mut events = MaintenanceEvents::default();
    // Fleet index 1 is waiting alone at its rendezvous point.
    events.mission_events.push(MissionEvent {
        fleet_idx: 1,
        owner_empire: 1,
        kind: Mission::RendezvousSector,
        outcome: MissionOutcome::Arrived,
        abort_reason: None,
        planet_idx: None,
        location_coords: Some([20, 20]),
        target_coords: Some([20, 20]),
        stardate_week: Some(1),
    });
    // Another rendezvous elsewhere absorbed a fleet whose old index was 1.
    for (fleet_idx, survivor_side) in [(1, false), (0, true)] {
        events.fleet_merge_events.push(FleetMergeEvent {
            fleet_idx,
            owner_empire: 1,
            kind: Mission::RendezvousSector,
            host_fleet_id_raw: 1,
            absorbed_fleet_id_raw: 9,
            host_fleet_number: 1,
            absorbed_fleet_number: 9,
            coords: [12, 12],
            survivor_side,
            stardate_week: Some(1),
        });
    }

    let text = viewer_report_text(&game_data, &events, 1);
    assert!(text.contains("waiting for more fleets to arrive"), "{text}");
    assert!(text.contains("absorbing fleet 9"), "{text}");
}
