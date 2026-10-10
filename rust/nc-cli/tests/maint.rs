mod common;

use common::{
    cleanup_dir, copy_fixture_dir, repo_root, run_classic_ecgame_smoke,
    run_classic_ecgame_smoke_with_alias, run_maint_rust_failure_after_import,
    run_maint_rust_with_export, run_nc_cli_in_dir, set_mutual_enemy_in_player_dat, unique_temp_dir,
    write_mutual_enemy_diplomacy,
};
use nc_compat::DatabaseDat;
use nc_data::CoreGameData;
use nc_data::Order;
use nc_engine::GameStateBuilder;
use std::fs;

fn decode_chunked_report(bytes: &[u8]) -> String {
    const RESULTS_TEXT_SIZE: usize = 72;
    const RESULTS_TEXT_START: usize = 2;
    const RESULTS_TEXT_END: usize = RESULTS_TEXT_START + RESULTS_TEXT_SIZE;
    bytes
        .chunks(84)
        .flat_map(|chunk| {
            if chunk.len() != 84 {
                return Vec::new();
            }
            let used = chunk[1] as usize;
            if used <= RESULTS_TEXT_SIZE
                && chunk[RESULTS_TEXT_START + used..RESULTS_TEXT_END]
                    .iter()
                    .all(|byte| *byte == 0)
            {
                return chunk[RESULTS_TEXT_START..RESULTS_TEXT_START + used].to_vec();
            }
            let text = &chunk[1..76];
            let end = text.iter().position(|b| *b == 0).unwrap_or(text.len());
            text[..end].to_vec()
        })
        .map(char::from)
        .collect::<String>()
}

fn results_records(bytes: &[u8]) -> Vec<&[u8]> {
    bytes.chunks(84).filter(|chunk| chunk.len() == 84).collect()
}

fn result_header_record_indexes(records: &[&[u8]]) -> Vec<usize> {
    records
        .iter()
        .enumerate()
        .filter_map(|(idx, record)| {
            let used = record[1] as usize;
            let text = String::from_utf8_lossy(&record[2..2 + used.min(72)]);
            text.starts_with("From ").then_some(idx)
        })
        .collect()
}

fn result_record_text(record: &[u8]) -> String {
    let used = record[1] as usize;
    String::from_utf8_lossy(&record[2..2 + used.min(72)]).into_owned()
}

fn logical_result_reports(records: &[&[u8]]) -> Vec<(u8, Vec<String>)> {
    let header_indexes = result_header_record_indexes(records);
    let mut reports = Vec::new();
    for (pos, start) in header_indexes.iter().enumerate() {
        let end = header_indexes
            .get(pos + 1)
            .copied()
            .unwrap_or(records.len());
        let kind = records[*start][0];
        let lines = records[*start..end]
            .iter()
            .map(|record| result_record_text(record))
            .collect::<Vec<_>>();
        reports.push((kind, lines));
    }
    reports
}

fn joined_report_containing(bytes: &[u8], needle: &str) -> String {
    let records = results_records(bytes);
    let (_, lines) = logical_result_reports(&records)
        .into_iter()
        .find(|(_, lines)| lines.iter().any(|line| line.contains(needle)))
        .unwrap_or_else(|| panic!("expected report containing {needle:?}"));
    lines.join(" ")
}

fn zero_all_fleets(game_data: &mut CoreGameData) {
    for fleet in &mut game_data.fleets.records {
        let current = fleet.current_location_coords_raw();
        fleet.set_standing_order_kind(Order::HoldPosition);
        fleet.set_standing_order_target_coords_raw(current);
        fleet.set_current_speed(0);
        fleet.set_rules_of_engagement(10);
        fleet.set_destroyer_count(0);
        fleet.set_cruiser_count(0);
        fleet.set_battleship_count(0);
        fleet.set_scout_count(0);
        fleet.set_troop_transport_count(0);
        fleet.set_army_count(0);
        fleet.set_etac_count(0);
    }
}

fn configure_delayed_invade_arrival_directory(target: &std::path::Path) {
    let mut game_data = CoreGameData::load(target).expect("fixture should load");
    let coords = [15, 13];

    zero_all_fleets(&mut game_data);

    let target_world = &mut game_data.planets.records[13];
    target_world.set_as_owned_target_world(
        coords,
        [0x64, 0x87],
        [0x00, 0x00, 0x00, 0x00, 0x48, 0x87],
        0x04,
        0x0b,
        *b"TargetPrimeet",
        [0x05, 0x1d, 0x0b, 0x11, 0x25, 0x1c, 0x05],
        1,
        1,
        0,
        2,
    );

    let attacker = &mut game_data.fleets.records[0];
    attacker.set_current_location_coords_raw([14, 13]);
    attacker.set_standing_order_kind(Order::InvadeWorld);
    attacker.set_standing_order_target_coords_raw(coords);
    attacker.set_current_speed(3);
    attacker.set_rules_of_engagement(10);
    attacker.set_scout_count(0);
    attacker.set_battleship_count(20);
    attacker.set_cruiser_count(20);
    attacker.set_destroyer_count(20);
    attacker.set_troop_transport_count(5);
    attacker.set_army_count(5);
    attacker.set_etac_count(0);
    attacker.set_at_rest_motion();

    game_data.save(target).expect("mutated fixture should save");
}

fn configure_dominant_invalidated_invade_directory(target: &std::path::Path) {
    let mut game_data = CoreGameData::load(target).expect("fixture should load");
    let coords = [15, 13];

    game_data.bases.records.clear();
    for fleet in &mut game_data.fleets.records {
        let current = fleet.current_location_coords_raw();
        fleet.set_current_speed(0);
        fleet.set_standing_order_kind(Order::HoldPosition);
        fleet.set_standing_order_target_coords_raw(current);
        fleet.set_rules_of_engagement(10);
        fleet.set_destroyer_count(0);
        fleet.set_cruiser_count(0);
        fleet.set_battleship_count(0);
        fleet.set_scout_count(0);
        fleet.set_troop_transport_count(0);
        fleet.set_army_count(0);
        fleet.set_etac_count(0);
    }

    let target_world = &mut game_data.planets.records[13];
    target_world.set_as_owned_target_world(
        coords,
        [0x64, 0x87],
        [0x00, 0x00, 0x00, 0x00, 0x48, 0x87],
        0x04,
        0x0b,
        *b"TargetPrimeet",
        [0x05, 0x1d, 0x0b, 0x11, 0x25, 0x1c, 0x05],
        10,
        4,
        2,
        2,
    );

    let attacker = &mut game_data.fleets.records[0];
    attacker.set_current_location_coords_raw(coords);
    attacker.set_standing_order_kind(Order::InvadeWorld);
    attacker.set_standing_order_target_coords_raw(coords);
    attacker.set_current_speed(3);
    attacker.raw[0x19] = 0x80;
    attacker.set_destroyer_count(1);
    attacker.set_troop_transport_count(1);
    attacker.set_army_count(1);

    let defender = &mut game_data.fleets.records[4];
    defender.set_current_location_coords_raw(coords);
    defender.set_standing_order_kind(Order::HoldPosition);
    defender.set_standing_order_target_coords_raw(coords);
    defender.set_current_speed(0);
    defender.set_destroyer_count(1);

    game_data.save(target).expect("mutated fixture should save");
}

#[test]
fn maint_rust_econ_updates_database_owner_intel_from_post_combat_planet_state() {
    let target = unique_temp_dir("nc-cli-maint-rust-econ");
    copy_fixture_dir("fixtures/ecmaint-econ-pre/v1.5", &target);

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Running Rust maintenance on:"));
    assert!(stdout.contains("Rust maintenance complete."));

    let game_data = CoreGameData::load(&target).expect("maint-rust output should load");
    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");

    let (planet_idx, planet) = game_data
        .planets
        .records
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.coords_raw() == [15, 13])
        .expect("econ combat target should exist");
    let year_bytes = (game_data.conquest.game_year() - 1).to_le_bytes();
    let owner_player = planet.owner_empire_slot_raw().saturating_sub(1) as usize;
    let planet_name = planet.planet_name();

    let owner_record = database.record(planet_idx, owner_player, game_data.planets.records.len());
    assert_eq!(owner_record.planet_name_bytes(), planet_name.as_bytes());
    assert_eq!(owner_record.raw[0x15], planet.owner_empire_slot_raw());
    assert_eq!(owner_record.raw[0x16], year_bytes[0]);
    assert_eq!(owner_record.raw[0x17], year_bytes[1]);
    assert_eq!(
        owner_record.raw[0x1e],
        0x40 + planet.owner_empire_slot_raw()
    );
    assert_eq!(owner_record.raw[0x23], planet.army_count_raw());
    assert_eq!(owner_record.raw[0x25], planet.ground_batteries_raw());

    let unrelated_player = (owner_player + 1) % 4;
    let unrelated_record = database.record(
        planet_idx,
        unrelated_player,
        game_data.planets.records.len(),
    );
    assert_eq!(unrelated_record.planet_name_bytes(), b"UNKNOWN");
    assert_eq!(unrelated_record.raw[0x15], 0xff);
    let messages = fs::read(target.join("MESSAGES.DAT")).expect("MESSAGES.DAT should exist");
    assert!(messages.is_empty());

    cleanup_dir(&target);
}

#[test]
fn maint_rust_requires_explicit_export_to_project_snapshot_into_directory() {
    let target = unique_temp_dir("nc-cli-maint-rust-in-place-classic");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let pre = CoreGameData::load(&target).expect("fixture should load");
    let stdout = run_nc_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Rust maintenance complete."));

    let post = CoreGameData::load(&target).expect("maint-rust should leave directory untouched");
    assert_eq!(post.conquest.game_year(), pre.conquest.game_year());
    assert!(!target.join("ECGAME.EXE").exists());
    assert!(!target.join("ECMAINT.EXE").exists());

    common::export_campaign_db(&target, &target);

    let post = CoreGameData::load(&target).expect("explicit export should project latest snapshot");
    assert_eq!(post.conquest.game_year(), pre.conquest.game_year() + 1);
    assert!(target.join("DATABASE.DAT").exists());
    assert!(target.join("RESULTS.DAT").exists());
    assert!(target.join("MESSAGES.DAT").exists());
    assert!(target.join("ECGAME.EXE").exists());
    assert!(target.join("ECMAINT.EXE").exists());

    cleanup_dir(&target);
}

#[test]
fn maint_rust_ignores_live_classic_directory_changes_until_explicit_import() {
    let target = unique_temp_dir("nc-cli-maint-rust-live-classic-sync");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);
    common::import_campaign_db(&target);

    let mut classic_side = CoreGameData::load(&target).expect("fixture should load");
    classic_side.player.records[1].set_assigned_player_handle_raw("SYSOP");
    classic_side.player.records[1].set_controlled_empire_name_raw("foo");
    classic_side
        .save(&target)
        .expect("classic-side edit should save");

    let stdout = run_nc_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Rust maintenance complete."));

    common::export_campaign_db(&target, &target);

    let post = CoreGameData::load(&target).expect("exported runtime should load");
    assert_ne!(
        post.player.records[1].assigned_player_handle_summary(),
        "SYSOP"
    );
    assert_ne!(
        post.player.records[1].controlled_empire_name_summary(),
        "foo"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_preserves_prepared_classic_login_classification_across_turns() {
    let target = unique_temp_dir("nc-cli-maint-rust-classic-login-classification");
    copy_fixture_dir("fixtures/ecutil-init/v1.5", &target);

    let prepare = run_nc_cli_in_dir(
        &[
            "classic-login-prepare",
            target.to_str().unwrap(),
            "2",
            "SYSOP",
            "foo",
        ],
        common::rust_workspace(),
    );
    assert!(prepare.contains("Prepared classic login for player 2"));

    let before = run_nc_cli_in_dir(
        &["inspect-classic-login", target.to_str().unwrap(), "SYSOP"],
        common::rust_workspace(),
    );
    assert!(before.contains("slot 2: classification=matched-preloaded-first-login"));

    let stdout = run_nc_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Rust maintenance complete."));

    let after = run_nc_cli_in_dir(
        &["inspect-classic-login", target.to_str().unwrap(), "SYSOP"],
        common::rust_workspace(),
    );
    assert!(after.contains("slot 2: classification=matched-preloaded-first-login"));
    assert!(after.contains("handle='SYSOP'"));

    cleanup_dir(&target);
}

#[test]
#[ignore = "launches classic ECGAME through dosbox-x"]
fn maint_rust_output_reopens_in_classic_ecgame_smoke() {
    let target = unique_temp_dir("nc-cli-maint-rust-classic-ecgame");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let stdout = run_nc_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Rust maintenance complete."));

    run_classic_ecgame_smoke(&target, 1);

    cleanup_dir(&target);
}

#[test]
fn maint_rust_fleet_battle_generates_results_report_from_battle_events() {
    let target = unique_temp_dir("nc-cli-maint-rust-fleet-battle");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);
    write_mutual_enemy_diplomacy(&target, 1, 2);

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    assert!(
        !results.is_empty(),
        "RESULTS.DAT should contain battle summaries"
    );
    let post = CoreGameData::load(&target).expect("maint-rust output should load");
    let text = String::from_utf8_lossy(&results);
    assert!(
        text.contains("ALERT: Enemy fleet contact!")
            || text.contains("ALERT: Fleet contact lost!")
            || text.contains("ALERT: Starbase contact lost!")
    );
    assert!(text.contains("<end of transmission>"));
    assert!(text.contains("System("));
    assert!(text.contains("Alien forces:"), "{text}");
    assert_eq!(
        &results[82..84],
        &(post.conquest.game_year() - 1).to_le_bytes(),
        "battle fixture should stamp the year maintenance ran into the results tail, as classic does"
    );
    let records = results_records(&results);
    assert!(!records.is_empty(), "expected RESULTS.DAT records");
    let header_indexes = result_header_record_indexes(&records);
    assert!(
        header_indexes.len() >= 2,
        "expected multiple logical reports in RESULTS.DAT"
    );
    let first_chain_id = u16::from_le_bytes([records[0][74], records[0][75]]);
    let first_next_id = u16::from_le_bytes([records[0][78], records[0][79]]);
    assert_eq!(
        first_chain_id, 0,
        "first logical report should start with cursor id 0"
    );
    assert_eq!(
        first_next_id,
        (header_indexes[1] + 1) as u16,
        "header should point at the next header record index plus one"
    );
    assert_eq!(
        u16::from_le_bytes([records[1][74], records[1][75]]),
        first_chain_id,
        "continuation should stay on the same report chain id"
    );
    assert_eq!(
        u16::from_le_bytes([records[1][78], records[1][79]]),
        0,
        "continuation records should have next-header id zero (matching oracle behavior)"
    );
    let eot = records
        .iter()
        .find(|record| record[1] == 21 && &record[2..23] == b"<end of transmission>")
        .expect("expected explicit end-of-transmission record");
    assert_eq!(
        u16::from_le_bytes([eot[74], eot[75]]),
        first_chain_id,
        "EOT should remain part of the same report chain"
    );
    assert_eq!(
        u16::from_le_bytes([eot[78], eot[79]]),
        0,
        "EOT should have next-header id zero (matching oracle behavior)"
    );
    assert_eq!(post.player.records[0].classic_results_first_header_raw(), 1);
    // A viewer's first report starts its chain at 0; its later reports link
    // into it.
    assert!(
        header_indexes
            .iter()
            .skip(1)
            .any(|&idx| u16::from_le_bytes([records[idx][74], records[idx][75]]) != 0),
        "later headers should remain on a linked results chain"
    );
    let last_header = usize::from(post.player.records[0].classic_results_last_header_raw());
    assert!(
        header_indexes.contains(&last_header.wrapping_sub(1)),
        "player should point at the last header of its results chain"
    );
    assert_eq!(
        u16::from_le_bytes([records[last_header - 1][78], records[last_header - 1][79]]),
        0,
        "the last header should end the player's results chain"
    );
    let messages = fs::read(target.join("MESSAGES.DAT")).expect("MESSAGES.DAT should exist");
    assert!(messages.is_empty());

    cleanup_dir(&target);
}

#[test]
fn preserved_classic_results_allow_long_multi_page_reports() {
    let path = repo_root().join("fixtures/ecmaint-invade-post/v1.5/RESULTS.DAT");
    let results = fs::read(path).expect("preserved RESULTS.DAT should exist");
    let reports = logical_result_reports(&results_records(&results));
    let long_report = reports
        .iter()
        .find(|(_, lines)| {
            lines
                .iter()
                .any(|line| line.contains("We have been invaded and conquored"))
        })
        .expect("expected preserved invasion report");

    assert!(
        long_report.1.len() > 10,
        "classic preserved output should include long logical reports"
    );
    assert_eq!(
        long_report.1.first().map(String::as_str),
        Some("From planet \"TargetPrime\" in System(15,13):             Stardate: 1/3011")
    );
    assert_eq!(
        long_report.1.last().map(String::as_str),
        Some("<end of transmission>")
    );
    assert_eq!(
        long_report
            .1
            .iter()
            .filter(|line| line.starts_with("From "))
            .count(),
        1,
        "classic long reports do not restart with extra headers mid-report"
    );
}

