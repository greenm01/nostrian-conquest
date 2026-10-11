mod common;

use common::{
    cleanup_dir, copy_fixture_dir, export_campaign_db, import_campaign_db, repo_root,
    run_classic_ecgame_smoke, run_classic_ecgame_smoke_with_alias,
    run_maint_rust_failure_after_import, run_maint_rust_with_export,
    run_maint_rust_with_export_and_seed, run_nc_cli_in_dir, rust_workspace,
    set_mutual_enemy_in_player_dat, unique_temp_dir, write_mutual_enemy_diplomacy,
};
use nc_compat::{DatabaseDat, load_classic_core_files, save_classic_core_files};
use nc_data::{
    BuildSlot, CampaignStore, Coords, CoreGameData, DiplomaticRelation, EmpireId, Order, Planet,
    PlayerMode, ProductionItemKind, Real48, diplomatic_relation_toward, recompute_max_speed,
    set_armies_aboard, set_diplomatic_relation_toward, set_empire_name, set_fleet_at_rest,
    set_mission_target, set_planet_name, set_player_handle, set_troop_transports,
    stored_production_points, troop_transports,
};
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

/// `run_maint_rust_with_export`, with a build slot only Rust can hold planted
/// in the imported store: classic's counts by kind cannot express a slot of
/// points under an unknown kind.
fn run_maint_rust_with_planted_build_slot(
    target: &std::path::Path,
    planet_idx: usize,
    slot: BuildSlot,
) -> String {
    import_campaign_db(target);
    let store = CampaignStore::open_default_in_dir(target).expect("open campaign store");
    let state = store
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("imported runtime state");
    let mut game_data = state.game_data;
    game_data.planets[planet_idx].build_queue[0] = slot;
    store
        .save_runtime_state_structured(
            game_data.conquest.game_year,
            &game_data,
            &state.report_block_rows,
            &state.queued_mail,
        )
        .expect("save planted state");
    drop(store);
    let stdout = run_nc_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        rust_workspace(),
    );
    export_campaign_db(target, target);
    stdout
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
    for fleet in &mut game_data.fleets {
        let current = fleet.sector.to_array();
        fleet.order = Order::HoldPosition;
        fleet.target = Coords::from_array(current);
        fleet.speed = 0;
        fleet.rules_of_engagement = 10;
        fleet.ships.destroyers = 0;
        fleet.ships.cruisers = 0;
        fleet.ships.battleships = 0;
        fleet.ships.scouts = 0;
        set_troop_transports(fleet, 0);
        set_armies_aboard(fleet, 0);
        fleet.ships.etacs = 0;
    }
}

fn configure_delayed_invade_arrival_directory(target: &std::path::Path) {
    let mut game_data = load_classic_core_files(target).expect("fixture should load");
    let coords = [15, 13];

    zero_all_fleets(&mut game_data);

    let target_world = &mut game_data.planets[13];
    *target_world = Planet {
        sector: Coords::from_array(coords),
        potential_production: 100,
        production: Real48::from_int(100),
        stored_goods: Real48::from_int(66),
        name: "TargetPrime".to_string(),
        armies: 1,
        ground_batteries: 1,
        years_under_owner: 0,
        owner: EmpireId(2),
        ..Planet::default()
    };

    let attacker = &mut game_data.fleets[0];
    attacker.sector = Coords::new(14, 13);
    attacker.order = Order::InvadeWorld;
    attacker.target = Coords::from_array(coords);
    attacker.speed = 3;
    attacker.rules_of_engagement = 10;
    attacker.ships.scouts = 0;
    attacker.ships.battleships = 20;
    attacker.ships.cruisers = 20;
    attacker.ships.destroyers = 20;
    set_troop_transports(attacker, 5);
    set_armies_aboard(attacker, 5);
    attacker.ships.etacs = 0;
    set_fleet_at_rest(attacker);

    save_classic_core_files(target, &game_data).expect("mutated fixture should save");
}

fn configure_dominant_invalidated_invade_directory(target: &std::path::Path) {
    let mut game_data = load_classic_core_files(target).expect("fixture should load");
    let coords = [15, 13];

    game_data.bases.clear();
    for fleet in &mut game_data.fleets {
        let current = fleet.sector.to_array();
        fleet.speed = 0;
        fleet.order = Order::HoldPosition;
        fleet.target = Coords::from_array(current);
        fleet.rules_of_engagement = 10;
        fleet.ships.destroyers = 0;
        fleet.ships.cruisers = 0;
        fleet.ships.battleships = 0;
        fleet.ships.scouts = 0;
        set_troop_transports(fleet, 0);
        set_armies_aboard(fleet, 0);
        fleet.ships.etacs = 0;
    }

    let target_world = &mut game_data.planets[13];
    *target_world = Planet {
        sector: Coords::from_array(coords),
        potential_production: 100,
        production: Real48::from_int(100),
        stored_goods: Real48::from_int(66),
        name: "TargetPrime".to_string(),
        armies: 10,
        ground_batteries: 4,
        years_under_owner: 2,
        owner: EmpireId(2),
        ..Planet::default()
    };

    let attacker = &mut game_data.fleets[0];
    attacker.sector = Coords::from_array(coords);
    attacker.order = Order::InvadeWorld;
    attacker.target = Coords::from_array(coords);
    attacker.speed = 3;
    attacker.depth = Real48::HALF;
    attacker.ships.destroyers = 1;
    set_troop_transports(attacker, 1);
    set_armies_aboard(attacker, 1);

    let defender = &mut game_data.fleets[4];
    defender.sector = Coords::from_array(coords);
    defender.order = Order::HoldPosition;
    defender.target = Coords::from_array(coords);
    defender.speed = 0;
    defender.ships.destroyers = 1;

    save_classic_core_files(target, &game_data).expect("mutated fixture should save");
}

