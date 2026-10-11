mod common;

use common::{cleanup_dir, export_campaign_db, run_nc_cli, run_nc_cli_in_dir, unique_temp_dir};
use nc_compat::{load_classic_core_files, save_classic_core_files};
use std::fs;

#[test]
fn match_identifies_original_fixture() {
    let stdout = run_nc_cli(&["match", "original/v1.5"]);
    assert!(stdout.contains("MATCH original/v1.5"));
}

#[test]
fn match_identifies_initialized_fixture() {
    let stdout = run_nc_cli(&["match", "fixtures/ecutil-init/v1.5"]);
    assert!(stdout.contains("MATCH fixtures/ecutil-init/v1.5"));
}

#[test]
fn match_identifies_current_known_post_maint_baseline() {
    let stdout = run_nc_cli(&["match", "fixtures/ecmaint-post/v1.5"]);
    assert!(stdout.contains("MATCH current-known-post-maint-baseline-core"));
    assert!(stdout.contains("MATCH fixtures/ecmaint-post/v1.5"));
}

#[test]
fn scenario_list_prints_known_scenarios() {
    let stdout = run_nc_cli(&["scenario", "original/v1.5", "list"]);
    assert!(stdout.contains("Known scenarios:"));
    assert!(stdout.contains("fleet-order: accepted fleet movement/order fixture"));
    assert!(stdout.contains("planet-build: accepted planet build-queue fixture"));
    assert!(stdout.contains("guard-starbase: accepted one-base guard-starbase fixture"));
}

#[test]
fn scenario_show_prints_fixture_metadata() {
    let stdout = run_nc_cli(&["scenario", "original/v1.5", "show", "guard-starbase"]);
    assert!(stdout.contains("Scenario: guard-starbase"));
    assert!(stdout.contains("Description: accepted one-base guard-starbase fixture"));
    assert!(stdout.contains("fixtures/ecmaint-starbase-pre/v1.5"));
    assert!(stdout.contains("PLAYER.DAT"));
    assert!(stdout.contains("FLEETS.DAT"));
    assert!(stdout.contains("BASES.DAT"));
}

#[test]
fn headers_prints_known_setup_and_conquest_values() {
    let stdout = run_nc_cli(&["headers", "original/v1.5"]);
    assert!(stdout.contains("SETUP.version=EC151"));
    assert!(stdout.contains("SETUP.option_prefix=[04, 03, 04, 03, 01, 01, 01, 01]"));
    assert!(stdout.contains("SETUP.com_irqs=[4, 3, 4, 3]"));
    assert!(stdout.contains("SETUP.com_flow_control=[true, true, true, true]"));
    assert!(stdout.contains("SETUP.snoop_enabled=true"));
    assert!(stdout.contains("SETUP.local_timeout_enabled=false"));
    assert!(stdout.contains("SETUP.remote_timeout_enabled=true"));
    assert!(stdout.contains("SETUP.max_time_between_keys_minutes_raw=10"));
    assert!(stdout.contains("SETUP.minimum_time_granted_minutes_raw=0"));
    assert!(stdout.contains("SETUP.purge_after_turns_raw=0"));
    assert!(stdout.contains("SETUP.autopilot_inactive_turns_raw=0"));
    assert!(stdout.contains("CONQUEST.game_year=3022"));
    assert!(stdout.contains("CONQUEST.player_count=4"));
    assert!(stdout.contains("CONQUEST.maintenance_schedule=[01, 01, 01, 01, 01, 01, 01]"));
    assert!(stdout.contains("CONQUEST.previous_production=[100, 100, 100, 100]"));
    assert!(stdout.contains("CONQUEST.previous_planet_count=[1, 1, 1, 1]"));
}

#[test]
fn headers_accepts_relative_fixture_paths_from_rust_workspace() {
    let stdout = run_nc_cli(&["headers", "../fixtures/ecutil-init/v1.5"]);
    assert!(stdout.contains("CONQUEST.game_year=3000"));
    assert!(stdout.contains("CONQUEST.player_count=4"));
}

