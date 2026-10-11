//! Local sighting memory (combat decision 6, surprise): the set arithmetic,
//! the store's own table, and what a classic export and import do with it.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use nc_compat::{
    export_latest_snapshot_to_dir, import_directory_snapshot, load_classic_core_files,
};
use nc_data::{CampaignStore, Coords, DEFAULT_CAMPAIGN_DB_NAME, Sighting, SightingMemory};

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

const HERE: Coords = Coords::new(7, 9);

#[test]
fn a_sighting_is_per_viewer_place_layer_and_opponent() {
    let mut memory = SightingMemory::default();
    assert!(memory.is_empty());
    memory.note(1, HERE, 1, 2);
    memory.note(1, HERE, 1, 25);

    assert!(memory.knows(1, HERE, 1, 2));
    assert!(memory.knows(1, HERE, 1, 25));
    assert!(!memory.knows(1, HERE, 1, 3), "another opponent");
    assert!(
        !memory.knows(2, HERE, 1, 1),
        "the opponent has no memory of it"
    );
    assert!(!memory.knows(1, HERE, 0, 2), "the other layer");
    assert!(!memory.knows(1, Coords::new(9, 7), 1, 2), "another sector");
    assert_eq!(memory.len(), 1, "one place, shared by every unit there");
}

#[test]
fn forgetting_a_place_leaves_the_others() {
    let mut memory = SightingMemory::default();
    memory.note(1, HERE, 1, 2);
    memory.note(1, HERE, 0, 2);
    memory.note(3, HERE, 1, 2);

    memory.forget(1, HERE, 1);
    assert!(!memory.holds(1, HERE, 1));
    assert!(memory.holds(1, HERE, 0) && memory.holds(3, HERE, 1));

    memory.retain(|viewer, _, _| viewer != 1);
    assert_eq!(memory.len(), 1);
    assert!(memory.knows(3, HERE, 1, 2));
}

#[test]
fn memory_iterates_in_a_fixed_order_and_sets_whole() {
    let mut memory = SightingMemory::default();
    memory.note(3, Coords::new(2, 2), 1, 1);
    memory.note(1, Coords::new(5, 1), 0, 4);
    memory.note(1, Coords::new(2, 9), 1, 2);
    memory.note(1, Coords::new(2, 9), 1, 3);

    let places: Vec<_> = memory
        .iter()
        .map(|s| (s.viewer, s.sector.to_array(), s.layer, s.opponents))
        .collect();
    assert_eq!(
        places,
        [
            (1, [2, 9], 1, 0b110),
            (1, [5, 1], 0, 0b1000),
            (3, [2, 2], 1, 0b1)
        ]
    );

    let mut loaded = SightingMemory::default();
    for sighting in memory.iter() {
        loaded.set(sighting);
    }
    assert_eq!(loaded, memory);
    loaded.set(Sighting {
        viewer: 1,
        sector: Coords::new(5, 1),
        layer: 0,
        opponents: 0,
    });
    assert_eq!(loaded.len(), 2, "no opponents is no memory");
}

#[test]
fn empires_outside_the_table_are_never_remembered() {
    let mut memory = SightingMemory::default();
    memory.note(1, HERE, 1, 0);
    memory.note(1, HERE, 1, 40);
    assert!(memory.is_empty());
}

