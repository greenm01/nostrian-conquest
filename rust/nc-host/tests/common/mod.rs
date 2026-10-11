#![allow(dead_code)]

use blake3::Hasher;
use nc_data::hosted::HostedStore;
use nc_data::{CampaignSettings, CampaignStore, QueuedPlayerMail, ReportBlockRow};
use nc_host::invite::generate_invite_code;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

pub fn hash_invite_code(code: &str) -> String {
    Hasher::new()
        .update(code.as_bytes())
        .finalize()
        .to_hex()
        .to_string()
}

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn rust_workspace() -> PathBuf {
    repo_root().join("rust")
}

pub fn temp_game_dir(prefix: &str) -> (TempDir, PathBuf) {
    let temp = tempfile::Builder::new()
        .prefix(prefix)
        .tempdir()
        .expect("temp dir should create");
    let path = temp.path().to_path_buf();
    (temp, path)
}

pub fn create_test_game(game_id: &str, player_count: u32) -> (TempDir, PathBuf, HostedStore) {
    let (temp, path) = temp_game_dir(&format!("nc-host-test-{}-", game_id));
    let path = path.clone();
    let game_dir = path.join(game_id);
    fs::create_dir_all(&game_dir).expect("game dir should create");

    let store = init_test_game_dir(&game_dir, game_id, player_count);

    (temp, game_dir, store)
}

pub fn create_test_game_in_root(
    root: &std::path::Path,
    game_id: &str,
    player_count: u32,
) -> (PathBuf, HostedStore) {
    let game_dir = root.join(game_id);
    fs::create_dir_all(&game_dir).expect("game dir should create");
    let store = init_test_game_dir(&game_dir, game_id, player_count);
    (game_dir, store)
}

fn init_test_game_dir(game_dir: &std::path::Path, game_id: &str, player_count: u32) -> HostedStore {
    let db_path = game_dir.join("hosted.db");
    let store = HostedStore::create(&db_path).expect("store should create");
    let now = chrono::Utc::now().timestamp();
    store
        .connection()
        .execute(
            "INSERT INTO game_metadata (id, name, status, created_at, updated_at, current_year, current_turn, players)
             VALUES (?1, ?2, 'setup', ?3, ?3, 3000, 0, ?4)",
            rusqlite::params![game_id, game_id, now, player_count],
        )
        .expect("game metadata should insert");

    let mut existing = std::collections::HashSet::new();
    let invite_codes = (0..player_count)
        .map(|_| {
            let code = generate_invite_code(&existing);
            existing.insert(code.clone());
            code
        })
        .collect::<Vec<_>>();

    nc_data::hosted::create_seats(store.connection(), game_id, &invite_codes)
        .expect("seats should create");

    nc_data::hosted::update_settings(
        store.connection(),
        game_id,
        &nc_data::hosted::GameSettings {
            recruiting: nc_data::hosted::RecruitingMode::NewPlayers,
            lobby_visibility: nc_data::hosted::LobbyVisibility::Public,
            catalog_state: nc_data::hosted::CatalogState::Listed,
            host_alias: Some("Test Host".to_string()),
            summary: Some("Test game for nc-host".to_string()),
            maintenance_enabled: true,
            maintenance_interval_minutes: 1440,
            maintenance_next_due_unix_seconds: None,
            game_tier: nc_data::hosted::GameTier::League,
        },
    )
    .expect("settings should update");

    store
}

pub fn create_seat_with_code(
    store: &HostedStore,
    game_id: &str,
    seat_number: u32,
    invite_code: &str,
) {
    nc_data::hosted::open_seat(store.connection(), game_id, seat_number, invite_code)
        .expect("seat should open");
    let seats = nc_data::hosted::list_seats(store.connection(), game_id).expect("should list");
    assert!(
        seats.iter().any(|s| s.seat_number == seat_number),
        "seat should exist"
    );
}

pub fn cleanup_temp_dir(temp: TempDir) {
    drop(temp);
}

pub fn seed_runtime_snapshot(
    game_dir: &std::path::Path,
    game_id: &str,
    game_name: &str,
    player_count: u8,
    queued_mail: &[QueuedPlayerMail],
    report_block_rows: &[ReportBlockRow],
) {
    let game_data = nc_engine::build_seeded_new_game(player_count, 3000, 12345)
        .expect("game state should build");

    let store = CampaignStore::open_default_in_dir(game_dir).expect("campaign store should open");
    let intel_by_viewer = (1..=player_count)
        .map(|viewer_empire_id| {
            nc_data::merge_player_intel_from_runtime(
                &game_data,
                viewer_empire_id,
                game_data.conquest.game_year,
                None,
                None,
            )
        })
        .collect::<Vec<_>>();
    store
        .save_runtime_state_structured_with_intel(
            &game_data,
            &std::collections::BTreeSet::new(),
            report_block_rows,
            queued_mail,
            &intel_by_viewer,
        )
        .expect("runtime state should save");
    store
        .save_campaign_settings(&CampaignSettings::new(game_id, game_name))
        .expect("campaign settings should save");
}
