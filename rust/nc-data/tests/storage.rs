mod common;

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use nc_compat::{
    DatabaseDat, export_latest_snapshot_to_dir, import_directory_snapshot,
    import_directory_snapshot_with_seed,
};
use nc_data::{CampaignStore, CampaignStoreError, DEFAULT_CAMPAIGN_DB_NAME, SeatBindSource};
use rusqlite::Connection;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn temp_dir(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time ok")
            .as_nanos()
    ))
}

fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) {
    fs::create_dir_all(dst).expect("create temp dir");
    for entry in fs::read_dir(src).expect("read src dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_all(&path, &target);
        } else {
            fs::copy(&path, &target).expect("copy file");
        }
    }
}

#[test]
fn sqlite_store_round_trips_directory_export() {
    let source = repo_root().join("fixtures/ecutil-init/v1.5");
    let imported = temp_dir("nc-data-storage-import");
    let exported = temp_dir("nc-data-storage-export");
    copy_dir_all(&source, &imported);

    let store = CampaignStore::open(imported.join(DEFAULT_CAMPAIGN_DB_NAME)).expect("open store");
    import_directory_snapshot(&store, &imported).expect("import directory");
    let year = export_latest_snapshot_to_dir(&store, &exported).expect("export snapshot");
    assert_eq!(year, 3000);

    for name in [
        "PLAYER.DAT",
        "PLANETS.DAT",
        "FLEETS.DAT",
        "BASES.DAT",
        "IPBM.DAT",
        "SETUP.DAT",
        "CONQUEST.DAT",
        "MESSAGES.DAT",
        "RESULTS.DAT",
    ] {
        assert_eq!(
            fs::read(imported.join(name)).expect("source bytes"),
            fs::read(exported.join(name)).expect("exported bytes"),
            "mismatch for {name}",
        );
    }

    let database_bytes =
        fs::read(exported.join("DATABASE.DAT")).expect("DATABASE.DAT should exist");
    assert_eq!(
        fs::read(imported.join("DATABASE.DAT")).expect("source DATABASE.DAT"),
        database_bytes,
        "mismatch for DATABASE.DAT",
    );
    let database = DatabaseDat::parse(&database_bytes).expect("exported DATABASE.DAT should parse");
    assert_eq!(
        database.records.len(),
        80,
        "expected 4x20 DATABASE.DAT records"
    );
}

#[test]
fn sqlite_store_preserves_orbit_seed_rows_as_orbit_intel() {
    let source = repo_root().join("fixtures/ecutil-init/v1.5");
    let imported = temp_dir("nc-data-storage-orbit-seed");
    copy_dir_all(&source, &imported);

    let store = CampaignStore::open(imported.join(DEFAULT_CAMPAIGN_DB_NAME)).expect("open store");
    import_directory_snapshot(&store, &imported).expect("import directory");

    for (viewer_empire_id, planet_record_index) in
        [(1, 15usize), (2, 13usize), (3, 5usize), (4, 6usize)]
    {
        let snapshot = store
            .latest_planet_intel_for_viewer(viewer_empire_id)
            .expect("load viewer intel")
            .into_iter()
            .find(|row| row.planet_record_index_1_based == planet_record_index)
            .expect("orbit seed row should exist");
        assert!(
            snapshot.compat_is_orbit_seed,
            "viewer {viewer_empire_id} planet {planet_record_index} should stay tagged as orbit seed"
        );
    }
}

#[test]
fn sqlite_store_latest_runtime_game_data_ignores_stale_classic_files() {
    let source = repo_root().join("fixtures/ecutil-init/v1.5");
    let imported = temp_dir("nc-data-storage-runtime-authority");
    copy_dir_all(&source, &imported);

    let store = CampaignStore::open(imported.join(DEFAULT_CAMPAIGN_DB_NAME)).expect("open store");
    import_directory_snapshot(&store, &imported).expect("import directory");

    let mut state = store
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("runtime snapshot");
    state.game_data.player.records[0].set_controlled_empire_name_raw("Runtime Empire");
    store
        .save_runtime_state_structured(
            &state.game_data,
            &state.planet_scorch_orders,
            &state.report_block_rows,
            &state.queued_mail,
        )
        .expect("save runtime snapshot");

    let runtime_game = store
        .load_latest_runtime_game_data()
        .expect("load latest runtime game data");
    assert_eq!(
        runtime_game.player.records[0].controlled_empire_name_summary(),
        "Runtime Empire"
    );

    let classic_game = nc_data::CoreGameData::load(&imported).expect("load classic directory");
    assert_ne!(
        classic_game.player.records[0].controlled_empire_name_summary(),
        "Runtime Empire"
    );
}

