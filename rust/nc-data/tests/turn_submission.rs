use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use nc_compat::import_directory_snapshot;
use nc_data::{
    CampaignStore, DiplomaticRelation, FleetTurnAction, Order, PlanetPlayerInputValidationError,
    TurnSubmission,
};
use nc_engine::build_seeded_initialized_game;

static TEMP_DIR_SEQ: AtomicU64 = AtomicU64::new(0);

#[test]
fn turn_submission_applies_mixed_player_actions() {
    let mut data = build_seeded_initialized_game(4, 3000, 1515).unwrap();
    let planet_record_index_1_based = data
        .planets
        .iter()
        .position(|planet| planet.owner_empire_slot_raw() == 1)
        .map(|idx| idx + 1)
        .unwrap();
    let fleet_indexes = data
        .fleets
        .iter()
        .enumerate()
        .filter(|(_, fleet)| fleet.owner_empire_raw() == 1)
        .map(|(idx, _)| idx + 1)
        .collect::<Vec<_>>();
    let fleet_record_index_1_based = fleet_indexes[0];
    let host_fleet_record_index_1_based = fleet_indexes[1];
    let target_coords = data.planets[planet_record_index_1_based - 1].coords_raw();
    let host_destroyers_before = data.fleets[host_fleet_record_index_1_based - 1].destroyer_count();

    {
        let fleet = &mut data.fleets[fleet_record_index_1_based - 1];
        fleet.set_destroyer_count(2);
        fleet.set_scout_count(1);
        fleet.set_troop_transport_count(2);
        fleet.set_army_count(0);
        fleet.recompute_max_speed_from_composition();
        fleet.set_current_speed(0);
    }
    {
        let host = &mut data.fleets[host_fleet_record_index_1_based - 1];
        host.set_current_location_coords_raw(target_coords);
    }
    {
        let planet = &mut data.planets[planet_record_index_1_based - 1];
        planet.set_stardock_count_raw(0, 2);
        planet.set_stored_production_points(50);
    }

    let kdl = format!(
        r#"
turn player=1 year=3000
tax rate=41
diplomacy to=2 relation="enemy"
planet record={planet_record_index_1_based} {{
  rename name="New Aurora"
  clear_build_queue
  build points=15 kind="scout"
  commission slot=1
}}
fleet record={fleet_record_index_1_based} {{
  roe value=4
  order speed=3 kind="scout_system" x={target_x} y={target_y}
  transfer to={host_fleet_record_index_1_based} destroyers=1
  load_armies planet={planet_record_index_1_based} qty=2
  unload_armies planet={planet_record_index_1_based} qty=1
}}
message to=2 subject="Border" body="Holding lane."
"#,
        target_x = target_coords[0],
        target_y = target_coords[1],
    );

    let submission = TurnSubmission::parse_kdl_str(&kdl).unwrap();
    let mut queued_mail = Vec::new();
    let report = submission.apply_to(&mut data, &mut queued_mail).unwrap();

    assert!(report.tax_changed);
    assert_eq!(report.diplomacy_updates, 1);
    assert_eq!(report.planet_blocks, 1);
    assert_eq!(report.fleet_blocks, 1);
    assert_eq!(report.messages_queued, 1);

    assert_eq!(data.players[0].tax_rate(), 41);
    assert_eq!(
        data.stored_diplomatic_relation(1, 2),
        Some(DiplomaticRelation::Enemy)
    );
    assert_eq!(
        data.planets[planet_record_index_1_based - 1].planet_name(),
        "New Aurora"
    );
    assert_eq!(
        data.planets[planet_record_index_1_based - 1].build_count_raw(0),
        15
    );
    assert_eq!(
        data.planets[planet_record_index_1_based - 1].build_kind_raw(0),
        4
    );
    assert_eq!(
        data.planets[planet_record_index_1_based - 1].stardock_kind_raw(0),
        0
    );
    assert_eq!(
        data.fleets[fleet_record_index_1_based - 1].standing_order_kind(),
        Order::ScoutSolarSystem
    );
    assert_eq!(
        data.fleets[fleet_record_index_1_based - 1].destroyer_count(),
        1
    );
    assert_eq!(data.fleets[fleet_record_index_1_based - 1].army_count(), 1);
    assert_eq!(
        data.fleets[host_fleet_record_index_1_based - 1].destroyer_count(),
        host_destroyers_before + 1
    );
    assert_eq!(queued_mail.len(), 1);
    assert_eq!(queued_mail[0].sender_empire_id, 1);
    assert_eq!(queued_mail[0].recipient_empire_id, 2);
    assert_eq!(queued_mail[0].subject, "Border");
    assert_eq!(queued_mail[0].body, "Holding lane.");
    assert!(data.fleets.len() > fleet_indexes.len());
}

