mod common;

use common::{cleanup_dir, run_nc_cli, run_nc_cli_in_dir, unique_temp_dir};
use nc_data::CoreGameData;

#[test]
fn compliance_report_summarizes_known_post_fixture_failures() {
    let stdout = run_nc_cli(&["compliance-report", "fixtures/ecmaint-post/v1.5"]);
    assert!(stdout.contains("Compliance Report"));
    assert!(stdout.contains("FAIL guard-starbase-linkage:"));
    assert!(stdout.contains("OK   ipbm-count-length"));
    assert!(stdout.contains("Key words: player.first_base_record=0 player.ipbm_count=0"));
    assert!(stdout.contains("guarding_fleet_count=0"));
}

#[test]
fn core_report_summarizes_known_post_fixture_counts() {
    let stdout = run_nc_cli(&["core-report", "fixtures/ecmaint-post/v1.5"]);
    assert!(stdout.contains("Core State Report"));
    assert!(stdout.contains("player_record_count=4"));
    assert!(stdout.contains("planet_record_count=20"));
    assert!(stdout.contains("fleet_record_count=16"));
    assert!(stdout.contains("base_record_count=0"));
    assert!(stdout.contains("ipbm_record_count=0"));
    assert!(stdout.contains("initialized_fleet_blocks=true"));
    assert!(stdout.contains("initialized_fleet_payloads=true"));
    assert!(stdout.contains("initialized_fleet_missions=true"));
    assert!(stdout.contains("initialized_homeworld_alignment=true"));
    assert!(stdout.contains("initialized_planet_ownership=true"));
    assert!(stdout.contains("homeworld_seed_payloads=true"));
    assert!(stdout.contains("unowned_planet_payloads=true"));
    assert!(stdout.contains("empty_auxiliary_state=true"));
    assert!(stdout.contains("setup_baseline=true"));
    assert!(stdout.contains("conquest_baseline=true"));
    assert!(stdout.contains("initialized_fleet_block_head_ids=[1, 5, 9, 13]"));
    assert!(stdout.contains("player1_starbase_count=0"));
    assert!(stdout.contains("player1_owned_base_record_count=0"));
    assert!(stdout.contains("player1_ipbm_count=0"));
    assert!(stdout.contains("player 01: campaign_state=civil_disorder owned_planet_count=1 homeworld_seed_coords=Some([16, 13]) owned_base_count=0 first_base_record_raw=0 ipbm_count_word_raw=0 fleet_chain_head_raw=1"));
}

#[test]
fn core_validate_accepts_known_post_fixture_state() {
    let stdout = run_nc_cli(&["core-validate", "fixtures/ecmaint-post/v1.5"]);
    assert!(stdout.contains("Valid core state"));
    assert!(stdout.contains("base_record_count = 0"));
    assert!(stdout.contains("ipbm_record_count = 0"));
    assert!(stdout.contains("initialized_fleet_blocks = true"));
    assert!(stdout.contains("initialized_fleet_payloads = true"));
    assert!(stdout.contains("initialized_fleet_missions = true"));
    assert!(stdout.contains("initialized_homeworld_alignment = true"));
    assert!(stdout.contains("initialized_planet_ownership = true"));
    assert!(stdout.contains("homeworld_seed_payloads = true"));
    assert!(stdout.contains("unowned_planet_payloads = true"));
    assert!(stdout.contains("empty_auxiliary_state = true"));
    assert!(stdout.contains("setup_baseline = true"));
    assert!(stdout.contains("conquest_baseline = true"));
    assert!(stdout.contains("player_inputs = true"));
    assert!(stdout.contains("player1_starbase_count = 0"));
    assert!(stdout.contains("player1_owned_base_record_count = 0"));
    assert!(stdout.contains("player1_ipbm_count = 0"));
}