#[test]
fn inspect_summarizes_core_directory_state() {
    let stdout = run_nc_cli(&["inspect", "fixtures/ecmaint-post/v1.5"]);
    assert!(stdout.contains("Directory:"));
    assert!(stdout.contains("Players:"));
    assert!(stdout.contains("campaign_state="));
    assert!(stdout.contains("reports_pending="));
    assert!(stdout.contains("messages_pending="));
    assert!(stdout.contains("Planets:"));
    assert!(stdout.contains("Fleets:"));
    assert!(stdout.contains("Bases:"));
    assert!(stdout.contains("IPBM:"));
}

#[test]
fn inspect_fleet_movement_reports_machine_readable_eta_and_route() {
    let target = unique_temp_dir("nc-cli-inspect-fleet-movement");
    common::copy_fixture_dir("fixtures/ecutil-init/v1.5", &target);

    let stdout = run_nc_cli(&[
        "fleet-order",
        target.to_str().unwrap(),
        "1",
        "3",
        "1",
        "15",
        "13",
    ]);
    assert!(stdout.contains("Fleet record 1 updated"));

    let movement = run_nc_cli(&["inspect-fleet-movement", target.to_str().unwrap(), "1"]);
    assert!(movement.contains("fleet_record=1"));
    assert!(movement.contains("order=move"));
    assert!(movement.contains("current_speed=3"));
    assert!(movement.contains("eta_status=years"));
    assert!(movement.contains("route="));

    cleanup_dir(&target);
}

#[test]
fn inspect_classic_login_reports_first_time_and_matched_preloaded_states() {
    let target = unique_temp_dir("nc-cli-inspect-classic-login");
    common::copy_fixture_dir("fixtures/ecutil-init/v1.5", &target);

    let first_time = run_nc_cli(&["inspect-classic-login", target.to_str().unwrap(), "SYSOP"]);
    assert!(first_time.contains("slot 1: classification=first-time-menu"));

    let prepared = run_nc_cli(&[
        "classic-login-prepare",
        target.to_str().unwrap(),
        "1",
        "SYSOP",
        "Auroran_Combine",
    ]);
    assert!(prepared.contains("Prepared classic login for player 1"));

    let matched = run_nc_cli(&["inspect-classic-login", target.to_str().unwrap(), "SYSOP"]);
    assert!(matched.contains("slot 1: classification=matched-preloaded-first-login"));
    assert!(matched.contains("homeworld='Not Named Yet'"));

    cleanup_dir(&target);
}

#[test]
fn inspect_classic_login_reports_returning_player_for_matching_named_homeworld() {
    let stdout = run_nc_cli(&["inspect-classic-login", "original/v1.5", "HANNIBAL"]);
    assert!(stdout.contains("slot 1: classification=returning-player"));
    assert!(stdout.contains("handle='HANNIBAL'"));
    assert!(stdout.contains("homeworld='Dust Bowl'"));
    assert!(stdout.contains("reports_pending="));
    assert!(stdout.contains("messages_pending="));
}

#[test]
fn inspect_messages_decodes_classic_mail_sample() {
    let target = unique_temp_dir("nc-cli-inspect-messages");
    fs::write(
        target.join("MESSAGES.DAT"),
        b"\x18this is a message to you\x00\x6e\xf7E\x00\xc5\x0b\x00\x00\xc5\x0b\x012\x06\x01A\xcfx\xf7\x8a\x02\x06\x032BFr\xd4\x03\xc5\x0b\x1d\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\xb8\x0b\x01\x02",
    )
    .unwrap();

    let stdout = run_nc_cli(&["inspect-messages", target.to_str().unwrap()]);
    assert!(stdout.contains("MESSAGES.DAT bytes="));
    assert!(stdout.contains("Subject: this is a message to you"));
    assert!(stdout.contains("Printable runs:"));
    assert!(stdout.contains("this is a message to you"));

    cleanup_dir(&target);
}