#[test]
fn turn_submission_rejects_year_mismatch() {
    let mut data = build_seeded_initialized_game(4, 3000, 1515).unwrap();
    let submission = TurnSubmission::parse_kdl_str(
        r#"
turn player=1 year=3001
tax rate=20
"#,
    )
    .unwrap();

    let err = submission
        .apply_to(&mut data, &mut Vec::new())
        .expect_err("mismatched year should fail");
    assert!(err.to_string().contains("turn year mismatch"));
}

#[test]
fn turn_submission_rejects_non_ascii_planet_rename() {
    let mut data = build_seeded_initialized_game(4, 3000, 1515).unwrap();
    let planet_record_index_1_based = data
        .planets
        .iter()
        .position(|planet| planet.owner_empire_slot_raw() == 1)
        .map(|idx| idx + 1)
        .unwrap();
    let before = data.planets[planet_record_index_1_based - 1].planet_name();
    let submission = TurnSubmission::parse_kdl_str(&format!(
        r#"
turn player=1 year=3000
planet record={planet_record_index_1_based} {{
  rename name="😀😀😀😀"
}}
"#
    ))
    .unwrap();

    let err = submission
        .apply_to(&mut data, &mut Vec::new())
        .expect_err("non-ASCII planet name should be rejected");
    assert!(err.to_string().contains("printable ASCII"), "{err}");
    assert_eq!(
        data.planets[planet_record_index_1_based - 1].planet_name(),
        before
    );
}

#[test]
fn turn_submission_clear_build_kind_round_trips_and_clears_only_selected_kind() {
    let mut data = build_seeded_initialized_game(4, 3000, 1515).unwrap();
    let planet_record_index_1_based = data
        .planets
        .iter()
        .position(|planet| planet.owner_empire_slot_raw() == 1)
        .map(|idx| idx + 1)
        .unwrap();
    let planet = &mut data.planets[planet_record_index_1_based - 1];
    planet.set_build_kind_raw(0, 1);
    planet.set_build_count_raw(0, 10);
    planet.set_build_kind_raw(1, 2);
    planet.set_build_count_raw(1, 30);

    let submission = TurnSubmission::parse_kdl_str(&format!(
        r#"
turn player=1 year=3000
planet record={planet_record_index_1_based} {{
  clear_build_kind kind="destroyer"
}}
"#
    ))
    .expect("parse clear_build_kind");

    assert_eq!(
        TurnSubmission::parse_kdl_str(&submission.to_kdl_string()).expect("round-trip parse"),
        submission
    );

    submission
        .apply_to(&mut data, &mut Vec::new())
        .expect("apply clear_build_kind");

    let planet = &data.planets[planet_record_index_1_based - 1];
    assert_eq!(planet.build_kind_raw(0), 0);
    assert_eq!(planet.build_count_raw(0), 0);
    assert_eq!(planet.build_kind_raw(1), 2);
    assert_eq!(planet.build_count_raw(1), 30);
}

#[test]
fn turn_submission_remove_build_round_trips_and_removes_one_unit_of_selected_kind() {
    let mut data = build_seeded_initialized_game(4, 3000, 1515).unwrap();
    let planet_record_index_1_based = data
        .planets
        .iter()
        .position(|planet| planet.owner_empire_slot_raw() == 1)
        .map(|idx| idx + 1)
        .unwrap();
    let planet = &mut data.planets[planet_record_index_1_based - 1];
    planet.set_build_kind_raw(0, 1);
    planet.set_build_count_raw(0, 15);
    planet.set_build_kind_raw(1, 2);
    planet.set_build_count_raw(1, 30);

    let submission = TurnSubmission::parse_kdl_str(&format!(
        r#"
turn player=1 year=3000
planet record={planet_record_index_1_based} {{
  remove_build kind="destroyer" qty=1
}}
"#
    ))
    .expect("parse remove_build");

    assert_eq!(
        TurnSubmission::parse_kdl_str(&submission.to_kdl_string()).expect("round-trip parse"),
        submission
    );

    submission
        .apply_to(&mut data, &mut Vec::new())
        .expect("apply remove_build");

    let planet = &data.planets[planet_record_index_1_based - 1];
    assert_eq!(planet.build_kind_raw(0), 1);
    assert_eq!(planet.build_count_raw(0), 10);
    assert_eq!(planet.build_kind_raw(1), 2);
    assert_eq!(planet.build_count_raw(1), 30);
}