#[test]
fn maint_rust_econ_updates_database_owner_intel_from_post_combat_planet_state() {
    let target = unique_temp_dir("nc-cli-maint-rust-econ");
    copy_fixture_dir("fixtures/ecmaint-econ-pre/v1.5", &target);

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Running Rust maintenance on:"));
    assert!(stdout.contains("Rust maintenance complete."));

    let game_data = load_classic_core_files(&target).expect("maint-rust output should load");
    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");

    let (planet_idx, planet) = game_data
        .planets
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.sector.to_array() == [15, 13])
        .expect("econ combat target should exist");
    let year_bytes = (game_data.conquest.game_year - 1).to_le_bytes();
    let owner_player = planet.owner.0.saturating_sub(1) as usize;
    let planet_name = planet.name.clone();

    let owner_record = database.record(planet_idx, owner_player, game_data.planets.len());
    assert_eq!(owner_record.planet_name_bytes(), planet_name.as_bytes());
    assert_eq!(owner_record.raw[0x15], planet.owner.0);
    assert_eq!(owner_record.raw[0x16], year_bytes[0]);
    assert_eq!(owner_record.raw[0x17], year_bytes[1]);
    assert_eq!(owner_record.raw[0x1e], 0x40 + planet.owner.0);
    assert_eq!(
        owner_record.raw[0x23],
        u8::try_from(planet.armies).unwrap_or(u8::MAX)
    );
    assert_eq!(
        owner_record.raw[0x25],
        u8::try_from(planet.ground_batteries).unwrap_or(u8::MAX)
    );

    let unrelated_player = (owner_player + 1) % 4;
    let unrelated_record = database.record(planet_idx, unrelated_player, game_data.planets.len());
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

    let pre = load_classic_core_files(&target).expect("fixture should load");
    let stdout = run_nc_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Rust maintenance complete."));

    let post =
        load_classic_core_files(&target).expect("maint-rust should leave directory untouched");
    assert_eq!(post.conquest.game_year, pre.conquest.game_year);
    assert!(!target.join("ECGAME.EXE").exists());
    assert!(!target.join("ECMAINT.EXE").exists());

    common::export_campaign_db(&target, &target);

    let post =
        load_classic_core_files(&target).expect("explicit export should project latest snapshot");
    assert_eq!(post.conquest.game_year, pre.conquest.game_year + 1);
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

    let mut classic_side = load_classic_core_files(&target).expect("fixture should load");
    set_player_handle(&mut classic_side.players[1], "SYSOP");
    set_empire_name(&mut classic_side.players[1], "foo");
    save_classic_core_files(&target, &classic_side).expect("classic-side edit should save");

    let stdout = run_nc_cli_in_dir(
        &["maint-rust", target.to_str().unwrap(), "1"],
        common::rust_workspace(),
    );
    assert!(stdout.contains("Rust maintenance complete."));

    common::export_campaign_db(&target, &target);

    let post = load_classic_core_files(&target).expect("exported runtime should load");
    assert_ne!(post.players[1].handle.trim().to_string(), "SYSOP");
    assert_ne!(post.players[1].empire_name.trim().to_string(), "foo");

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
    let post = load_classic_core_files(&target).expect("maint-rust output should load");
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
        &(post.conquest.game_year - 1).to_le_bytes(),
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
    assert_eq!(post.players[0].results_first, 1);
    // A viewer's first report starts its chain at 0; its later reports link
    // into it.
    assert!(
        header_indexes
            .iter()
            .skip(1)
            .any(|&idx| u16::from_le_bytes([records[idx][74], records[idx][75]]) != 0),
        "later headers should remain on a linked results chain"
    );
    let last_header = usize::from(post.players[0].results_last as u16);
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

    let game_data = load_classic_core_files(&target).expect("maint-rust output should load");
    assert_eq!(game_data.players[0].messages_last, 1);
    assert_eq!(game_data.players[0].messages_first, 0);
    assert_eq!(game_data.players[0].messages_last, 1);
    assert_eq!(game_data.players[0].messages_first, 0);
    assert_eq!(game_data.players[1].messages_last, 1);
    assert_eq!(game_data.players[1].messages_first, 0);
    assert_eq!(game_data.players[1].messages_last, 1);
    assert_eq!(game_data.players[1].messages_first, 0);

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

    let game_data = load_classic_core_files(&target).expect("maint-rust output should load");
    assert_eq!(
        diplomatic_relation_toward(&game_data.players[0], 2),
        Some(nc_data::DiplomaticRelation::Enemy)
    );
    assert_eq!(
        diplomatic_relation_toward(&game_data.players[1], 1),
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

    let mut game_data = load_classic_core_files(&target).expect("generated game should load");
    let fleet_a = &mut game_data.fleets[0];
    fleet_a.sector = Coords::new(8, 8);
    fleet_a.order = Order::ScoutSector;
    fleet_a.target = Coords::new(8, 8);
    fleet_a.speed = 0;
    fleet_a.depth = Real48::ONE;
    fleet_a.ships.destroyers = 1;
    fleet_a.ships.cruisers = 0;
    fleet_a.ships.battleships = 0;
    set_troop_transports(fleet_a, 0);
    set_armies_aboard(fleet_a, 0);
    fleet_a.ships.scouts = 1;
    fleet_a.ships.etacs = 0;
    fleet_a.rules_of_engagement = 10;

    let fleet_b = &mut game_data.fleets[(8 * 4) as usize];
    fleet_b.sector = Coords::new(8, 8);
    fleet_b.order = Order::HoldPosition;
    fleet_b.target = Coords::new(8, 8);
    fleet_b.speed = 0;
    fleet_b.depth = Real48::ONE;
    fleet_b.ships.destroyers = 1;
    fleet_b.ships.cruisers = 0;
    fleet_b.ships.battleships = 0;
    set_troop_transports(fleet_b, 0);
    set_armies_aboard(fleet_b, 0);
    fleet_b.ships.scouts = 0;
    fleet_b.ships.etacs = 0;
    fleet_b.rules_of_engagement = 10;

    game_data
        .set_stored_diplomatic_relation(1, 9, nc_data::DiplomaticRelation::Enemy)
        .expect("player 1 -> 9 diplomacy should set");
    game_data
        .set_stored_diplomatic_relation(9, 1, nc_data::DiplomaticRelation::Enemy)
        .expect("player 9 -> 1 diplomacy should set");
    save_classic_core_files(&target, &game_data).expect("mutated game should save");

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

    let mut game_data = load_classic_core_files(&target).expect("generated game should load");
    zero_all_fleets(&mut game_data);
    let (planet_idx, coords, owner_empire) = game_data
        .planets
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner.0 == 2)
        .map(|(idx, planet)| (idx, planet.sector.to_array(), planet.owner.0))
        .expect("generated game should contain an empire 2 world");

    let scout = &mut game_data.fleets[0];
    scout.sector = Coords::new(coords[0].saturating_add(1), coords[1]);
    scout.order = Order::ScoutSolarSystem;
    scout.target = Coords::from_array(coords);
    scout.speed = 3;
    scout.depth = Real48::ZERO;
    scout.ships.scouts = 1;
    save_classic_core_files(&target, &game_data).expect("mutated game should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let game_data = load_classic_core_files(&target).expect("maint-rust output should load");
    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let planet_count = game_data.planets.len();

    let viewer_record = database.record(planet_idx, 0, planet_count);
    assert_ne!(viewer_record.planet_name_bytes(), b"UNKNOWN");
    assert!(!viewer_record.planet_name_bytes().is_empty());
    assert_eq!(viewer_record.raw[0x15], owner_empire);

    let unrelated_viewer_record = database.record(planet_idx, 2, planet_count);
    assert_eq!(unrelated_viewer_record.planet_name_bytes(), b"UNKNOWN");
    assert_ne!(unrelated_viewer_record.raw[0x15], owner_empire);

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

    let mut game_data = load_classic_core_files(&target).expect("generated game should load");
    let (planet_idx, coords, owner_empire) = game_data
        .planets
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner.0 == 2)
        .map(|(idx, planet)| (idx, planet.sector.to_array(), planet.owner.0))
        .expect("generated game should contain an empire 2 world");

    for fleet in &mut game_data.fleets {
        let current = fleet.sector.to_array();
        fleet.speed = 0;
        fleet.order = Order::HoldPosition;
        fleet.target = Coords::from_array(current);
    }

    let scout = &mut game_data.fleets[0];
    scout.sector = Coords::new(coords[0].saturating_add(1), coords[1]);
    scout.order = Order::ScoutSolarSystem;
    scout.target = Coords::from_array(coords);
    scout.speed = 3;
    scout.depth = Real48::ZERO;
    scout.ships.destroyers = 0;
    scout.ships.cruisers = 0;
    scout.ships.battleships = 0;
    set_troop_transports(scout, 0);
    set_armies_aboard(scout, 0);
    scout.ships.etacs = 0;
    scout.ships.scouts = 1;
    save_classic_core_files(&target, &game_data).expect("mutated game should save");

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

    let game_data = load_classic_core_files(&target).expect("maint-rust output should load");
    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let planet_count = game_data.planets.len();
    let viewer_record = database.record(planet_idx, 0, planet_count);
    assert_ne!(viewer_record.planet_name_bytes(), b"UNKNOWN");
    assert!(!viewer_record.planet_name_bytes().is_empty());
    assert_eq!(viewer_record.raw[0x15], owner_empire);

    cleanup_dir(&target);
}

