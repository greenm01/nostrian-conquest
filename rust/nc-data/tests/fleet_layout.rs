//! FLEETS.DAT bytes 0x23..0x35 follow the classic layout (t033): a u16
//! mission id at 0x23, empty and loaded transports at 0x2c and 0x2e, scouts at
//! 0x32, and a dead word at 0x34 that the model does not carry.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use nc_compat::{
    export_latest_snapshot_to_dir, import_directory_snapshot, load_classic_core_files,
};
use nc_data::classic::parse_fleets_dat;
use nc_data::classic::{ClassicFile, without_ignored_bytes};
use nc_data::{
    CampaignStore, CoreGameData, DEFAULT_CAMPAIGN_DB_NAME, Fleet, FleetDetachSelection, Mission,
    Order, PlayerMode, recompute_max_speed, set_armies_aboard, set_mission_target,
    set_troop_transports, troop_transports,
};
use nc_engine::run_maintenance_turn;

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

fn copy_dir_files(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).expect("create temp dir");
    for entry in fs::read_dir(src).expect("read src dir") {
        let path = entry.expect("dir entry").path();
        if path.is_file() {
            fs::copy(&path, dst.join(path.file_name().unwrap())).expect("copy file");
        }
    }
}

fn load_fixture(name: &str) -> CoreGameData {
    load_classic_core_files(&repo_root().join("fixtures").join(name).join("v1.5"))
        .unwrap_or_else(|e| panic!("load fixture {name}: {e}"))
}

#[test]
fn original_fleets_dat_reads_the_classic_layout() {
    let bytes = fs::read(repo_root().join("original/v1.5/FLEETS.DAT")).unwrap();
    let fleets = parse_fleets_dat(&bytes).unwrap();
    assert_eq!(fleets.len(), 13);

    // ECUTIL leaves a stray high byte in the mission id of its Guard/Blockade
    // fleets; it is not a scout count.
    let fleet2 = &fleets[1];
    assert_eq!(fleet2.order, Order::GuardBlockadeWorld);
    assert_eq!(fleet2.mission_target, 0x46b0);
    assert_eq!(fleets[2].mission_target, u16::from_le_bytes([154, 183]));
    assert_eq!(fleets[4].mission_target, u16::from_le_bytes([61, 139]));

    // Fleet 1 guards starbase 1.
    assert_eq!(fleets[0].order, Order::GuardStarbase);
    assert_eq!(fleets[0].mission_target, 1);

    for fleet in &fleets {
        assert_eq!(fleet.ships.scouts, 0);
        assert_eq!(fleet.ships.loaded_transports, 0);
        assert_eq!(fleet.ships.loaded_transports, 0);
    }
}

#[test]
fn original_fleets_dat_round_trips_through_the_campaign_database() {
    let imported = temp_dir("nc-data-fleet-layout-import");
    let exported = temp_dir("nc-data-fleet-layout-export");
    copy_dir_files(&repo_root().join("original/v1.5"), &imported);

    let store = CampaignStore::open(imported.join(DEFAULT_CAMPAIGN_DB_NAME)).expect("open store");
    import_directory_snapshot(&store, &imported).expect("import directory");
    export_latest_snapshot_to_dir(&store, &exported).expect("export snapshot");

    // ECUTIL left stack garbage in the dead 0x34 word; the model does not
    // carry it, so the export holds zeros there.
    assert_eq!(
        without_ignored_bytes(
            ClassicFile::Fleets,
            &fs::read(exported.join("FLEETS.DAT")).unwrap()
        ),
        without_ignored_bytes(
            ClassicFile::Fleets,
            &fs::read(imported.join("FLEETS.DAT")).unwrap()
        ),
    );

    let _ = fs::remove_dir_all(&imported);
    let _ = fs::remove_dir_all(&exported);
}

