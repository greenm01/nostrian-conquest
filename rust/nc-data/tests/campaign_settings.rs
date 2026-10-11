use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use nc_data::{CampaignSettings, CampaignStore, CoreGameData, SeatReservation};
use nc_engine::build_seeded_new_game;

static TEMP_DIR_SEQ: AtomicU64 = AtomicU64::new(0);

fn temp_dir(prefix: &str) -> PathBuf {
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

fn seeded_store(prefix: &str) -> (PathBuf, CampaignStore, CoreGameData) {
    let root = temp_dir(prefix);
    let store = CampaignStore::open_default_in_dir(&root).expect("open campaign store");
    let game_data = build_seeded_new_game(4, 3000, 1515).expect("seeded game");
    store
        .save_runtime_state_structured(
            game_data.conquest.game_year,
            &game_data,
            &BTreeSet::new(),
            &[],
            &[],
        )
        .expect("save runtime state");
    (root, store, game_data)
}

#[test]
fn campaign_settings_round_trip_and_update_runtime_policy_bytes() {
    let (root, store, _) = seeded_store("nc-data-settings-roundtrip");

    let settings = CampaignSettings {
        slug: "friday-night".to_string(),
        game_name: "Friday Night EC".to_string(),
        default_theme_key: "matrix".to_string(),
        snoop_enabled: false,
        session_max_idle_minutes: 7,
        session_minimum_time_minutes: 9,
        session_local_timeout: true,
        session_remote_timeout: false,
        inactivity_autopilot_after_turns: 3,
        maintenance_enabled: true,
        maintenance_interval_minutes: 30,
        maintenance_next_due_unix_seconds: Some(12_345),
        reservations: vec![SeatReservation {
            player_record_index_1_based: 2,
            alias: "Sysop".to_string(),
        }],
    };
    store
        .save_campaign_settings(&settings)
        .expect("save settings");

    let loaded = store.load_campaign_settings().expect("load settings");
    assert_eq!(loaded, settings);

    let runtime = store
        .load_latest_runtime_state()
        .expect("load runtime")
        .expect("runtime snapshot");
    assert!(!runtime.game_data.setup.snoop);
    assert_eq!(runtime.game_data.setup.max_key_gap_minutes, 7);
    assert_eq!(runtime.game_data.setup.minimum_time_minutes, 9);
    assert!(runtime.game_data.setup.local_timeout);
    assert!(!runtime.game_data.setup.remote_timeout);
    assert_eq!(runtime.game_data.setup.autopilot_after_turns, 3);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn campaign_settings_validate_duplicate_reservations() {
    let err = CampaignSettings {
        slug: "friday-night".to_string(),
        game_name: "Friday Night EC".to_string(),
        default_theme_key: "tokyo_night".to_string(),
        snoop_enabled: true,
        session_max_idle_minutes: 10,
        session_minimum_time_minutes: 0,
        session_local_timeout: false,
        session_remote_timeout: true,
        inactivity_autopilot_after_turns: 0,
        maintenance_enabled: false,
        maintenance_interval_minutes: 60,
        maintenance_next_due_unix_seconds: None,
        reservations: vec![
            SeatReservation {
                player_record_index_1_based: 1,
                alias: "sysop".to_string(),
            },
            SeatReservation {
                player_record_index_1_based: 2,
                alias: "Sysop".to_string(),
            },
        ],
    }
    .validate()
    .expect_err("duplicate aliases should be rejected");

    assert!(format!("{err}").contains("duplicate reservation alias"));
}

#[test]
fn campaign_settings_new_defaults_inactivity_autopilot_to_three_turns() {
    let settings = CampaignSettings::new("friday-night", "Friday Night EC");
    assert_eq!(settings.inactivity_autopilot_after_turns, 3);
}

#[test]
fn latest_player_activity_states_derives_from_last_run_year_when_sidecar_missing() {
    let (root, store, mut game_data) = seeded_store("nc-data-settings-activity-derive");
    game_data.players[0].last_year_on = 2998;
    store
        .save_runtime_state_structured(
            game_data.conquest.game_year,
            &game_data,
            &BTreeSet::new(),
            &[],
            &[],
        )
        .expect("save runtime state");

    let conn = rusqlite::Connection::open(root.join("ncgame.db")).expect("open sqlite db");
    conn.execute("DELETE FROM player_activity", [])
        .expect("delete player activity");

    let activity = store
        .latest_player_activity_states(game_data.conquest.player_count)
        .expect("load player activity");
    assert_eq!(activity[0].last_participation_year, 2998);
    assert!(!activity[0].inactivity_autopilot_pending_clear);

    let _ = fs::remove_dir_all(root);
}
