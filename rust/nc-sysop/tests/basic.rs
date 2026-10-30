use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use nc_data::{BbsGameConfig, CampaignStore, SeatReservation};

static TEMP_DIR_SEQ: AtomicU64 = AtomicU64::new(0);

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

fn run_nc_sysop(args: &[&str]) -> String {
    let output = run_nc_sysop_output(args, None);

    assert!(
        output.status.success(),
        "nc-sysop failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("stdout should be utf-8")
}

fn run_nc_sysop_failure(args: &[&str]) -> String {
    let output = run_nc_sysop_output(args, None);

    assert!(
        !output.status.success(),
        "nc-sysop unexpectedly succeeded: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stderr).expect("stderr should be utf-8")
}

fn run_nc_sysop_output(args: &[&str], cwd: Option<&PathBuf>) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_nc-sysop"));
    cmd.args(args);
    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }
    cmd.output().expect("nc-sysop should run")
}

#[test]
fn nc_sysop_new_game_initializes_default_campaign_without_hosted_seats() {
    let target = unique_temp_dir("nc-sysop-new-game");

    let stdout = run_nc_sysop(&["new-game", target.to_str().expect("utf-8 path")]);
    assert!(stdout.contains("Initialized new game"));
    assert!(stdout.contains("players=4"));
    assert!(target.join("ncgame.db").exists());
    assert!(!target.join("DATABASE.DAT").exists());
    let config =
        BbsGameConfig::load_kdl(&target.join("config.kdl")).expect("new games write config.kdl");
    assert_eq!(config.autopilot_after_turns, 3);
    let text = fs::read_to_string(target.join("config.kdl")).expect("read config.kdl");
    assert!(text.contains("// players 4, seed "), "{text}");

    let runtime = CampaignStore::open_default_in_dir(&target)
        .expect("open campaign store")
        .load_latest_runtime_state()
        .expect("load runtime")
        .expect("runtime snapshot should exist");
    assert_eq!(runtime.game_data.players[0].mode.code(), 0);
    assert_eq!(fs::read_dir(&target).expect("read dir").count(), 2);

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn nc_sysop_new_game_bbs_requires_existing_config_kdl() {
    let target = unique_temp_dir("nc-sysop-new-game-bbs-missing-config");

    let stderr = run_nc_sysop_failure(&["new-game", "--bbs", target.to_str().expect("utf-8 path")]);
    assert!(stderr.contains("requires an existing config.kdl"));

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn nc_sysop_new_game_bbs_reads_minimal_config_kdl() {
    let target = unique_temp_dir("nc-sysop-new-game-bbs");
    let config = BbsGameConfig {
        reservations: vec![SeatReservation {
            player_record_index_1_based: 1,
            alias: "SYSOP".to_string(),
        }],
        ..BbsGameConfig::new()
    };
    config
        .save_kdl(&target.join("config.kdl"))
        .expect("write BBS config");

    let stdout = run_nc_sysop(&[
        "new-game",
        "--bbs",
        "--players",
        "6",
        target.to_str().expect("utf-8 path"),
    ]);
    assert!(stdout.contains("Initialized new game"));
    assert!(stdout.contains("players=6"));
    assert!(target.join("config.kdl").exists());
    assert!(target.join("ncgame.db").exists());

    let runtime = CampaignStore::open_default_in_dir(&target)
        .expect("open campaign store")
        .load_latest_runtime_state()
        .expect("load runtime")
        .expect("runtime snapshot");
    assert_eq!(runtime.game_data.conquest.game_year, 3000);
    assert_eq!(runtime.game_data.players.len(), 6);

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn nc_sysop_new_game_bbs_accepts_seed_as_creation_override() {
    let target = unique_temp_dir("nc-sysop-new-game-bbs-seed");
    let config = BbsGameConfig {
        reservations: vec![],
        ..BbsGameConfig::new()
    };
    config
        .save_kdl(&target.join("config.kdl"))
        .expect("write BBS config");

    let stdout = run_nc_sysop(&[
        "new-game",
        "--bbs",
        target.to_str().expect("utf-8 path"),
        "--seed",
        "1515",
    ]);
    assert!(stdout.contains("seed=1515"));
    let text = fs::read_to_string(target.join("config.kdl")).expect("read config.kdl");
    assert!(
        text.contains("// players 4, seed 1515, map 18x18"),
        "{text}"
    );

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn nc_sysop_maint_runs_rust_maintenance() {
    let target = unique_temp_dir("nc-sysop-maint");

    run_nc_sysop(&[
        "new-game",
        target.to_str().expect("utf-8 path"),
        "--seed",
        "1515",
    ]);

    let stdout = run_nc_sysop(&["maint", target.to_str().expect("utf-8 path"), "1"]);
    assert!(stdout.contains("Rust maintenance complete."));

    let store = CampaignStore::open_default_in_dir(&target).expect("open campaign store");
    let runtime = store
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("runtime snapshot should exist");
    assert_eq!(runtime.game_data.conquest.game_year, 3001);

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn nc_sysop_settings_show_for_bbs_campaign_omits_seed_and_lists_reservations() {
    let target = unique_temp_dir("nc-sysop-settings-show-bbs");
    let config = BbsGameConfig {
        reservations: vec![SeatReservation {
            player_record_index_1_based: 2,
            alias: "NightShade".to_string(),
        }],
        ..BbsGameConfig::new()
    };
    config
        .save_kdl(&target.join("config.kdl"))
        .expect("write BBS config");

    run_nc_sysop(&["new-game", "--bbs", target.to_str().expect("utf-8 path")]);

    let stdout = run_nc_sysop(&[
        "settings",
        "show",
        "--dir",
        target.to_str().expect("utf-8 path"),
    ]);
    assert!(stdout.contains("players=4"));
    assert!(stdout.contains("reservation seat=2 alias=NightShade"));
    assert!(!stdout.contains("seed="));

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn nc_sysop_settings_reserve_and_unreserve_work_for_bbs_campaigns() {
    let target = unique_temp_dir("nc-sysop-settings-reserve-bbs");
    BbsGameConfig {
        reservations: Vec::new(),
        ..BbsGameConfig::new()
    }
    .save_kdl(&target.join("config.kdl"))
    .expect("write BBS config");

    run_nc_sysop(&["new-game", "--bbs", target.to_str().expect("utf-8 path")]);

    let reserve_stdout = run_nc_sysop(&[
        "settings",
        "reserve",
        "--dir",
        target.to_str().expect("utf-8 path"),
        "--player",
        "1",
        "--alias",
        "SYSOP",
    ]);
    assert!(reserve_stdout.contains("Reserved seat 1"));

    let config = BbsGameConfig::load_kdl(&target.join("config.kdl")).expect("load BBS config");
    assert_eq!(
        config.reservations,
        vec![SeatReservation {
            player_record_index_1_based: 1,
            alias: "SYSOP".to_string(),
        }]
    );

    let unreserve_stdout = run_nc_sysop(&[
        "settings",
        "unreserve",
        "--dir",
        target.to_str().expect("utf-8 path"),
        "--player",
        "1",
    ]);
    assert!(unreserve_stdout.contains("Removed reservation for seat 1"));

    let config = BbsGameConfig::load_kdl(&target.join("config.kdl")).expect("reload BBS config");
    assert!(config.reservations.is_empty());

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn nc_sysop_settings_set_accepts_dir_with_other_flags_for_local_campaigns() {
    let target = unique_temp_dir("nc-sysop-settings-set-local");
    run_nc_sysop(&["new-game", target.to_str().expect("utf-8 path")]);

    let stdout = run_nc_sysop(&[
        "settings",
        "set",
        "--dir",
        target.to_str().expect("utf-8 path"),
        "--game-name",
        "Friday Night NC",
        "--inactivity-autopilot-after",
        "5",
    ]);
    assert!(stdout.contains("Updated settings"));

    let settings = CampaignStore::open_default_in_dir(&target)
        .expect("open campaign store")
        .load_campaign_settings()
        .expect("load campaign settings");
    assert_eq!(settings.game_name, "Friday Night NC");
    // The file is written too, so the next sync from it keeps the change.
    let config = BbsGameConfig::load_kdl(&target.join("config.kdl")).expect("load config");
    assert_eq!(config.game_name.as_deref(), Some("Friday Night NC"));
    assert_eq!(config.autopilot_after_turns, 5);

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn nc_sysop_help_lists_only_local_and_bbs_public_subcommands() {
    let output = run_nc_sysop_output(&["--help"], None);
    assert!(output.status.success(), "help should succeed");
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("new-game <target_dir>"));
    assert!(stdout.contains("maint <dir> [turns]"));
    assert!(stdout.contains("settings <show|set|reserve|unreserve|ai-seat|remove-ai-seat>"));
    assert!(stdout.contains("map-export --dir <game_dir> [--player <N>]"));
    assert!(stdout.contains("economy-report --dir <game_dir> --player <N>"));
    assert!(stdout.contains("rankings-export --dir <game_dir>"));
    assert!(stdout.contains("check --dir <game_dir>"));
    assert!(!stdout.contains("maint-all"));
    assert!(!stdout.contains("host "));
    assert!(!stdout.contains("nostr "));
}

#[test]
fn nc_sysop_new_game_help_does_not_treat_help_as_target_dir() {
    let cwd = unique_temp_dir("nc-sysop-help-cwd");
    let output = run_nc_sysop_output(&["new-game", "--help"], Some(&cwd));
    assert!(output.status.success(), "new-game help should succeed");
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("new-game <target_dir>"));
    assert!(!stdout.contains("Initialized new game"));
    assert!(
        !cwd.join("--help").exists(),
        "help must not create a campaign"
    );
    let _ = fs::remove_dir_all(&cwd);
}

#[test]
fn nc_sysop_removed_hosted_subcommands_fail_cleanly() {
    for command in [["maint-all"], ["host"], ["nostr"]] {
        let output = run_nc_sysop_output(&command, None);
        assert!(!output.status.success(), "{command:?} should fail");

        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
        let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
        assert!(stdout.contains("new-game <target_dir>"));
        assert!(!stdout.contains("maint-all [--config <path>]"));
        assert!(!stdout.contains("host <games|status>"));
        assert!(!stdout.contains("nostr init"));
        assert!(
            stderr.contains(&format!("unknown subcommand: {}", command[0])),
            "stderr={stderr:?}"
        );
    }
}

#[test]
fn nc_sysop_map_export_writes_the_players_star_map() {
    let target = unique_temp_dir("nc-sysop-map-export");
    let dir = target.to_str().expect("utf-8 path");
    run_nc_sysop(&["new-game", dir, "--players", "4", "--seed", "7"]);

    let stdout = run_nc_sysop(&["map-export", "--dir", dir, "--player", "2"]);
    assert!(stdout.contains("Exported player 2 star map"));
    let exports = target.join("exports");
    let text = fs::read_to_string(exports.join("ECMAP-P2-Y3000.TXT")).expect("text map");
    assert_eq!(
        text,
        nc_data::build_player_map_export_data(&target, 2)
            .expect("export data")
            .ascii_export
    );
    assert!(exports.join("ECMAP-P2-Y3000.CSV").exists());
    assert!(exports.join("ECMAP-P2-Y3000-DETAILS.CSV").exists());

    let out = target.join("maps");
    run_nc_sysop(&[
        "map-export",
        "--dir",
        dir,
        "--player",
        "1",
        "--out-dir",
        out.to_str().expect("utf-8 path"),
    ]);
    assert!(out.join("ECMAP-P1-Y3000.TXT").exists());

    let stderr = run_nc_sysop_failure(&["map-export", "--dir", dir, "--player", "5"]);
    assert!(stderr.contains("--player must be 1-4"));

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn nc_sysop_map_export_without_player_writes_the_galaxy_without_intel() {
    let target = unique_temp_dir("nc-sysop-galaxy-export");
    let dir = target.to_str().expect("utf-8 path");
    run_nc_sysop(&["new-game", dir, "--players", "4", "--seed", "7"]);
    let game = CampaignStore::open_default_in_dir(&target)
        .expect("open campaign store")
        .load_latest_runtime_state()
        .expect("load runtime")
        .expect("runtime snapshot should exist")
        .game_data;

    let stdout = run_nc_sysop(&["map-export", "--dir", dir]);
    assert!(stdout.contains("Exported the galaxy map"));
    let exports = target.join("exports");
    let text = fs::read_to_string(exports.join("ECMAP-GALAXY-Y3000.TXT")).expect("galaxy map");
    assert!(text.starts_with("NOSTRIAN CONQUEST STARMAP  YEAR 3000  GALAXY"));
    assert!(!text.contains("KNOWN WORLD DETAILS"));
    // Every world is marked, and no name is shown, owned or not.
    assert_eq!(text.matches('*').count(), game.planets.len());
    for planet in &game.planets {
        let name = planet.name.trim();
        assert!(name.is_empty() || !text.contains(name), "{name} leaked");
    }
    let csv = fs::read_to_string(exports.join("ECMAP-GALAXY-Y3000.CSV")).expect("galaxy csv");
    assert_eq!(csv.matches('*').count(), game.planets.len());
    assert!(!exports.join("ECMAP-GALAXY-Y3000-DETAILS.CSV").exists());

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn nc_sysop_economy_report_prints_one_empires_economy() {
    let target = unique_temp_dir("nc-sysop-economy-report");
    let dir = target.to_str().expect("utf-8 path");
    run_nc_sysop(&["new-game", dir, "--players", "4", "--seed", "7"]);
    let game = CampaignStore::open_default_in_dir(&target)
        .expect("open campaign store")
        .load_latest_runtime_state()
        .expect("load runtime")
        .expect("runtime snapshot should exist")
        .game_data;

    let stdout = run_nc_sysop(&["economy-report", "--dir", dir, "--player", "3"]);
    assert_eq!(stdout, game.economy_report_text(3).expect("player 3"));
    assert!(stdout.starts_with("Economy report: player=3"));
    assert!(stdout.contains(" homeworld"));

    let stderr = run_nc_sysop_failure(&["economy-report", "--dir", dir, "--player", "0"]);
    assert!(stderr.contains("--player must be 1-4"));

    let _ = fs::remove_dir_all(&target);
}

#[test]
fn nc_sysop_check_passes_a_played_campaign_and_reports_a_damaged_file() {
    let target = unique_temp_dir("nc-sysop-check");
    let dir = target.to_str().expect("utf-8 path");
    run_nc_sysop(&["new-game", dir, "--players", "4", "--seed", "7"]);
    run_nc_sysop(&["maint", dir, "2"]);

    let stdout = run_nc_sysop(&["check", "--dir", dir]);
    assert!(stdout.contains("File: intact"), "{stdout}");
    assert!(stdout.contains("Year 3002: loads (4 players"), "{stdout}");
    assert!(stdout.contains("Maintenance: ready"), "{stdout}");

    // Overwrite part of a table page, as a failing disk might.
    let db = target.join("ncgame.db");
    let mut bytes = fs::read(&db).expect("read campaign");
    let page = 4096;
    for byte in &mut bytes[page * 3..page * 3 + 512] {
        *byte = 0xA5;
    }
    fs::write(&db, bytes).expect("damage campaign");
    let stderr = run_nc_sysop_failure(&["check", "--dir", dir]);
    assert!(stderr.contains("damaged"), "{stderr}");

    let empty = unique_temp_dir("nc-sysop-check-empty");
    let stderr = run_nc_sysop_failure(&["check", "--dir", empty.to_str().expect("utf-8 path")]);
    assert!(stderr.contains("has no campaign"), "{stderr}");
    assert!(
        !empty.join("ncgame.db").exists(),
        "check must not create a campaign"
    );

    let _ = fs::remove_dir_all(&target);
    let _ = fs::remove_dir_all(&empty);
}

#[test]
fn nc_sysop_rankings_export_writes_kdl_only_when_asked() {
    let target = unique_temp_dir("nc-sysop-rankings-export");
    let dir = target.to_str().expect("utf-8 path");
    run_nc_sysop(&["new-game", dir, "--players", "4", "--seed", "7"]);
    run_nc_sysop(&["maint", dir]);
    assert!(
        !target.join("RANKINGS.TXT").exists(),
        "maintenance writes no rankings"
    );

    let stdout = run_nc_sysop(&["rankings-export", "--dir", dir]);
    assert!(stdout.contains("Exported rankings"));
    let text =
        fs::read_to_string(target.join("exports/RANKINGS-Y3001.kdl")).expect("rankings file");
    let doc: kdl::KdlDocument = text.parse().expect("valid KDL");
    let rankings = doc.get("rankings").expect("rankings node");
    assert_eq!(
        rankings.get("year").and_then(|v| v.as_integer()),
        Some(3001)
    );
    let empires = rankings.children().expect("empires").nodes();
    assert_eq!(empires.len(), 4);
    for (rank, empire) in empires.iter().enumerate() {
        assert_eq!(
            empire.get("rank").and_then(|v| v.as_integer()),
            Some(rank as i128 + 1)
        );
        assert!(empire.get("production").is_some() && empire.get("prev-production").is_some());
    }
    let _ = fs::remove_dir_all(&target);
}
