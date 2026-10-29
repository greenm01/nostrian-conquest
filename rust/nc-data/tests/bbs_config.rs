use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use nc_data::{
    BbsGameConfig, BbsGameConfigError, CampaignSettings, CampaignStore, SeatReservation,
    sync_game_config,
};

static TEMP_DIR_SEQ: AtomicU64 = AtomicU64::new(0);

fn temp_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "{label}-{}-{}-{}",
        std::process::id(),
        TEMP_DIR_SEQ.fetch_add(1, Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time ok")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[test]
fn parses_reservations() {
    let kdl = r#"
reservations {
    seat player=1 alias="SYSOP"
    seat player=2 alias="NightShade"
}
"#;

    let config = BbsGameConfig::parse_kdl_str(kdl).expect("should parse");

    assert_eq!(
        config.reservations,
        vec![
            SeatReservation {
                player_record_index_1_based: 1,
                alias: "SYSOP".to_string(),
            },
            SeatReservation {
                player_record_index_1_based: 2,
                alias: "NightShade".to_string(),
            },
        ]
    );
}

#[test]
fn reservations_are_optional() {
    let config = BbsGameConfig::parse_kdl_str("").expect("should parse");
    assert!(config.reservations.is_empty());
}

#[test]
fn unsupported_fields_are_rejected() {
    for field in ["players", "year", "seed", "map_size"] {
        let kdl = format!("{field} 1\n");
        let err = BbsGameConfig::parse_kdl_str(&kdl).expect_err("should reject unsupported field");
        assert!(
            matches!(err, BbsGameConfigError::Parse(ref message) if message.contains(field)),
            "unexpected error: {err}"
        );
    }
}

#[test]
fn duplicate_reservation_player_is_rejected() {
    let kdl = r#"
reservations {
    seat player=1 alias="SYSOP"
    seat player=1 alias="RIVAL"
}
"#;
    let err = BbsGameConfig::parse_kdl_str(kdl).expect_err("should reject duplicate player");
    assert!(
        matches!(err, BbsGameConfigError::Parse(ref message) if message.contains("duplicate reservation for player 1")),
        "unexpected error: {err}"
    );
}

#[test]
fn duplicate_reservation_alias_is_rejected_case_insensitively() {
    let kdl = r#"
reservations {
    seat player=1 alias="Sysop"
    seat player=2 alias="SYSOP"
}
"#;
    let err = BbsGameConfig::parse_kdl_str(kdl).expect_err("should reject duplicate alias");
    assert!(
        matches!(err, BbsGameConfigError::Parse(ref message) if message.contains("duplicate reservation alias")),
        "unexpected error: {err}"
    );
}

#[test]
fn blank_reservation_alias_is_rejected() {
    let kdl = r#"
reservations {
    seat player=1 alias="   "
}
"#;
    let err = BbsGameConfig::parse_kdl_str(kdl).expect_err("should reject blank alias");
    assert!(
        matches!(err, BbsGameConfigError::Parse(ref message) if message.contains("reservation alias")),
        "unexpected error: {err}"
    );
}

#[test]
fn reservation_lookup_is_trimmed_and_case_insensitive() {
    let config = BbsGameConfig::parse_kdl_str(
        r#"
reservations {
    seat player=2 alias="NightShade"
}
"#,
    )
    .expect("should parse");

    let reservation = config
        .reservation_for_alias("  nightshade ")
        .expect("alias should resolve");
    assert_eq!(reservation.player_record_index_1_based, 2);
}

#[test]
fn reservation_player_count_validation_rejects_out_of_range_seat() {
    let config = BbsGameConfig::parse_kdl_str(
        r#"
reservations {
    seat player=5 alias="SYSOP"
}
"#,
    )
    .expect("should parse");

    let err = config
        .validate_reservations_for_player_count(4)
        .expect_err("seat should exceed player count");
    assert!(
        matches!(err, BbsGameConfigError::Parse(ref message) if message.contains("exceeds player count")),
        "unexpected error: {err}"
    );
}

#[test]
fn save_and_load_round_trip() {
    let dir = temp_dir("nc-data-bbs-config");
    let path = dir.join("config.kdl");
    let config = BbsGameConfig {
        reservations: vec![SeatReservation {
            player_record_index_1_based: 1,
            alias: "SYSOP".to_string(),
        }],
        ..BbsGameConfig::new()
    };

    config.save_kdl(&path).expect("save config");
    let loaded = BbsGameConfig::load_kdl(&path).expect("load config");

    assert_eq!(loaded, config);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn sysop_settings_round_trip_through_kdl() {
    let config = BbsGameConfig::parse_kdl_str(
        r#"
game_name "Friday Night"
snoop #false
session {
    max_idle_minutes 20
    remote_timeout #false
}
inactivity {
    autopilot_after_turns 5
}
maintenance {
    enabled #true
    interval_minutes 720
}
ai_seat player=3 difficulty="hard" personality="turtle"
"#,
    )
    .expect("should parse");
    assert_eq!(config.game_name.as_deref(), Some("Friday Night"));
    assert!(!config.snoop);
    assert_eq!(config.session_max_idle_minutes, 20);
    assert!(!config.session_remote_timeout);
    assert_eq!(config.autopilot_after_turns, 5);
    assert!(config.maintenance_enabled);
    assert_eq!(config.maintenance_interval_minutes, 720);
    assert_eq!(config.ai_seats.len(), 1);
    assert_eq!(config.ai_seats[0].personality, "turtle");
    let reparsed =
        BbsGameConfig::parse_kdl_str(&config.to_kdl_string()).expect("written config parses");
    assert_eq!(reparsed, config);
}

#[test]
fn a_bare_config_keeps_the_default_settings() {
    let config = BbsGameConfig::parse_kdl_str("").expect("should parse");
    assert_eq!(config, BbsGameConfig::new());
    assert_eq!(config.autopilot_after_turns, 3);
}

#[test]
fn out_of_range_settings_are_rejected() {
    for kdl in [
        "inactivity {\n autopilot_after_turns 101\n}\n",
        "session {\n max_idle_minutes 121\n}\n",
        "session {\n idle 5\n}\n",
        "ai_seat player=0\n",
    ] {
        assert!(
            BbsGameConfig::parse_kdl_str(kdl).is_err(),
            "accepted: {kdl}"
        );
    }
}

fn started_game(label: &str) -> (PathBuf, CampaignStore) {
    let dir = temp_dir(label);
    let store = CampaignStore::open_default_in_dir(&dir).expect("open store");
    let mut game_data = nc_engine::build_seeded_new_game(4, 3000, 1515).expect("seeded game");
    // Seat 1 has joined; seats 2-4 are open.
    game_data.players[0].mode = nc_data::PlayerMode::Human;
    store
        .save_year(game_data.conquest.game_year, &game_data, &[], &[])
        .expect("save game");
    let settings = CampaignSettings::new("sync", "Sync");
    store
        .save_campaign_settings(&settings)
        .expect("save settings");
    nc_data::write_game_config(&dir, &store, &settings).expect("write config");
    (dir, store)
}

fn sysop_reports(store: &CampaignStore) -> Vec<String> {
    store
        .load_latest_runtime_state()
        .expect("load")
        .expect("state")
        .report_block_rows
        .into_iter()
        .filter(|row| row.decoded_text.starts_with("Game Notice"))
        .map(|row| row.decoded_text)
        .collect()
}

#[test]
fn the_file_shows_the_fixed_game_for_reference_and_seats_must_exist() {
    let (dir, store) = started_game("nc-config-fixed");
    let path = dir.join("config.kdl");
    let text = std::fs::read_to_string(&path).expect("read");
    assert!(text.contains("// players 4, seed "), "{text}");
    assert!(text.contains("map 18x18"), "{text}");
    sync_game_config(&dir, &store).expect("the comment is ignored");

    std::fs::write(&path, format!("{text}ai_seat player=5\n")).expect("write");
    let err = sync_game_config(&dir, &store).expect_err("there is no seat 5");
    assert!(
        err.to_string().contains("not a seat of a 4-player game"),
        "{err}"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn changed_settings_reach_the_store_and_are_announced_once() {
    let (dir, store) = started_game("nc-config-announce");
    let mut config = BbsGameConfig::load_kdl(&dir.join("config.kdl")).expect("load");
    config.autopilot_after_turns = 5;
    config.maintenance_interval_minutes = 720;
    config.theme_key = Some("matrix".to_string());
    config.save_kdl(&dir.join("config.kdl")).expect("save");

    sync_game_config(&dir, &store).expect("sync");
    sync_game_config(&dir, &store).expect("a second sync changes nothing");

    let settings = store.load_campaign_settings().expect("settings");
    assert_eq!(settings.inactivity_autopilot_after_turns, 5);
    assert_eq!(settings.default_theme_key, "matrix");
    let reports = sysop_reports(&store);
    assert_eq!(reports.len(), 1, "{reports:?}");
    assert!(
        reports[0].contains("takes over an empire after 5 missed turns (it was 3 missed turns)")
    );
    assert!(reports[0].contains("every 12 hours (it was every day)"));
    assert!(!reports[0].contains("matrix"), "the theme is not announced");
    let state = store
        .load_latest_runtime_state()
        .expect("load")
        .expect("state");
    let inbox = nc_data::runtime_inbox_items(&state.game_data, 1, &state.report_block_rows, &[]);
    assert!(inbox.iter().any(|item| item.subject == "Notice"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_theme_change_alone_is_not_announced() {
    let (dir, store) = started_game("nc-config-quiet");
    let mut config = BbsGameConfig::load_kdl(&dir.join("config.kdl")).expect("load");
    config.theme_key = Some("matrix".to_string());
    config.save_kdl(&dir.join("config.kdl")).expect("save");
    sync_game_config(&dir, &store).expect("sync");
    assert!(sysop_reports(&store).is_empty());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_shipped_template_parses_to_the_defaults() {
    let template = concat!(env!("CARGO_MANIFEST_DIR"), "/config/config.kdl");
    let config = BbsGameConfig::load_kdl(std::path::Path::new(template)).expect("template parses");
    assert_eq!(config.autopilot_after_turns, 3);
    assert_eq!(
        BbsGameConfig {
            game_name: None,
            theme_key: None,
            ..config
        },
        BbsGameConfig::new()
    );
}

#[test]
fn the_computer_takes_only_open_seats() {
    let (dir, store) = started_game("nc-config-ai-seats");
    let path = dir.join("config.kdl");
    let mut config = BbsGameConfig::load_kdl(&path).expect("load");
    config.ai_seats = vec![nc_data::AiSeat {
        player_record_index_1_based: 1,
        difficulty: nc_data::AiDifficulty::Normal,
        personality: "land-grab".to_string(),
    }];
    config.save_kdl(&path).expect("save");
    let err = sync_game_config(&dir, &store).expect_err("seat 1 is held");
    assert!(
        err.to_string().contains("seat 1 is played by a person"),
        "{err}"
    );

    config.ai_seats[0].player_record_index_1_based = 2;
    config.save_kdl(&path).expect("save");
    sync_game_config(&dir, &store).expect("seat 2 is open");
    assert!(sysop_reports(&store)[0].contains("Empire 2 is now played by the computer."));
    // The seat claims its empire at once, so players see it on their first turn.
    let players = store
        .load_latest_runtime_state()
        .expect("load")
        .expect("state")
        .game_data
        .players;
    assert_eq!(players[1].mode, nc_data::PlayerMode::Human);
    assert_eq!(players[1].empire_name, "AI Empire 2");
    assert_eq!(players[2].mode, nc_data::PlayerMode::CivilDisorder);
    sync_game_config(&dir, &store).expect("a joined AI seat syncs again");
    let _ = std::fs::remove_dir_all(dir);
}