#[test]
fn fleet_accessors_use_the_classic_offsets() {
    let mut fleet = Fleet {
        mission_target: 0x0203,
        rules_of_engagement: 4,
        ..Fleet::default()
    };
    fleet.ships.battleships = 0x0405;
    fleet.ships.cruisers = 0x0607;
    fleet.ships.destroyers = 0x0809;
    fleet.ships.empty_transports = 0x0a0b;
    fleet.ships.loaded_transports = 0x0c0d;
    fleet.ships.etacs = 0x0e0f;
    fleet.ships.scouts = 0x1011;

    // The dead 0x34 word encodes as zero.
    assert_eq!(
        &fleet.to_bytes()[0x23..0x36],
        &[
            0x03, 0x02, 4, 0x05, 0x04, 0x07, 0x06, 0x09, 0x08, 0x0b, 0x0a, 0x0d, 0x0c, 0x0f, 0x0e,
            0x11, 0x10, 0, 0,
        ]
    );
    assert_eq!(fleet.mission_target, 0x0203);
    assert_eq!(fleet.mission_target, 0x0203);
    assert_eq!(fleet.mission_target, 0x0203);
    assert_eq!(troop_transports(&fleet), 0x0a0b + 0x0c0d);
    assert_eq!(fleet.ships.loaded_transports, 0x0c0d);
    assert_eq!(fleet.ships.scouts, 0x1011);
}

#[test]
fn mission_target_writes_the_whole_mission_id_word() {
    let mut fleet = Fleet {
        mission_target: 70 << 8,
        ..Fleet::default()
    };
    set_mission_target(&mut fleet, 1, 3);
    assert_eq!(&fleet.to_bytes()[0x22..0x25], &[1, 3, 0]);
}

#[test]
fn transport_setters_move_transports_between_empty_and_loaded() {
    let mut fleet = Fleet::default();
    set_troop_transports(&mut fleet, 5);
    assert_eq!(
        (fleet.ships.empty_transports, fleet.ships.loaded_transports),
        (5, 0)
    );

    set_armies_aboard(&mut fleet, 3);
    assert_eq!(
        (fleet.ships.empty_transports, fleet.ships.loaded_transports),
        (2, 3)
    );

    set_armies_aboard(&mut fleet, 1);
    assert_eq!(
        (fleet.ships.empty_transports, fleet.ships.loaded_transports),
        (4, 1)
    );

    // Shrinking the total keeps loaded transports first.
    set_armies_aboard(&mut fleet, 3);
    set_troop_transports(&mut fleet, 4);
    assert_eq!(
        (fleet.ships.empty_transports, fleet.ships.loaded_transports),
        (1, 3)
    );
    set_troop_transports(&mut fleet, 2);
    assert_eq!(
        (fleet.ships.empty_transports, fleet.ships.loaded_transports),
        (0, 2)
    );
}

#[test]
fn max_speed_is_the_slowest_ship_type_present() {
    let cases: [(fn(&mut Fleet), u8); 9] = [
        (|_| {}, 7),
        (|f| f.ships.scouts = 1, 6),
        (|f| f.ships.destroyers = 1, 6),
        (|f| f.ships.empty_transports = 1, 5),
        (|f| f.ships.loaded_transports = 1, 5),
        (|f| f.ships.cruisers = 1, 5),
        (|f| f.ships.battleships = 1, 4),
        (|f| f.ships.etacs = 1, 3),
        (
            |f| {
                f.ships.scouts = 1;
                f.ships.battleships = 1;
                f.ships.loaded_transports = 1;
            },
            4,
        ),
    ];
    for (idx, (compose, expected)) in cases.into_iter().enumerate() {
        let mut fleet = Fleet {
            speed: 7,
            ..Fleet::default()
        };
        compose(&mut fleet);
        recompute_max_speed(&mut fleet);
        assert_eq!(fleet.max_speed, expected, "case {idx}");
        assert_eq!(fleet.speed, expected, "case {idx}");
    }
}