#[test]
fn preserved_classic_results_can_zero_next_header_on_continuations_in_immediate_oracle_output() {
    let path = repo_root()
        .join("fixtures/ecmaint-fleet-battle-pre/v1.5/.oracle/after-ecmaint/RESULTS.DAT");
    let results = fs::read(path).expect("preserved oracle RESULTS.DAT should exist");
    let records = results_records(&results);
    let header_indexes = result_header_record_indexes(&records);
    assert!(
        header_indexes.len() >= 2,
        "expected multiple preserved reports"
    );

    let first_header = records[header_indexes[0]];
    assert_eq!(u16::from_le_bytes([first_header[74], first_header[75]]), 0);
    assert_eq!(
        u16::from_le_bytes([first_header[78], first_header[79]]),
        (header_indexes[1] + 1) as u16
    );

    let continuation = records[header_indexes[0] + 1];
    assert_eq!(
        u16::from_le_bytes([continuation[74], continuation[75]]),
        0,
        "continuation should stay on the first report chain"
    );
    assert_eq!(
        u16::from_le_bytes([continuation[78], continuation[79]]),
        0,
        "immediate oracle output can clear the next-header pointer on continuation records"
    );

    let eot = records[header_indexes[1] - 1];
    assert_eq!(
        result_record_text(eot),
        "<end of transmission>",
        "expected EOT before the next preserved report header"
    );
    assert_eq!(u16::from_le_bytes([eot[74], eot[75]]), 0);
    assert_eq!(
        u16::from_le_bytes([eot[78], eot[79]]),
        0,
        "immediate oracle output can clear the next-header pointer on EOT records"
    );
}

#[test]
fn preserved_classic_results_can_repeat_next_header_on_all_report_records() {
    let path = repo_root().join("fixtures/ecmaint-fleet-battle-post/v1.5/RESULTS.DAT");
    let results = fs::read(path).expect("preserved RESULTS.DAT should exist");
    let records = results_records(&results);
    let header_indexes = result_header_record_indexes(&records);
    assert!(
        header_indexes.len() >= 2,
        "expected multiple preserved reports"
    );

    let first_header = records[header_indexes[0]];
    let first_next = u16::from_le_bytes([first_header[78], first_header[79]]);
    assert_ne!(first_next, 0, "expected a later preserved report header");

    let continuation = records[header_indexes[0] + 1];
    assert_eq!(
        u16::from_le_bytes([continuation[74], continuation[75]]),
        0,
        "continuation should stay on the first report chain"
    );
    assert_eq!(
        u16::from_le_bytes([continuation[78], continuation[79]]),
        first_next,
        "preserved accumulated output can repeat the next-header pointer on continuation records"
    );

    let eot = records[header_indexes[1] - 1];
    assert_eq!(
        result_record_text(eot),
        "<end of transmission>",
        "expected EOT before the next preserved report header"
    );
    assert_eq!(
        u16::from_le_bytes([eot[78], eot[79]]),
        first_next,
        "preserved accumulated output can repeat the next-header pointer on EOT records"
    );
}