#[test]
fn turn_submission_rejects_fleet_targets_outside_the_map() {
    let data = build_seeded_initialized_game(4, 3000, 1515).unwrap();
    let fleet_index = data
        .fleets
        .iter()
        .position(|fleet| fleet.owner_empire_raw() == 1)
        .map(|idx| idx + 1)
        .expect("empire 1 should have a fleet");
    let submit_move = |x: u8, y: u8| {
        let kdl = format!(
            "turn player=1 year=3000\nfleet record={fleet_index} {{\n  order speed=1 kind=\"move\" x={x} y={y}\n}}\n"
        );
        let mut data = data.clone();
        TurnSubmission::parse_kdl_str(&kdl)
            .unwrap()
            .apply_to(&mut data, &mut Vec::new())
    };

    // A 4-player map is 18x18 with 1-based sectors.
    for (x, y) in [(0, 5), (5, 0), (19, 5), (5, 19), (255, 255)] {
        let err = submit_move(x, y).expect_err("out-of-map target should be rejected");
        assert!(
            err.to_string().contains("outside the map"),
            "({x},{y}): {err}"
        );
    }
    submit_move(18, 18).expect("map corner should be a valid target");
    submit_move(1, 1).expect("map origin should be a valid target");
}

#[test]
fn turn_submission_rejects_record_index_zero_at_parse_time() {
    for (block, expected) in [
        (
            "planet record=0 {\n  rename name=\"Zero\"\n}",
            "planet.record",
        ),
        ("fleet record=0 {\n  roe value=3\n}", "fleet.record"),
        ("fleet record=1 {\n  join host=0\n}", "join.host"),
        (
            "fleet record=1 {\n  transfer to=0 destroyers=1\n}",
            "transfer.to",
        ),
        (
            "fleet record=1 {\n  load_armies planet=0 qty=1\n}",
            "load_armies.planet",
        ),
        (
            "fleet record=1 {\n  unload_armies planet=0 qty=1\n}",
            "unload_armies.planet",
        ),
    ] {
        let kdl = format!("turn player=1 year=3000\n{block}\n");
        let err = TurnSubmission::parse_kdl_str(&kdl).expect_err(block);
        assert!(
            err.to_string()
                .contains(&format!("{expected} must be 1-based")),
            "{block}: {err}"
        );
    }
}

#[test]
fn turn_submission_rejects_duplicate_friendly_colonize_targets_without_mutation() {
    let mut data = build_seeded_initialized_game(4, 3000, 1515).unwrap();
    let fleet_indexes = data
        .fleets
        .iter()
        .enumerate()
        .filter(|(_, fleet)| fleet.owner_empire_raw() == 1)
        .map(|(idx, _)| idx + 1)
        .take(2)
        .collect::<Vec<_>>();
    let target_coords = data
        .planets
        .iter()
        .find(|planet| planet.owner_empire_slot_raw() == 0)
        .map(|planet| planet.coords_raw())
        .expect("initialized game should have an unowned planet");
    let first_order_before = data.fleets[fleet_indexes[0] - 1].standing_order_kind();
    let second_order_before = data.fleets[fleet_indexes[1] - 1].standing_order_kind();

    for fleet_index in &fleet_indexes {
        let fleet = &mut data.fleets[*fleet_index - 1];
        fleet.set_etac_count(1);
        fleet.recompute_max_speed_from_composition();
        fleet.set_current_speed(0);
    }

    let kdl = format!(
        r#"
turn player=1 year=3000
fleet record={} {{
  order speed=0 kind="colonize" x={} y={}
}}
fleet record={} {{
  order speed=0 kind="colonize" x={} y={}
}}
"#,
        fleet_indexes[0],
        target_coords[0],
        target_coords[1],
        fleet_indexes[1],
        target_coords[0],
        target_coords[1],
    );

    let submission = TurnSubmission::parse_kdl_str(&kdl).unwrap();
    let err = submission
        .apply_to(&mut data, &mut Vec::new())
        .expect_err("duplicate colonize targets should fail");

    assert!(err.to_string().contains("friendly fleet"));
    assert_eq!(
        data.fleets[fleet_indexes[0] - 1].standing_order_kind(),
        first_order_before
    );
    assert_eq!(
        data.fleets[fleet_indexes[1] - 1].standing_order_kind(),
        second_order_before
    );
}

