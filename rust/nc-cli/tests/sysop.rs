mod common;

use common::{
    cleanup_dir, run_classic_ecgame_smoke, run_classic_ecgame_smoke_with_alias, run_nc_cli,
    run_nc_cli_failure_in_dir, run_nc_cli_in_dir, unique_temp_dir,
};
use nc_compat::load_classic_core_files;

#[test]
fn sysop_new_game_default_four_player() {
    let target = unique_temp_dir("nc-cli-sysop-init");
    let stdout = run_nc_cli(&["sysop", "new-game", target.to_str().unwrap()]);
    assert!(stdout.contains("Initialized new game"));
    assert!(stdout.contains("players=4"));
    assert!(target.join("DATABASE.DAT").exists());
    let game_data = load_classic_core_files(&target).expect("generated game should load");
    assert_eq!(game_data.player.records[0].owner_mode_raw(), 0);
    assert_eq!(game_data.planets.records[0].planet_name(), "Not Named Yet");
    assert_eq!(game_data.player.records[0].tax_rate(), 50);
    assert_eq!(
        game_data.planets.records[0].present_production_points(),
        Some(100)
    );
    assert!(target.join("ECGAME.EXE").exists());
    assert!(target.join("ECMAINT.EXE").exists());
    assert!(target.join("ECUTIL.EXE").exists());
    cleanup_dir(&target);
}

#[test]
#[ignore = "launches classic ECGAME through dosbox-x"]
fn sysop_new_game_launches_classic_ecgame_smoke() {
    let target = unique_temp_dir("nc-cli-sysop-new-game-ecgame");
    let stdout = run_nc_cli(&["sysop", "new-game", target.to_str().unwrap()]);
    assert!(stdout.contains("Initialized new game"));

    run_classic_ecgame_smoke(&target, 1);

    cleanup_dir(&target);
}

#[test]
#[ignore = "launches classic ECGAME through dosbox-x"]
fn classic_login_prepare_supports_matched_preloaded_ecgame_smoke() {
    let target = unique_temp_dir("nc-cli-classic-login-preloaded-ecgame");
    let stdout = run_nc_cli(&["sysop", "new-game", target.to_str().unwrap()]);
    assert!(stdout.contains("Initialized new game"));

    let prepare = run_nc_cli(&[
        "classic-login-prepare",
        target.to_str().unwrap(),
        "2",
        "SYSOP",
        "foo",
    ]);
    assert!(prepare.contains("Prepared classic login for player 2"));

    run_classic_ecgame_smoke_with_alias(&target, 2, "SYSOP");

    cleanup_dir(&target);
}

#[test]
#[ignore = "launches classic ECGAME through dosbox-x"]
fn hybrid_campaign_loop_reopens_classic_client_after_rust_maintenance() {
    let target = unique_temp_dir("nc-cli-hybrid-campaign-loop");

    let stdout = run_nc_cli(&["sysop", "new-game", target.to_str().unwrap()]);
    assert!(stdout.contains("Initialized new game"));

    run_classic_ecgame_smoke(&target, 1);

    let prepare = run_nc_cli(&[
        "classic-login-prepare",
        target.to_str().unwrap(),
        "1",
        "SYSOP",
        "foo",
    ]);
    assert!(prepare.contains("Prepared classic login for player 1"));

    run_classic_ecgame_smoke_with_alias(&target, 1, "SYSOP");

    let maint = run_nc_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(maint.contains("Rust maintenance complete."));

    run_classic_ecgame_smoke_with_alias(&target, 1, "SYSOP");

    cleanup_dir(&target);
}

#[test]
fn sysop_new_game_accepts_player_count_flag() {
    let target = unique_temp_dir("nc-cli-sysop-new-game-players");
    let stdout = run_nc_cli(&[
        "sysop",
        "new-game",
        target.to_str().unwrap(),
        "--players",
        "2",
    ]);
    assert!(stdout.contains("Initialized new game"));
    assert!(stdout.contains("players=2"));
    cleanup_dir(&target);
}

#[test]
fn sysop_new_game_accepts_internal_kdl_setup_preset() {
    let target = unique_temp_dir("nc-cli-sysop-new-game-config");
    let stdout = run_nc_cli(&[
        "sysop",
        "new-game",
        target.to_str().unwrap(),
        "--config",
        "nc-cli/config/setup.example.kdl",
    ]);
    assert!(stdout.contains("Initialized new game"));
    assert!(stdout.contains("setup.example.kdl"));
    assert!(target.join("DATABASE.DAT").exists());
    cleanup_dir(&target);
}

#[test]
fn sysop_new_game_rejects_removed_runtime_setup_fields_in_setup_kdl() {
    let target = unique_temp_dir("nc-cli-sysop-new-game-invalid-config");
    let preset = target.join("invalid-setup.kdl");
    std::fs::write(
        &preset,
        "game player_count=4\nmaintenance_days { day \"mon\" }\n",
    )
    .expect("write invalid setup preset");

    let stderr = run_nc_cli_failure_in_dir(
        &[
            "sysop",
            "new-game",
            target.to_str().unwrap(),
            "--config",
            preset.to_str().unwrap(),
        ],
        common::rust_workspace(),
    );
    assert!(stderr.contains("unsupported setup.kdl node: maintenance_days"));

    cleanup_dir(&target);
}

#[test]
fn sysop_new_game_accepts_seed_and_reports_it() {
    let target = unique_temp_dir("nc-cli-sysop-new-game-seed");
    let stdout = run_nc_cli(&[
        "sysop",
        "new-game",
        target.to_str().unwrap(),
        "--players",
        "4",
        "--seed",
        "1515",
    ]);
    assert!(stdout.contains("Initialized new game"));
    assert!(stdout.contains("seed=1515"));
    cleanup_dir(&target);
}

#[test]
fn sysop_new_game_accepts_year_and_applies_it() {
    let target = unique_temp_dir("nc-cli-sysop-new-game-year");
    let stdout = run_nc_cli(&[
        "sysop",
        "new-game",
        target.to_str().unwrap(),
        "--players",
        "4",
        "--year",
        "3012",
        "--seed",
        "1515",
    ]);
    assert!(stdout.contains("Initialized new game"));
    assert!(stdout.contains("year=3012"));

    let game_data = load_classic_core_files(&target).expect("generated game should load");
    assert_eq!(game_data.conquest.game_year(), 3012);
    cleanup_dir(&target);
}

#[test]
fn sysop_new_game_accepts_manual_nine_player_tier() {
    let target = unique_temp_dir("nc-cli-sysop-new-game-nine");
    let stdout = run_nc_cli(&[
        "sysop",
        "new-game",
        target.to_str().unwrap(),
        "--players",
        "9",
        "--seed",
        "2025",
    ]);
    assert!(stdout.contains("Initialized new game"));
    assert!(stdout.contains("players=9"));
    assert!(target.join("PLAYER.DAT").exists());
    cleanup_dir(&target);
}

#[test]
fn sysop_generate_gamestate_writes_preflight_clean_directory() {
    let target = unique_temp_dir("nc-cli-sysop-generate");
    let stdout = run_nc_cli(&[
        "sysop",
        "generate-gamestate",
        target.to_str().unwrap(),
        "4",
        "3001",
        "16:13",
        "30:6",
        "2:25",
        "26:26",
    ]);
    assert!(stdout.contains("Generated gamestate at:"));
    assert!(stdout.contains("Preflight validation: OK"));
    cleanup_dir(&target);
}