#[test]
fn map_export_uses_runtime_sqlite_and_ignores_stale_database_dat() {
    let target = unique_temp_dir("nc-cli-map-export");
    common::copy_fixture_dir("fixtures/ecutil-init/v1.5", &target);
    let import_stdout = run_nc_cli(&["db-import", target.to_str().unwrap()]);
    assert!(import_stdout.contains("Imported"));

    fs::write(target.join("DATABASE.DAT"), b"not a valid database").unwrap();

    let out_txt = target.join("map.txt");

    let stdout = run_nc_cli(&[
        "map-export",
        target.to_str().unwrap(),
        "1",
        out_txt.to_str().unwrap(),
    ]);
    assert!(stdout.contains("Exported player 1 starmap"));

    let txt = fs::read_to_string(&out_txt).unwrap();
    let csv = fs::read_to_string(target.join("map.csv")).unwrap();
    let details_csv = fs::read_to_string(target.join("map-DETAILS.csv")).unwrap();
    assert!(txt.contains("NOSTRIAN CONQUEST STARMAP"));
    assert!(txt.contains('*'));
    assert!(csv.starts_with(",01,02,03"));
    assert!(csv.contains(",*,"));
    assert!(details_csv.contains("x,y,known_name"));

    cleanup_dir(&target);
}

#[test]
fn planet_stored_sets_stored_production_points() {
    let target = unique_temp_dir("nc-cli-planet-stored");
    common::copy_fixture_dir("fixtures/ecutil-init/v1.5", &target);

    let stdout = run_nc_cli(&["planet-stored", target.to_str().unwrap(), "1", "50"]);
    assert!(stdout.contains("treasury set to 50"));

    export_campaign_db(&target, &target);
    let game_data = load_classic_core_files(&target).unwrap();
    assert_eq!(game_data.planets[0].stored_production_points(), 50);

    cleanup_dir(&target);
}

#[test]
fn planet_present_sets_present_production_points() {
    let target = unique_temp_dir("nc-cli-planet-present");
    common::copy_fixture_dir("fixtures/ecutil-init/v1.5", &target);

    let stdout = run_nc_cli(&["planet-present", target.to_str().unwrap(), "1", "75"]);
    assert!(stdout.contains("present production points set to 75"));

    export_campaign_db(&target, &target);
    let game_data = load_classic_core_files(&target).unwrap();
    assert_eq!(game_data.planets[0].present_production_points(), Some(75));

    cleanup_dir(&target);
}

#[test]
fn fleet_location_sets_current_coords() {
    let target = unique_temp_dir("nc-cli-fleet-location");
    common::copy_fixture_dir("fixtures/ecutil-init/v1.5", &target);

    let stdout = run_nc_cli(&["fleet-location", target.to_str().unwrap(), "1", "9", "2"]);
    assert!(stdout.contains("current location set to (9,2)"));

    export_campaign_db(&target, &target);
    let game_data = load_classic_core_files(&target).unwrap();
    assert_eq!(game_data.fleets[0].current_location_coords_raw(), [9, 2]);

    cleanup_dir(&target);
}

#[test]
fn planet_stardock_sets_a_stardock_slot() {
    let target = unique_temp_dir("nc-cli-planet-stardock");
    common::copy_fixture_dir("fixtures/ecutil-init/v1.5", &target);

    let stdout = run_nc_cli(&[
        "planet-stardock",
        target.to_str().unwrap(),
        "1",
        "0",
        "1",
        "2",
    ]);
    assert!(stdout.contains("stardock slot 0 set"));

    export_campaign_db(&target, &target);
    let game_data = load_classic_core_files(&target).unwrap();
    assert_eq!(game_data.planets[0].stardock_kind_raw(0), 1);
    assert_eq!(game_data.planets[0].stardock_count_raw(0), 2);

    cleanup_dir(&target);
}