#[test]
fn turn_submission_rejects_non_multiple_ship_build_points() {
    let mut data = build_seeded_initialized_game(4, 3000, 1515).unwrap();
    let planet_record_index_1_based = data
        .planets
        .iter()
        .position(|planet| planet.owner_empire_slot_raw() == 1)
        .map(|idx| idx + 1)
        .unwrap();
    data.planets[planet_record_index_1_based - 1].set_stored_production_points(50);

    let submission = TurnSubmission::parse_kdl_str(&format!(
        r#"
turn player=1 year=3000
planet record={planet_record_index_1_based} {{
  build points=14 kind="scout"
}}
"#
    ))
    .unwrap();

    let err = submission
        .apply_to(&mut data, &mut Vec::new())
        .expect_err("invalid build points should fail");

    match err {
        nc_data::TurnSubmissionError::Mutation(
            nc_data::GameStateMutationError::InvalidPlanetPlayerInput {
                planet_index_1_based,
                reason:
                    PlanetPlayerInputValidationError::InvalidBuildPointsForKind {
                        kind_raw,
                        points_remaining_raw,
                    },
            },
        ) => {
            assert_eq!(planet_index_1_based, planet_record_index_1_based);
            assert_eq!(kind_raw, 4);
            assert_eq!(points_remaining_raw, 14);
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn turn_submission_rejects_build_points_beyond_planet_budget() {
    let mut data = build_seeded_initialized_game(4, 3000, 1515).unwrap();
    let planet_record_index_1_based = 2;
    data.planets[planet_record_index_1_based - 1].set_stored_production_points(0);

    let submission = TurnSubmission::parse_kdl_str(&format!(
        r#"
turn player=2 year=3000
planet record={planet_record_index_1_based} {{
  build points=100 kind="destroyer"
}}
"#
    ))
    .unwrap();

    let err = submission
        .apply_to(&mut data, &mut Vec::new())
        .expect_err("build beyond an empty treasury should fail");
    assert!(
        matches!(err, nc_data::TurnSubmissionError::Validation(_)),
        "unexpected error: {err:?}"
    );
    assert_eq!(
        data.planets[planet_record_index_1_based - 1].build_count_raw(0),
        0
    );
}

#[test]
fn turn_submission_build_budget_counts_points_already_queued() {
    let mut data = build_seeded_initialized_game(4, 3000, 1515).unwrap();
    let planet_record_index_1_based = 2;
    // Production 100 gives capacity 100; a treasury of 60 makes the budget 60.
    data.planets[planet_record_index_1_based - 1].set_stored_production_points(60);

    let within_budget = TurnSubmission::parse_kdl_str(&format!(
        r#"
turn player=2 year=3000
planet record={planet_record_index_1_based} {{
  build points=40 kind="destroyer"
  build points=20 kind="ground_battery"
}}
"#
    ))
    .unwrap();
    within_budget
        .apply_to(&mut data, &mut Vec::new())
        .expect("orders within the budget should apply");

    let over_budget = TurnSubmission::parse_kdl_str(&format!(
        r#"
turn player=2 year=3000
planet record={planet_record_index_1_based} {{
  build points=2 kind="army"
}}
"#
    ))
    .unwrap();
    let err = over_budget
        .apply_to(&mut data, &mut Vec::new())
        .expect_err("queued points already use the whole budget");
    assert!(
        matches!(err, nc_data::TurnSubmissionError::Validation(_)),
        "unexpected error: {err:?}"
    );
}

#[test]
fn turn_submission_kdl_renderer_round_trips_supported_actions() {
    let submission = TurnSubmission::parse_kdl_str(
        r#"
turn player=1 year=3004
tax rate=41
diplomacy to=2 relation="enemy"
planet record=7 {
  clear_build_queue
  build points=15 kind="scout"
}
fleet record=3 {
  roe value=4
  order speed=5 kind="scout_system" x=8 y=9
  transfer to=4 destroyers=1 scouts=1
}
message to=2 subject="Border" body="Holding lane."
"#,
    )
    .expect("parse source submission");

    let rendered = submission.to_kdl_string();
    let reparsed = TurnSubmission::parse_kdl_str(&rendered).expect("reparse rendered kdl");

    assert_eq!(reparsed, submission);
}

#[test]
fn turn_submission_detach_defaults_the_new_fleet_roe_to_six() {
    let submission = TurnSubmission::parse_kdl_str(
        "turn player=1 year=3004\nfleet record=3 {\n  detach destroyers=1\n}\n",
    )
    .expect("parse detach without new_roe");
    assert!(matches!(
        submission.fleets[0].actions[0],
        FleetTurnAction::Detach {
            new_fleet_roe: 6,
            ..
        }
    ));
}

#[test]
fn turn_submission_loads_from_kdl_file() {
    let dir = std::env::temp_dir().join("nc-data-turn-kdl-load");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("turn.kdl");
    std::fs::write(
        &path,
        r#"
turn player=1 year=3000
message to=2 body="hello"
"#,
    )
    .unwrap();

    let submission = TurnSubmission::load_kdl(&path).unwrap();
    assert_eq!(submission.player_record_index_1_based, 1);
    assert_eq!(submission.year, 3000);
    assert_eq!(submission.messages.len(), 1);

    let _ = CampaignStore::open_default_in_dir(&dir);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn turn_submission_runtime_helper_check_only_leaves_the_store_unchanged() {
    let dir = imported_fixture_store("nc-data-submit-turn-check");
    let path = dir.join("turn.kdl");
    fs::write(
        &path,
        r#"
turn player=1 year=3000
tax rate=42
"#,
    )
    .unwrap();
    let store = CampaignStore::open_default_in_dir(&dir).unwrap();
    let before = store.load_latest_runtime_state().unwrap().unwrap();

    let report = TurnSubmission::submit_kdl_file_to_campaign_dir(&dir, 1, &path, true).unwrap();

    assert_eq!(report.player_record_index_1_based, 1);
    assert!(report.tax_changed);
    let after = store.load_latest_runtime_state().unwrap().unwrap();
    assert_eq!(after.snapshot_id, before.snapshot_id);
    assert_eq!(after.game_data, before.game_data);

    cleanup_dir(&dir);
}

#[test]
fn turn_submission_runtime_helper_apply_updates_the_store() {
    let dir = imported_fixture_store("nc-data-submit-turn-apply");
    let path = dir.join("turn.kdl");
    fs::write(
        &path,
        r#"
turn player=1 year=3000
tax rate=37
message to=2 subject="Scout" body="Watch the lane."
"#,
    )
    .unwrap();

    let report = TurnSubmission::submit_kdl_file_to_campaign_dir(&dir, 1, &path, false).unwrap();

    assert_eq!(report.messages_queued, 1);
    let store = CampaignStore::open_default_in_dir(&dir).unwrap();
    let state = store.load_latest_runtime_state().unwrap().unwrap();
    assert_eq!(state.game_data.players[0].tax_rate(), 37);
    assert_eq!(state.queued_mail.len(), 1);

    cleanup_dir(&dir);
}

#[test]
fn turn_submission_runtime_helper_rejects_a_directory_without_a_store() {
    // Classic files alone are not a campaign: the runtime reads the store.
    let dir = fixture_copy("nc-data-submit-turn-no-store");
    let path = dir.join("turn.kdl");
    fs::write(
        &path,
        r#"
turn player=1 year=3000
tax rate=37
"#,
    )
    .unwrap();

    for check_only in [true, false] {
        let err = TurnSubmission::submit_kdl_file_to_campaign_dir(&dir, 1, &path, check_only)
            .expect_err("a directory without ncgame.db should be rejected");
        assert!(
            err.to_string().contains("campaign store not found"),
            "{err}"
        );
    }
    assert!(!dir.join("ncgame.db").exists());

    cleanup_dir(&dir);
}

#[test]
fn turn_submission_runtime_helper_clears_inactivity_auto_enabled_autopilot() {
    let dir = fixture_copy("nc-data-submit-turn-autopilot");
    let store = CampaignStore::open_default_in_dir(&dir).unwrap();
    import_directory_snapshot(&store, &dir).unwrap();

    let mut state = store.load_latest_runtime_state().unwrap().unwrap();
    state.game_data.join_player(1, "Codex Dominion").unwrap();
    state
        .game_data
        .rename_player_homeworld(1, "Codex Prime")
        .unwrap();
    state.game_data.players[0].set_autopilot_flag(1);
    state.game_data.players[0].set_last_run_year_raw(2997);
    let planet_intel_by_viewer = (1..=state.game_data.conquest.player_count())
        .map(|viewer_empire_id| {
            store
                .latest_planet_intel_for_viewer(viewer_empire_id)
                .unwrap()
                .into_iter()
                .map(|snapshot| (snapshot.planet_record_index_1_based, snapshot))
                .collect::<std::collections::BTreeMap<_, _>>()
        })
        .collect::<Vec<_>>();
    let mut player_activity_states = store
        .latest_player_activity_states(state.game_data.conquest.player_count())
        .unwrap();
    player_activity_states[0].last_participation_year = 2997;
    player_activity_states[0].inactivity_autopilot_pending_clear = true;
    store
        .save_runtime_state_structured_with_intel_and_activity(
            &state.game_data,
            &state.planet_scorch_orders,
            &state.report_block_rows,
            &state.queued_mail,
            &planet_intel_by_viewer,
            &player_activity_states,
        )
        .unwrap();

    let path = dir.join("turn.kdl");
    fs::write(
        &path,
        r#"
turn player=1 year=3000
tax rate=37
"#,
    )
    .unwrap();

    TurnSubmission::submit_kdl_file_to_campaign_dir(&dir, 1, &path, false).unwrap();

    let state = store.load_latest_runtime_state().unwrap().unwrap();
    assert_eq!(state.game_data.players[0].tax_rate(), 37);
    assert_eq!(state.game_data.players[0].autopilot_flag(), 0);
    assert_eq!(state.game_data.players[0].last_run_year_raw(), 2997);
    let activity = store
        .latest_player_activity_states(state.game_data.conquest.player_count())
        .unwrap();
    assert_eq!(activity[0].last_participation_year, 3000);
    assert!(!activity[0].inactivity_autopilot_pending_clear);

    cleanup_dir(&dir);
}

#[test]
fn turn_submission_rejects_fourth_message_to_same_recipient_in_same_year() {
    let mut data = build_seeded_initialized_game(4, 3000, 1515).unwrap();
    let mut queued_mail = vec![
        nc_data::QueuedPlayerMail {
            sender_empire_id: 1,
            recipient_empire_id: 2,
            year: 3000,
            subject: "One".to_string(),
            body: "Queued".to_string(),
            recipient_deleted: false,
        },
        nc_data::QueuedPlayerMail {
            sender_empire_id: 1,
            recipient_empire_id: 2,
            year: 3000,
            subject: "Two".to_string(),
            body: "Queued".to_string(),
            recipient_deleted: false,
        },
        nc_data::QueuedPlayerMail {
            sender_empire_id: 1,
            recipient_empire_id: 2,
            year: 3000,
            subject: "Three".to_string(),
            body: "Queued".to_string(),
            recipient_deleted: false,
        },
    ];
    let submission = TurnSubmission::parse_kdl_str(
        r#"
turn player=1 year=3000
message to=2 subject="Four" body="Blocked"
"#,
    )
    .unwrap();

    let err = submission
        .apply_to(&mut data, &mut queued_mail)
        .expect_err("4th message should be rejected");
    assert!(
        err.to_string()
            .contains("You may only queue 3 messages to Empire 2 this turn.")
    );
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "{prefix}-{}-{}-{}",
        std::process::id(),
        TEMP_DIR_SEQ.fetch_add(1, Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time ok")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("create temp dir");
    root
}

fn fixture_copy(prefix: &str) -> PathBuf {
    let root = unique_temp_dir(prefix);
    copy_dir_files(&repo_root().join("fixtures/ecutil-init/v1.5"), &root);
    root
}

/// A campaign directory holding only a store seeded from the ECUTIL init
/// fixture.
fn imported_fixture_store(prefix: &str) -> PathBuf {
    let dir = unique_temp_dir(prefix);
    let store = CampaignStore::open_default_in_dir(&dir).unwrap();
    import_directory_snapshot(&store, &repo_root().join("fixtures/ecutil-init/v1.5")).unwrap();
    dir
}

fn copy_dir_files(source: &Path, target: &Path) {
    fs::create_dir_all(target).expect("create target dir");
    for entry in fs::read_dir(source).expect("read source dir") {
        let entry = entry.expect("dir entry");
        if !entry.file_type().expect("file type").is_file() {
            continue;
        }
        fs::copy(entry.path(), target.join(entry.file_name())).expect("copy file");
    }
}

fn cleanup_dir(path: &Path) {
    let _ = fs::remove_dir_all(path);
}