#[test]
fn maint_rust_scout_contact_is_merged_single_report() {
    let target = unique_temp_dir("nc-cli-maint-rust-classic-scout-kinds");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);
    write_mutual_enemy_diplomacy(&target, 1, 2);

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let records = results_records(&results);

    // The merged contact report contains both "Sensor contact" and identity info
    // in the same record chain — find the header record for it.
    let contact_record = records
        .iter()
        .find(|record| {
            let used = record[1] as usize;
            let line = String::from_utf8_lossy(&record[2..2 + used.min(72)]);
            line.contains("Sensor contact")
        })
        .or_else(|| {
            // May span multiple records; check the decoded report text instead
            let text = decode_chunked_report(&results);
            if text.contains("Sensor contact") {
                records.first()
            } else {
                None
            }
        });

    let text = decode_chunked_report(&results);
    assert!(
        text.contains("Sensor contact"),
        "merged contact report should contain sensor-contact flavor"
    );
    assert!(
        contact_record.is_some(),
        "contact kind should be present in results"
    );

    // There should be no standalone "Closing to check it out" prelude report
    assert!(
        !text.contains("Closing to check it out"),
        "prelude-only report should not appear in merged output"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_results_emit_left_justified_wrapped_lines_without_generic_fleet_headers() {
    let target = unique_temp_dir("nc-cli-maint-rust-classic-results-lines");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);
    write_mutual_enemy_diplomacy(&target, 1, 2);

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let records = results_records(&results);
    for record in &records {
        let line = result_record_text(record);
        assert!(
            line.chars().count() <= 72,
            "classic RESULTS line exceeded width: {:?}",
            line
        );
        // Structured label rows wrap with an 18-column hanging indent.
        let body = line.strip_prefix(&" ".repeat(18)).unwrap_or(&line);
        if !line.starts_with("From ") && line != "<end of transmission>" {
            assert!(
                !body.starts_with(' '),
                "body lines should be left-justified: {:?}",
                line
            );
        }
    }

    let normalized = decode_chunked_report(&results);
    assert!(
        !normalized.contains("From your fleet, located"),
        "fleet-origin reports should name a specific fleet: {:?}",
        normalized
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_scout_contact_is_single_merged_classic_report() {
    let target = unique_temp_dir("nc-cli-maint-rust-classic-scout-boundaries");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);
    write_mutual_enemy_diplomacy(&target, 1, 2);

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let reports = logical_result_reports(&results_records(&results));

    // The merged report contains "Sensor contact" flavor and identification in one logical report
    let merged_idx = reports
        .iter()
        .position(|(_, lines)| lines.iter().any(|l| l.contains("Sensor contact")))
        .expect("expected merged sensor-contact+identification report");

    // No standalone prelude-only report
    assert!(
        !reports
            .iter()
            .any(|(_, lines)| { lines.iter().any(|l| l.contains("Closing to check it out")) }),
        "prelude-only report should not appear"
    );

    // No separate standalone identification report
    assert!(
        !reports.iter().enumerate().any(|(i, (_, lines))| {
            i != merged_idx
                && lines
                    .iter()
                    .any(|l| l.contains("We identified an alien fleet"))
        }),
        "identification should not appear as a separate standalone report"
    );

    assert!(
        reports[merged_idx].0 > 0,
        "merged contact kind should be non-zero"
    );
    assert!(
        reports[merged_idx]
            .1
            .iter()
            .any(|l| l == "<end of transmission>"),
        "merged contact report should contain end-of-transmission marker"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_routed_reports_set_classic_pending_flags() {
    let target = unique_temp_dir("nc-cli-maint-rust-pending-flags");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);
    write_mutual_enemy_diplomacy(&target, 1, 2);

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let game_data = CoreGameData::load(&target).expect("maint-rust output should load");
    assert_eq!(
        game_data.player.records[0].classic_reports_pending_flag_raw(),
        1
    );
    assert_eq!(
        game_data.player.records[0].classic_messages_pending_flag_raw(),
        0
    );
    assert_eq!(
        game_data.player.records[0].classic_results_review_word_raw(),
        1
    );
    assert_eq!(
        game_data.player.records[0].classic_message_review_word_raw(),
        0
    );
    assert_eq!(
        game_data.player.records[1].classic_reports_pending_flag_raw(),
        1
    );
    assert_eq!(
        game_data.player.records[1].classic_messages_pending_flag_raw(),
        0
    );
    assert_eq!(
        game_data.player.records[1].classic_results_review_word_raw(),
        1
    );
    assert_eq!(
        game_data.player.records[1].classic_message_review_word_raw(),
        0
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_uses_stored_player_diplomacy_without_sidecar() {
    let target = unique_temp_dir("nc-cli-maint-rust-player-diplomacy");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);
    set_mutual_enemy_in_player_dat(&target, 1, 2);

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(
        text.contains("ALERT: Enemy fleet contact!")
            || text.contains("ALERT: Fleet contact lost!")
            || text.contains("ALERT: Starbase contact lost!")
    );
    assert!(text.contains("ALERT: Fleet contact lost!") || text.contains("held the field"));

    let diplomacy_sidecar = target.join("diplomacy.kdl");
    assert!(
        !diplomacy_sidecar.exists(),
        "stored PLAYER.DAT diplomacy should not require a sidecar"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_absorbs_small_game_sidecar_diplomacy_into_player_dat() {
    let target = unique_temp_dir("nc-cli-maint-rust-sidecar-persist-four");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);
    write_mutual_enemy_diplomacy(&target, 1, 2);

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let game_data = CoreGameData::load(&target).expect("maint-rust output should load");
    assert_eq!(
        game_data.player.records[0].diplomatic_relation_toward(2),
        Some(nc_data::DiplomaticRelation::Enemy)
    );
    assert_eq!(
        game_data.player.records[1].diplomatic_relation_toward(1),
        Some(nc_data::DiplomaticRelation::Enemy)
    );

    let diplomacy_sidecar = target.join("diplomacy.kdl");
    let sidecar_text =
        fs::read_to_string(&diplomacy_sidecar).expect("diplomacy.kdl should still exist");
    assert!(
        sidecar_text.trim().is_empty(),
        "persistable small-game diplomacy should migrate into PLAYER.DAT and clear the sidecar"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_uses_stored_player_diplomacy_without_sidecar_for_large_games() {
    let target = unique_temp_dir("nc-cli-maint-rust-player-diplomacy-nine");
    let stdout = run_nc_cli_in_dir(
        &[
            "sysop",
            "new-game",
            target.to_str().unwrap(),
            "--config",
            "nc-cli/config/setup.example.kdl",
            "--players",
            "9",
            "--seed",
            "1515",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("seed=1515"));

    let mut game_data = CoreGameData::load(&target).expect("generated game should load");
    let fleet_a = &mut game_data.fleets.records[0];
    fleet_a.set_current_location_coords_raw([8, 8]);
    fleet_a.set_standing_order_kind(Order::ScoutSector);
    fleet_a.set_standing_order_target_coords_raw([8, 8]);
    fleet_a.set_current_speed(0);
    fleet_a.raw[0x19] = 0x81;
    fleet_a.set_destroyer_count(1);
    fleet_a.set_cruiser_count(0);
    fleet_a.set_battleship_count(0);
    fleet_a.set_troop_transport_count(0);
    fleet_a.set_army_count(0);
    fleet_a.set_scout_count(1);
    fleet_a.set_etac_count(0);
    fleet_a.set_rules_of_engagement(10);

    let fleet_b = &mut game_data.fleets.records[(8 * 4) as usize];
    fleet_b.set_current_location_coords_raw([8, 8]);
    fleet_b.set_standing_order_kind(Order::HoldPosition);
    fleet_b.set_standing_order_target_coords_raw([8, 8]);
    fleet_b.set_current_speed(0);
    fleet_b.raw[0x19] = 0x81;
    fleet_b.set_destroyer_count(1);
    fleet_b.set_cruiser_count(0);
    fleet_b.set_battleship_count(0);
    fleet_b.set_troop_transport_count(0);
    fleet_b.set_army_count(0);
    fleet_b.set_scout_count(0);
    fleet_b.set_etac_count(0);
    fleet_b.set_rules_of_engagement(10);

    game_data
        .set_stored_diplomatic_relation(1, 9, nc_data::DiplomaticRelation::Enemy)
        .expect("player 1 -> 9 diplomacy should set");
    game_data
        .set_stored_diplomatic_relation(9, 1, nc_data::DiplomaticRelation::Enemy)
        .expect("player 9 -> 1 diplomacy should set");
    game_data.save(&target).expect("mutated game should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(
        text.contains("ALERT: Enemy fleet contact!")
            || text.contains("ALERT: Fleet contact lost!")
            || text.contains("ALERT: Starbase contact lost!")
    );

    let diplomacy_sidecar = target.join("diplomacy.kdl");
    assert!(
        !diplomacy_sidecar.exists(),
        "stored PLAYER.DAT diplomacy should cover larger tiers without a sidecar"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_updates_large_game_database_from_scout_intel_event() {
    let target = unique_temp_dir("nc-cli-maint-rust-large-database-intel");
    let stdout = run_nc_cli_in_dir(
        &[
            "sysop",
            "new-game",
            target.to_str().unwrap(),
            "--config",
            "nc-cli/config/setup.example.kdl",
            "--players",
            "9",
            "--seed",
            "1515",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("seed=1515"));

    let mut game_data = CoreGameData::load(&target).expect("generated game should load");
    zero_all_fleets(&mut game_data);
    let (planet_idx, coords, owner_empire_raw) = game_data
        .planets
        .records
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner_empire_slot_raw() == 2)
        .map(|(idx, planet)| (idx, planet.coords_raw(), planet.owner_empire_slot_raw()))
        .expect("generated game should contain an empire 2 world");

    let scout = &mut game_data.fleets.records[0];
    scout.set_current_location_coords_raw([coords[0].saturating_add(1), coords[1]]);
    scout.set_standing_order_kind(Order::ScoutSolarSystem);
    scout.set_standing_order_target_coords_raw(coords);
    scout.set_current_speed(3);
    scout.raw[0x19] = 0x00;
    scout.set_scout_count(1);
    game_data.save(&target).expect("mutated game should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let game_data = CoreGameData::load(&target).expect("maint-rust output should load");
    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let planet_count = game_data.planets.records.len();

    let viewer_record = database.record(planet_idx, 0, planet_count);
    assert_ne!(viewer_record.planet_name_bytes(), b"UNKNOWN");
    assert!(!viewer_record.planet_name_bytes().is_empty());
    assert_eq!(viewer_record.raw[0x15], owner_empire_raw);

    let unrelated_viewer_record = database.record(planet_idx, 2, planet_count);
    assert_eq!(unrelated_viewer_record.planet_name_bytes(), b"UNKNOWN");
    assert_ne!(unrelated_viewer_record.raw[0x15], owner_empire_raw);

    cleanup_dir(&target);
}

#[test]
fn maint_rust_lone_scout_system_does_not_reproduce_classic_abort_bug() {
    let target = unique_temp_dir("nc-cli-maint-rust-lone-scout-system");
    let stdout = run_nc_cli_in_dir(
        &[
            "sysop",
            "new-game",
            target.to_str().unwrap(),
            "--config",
            "nc-cli/config/setup.example.kdl",
            "--players",
            "9",
            "--seed",
            "1515",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("seed=1515"));

    let mut game_data = CoreGameData::load(&target).expect("generated game should load");
    let (planet_idx, coords, owner_empire_raw) = game_data
        .planets
        .records
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner_empire_slot_raw() == 2)
        .map(|(idx, planet)| (idx, planet.coords_raw(), planet.owner_empire_slot_raw()))
        .expect("generated game should contain an empire 2 world");

    for fleet in &mut game_data.fleets.records {
        let current = fleet.current_location_coords_raw();
        fleet.set_current_speed(0);
        fleet.set_standing_order_kind(Order::HoldPosition);
        fleet.set_standing_order_target_coords_raw(current);
    }

    let scout = &mut game_data.fleets.records[0];
    scout.set_current_location_coords_raw([coords[0].saturating_add(1), coords[1]]);
    scout.set_standing_order_kind(Order::ScoutSolarSystem);
    scout.set_standing_order_target_coords_raw(coords);
    scout.set_current_speed(3);
    scout.raw[0x19] = 0x00;
    scout.set_destroyer_count(0);
    scout.set_cruiser_count(0);
    scout.set_battleship_count(0);
    scout.set_troop_transport_count(0);
    scout.set_army_count(0);
    scout.set_etac_count(0);
    scout.set_scout_count(1);
    game_data.save(&target).expect("mutated game should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    assert!(
        !results.is_empty(),
        "lone scout mission should still generate a Rust report"
    );
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Scouting mission report"));
    assert!(
        !text.contains("Since we have lost all of our scouts"),
        "Rust maint should not reproduce the original ECMAINT lone-scout abort bug"
    );

    let game_data = CoreGameData::load(&target).expect("maint-rust output should load");
    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let planet_count = game_data.planets.records.len();
    let viewer_record = database.record(planet_idx, 0, planet_count);
    assert_ne!(viewer_record.planet_name_bytes(), b"UNKNOWN");
    assert!(!viewer_record.planet_name_bytes().is_empty());
    assert_eq!(viewer_record.raw[0x15], owner_empire_raw);

    cleanup_dir(&target);
}

#[test]
fn maint_rust_blockade_arrival_persists_enemy_relation_in_player_dat() {
    let target = unique_temp_dir("nc-cli-maint-rust-blockade-escalation");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let coords = game_data
        .planets
        .records
        .iter()
        .find(|planet| planet.owner_empire_slot_raw() == 2)
        .map(|planet| planet.coords_raw())
        .expect("fixture should contain an empire 2 world");

    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw([coords[0].saturating_add(1), coords[1]]);
    fleet.set_standing_order_kind(Order::GuardBlockadeWorld);
    fleet.set_standing_order_target_coords_raw(coords);
    fleet.set_current_speed(3);
    fleet.raw[0x19] = 0x00;
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let game_data = CoreGameData::load(&target).expect("maint-rust output should load");
    assert_eq!(
        game_data.player.records[0].diplomatic_relation_toward(2),
        Some(nc_data::DiplomaticRelation::Enemy)
    );
    assert_eq!(
        game_data.player.records[1].diplomatic_relation_toward(1),
        Some(nc_data::DiplomaticRelation::Enemy)
    );

    let diplomacy_sidecar = target.join("diplomacy.kdl");
    assert!(
        !diplomacy_sidecar.exists(),
        "blockade escalation should persist directly into PLAYER.DAT"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_without_enemy_declaration_reports_contact_without_forcing_battle() {
    let target = unique_temp_dir("nc-cli-maint-rust-peaceful-contact");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let fleet_a = &mut game_data.fleets.records[0];
    fleet_a.set_current_location_coords_raw([8, 8]);
    fleet_a.set_standing_order_kind(Order::ScoutSector);
    fleet_a.set_standing_order_target_coords_raw([8, 8]);
    fleet_a.set_current_speed(0);
    fleet_a.raw[0x19] = 0x81;
    fleet_a.set_destroyer_count(1);
    fleet_a.set_cruiser_count(0);
    fleet_a.set_battleship_count(0);
    fleet_a.set_troop_transport_count(0);
    fleet_a.set_army_count(0);
    fleet_a.set_scout_count(1);
    fleet_a.set_etac_count(0);
    fleet_a.set_rules_of_engagement(10);

    let fleet_b = &mut game_data.fleets.records[4];
    fleet_b.set_current_location_coords_raw([8, 8]);
    fleet_b.set_standing_order_kind(Order::HoldPosition);
    fleet_b.set_standing_order_target_coords_raw([8, 8]);
    fleet_b.set_current_speed(0);
    fleet_b.raw[0x19] = 0x81;
    fleet_b.set_destroyer_count(1);
    fleet_b.set_cruiser_count(0);
    // Big enough for the scout to sight it within the year.
    fleet_b.set_battleship_count(3);
    fleet_b.set_troop_transport_count(0);
    fleet_b.set_army_count(0);
    fleet_b.set_scout_count(0);
    fleet_b.set_etac_count(0);
    fleet_b.set_rules_of_engagement(10);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Sensor contact") || text.contains("contact shows"));
    assert!(!text.contains("ALERT: Enemy fleet contact!"));
    assert!(!text.contains("ALERT: Fleet contact lost!"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_rejects_invalid_diplomacy_sidecar() {
    let target = unique_temp_dir("nc-cli-maint-rust-invalid-diplomacy");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);
    fs::write(
        target.join("diplomacy.kdl"),
        "relation from=1 to=99 status=\"enemy\"\n",
    )
    .expect("invalid diplomacy.kdl should write");

    let stderr = run_maint_rust_failure_after_import(&target, 1);
    assert!(stderr.contains("1..=4") || stderr.contains("1..=25"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_persists_sidecar_diplomacy_into_player_dat_for_large_games() {
    let target = unique_temp_dir("nc-cli-maint-rust-sidecar-persist-nine");
    let stdout = run_nc_cli_in_dir(
        &[
            "sysop",
            "new-game",
            target.to_str().unwrap(),
            "--config",
            "nc-cli/config/setup.example.kdl",
            "--players",
            "9",
            "--seed",
            "1515",
        ],
        common::rust_workspace(),
    );
    assert!(stdout.contains("seed=1515"));

    write_mutual_enemy_diplomacy(&target, 1, 9);

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let game_data = CoreGameData::load(&target).expect("maint-rust output should load");
    assert_eq!(
        game_data.player.records[0].diplomatic_relation_toward(9),
        Some(nc_data::DiplomaticRelation::Enemy)
    );
    assert_eq!(
        game_data.player.records[8].diplomatic_relation_toward(1),
        Some(nc_data::DiplomaticRelation::Enemy)
    );

    let diplomacy_sidecar = target.join("diplomacy.kdl");
    let sidecar_text =
        fs::read_to_string(&diplomacy_sidecar).expect("diplomacy.kdl should still exist");
    assert!(
        sidecar_text.trim().is_empty(),
        "persistable diplomacy should migrate into PLAYER.DAT and clear the sidecar"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_invalidated_scout_mission_reports_abort_before_contact_loss() {
    let target = unique_temp_dir("nc-cli-maint-rust-lost-contact");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);
    write_mutual_enemy_diplomacy(&target, 1, 2);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    zero_all_fleets(&mut game_data);

    let battle_coords = [10u8, 10u8];
    {
        let f = &mut game_data.fleets.records[0];
        f.set_current_location_coords_raw(battle_coords);
        f.set_standing_order_kind(Order::ScoutSector);
        f.set_standing_order_target_coords_raw(battle_coords);
        f.set_current_speed(3);
        f.set_destroyer_count(1);
        f.set_troop_transport_count(2);
        f.set_army_count(0);
        f.set_scout_count(0);
        f.set_etac_count(0);
        // Neither side starts a fight, so the aborted fleet withdraws
        // whatever the encounter rolls are.
        f.set_rules_of_engagement(0);
    }
    {
        let f = &mut game_data.fleets.records[4];
        f.set_current_location_coords_raw(battle_coords);
        f.set_standing_order_kind(Order::MoveOnly);
        f.set_standing_order_target_coords_raw(battle_coords);
        f.set_current_speed(3);
        f.set_battleship_count(50);
        f.set_cruiser_count(50);
        f.set_destroyer_count(50);
        f.set_rules_of_engagement(0);
    }
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(text.contains("Hostile action forced us to abort the scout sector mission because"));
    assert!(text.contains("lacks the required scout ship"));
    assert!(text.contains("withdrawing toward"), "{text}");
    assert!(!text.contains("flight recorder"));
    assert!(!text.contains("ALERT: Fleet contact lost!"));
    assert!(!text.contains("Scout mission report: We were attacked by"));
    assert!(!text.contains("Scout mission report: We successfully intercepted"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_surviving_fleet_battle_reports_loaded_armies_without_zero_army_clauses() {
    let target = unique_temp_dir("nc-cli-maint-rust-battle-loaded-armies");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);
    write_mutual_enemy_diplomacy(&target, 1, 2);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    zero_all_fleets(&mut game_data);

    let battle_coords = [11u8, 11u8];
    {
        let f = &mut game_data.fleets.records[0];
        f.set_current_location_coords_raw(battle_coords);
        f.set_standing_order_kind(Order::MoveOnly);
        f.set_standing_order_target_coords_raw(battle_coords);
        f.set_current_speed(3);
        f.set_destroyer_count(3);
        f.set_troop_transport_count(2);
        f.set_army_count(2);
    }
    {
        let f = &mut game_data.fleets.records[4];
        f.set_current_location_coords_raw(battle_coords);
        f.set_standing_order_kind(Order::HoldPosition);
        f.set_standing_order_target_coords_raw(battle_coords);
        f.set_current_speed(0);
        f.set_destroyer_count(1);
        f.set_troop_transport_count(2);
        f.set_army_count(0);
    }
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let records = results_records(&results);
    let report_texts = logical_result_reports(&records)
        .into_iter()
        .map(|(_, lines)| lines.join(" "))
        .collect::<Vec<_>>();
    assert!(
        report_texts
            .iter()
            .any(|report| report.contains("From your 1st Fleet, located in System(11,11):"))
    );
    assert!(
        report_texts
            .iter()
            .any(|report| report.contains("Our forces:       3DD, 2TT*")),
        "expected friendly force summary with loaded armies, got: {report_texts:?}"
    );
    assert!(
        report_texts
            .iter()
            .any(|report| { report.contains("Alien forces:     1DD, 2TT") })
    );
    assert!(
        !report_texts
            .iter()
            .any(|report| report.contains("carrying 0 armies"))
    );
    cleanup_dir(&target);
}

#[test]
fn maint_rust_destroyed_starbase_generates_lost_contact_report() {
    let target = unique_temp_dir("nc-cli-maint-rust-lost-starbase");
    copy_fixture_dir("fixtures/ecmaint-starbase-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let starbase_coords = game_data.bases.records[0].coords_raw();
    let attacker = &mut game_data.fleets.records[4];
    attacker.set_current_location_coords_raw(starbase_coords);
    attacker.set_standing_order_kind(Order::MoveOnly);
    attacker.set_standing_order_target_coords_raw(starbase_coords);
    attacker.set_current_speed(0);
    attacker.raw[0x19] = 0x81;
    attacker.set_rules_of_engagement(10);
    attacker.set_destroyer_count(20);
    attacker.set_cruiser_count(10);
    attacker.set_battleship_count(5);
    attacker.set_scout_count(0);
    attacker.set_troop_transport_count(0);
    attacker.set_etac_count(0);
    game_data
        .save(&target)
        .expect("mutated fixture should save");
    // A fleet merely in the sector's deep space is attacked only by or
    // against a declared enemy.
    set_mutual_enemy_in_player_dat(&target, 1, 2);

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(text.contains("From your Fleet Command Center"));
    assert!(text.contains("ALERT: Starbase contact lost!"));
    assert!(text.contains("Enemy losses:"));

    let game_data = CoreGameData::load(&target).expect("maint-rust output should load");
    assert_eq!(game_data.player.records[0].starbase_count_raw(), 0);
    assert!(
        game_data
            .bases
            .records
            .iter()
            .all(|base| !(base.coords_raw() == starbase_coords
                && base.owner_empire_raw() == 1
                && base.base_id_raw() != 0))
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_attacker_battle_report_mentions_destroyed_starbase() {
    let target = unique_temp_dir("nc-cli-maint-rust-attacker-starbase-report");
    copy_fixture_dir("fixtures/ecmaint-starbase-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let starbase_coords = game_data.bases.records[0].coords_raw();
    let attacker = &mut game_data.fleets.records[4];
    attacker.set_current_location_coords_raw(starbase_coords);
    attacker.set_standing_order_kind(Order::MoveOnly);
    attacker.set_standing_order_target_coords_raw(starbase_coords);
    attacker.set_current_speed(0);
    attacker.raw[0x19] = 0x81;
    attacker.set_rules_of_engagement(10);
    attacker.set_destroyer_count(20);
    attacker.set_cruiser_count(10);
    attacker.set_battleship_count(5);
    attacker.set_scout_count(0);
    attacker.set_troop_transport_count(0);
    attacker.set_etac_count(0);
    game_data
        .save(&target)
        .expect("mutated fixture should save");
    // A fleet merely in the sector's deep space is attacked only by or
    // against a declared enemy.
    set_mutual_enemy_in_player_dat(&target, 1, 2);

    let stdout = run_maint_rust_with_export(&target, 2);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let report = joined_report_containing(&results, "Enemy starbase destroyed.");
    assert!(report.contains("ALERT: Enemy fleet contact!"), "{report}");
    assert!(report.contains("Enemy starbase destroyed."), "{report}");
    assert!(
        report.contains("The aliens were completely destroyed."),
        "{report}"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_colonization_generates_results_report_from_colony_event() {
    let target = unique_temp_dir("nc-cli-maint-rust-colonize");
    copy_fixture_dir("fixtures/ecmaint-fleet-pre/v1.5", &target);

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    assert!(
        !results.is_empty(),
        "RESULTS.DAT should contain colonization summaries"
    );
    let text = decode_chunked_report(&results);
    assert!(text.contains("From your 1st Fleet, located in System("));
    assert!(text.contains("successfully terraformed"));
    assert!(text.contains("started a new colony"));
    let messages = fs::read(target.join("MESSAGES.DAT")).expect("MESSAGES.DAT should exist");
    assert!(messages.is_empty());

    cleanup_dir(&target);
}

#[test]
fn maint_rust_exports_empty_messages_dat_when_classic_mail_is_not_in_queued_mail() {
    let target = unique_temp_dir("nc-cli-maint-rust-preserve-classic-mail");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    for fleet in &mut game_data.fleets.records {
        fleet.set_standing_order_kind(Order::HoldPosition);
        fleet.set_current_speed(0);
        fleet.raw[0x19] = 0x00;
    }
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let classic_mail = b"\x18this is a message to you\x00classic-payload".to_vec();
    fs::write(target.join("MESSAGES.DAT"), &classic_mail).expect("should seed classic mail");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    // MESSAGES.DAT is now always exported empty; classic mail blobs are no
    // longer preserved through the SQLite round-trip.  Messages live in the
    // queued_mail table instead.
    let messages = fs::read(target.join("MESSAGES.DAT")).expect("MESSAGES.DAT should exist");
    assert!(
        messages.is_empty(),
        "MESSAGES.DAT should be empty after structured storage migration"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_colonization_blocked_by_owner_generates_report() {
    let target = unique_temp_dir("nc-cli-maint-rust-colonize-blocked");
    copy_fixture_dir("fixtures/ecmaint-fleet-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let blocked = &mut game_data.planets.records[13];
    blocked.set_owner_empire_slot_raw(2);
    blocked.set_ownership_status_raw(2);
    blocked.set_planet_name("TargetPrime");
    blocked.set_army_count_raw(10);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    assert!(
        !results.is_empty(),
        "RESULTS.DAT should contain blocked colonization summaries"
    );
    let text = decode_chunked_report(&results);
    assert!(text.contains("From your 1st Fleet, located in System("));
    assert!(text.contains("aliens are already living on the world found within"));
    assert!(text.contains("aborting our mission"));
    assert!(text.contains("(Empire #"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_scout_sector_generates_results_report() {
    let target = unique_temp_dir("nc-cli-maint-rust-scout-sector");
    copy_fixture_dir("fixtures/ecmaint-fleet-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let scout = &mut game_data.fleets.records[0];
    scout.set_standing_order_kind(Order::ScoutSector);
    scout.set_standing_order_target_coords_raw([15, 13]);
    scout.set_scout_count(1);
    scout.set_etac_count(0);
    // ECGAME writes the 0x22 world-target flag as 0 for sector orders.
    scout.set_mission_aux_bytes([0, 0]);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    assert!(
        !results.is_empty(),
        "RESULTS.DAT should contain scout summaries"
    );
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Scouting mission report"));
    // "beginning to scout this sector" may span record boundaries with the new Stardate header.
    assert!(text.contains("arrived at our destination") || text.contains("beginning to scout"));
    assert!(text.contains("Sector(15,13)"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_scout_system_generates_results_report() {
    let target = unique_temp_dir("nc-cli-maint-rust-scout-system");
    copy_fixture_dir("fixtures/ecmaint-fleet-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let scout = &mut game_data.fleets.records[0];
    scout.set_standing_order_kind(Order::ScoutSolarSystem);
    scout.set_standing_order_target_coords_raw([15, 13]);
    scout.set_scout_count(1);
    scout.set_etac_count(0);
    // Cruiser speed settles in orbit one sector away within the year.
    scout.recompute_max_speed_from_composition();
    scout.set_current_speed(scout.max_speed());
    game_data.planets.records[13].set_stardock_count_raw(0, 2);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    assert!(
        !results.is_empty(),
        "RESULTS.DAT should contain scout summaries"
    );
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Scouting mission report"));
    assert!(text.contains("Owned by:"));
    assert!(text.contains("Number of ground batteries:"));
    assert!(text.contains("Scanning the planet's stardock"));
    assert!(text.contains("2 destroyers"));
    assert!(text.contains("System(15,13)"));

    let game_data = CoreGameData::load(&target).expect("maint-rust output should load");
    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let viewer_record = database.record(13, 0, game_data.planets.records.len());
    assert_eq!(
        viewer_record.planet_name_bytes(),
        game_data.planets.records[13].planet_name().as_bytes()
    );
    assert_eq!(
        viewer_record.raw[0x15],
        game_data.planets.records[13].owner_empire_slot_raw()
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_view_world_generates_results_and_database_intel() {
    let target = unique_temp_dir("nc-cli-maint-rust-view-world");
    copy_fixture_dir("fixtures/ecmaint-fleet-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let viewer = &mut game_data.fleets.records[0];
    viewer.set_standing_order_kind(Order::ViewWorld);
    viewer.set_standing_order_target_coords_raw([15, 13]);
    viewer.set_scout_count(0);
    viewer.set_etac_count(0);
    // Cruiser speed settles in orbit one sector away within the year.
    viewer.recompute_max_speed_from_composition();
    viewer.set_current_speed(viewer.max_speed());
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Viewing mission report"));
    // Strings near the 75-byte chunk boundary may be split across records in the new Stardate
    // header format. Check for unambiguous early-body content instead.
    assert!(text.contains("entered System(15,13)") || text.contains("long range"));
    let game_data = CoreGameData::load(&target).expect("maint-rust output should load");
    let expected_potential =
        game_data.planets.records[13].potential_production_points_current_known();
    assert!(text.contains(&format!("potential of {expected_potential} points")));
    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let viewer_record = database.record(13, 0, game_data.planets.records.len());
    assert_eq!(
        viewer_record.planet_name_bytes(),
        game_data.planets.records[13].planet_name().as_bytes()
    );
    assert_eq!(
        viewer_record.raw[0x15],
        game_data.planets.records[13].owner_empire_slot_raw()
    );
    assert_eq!(
        viewer_record.raw[0x1c],
        game_data.planets.records[13].potential_production_points_current_known() as u8
    );
    assert_eq!(viewer_record.raw[0x23], 0xff);
    assert_eq!(viewer_record.raw[0x24], 0xff);
    assert_eq!(viewer_record.raw[0x25], 0xff);
    assert_eq!(viewer_record.raw[0x26], 0xff);

    cleanup_dir(&target);
}

#[test]
fn maint_rust_refreshes_database_between_turns_for_route_hazards() {
    let target = unique_temp_dir("nc-cli-maint-rust-routing-refresh");

    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    let foreign_world = &mut game_data.planets.records[4];
    foreign_world.set_coords_raw([4, 2]);
    foreign_world.set_owner_empire_slot_raw(2);
    foreign_world.set_ownership_status_raw(2);
    foreign_world.set_planet_name("TargetPrime");
    foreign_world.set_ground_batteries_raw(3);
    foreign_world.set_army_count_raw(9);

    let scout = &mut game_data.fleets.records[0];
    scout.set_current_location_coords_raw([2, 2]);
    scout.set_standing_order_kind(Order::ScoutSolarSystem);
    scout.set_standing_order_target_coords_raw([4, 2]);
    scout.set_current_speed(3);
    scout.set_scout_count(1);

    let mover = &mut game_data.fleets.records[1];
    mover.set_current_location_coords_raw([0, 2]);
    mover.set_standing_order_kind(Order::MoveOnly);
    mover.set_standing_order_target_coords_raw([6, 2]);
    mover.set_current_speed(3);

    game_data.save(&target).expect("baseline should save");

    let database = DatabaseDat::generate_from_planets_and_year(
        &game_data
            .planets
            .records
            .iter()
            .map(|planet| planet.planet_name())
            .collect::<Vec<_>>(),
        game_data.conquest.game_year(),
        game_data.conquest.player_count() as usize,
        None,
    );
    fs::write(target.join("DATABASE.DAT"), database.to_bytes()).expect("DATABASE.DAT should save");

    let stdout = run_maint_rust_with_export(&target, 2);
    assert!(stdout.contains("Turn 1: year 3001"));
    assert!(stdout.contains("Turn 2: year 3002"));

    let game_data = CoreGameData::load(&target).expect("maint-rust output should load");
    let mover_location = game_data.fleets.records[1].current_location_coords_raw();
    assert_ne!(
        mover_location,
        [4, 2],
        "second-turn routing should avoid the now-known foreign world"
    );

    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let viewer_record = database.record(4, 0, game_data.planets.records.len());
    assert_eq!(viewer_record.planet_name_bytes(), b"TargetPrime");
    assert_eq!(viewer_record.raw[0x15], 2);

    cleanup_dir(&target);
}

#[test]
fn maint_rust_guard_starbase_generates_arrival_report() {
    let target = unique_temp_dir("nc-cli-maint-rust-guard-starbase");
    copy_fixture_dir("fixtures/ecmaint-starbase-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let coords = game_data.bases.records[0].coords_raw();
    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw([coords[0].saturating_sub(1), coords[1]]);
    fleet.set_standing_order_kind(Order::GuardStarbase);
    fleet.set_standing_order_target_coords_raw(coords);
    fleet.set_current_speed(3);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Guard Starbase mission report"));
    assert!(text.contains("guard/escort"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_guard_blockade_generates_arrival_report() {
    let target = unique_temp_dir("nc-cli-maint-rust-guard-blockade");
    copy_fixture_dir("fixtures/ecmaint-fleet-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let guard = &mut game_data.fleets.records[0];
    guard.set_standing_order_kind(Order::GuardBlockadeWorld);
    guard.set_standing_order_target_coords_raw([15, 13]);
    guard.set_scout_count(0);
    guard.set_etac_count(0);
    // A destroyer reaches orbit one sector away within the year.
    guard.set_destroyer_count(1);
    guard.recompute_max_speed_from_composition();
    guard.set_current_speed(guard.max_speed());
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(text.contains("Guard/Blockade World mission report"));
    assert!(text.contains("arrived at planet"));
    assert!(text.contains("assignment"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_salvage_generates_report_and_removes_fleet() {
    let target = unique_temp_dir("nc-cli-maint-rust-salvage");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let (planet_idx, target_coords) = game_data
        .planets
        .records
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner_empire_slot_raw() == 1)
        .map(|(idx, planet)| (idx, planet.coords_raw()))
        .expect("fixture should contain an owned planet");
    let start_coords = if target_coords[0] > 1 {
        [target_coords[0] - 1, target_coords[1]]
    } else {
        [target_coords[0] + 1, target_coords[1]]
    };
    let stored_before = game_data.planets.records[planet_idx].stored_production_points();
    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw(start_coords);
    fleet.set_standing_order_kind(Order::Salvage);
    fleet.set_standing_order_target_coords_raw(target_coords);
    fleet.set_current_speed(3);
    fleet.set_destroyer_count(1);
    fleet.set_cruiser_count(1);
    fleet.set_battleship_count(0);
    fleet.set_scout_count(0);
    fleet.set_troop_transport_count(0);
    fleet.set_army_count(0);
    fleet.set_etac_count(0);
    fleet.raw[0x0d] = 0x80;
    fleet.raw[0x0f] = 0;
    fleet.raw[0x19] = 0x00;
    game_data
        .save(&target)
        .expect("mutated fixture should save");
    let db_path = target.join("ncgame.db");
    if db_path.exists() {
        fs::remove_file(&db_path).expect("stale ncgame.db should be removable");
    }

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Salvage mission report"));
    assert!(text.contains("yield 10 production point(s)"));

    let game_data = CoreGameData::load(&target).expect("maint-rust output should load");
    assert_eq!(game_data.fleets.records.len(), 15);
    assert_eq!(
        game_data.planets.records[planet_idx].stored_production_points(),
        stored_before + 10
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_bombardment_generates_attacker_side_report() {
    let target = unique_temp_dir("nc-cli-maint-rust-bombard-report");
    copy_fixture_dir("fixtures/ecmaint-bombard-arrive/v1.5", &target);

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(text.contains("ALERT: Orbital bombardment underway!"));
    assert!(text.contains("bombing run"));
    assert!(text.contains("Our forces:"), "{text}");
    assert!(!text.contains("appeared to contain"));
    assert!(text.contains("World defenses:"), "{text}");
    assert!(!text.contains("We were unable to inflict any ground losses."));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_bombardment_does_not_refresh_database_intel() {
    let target = unique_temp_dir("nc-cli-maint-rust-bombard-database");
    copy_fixture_dir("fixtures/ecmaint-bombard-arrive/v1.5", &target);

    let game_data = CoreGameData::load(&target).expect("fixture should load");
    let database = DatabaseDat::generate_from_planets_and_year(
        &game_data
            .planets
            .records
            .iter()
            .map(|planet| planet.planet_name())
            .collect::<Vec<_>>(),
        game_data.conquest.game_year(),
        game_data.conquest.player_count() as usize,
        None,
    );
    fs::write(target.join("DATABASE.DAT"), database.to_bytes()).expect("DATABASE.DAT should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let reloaded = CoreGameData::load(&target).expect("maint-rust output should load");
    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let viewer_record = database.record(13, 0, reloaded.planets.records.len());
    assert_eq!(viewer_record.planet_name_bytes(), b"UNKNOWN");
    for offset in [
        0x15usize, 0x1c, 0x1d, 0x1e, 0x1f, 0x20, 0x23, 0x24, 0x25, 0x26,
    ] {
        assert_eq!(
            viewer_record.raw[offset], 0xff,
            "offset {offset:#x} should remain unknown after bombard-only intel"
        );
    }

    cleanup_dir(&target);
}

#[test]
fn maint_rust_invade_failure_generates_attacker_side_report() {
    let target = unique_temp_dir("nc-cli-maint-rust-invade-report");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let target_world = &mut game_data.planets.records[13];
    target_world.set_as_owned_target_world(
        [15, 13],
        [0x64, 0x87],
        [0x00, 0x00, 0x00, 0x00, 0x48, 0x87],
        0x04,
        0x0b,
        *b"TargetPrimeet",
        [0x05, 0x1d, 0x0b, 0x11, 0x25, 0x1c, 0x05],
        40,
        0,
        0,
        2,
    );
    let attacker = &mut game_data.fleets.records[0];
    attacker.set_current_location_coords_raw([15, 13]);
    attacker.set_standing_order_kind(Order::InvadeWorld);
    attacker.set_standing_order_target_coords_raw([15, 13]);
    attacker.set_current_speed(0);
    attacker.raw[0x19] = 0x80;
    attacker.set_rules_of_engagement(10);
    attacker.set_scout_count(0);
    attacker.set_battleship_count(0);
    attacker.set_cruiser_count(0);
    attacker.set_destroyer_count(1);
    attacker.set_troop_transport_count(2);
    attacker.set_army_count(2);
    attacker.set_etac_count(0);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(text.contains("ALERT: Planetary invasion repulsed!"));
    assert!(text.contains("Our forces:"), "{text}");
    assert!(text.contains("The landing was repulsed."), "{text}");
    assert!(text.contains("World defenses:"), "{text}");
    assert!(text.contains("Enemy orbital softening losses:"), "{text}");
    assert!(text.contains("Enemy ground battle losses:"), "{text}");
    assert!(!text.contains("initially contained"), "{text}");
    assert!(!text.contains("0 ground battery(ies)"), "{text}");
    assert!(!text.contains("0 army(ies)"), "{text}");
    assert!(text.contains("ground batteries") || text.contains("armies") || text.contains("none"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_invade_failure_exports_ecgame_accepted_enemy_view_row_shape() {
    let target = unique_temp_dir("nc-cli-maint-rust-invade-failure-database");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let target_world = &mut game_data.planets.records[13];
    target_world.set_as_owned_target_world(
        [15, 13],
        [0x64, 0x87],
        [0x00, 0x00, 0x00, 0x00, 0x48, 0x87],
        0x04,
        0x0b,
        *b"TargetPrimeet",
        [0x05, 0x1d, 0x0b, 0x11, 0x25, 0x1c, 0x05],
        40,
        0,
        0,
        2,
    );
    let attacker = &mut game_data.fleets.records[0];
    attacker.set_current_location_coords_raw([15, 13]);
    attacker.set_standing_order_kind(Order::InvadeWorld);
    attacker.set_standing_order_target_coords_raw([15, 13]);
    attacker.set_current_speed(0);
    attacker.raw[0x19] = 0x80;
    attacker.set_rules_of_engagement(10);
    attacker.set_scout_count(0);
    attacker.set_battleship_count(0);
    attacker.set_cruiser_count(0);
    attacker.set_destroyer_count(1);
    attacker.set_troop_transport_count(2);
    attacker.set_army_count(2);
    attacker.set_etac_count(0);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let reloaded = CoreGameData::load(&target).expect("maint-rust output should load");
    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let viewer_record = database.record(13, 0, reloaded.planets.records.len());
    assert_eq!(viewer_record.planet_name_bytes(), b"TargetPrime");
    assert_eq!(viewer_record.raw[0x15], 2);
    assert_eq!(
        u16::from_le_bytes([viewer_record.raw[0x16], viewer_record.raw[0x17]]),
        reloaded.conquest.game_year()
    );
    assert_eq!(
        u16::from_le_bytes([viewer_record.raw[0x18], viewer_record.raw[0x19]]),
        reloaded.conquest.game_year()
    );
    assert_eq!(viewer_record.raw[0x1c], 100);
    assert_eq!(viewer_record.raw[0x1d], 0xff);
    assert_eq!(
        u16::from_le_bytes([viewer_record.raw[0x1e], viewer_record.raw[0x1f]]),
        u16::MAX
    );
    assert_eq!(viewer_record.raw[0x23], 0xff);
    assert_eq!(viewer_record.raw[0x24], 0xff);
    assert_eq!(viewer_record.raw[0x25], 0xff);
    assert_eq!(viewer_record.raw[0x26], 0xff);
    assert_eq!(
        u16::from_le_bytes([viewer_record.raw[0x27], viewer_record.raw[0x28]]),
        0
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_invade_success_exports_ecgame_accepted_owned_row_shape() {
    let target = unique_temp_dir("nc-cli-maint-rust-invade-success-database");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let target_world = &mut game_data.planets.records[13];
    target_world.set_as_owned_target_world(
        [15, 13],
        [0x64, 0x87],
        [0x00, 0x00, 0x00, 0x00, 0x48, 0x87],
        0x04,
        0x0b,
        *b"TargetPrimeet",
        [0x05, 0x1d, 0x0b, 0x11, 0x25, 0x1c, 0x05],
        142,
        15,
        0,
        2,
    );
    let attacker = &mut game_data.fleets.records[0];
    attacker.set_current_location_coords_raw([15, 13]);
    attacker.set_standing_order_kind(Order::InvadeWorld);
    attacker.set_standing_order_target_coords_raw([15, 13]);
    attacker.set_current_speed(0);
    attacker.raw[0x19] = 0x80;
    attacker.set_rules_of_engagement(10);
    attacker.set_scout_count(0);
    attacker.set_battleship_count(20);
    attacker.set_cruiser_count(20);
    attacker.set_destroyer_count(20);
    attacker.set_troop_transport_count(120);
    attacker.set_army_count(120);
    attacker.set_etac_count(0);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let reloaded = CoreGameData::load(&target).expect("maint-rust output should load");
    let expected_year = reloaded.conquest.game_year() - 1;
    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let viewer_record = database.record(13, 0, reloaded.planets.records.len());
    assert_eq!(viewer_record.planet_name_bytes(), b"TargetPrime");
    assert_eq!(viewer_record.raw[0x15], 1);
    assert_eq!(
        u16::from_le_bytes([viewer_record.raw[0x16], viewer_record.raw[0x17]]),
        expected_year
    );
    assert_eq!(
        u16::from_le_bytes([viewer_record.raw[0x18], viewer_record.raw[0x19]]),
        expected_year
    );
    assert_eq!(viewer_record.raw[0x1c], 100);
    assert_eq!(viewer_record.raw[0x1d], 100);
    assert_eq!(
        u16::from_le_bytes([viewer_record.raw[0x1e], viewer_record.raw[0x1f]]),
        65
    );
    assert_eq!(
        viewer_record.raw[0x23],
        reloaded.planets.records[13].army_count_raw()
    );
    assert_eq!(viewer_record.raw[0x24], 0x00);
    assert_eq!(viewer_record.raw[0x25], 0);
    assert_eq!(viewer_record.raw[0x26], 0x00);
    assert_eq!(
        u16::from_le_bytes([viewer_record.raw[0x27], viewer_record.raw[0x28]]),
        expected_year
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_invasion_success_reports_armies_for_attacker_and_defender() {
    let target = unique_temp_dir("nc-cli-maint-rust-invade-army-report");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let target_world = &mut game_data.planets.records[13];
    target_world.set_as_owned_target_world(
        [15, 13],
        [0x64, 0x87],
        [0x00, 0x00, 0x00, 0x00, 0x48, 0x87],
        0x04,
        0x0b,
        *b"TargetPrimeet",
        [0x05, 0x1d, 0x0b, 0x11, 0x25, 0x1c, 0x05],
        142,
        15,
        0,
        2,
    );
    let attacker = &mut game_data.fleets.records[0];
    attacker.set_current_location_coords_raw([15, 13]);
    attacker.set_standing_order_kind(Order::InvadeWorld);
    attacker.set_standing_order_target_coords_raw([15, 13]);
    attacker.set_current_speed(0);
    attacker.raw[0x19] = 0x80;
    attacker.set_rules_of_engagement(10);
    attacker.set_scout_count(0);
    attacker.set_battleship_count(20);
    attacker.set_cruiser_count(20);
    attacker.set_destroyer_count(20);
    attacker.set_troop_transport_count(120);
    attacker.set_army_count(120);
    attacker.set_etac_count(0);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let attacker_report =
        joined_report_containing(&results, "ALERT: Planetary invasion successful!");
    assert!(
        attacker_report.contains("World defenses:   15 ground batteries and 142 armies"),
        "{attacker_report}"
    );
    assert!(
        attacker_report.contains("Enemy losses:     15 ground batteries and 142 armies"),
        "{attacker_report}"
    );
    assert!(
        attacker_report.contains("Enemy orbital softening losses: 71 armies"),
        "{attacker_report}"
    );
    assert!(
        attacker_report.contains("Enemy ground battle losses: 71 armies"),
        "{attacker_report}"
    );

    let defender_report =
        joined_report_containing(&results, "ALERT: Planet lost to enemy invasion!");
    assert!(
        defender_report.contains("Attacking force:"),
        "{defender_report}"
    );
    assert!(
        defender_report.contains("20BB, 20CA, 20DD"),
        "{defender_report}"
    );
    assert!(defender_report.contains("120TT*"), "{defender_report}");
    assert!(
        defender_report.contains("Our defenses:     15 ground batteries and 142 armies"),
        "{defender_report}"
    );
    assert!(
        defender_report.contains("Our orbital softening losses: 71 armies"),
        "{defender_report}"
    );
    assert!(
        defender_report.contains("Our ground battle losses: 71 armies"),
        "{defender_report}"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_invasion_reports_arrival_and_execution_on_separate_turns() {
    let target = unique_temp_dir("nc-cli-maint-rust-invade-arrival-delay");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);
    configure_delayed_invade_arrival_directory(&target);

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let first_results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let first_text = decode_chunked_report(&first_results);
    assert!(first_text.contains("preparing"), "{first_text}");
    assert!(first_text.contains("invasion"), "{first_text}");
    assert!(!first_text.contains("Target world:"), "{first_text}");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let second_results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let second_text = decode_chunked_report(&second_results);
    assert!(
        second_text.contains("ALERT: Planetary invasion successful!"),
        "{second_text}"
    );
    assert!(second_text.contains("Target world:"), "{second_text}");
    assert!(
        !second_text.contains("Our armies have captured planet"),
        "{second_text}"
    );
    assert!(
        !second_text.contains(
            "We have arrived at our target world and are preparing to begin the invasion."
        ),
        "{second_text}"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_blitz_success_generates_attacker_side_report() {
    let target = unique_temp_dir("nc-cli-maint-rust-blitz-report");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let target_world = &mut game_data.planets.records[13];
    target_world.set_as_owned_target_world(
        [15, 13],
        [0x64, 0x87],
        [0x00, 0x00, 0x00, 0x00, 0x48, 0x87],
        0x04,
        0x0b,
        *b"TargetPrimeet",
        [0x05, 0x1d, 0x0b, 0x11, 0x25, 0x1c, 0x05],
        1,
        1,
        0,
        2,
    );
    let attacker = &mut game_data.fleets.records[0];
    attacker.set_current_location_coords_raw([15, 13]);
    attacker.set_standing_order_kind(Order::BlitzWorld);
    attacker.set_standing_order_target_coords_raw([15, 13]);
    attacker.set_current_speed(0);
    attacker.raw[0x19] = 0x80;
    attacker.set_rules_of_engagement(10);
    attacker.set_scout_count(0);
    // Cover fire must pay the battery's 20-hit DS; a lone destroyer cannot.
    attacker.set_battleship_count(1);
    attacker.set_cruiser_count(0);
    attacker.set_destroyer_count(1);
    attacker.set_troop_transport_count(10);
    attacker.set_army_count(10);
    attacker.set_etac_count(0);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(text.contains("ALERT: Blitz assault successful!"), "{text}");
    assert!(text.contains("We have seized planet"), "{text}");
    assert!(
        text.contains("in a fast") && text.contains("assault."),
        "{text}"
    );
    assert!(text.contains("Our forces:"), "{text}");
    assert!(text.contains("World defenses:"), "{text}");
    assert!(text.contains("Our losses:"), "{text}");
    assert!(text.contains("Enemy") && text.contains("losses:"), "{text}");
    assert!(text.contains("Transport losses:"), "{text}");
    assert!(text.matches("ALERT: Blitz assault successful!").count() >= 1);

    cleanup_dir(&target);
}

#[test]
fn maint_rust_blitz_success_reports_armies_for_attacker_and_defender() {
    let target = unique_temp_dir("nc-cli-maint-rust-blitz-army-report");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let target_world = &mut game_data.planets.records[13];
    target_world.set_as_owned_target_world(
        [15, 13],
        [0x64, 0x87],
        [0x00, 0x00, 0x00, 0x00, 0x48, 0x87],
        0x04,
        0x0b,
        *b"TargetPrimeet",
        [0x05, 0x1d, 0x0b, 0x11, 0x25, 0x1c, 0x05],
        1,
        1,
        0,
        2,
    );
    let attacker = &mut game_data.fleets.records[0];
    attacker.set_current_location_coords_raw([15, 13]);
    attacker.set_standing_order_kind(Order::BlitzWorld);
    attacker.set_standing_order_target_coords_raw([15, 13]);
    attacker.set_current_speed(0);
    attacker.raw[0x19] = 0x80;
    attacker.set_rules_of_engagement(10);
    attacker.set_scout_count(0);
    // Cover fire must pay the battery's 20-hit DS; a lone destroyer cannot.
    attacker.set_battleship_count(1);
    attacker.set_cruiser_count(0);
    attacker.set_destroyer_count(1);
    attacker.set_troop_transport_count(10);
    attacker.set_army_count(10);
    attacker.set_etac_count(0);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let attacker_report = joined_report_containing(&results, "ALERT: Blitz assault successful!");
    assert!(
        attacker_report.contains("World defenses:   1 ground battery and 1 army"),
        "{attacker_report}"
    );
    assert!(
        attacker_report.contains("Enemy losses:     1 ground battery and 1 army"),
        "{attacker_report}"
    );

    let defender_report = joined_report_containing(&results, "ALERT: Planet lost to enemy blitz!");
    assert!(
        defender_report.contains("Attacking force:"),
        "{defender_report}"
    );
    assert!(defender_report.contains("1DD"), "{defender_report}");
    assert!(defender_report.contains("10TT*"), "{defender_report}");
    assert!(
        defender_report.contains("Our defenses:     1 ground battery and 1 army"),
        "{defender_report}"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_blitz_success_exports_ecgame_accepted_owned_row_shape() {
    let target = unique_temp_dir("nc-cli-maint-rust-blitz-success-database");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let target_world = &mut game_data.planets.records[13];
    target_world.set_as_owned_target_world(
        [15, 13],
        [0x64, 0x87],
        [0x00, 0x00, 0x00, 0x00, 0x48, 0x87],
        0x04,
        0x0b,
        *b"TargetPrimeet",
        [0x05, 0x1d, 0x0b, 0x11, 0x25, 0x1c, 0x05],
        1,
        1,
        0,
        2,
    );
    let attacker = &mut game_data.fleets.records[0];
    attacker.set_current_location_coords_raw([15, 13]);
    attacker.set_standing_order_kind(Order::BlitzWorld);
    attacker.set_standing_order_target_coords_raw([15, 13]);
    attacker.set_current_speed(0);
    attacker.raw[0x19] = 0x80;
    attacker.set_rules_of_engagement(10);
    attacker.set_scout_count(0);
    // Cover fire must pay the battery's 20-hit DS; a lone destroyer cannot.
    attacker.set_battleship_count(1);
    attacker.set_cruiser_count(0);
    attacker.set_destroyer_count(1);
    attacker.set_troop_transport_count(10);
    attacker.set_army_count(10);
    attacker.set_etac_count(0);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(
        text.contains("ALERT: Blitz assault successful!") && text.contains("We have seized planet"),
        "{text}"
    );
    assert!(text.contains("Our forces:"), "{text}");

    let reloaded = CoreGameData::load(&target).expect("maint-rust output should load");
    let expected_year = reloaded.conquest.game_year() - 1;
    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let viewer_record = database.record(13, 0, reloaded.planets.records.len());
    assert_eq!(viewer_record.planet_name_bytes(), b"TargetPrime");
    assert_eq!(viewer_record.raw[0x15], 1);
    assert_eq!(
        u16::from_le_bytes([viewer_record.raw[0x16], viewer_record.raw[0x17]]),
        expected_year
    );
    assert_eq!(
        u16::from_le_bytes([viewer_record.raw[0x18], viewer_record.raw[0x19]]),
        expected_year
    );
    assert_eq!(viewer_record.raw[0x1c], 100);
    assert_eq!(viewer_record.raw[0x1d], 100);
    assert_eq!(
        u16::from_le_bytes([viewer_record.raw[0x1e], viewer_record.raw[0x1f]]),
        65
    );
    // Seeded garrison: 10 landed armies less 1 lost in the ground battle.
    assert_eq!(viewer_record.raw[0x23], 9);
    assert_eq!(viewer_record.raw[0x24], 0x00);
    assert_eq!(viewer_record.raw[0x25], 0);
    assert_eq!(viewer_record.raw[0x26], 0x00);
    assert_eq!(
        u16::from_le_bytes([viewer_record.raw[0x27], viewer_record.raw[0x28]]),
        expected_year
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_named_homeworld_seed_preserves_oracle_compat_word_family() {
    let target = unique_temp_dir("nc-cli-maint-rust-homeworld-seed-database");
    copy_fixture_dir("fixtures/ecutil-init/v1.5", &target);

    let prepare_stdout = run_nc_cli_in_dir(
        &[
            "classic-login-prepare",
            target.to_str().unwrap(),
            "1",
            "SYSOP",
            "Auroran_Combine",
        ],
        common::rust_workspace(),
    );
    assert!(prepare_stdout.contains("Prepared classic login for player 1"));

    let mut game_data = CoreGameData::load(&target).expect("prepared fixture should load");
    game_data.planets.records[14].set_planet_name("ssddsf");
    game_data.save(&target).expect("named fixture should save");

    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let mut database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    database
        .record_mut(14, 0, game_data.planets.records.len())
        .set_planet_name("ssddsf");
    fs::write(target.join("DATABASE.DAT"), database.to_bytes()).expect("DATABASE.DAT should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let reloaded = CoreGameData::load(&target).expect("maint-rust output should load");
    let expected_year = reloaded.conquest.game_year() - 1;
    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let viewer_record = database.record(14, 0, reloaded.planets.records.len());
    assert_eq!(viewer_record.planet_name_bytes(), b"ssddsf");
    assert_eq!(viewer_record.raw[0x15], 1);
    assert_eq!(
        u16::from_le_bytes([viewer_record.raw[0x16], viewer_record.raw[0x17]]),
        expected_year
    );
    assert_eq!(
        u16::from_le_bytes([viewer_record.raw[0x18], viewer_record.raw[0x19]]),
        expected_year
    );
    assert_eq!(viewer_record.raw[0x1c], 100);
    assert_eq!(viewer_record.raw[0x1d], 100);
    assert_eq!(
        u16::from_le_bytes([viewer_record.raw[0x1e], viewer_record.raw[0x1f]]),
        0x23
    );
    assert_eq!(viewer_record.raw[0x23], 10);
    assert_eq!(viewer_record.raw[0x24], 0x00);
    assert_eq!(viewer_record.raw[0x25], 4);
    assert_eq!(viewer_record.raw[0x26], 0x00);
    assert_eq!(
        u16::from_le_bytes([viewer_record.raw[0x27], viewer_record.raw[0x28]]),
        expected_year
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_preserves_loaded_homeworld_seed_orbit_family_for_all_players() {
    let target = unique_temp_dir("nc-cli-maint-rust-homeworld-seed-all-players");
    copy_fixture_dir("fixtures/ecutil-init/v1.5", &target);

    let prepare_stdout = run_nc_cli_in_dir(
        &[
            "classic-login-prepare",
            target.to_str().unwrap(),
            "1",
            "SYSOP",
            "Auroran_Combine",
        ],
        common::rust_workspace(),
    );
    assert!(prepare_stdout.contains("Prepared classic login for player 1"));

    let mut game_data = CoreGameData::load(&target).expect("prepared fixture should load");
    game_data.planets.records[14].set_planet_name("ssddsf");
    game_data.save(&target).expect("named fixture should save");

    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let mut database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    database
        .record_mut(14, 0, game_data.planets.records.len())
        .set_planet_name("ssddsf");
    fs::write(target.join("DATABASE.DAT"), database.to_bytes()).expect("DATABASE.DAT should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let reloaded = CoreGameData::load(&target).expect("maint-rust output should load");
    let expected_year = reloaded.conquest.game_year() - 1;
    let planet_count = reloaded.planets.records.len();
    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");

    for (planet_idx, player_idx, expected_name, expected_owner) in [
        (14usize, 0usize, "ssddsf", 1u8),
        (12, 1, "Not Named Yet", 2),
        (4, 2, "Not Named Yet", 3),
        (5, 3, "Not Named Yet", 4),
    ] {
        let row = database.record(planet_idx, player_idx, planet_count);
        assert_eq!(row.planet_name_bytes(), expected_name.as_bytes());
        assert_eq!(row.raw[0x15], expected_owner);
        assert_eq!(
            u16::from_le_bytes([row.raw[0x16], row.raw[0x17]]),
            expected_year
        );
        assert_eq!(
            u16::from_le_bytes([row.raw[0x18], row.raw[0x19]]),
            expected_year
        );
        assert_eq!(row.raw[0x1c], 100);
        assert_eq!(row.raw[0x1d], 100);
        assert_eq!(u16::from_le_bytes([row.raw[0x1e], row.raw[0x1f]]), 0x23);
        assert_eq!(row.raw[0x23], 10);
        assert_eq!(row.raw[0x24], 0x00);
        assert_eq!(row.raw[0x25], 4);
        assert_eq!(row.raw[0x26], 0x00);
        assert_eq!(
            u16::from_le_bytes([row.raw[0x27], row.raw[0x28]]),
            expected_year
        );
    }

    cleanup_dir(&target);
}

#[test]
#[ignore = "launches classic ECGAME through dosbox-x"]
fn maint_rust_loaded_homeworld_seed_orbit_family_reopens_in_classic_ecgame_smoke() {
    let target = unique_temp_dir("nc-cli-maint-rust-homeworld-seed-ecgame");
    copy_fixture_dir("fixtures/ecutil-init/v1.5", &target);

    let prepare_stdout = run_nc_cli_in_dir(
        &[
            "classic-login-prepare",
            target.to_str().unwrap(),
            "1",
            "SYSOP",
            "Auroran_Combine",
        ],
        common::rust_workspace(),
    );
    assert!(prepare_stdout.contains("Prepared classic login for player 1"));

    let mut game_data = CoreGameData::load(&target).expect("prepared fixture should load");
    game_data.planets.records[14].set_planet_name("ssddsf");
    game_data.save(&target).expect("named fixture should save");

    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let mut database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    database
        .record_mut(14, 0, game_data.planets.records.len())
        .set_planet_name("ssddsf");
    fs::write(target.join("DATABASE.DAT"), database.to_bytes()).expect("DATABASE.DAT should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    run_classic_ecgame_smoke_with_alias(&target, 1, "SYSOP");

    cleanup_dir(&target);
}

#[test]
fn maint_rust_blitz_failure_exports_ecgame_accepted_enemy_view_row_shape() {
    let target = unique_temp_dir("nc-cli-maint-rust-blitz-failure-database");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let target_world = &mut game_data.planets.records[13];
    target_world.set_as_owned_target_world(
        [15, 13],
        [0x64, 0x87],
        [0x00, 0x00, 0x00, 0x00, 0x48, 0x87],
        0x04,
        0x0b,
        *b"TargetPrimeet",
        [0x05, 0x1d, 0x0b, 0x11, 0x25, 0x1c, 0x05],
        10,
        2,
        0,
        2,
    );
    let attacker = &mut game_data.fleets.records[0];
    attacker.set_current_location_coords_raw([15, 13]);
    attacker.set_standing_order_kind(Order::BlitzWorld);
    attacker.set_standing_order_target_coords_raw([15, 13]);
    attacker.set_current_speed(0);
    attacker.raw[0x19] = 0x80;
    attacker.set_rules_of_engagement(10);
    attacker.set_scout_count(0);
    attacker.set_battleship_count(0);
    attacker.set_cruiser_count(0);
    attacker.set_destroyer_count(1);
    attacker.set_troop_transport_count(4);
    attacker.set_army_count(4);
    attacker.set_etac_count(0);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let reloaded = CoreGameData::load(&target).expect("maint-rust output should load");
    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let viewer_record = database.record(13, 0, reloaded.planets.records.len());
    assert_eq!(viewer_record.planet_name_bytes(), b"TargetPrime");
    assert_eq!(viewer_record.raw[0x15], 2);
    assert_eq!(
        u16::from_le_bytes([viewer_record.raw[0x16], viewer_record.raw[0x17]]),
        reloaded.conquest.game_year()
    );
    assert_eq!(
        u16::from_le_bytes([viewer_record.raw[0x18], viewer_record.raw[0x19]]),
        reloaded.conquest.game_year()
    );
    assert_eq!(viewer_record.raw[0x1c], 100);
    assert_eq!(viewer_record.raw[0x1d], 0xff);
    assert_eq!(
        u16::from_le_bytes([viewer_record.raw[0x1e], viewer_record.raw[0x1f]]),
        u16::MAX
    );
    assert_eq!(viewer_record.raw[0x23], 0xff);
    assert_eq!(viewer_record.raw[0x24], 0xff);
    assert_eq!(viewer_record.raw[0x25], 0xff);
    assert_eq!(viewer_record.raw[0x26], 0xff);
    assert_eq!(
        u16::from_le_bytes([viewer_record.raw[0x27], viewer_record.raw[0x28]]),
        0
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_battle_abort_generates_move_abort_report() {
    let target = unique_temp_dir("nc-cli-maint-rust-battle-abort");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);
    write_mutual_enemy_diplomacy(&target, 1, 2);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    game_data.fleets.records[0].set_standing_order_kind(Order::MoveOnly);
    // Give the fleet enough scouts to survive the battle (loses but not destroyed).
    // Max scouts (u8::MAX) to survive against 100 enemy battleships.
    game_data.fleets.records[0].set_scout_count(255);
    // Reduce enemy fleet 4 (empire 2 at same coords) to 1 BB so our fleet survives.
    game_data.fleets.records[4].set_battleship_count(1);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(text.contains("Move mission report"));
    assert!(
        text.contains("abort our mission")
            || text.contains("abort our")
            || text.contains("arrived at our destination")
    );
    // Under the corrected MoveOnly semantics, hostile contact after arrival
    // can yield a normal arrival-complete report instead of an abort report.
    assert!(
        text.contains("seek safe")
            || text.contains("abort our mission")
            || text.contains("arrived at our destination")
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_dominant_invalidated_invade_report_retreats_after_capability_loss() {
    let target = unique_temp_dir("nc-cli-maint-rust-invade-abort-hold");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);
    write_mutual_enemy_diplomacy(&target, 1, 2);
    configure_dominant_invalidated_invade_directory(&target);

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let reloaded = CoreGameData::load(&target).expect("maint-rust output should load");
    // The battle comes mid-year, so the fleet may reach home (16,13) and
    // stop there before the year ends.
    let attacker = &reloaded.fleets.records[0];
    assert!(matches!(
        attacker.standing_order_kind(),
        Order::SeekHome | Order::HoldPosition
    ));
    assert_eq!(attacker.standing_order_target_coords_raw(), [16, 13]);

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let normalized = results_records(&results)
        .into_iter()
        .map(result_record_text)
        .collect::<Vec<_>>()
        .join(" ");
    assert!(normalized.contains("Invasion mission report"));
    assert!(normalized.contains("Hostile action stripped us of our invasion capability"));
    assert!(
        normalized.contains("withdrawing toward planet"),
        "{normalized}"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_enemy_roe_withdrawal_does_not_abort_invade_mission() {
    let target = unique_temp_dir("nc-cli-maint-rust-enemy-roe-withdrawal-invade");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);
    write_mutual_enemy_diplomacy(&target, 1, 2);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let coords = [9, 13];

    let attacker = &mut game_data.fleets.records[0];
    attacker.set_current_location_coords_raw(coords);
    attacker.set_standing_order_kind(Order::InvadeWorld);
    attacker.set_standing_order_target_coords_raw([15, 13]);
    attacker.set_current_speed(0);
    attacker.set_destroyer_count(2);
    attacker.set_cruiser_count(4);
    attacker.set_battleship_count(1);
    attacker.set_scout_count(0);
    attacker.set_troop_transport_count(2);
    attacker.set_army_count(2);
    attacker.set_etac_count(0);
    attacker.set_rules_of_engagement(10);
    // Stalled in place (no speed to restore), so the contact happens here.
    attacker.set_max_speed(0);

    let hostile = &mut game_data.fleets.records[4];
    hostile.set_current_location_coords_raw(coords);
    hostile.set_standing_order_kind(Order::MoveOnly);
    hostile.set_standing_order_target_coords_raw([8, 13]);
    hostile.set_current_speed(0);
    hostile.set_destroyer_count(0);
    hostile.set_cruiser_count(2);
    hostile.set_battleship_count(0);
    hostile.set_scout_count(0);
    hostile.set_troop_transport_count(0);
    hostile.set_army_count(0);
    // ETACs carry no guns but make the fleet easy to see (14% a tick), so
    // the invader sights it within the year whatever the encounter rolls.
    hostile.set_etac_count(500);
    hostile.set_rules_of_engagement(0);
    // Stalled in place (no speed to restore), so the contact happens here.
    hostile.set_max_speed(0);

    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let reloaded = CoreGameData::load(&target).expect("maint-rust output should load");
    let attacker = &reloaded.fleets.records[0];
    assert_eq!(attacker.standing_order_kind(), Order::InvadeWorld);
    assert_eq!(attacker.standing_order_target_coords_raw(), [15, 13]);
    assert_ne!(attacker.standing_order_kind(), Order::SeekHome);

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let invasion_report = joined_report_containing(&results, "Invasion mission report");
    assert!(
        invasion_report.contains("Sensor contact"),
        "{invasion_report}"
    );
    assert!(
        !invasion_report.contains("In accordance with our ROE, we withdrew"),
        "{invasion_report}"
    );
    assert!(
        !invasion_report.contains("abort the invasion before the landing could begin."),
        "{invasion_report}"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_blitz_clears_hostile_empty_transports_in_orbit() {
    let target = unique_temp_dir("nc-cli-maint-rust-blitz-clears-empty-transports");
    fs::create_dir_all(&target).expect("temp dir should exist");

    let mut game_data = GameStateBuilder::new()
        .with_player_count(2)
        .build_joinable_new_game_baseline()
        .expect("baseline should build");
    game_data.join_player(1, "Player1").expect("join player 1");
    game_data.join_player(2, "Player2").expect("join player 2");
    let target_coords = game_data
        .planets
        .records
        .iter()
        .find(|planet| planet.owner_empire_slot_raw() == 2)
        .map(|planet| planet.coords_raw())
        .expect("player 2 should own a world");

    zero_all_fleets(&mut game_data);

    let attacker_idx = game_data
        .fleets
        .records
        .iter()
        .position(|fleet| fleet.owner_empire_raw() == 1)
        .expect("player 1 fleet");
    let defender_idx = game_data
        .fleets
        .records
        .iter()
        .position(|fleet| fleet.owner_empire_raw() == 2)
        .expect("player 2 fleet");

    let attacker = &mut game_data.fleets.records[attacker_idx];
    attacker.set_current_location_coords_raw(target_coords);
    attacker.set_standing_order_kind(Order::BlitzWorld);
    attacker.set_standing_order_target_coords_raw(target_coords);
    attacker.set_current_speed(0);
    attacker.set_battleship_count(4);
    attacker.set_cruiser_count(8);
    attacker.set_destroyer_count(6);
    attacker.set_troop_transport_count(12);
    attacker.set_army_count(12);
    attacker.set_rules_of_engagement(10);

    let defender = &mut game_data.fleets.records[defender_idx];
    defender.set_current_location_coords_raw(target_coords);
    defender.set_standing_order_kind(Order::HoldPosition);
    defender.set_standing_order_target_coords_raw(target_coords);
    defender.set_current_speed(0);
    defender.set_troop_transport_count(25);
    defender.set_army_count(0);
    defender.set_rules_of_engagement(10);

    let target_world = game_data
        .planets
        .records
        .iter_mut()
        .find(|planet| planet.coords_raw() == target_coords)
        .expect("target world should exist");
    target_world.set_ground_batteries_raw(0);
    target_world.set_army_count_raw(2);

    game_data.save(&target).expect("scenario should save");
    write_mutual_enemy_diplomacy(&target, 1, 2);

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let attacker_report = joined_report_containing(&results, "ALERT: Enemy fleet contact!");
    assert!(attacker_report.contains("The aliens were completely destroyed."));
    let defender_report = joined_report_containing(&results, "ALERT: Fleet contact lost!");
    assert!(defender_report.contains("Fleet lost:"));

    let reloaded = CoreGameData::load(&target).expect("maint-rust output should load");
    let defender = reloaded
        .fleets
        .records
        .iter()
        .find(|fleet| fleet.owner_empire_raw() == 2)
        .expect("player 2 fleet should remain addressable");
    assert_eq!(defender.troop_transport_count(), 0);

    cleanup_dir(&target);
}

#[test]
fn maint_rust_invalid_fleet_order_generates_sanitization_report() {
    let target = unique_temp_dir("nc-cli-maint-rust-invalid-fleet-order");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw([15, 13]);
    fleet.set_standing_order_kind(Order::BombardWorld);
    fleet.set_standing_order_target_coords_raw([15, 13]);
    fleet.set_current_speed(0);
    fleet.raw[0x19] = 0x80;
    fleet.set_destroyer_count(0);
    fleet.set_cruiser_count(0);
    fleet.set_battleship_count(0);
    fleet.set_scout_count(1);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(text.contains("Hostile action forced us to abort the"));
    assert!(text.contains("required combat ships"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_invalid_planet_inputs_generate_admin_report() {
    let target = unique_temp_dir("nc-cli-maint-rust-invalid-planet-input");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let planet_idx = game_data
        .planets
        .records
        .iter()
        .position(|planet| planet.owner_empire_slot_raw() == 1)
        .expect("fixture should contain owned planet");
    game_data.planets.records[planet_idx].set_build_count_raw(0, 12);
    game_data.planets.records[planet_idx].set_build_kind_raw(0, 0xfe);
    game_data.player.records[0].set_tax_rate_raw(255);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(text.contains("Invalid planetary input was cleared because"));
    assert!(text.contains("Tax rate input 255%"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_sanitizes_mixed_invalid_player_inputs_and_exports_loadable_state() {
    let target = unique_temp_dir("nc-cli-maint-rust-mixed-invalid-inputs");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw([15, 13]);
    fleet.set_standing_order_kind(Order::BombardWorld);
    fleet.set_standing_order_target_coords_raw([15, 13]);
    fleet.set_current_speed(fleet.max_speed().saturating_add(3));
    fleet.set_destroyer_count(0);
    fleet.set_cruiser_count(0);
    fleet.set_battleship_count(0);
    fleet.set_scout_count(1);
    fleet.set_rules_of_engagement(6);
    // Planet record 15 is Empire #1's world; invalid-input reports for
    // unowned planets are not delivered to anyone.
    game_data.planets.records[14].set_build_count_raw(0, 9);
    game_data.planets.records[14].set_build_kind_raw(0, 0xfe);
    game_data.player.records[0].set_tax_rate_raw(255);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let reloaded = CoreGameData::load(&target).expect("maint-rust output should remain loadable");
    assert_eq!(
        reloaded.fleets.records[0].standing_order_kind(),
        Order::HoldPosition
    );
    assert_eq!(reloaded.fleets.records[0].current_speed(), 0);
    assert_eq!(reloaded.fleets.records[0].rules_of_engagement(), 0);
    assert_eq!(reloaded.player.records[0].tax_rate(), 100);
    assert_eq!(reloaded.planets.records[14].build_count_raw(0), 0);
    assert_eq!(reloaded.planets.records[14].build_kind_raw(0), 0);

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let result_text = decode_chunked_report(&results);
    assert!(result_text.contains("Hostile action forced us to abort the"));
    assert!(result_text.contains("Invalid fleet input was corrected because"));
    assert!(result_text.contains("Invalid planetary input was cleared because"));
    assert!(result_text.contains("Tax rate input 255%"));

    let messages = fs::read(target.join("MESSAGES.DAT")).expect("MESSAGES.DAT should exist");
    assert!(messages.is_empty());

    cleanup_dir(&target);
}

#[test]
fn maint_rust_survives_deterministic_malformed_directory_matrix() {
    for order_code in [16u8, 17, 24, 31] {
        let target = unique_temp_dir(&format!("nc-cli-maint-rust-invalid-matrix-{order_code}"));
        copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

        let mut game_data = CoreGameData::load(&target).expect("fixture should load");
        let fleet = &mut game_data.fleets.records[0];
        fleet.set_current_location_coords_raw([15, 13]);
        fleet.set_standing_order_code_raw(order_code);
        fleet.set_standing_order_target_coords_raw([15, 13]);
        fleet.set_current_speed(99);
        fleet.set_mission_aux_bytes([0xfe, 0xfe]);
        fleet.set_destroyer_count(0);
        fleet.set_cruiser_count(0);
        fleet.set_battleship_count(0);
        fleet.set_scout_count(1);
        fleet.set_troop_transport_count(1);
        fleet.set_army_count(4);
        fleet.set_rules_of_engagement(42);
        game_data.planets.records[0].set_build_count_raw(0, 9);
        game_data.planets.records[0].set_build_kind_raw(0, 0xfe);
        game_data.planets.records[0].set_stardock_count_raw(0, 2);
        game_data.player.records[0].set_tax_rate_raw(255);
        game_data
            .save(&target)
            .expect("mutated fixture should save");

        let stdout = run_maint_rust_with_export(&target, 1);
        assert!(
            stdout.contains("Rust maintenance complete."),
            "maint-rust failed for order code {order_code:#04x}: {stdout}"
        );

        let reloaded =
            CoreGameData::load(&target).expect("maint-rust output should remain loadable");
        assert_eq!(
            reloaded.player.records[0].tax_rate(),
            100,
            "tax rate should clamp for order code {order_code:#04x}"
        );

        let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
        let result_text = decode_chunked_report(&results);
        assert!(
            result_text.contains("Maintenance canceled this fleet's")
                || result_text.contains("Maintenance aborted this fleet's")
                || result_text.contains("Invalid fleet input was corrected because"),
            "RESULTS.DAT decoded text was: {:?}",
            result_text
        );

        cleanup_dir(&target);
    }
}

#[test]
fn maint_rust_survives_multi_fixture_invalid_input_sweep() {
    for fixture in [
        "fixtures/ecmaint-post/v1.5",
        "fixtures/ecmaint-fleet-pre/v1.5",
        "fixtures/ecmaint-fleet-battle-pre/v1.5",
    ] {
        let slug = fixture
            .split('/')
            .nth(1)
            .unwrap_or("fixture")
            .replace("ecmaint-", "");
        let target = unique_temp_dir(&format!("nc-cli-maint-rust-sweep-{slug}"));
        copy_fixture_dir(fixture, &target);

        let mut game_data = CoreGameData::load(&target).expect("fixture should load");
        if let Some(fleet) = game_data.fleets.records.get_mut(0) {
            fleet.set_standing_order_code_raw(0xfe);
            fleet.set_current_speed(99);
            fleet.set_troop_transport_count(1);
            fleet.set_army_count(4);
            fleet.set_rules_of_engagement(42);
        }
        if let Some(planet) = game_data.planets.records.get_mut(0) {
            planet.set_build_count_raw(0, 9);
            planet.set_build_kind_raw(0, 0xfe);
            planet.set_stardock_count_raw(0, 2);
        }
        if let Some(player) = game_data.player.records.get_mut(0) {
            player.set_tax_rate_raw(255);
            player.raw[0x54] = 0x01;
            player.raw[0x55] = 0xfe;
        }
        game_data
            .save(&target)
            .expect("mutated fixture should save");

        let stdout = run_maint_rust_with_export(&target, 1);
        assert!(stdout.contains("Rust maintenance complete."));

        let reloaded = CoreGameData::load(&target).expect("maint-rust output should load");
        assert!(
            reloaded.player.records[0].tax_rate() <= 100,
            "fixture {fixture} left invalid tax rate {}",
            reloaded.player.records[0].tax_rate()
        );
        let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
        let text = decode_chunked_report(&results);
        assert!(
            text.contains("Maintenance canceled this fleet's")
                || text.contains("Maintenance aborted this fleet's")
                || text.contains("Invalid fleet input was corrected because")
                || text.contains("foreign ministry"),
            "fixture {fixture} produced unexpected RESULTS.DAT text: {:?}",
            text
        );

        cleanup_dir(&target);
    }
}

#[test]
fn maint_rust_reports_invalid_diplomacy_input_sanitization() {
    let target = unique_temp_dir("nc-cli-maint-rust-invalid-diplomacy");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    game_data.player.records[0].raw[0x54] = 0x01;
    game_data.player.records[0].raw[0x55] = 0xfe;
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let reloaded = CoreGameData::load(&target).expect("maint-rust output should remain loadable");
    assert_eq!(reloaded.player.records[0].raw[0x54], 0x00);
    assert_eq!(reloaded.player.records[0].raw[0x55], 0x00);

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(text.contains("foreign ministry"));
    assert!(text.contains("Invalid diplomacy input"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_battle_abort_scout_report_mentions_retreat_destination() {
    let target = unique_temp_dir("nc-cli-maint-rust-scout-abort");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);
    write_mutual_enemy_diplomacy(&target, 1, 2);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    game_data.fleets.records[0].set_standing_order_kind(Order::ScoutSector);
    // Remove big ships so fleet loses the battle (scouts absorb hits and survive).
    game_data.fleets.records[0].set_battleship_count(0);
    game_data.fleets.records[0].set_cruiser_count(0);
    game_data.fleets.records[0].set_destroyer_count(0);
    // Max scouts to absorb hits without being fully destroyed.
    game_data.fleets.records[0].set_scout_count(255);
    // Reduce enemy fleet 4 (empire 2 at same coords) to 1 BB.
    game_data.fleets.records[4].set_battleship_count(1);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    // The patrol, looking ten times as hard, usually catches the scouts before
    // they report, so either side's contact report will do.
    assert!(
        text.contains("Sensor contact") || text.contains("contact shows"),
        "{text}"
    );
    assert!(
        text.contains("detected and identified")
            || text.contains("identified the alien fleet")
            || text.contains("located and ident")
    );
    assert!(text.contains("From your 1st Fleet, located in"));
    assert!(text.contains("the ") && text.contains(" Fleet of "));
    assert!(text.contains("Fleet of \""));
    assert!(text.contains("(Empire #"));
    assert!(
        text.contains("seeking safety")
            || text.contains("seek safety")
            || text.contains("holding position")
            || text.contains("ALERT: Fleet contact lost!")
    );
    assert!(text.contains("planet \"") || text.contains("System("));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_destroyed_fleet_report_separates_last_contact_from_force_rows() {
    let target = unique_temp_dir("nc-cli-maint-rust-destroyed-fleet-layout");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);
    write_mutual_enemy_diplomacy(&target, 1, 2);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let fleet = &mut game_data.fleets.records[0];
    let coords = fleet.current_location_coords_raw();
    fleet.set_standing_order_kind(Order::HoldPosition);
    fleet.set_standing_order_target_coords_raw(coords);
    fleet.set_current_speed(0);
    fleet.set_destroyer_count(1);
    fleet.set_cruiser_count(0);
    fleet.set_battleship_count(0);
    fleet.set_troop_transport_count(0);
    fleet.set_army_count(0);
    fleet.set_scout_count(0);
    fleet.set_etac_count(0);

    let enemy = &mut game_data.fleets.records[4];
    enemy.set_battleship_count(9);
    enemy.set_cruiser_count(1);
    enemy.set_destroyer_count(0);
    enemy.set_troop_transport_count(13);
    enemy.set_army_count(12);
    enemy.set_scout_count(0);
    enemy.set_etac_count(0);

    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let records = results_records(&results);
    let (_, report_lines) = logical_result_reports(&records)
        .into_iter()
        .find(|(_, lines)| {
            lines
                .iter()
                .any(|line| line == "ALERT: Fleet contact lost!")
        })
        .expect("expected fleet contact lost report");
    let report = report_lines.join("\n");

    assert!(
        report_lines
            .iter()
            .any(|line| line.starts_with("Last contact:"))
    );
    assert!(
        report.contains("destroyed by") || report.contains("destroyed while intercepting"),
        "{report}"
    );
    let our_forces_idx = report_lines
        .iter()
        .position(|line| line.starts_with("Our forces:"))
        .expect("expected Our forces section");
    assert_eq!(report_lines[our_forces_idx - 1], "", "{report}");

    cleanup_dir(&target);
}

#[test]
fn maint_rust_rendezvous_arrival_generates_waiting_report() {
    let target = unique_temp_dir("nc-cli-maint-rust-rendezvous-wait");
    copy_fixture_dir("fixtures/ecmaint-fleet-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let fleet = &mut game_data.fleets.records[0];
    fleet.set_standing_order_kind(Order::RendezvousSector);
    fleet.set_standing_order_target_coords_raw([15, 13]);
    // ECGAME writes the 0x22 world-target flag as 0 for sector orders.
    fleet.set_mission_aux_bytes([0, 0]);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Rendezvous mission report"));
    assert!(text.contains("waiting for more fleets"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_join_merge_generates_join_report() {
    let target = unique_temp_dir("nc-cli-maint-rust-join-merge");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    game_data.player.records[0].raw[0x00] = 0xff;
    let coords = game_data.fleets.records[0].current_location_coords_raw();
    game_data.fleets.records[1].set_current_location_coords_raw(coords);
    game_data.fleets.records[1].set_standing_order_kind(Order::JoinAnotherFleet);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(text.contains("From your Fleet Command Center:"));
    assert!(text.contains("Join mission summary"));
    assert!(text.contains("Completed joins:"));
    assert!(text.contains("merged into Fleet"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_rendezvous_merge_generates_absorbing_report() {
    let target = unique_temp_dir("nc-cli-maint-rust-rendezvous-absorb");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    game_data.player.records[0].raw[0x00] = 0xff;
    let coords = game_data.fleets.records[0].current_location_coords_raw();
    game_data.fleets.records[0].set_standing_order_kind(Order::RendezvousSector);
    game_data.fleets.records[1].set_current_location_coords_raw(coords);
    game_data.fleets.records[1].set_standing_order_kind(Order::RendezvousSector);
    game_data.fleets.records[2].set_current_location_coords_raw(coords);
    game_data.fleets.records[2].set_standing_order_kind(Order::RendezvousSector);
    game_data.fleets.records[3].set_current_location_coords_raw(coords);
    game_data.fleets.records[3].set_standing_order_kind(Order::RendezvousSector);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(text.contains("Rendezvous mission report"));
    assert!(text.contains("absorbing fleets 2, 3, and 4."), "{text}");
    assert!(!text.contains("the 2nd Fleet"), "{text}");

    cleanup_dir(&target);
}

#[test]
fn maint_rust_join_contact_uses_join_report_label() {
    let target = unique_temp_dir("nc-cli-maint-rust-join-contact");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    game_data.fleets.records[0].set_standing_order_kind(Order::JoinAnotherFleet);
    let host_id = game_data.fleets.records[1].fleet_id_word_raw();
    game_data.fleets.records[0].set_join_host_fleet_id_raw(host_id);
    // Stalled in place (no speed to restore), so the contact happens here.
    game_data.fleets.records[0].set_max_speed(0);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Join mission report"));
    assert!(text.contains("Sensor contact") || text.contains("contact shows"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_join_host_destroyed_report_never_shows_zero_fleet_number() {
    let target = unique_temp_dir("nc-cli-maint-rust-join-host-destroyed");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    game_data.player.records[0].raw[0x00] = 0xff;
    let host_id = game_data.fleets.records[0].fleet_id_word_raw();
    game_data.fleets.records[0].set_destroyer_count(0);
    game_data.fleets.records[0].set_cruiser_count(0);
    game_data.fleets.records[0].set_battleship_count(0);
    game_data.fleets.records[0].set_scout_count(0);
    game_data.fleets.records[0].set_troop_transport_count(0);
    game_data.fleets.records[0].set_etac_count(0);
    let joiner = &mut game_data.fleets.records[1];
    joiner.set_current_location_coords_raw([7, 9]);
    joiner.set_standing_order_kind(Order::JoinAnotherFleet);
    joiner.set_join_host_fleet_id_raw(host_id);
    joiner.set_standing_order_target_coords_raw([10, 10]);
    joiner.set_current_speed(3);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(text.contains("From your Fleet Command Center:"));
    assert!(text.contains("Join mission summary"));
    assert!(text.contains("Lost hosts: Fleet 2 lost host Fleet 1 and is holding position."));
    assert!(!text.contains("(0th Fleet)"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_join_retarget_report_uses_fleet_command_summary_without_sector_noise() {
    let target = unique_temp_dir("nc-cli-maint-rust-join-retarget");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    game_data.player.records[0].raw[0x00] = 0xff;
    let host_id = game_data.fleets.records[0].fleet_id_word_raw();
    let joiner_fleet_number = game_data.fleets.records[1].local_slot_word_raw() as usize;
    let joiner = &mut game_data.fleets.records[1];
    joiner.set_current_location_coords_raw([3, 9]);
    joiner.set_standing_order_kind(Order::JoinAnotherFleet);
    joiner.set_join_host_fleet_id_raw(host_id);
    joiner.set_standing_order_target_coords_raw([1, 1]);
    joiner.set_current_speed(3);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(text.contains("From your Fleet Command Center:"));
    assert!(text.contains("Join mission summary"));
    assert!(text.contains(&format!(
        "Retargeted to follow host: Fleet {}.",
        joiner_fleet_number
    )));
    assert!(!text.contains("From your fleet, located in Sector(3,9):"));
    assert!(!text.contains("Sector(3,9)"));
    assert!(!text.contains("Sector(1,1)"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_join_summary_uses_compact_oxford_fleet_lists() {
    let target = unique_temp_dir("nc-cli-maint-rust-join-compact-lists");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    game_data.player.records[0].raw[0x00] = 0xff;
    let host_id = game_data.fleets.records[0].fleet_id_word_raw();

    for fleet_idx in [1usize, 2, 3] {
        game_data.fleets.records[fleet_idx].set_current_location_coords_raw([3, 9]);
        game_data.fleets.records[fleet_idx].set_standing_order_kind(Order::JoinAnotherFleet);
        game_data.fleets.records[fleet_idx].set_join_host_fleet_id_raw(host_id);
        game_data.fleets.records[fleet_idx].set_standing_order_target_coords_raw([1, 1]);
        game_data.fleets.records[fleet_idx].set_current_speed(3);
    }

    let lost_host = game_data.fleets.records[4].fleet_id_word_raw();
    game_data.fleets.records[4].set_current_location_coords_raw([7, 9]);
    game_data.fleets.records[4].set_standing_order_kind(Order::HoldPosition);
    game_data.fleets.records[4].set_standing_order_target_coords_raw([7, 9]);
    game_data.fleets.records[4].set_current_speed(0);
    game_data.fleets.records[4].set_battleship_count(0);
    game_data.fleets.records[4].set_cruiser_count(0);
    game_data.fleets.records[4].set_destroyer_count(0);
    game_data.fleets.records[4].set_scout_count(0);
    game_data.fleets.records[4].set_troop_transport_count(0);
    game_data.fleets.records[4].set_army_count(0);
    game_data.fleets.records[4].set_etac_count(0);

    for fleet_idx in [5usize, 6, 7] {
        game_data.fleets.records[fleet_idx].set_current_location_coords_raw([7, 9]);
        game_data.fleets.records[fleet_idx].set_standing_order_kind(Order::JoinAnotherFleet);
        game_data.fleets.records[fleet_idx].set_join_host_fleet_id_raw(lost_host);
        game_data.fleets.records[fleet_idx].set_standing_order_target_coords_raw([10, 10]);
        game_data.fleets.records[fleet_idx].set_current_speed(3);
    }

    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(text.contains("From your Fleet Command Center:"));
    assert!(text.contains("Join mission summary"));
    assert!(
        text.contains("Completed joins: Fleets 3 and 4 merged into Fleet 2."),
        "{text}"
    );
    assert!(
        text.contains("Lost hosts: Fleets 2, 3, and 4 lost host Fleet 1"),
        "{text}"
    );
    assert!(!text.contains("Lost hosts: Fleets 2, 3 and"), "{text}");

    cleanup_dir(&target);
}

#[test]
fn maint_rust_join_host_absorbed_into_other_fleet_retargets_joiners() {
    let target = unique_temp_dir("nc-cli-maint-rust-join-host-absorbed-retarget");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    game_data.player.records[0].raw[0x00] = 0x00;

    let survivor_coords = [10, 10];
    let survivor_id = game_data.fleets.records[0].fleet_id_word_raw();
    let absorbed_host_id = game_data.fleets.records[1].fleet_id_word_raw();

    game_data.fleets.records[0].set_current_location_coords_raw(survivor_coords);
    game_data.fleets.records[0].set_standing_order_kind(Order::HoldPosition);
    game_data.fleets.records[0].set_standing_order_target_coords_raw(survivor_coords);
    game_data.fleets.records[0].set_current_speed(0);

    game_data.fleets.records[1].set_current_location_coords_raw(survivor_coords);
    game_data.fleets.records[1].set_standing_order_kind(Order::JoinAnotherFleet);
    game_data.fleets.records[1].set_join_host_fleet_id_raw(survivor_id);
    game_data.fleets.records[1].set_standing_order_target_coords_raw(survivor_coords);
    game_data.fleets.records[1].set_current_speed(0);

    game_data.fleets.records[2].set_current_location_coords_raw([5, 10]);
    game_data.fleets.records[2].set_standing_order_kind(Order::JoinAnotherFleet);
    game_data.fleets.records[2].set_join_host_fleet_id_raw(absorbed_host_id);
    game_data.fleets.records[2].set_standing_order_target_coords_raw(survivor_coords);
    game_data.fleets.records[2].set_current_speed(3);

    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(text.contains("From your Fleet Command Center:"), "{text}");
    assert!(text.contains("Join mission summary"), "{text}");
    assert!(
        text.contains("Completed joins: Fleet 2 merged into Fleet 1."),
        "{text}"
    );
    assert!(
        text.contains("Retargeted to follow host: Fleet 3."),
        "{text}"
    );
    assert!(!text.contains("target fleet no longer exists"), "{text}");
    assert!(
        !text.contains("holding position and awaiting orders"),
        "{text}"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_join_summary_emits_single_footer_line() {
    let target = unique_temp_dir("nc-cli-maint-rust-join-single-footer");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    game_data.player.records[0].raw[0x00] = 0xff;
    let host_id = game_data.fleets.records[0].fleet_id_word_raw();

    game_data.fleets.records[1].set_current_location_coords_raw([3, 9]);
    game_data.fleets.records[1].set_standing_order_kind(Order::JoinAnotherFleet);
    game_data.fleets.records[1].set_join_host_fleet_id_raw(host_id);
    game_data.fleets.records[1].set_standing_order_target_coords_raw([1, 1]);
    game_data.fleets.records[1].set_current_speed(3);

    game_data.fleets.records[2].set_current_location_coords_raw([3, 9]);
    game_data.fleets.records[2].set_standing_order_kind(Order::JoinAnotherFleet);
    game_data.fleets.records[2].set_join_host_fleet_id_raw(host_id);
    game_data.fleets.records[2].set_standing_order_target_coords_raw([1, 1]);
    game_data.fleets.records[2].set_current_speed(3);

    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let records = results_records(&results);
    let (_, report_lines) = logical_result_reports(&records)
        .into_iter()
        .find(|(_, lines)| lines.iter().any(|line| line == "Join mission summary"))
        .expect("expected join mission summary report");

    assert_eq!(
        report_lines
            .iter()
            .filter(|line| line.as_str() == "<end of transmission>")
            .count(),
        1,
        "{report_lines:?}"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_rendezvous_contact_uses_rendezvous_report_label() {
    let target = unique_temp_dir("nc-cli-maint-rust-rendezvous-contact");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let coords = game_data.fleets.records[0].current_location_coords_raw();
    game_data.fleets.records[0].set_standing_order_kind(Order::RendezvousSector);
    game_data.fleets.records[0].set_standing_order_target_coords_raw(coords);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Rendezvous mission report"));
    assert!(text.contains("Sensor contact") || text.contains("contact shows"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_guard_contact_uses_guard_report_label() {
    let target = unique_temp_dir("nc-cli-maint-rust-guard-contact");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    game_data.fleets.records[0].set_standing_order_kind(Order::GuardBlockadeWorld);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Guard/Blockade World mission report"));
    assert!(text.contains("Sensor contact") || text.contains("contact shows"));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_reports_empire_defeat_without_recovery_path() {
    let target = unique_temp_dir("nc-cli-maint-rust-civil-disorder");
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    for planet in &mut game_data.planets.records {
        if planet.owner_empire_slot_raw() == 1 {
            planet.set_owner_empire_slot_raw(0);
            planet.set_ownership_status_raw(0);
        }
    }
    for fleet in &mut game_data.fleets.records {
        if fleet.owner_empire_raw() == 1 {
            fleet.set_etac_count(0);
            fleet.set_troop_transport_count(0);
            fleet.set_army_count(0);
        }
    }
    game_data.save(&target).expect("mutated game should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let post = CoreGameData::load(&target).expect("maint-rust output should load");
    assert_eq!(post.player.records[0].owner_mode_raw(), 0x00);
    assert_eq!(post.player.records[0].legacy_status_name_summary(), "");

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let normalized = decode_chunked_report(&results);
    assert!(
        normalized.to_ascii_lowercase().contains("empire defeated"),
        "RESULTS.DAT decoded text was: {:?}",
        normalized
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_reports_when_one_serious_contender_remains() {
    let target = unique_temp_dir("nc-cli-maint-rust-campaign-outlook");
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    for empire_raw in 2..=4u8 {
        for planet in &mut game_data.planets.records {
            if planet.owner_empire_slot_raw() == empire_raw {
                planet.set_owner_empire_slot_raw(0);
                planet.set_ownership_status_raw(0);
            }
        }
        for fleet in &mut game_data.fleets.records {
            if fleet.owner_empire_raw() == empire_raw {
                fleet.set_etac_count(0);
                fleet.set_troop_transport_count(0);
                fleet.set_army_count(0);
                fleet.set_destroyer_count(0);
                fleet.set_cruiser_count(0);
                fleet.set_battleship_count(0);
                fleet.set_scout_count(0);
            }
        }
    }
    game_data.save(&target).expect("mutated game should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let post = CoreGameData::load(&target).expect("maint-rust output should load");
    assert_eq!(
        post.campaign_outlook(),
        nc_data::CampaignOutlook::SoleContender(1)
    );

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let normalized = decode_chunked_report(&results);
    assert!(
        normalized.to_ascii_lowercase().contains("empire defeated"),
        "RESULTS.DAT decoded text was: {:?}",
        normalized
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_reports_emperor_recognition_when_only_stable_empire_remains() {
    let target = unique_temp_dir("nc-cli-maint-rust-emperor-recognition");
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    for empire_raw in 2..=4u8 {
        for planet in &mut game_data.planets.records {
            if planet.owner_empire_slot_raw() == empire_raw {
                planet.set_owner_empire_slot_raw(0);
                planet.set_ownership_status_raw(0);
            }
        }
        for fleet in &mut game_data.fleets.records {
            if fleet.owner_empire_raw() == empire_raw {
                fleet.set_etac_count(0);
                fleet.set_troop_transport_count(0);
                fleet.set_army_count(0);
                fleet.set_destroyer_count(0);
                fleet.set_cruiser_count(0);
                fleet.set_battleship_count(0);
                fleet.set_scout_count(0);
            }
        }
    }
    game_data.save(&target).expect("mutated game should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let post = CoreGameData::load(&target).expect("maint-rust output should load");
    assert_eq!(
        post.campaign_outcome(),
        nc_data::CampaignOutcome::RecognizedEmperor(1)
    );

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let normalized = decode_chunked_report(&results);
    assert!(
        normalized.to_ascii_lowercase().contains("empire defeated"),
        "RESULTS.DAT decoded text was: {:?}",
        normalized
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_reports_fleet_defection_after_civil_disorder() {
    let target = unique_temp_dir("nc-cli-maint-rust-fleet-defection");
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    for planet in &mut game_data.planets.records {
        if planet.owner_empire_slot_raw() == 1 {
            planet.set_owner_empire_slot_raw(0);
            planet.set_ownership_status_raw(0);
        }
    }
    for fleet in &mut game_data.fleets.records {
        if fleet.owner_empire_raw() == 1 {
            fleet.set_etac_count(0);
            fleet.set_troop_transport_count(0);
            fleet.set_army_count(0);
        }
    }
    game_data.save(&target).expect("mutated game should save");

    let stdout = run_maint_rust_with_export(&target, 2);
    assert!(stdout.contains("Rust maintenance complete."));

    let post = CoreGameData::load(&target).expect("maint-rust output should load");
    let remaining_player_fleets = post
        .fleets
        .records
        .iter()
        .filter(|fleet| fleet.owner_empire_raw() == 1)
        .count();
    assert_eq!(remaining_player_fleets, 3);

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let normalized = decode_chunked_report(&results);
    assert!(
        normalized
            .to_ascii_lowercase()
            .contains("crews have defected"),
        "RESULTS.DAT decoded text was: {:?}",
        normalized
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_seeded_games_survive_five_turns_across_manual_player_tiers() {
    for (player_count, seed) in [(4u8, 1515u64), (9, 2025), (16, 4242), (25, 5151)] {
        let target = unique_temp_dir(&format!("nc-cli-maint-rust-multiturn-{player_count}p"));

        let stdout = run_nc_cli_in_dir(
            &[
                "sysop",
                "new-game",
                target.to_str().unwrap(),
                "--config",
                "nc-cli/config/setup.example.kdl",
                "--players",
                &player_count.to_string(),
                "--seed",
                &seed.to_string(),
            ],
            common::rust_workspace(),
        );
        assert!(
            stdout.contains("Initialized new game at:"),
            "sysop new-game stdout was: {stdout:?}"
        );

        let stdout = run_maint_rust_with_export(&target, 5);
        assert!(stdout.contains("Rust maintenance complete."));

        let post = CoreGameData::load(&target).expect("multi-turn maint output should load");
        assert_eq!(post.conquest.game_year(), 3005);
        assert!(
            post.ecmaint_preflight_errors().is_empty(),
            "preflight errors after five turns for {player_count} players: {:?}",
            post.ecmaint_preflight_errors()
        );

        let database_bytes =
            fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
        let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
        assert_eq!(
            database.records.len(),
            post.player.records.len() * post.planets.records.len()
        );

        cleanup_dir(&target);
    }
}

// ---------------------------------------------------------------------------
// Batched abort report tests
// ---------------------------------------------------------------------------

/// Two empire-1 fleets at the same coords with different missions both lose
/// the battle; the result should be a single batched abort report listing
/// both fleet numbers and their short mission labels instead of two separate
/// per-fleet per-mission reports.
#[test]
fn maint_rust_destroyed_fleet_abort_is_suppressed() {
    let target = unique_temp_dir("nc-cli-maint-rust-destroyed-abort-suppress");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);
    write_mutual_enemy_diplomacy(&target, 1, 2);

    let battle_coords = [10u8, 10u8];
    let mut game_data = CoreGameData::load(&target).expect("fixture should load");

    // Fleet 0 (empire 1): minimal force with scout mission — will be destroyed.
    {
        let f = &mut game_data.fleets.records[0];
        f.set_current_location_coords_raw(battle_coords);
        f.set_standing_order_kind(Order::ScoutSector);
        f.set_standing_order_target_coords_raw(battle_coords);
        f.set_current_speed(3);
        f.set_battleship_count(0);
        f.set_cruiser_count(0);
        f.set_destroyer_count(1);
        f.set_scout_count(1);
        f.set_troop_transport_count(0);
        f.set_army_count(0);
        f.set_etac_count(0);
        f.set_rules_of_engagement(10);
    }
    // Fleet 4 (empire 2): overwhelming force.
    {
        let f = &mut game_data.fleets.records[4];
        f.set_current_location_coords_raw(battle_coords);
        f.set_standing_order_kind(Order::MoveOnly);
        f.set_standing_order_target_coords_raw(battle_coords);
        f.set_current_speed(3);
        f.set_battleship_count(50);
        f.set_cruiser_count(50);
        f.set_destroyer_count(50);
        f.set_scout_count(0);
        f.set_troop_transport_count(0);
        f.set_army_count(0);
        f.set_etac_count(0);
        f.set_rules_of_engagement(10);
    }
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);

    // Fleet destruction report must appear.
    assert!(
        text.contains("ALERT: Fleet contact lost!"),
        "expected fleet destroyed report, got: {text}"
    );
    // No separate abort report for the destroyed fleet.
    assert!(
        !text.contains("Scouting mission report: Hostile action"),
        "should not produce scout abort report for destroyed fleet, got: {text}"
    );
    assert!(
        !text.contains("abort their missions"),
        "should not produce batched abort report for destroyed fleet, got: {text}"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_bombard_destruction_emits_lost_contact_telemetry() {
    let target = unique_temp_dir("nc-cli-maint-rust-bombard-destroyed-telemetry");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let target_world = &mut game_data.planets.records[13];
    target_world.set_as_owned_target_world(
        [15, 13],
        [0x64, 0x87],
        [0x00, 0x00, 0x00, 0x00, 0x48, 0x87],
        0x04,
        0x0b,
        *b"TargetPrimeet",
        [0x05, 0x1d, 0x0b, 0x11, 0x25, 0x1c, 0x05],
        25,
        15,
        0,
        2,
    );
    let attacker = &mut game_data.fleets.records[0];
    attacker.set_current_location_coords_raw([15, 13]);
    attacker.set_standing_order_kind(Order::BombardWorld);
    attacker.set_standing_order_target_coords_raw([15, 13]);
    attacker.set_current_speed(0);
    attacker.raw[0x19] = 0x80;
    attacker.set_rules_of_engagement(10);
    attacker.set_scout_count(0);
    attacker.set_battleship_count(0);
    attacker.set_cruiser_count(0);
    attacker.set_destroyer_count(1);
    attacker.set_troop_transport_count(0);
    attacker.set_army_count(0);
    attacker.set_etac_count(0);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(text.contains("ALERT: Fleet contact lost!"), "{text}");
    assert!(
        text.contains("Alien forces:     15 ground batteries and 25 armies"),
        "{text}"
    );
    assert!(
        text.contains("ALERT: Orbital bombardment underway!"),
        "{text}"
    );
    assert!(
        text.contains("Hostile return fire destroyed the bombardment force."),
        "{text}"
    );
    assert!(
        !text.contains("We are maintaining bombardment position and will continue next turn."),
        "{text}"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_invasion_destruction_emits_lost_contact_telemetry() {
    let target = unique_temp_dir("nc-cli-maint-rust-invade-destroyed-telemetry");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let target_world = &mut game_data.planets.records[13];
    target_world.set_as_owned_target_world(
        [15, 13],
        [0x64, 0x87],
        [0x00, 0x00, 0x00, 0x00, 0x48, 0x87],
        0x04,
        0x0b,
        *b"TargetPrimeet",
        [0x05, 0x1d, 0x0b, 0x11, 0x25, 0x1c, 0x05],
        25,
        15,
        0,
        2,
    );
    let attacker = &mut game_data.fleets.records[0];
    attacker.set_current_location_coords_raw([15, 13]);
    attacker.set_standing_order_kind(Order::InvadeWorld);
    attacker.set_standing_order_target_coords_raw([15, 13]);
    attacker.set_current_speed(0);
    attacker.raw[0x19] = 0x80;
    attacker.set_rules_of_engagement(10);
    attacker.set_scout_count(0);
    attacker.set_battleship_count(0);
    attacker.set_cruiser_count(0);
    attacker.set_destroyer_count(1);
    attacker.set_troop_transport_count(5);
    attacker.set_army_count(5);
    attacker.set_etac_count(0);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(text.contains("ALERT: Fleet contact lost!"), "{text}");
    assert!(
        text.contains("Alien forces:     15 ground batteries and 25 armies"),
        "{text}"
    );
    assert!(
        text.contains("ALERT: Planetary invasion blocked!")
            || text.contains("ALERT: Planetary invasion repulsed!"),
        "{text}"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_invasion_failure_generates_defender_side_report() {
    let target = unique_temp_dir("nc-cli-maint-rust-invade-failure-defender-report");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let target_world = &mut game_data.planets.records[13];
    target_world.set_as_owned_target_world(
        [15, 13],
        [0x64, 0x87],
        [0x00, 0x00, 0x00, 0x00, 0x48, 0x87],
        0x04,
        0x0b,
        *b"TargetPrimeet",
        [0x05, 0x1d, 0x0b, 0x11, 0x25, 0x1c, 0x05],
        25,
        15,
        0,
        2,
    );
    let attacker = &mut game_data.fleets.records[0];
    attacker.set_current_location_coords_raw([15, 13]);
    attacker.set_standing_order_kind(Order::InvadeWorld);
    attacker.set_standing_order_target_coords_raw([15, 13]);
    attacker.set_current_speed(0);
    attacker.raw[0x19] = 0x80;
    attacker.set_rules_of_engagement(10);
    attacker.set_scout_count(0);
    attacker.set_battleship_count(0);
    attacker.set_cruiser_count(0);
    attacker.set_destroyer_count(1);
    attacker.set_troop_transport_count(5);
    attacker.set_army_count(5);
    attacker.set_etac_count(0);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 2);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(
        text.contains("ALERT: Enemy invasion blocked!")
            || text.contains("ALERT: Enemy invasion repulsed!"),
        "{text}"
    );
    assert!(text.contains("Attacking force:"), "{text}");
    assert!(text.contains("Our defenses:"), "{text}");
    assert!(text.contains("Enemy losses:"), "{text}");

    cleanup_dir(&target);
}

#[test]
fn maint_rust_blitz_destruction_emits_lost_contact_telemetry() {
    let target = unique_temp_dir("nc-cli-maint-rust-blitz-destroyed-telemetry");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let target_world = &mut game_data.planets.records[13];
    target_world.set_as_owned_target_world(
        [15, 13],
        [0x64, 0x87],
        [0x00, 0x00, 0x00, 0x00, 0x48, 0x87],
        0x04,
        0x0b,
        *b"TargetPrimeet",
        [0x05, 0x1d, 0x0b, 0x11, 0x25, 0x1c, 0x05],
        25,
        15,
        0,
        2,
    );
    let attacker = &mut game_data.fleets.records[0];
    attacker.set_current_location_coords_raw([15, 13]);
    attacker.set_standing_order_kind(Order::BlitzWorld);
    attacker.set_standing_order_target_coords_raw([15, 13]);
    attacker.set_current_speed(0);
    attacker.raw[0x19] = 0x80;
    attacker.set_rules_of_engagement(10);
    attacker.set_scout_count(0);
    attacker.set_battleship_count(0);
    attacker.set_cruiser_count(0);
    attacker.set_destroyer_count(1);
    attacker.set_troop_transport_count(5);
    attacker.set_army_count(5);
    attacker.set_etac_count(0);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(text.contains("ALERT: Fleet contact lost!"), "{text}");
    assert!(
        text.contains("Alien forces:     15 ground batteries and 25 armies"),
        "{text}"
    );
    assert!(
        text.contains("ALERT: Blitz assault blocked!")
            || text.contains("ALERT: Blitz assault failed!"),
        "{text}"
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_blitz_failure_generates_defender_side_report() {
    let target = unique_temp_dir("nc-cli-maint-rust-blitz-failure-defender-report");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let target_world = &mut game_data.planets.records[13];
    target_world.set_as_owned_target_world(
        [15, 13],
        [0x64, 0x87],
        [0x00, 0x00, 0x00, 0x00, 0x48, 0x87],
        0x04,
        0x0b,
        *b"TargetPrimeet",
        [0x05, 0x1d, 0x0b, 0x11, 0x25, 0x1c, 0x05],
        25,
        15,
        0,
        2,
    );
    let attacker = &mut game_data.fleets.records[0];
    attacker.set_current_location_coords_raw([15, 13]);
    attacker.set_standing_order_kind(Order::BlitzWorld);
    attacker.set_standing_order_target_coords_raw([15, 13]);
    attacker.set_current_speed(0);
    attacker.raw[0x19] = 0x80;
    attacker.set_rules_of_engagement(10);
    attacker.set_scout_count(0);
    attacker.set_battleship_count(0);
    attacker.set_cruiser_count(0);
    attacker.set_destroyer_count(1);
    attacker.set_troop_transport_count(5);
    attacker.set_army_count(5);
    attacker.set_etac_count(0);
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 2);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(
        text.contains("ALERT: Enemy blitz blocked!")
            || text.contains("ALERT: Enemy blitz repulsed!"),
        "{text}"
    );
    assert!(text.contains("Attacking force:"), "{text}");
    assert!(text.contains("Our defenses:"), "{text}");
    assert!(text.contains("Enemy losses:"), "{text}");

    cleanup_dir(&target);
}