/// Appends empire-4 filler fleets until the next record would get id `count`.
fn pad_fleets_to(game_data: &mut CoreGameData, count: usize) {
    let template = game_data.fleets[15].clone();
    while game_data.fleets.len() + 1 < count {
        let mut filler = template.clone();
        let id = (game_data.fleets.len() + 1) as u16;
        filler.record.0 = id;
        filler.fleet_number.0 = id;
        filler.prev.0 = 0;
        filler.next.0 = 0;
        game_data.fleets.push(filler);
    }
}

#[test]
fn join_merges_into_a_host_whose_number_exceeds_one_byte() {
    // 0x23 is the host's fleet number (its 0x00 word), stored as a word; the
    // host's record id (300) differs from its number (299).
    let mut game_data = load_fixture("ecmaint-post");
    game_data.players[0].mode = PlayerMode::CivilDisorder;
    pad_fleets_to(&mut game_data, 300);

    let mut host = game_data.fleets[0].clone();
    host.record.0 = 300;
    host.fleet_number.0 = 299;
    host.prev.0 = 0;
    host.next.0 = 0;
    host.order = Order::HoldPosition;
    host.ships.battleships = 2;
    game_data.fleets.push(host);
    assert_eq!(game_data.fleets.len(), 300);

    let joiner = &mut game_data.fleets[1];
    joiner.order = Order::JoinAnotherFleet;
    set_mission_target(joiner, 1, 299);
    assert_eq!(&joiner.to_bytes()[0x23..0x25], &[0x2b, 0x01]);
    let joiner_cruisers = joiner.ships.cruisers;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance");

    assert!(
        events
            .fleet_merge_events
            .iter()
            .any(|event| { event.kind == Mission::JoinAnotherFleet && !event.survivor_side })
    );
    let host = game_data
        .fleets
        .iter()
        .find(|fleet| fleet.owner.0 == 1 && fleet.fleet_number.0 == 299)
        .expect("host survives");
    assert_eq!(host.ships.battleships, 2);
    assert_eq!(host.ships.cruisers, 1 + joiner_cruisers);
}

#[test]
fn fleet_chain_links_are_words() {
    // t038: ECGAME and ECMAINT read and write 0x03, 0x05, and 0x07 as words.
    let mut bytes = [0u8; 54];
    bytes[0x03..0x09].copy_from_slice(&[0x2c, 0x01, 0x2d, 0x01, 0x2b, 0x01]);
    let mut fleet = Fleet::from_raw(bytes);
    assert_eq!(fleet.next.0, 300);
    assert_eq!(fleet.record.0, 301);
    assert_eq!(fleet.prev.0, 299);
    fleet.prev.0 = 0x0203;
    assert_eq!(&fleet.to_bytes()[0x07..0x09], &[0x03, 0x02]);

    // The invade pre-fixture's byte 0x08 (an old tool's "invasion armies")
    // is the previous-fleet word's high byte.
    let fixture = load_fixture("ecmaint-invade-pre");
    assert_eq!(fixture.fleets[2].prev.0, 0x6402);
}

#[test]
fn detached_fleets_link_to_predecessors_above_one_byte() {
    let mut game_data = load_fixture("ecmaint-post");
    pad_fleets_to(&mut game_data, 300);
    let one_destroyer = FleetDetachSelection {
        destroyers: 1,
        ..FleetDetachSelection::default()
    };
    game_data.fleets[2].ships.destroyers = 3;
    let first = game_data
        .detach_ships_to_new_fleet(1, 3, one_destroyer, None, 6)
        .expect("first detach");
    let second = game_data
        .detach_ships_to_new_fleet(1, 3, one_destroyer, None, 6)
        .expect("second detach");
    let first = &game_data.fleets[first.new_fleet_record_index_1_based - 1];
    let second = &game_data.fleets[second.new_fleet_record_index_1_based - 1];
    assert_eq!(first.record.0, 300);
    assert_eq!(second.record.0, 301);
    assert_eq!(second.prev.0, 300);
    assert_eq!(first.next.0, 301);
}