/// The store keeps the memory in its own table, year by year: it comes back
/// as saved, a later year without it comes back without it, and an older
/// snapshot keeps what it had.
#[test]
fn the_store_round_trips_sighting_memory_per_snapshot() {
    let source = repo_root().join("fixtures/ecutil-init/v1.5");
    let dir = temp_dir("nc-data-sighting-store");
    copy_dir_all(&source, &dir);
    let store = CampaignStore::open(dir.join(DEFAULT_CAMPAIGN_DB_NAME)).expect("open store");
    import_directory_snapshot(&store, &dir).expect("import directory");
    let mut state = store
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("runtime snapshot");
    assert!(
        state.game_data.sightings.is_empty(),
        "an import starts empty"
    );

    let year = state.game_data.conquest.game_year;
    state.game_data.sightings.note(1, HERE, 1, 2);
    state.game_data.sightings.note(1, HERE, 1, 3);
    state.game_data.sightings.note(2, HERE, 0, 1);
    let first = store
        .save_runtime_state_structured(year, &state.game_data, &[], &[])
        .expect("save with memory");
    assert_eq!(
        store
            .load_snapshot_game_data(first)
            .expect("load")
            .sightings,
        state.game_data.sightings
    );
    assert_eq!(
        store
            .load_latest_runtime_game_data()
            .expect("load latest")
            .sightings,
        state.game_data.sightings
    );

    let mut later = state.game_data.clone();
    later.conquest.game_year = year + 1;
    later.sightings.forget(1, HERE, 1);
    let second = store
        .save_runtime_state_structured(year + 1, &later, &[], &[])
        .expect("save the next year");
    let loaded = store.load_snapshot_game_data(second).expect("load");
    assert_eq!(loaded.sightings, later.sightings);
    assert_eq!(loaded.sightings.len(), 1);
    assert_eq!(
        store
            .load_snapshot_game_data(first)
            .expect("load the first snapshot")
            .sightings
            .len(),
        2
    );
}

/// A classic export cannot hold the memory: the files are the same as
/// without it, the store keeps it, and importing the export starts fresh.
#[test]
fn a_classic_export_drops_sighting_memory_and_leaves_the_store_alone() {
    let source = repo_root().join("fixtures/ecutil-init/v1.5");
    let imported = temp_dir("nc-data-sighting-export-source");
    let plain = temp_dir("nc-data-sighting-export-plain");
    let exported = temp_dir("nc-data-sighting-export");
    let reimported = temp_dir("nc-data-sighting-reimport");
    copy_dir_all(&source, &imported);
    copy_dir_all(&source, &plain);

    let store = CampaignStore::open(imported.join(DEFAULT_CAMPAIGN_DB_NAME)).expect("open store");
    import_directory_snapshot(&store, &imported).expect("import directory");
    export_latest_snapshot_to_dir(&store, &plain).expect("export without memory");

    let mut state = store
        .load_latest_runtime_state()
        .expect("load runtime state")
        .expect("runtime snapshot");
    state.game_data.sightings.note(1, HERE, 1, 2);
    store
        .save_runtime_state_structured(
            state.game_data.conquest.game_year,
            &state.game_data,
            &state.report_block_rows,
            &state.queued_mail,
        )
        .expect("save runtime snapshot");
    copy_dir_all(&source, &exported);
    export_latest_snapshot_to_dir(&store, &exported).expect("export with memory");

    for file in [
        "PLAYER.DAT",
        "PLANETS.DAT",
        "FLEETS.DAT",
        "BASES.DAT",
        "CONQUEST.DAT",
    ] {
        assert_eq!(
            fs::read(exported.join(file)).expect("exported file"),
            fs::read(plain.join(file)).expect("plain file"),
            "{file}"
        );
    }
    assert!(
        load_classic_core_files(&exported)
            .expect("load exported directory")
            .sightings
            .is_empty()
    );
    assert!(
        store
            .load_latest_runtime_game_data()
            .expect("the store is unchanged")
            .sightings
            .knows(1, HERE, 1, 2)
    );

    copy_dir_all(&exported, &reimported);
    let fresh_store =
        CampaignStore::open(reimported.join(DEFAULT_CAMPAIGN_DB_NAME)).expect("open store");
    import_directory_snapshot(&fresh_store, &reimported).expect("import export");
    assert!(
        fresh_store
            .load_latest_runtime_game_data()
            .expect("load reimported game data")
            .sightings
            .is_empty()
    );
}