#[test]
fn core_validate_rejects_invalid_fleet_player_inputs() {
    let target = unique_temp_dir("nc-cli-core-validate-player-inputs");
    common::copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut data = CoreGameData::load(&target).unwrap();
    let fleet = &mut data.fleets.records[0];
    fleet.set_current_speed(fleet.max_speed().saturating_add(2));
    fleet.set_destroyer_count(0);
    fleet.set_cruiser_count(0);
    fleet.set_battleship_count(0);
    fleet.set_scout_count(1);
    fleet.set_rules_of_engagement(6);
    data.save(&target).unwrap();

    let stderr = common::run_nc_cli_failure_in_dir(
        &["core-validate", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(stderr.contains("FLEET[1] invalid player input"));
    assert!(stderr.contains("SpeedExceedsMaximum"));

    cleanup_dir(&target);
}

#[test]
fn core_validate_rejects_invalid_diplomacy_bytes() {
    let target = unique_temp_dir("nc-cli-core-validate-diplomacy");
    common::copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut data = CoreGameData::load(&target).unwrap();
    data.player.records[0].raw[0x54] = 0x01;
    data.player.records[0].raw[0x55] = 0xfe;
    data.save(&target).unwrap();

    let stderr = common::run_nc_cli_failure_in_dir(
        &["core-validate", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(stderr.contains("PLAYER[1] invalid diplomacy input"));
    assert!(stderr.contains("SelfTarget"));

    cleanup_dir(&target);
}

#[test]
fn core_validate_current_known_baseline_accepts_known_post_fixture_state() {
    let stdout = run_nc_cli(&[
        "core-validate-current-known-baseline",
        "fixtures/ecmaint-post/v1.5",
    ]);
    assert!(stdout.contains("Exact canonical current-known baseline match"));
}

#[test]
fn core_sync_counts_repairs_player1_count_words() {
    let target = unique_temp_dir("nc-cli-core-sync");
    common::copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut data = CoreGameData::load(&target).unwrap();
    data.player.records[0].set_first_base_record_raw(3);
    data.player.records[0].set_ipbm_count_raw(2);
    data.save(&target).unwrap();

    let stderr = common::run_nc_cli_failure_in_dir(
        &["core-validate", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(stderr.contains("PLAYER[1] base chain names BASES record 3, past the end"));
    assert!(stderr.contains("IPBM.DAT record count expected 2, got 0"));

    let sync_stdout = run_nc_cli_in_dir(
        &["core-sync-counts", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(sync_stdout.contains("Core counts synchronized"));
    assert!(sync_stdout.contains("player1_starbase_count = 0"));
    assert!(sync_stdout.contains("player1_owned_base_record_count = 0"));
    assert!(sync_stdout.contains("player1_ipbm_count = 0"));
    assert!(sync_stdout.contains("initialized_fleet_blocks = true"));
    assert!(sync_stdout.contains("initialized_fleet_payloads = true"));
    assert!(sync_stdout.contains("initialized_fleet_missions = true"));
    assert!(sync_stdout.contains("initialized_homeworld_alignment = true"));
    assert!(sync_stdout.contains("initialized_planet_ownership = true"));
    assert!(sync_stdout.contains("homeworld_seed_payloads = true"));
    assert!(sync_stdout.contains("unowned_planet_payloads = true"));
    assert!(sync_stdout.contains("empty_auxiliary_state = true"));
    assert!(sync_stdout.contains("setup_baseline = true"));
    assert!(sync_stdout.contains("conquest_baseline = true"));
    assert!(sync_stdout.contains("player 02: owned_planet_count = 1 homeworld_seed_coords = Some([4, 13]) owned_base_count = 0 first_base_record_raw = 0 ipbm_count_word_raw = 0 fleet_chain_head_raw = 5"));

    let validate_stdout = run_nc_cli_in_dir(
        &["core-validate", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(validate_stdout.contains("Valid core state"));

    cleanup_dir(&target);
}

#[test]
fn core_sync_baseline_repairs_control_and_count_fields() {
    let target = unique_temp_dir("nc-cli-core-sync-baseline");
    common::copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut data = CoreGameData::load(&target).unwrap();
    data.player.records[0].set_first_base_record_raw(3);
    data.player.records[0].set_ipbm_count_raw(2);
    data.setup.raw[..5].copy_from_slice(b"BAD!!");
    data.setup.set_remote_timeout_enabled(false);
    data.conquest.raw[0..2].copy_from_slice(&2999u16.to_le_bytes());
    data.conquest.raw[2] = 9;
    data.conquest.set_maintenance_schedule_enabled([false; 7]);
    data.save(&target).unwrap();

    let sync_stdout = run_nc_cli_in_dir(
        &["core-sync-baseline", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(sync_stdout.contains("Core baseline synchronized"));
    assert!(sync_stdout.contains("setup_baseline = true"));
    assert!(sync_stdout.contains("conquest_baseline = true"));

    let validate_stdout = run_nc_cli_in_dir(
        &["core-validate", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(validate_stdout.contains("Valid core state"));

    cleanup_dir(&target);
}

#[test]
fn core_sync_initialized_fleets_repairs_fleet_baseline() {
    let target = unique_temp_dir("nc-cli-core-sync-fleets");
    common::copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut data = CoreGameData::load(&target).unwrap();
    data.fleets.records.clear();
    data.fleets.records.push(nc_data::FleetRecord::new_zeroed());
    data.save(&target).unwrap();

    let sync_stdout = run_nc_cli_in_dir(
        &["core-sync-initialized-fleets", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(sync_stdout.contains("Initialized fleet baseline synchronized"));
    assert!(sync_stdout.contains("initialized_fleet_blocks = true"));
    assert!(sync_stdout.contains("initialized_fleet_payloads = true"));
    assert!(sync_stdout.contains("initialized_fleet_missions = true"));
    assert!(sync_stdout.contains("initialized_homeworld_alignment = true"));

    let validate_stdout = run_nc_cli_in_dir(
        &["core-validate", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(validate_stdout.contains("Valid core state"));

    cleanup_dir(&target);
}

#[test]
fn core_sync_initialized_planets_repairs_planet_payload_baseline() {
    let target = unique_temp_dir("nc-cli-core-sync-planets");
    common::copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut data = CoreGameData::load(&target).unwrap();
    data.planets.records[14].set_stored_goods(3.0);
    data.planets.records[0].set_status_or_name_summary_raw("Broken");
    data.planets.records[0].set_army_count_raw(9);
    data.save(&target).unwrap();

    let sync_stdout = run_nc_cli_in_dir(
        &["core-sync-initialized-planets", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(sync_stdout.contains("Initialized planet payloads synchronized"));
    assert!(sync_stdout.contains("initialized_planet_ownership = true"));
    assert!(sync_stdout.contains("homeworld_seed_payloads = true"));
    assert!(sync_stdout.contains("unowned_planet_payloads = true"));

    let validate_stdout = run_nc_cli_in_dir(
        &["core-validate", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(validate_stdout.contains("Valid core state"));

    cleanup_dir(&target);
}

#[test]
fn core_sync_current_known_baseline_repairs_combined_baseline_state() {
    let target = unique_temp_dir("nc-cli-core-sync-current-known");
    common::copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut data = CoreGameData::load(&target).unwrap();
    data.player.records[0].set_first_base_record_raw(7);
    data.player.records[0].set_ipbm_count_raw(4);
    data.setup.raw[..5].copy_from_slice(b"BAD!!");
    data.bases.records.push(nc_data::BaseRecord::new_zeroed());
    data.ipbm.records.push(nc_data::IpbmRecord {
        raw: [0u8; nc_data::IPBM_RECORD_SIZE],
    });
    data.fleets.records.clear();
    data.fleets.records.push(nc_data::FleetRecord::new_zeroed());
    data.planets.records[14].set_stored_goods(3.0);
    data.planets.records[0].set_status_or_name_summary_raw("Broken");
    data.save(&target).unwrap();

    let sync_stdout = run_nc_cli_in_dir(
        &["core-sync-current-known-baseline", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(sync_stdout.contains("Current-known baseline synchronized"));
    assert!(sync_stdout.contains("initialized_fleet_blocks = true"));
    assert!(sync_stdout.contains("initialized_fleet_payloads = true"));
    assert!(sync_stdout.contains("initialized_fleet_missions = true"));
    assert!(sync_stdout.contains("initialized_planet_ownership = true"));
    assert!(sync_stdout.contains("homeworld_seed_payloads = true"));
    assert!(sync_stdout.contains("unowned_planet_payloads = true"));
    assert!(sync_stdout.contains("empty_auxiliary_state = true"));
    assert!(sync_stdout.contains("setup_baseline = true"));
    assert!(sync_stdout.contains("conquest_baseline = true"));

    let validate_stdout = run_nc_cli_in_dir(
        &["core-validate", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(validate_stdout.contains("Valid core state"));

    cleanup_dir(&target);
}

#[test]
fn core_sync_canonical_current_known_baseline_repairs_to_exact_fixture() {
    let target = unique_temp_dir("nc-cli-core-sync-canonical-current-known");
    common::copy_fixture_dir("original/v1.5", &target);

    run_nc_cli_in_dir(
        &[
            "core-sync-canonical-current-known-baseline",
            target.to_str().unwrap(),
        ],
        common::rust_workspace(),
    );

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
fn compliance_report_summarizes_valid_parameterized_guard_starbase_directory() {
    let target = unique_temp_dir("nc-cli-compliance-report");

    run_nc_cli_in_dir(
        &["guard-starbase-init", target.to_str().unwrap(), "12", "9"],
        common::rust_workspace(),
    );

    let stdout = run_nc_cli_in_dir(
        &["compliance-report", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(stdout.contains("OK   guard-starbase-linkage"));
    assert!(stdout.contains("OK   ipbm-count-length"));
    assert!(stdout.contains("guarding_fleet_count=1"));
    assert!(stdout.contains("guarding_fleet[1]: guard_index=1 target=[12, 9]"));
    assert!(stdout.contains("fleet1.local_slot=1 fleet1.id=1"));
    assert!(stdout.contains("base1.escort=1 base1.id=1 base1.record=1 coords=[12, 9]"));

    cleanup_dir(&target);
}

#[test]
fn compliance_batch_report_summarizes_batch_directory_status() {
    let target = unique_temp_dir("nc-cli-compliance-batch");

    run_nc_cli_in_dir(
        &[
            "guard-starbase-batch-init",
            target.to_str().unwrap(),
            "12:9",
            "14:7",
        ],
        common::rust_workspace(),
    );

    let stdout = run_nc_cli_in_dir(
        &["compliance-batch-report", target.to_str().unwrap()],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Compliance Batch Report"));
    assert!(
        stdout.contains("x12-y09: fleet-order=fail planet-build=fail guard-starbase=ok ipbm=ok")
    );
    assert!(
        stdout.contains("x14-y07: fleet-order=fail planet-build=fail guard-starbase=ok ipbm=ok")
    );

    cleanup_dir(&target);
}

#[test]
fn validate_preserved_all_classifies_known_build_fixture() {
    let stdout = run_nc_cli(&[
        "validate-preserved",
        "fixtures/ecmaint-build-pre/v1.5",
        "all",
    ]);
    assert!(stdout.contains("OK   planet-build"));
    assert!(stdout.contains("FAIL fleet-order:"));
    assert!(stdout.contains("FAIL guard-starbase:"));
}

#[test]
fn validate_all_classifies_known_fleet_fixture() {
    let stdout = run_nc_cli(&["validate", "fixtures/ecmaint-fleet-pre/v1.5", "all"]);
    assert!(stdout.contains("OK   fleet-order"));
    assert!(stdout.contains("FAIL planet-build:"));
    assert!(stdout.contains("FAIL guard-starbase:"));
}

#[test]
fn validate_all_rejects_post_maint_fixture_as_fleet_or_build_or_starbase() {
    let stdout = run_nc_cli_in_dir(
        &["validate", "fixtures/ecmaint-post/v1.5", "all"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("FAIL fleet-order:"));
    assert!(stdout.contains("FAIL planet-build:"));
    assert!(stdout.contains("FAIL guard-starbase:"));
    assert!(stdout.contains("OK   ipbm"));
}

#[test]
fn validate_preserved_rejects_post_maint_fixture_as_fleet_or_build_or_starbase() {
    let stdout = run_nc_cli_in_dir(
        &["validate-preserved", "fixtures/ecmaint-post/v1.5", "all"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("FAIL fleet-order:"));
    assert!(stdout.contains("FAIL planet-build:"));
    assert!(stdout.contains("FAIL guard-starbase:"));
    assert!(stdout.contains("OK   ipbm"));
}