#[test]
fn sqlite_store_persists_explicit_campaign_seed() {
    let source = repo_root().join("fixtures/ecutil-init/v1.5");
    let imported = temp_dir("nc-data-storage-seed-explicit");
    copy_dir_all(&source, &imported);

    let store = CampaignStore::open(imported.join(DEFAULT_CAMPAIGN_DB_NAME)).expect("open store");
    let expected_seed = 0xEC15_2026_0000_0042u64;
    import_directory_snapshot_with_seed(&store, &imported, Some(expected_seed))
        .expect("import directory");

    let initial = store
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("runtime snapshot");
    assert_eq!(initial.campaign_seed, expected_seed);

    store
        .save_runtime_state_structured(
            &initial.game_data,
            &initial.planet_scorch_orders,
            &initial.report_block_rows,
            &initial.queued_mail,
        )
        .expect("resave runtime state");

    let reloaded = store
        .load_latest_runtime_state()
        .expect("reload runtime state")
        .expect("runtime snapshot");
    assert_eq!(reloaded.campaign_seed, expected_seed);
}

#[test]
fn sqlite_store_generates_and_reuses_campaign_seed() {
    let source = repo_root().join("fixtures/ecutil-init/v1.5");
    let imported = temp_dir("nc-data-storage-seed-generated");
    copy_dir_all(&source, &imported);

    let store = CampaignStore::open(imported.join(DEFAULT_CAMPAIGN_DB_NAME)).expect("open store");
    import_directory_snapshot(&store, &imported).expect("import directory");

    let initial = store
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("runtime snapshot");
    assert_ne!(initial.campaign_seed, 0);

    store
        .save_runtime_state_structured(
            &initial.game_data,
            &initial.planet_scorch_orders,
            &initial.report_block_rows,
            &initial.queued_mail,
        )
        .expect("resave runtime state");

    let reloaded = store
        .load_latest_runtime_state()
        .expect("reload runtime state")
        .expect("runtime snapshot");
    assert_eq!(reloaded.campaign_seed, initial.campaign_seed);
}

#[test]
fn sqlite_store_persists_planet_scorch_orders() {
    let source = repo_root().join("fixtures/ecutil-init/v1.5");
    let imported = temp_dir("nc-data-storage-scorch");
    copy_dir_all(&source, &imported);

    let store = CampaignStore::open(imported.join(DEFAULT_CAMPAIGN_DB_NAME)).expect("open store");
    import_directory_snapshot(&store, &imported).expect("import directory");

    let mut initial = store
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("runtime snapshot");
    initial.planet_scorch_orders = BTreeSet::from([3usize, 7usize]);

    store
        .save_runtime_state_structured(
            &initial.game_data,
            &initial.planet_scorch_orders,
            &initial.report_block_rows,
            &initial.queued_mail,
        )
        .expect("resave runtime state");

    let reloaded = store
        .load_latest_runtime_state()
        .expect("reload runtime state")
        .expect("runtime snapshot");
    assert_eq!(
        reloaded.planet_scorch_orders,
        BTreeSet::from([3usize, 7usize])
    );
}

#[test]
fn sqlite_store_schema_has_no_blob_columns_or_compat_files_table() {
    let source = repo_root().join("fixtures/ecutil-init/v1.5");
    let imported = temp_dir("nc-data-storage-schema");
    copy_dir_all(&source, &imported);

    let store_path = imported.join(DEFAULT_CAMPAIGN_DB_NAME);
    let store = CampaignStore::open(&store_path).expect("open store");
    import_directory_snapshot(&store, &imported).expect("import directory");

    let conn = Connection::open(store_path).expect("open sqlite db");
    let table_names = conn
        .prepare(
            "SELECT name
             FROM sqlite_master
             WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
             ORDER BY name",
        )
        .expect("prepare table list")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query table list")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect table list");
    assert!(
        !table_names.iter().any(|name| name == "compat_files"),
        "compat_files table should be gone: {table_names:?}"
    );
    assert!(
        !table_names
            .iter()
            .any(|name| name == "compat_database_record_fields"),
        "compat_database_record_fields table should be gone: {table_names:?}"
    );
    for legacy in [
        "player_record_fields",
        "planet_record_fields",
        "fleet_record_fields",
        "base_record_fields",
        "ipbm_record_fields",
        "setup_record_fields",
        "conquest_record_fields",
    ] {
        assert!(
            !table_names.iter().any(|name| name == legacy),
            "legacy byte table {legacy} should be gone: {table_names:?}"
        );
    }
    for retired in [
        "hosted_player_seats",
        "hosted_publish_jobs",
        "active_sessions",
    ] {
        assert!(
            !table_names.iter().any(|name| name == retired),
            "retired hosted/session table {retired} should be gone: {table_names:?}"
        );
    }
    for current in [
        "snapshot_players",
        "snapshot_planets",
        "snapshot_fleets",
        "snapshot_bases",
        "snapshot_ipbms",
        "snapshot_setup",
        "snapshot_conquest",
    ] {
        assert!(
            table_names.iter().any(|name| name == current),
            "normalized snapshot table {current} should exist: {table_names:?}"
        );
    }

    let schema_rows = conn
        .prepare(
            "SELECT sql
             FROM sqlite_master
             WHERE type = 'table' AND sql IS NOT NULL",
        )
        .expect("prepare schema query")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query schema")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect schema");
    assert!(
        schema_rows
            .iter()
            .any(|sql| sql.contains("known_starbase_count INTEGER")),
        "planet_intel schema should include known_starbase_count"
    );
    assert!(
        schema_rows
            .iter()
            .any(|sql| sql.contains("control_word_0a_raw INTEGER NOT NULL")),
        "snapshot schema should expose explicit conquest control words"
    );
    assert!(
        schema_rows
            .iter()
            .any(|sql| sql.contains("control_byte_54_raw INTEGER NOT NULL")),
        "snapshot schema should expose explicit conquest control bytes"
    );
    for sql in schema_rows {
        assert!(
            !sql.contains("compat_raw_hex"),
            "legacy whole-record compat_raw_hex columns should be gone: {sql}"
        );
        assert!(
            !sql.contains("compat_prelude_raw_hex"),
            "setup residue slab should be gone: {sql}"
        );
        assert!(
            !sql.contains("control_header_tail_raw_hex"),
            "legacy grouped conquest residue column should be gone: {sql}"
        );
        assert!(
            !sql.to_ascii_uppercase().contains("BLOB"),
            "sqlite schema should not use BLOB columns: {sql}"
        );
    }
}

