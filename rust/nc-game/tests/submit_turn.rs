use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use nc_data::{CampaignStore, CoreGameData, Order};

static TEMP_DIR_SEQ: AtomicU64 = AtomicU64::new(0);

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

#[test]
fn submit_turn_check_mode_does_not_create_runtime_db() {
    let target = fixture_copy("nc-game-submit-turn-check");
    let turn_path = target.join("turn.kdl");
    fs::write(
        &turn_path,
        r#"
turn player=1 year=3000
tax rate=42
"#,
    )
    .unwrap();
    let db_path = target.join("ncgame.db");
    assert!(!db_path.exists());

    let output = Command::new(env!("CARGO_BIN_EXE_nc-game"))
        .args([
            "submit-turn",
            "--check",
            "--dir",
            target.to_str().unwrap(),
            "--player",
            "1",
            "--file",
            turn_path.to_str().unwrap(),
        ])
        .output()
        .expect("nc-game submit-turn should run");

    assert!(
        output.status.success(),
        "nc-game submit-turn failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Validated turn submission"));
    assert!(stdout.contains("mode=check-only"));
    assert!(!db_path.exists());

    cleanup_dir(&target);
}

#[test]
fn submit_turn_apply_updates_runtime_state_and_queued_mail() {
    let target = fixture_copy("nc-game-submit-turn-apply");
    let turn_path = target.join("turn.kdl");
    fs::write(
        &turn_path,
        r#"
turn player=1 year=3000
tax rate=37
message to=2 subject="Scout" body="Watch the lane."
"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_nc-game"))
        .args([
            "submit-turn",
            "--dir",
            target.to_str().unwrap(),
            "--player",
            "1",
            "--file",
            turn_path.to_str().unwrap(),
        ])
        .output()
        .expect("nc-game submit-turn should run");

    assert!(
        output.status.success(),
        "nc-game submit-turn failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Applied turn submission"));
    assert!(target.join("ncgame.db").exists());

    let store = CampaignStore::open_default_in_dir(&target).unwrap();
    let state = store.load_latest_runtime_state().unwrap().unwrap();
    assert_eq!(state.game_data.player.records[0].tax_rate(), 37);
    assert_eq!(state.queued_mail.len(), 1);
    assert_eq!(state.queued_mail[0].sender_empire_id, 1);
    assert_eq!(state.queued_mail[0].recipient_empire_id, 2);
    assert_eq!(state.queued_mail[0].subject, "Scout");
    assert_eq!(state.queued_mail[0].body, "Watch the lane.");

    cleanup_dir(&target);
}

#[test]
fn submit_turn_rejects_cli_and_kdl_player_mismatch() {
    let target = fixture_copy("nc-game-submit-turn-mismatch");
    let turn_path = target.join("turn.kdl");
    fs::write(
        &turn_path,
        r#"
turn player=2 year=3000
tax rate=20
"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_nc-game"))
        .args([
            "submit-turn",
            "--check",
            "--dir",
            target.to_str().unwrap(),
            "--player",
            "1",
            "--file",
            turn_path.to_str().unwrap(),
        ])
        .output()
        .expect("nc-game submit-turn should run");

    assert!(
        !output.status.success(),
        "nc-game submit-turn unexpectedly succeeded: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("player mismatch"));

    cleanup_dir(&target);
}

#[test]
fn root_help_lists_submit_turn_subcommand() {
    let output = Command::new(env!("CARGO_BIN_EXE_nc-game"))
        .arg("--help")
        .output()
        .expect("nc-game --help should run");

    assert!(
        output.status.success(),
        "nc-game --help failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("nc-game submit-turn"));
}

#[test]
fn submit_turn_keeps_motion_for_on_station_hostile_orders() {
    let target = fixture_copy("nc-game-submit-turn-on-station-hostile-order");
    let turn_path = target.join("turn.kdl");

    let mut game_data = CoreGameData::load(&target).expect("fixture should load");
    let fleet_record_index_1_based = game_data
        .fleets
        .records
        .iter()
        .enumerate()
        .find(|(_, fleet)| fleet.owner_empire_raw() == 1)
        .map(|(idx, _)| idx + 1)
        .expect("fixture should contain a player 1 fleet");
    let target_planet_record_index_1_based = game_data
        .planets
        .records
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner_empire_slot_raw() == 2)
        .map(|(idx, _)| idx + 1)
        .expect("fixture should contain a player 2 world");
    let target_coords =
        game_data.planets.records[target_planet_record_index_1_based - 1].coords_raw();
    let year = game_data.conquest.game_year();

    {
        let fleet = &mut game_data.fleets.records[fleet_record_index_1_based - 1];
        fleet.set_current_location_coords_raw(target_coords);
        fleet.set_battleship_count(1);
        fleet.set_cruiser_count(0);
        fleet.set_destroyer_count(1);
        fleet.set_troop_transport_count(2);
        fleet.set_army_count(6);
        fleet.set_scout_count(0);
        fleet.set_etac_count(0);
        fleet.recompute_max_speed_from_composition();
        fleet.set_current_speed(fleet.max_speed());
    }
    let depth_before = game_data.fleets.records[fleet_record_index_1_based - 1].depth();
    game_data
        .save(&target)
        .expect("mutated fixture should save");

    fs::write(
        &turn_path,
        format!(
            r#"
turn player=1 year={year}
fleet record={fleet_record_index_1_based} {{
  order speed=4 kind="invade" x={x} y={y}
}}
"#,
            x = target_coords[0],
            y = target_coords[1],
        ),
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_nc-game"))
        .args([
            "submit-turn",
            "--dir",
            target.to_str().unwrap(),
            "--player",
            "1",
            "--file",
            turn_path.to_str().unwrap(),
        ])
        .output()
        .expect("nc-game submit-turn should run");

    assert!(
        output.status.success(),
        "nc-game submit-turn failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let store = CampaignStore::open_default_in_dir(&target).unwrap();
    let state = store.load_latest_runtime_state().unwrap().unwrap();
    let fleet = &state.game_data.fleets.records[fleet_record_index_1_based - 1];

    assert_eq!(fleet.standing_order_kind(), Order::InvadeWorld);
    assert_eq!(fleet.current_location_coords_raw(), target_coords);
    assert_eq!(fleet.standing_order_target_coords_raw(), target_coords);
    // Like ECGAME, the order keeps the fleet's motion; the strike test
    // passes once maintenance finds it settled in orbit.
    assert_eq!(fleet.current_speed(), 4);
    assert_eq!(fleet.depth(), depth_before);

    cleanup_dir(&target);
}
