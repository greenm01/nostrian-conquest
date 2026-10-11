//! The store's typed snapshot tables carry the whole core game state: what
//! goes in comes back out with the same classic encoding, bit for bit.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use nc_compat::classic::{
    serialize_bases_dat, serialize_conquest_dat, serialize_fleets_dat, serialize_ipbm_dat,
    serialize_planets_dat, serialize_player_dat, serialize_setup_dat,
};
use nc_compat::load_classic_core_files;
use nc_data::model::{BuildSlot, DiplomaticRelation, ProductionItemKind};
use nc_data::{CampaignStore, CoreGameData, Real48};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn temp_store(label: &str) -> CampaignStore {
    let dir = std::env::temp_dir().join(format!(
        "{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time ok")
            .as_nanos()
    ));
    fs::create_dir_all(&dir).expect("create temp dir");
    CampaignStore::open_default_in_dir(&dir).expect("open store")
}

/// Every fixture directory that holds all seven core files.
fn game_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let mut stack = vec![repo_root().join("fixtures")];
    while let Some(dir) = stack.pop() {
        let mut has_player = false;
        for entry in fs::read_dir(&dir).expect("read fixture dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().is_some_and(|name| name == "PLAYER.DAT") {
                has_player = true;
            }
        }
        if has_player && dir.join("IPBM.DAT").is_file() {
            dirs.push(dir);
        }
    }
    dirs.sort();
    dirs
}

fn classic_files(game_data: &CoreGameData) -> [Vec<u8>; 7] {
    [
        serialize_player_dat(&game_data.players),
        serialize_planets_dat(&game_data.planets),
        serialize_fleets_dat(&game_data.fleets),
        serialize_bases_dat(&game_data.bases),
        serialize_ipbm_dat(&game_data.ipbms),
        serialize_setup_dat(&game_data.setup),
        serialize_conquest_dat(&game_data.conquest),
    ]
}

fn store_round_trip(store: &CampaignStore, game_data: &CoreGameData) -> CoreGameData {
    let snapshot_id = store
        .save_runtime_state_structured(
            game_data.conquest.game_year,
            game_data,
            &BTreeSet::new(),
            &[],
            &[],
        )
        .expect("save snapshot");
    store
        .load_snapshot_game_data(snapshot_id)
        .expect("load snapshot")
}

fn assert_round_trips(store: &CampaignStore, game_data: &CoreGameData, label: &Path) {
    let loaded = store_round_trip(store, game_data);
    assert_eq!(&loaded, game_data, "{}", label.display());
    assert_eq!(
        classic_files(&loaded),
        classic_files(game_data),
        "{}",
        label.display()
    );
}

#[test]
fn every_fixture_game_round_trips_through_the_store() {
    let dirs = game_dirs();
    assert!(
        dirs.len() > 40,
        "expected the fixture tree, found {}",
        dirs.len()
    );
    // The fixtures span several game years, and a store refuses a save older
    // than its newest snapshot, so each fixture gets a store of its own.
    for dir in dirs {
        let store = temp_store("nc-data-storage-core-fixtures");
        let game_data = load_classic_core_files(&dir).expect("load fixture");
        assert_round_trips(&store, &game_data, &dir);
        let _ = fs::remove_dir_all(store.path().parent().expect("store dir"));
    }
}

#[test]
fn store_keeps_real48_bits_and_rust_only_values() {
    let dir = repo_root().join("fixtures/ecmaint-build-pre/v1.5");
    let mut game_data = load_classic_core_files(&dir).expect("load fixture");

    // A zero exponent with mantissa and sign bits set equals 0.0 as a
    // number but must keep its bits; so must the largest bit pattern.
    let fleet = &mut game_data.fleets[0];
    fleet.x_fraction = Real48::from_bytes([0x00, 0x12, 0x34, 0x56, 0x78, 0x9A]);
    fleet.y_fraction = Real48::from_bytes([0xFF; 6]);
    fleet.depth = Real48::from_bytes([0x80, 0xB9, 0xFF, 0xFF, 0xFF, 0x7F]);

    // Orders by kind and quantity, past what a byte or a word of points held.
    game_data
        .append_planet_build_units(1, ProductionItemKind::Starbase, 4)
        .expect("order starbases");
    game_data
        .append_planet_build_units(1, ProductionItemKind::Army, 40_000)
        .expect("order armies");
    // Slots only Rust can hold: a kind with nothing owed, and an unknown kind.
    let planet = &mut game_data.planets[0];
    planet.build_queue[3] = BuildSlot {
        points_owed: 0,
        kind: ProductionItemKind::Cruiser,
    };
    planet.build_queue[9] = BuildSlot {
        points_owed: 7,
        kind: ProductionItemKind::Unknown(0xAB),
    };
    planet.name = "Caf\u{e9} \u{ff}".to_string();

    let player = &mut game_data.players[1];
    player.diplomacy[0] = DiplomaticRelation::Enemy;
    player.diplomacy[24] = DiplomaticRelation::Unknown(7);
    player.messages_first = -1;

    game_data.setup.version_tag = [0, b'E', 0xE9, b'1', 0xFF];
    game_data.setup.com_irq = [4, 3, 0, 255];
    game_data.setup.com_hw_flow_control = [true, false, false, true];

    let conquest = &mut game_data.conquest;
    conquest.maintenance_days.0 = [true, false, true, false, false, false, true];
    conquest.previous_production[24] = u16::MAX;
    conquest.previous_planet_count[0] = 255;

    let store = temp_store("nc-data-storage-core-values");
    assert_round_trips(&store, &game_data, &dir);
    let loaded = store_round_trip(&store, &game_data);
    assert_eq!(
        loaded.fleets[0].x_fraction.to_bytes(),
        [0x00, 0x12, 0x34, 0x56, 0x78, 0x9A]
    );
}