#[test]
fn compare_reports_expected_initialized_to_post_maint_shape() {
    let stdout = run_nc_cli(&[
        "compare",
        "fixtures/ecutil-init/v1.5",
        "fixtures/ecmaint-post/v1.5",
    ]);
    assert!(stdout.contains("SETUP.DAT: size 522 vs 522, differing bytes 0"));
    assert!(stdout.contains("CONQUEST.DAT: size 2085 vs 2085, differing bytes 51"));
    assert!(stdout.contains("DATABASE.DAT: size 8000 vs 8000, differing bytes 80"));
    assert!(stdout.contains("FLEETS.DAT: size 864 vs 864, differing bytes 0"));
}

#[test]
fn scenario_init_all_materializes_all_known_scenarios() {
    let target = unique_temp_dir("nc-cli-all-scenarios");

    let stdout = run_nc_cli_in_dir(
        &["scenario-init-all", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Initialized all known scenarios under"));

    let manifest = fs::read_to_string(target.join("SCENARIOS.txt")).unwrap();
    assert!(manifest.contains("source="));
    assert!(manifest.contains("fixtures/ecmaint-post/v1.5"));
    assert!(manifest.contains("fleet-order"));
    assert!(manifest.contains("planet-build"));
    assert!(manifest.contains("guard-starbase"));

    let fleet_validate = run_nc_cli_in_dir(
        &[
            "validate",
            target.join("fleet-order").to_str().unwrap(),
            "fleet-order",
        ],
        common::rust_workspace(),
    );
    assert!(fleet_validate.contains("Valid fleet-order scenario"));

    let build_validate = run_nc_cli_in_dir(
        &[
            "validate",
            target.join("planet-build").to_str().unwrap(),
            "planet-build",
        ],
        common::rust_workspace(),
    );
    assert!(build_validate.contains("Valid planet-build scenario"));

    let starbase_validate = run_nc_cli_in_dir(
        &[
            "validate",
            target.join("guard-starbase").to_str().unwrap(),
            "guard-starbase",
        ],
        common::rust_workspace(),
    );
    assert!(starbase_validate.contains("Valid guard-starbase scenario"));

    cleanup_dir(&target);
}

#[test]
fn scenario_init_compose_materializes_combined_directory() {
    let target = unique_temp_dir("nc-cli-scenario-compose");

    let stdout = run_nc_cli_in_dir(
        &[
            "scenario-init-compose",
            target.to_str().unwrap(),
            "fleet-order",
            "planet-build",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Applied scenarios: fleet-order, planet-build"));
    assert!(stdout.contains("Scenario chain directory initialized at"));

    let fleet_validate = run_nc_cli_in_dir(
        &["validate", target.to_str().unwrap(), "fleet-order"],
        common::rust_workspace(),
    );
    assert!(fleet_validate.contains("Valid fleet-order scenario"));

    let build_validate = run_nc_cli_in_dir(
        &["validate", target.to_str().unwrap(), "planet-build"],
        common::rust_workspace(),
    );
    assert!(build_validate.contains("Valid planet-build scenario"));

    cleanup_dir(&target);
}

#[test]
fn core_init_current_known_baseline_materializes_valid_directory() {
    let target = unique_temp_dir("nc-cli-core-init-current-known");

    let stdout = run_nc_cli_in_dir(
        &["core-init-current-known-baseline", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Current-known baseline directory initialized at"));
    assert!(stdout.contains("source snapshot:"));
    assert!(stdout.contains("fixtures/ecmaint-post/v1.5"));
    assert!(stdout.contains("initialized_fleet_blocks = true"));
    assert!(stdout.contains("homeworld_seed_payloads = true"));
    assert!(stdout.contains("setup_baseline = true"));
    assert!(stdout.contains("conquest_baseline = true"));

    let validate_stdout = run_nc_cli_in_dir(
        &["core-validate", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(validate_stdout.contains("Valid core state"));

    let exact_stdout = run_nc_cli_in_dir(
        &[
            "core-validate-current-known-baseline",
            target.to_str().unwrap(),
        ],
        common::rust_workspace(),
    );
    assert!(exact_stdout.contains("Exact canonical current-known baseline match"));

    cleanup_dir(&target);
}

#[test]
fn core_init_current_known_baseline_accepts_original_source_snapshot() {
    let target = unique_temp_dir("nc-cli-core-init-current-known-original");

    let stdout = run_nc_cli_in_dir(
        &[
            "core-init-current-known-baseline",
            "original/v1.5",
            target.to_str().unwrap(),
        ],
        common::repo_root(),
    );
    assert!(stdout.contains("Current-known baseline directory initialized at"));
    assert!(stdout.contains("source snapshot: original/v1.5"));

    let stderr = common::run_nc_cli_failure_in_dir(
        &[
            "core-validate-current-known-baseline",
            target.to_str().unwrap(),
        ],
        common::rust_workspace(),
    );
    assert!(stderr.contains("canonical current-known post-maint baseline"));

    let match_stdout = run_nc_cli_in_dir(
        &["match", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(!match_stdout.contains("MATCH current-known-post-maint-baseline-core"));

    assert!(target.join("ECGAME.EXE").exists());
    assert!(target.join("ECMAINT.EXE").exists());
    assert!(target.join("IPBM.DAT").exists());

    cleanup_dir(&target);
}

#[test]
fn core_report_canonical_transition_clusters_groups_original_sample_drift() {
    let target = unique_temp_dir("nc-cli-core-transition-clusters-original");

    run_nc_cli_in_dir(
        &[
            "core-init-current-known-baseline",
            "original/v1.5",
            target.to_str().unwrap(),
        ],
        common::repo_root(),
    );

    let stdout = run_nc_cli_in_dir(
        &[
            "core-report-canonical-transition-clusters",
            target.to_str().unwrap(),
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Canonical Transition Clusters"));
    assert!(stdout.contains("PLAYER.DAT:"));
    assert!(stdout.contains("record 1 -> [0, 1, 2, 3, 4, 5, 6, 7, 8"));
    assert!(stdout.contains("PLANETS.DAT:"));
    assert!(stdout.contains("record 6 -> [0, 1, 2, 3, 8, 9"));
    assert!(stdout.contains("FLEETS.DAT:"));
    assert!(stdout.contains("record 5 -> [11, 12, 32, 33]"));
    assert!(stdout.contains("CONQUEST.DAT: differing_offsets=[]"));

    cleanup_dir(&target);
}

#[test]
fn core_report_canonical_transition_details_summarizes_differing_records() {
    let target = unique_temp_dir("nc-cli-core-transition-details-original");

    run_nc_cli_in_dir(
        &[
            "core-init-current-known-baseline",
            "original/v1.5",
            target.to_str().unwrap(),
        ],
        common::repo_root(),
    );

    let stdout = run_nc_cli_in_dir(
        &[
            "core-report-canonical-transition-details",
            target.to_str().unwrap(),
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Canonical Transition Details"));
    assert!(stdout.contains("PLANETS.DAT:"));
    assert!(stdout.contains("record 6 current:"));
    assert!(stdout.contains("record 6 canonical:"));
    assert!(stdout.contains("FLEETS.DAT:"));
    assert!(stdout.contains("record 9 current: loc="));
    assert!(stdout.contains("record 9 canonical: loc="));

    cleanup_dir(&target);
}

#[test]
fn core_diff_current_known_baseline_reports_mutated_files() {
    let target = unique_temp_dir("nc-cli-core-diff-current-known");
    common::copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut data = load_classic_core_files(&target).unwrap();
    data.setup.set_version_tag(b"BAD!!");
    data.planets[14].set_stored_goods(3.0);
    save_classic_core_files(&target, &data).unwrap();

    let stdout = run_nc_cli_in_dir(
        &["core-diff-current-known-baseline", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Current-known Baseline Diff"));
    assert!(stdout.contains("SETUP.DAT: differing_bytes="));
    assert!(stdout.contains("PLANETS.DAT: differing_bytes="));

    cleanup_dir(&target);
}

#[test]
fn core_diff_current_known_baseline_offsets_reports_mutated_offsets() {
    let target = unique_temp_dir("nc-cli-core-diff-current-known-offsets");
    common::copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut data = load_classic_core_files(&target).unwrap();
    data.setup.set_version_tag(b"BAD!!");
    data.planets[14].set_stored_goods(3.0);
    save_classic_core_files(&target, &data).unwrap();

    let stdout = run_nc_cli_in_dir(
        &[
            "core-diff-current-known-baseline-offsets",
            target.to_str().unwrap(),
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Current-known Baseline Diff Offsets"));
    assert!(stdout.contains("SETUP.DAT: differing_offsets=[0, 1, 2, 3, 4"));
    assert!(stdout.contains("PLANETS.DAT: differing_offsets="));

    cleanup_dir(&target);
}

#[test]
fn core_diff_canonical_current_known_baseline_reports_original_gap() {
    let target = unique_temp_dir("nc-cli-core-diff-canonical-current-known");
    run_nc_cli_in_dir(
        &[
            "core-init-current-known-baseline",
            "original/v1.5",
            target.to_str().unwrap(),
        ],
        common::rust_workspace(),
    );

    let stdout = run_nc_cli_in_dir(
        &[
            "core-diff-canonical-current-known-baseline",
            target.to_str().unwrap(),
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Canonical Current-known Baseline Diff"));
    assert!(stdout.contains("PLAYER.DAT: differing_bytes="));
    assert!(stdout.contains("PLANETS.DAT: differing_bytes="));
    assert!(stdout.contains("FLEETS.DAT: differing_bytes="));
    assert!(stdout.contains("CONQUEST.DAT: differing_bytes="));

    cleanup_dir(&target);
}

#[test]
fn core_diff_canonical_current_known_baseline_offsets_reports_mutated_offsets() {
    let target = unique_temp_dir("nc-cli-core-diff-canonical-current-known-offsets");
    common::copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut data = load_classic_core_files(&target).unwrap();
    data.setup.set_version_tag(b"BAD!!");
    data.planets[14].set_stored_goods(3.0);
    save_classic_core_files(&target, &data).unwrap();

    let stdout = run_nc_cli_in_dir(
        &[
            "core-diff-canonical-current-known-baseline-offsets",
            target.to_str().unwrap(),
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Canonical Current-known Baseline Diff Offsets"));
    assert!(stdout.contains("SETUP.DAT: differing_offsets=[0, 1, 2, 3, 4"));
    assert!(stdout.contains("PLANETS.DAT: differing_offsets="));

    cleanup_dir(&target);
}

#[test]
fn core_init_canonical_current_known_baseline_materializes_exact_directory() {
    let target = unique_temp_dir("nc-cli-core-init-canonical-current-known");

    let stdout = run_nc_cli_in_dir(
        &[
            "core-init-canonical-current-known-baseline",
            "original/v1.5",
            target.to_str().unwrap(),
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Canonical current-known baseline directory initialized"));
    assert!(stdout.contains("exact_canonical_current_known_baseline = true"));

    let exact_stdout = run_nc_cli_in_dir(
        &[
            "core-validate-current-known-baseline",
            target.to_str().unwrap(),
        ],
        common::rust_workspace(),
    );
    assert!(exact_stdout.contains("Exact canonical current-known baseline match"));

    let match_stdout = run_nc_cli_in_dir(
        &["match", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(match_stdout.contains("MATCH current-known-post-maint-baseline-core"));
    assert!(target.join("ECGAME.EXE").exists());
    assert!(target.join("ECMAINT.EXE").exists());

    cleanup_dir(&target);
}