#[test]
fn sqlite_store_seat_bind_sources_default_to_real_name_on_existing_databases() {
    let source = repo_root().join("fixtures/ecutil-init/v1.5");
    let imported = temp_dir("nc-data-storage-seat-bind-sources");
    copy_dir_all(&source, &imported);
    let store_path = imported.join(DEFAULT_CAMPAIGN_DB_NAME);
    let store = CampaignStore::open(&store_path).expect("open store");
    import_directory_snapshot(&store, &imported).expect("import directory");
    // A database written before seat bind sources were stored has no table.
    Connection::open(&store_path)
        .expect("open sqlite db")
        .execute_batch("DROP TABLE seat_bind_sources;")
        .expect("drop seat bind sources");

    let store = CampaignStore::open(&store_path).expect("reopen existing store");
    assert_eq!(
        store.handle_bound_seats().expect("handle-bound seats"),
        Vec::<usize>::new()
    );

    store
        .set_seat_bind_source(3, SeatBindSource::Handle)
        .expect("record handle binding");
    store
        .set_seat_bind_source(2, SeatBindSource::Handle)
        .expect("record handle binding");
    store
        .set_seat_bind_source(4, SeatBindSource::RealName)
        .expect("record real-name binding");
    assert_eq!(
        store.handle_bound_seats().expect("handle-bound seats"),
        vec![2, 3]
    );

    store
        .set_seat_bind_source(3, SeatBindSource::RealName)
        .expect("rebind seat by real name");
    assert_eq!(
        store.handle_bound_seats().expect("handle-bound seats"),
        vec![2]
    );
}

#[test]
fn sqlite_store_rejects_legacy_byte_table_schema() {
    let imported = temp_dir("nc-data-storage-legacy-schema");
    fs::create_dir_all(&imported).expect("create temp dir");
    let store_path = imported.join(DEFAULT_CAMPAIGN_DB_NAME);
    let conn = Connection::open(&store_path).expect("open sqlite db");
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE snapshots (
             id INTEGER PRIMARY KEY,
             game_year INTEGER NOT NULL UNIQUE
         );
         CREATE TABLE campaign_metadata (
             key TEXT PRIMARY KEY,
             int_value INTEGER NOT NULL
         );
         CREATE TABLE player_record_fields (
             snapshot_id INTEGER NOT NULL,
             record_index INTEGER NOT NULL,
             byte_offset INTEGER NOT NULL,
             byte_value INTEGER NOT NULL,
             PRIMARY KEY(snapshot_id, record_index, byte_offset)
         );
         INSERT INTO snapshots(id, game_year) VALUES (1, 3000);",
    )
    .expect("seed legacy schema");
    drop(conn);

    let err = CampaignStore::open(&store_path).expect_err("legacy schema should be rejected");
    assert!(
        matches!(
            err,
            CampaignStoreError::SchemaVersionMismatch {
                expected: 12,
                found: None
            }
        ),
        "unexpected error: {err}"
    );
}

#[test]
fn sqlite_store_rejects_prior_runtime_schema_version() {
    let imported = temp_dir("nc-data-storage-prior-runtime-schema");
    fs::create_dir_all(&imported).expect("create temp dir");
    let store_path = imported.join(DEFAULT_CAMPAIGN_DB_NAME);
    let conn = Connection::open(&store_path).expect("open sqlite db");
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE snapshots (
             id INTEGER PRIMARY KEY,
             game_year INTEGER NOT NULL UNIQUE
         );
         CREATE TABLE campaign_metadata (
             key TEXT PRIMARY KEY,
             int_value INTEGER NOT NULL
         );
         INSERT INTO campaign_metadata(key, int_value)
         VALUES ('runtime_schema_version', 11);",
    )
    .expect("seed prior runtime schema");
    drop(conn);

    let err =
        CampaignStore::open(&store_path).expect_err("prior runtime schema should be rejected");
    assert!(
        matches!(
            err,
            CampaignStoreError::SchemaVersionMismatch {
                expected: 12,
                found: Some(11)
            }
        ),
        "unexpected error: {err}"
    );
}