#[test]
fn maint_rust_blockade_arrival_persists_enemy_relation_in_player_dat() {
    let target = unique_temp_dir("nc-cli-maint-rust-blockade-escalation");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    let coords = game_data
        .planets
        .iter()
        .find(|planet| planet.owner.0 == 2)
        .map(|planet| planet.sector.to_array())
        .expect("fixture should contain an empire 2 world");

    let fleet = &mut game_data.fleets[0];
    fleet.sector = Coords::new(coords[0].saturating_add(1), coords[1]);
    fleet.order = Order::GuardBlockadeWorld;
    fleet.target = Coords::from_array(coords);
    fleet.speed = 3;
    fleet.depth = Real48::ZERO;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let game_data = load_classic_core_files(&target).expect("maint-rust output should load");
    assert_eq!(
        diplomatic_relation_toward(&game_data.players[0], 2),
        Some(nc_data::DiplomaticRelation::Enemy)
    );
    assert_eq!(
        diplomatic_relation_toward(&game_data.players[1], 1),
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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    let fleet_a = &mut game_data.fleets[0];
    fleet_a.sector = Coords::new(8, 8);
    fleet_a.order = Order::ScoutSector;
    fleet_a.target = Coords::new(8, 8);
    fleet_a.speed = 0;
    fleet_a.depth = Real48::ONE;
    fleet_a.ships.destroyers = 1;
    fleet_a.ships.cruisers = 0;
    fleet_a.ships.battleships = 0;
    set_troop_transports(fleet_a, 0);
    set_armies_aboard(fleet_a, 0);
    fleet_a.ships.scouts = 1;
    fleet_a.ships.etacs = 0;
    fleet_a.rules_of_engagement = 10;

    let fleet_b = &mut game_data.fleets[4];
    fleet_b.sector = Coords::new(8, 8);
    fleet_b.order = Order::HoldPosition;
    fleet_b.target = Coords::new(8, 8);
    fleet_b.speed = 0;
    fleet_b.depth = Real48::ONE;
    fleet_b.ships.destroyers = 1;
    fleet_b.ships.cruisers = 0;
    // Big enough for the scout to sight it within the year.
    fleet_b.ships.battleships = 3;
    set_troop_transports(fleet_b, 0);
    set_armies_aboard(fleet_b, 0);
    fleet_b.ships.scouts = 0;
    fleet_b.ships.etacs = 0;
    fleet_b.rules_of_engagement = 10;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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

    let game_data = load_classic_core_files(&target).expect("maint-rust output should load");
    assert_eq!(
        diplomatic_relation_toward(&game_data.players[0], 9),
        Some(nc_data::DiplomaticRelation::Enemy)
    );
    assert_eq!(
        diplomatic_relation_toward(&game_data.players[8], 1),
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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    zero_all_fleets(&mut game_data);

    let battle_coords = [10u8, 10u8];
    {
        let f = &mut game_data.fleets[0];
        f.sector = Coords::from_array(battle_coords);
        f.order = Order::ScoutSector;
        f.target = Coords::from_array(battle_coords);
        f.speed = 3;
        f.ships.destroyers = 1;
        set_troop_transports(f, 2);
        set_armies_aboard(f, 0);
        f.ships.scouts = 0;
        f.ships.etacs = 0;
        // Neither side starts a fight, so the aborted fleet withdraws
        // whatever the encounter rolls are.
        f.rules_of_engagement = 0;
    }
    {
        let f = &mut game_data.fleets[4];
        f.sector = Coords::from_array(battle_coords);
        f.order = Order::MoveOnly;
        f.target = Coords::from_array(battle_coords);
        f.speed = 3;
        f.ships.battleships = 50;
        f.ships.cruisers = 50;
        f.ships.destroyers = 50;
        f.rules_of_engagement = 0;
    }
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    // The fixture's empire 1 is rogue and would send its fleets home; this
    // case is about the fleet where it stands.
    game_data.players[0].mode = PlayerMode::Human;
    zero_all_fleets(&mut game_data);

    let battle_coords = [11u8, 11u8];
    {
        let f = &mut game_data.fleets[0];
        f.sector = Coords::from_array(battle_coords);
        f.order = Order::MoveOnly;
        f.target = Coords::from_array(battle_coords);
        f.speed = 3;
        f.ships.destroyers = 3;
        set_troop_transports(f, 2);
        set_armies_aboard(f, 2);
    }
    {
        let f = &mut game_data.fleets[4];
        f.sector = Coords::from_array(battle_coords);
        f.order = Order::HoldPosition;
        f.target = Coords::from_array(battle_coords);
        f.speed = 0;
        f.ships.destroyers = 1;
        set_troop_transports(f, 2);
        set_armies_aboard(f, 0);
    }
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    let starbase_coords = game_data.bases[0].sector.to_array();
    let attacker = &mut game_data.fleets[4];
    attacker.sector = Coords::from_array(starbase_coords);
    attacker.order = Order::MoveOnly;
    attacker.target = Coords::from_array(starbase_coords);
    attacker.speed = 0;
    attacker.depth = Real48::ONE;
    attacker.rules_of_engagement = 10;
    attacker.ships.destroyers = 20;
    attacker.ships.cruisers = 10;
    attacker.ships.battleships = 5;
    attacker.ships.scouts = 0;
    set_troop_transports(attacker, 0);
    attacker.ships.etacs = 0;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");
    // A fleet merely in the sector's deep space is attacked only by or
    // against a declared enemy.
    set_mutual_enemy_in_player_dat(&target, 1, 2);

    // Whether the attacker finds and destroys the base within one year turns
    // on the campaign seed's rolls; pin one where it does.
    let stdout = run_maint_rust_with_export_and_seed(&target, 1, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(text.contains("From your Fleet Command Center"));
    assert!(text.contains("ALERT: Starbase contact lost!"));
    assert!(text.contains("Enemy losses:"));

    let game_data = load_classic_core_files(&target).expect("maint-rust output should load");
    assert_eq!(game_data.players[0].bases_first.0, 0);
    assert!(
        game_data
            .bases
            .iter()
            .all(|base| !(base.sector.to_array() == starbase_coords
                && base.owner.0 == 1
                && base.base_id.0 != 0))
    );

    cleanup_dir(&target);
}

#[test]
fn maint_rust_attacker_battle_report_mentions_destroyed_starbase() {
    let target = unique_temp_dir("nc-cli-maint-rust-attacker-starbase-report");
    copy_fixture_dir("fixtures/ecmaint-starbase-pre/v1.5", &target);

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    let starbase_coords = game_data.bases[0].sector.to_array();
    let attacker = &mut game_data.fleets[4];
    attacker.sector = Coords::from_array(starbase_coords);
    attacker.order = Order::MoveOnly;
    attacker.target = Coords::from_array(starbase_coords);
    attacker.speed = 0;
    attacker.depth = Real48::ONE;
    attacker.rules_of_engagement = 10;
    attacker.ships.destroyers = 20;
    attacker.ships.cruisers = 10;
    attacker.ships.battleships = 5;
    attacker.ships.scouts = 0;
    set_troop_transports(attacker, 0);
    attacker.ships.etacs = 0;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");
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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    for fleet in &mut game_data.fleets {
        fleet.order = Order::HoldPosition;
        fleet.speed = 0;
        fleet.depth = Real48::ZERO;
    }
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    let blocked = &mut game_data.planets[13];
    blocked.owner.0 = 2;
    blocked.years_under_owner = 2;
    set_planet_name(blocked, "TargetPrime");
    blocked.armies = 10;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    let scout = &mut game_data.fleets[0];
    scout.order = Order::ScoutSector;
    scout.target = Coords::new(15, 13);
    scout.ships.scouts = 1;
    scout.ships.etacs = 0;
    // ECGAME writes the 0x22 world-target flag as 0 for sector orders.
    set_mission_target(scout, 0, 0);
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    let scout = &mut game_data.fleets[0];
    scout.order = Order::ScoutSolarSystem;
    scout.target = Coords::new(15, 13);
    scout.ships.scouts = 1;
    scout.ships.etacs = 0;
    // Cruiser speed settles in orbit one sector away within the year.
    recompute_max_speed(scout);
    scout.speed = scout.max_speed;
    game_data.planets[13].stardock.destroyers = 2;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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

    let game_data = load_classic_core_files(&target).expect("maint-rust output should load");
    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let viewer_record = database.record(13, 0, game_data.planets.len());
    assert_eq!(
        viewer_record.planet_name_bytes(),
        game_data.planets[13].name.clone().as_bytes()
    );
    assert_eq!(viewer_record.raw[0x15], game_data.planets[13].owner.0);

    cleanup_dir(&target);
}

#[test]
fn maint_rust_view_world_generates_results_and_database_intel() {
    let target = unique_temp_dir("nc-cli-maint-rust-view-world");
    copy_fixture_dir("fixtures/ecmaint-fleet-pre/v1.5", &target);

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    let viewer = &mut game_data.fleets[0];
    viewer.order = Order::ViewWorld;
    viewer.target = Coords::new(15, 13);
    viewer.ships.scouts = 0;
    viewer.ships.etacs = 0;
    // Cruiser speed settles in orbit one sector away within the year.
    recompute_max_speed(viewer);
    viewer.speed = viewer.max_speed;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Viewing mission report"));
    // Strings near the 75-byte chunk boundary may be split across records in the new Stardate
    // header format. Check for unambiguous early-body content instead.
    assert!(text.contains("entered System(15,13)") || text.contains("long range"));
    let game_data = load_classic_core_files(&target).expect("maint-rust output should load");
    let expected_potential = u16::from(game_data.planets[13].potential_production);
    assert!(text.contains(&format!("potential of {expected_potential} points")));
    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let viewer_record = database.record(13, 0, game_data.planets.len());
    assert_eq!(
        viewer_record.planet_name_bytes(),
        game_data.planets[13].name.clone().as_bytes()
    );
    assert_eq!(viewer_record.raw[0x15], game_data.planets[13].owner.0);
    assert_eq!(
        viewer_record.raw[0x1c],
        u16::from(game_data.planets[13].potential_production) as u8
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

    let foreign_world = &mut game_data.planets[4];
    foreign_world.sector = Coords::new(4, 2);
    foreign_world.owner.0 = 2;
    foreign_world.years_under_owner = 2;
    set_planet_name(foreign_world, "TargetPrime");
    foreign_world.ground_batteries = 3;
    foreign_world.armies = 9;

    let scout = &mut game_data.fleets[0];
    scout.sector = Coords::new(2, 2);
    scout.order = Order::ScoutSolarSystem;
    scout.target = Coords::new(4, 2);
    scout.speed = 3;
    scout.ships.scouts = 1;

    let mover = &mut game_data.fleets[1];
    mover.sector = Coords::new(0, 2);
    mover.order = Order::MoveOnly;
    mover.target = Coords::new(6, 2);
    mover.speed = 3;

    save_classic_core_files(&target, &game_data).expect("baseline should save");

    let database = DatabaseDat::generate_from_planets_and_year(
        &game_data
            .planets
            .iter()
            .map(|planet| planet.name.clone())
            .collect::<Vec<_>>(),
        game_data.conquest.game_year,
        game_data.conquest.player_count as usize,
        None,
    );
    fs::write(target.join("DATABASE.DAT"), database.to_bytes()).expect("DATABASE.DAT should save");

    let stdout = run_maint_rust_with_export(&target, 2);
    assert!(stdout.contains("Turn 1: year 3001"));
    assert!(stdout.contains("Turn 2: year 3002"));

    let game_data = load_classic_core_files(&target).expect("maint-rust output should load");
    let mover_location = game_data.fleets[1].sector.to_array();
    assert_ne!(
        mover_location,
        [4, 2],
        "second-turn routing should avoid the now-known foreign world"
    );

    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let viewer_record = database.record(4, 0, game_data.planets.len());
    assert_eq!(viewer_record.planet_name_bytes(), b"TargetPrime");
    assert_eq!(viewer_record.raw[0x15], 2);

    cleanup_dir(&target);
}

#[test]
fn maint_rust_guard_starbase_generates_arrival_report() {
    let target = unique_temp_dir("nc-cli-maint-rust-guard-starbase");
    copy_fixture_dir("fixtures/ecmaint-starbase-pre/v1.5", &target);

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    let coords = game_data.bases[0].sector.to_array();
    let fleet = &mut game_data.fleets[0];
    fleet.sector = Coords::new(coords[0].saturating_sub(1), coords[1]);
    fleet.order = Order::GuardStarbase;
    fleet.target = Coords::from_array(coords);
    fleet.speed = 3;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    let guard = &mut game_data.fleets[0];
    guard.order = Order::GuardBlockadeWorld;
    guard.target = Coords::new(15, 13);
    guard.ships.scouts = 0;
    guard.ships.etacs = 0;
    // A destroyer reaches orbit one sector away within the year.
    guard.ships.destroyers = 1;
    recompute_max_speed(guard);
    guard.speed = guard.max_speed;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    let (planet_idx, target_coords) = game_data
        .planets
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner.0 == 1)
        .map(|(idx, planet)| (idx, planet.sector.to_array()))
        .expect("fixture should contain an owned planet");
    let start_coords = if target_coords[0] > 1 {
        [target_coords[0] - 1, target_coords[1]]
    } else {
        [target_coords[0] + 1, target_coords[1]]
    };
    let stored_before = stored_production_points(&game_data.planets[planet_idx]);
    let fleet = &mut game_data.fleets[0];
    fleet.sector = Coords::from_array(start_coords);
    fleet.order = Order::Salvage;
    fleet.target = Coords::from_array(target_coords);
    fleet.speed = 3;
    fleet.ships.destroyers = 1;
    fleet.ships.cruisers = 1;
    fleet.ships.battleships = 0;
    fleet.ships.scouts = 0;
    set_troop_transports(fleet, 0);
    set_armies_aboard(fleet, 0);
    fleet.ships.etacs = 0;
    fleet.x_fraction = Real48::HALF;
    fleet.depth = Real48::ZERO;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");
    let db_path = target.join("ncgame.db");
    if db_path.exists() {
        fs::remove_file(&db_path).expect("stale ncgame.db should be removable");
    }

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = String::from_utf8_lossy(&results);
    assert!(text.contains("Salvage mission report"));
    assert!(text.contains("yield 11 production point(s)"));

    let game_data = load_classic_core_files(&target).expect("maint-rust output should load");
    assert_eq!(game_data.fleets.len(), 15);
    assert_eq!(
        stored_production_points(&game_data.planets[planet_idx]),
        stored_before + 11
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

    let game_data = load_classic_core_files(&target).expect("fixture should load");
    let database = DatabaseDat::generate_from_planets_and_year(
        &game_data
            .planets
            .iter()
            .map(|planet| planet.name.clone())
            .collect::<Vec<_>>(),
        game_data.conquest.game_year,
        game_data.conquest.player_count as usize,
        None,
    );
    fs::write(target.join("DATABASE.DAT"), database.to_bytes()).expect("DATABASE.DAT should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let reloaded = load_classic_core_files(&target).expect("maint-rust output should load");
    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let viewer_record = database.record(13, 0, reloaded.planets.len());
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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    let target_world = &mut game_data.planets[13];
    *target_world = Planet {
        sector: Coords::new(15, 13),
        potential_production: 100,
        production: Real48::from_int(100),
        stored_goods: Real48::from_int(66),
        name: "TargetPrime".to_string(),
        armies: 40,
        ground_batteries: 0,
        years_under_owner: 0,
        owner: EmpireId(2),
        ..Planet::default()
    };
    let attacker = &mut game_data.fleets[0];
    attacker.sector = Coords::new(15, 13);
    attacker.order = Order::InvadeWorld;
    attacker.target = Coords::new(15, 13);
    attacker.speed = 0;
    attacker.depth = Real48::HALF;
    attacker.rules_of_engagement = 10;
    attacker.ships.scouts = 0;
    attacker.ships.battleships = 0;
    attacker.ships.cruisers = 0;
    attacker.ships.destroyers = 1;
    set_troop_transports(attacker, 2);
    set_armies_aboard(attacker, 2);
    attacker.ships.etacs = 0;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    let target_world = &mut game_data.planets[13];
    *target_world = Planet {
        sector: Coords::new(15, 13),
        potential_production: 100,
        production: Real48::from_int(100),
        stored_goods: Real48::from_int(66),
        name: "TargetPrime".to_string(),
        armies: 40,
        ground_batteries: 0,
        years_under_owner: 0,
        owner: EmpireId(2),
        ..Planet::default()
    };
    let attacker = &mut game_data.fleets[0];
    attacker.sector = Coords::new(15, 13);
    attacker.order = Order::InvadeWorld;
    attacker.target = Coords::new(15, 13);
    attacker.speed = 0;
    attacker.depth = Real48::HALF;
    attacker.rules_of_engagement = 10;
    attacker.ships.scouts = 0;
    attacker.ships.battleships = 0;
    attacker.ships.cruisers = 0;
    attacker.ships.destroyers = 1;
    set_troop_transports(attacker, 2);
    set_armies_aboard(attacker, 2);
    attacker.ships.etacs = 0;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let reloaded = load_classic_core_files(&target).expect("maint-rust output should load");
    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let viewer_record = database.record(13, 0, reloaded.planets.len());
    assert_eq!(viewer_record.planet_name_bytes(), b"TargetPrime");
    assert_eq!(viewer_record.raw[0x15], 2);
    assert_eq!(
        u16::from_le_bytes([viewer_record.raw[0x16], viewer_record.raw[0x17]]),
        reloaded.conquest.game_year
    );
    assert_eq!(
        u16::from_le_bytes([viewer_record.raw[0x18], viewer_record.raw[0x19]]),
        reloaded.conquest.game_year
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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    let target_world = &mut game_data.planets[13];
    *target_world = Planet {
        sector: Coords::new(15, 13),
        potential_production: 100,
        production: Real48::from_int(100),
        stored_goods: Real48::from_int(66),
        name: "TargetPrime".to_string(),
        armies: 142,
        ground_batteries: 15,
        years_under_owner: 0,
        owner: EmpireId(2),
        ..Planet::default()
    };
    let attacker = &mut game_data.fleets[0];
    attacker.sector = Coords::new(15, 13);
    attacker.order = Order::InvadeWorld;
    attacker.target = Coords::new(15, 13);
    attacker.speed = 0;
    attacker.depth = Real48::HALF;
    attacker.rules_of_engagement = 10;
    attacker.ships.scouts = 0;
    attacker.ships.battleships = 20;
    attacker.ships.cruisers = 20;
    attacker.ships.destroyers = 20;
    set_troop_transports(attacker, 120);
    set_armies_aboard(attacker, 120);
    attacker.ships.etacs = 0;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let reloaded = load_classic_core_files(&target).expect("maint-rust output should load");
    let expected_year = reloaded.conquest.game_year - 1;
    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let viewer_record = database.record(13, 0, reloaded.planets.len());
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
        u8::try_from(reloaded.planets[13].armies).unwrap_or(u8::MAX)
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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    let target_world = &mut game_data.planets[13];
    *target_world = Planet {
        sector: Coords::new(15, 13),
        potential_production: 100,
        production: Real48::from_int(100),
        stored_goods: Real48::from_int(66),
        name: "TargetPrime".to_string(),
        armies: 142,
        ground_batteries: 15,
        years_under_owner: 0,
        owner: EmpireId(2),
        ..Planet::default()
    };
    let attacker = &mut game_data.fleets[0];
    attacker.sector = Coords::new(15, 13);
    attacker.order = Order::InvadeWorld;
    attacker.target = Coords::new(15, 13);
    attacker.speed = 0;
    attacker.depth = Real48::HALF;
    attacker.rules_of_engagement = 10;
    attacker.ships.scouts = 0;
    attacker.ships.battleships = 20;
    attacker.ships.cruisers = 20;
    attacker.ships.destroyers = 20;
    set_troop_transports(attacker, 120);
    set_armies_aboard(attacker, 120);
    attacker.ships.etacs = 0;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    let target_world = &mut game_data.planets[13];
    *target_world = Planet {
        sector: Coords::new(15, 13),
        potential_production: 100,
        production: Real48::from_int(100),
        stored_goods: Real48::from_int(66),
        name: "TargetPrime".to_string(),
        armies: 1,
        ground_batteries: 1,
        years_under_owner: 0,
        owner: EmpireId(2),
        ..Planet::default()
    };
    let attacker = &mut game_data.fleets[0];
    attacker.sector = Coords::new(15, 13);
    attacker.order = Order::BlitzWorld;
    attacker.target = Coords::new(15, 13);
    attacker.speed = 0;
    attacker.depth = Real48::HALF;
    attacker.rules_of_engagement = 10;
    attacker.ships.scouts = 0;
    // Cover fire must pay the battery's 20-hit DS; a lone destroyer cannot.
    attacker.ships.battleships = 1;
    attacker.ships.cruisers = 0;
    attacker.ships.destroyers = 1;
    set_troop_transports(attacker, 10);
    set_armies_aboard(attacker, 10);
    attacker.ships.etacs = 0;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    let target_world = &mut game_data.planets[13];
    *target_world = Planet {
        sector: Coords::new(15, 13),
        potential_production: 100,
        production: Real48::from_int(100),
        stored_goods: Real48::from_int(66),
        name: "TargetPrime".to_string(),
        armies: 1,
        ground_batteries: 1,
        years_under_owner: 0,
        owner: EmpireId(2),
        ..Planet::default()
    };
    let attacker = &mut game_data.fleets[0];
    attacker.sector = Coords::new(15, 13);
    attacker.order = Order::BlitzWorld;
    attacker.target = Coords::new(15, 13);
    attacker.speed = 0;
    attacker.depth = Real48::HALF;
    attacker.rules_of_engagement = 10;
    attacker.ships.scouts = 0;
    // Cover fire must pay the battery's 20-hit DS; a lone destroyer cannot.
    attacker.ships.battleships = 1;
    attacker.ships.cruisers = 0;
    attacker.ships.destroyers = 1;
    set_troop_transports(attacker, 10);
    set_armies_aboard(attacker, 10);
    attacker.ships.etacs = 0;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    let target_world = &mut game_data.planets[13];
    *target_world = Planet {
        sector: Coords::new(15, 13),
        potential_production: 100,
        production: Real48::from_int(100),
        stored_goods: Real48::from_int(66),
        name: "TargetPrime".to_string(),
        armies: 1,
        ground_batteries: 1,
        years_under_owner: 0,
        owner: EmpireId(2),
        ..Planet::default()
    };
    let attacker = &mut game_data.fleets[0];
    attacker.sector = Coords::new(15, 13);
    attacker.order = Order::BlitzWorld;
    attacker.target = Coords::new(15, 13);
    attacker.speed = 0;
    attacker.depth = Real48::HALF;
    attacker.rules_of_engagement = 10;
    attacker.ships.scouts = 0;
    // Cover fire must pay the battery's 20-hit DS; a lone destroyer cannot.
    attacker.ships.battleships = 1;
    attacker.ships.cruisers = 0;
    attacker.ships.destroyers = 1;
    set_troop_transports(attacker, 10);
    set_armies_aboard(attacker, 10);
    attacker.ships.etacs = 0;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let text = decode_chunked_report(&results);
    assert!(
        text.contains("ALERT: Blitz assault successful!") && text.contains("We have seized planet"),
        "{text}"
    );
    assert!(text.contains("Our forces:"), "{text}");

    let reloaded = load_classic_core_files(&target).expect("maint-rust output should load");
    let expected_year = reloaded.conquest.game_year - 1;
    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let viewer_record = database.record(13, 0, reloaded.planets.len());
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
    // Seeded garrison: all 10 landed armies, the lone defender's first
    // volley having killed none.
    assert_eq!(viewer_record.raw[0x23], 10);
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

    let mut game_data = load_classic_core_files(&target).expect("prepared fixture should load");
    set_planet_name(&mut game_data.planets[14], "ssddsf");
    save_classic_core_files(&target, &game_data).expect("named fixture should save");

    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let mut database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    database
        .record_mut(14, 0, game_data.planets.len())
        .set_planet_name("ssddsf");
    fs::write(target.join("DATABASE.DAT"), database.to_bytes()).expect("DATABASE.DAT should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let reloaded = load_classic_core_files(&target).expect("maint-rust output should load");
    let expected_year = reloaded.conquest.game_year - 1;
    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let viewer_record = database.record(14, 0, reloaded.planets.len());
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

    let mut game_data = load_classic_core_files(&target).expect("prepared fixture should load");
    set_planet_name(&mut game_data.planets[14], "ssddsf");
    save_classic_core_files(&target, &game_data).expect("named fixture should save");

    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let mut database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    database
        .record_mut(14, 0, game_data.planets.len())
        .set_planet_name("ssddsf");
    fs::write(target.join("DATABASE.DAT"), database.to_bytes()).expect("DATABASE.DAT should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let reloaded = load_classic_core_files(&target).expect("maint-rust output should load");
    let expected_year = reloaded.conquest.game_year - 1;
    let planet_count = reloaded.planets.len();
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

    let mut game_data = load_classic_core_files(&target).expect("prepared fixture should load");
    set_planet_name(&mut game_data.planets[14], "ssddsf");
    save_classic_core_files(&target, &game_data).expect("named fixture should save");

    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let mut database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    database
        .record_mut(14, 0, game_data.planets.len())
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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    let target_world = &mut game_data.planets[13];
    *target_world = Planet {
        sector: Coords::new(15, 13),
        potential_production: 100,
        production: Real48::from_int(100),
        stored_goods: Real48::from_int(66),
        name: "TargetPrime".to_string(),
        armies: 10,
        ground_batteries: 2,
        years_under_owner: 0,
        owner: EmpireId(2),
        ..Planet::default()
    };
    let attacker = &mut game_data.fleets[0];
    attacker.sector = Coords::new(15, 13);
    attacker.order = Order::BlitzWorld;
    attacker.target = Coords::new(15, 13);
    attacker.speed = 0;
    attacker.depth = Real48::HALF;
    attacker.rules_of_engagement = 10;
    attacker.ships.scouts = 0;
    attacker.ships.battleships = 0;
    attacker.ships.cruisers = 0;
    attacker.ships.destroyers = 1;
    set_troop_transports(attacker, 4);
    set_armies_aboard(attacker, 4);
    attacker.ships.etacs = 0;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let reloaded = load_classic_core_files(&target).expect("maint-rust output should load");
    let database_bytes = fs::read(target.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    let database = DatabaseDat::parse(&database_bytes).expect("DATABASE.DAT should parse");
    let viewer_record = database.record(13, 0, reloaded.planets.len());
    assert_eq!(viewer_record.planet_name_bytes(), b"TargetPrime");
    assert_eq!(viewer_record.raw[0x15], 2);
    assert_eq!(
        u16::from_le_bytes([viewer_record.raw[0x16], viewer_record.raw[0x17]]),
        reloaded.conquest.game_year
    );
    assert_eq!(
        u16::from_le_bytes([viewer_record.raw[0x18], viewer_record.raw[0x19]]),
        reloaded.conquest.game_year
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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    // The fixture's empire 1 is rogue and would send its fleets home; this
    // case is about the fleet where it stands.
    game_data.players[0].mode = PlayerMode::Human;
    game_data.fleets[0].order = Order::MoveOnly;
    // Give the fleet enough scouts to survive the battle (loses but not destroyed).
    // Max scouts (u8::MAX) to survive against 100 enemy battleships.
    game_data.fleets[0].ships.scouts = 255;
    // Reduce enemy fleet 4 (empire 2 at same coords) to 1 BB so our fleet survives.
    game_data.fleets[4].ships.battleships = 1;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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

    // The defender must sight the invader, and the two destroyers must
    // destroy each other, leaving the invader only its transport; both are
    // rolls of the campaign seed, so pin one where they happen.
    let stdout = run_maint_rust_with_export_and_seed(&target, 1, 9);
    assert!(stdout.contains("Rust maintenance complete."));

    let reloaded = load_classic_core_files(&target).expect("maint-rust output should load");
    // The battle comes mid-year, so the fleet may reach home (16,13) and
    // stop there before the year ends.
    let attacker = &reloaded.fleets[0];
    assert!(matches!(
        attacker.order,
        Order::SeekHome | Order::HoldPosition
    ));
    assert_eq!(attacker.target.to_array(), [16, 13]);

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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    let coords = [9, 13];

    let attacker = &mut game_data.fleets[0];
    attacker.sector = Coords::from_array(coords);
    attacker.order = Order::InvadeWorld;
    attacker.target = Coords::new(15, 13);
    attacker.speed = 0;
    attacker.ships.destroyers = 2;
    attacker.ships.cruisers = 4;
    attacker.ships.battleships = 1;
    attacker.ships.scouts = 0;
    set_troop_transports(attacker, 2);
    set_armies_aboard(attacker, 2);
    attacker.ships.etacs = 0;
    attacker.rules_of_engagement = 10;
    // Stalled in place (no speed to restore), so the contact happens here.
    attacker.max_speed = 0;

    let hostile = &mut game_data.fleets[4];
    hostile.sector = Coords::from_array(coords);
    hostile.order = Order::MoveOnly;
    hostile.target = Coords::new(8, 13);
    hostile.speed = 0;
    hostile.ships.destroyers = 0;
    hostile.ships.cruisers = 2;
    hostile.ships.battleships = 0;
    hostile.ships.scouts = 0;
    set_troop_transports(hostile, 0);
    set_armies_aboard(hostile, 0);
    // ETACs carry no guns but make the fleet easy to see (14% a tick), so
    // the invader sights it within the year whatever the encounter rolls.
    hostile.ships.etacs = 500;
    hostile.rules_of_engagement = 0;
    // Stalled in place (no speed to restore), so the contact happens here.
    hostile.max_speed = 0;

    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let reloaded = load_classic_core_files(&target).expect("maint-rust output should load");
    let attacker = &reloaded.fleets[0];
    assert_eq!(attacker.order, Order::InvadeWorld);
    assert_eq!(attacker.target.to_array(), [15, 13]);
    assert_ne!(attacker.order, Order::SeekHome);

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
        .iter()
        .find(|planet| planet.owner.0 == 2)
        .map(|planet| planet.sector.to_array())
        .expect("player 2 should own a world");

    zero_all_fleets(&mut game_data);

    let attacker_idx = game_data
        .fleets
        .iter()
        .position(|fleet| fleet.owner.0 == 1)
        .expect("player 1 fleet");
    let defender_idx = game_data
        .fleets
        .iter()
        .position(|fleet| fleet.owner.0 == 2)
        .expect("player 2 fleet");

    let attacker = &mut game_data.fleets[attacker_idx];
    attacker.sector = Coords::from_array(target_coords);
    attacker.order = Order::BlitzWorld;
    attacker.target = Coords::from_array(target_coords);
    attacker.speed = 0;
    attacker.ships.battleships = 4;
    attacker.ships.cruisers = 8;
    attacker.ships.destroyers = 6;
    set_troop_transports(attacker, 12);
    set_armies_aboard(attacker, 12);
    attacker.rules_of_engagement = 10;

    let defender = &mut game_data.fleets[defender_idx];
    defender.sector = Coords::from_array(target_coords);
    defender.order = Order::HoldPosition;
    defender.target = Coords::from_array(target_coords);
    defender.speed = 0;
    set_troop_transports(defender, 25);
    set_armies_aboard(defender, 0);
    defender.rules_of_engagement = 10;

    let target_world = game_data
        .planets
        .iter_mut()
        .find(|planet| planet.sector.to_array() == target_coords)
        .expect("target world should exist");
    target_world.ground_batteries = 0;
    target_world.armies = 2;

    save_classic_core_files(&target, &game_data).expect("scenario should save");
    write_mutual_enemy_diplomacy(&target, 1, 2);

    // The attacker must sight the transports before its blitz strikes, a
    // roll of the campaign seed; pin one where it does.
    let stdout = run_maint_rust_with_export_and_seed(&target, 1, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let results = fs::read(target.join("RESULTS.DAT")).expect("RESULTS.DAT should exist");
    let attacker_report = joined_report_containing(&results, "ALERT: Enemy fleet contact!");
    assert!(attacker_report.contains("The aliens were completely destroyed."));
    let defender_report = joined_report_containing(&results, "ALERT: Fleet contact lost!");
    assert!(defender_report.contains("Fleet lost:"));

    let reloaded = load_classic_core_files(&target).expect("maint-rust output should load");
    let defender = reloaded
        .fleets
        .iter()
        .find(|fleet| fleet.owner.0 == 2)
        .expect("player 2 fleet should remain addressable");
    assert_eq!(troop_transports(defender), 0);

    cleanup_dir(&target);
}

#[test]
fn maint_rust_invalid_fleet_order_generates_sanitization_report() {
    let target = unique_temp_dir("nc-cli-maint-rust-invalid-fleet-order");
    copy_fixture_dir("fixtures/ecmaint-post/v1.5", &target);

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    let fleet = &mut game_data.fleets[0];
    fleet.sector = Coords::new(15, 13);
    fleet.order = Order::BombardWorld;
    fleet.target = Coords::new(15, 13);
    fleet.speed = 0;
    fleet.depth = Real48::HALF;
    fleet.ships.destroyers = 0;
    fleet.ships.cruisers = 0;
    fleet.ships.battleships = 0;
    fleet.ships.scouts = 1;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    let planet_idx = game_data
        .planets
        .iter()
        .position(|planet| planet.owner.0 == 1)
        .expect("fixture should contain owned planet");
    game_data.players[0].tax_rate = 255;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

    let stdout = run_maint_rust_with_planted_build_slot(
        &target,
        planet_idx,
        BuildSlot {
            points_owed: 12,
            kind: ProductionItemKind::from_code(0xfe),
        },
    );
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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    let fleet = &mut game_data.fleets[0];
    fleet.sector = Coords::new(15, 13);
    fleet.order = Order::BombardWorld;
    fleet.target = Coords::new(15, 13);
    fleet.speed = fleet.max_speed.saturating_add(3);
    fleet.ships.destroyers = 0;
    fleet.ships.cruisers = 0;
    fleet.ships.battleships = 0;
    fleet.ships.scouts = 1;
    fleet.rules_of_engagement = 6;
    // Planet record 15 is Empire #1's world; invalid-input reports for
    // unowned planets are not delivered to anyone.
    game_data.players[0].tax_rate = 255;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

    let stdout = run_maint_rust_with_planted_build_slot(
        &target,
        14,
        BuildSlot {
            points_owed: 9,
            kind: ProductionItemKind::from_code(0xfe),
        },
    );
    assert!(stdout.contains("Rust maintenance complete."));

    let reloaded =
        load_classic_core_files(&target).expect("maint-rust output should remain loadable");
    assert_eq!(reloaded.fleets[0].order, Order::HoldPosition);
    assert_eq!(reloaded.fleets[0].speed, 0);
    assert_eq!(reloaded.fleets[0].rules_of_engagement, 0);
    assert_eq!(reloaded.players[0].tax_rate, 100);
    assert_eq!(reloaded.planets[14].build_queue[0].points_owed, 0);
    assert_eq!(reloaded.planets[14].build_queue[0].kind.code(), 0);

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

        let mut game_data = load_classic_core_files(&target).expect("fixture should load");
        let fleet = &mut game_data.fleets[0];
        fleet.sector = Coords::new(15, 13);
        fleet.order = Order::from_code(order_code);
        fleet.target = Coords::new(15, 13);
        fleet.speed = 99;
        set_mission_target(fleet, 0xfe, 0xfe);
        fleet.ships.destroyers = 0;
        fleet.ships.cruisers = 0;
        fleet.ships.battleships = 0;
        fleet.ships.scouts = 1;
        set_troop_transports(fleet, 1);
        set_armies_aboard(fleet, 4);
        fleet.rules_of_engagement = 42;
        game_data.planets[0].stardock.destroyers = 2;
        game_data.players[0].tax_rate = 255;
        save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

        let stdout = run_maint_rust_with_export(&target, 1);
        assert!(
            stdout.contains("Rust maintenance complete."),
            "maint-rust failed for order code {order_code:#04x}: {stdout}"
        );

        let reloaded =
            load_classic_core_files(&target).expect("maint-rust output should remain loadable");
        assert_eq!(
            reloaded.players[0].tax_rate, 100,
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

        let mut game_data = load_classic_core_files(&target).expect("fixture should load");
        if let Some(fleet) = game_data.fleets.get_mut(0) {
            fleet.order = Order::from_code(0xfe);
            fleet.speed = 99;
            set_troop_transports(fleet, 1);
            set_armies_aboard(fleet, 4);
            fleet.rules_of_engagement = 42;
        }
        if let Some(planet) = game_data.planets.get_mut(0) {
            planet.stardock.destroyers = 2;
        }
        if let Some(player) = game_data.players.get_mut(0) {
            player.tax_rate = 255;
            set_diplomatic_relation_toward(player, 1, DiplomaticRelation::Enemy);
            set_diplomatic_relation_toward(player, 2, DiplomaticRelation::from_code(0xfe));
        }
        save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

        let stdout = run_maint_rust_with_export(&target, 1);
        assert!(stdout.contains("Rust maintenance complete."));

        let reloaded = load_classic_core_files(&target).expect("maint-rust output should load");
        assert!(
            reloaded.players[0].tax_rate <= 100,
            "fixture {fixture} left invalid tax rate {}",
            reloaded.players[0].tax_rate
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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    set_diplomatic_relation_toward(&mut game_data.players[0], 1, DiplomaticRelation::Enemy);
    set_diplomatic_relation_toward(
        &mut game_data.players[0],
        2,
        DiplomaticRelation::from_code(0xfe),
    );
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let reloaded =
        load_classic_core_files(&target).expect("maint-rust output should remain loadable");
    assert_eq!(
        reloaded.players[0].diplomacy[0],
        DiplomaticRelation::Neutral
    );
    assert_eq!(
        reloaded.players[0].diplomacy[1],
        DiplomaticRelation::Neutral
    );

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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    game_data.fleets[0].order = Order::ScoutSector;
    // Remove big ships so fleet loses the battle (scouts absorb hits and survive).
    game_data.fleets[0].ships.battleships = 0;
    game_data.fleets[0].ships.cruisers = 0;
    game_data.fleets[0].ships.destroyers = 0;
    // Max scouts to absorb hits without being fully destroyed.
    game_data.fleets[0].ships.scouts = 255;
    // Reduce enemy fleet 4 (empire 2 at same coords) to 1 BB.
    game_data.fleets[4].ships.battleships = 1;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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
    // Unarmed, the scouts escape after the first exchange and the report
    // says where they withdrew to.
    assert!(
        text.contains("seeking safety")
            || text.contains("seek safety")
            || text.contains("we withdrew toward")
            || text.contains("withdrawing toward")
            || text.contains("holding position")
            || text.contains("ALERT: Fleet contact lost!"),
        "{text}"
    );
    assert!(text.contains("planet \"") || text.contains("System("));

    cleanup_dir(&target);
}

#[test]
fn maint_rust_destroyed_fleet_report_separates_last_contact_from_force_rows() {
    let target = unique_temp_dir("nc-cli-maint-rust-destroyed-fleet-layout");
    copy_fixture_dir("fixtures/ecmaint-fleet-battle-pre/v1.5", &target);
    write_mutual_enemy_diplomacy(&target, 1, 2);

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    // The fixture's empire 1 is rogue and would send its fleets home; this
    // case is about the fleet where it stands.
    game_data.players[0].mode = PlayerMode::Human;
    let fleet = &mut game_data.fleets[0];
    let coords = fleet.sector.to_array();
    fleet.order = Order::HoldPosition;
    fleet.target = Coords::from_array(coords);
    fleet.speed = 0;
    fleet.ships.destroyers = 1;
    fleet.ships.cruisers = 0;
    fleet.ships.battleships = 0;
    set_troop_transports(fleet, 0);
    set_armies_aboard(fleet, 0);
    fleet.ships.scouts = 0;
    fleet.ships.etacs = 0;

    let enemy = &mut game_data.fleets[4];
    enemy.ships.battleships = 9;
    enemy.ships.cruisers = 1;
    enemy.ships.destroyers = 0;
    set_troop_transports(enemy, 13);
    set_armies_aboard(enemy, 12);
    enemy.ships.scouts = 0;
    enemy.ships.etacs = 0;

    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    let fleet = &mut game_data.fleets[0];
    fleet.order = Order::RendezvousSector;
    fleet.target = Coords::new(15, 13);
    // ECGAME writes the 0x22 world-target flag as 0 for sector orders.
    set_mission_target(fleet, 0, 0);
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    game_data.players[0].mode = PlayerMode::Rogue;
    let coords = game_data.fleets[0].sector.to_array();
    game_data.fleets[1].sector = Coords::from_array(coords);
    game_data.fleets[1].order = Order::JoinAnotherFleet;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    game_data.players[0].mode = PlayerMode::Rogue;
    let coords = game_data.fleets[0].sector.to_array();
    game_data.fleets[0].order = Order::RendezvousSector;
    game_data.fleets[1].sector = Coords::from_array(coords);
    game_data.fleets[1].order = Order::RendezvousSector;
    game_data.fleets[2].sector = Coords::from_array(coords);
    game_data.fleets[2].order = Order::RendezvousSector;
    game_data.fleets[3].sector = Coords::from_array(coords);
    game_data.fleets[3].order = Order::RendezvousSector;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    // The fixture's empire 1 is rogue and would send its fleets home; this
    // case is about the fleet where it stands.
    game_data.players[0].mode = PlayerMode::Human;
    game_data.fleets[0].order = Order::JoinAnotherFleet;
    let host_number = game_data.fleets[1].fleet_number.0;
    game_data.fleets[0].mission_target = host_number;
    // Stalled in place (no speed to restore), so the contact happens here.
    game_data.fleets[0].max_speed = 0;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    game_data.players[0].mode = PlayerMode::Rogue;
    let host_number = game_data.fleets[0].fleet_number.0;
    game_data.fleets[0].ships.destroyers = 0;
    game_data.fleets[0].ships.cruisers = 0;
    game_data.fleets[0].ships.battleships = 0;
    game_data.fleets[0].ships.scouts = 0;
    set_troop_transports(&mut game_data.fleets[0], 0);
    game_data.fleets[0].ships.etacs = 0;
    let joiner = &mut game_data.fleets[1];
    joiner.sector = Coords::new(7, 9);
    joiner.order = Order::JoinAnotherFleet;
    joiner.mission_target = host_number;
    joiner.target = Coords::new(10, 10);
    joiner.speed = 3;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    game_data.players[0].mode = PlayerMode::Rogue;
    let host_number = game_data.fleets[0].fleet_number.0;
    let joiner_fleet_number = game_data.fleets[1].fleet_number.0 as usize;
    let joiner = &mut game_data.fleets[1];
    joiner.sector = Coords::new(3, 9);
    joiner.order = Order::JoinAnotherFleet;
    joiner.mission_target = host_number;
    joiner.target = Coords::new(1, 1);
    joiner.speed = 3;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    game_data.players[0].mode = PlayerMode::Rogue;
    let host_number = game_data.fleets[0].fleet_number.0;

    for fleet_idx in [1usize, 2, 3] {
        game_data.fleets[fleet_idx].sector = Coords::new(3, 9);
        game_data.fleets[fleet_idx].order = Order::JoinAnotherFleet;
        game_data.fleets[fleet_idx].mission_target = host_number;
        game_data.fleets[fleet_idx].target = Coords::new(1, 1);
        game_data.fleets[fleet_idx].speed = 3;
    }

    let lost_host = game_data.fleets[4].fleet_number.0;
    game_data.fleets[4].sector = Coords::new(7, 9);
    game_data.fleets[4].order = Order::HoldPosition;
    game_data.fleets[4].target = Coords::new(7, 9);
    game_data.fleets[4].speed = 0;
    game_data.fleets[4].ships.battleships = 0;
    game_data.fleets[4].ships.cruisers = 0;
    game_data.fleets[4].ships.destroyers = 0;
    game_data.fleets[4].ships.scouts = 0;
    set_troop_transports(&mut game_data.fleets[4], 0);
    set_armies_aboard(&mut game_data.fleets[4], 0);
    game_data.fleets[4].ships.etacs = 0;

    for fleet_idx in [5usize, 6, 7] {
        game_data.fleets[fleet_idx].sector = Coords::new(7, 9);
        game_data.fleets[fleet_idx].order = Order::JoinAnotherFleet;
        game_data.fleets[fleet_idx].mission_target = lost_host;
        game_data.fleets[fleet_idx].target = Coords::new(10, 10);
        game_data.fleets[fleet_idx].speed = 3;
    }

    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    game_data.players[0].mode = PlayerMode::CivilDisorder;

    let survivor_coords = [10, 10];
    let survivor_number = game_data.fleets[0].fleet_number.0;
    let absorbed_host_number = game_data.fleets[1].fleet_number.0;

    game_data.fleets[0].sector = Coords::from_array(survivor_coords);
    game_data.fleets[0].order = Order::HoldPosition;
    game_data.fleets[0].target = Coords::from_array(survivor_coords);
    game_data.fleets[0].speed = 0;

    game_data.fleets[1].sector = Coords::from_array(survivor_coords);
    game_data.fleets[1].order = Order::JoinAnotherFleet;
    game_data.fleets[1].mission_target = survivor_number;
    game_data.fleets[1].target = Coords::from_array(survivor_coords);
    game_data.fleets[1].speed = 0;

    game_data.fleets[2].sector = Coords::new(5, 10);
    game_data.fleets[2].order = Order::JoinAnotherFleet;
    game_data.fleets[2].mission_target = absorbed_host_number;
    game_data.fleets[2].target = Coords::from_array(survivor_coords);
    game_data.fleets[2].speed = 3;

    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    game_data.players[0].mode = PlayerMode::Rogue;
    let host_number = game_data.fleets[0].fleet_number.0;

    game_data.fleets[1].sector = Coords::new(3, 9);
    game_data.fleets[1].order = Order::JoinAnotherFleet;
    game_data.fleets[1].mission_target = host_number;
    game_data.fleets[1].target = Coords::new(1, 1);
    game_data.fleets[1].speed = 3;

    game_data.fleets[2].sector = Coords::new(3, 9);
    game_data.fleets[2].order = Order::JoinAnotherFleet;
    game_data.fleets[2].mission_target = host_number;
    game_data.fleets[2].target = Coords::new(1, 1);
    game_data.fleets[2].speed = 3;

    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    // The fixture's empire 1 is rogue and would send its fleets home; this
    // case is about the fleet where it stands.
    game_data.players[0].mode = PlayerMode::Human;
    let coords = game_data.fleets[0].sector.to_array();
    game_data.fleets[0].order = Order::RendezvousSector;
    game_data.fleets[0].target = Coords::from_array(coords);
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    // The fixture's empire 1 is rogue and would send its fleets home; this
    // case is about the fleet where it stands.
    game_data.players[0].mode = PlayerMode::Human;
    game_data.fleets[0].order = Order::GuardBlockadeWorld;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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

    for planet in &mut game_data.planets {
        if planet.owner.0 == 1 {
            planet.owner.0 = 0;
            planet.years_under_owner = 0;
        }
    }
    for fleet in &mut game_data.fleets {
        if fleet.owner.0 == 1 {
            fleet.ships.etacs = 0;
            set_troop_transports(fleet, 0);
            set_armies_aboard(fleet, 0);
        }
    }
    save_classic_core_files(&target, &game_data).expect("mutated game should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let post = load_classic_core_files(&target).expect("maint-rust output should load");
    assert_eq!(post.players[0].mode.code(), 0x00);
    assert_eq!(post.players[0].empire_name.trim().to_string(), "");

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

    for empire in 2..=4u8 {
        for planet in &mut game_data.planets {
            if planet.owner.0 == empire {
                planet.owner.0 = 0;
                planet.years_under_owner = 0;
            }
        }
        for fleet in &mut game_data.fleets {
            if fleet.owner.0 == empire {
                fleet.ships.etacs = 0;
                set_troop_transports(fleet, 0);
                set_armies_aboard(fleet, 0);
                fleet.ships.destroyers = 0;
                fleet.ships.cruisers = 0;
                fleet.ships.battleships = 0;
                fleet.ships.scouts = 0;
            }
        }
    }
    save_classic_core_files(&target, &game_data).expect("mutated game should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let post = load_classic_core_files(&target).expect("maint-rust output should load");
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

    for empire in 2..=4u8 {
        for planet in &mut game_data.planets {
            if planet.owner.0 == empire {
                planet.owner.0 = 0;
                planet.years_under_owner = 0;
            }
        }
        for fleet in &mut game_data.fleets {
            if fleet.owner.0 == empire {
                fleet.ships.etacs = 0;
                set_troop_transports(fleet, 0);
                set_armies_aboard(fleet, 0);
                fleet.ships.destroyers = 0;
                fleet.ships.cruisers = 0;
                fleet.ships.battleships = 0;
                fleet.ships.scouts = 0;
            }
        }
    }
    save_classic_core_files(&target, &game_data).expect("mutated game should save");

    let stdout = run_maint_rust_with_export(&target, 1);
    assert!(stdout.contains("Rust maintenance complete."));

    let post = load_classic_core_files(&target).expect("maint-rust output should load");
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

    for planet in &mut game_data.planets {
        if planet.owner.0 == 1 {
            planet.owner.0 = 0;
            planet.years_under_owner = 0;
        }
    }
    for fleet in &mut game_data.fleets {
        if fleet.owner.0 == 1 {
            fleet.ships.etacs = 0;
            set_troop_transports(fleet, 0);
            set_armies_aboard(fleet, 0);
        }
    }
    save_classic_core_files(&target, &game_data).expect("mutated game should save");

    let stdout = run_maint_rust_with_export(&target, 2);
    assert!(stdout.contains("Rust maintenance complete."));

    let post = load_classic_core_files(&target).expect("maint-rust output should load");
    let remaining_player_fleets = post
        .fleets
        .iter()
        .filter(|fleet| fleet.owner.0 == 1)
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

        let post = load_classic_core_files(&target).expect("multi-turn maint output should load");
        assert_eq!(post.conquest.game_year, 3005);
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
            post.players.len() * post.planets.len()
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
    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    // The fixture's empire 1 is rogue and would send its fleets home; this
    // case is about the fleet where it stands.
    game_data.players[0].mode = PlayerMode::Human;

    // Fleet 0 (empire 1): minimal force with scout mission — will be destroyed.
    {
        let f = &mut game_data.fleets[0];
        f.sector = Coords::from_array(battle_coords);
        f.order = Order::ScoutSector;
        f.target = Coords::from_array(battle_coords);
        f.speed = 3;
        f.ships.battleships = 0;
        f.ships.cruisers = 0;
        f.ships.destroyers = 1;
        f.ships.scouts = 1;
        set_troop_transports(f, 0);
        set_armies_aboard(f, 0);
        f.ships.etacs = 0;
        f.rules_of_engagement = 10;
    }
    // Fleet 4 (empire 2): overwhelming force.
    {
        let f = &mut game_data.fleets[4];
        f.sector = Coords::from_array(battle_coords);
        f.order = Order::MoveOnly;
        f.target = Coords::from_array(battle_coords);
        f.speed = 3;
        f.ships.battleships = 50;
        f.ships.cruisers = 50;
        f.ships.destroyers = 50;
        f.ships.scouts = 0;
        set_troop_transports(f, 0);
        set_armies_aboard(f, 0);
        f.ships.etacs = 0;
        f.rules_of_engagement = 10;
    }
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    let target_world = &mut game_data.planets[13];
    *target_world = Planet {
        sector: Coords::new(15, 13),
        potential_production: 100,
        production: Real48::from_int(100),
        stored_goods: Real48::from_int(66),
        name: "TargetPrime".to_string(),
        armies: 25,
        ground_batteries: 15,
        years_under_owner: 0,
        owner: EmpireId(2),
        ..Planet::default()
    };
    let attacker = &mut game_data.fleets[0];
    attacker.sector = Coords::new(15, 13);
    attacker.order = Order::BombardWorld;
    attacker.target = Coords::new(15, 13);
    attacker.speed = 0;
    attacker.depth = Real48::HALF;
    attacker.rules_of_engagement = 10;
    attacker.ships.scouts = 0;
    attacker.ships.battleships = 0;
    attacker.ships.cruisers = 0;
    attacker.ships.destroyers = 1;
    set_troop_transports(attacker, 0);
    set_armies_aboard(attacker, 0);
    attacker.ships.etacs = 0;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    let target_world = &mut game_data.planets[13];
    *target_world = Planet {
        sector: Coords::new(15, 13),
        potential_production: 100,
        production: Real48::from_int(100),
        stored_goods: Real48::from_int(66),
        name: "TargetPrime".to_string(),
        armies: 25,
        ground_batteries: 15,
        years_under_owner: 0,
        owner: EmpireId(2),
        ..Planet::default()
    };
    let attacker = &mut game_data.fleets[0];
    attacker.sector = Coords::new(15, 13);
    attacker.order = Order::InvadeWorld;
    attacker.target = Coords::new(15, 13);
    attacker.speed = 0;
    attacker.depth = Real48::HALF;
    attacker.rules_of_engagement = 10;
    attacker.ships.scouts = 0;
    attacker.ships.battleships = 0;
    attacker.ships.cruisers = 0;
    attacker.ships.destroyers = 1;
    set_troop_transports(attacker, 5);
    set_armies_aboard(attacker, 5);
    attacker.ships.etacs = 0;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    let target_world = &mut game_data.planets[13];
    *target_world = Planet {
        sector: Coords::new(15, 13),
        potential_production: 100,
        production: Real48::from_int(100),
        stored_goods: Real48::from_int(66),
        name: "TargetPrime".to_string(),
        armies: 25,
        ground_batteries: 15,
        years_under_owner: 0,
        owner: EmpireId(2),
        ..Planet::default()
    };
    let attacker = &mut game_data.fleets[0];
    attacker.sector = Coords::new(15, 13);
    attacker.order = Order::InvadeWorld;
    attacker.target = Coords::new(15, 13);
    attacker.speed = 0;
    attacker.depth = Real48::HALF;
    attacker.rules_of_engagement = 10;
    attacker.ships.scouts = 0;
    attacker.ships.battleships = 0;
    attacker.ships.cruisers = 0;
    attacker.ships.destroyers = 1;
    set_troop_transports(attacker, 5);
    set_armies_aboard(attacker, 5);
    attacker.ships.etacs = 0;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    let target_world = &mut game_data.planets[13];
    *target_world = Planet {
        sector: Coords::new(15, 13),
        potential_production: 100,
        production: Real48::from_int(100),
        stored_goods: Real48::from_int(66),
        name: "TargetPrime".to_string(),
        armies: 25,
        ground_batteries: 15,
        years_under_owner: 0,
        owner: EmpireId(2),
        ..Planet::default()
    };
    let attacker = &mut game_data.fleets[0];
    attacker.sector = Coords::new(15, 13);
    attacker.order = Order::BlitzWorld;
    attacker.target = Coords::new(15, 13);
    attacker.speed = 0;
    attacker.depth = Real48::HALF;
    attacker.rules_of_engagement = 10;
    attacker.ships.scouts = 0;
    attacker.ships.battleships = 0;
    attacker.ships.cruisers = 0;
    attacker.ships.destroyers = 1;
    set_troop_transports(attacker, 5);
    set_armies_aboard(attacker, 5);
    attacker.ships.etacs = 0;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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

    let mut game_data = load_classic_core_files(&target).expect("fixture should load");
    let target_world = &mut game_data.planets[13];
    *target_world = Planet {
        sector: Coords::new(15, 13),
        potential_production: 100,
        production: Real48::from_int(100),
        stored_goods: Real48::from_int(66),
        name: "TargetPrime".to_string(),
        armies: 25,
        ground_batteries: 15,
        years_under_owner: 0,
        owner: EmpireId(2),
        ..Planet::default()
    };
    let attacker = &mut game_data.fleets[0];
    attacker.sector = Coords::new(15, 13);
    attacker.order = Order::BlitzWorld;
    attacker.target = Coords::new(15, 13);
    attacker.speed = 0;
    attacker.depth = Real48::HALF;
    attacker.rules_of_engagement = 10;
    attacker.ships.scouts = 0;
    attacker.ships.battleships = 0;
    attacker.ships.cruisers = 0;
    attacker.ships.destroyers = 1;
    set_troop_transports(attacker, 5);
    set_armies_aboard(attacker, 5);
    attacker.ships.etacs = 0;
    save_classic_core_files(&target, &game_data).expect("mutated fixture should save");

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
