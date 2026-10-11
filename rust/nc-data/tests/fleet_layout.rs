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
    Order,
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
    assert_eq!(fleet2.standing_order_kind(), Order::GuardBlockadeWorld);
    assert_eq!(fleet2.mission_id_raw(), 0x46b0);
    assert_eq!(fleets[2].mission_id_raw(), u16::from_le_bytes([154, 183]));
    assert_eq!(fleets[4].mission_id_raw(), u16::from_le_bytes([61, 139]));

    // Fleet 1 guards starbase 1.
    assert_eq!(fleets[0].standing_order_kind(), Order::GuardStarbase);
    assert_eq!(fleets[0].guard_starbase_index_raw(), 1);

    for fleet in &fleets {
        assert_eq!(fleet.scout_count(), 0);
        assert_eq!(fleet.loaded_transport_count(), 0);
        assert_eq!(fleet.army_count(), 0);
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
    let mut fleet = Fleet::default();
    fleet.set_mission_id_raw(0x0203);
    fleet.set_rules_of_engagement(4);
    fleet.set_battleship_count(0x0405);
    fleet.set_cruiser_count(0x0607);
    fleet.set_destroyer_count(0x0809);
    fleet.set_empty_transport_count(0x0a0b);
    fleet.set_loaded_transport_count(0x0c0d);
    fleet.set_etac_count(0x0e0f);
    fleet.set_scout_count(0x1011);

    // The dead 0x34 word encodes as zero.
    assert_eq!(
        &fleet.to_bytes()[0x23..0x36],
        &[
            0x03, 0x02, 4, 0x05, 0x04, 0x07, 0x06, 0x09, 0x08, 0x0b, 0x0a, 0x0d, 0x0c, 0x0f, 0x0e,
            0x11, 0x10, 0, 0,
        ]
    );
    assert_eq!(fleet.mission_id_raw(), 0x0203);
    assert_eq!(fleet.join_host_fleet_id_raw(), 0x0203);
    assert_eq!(fleet.guard_starbase_index_raw(), 0x0203);
    assert_eq!(fleet.troop_transport_count(), 0x0a0b + 0x0c0d);
    assert_eq!(fleet.army_count(), 0x0c0d);
    assert_eq!(fleet.scout_count(), 0x1011);
}

#[test]
fn mission_target_writes_the_whole_mission_id_word() {
    let mut fleet = Fleet::default();
    fleet.set_mission_id_raw(70 << 8);
    fleet.set_mission_target(1, 3);
    assert_eq!(&fleet.to_bytes()[0x22..0x25], &[1, 3, 0]);

    // The raw aux override touches only 0x22 and 0x23.
    fleet.set_mission_id_raw((70 << 8) | 3);
    fleet.set_mission_aux_bytes([1, 5]);
    assert_eq!(&fleet.to_bytes()[0x22..0x25], &[1, 5, 70]);
}

#[test]
fn transport_setters_move_transports_between_empty_and_loaded() {
    let mut fleet = Fleet::default();
    fleet.set_troop_transport_count(5);
    assert_eq!(
        (
            fleet.empty_transport_count(),
            fleet.loaded_transport_count()
        ),
        (5, 0)
    );

    fleet.set_army_count(3);
    assert_eq!(
        (
            fleet.empty_transport_count(),
            fleet.loaded_transport_count()
        ),
        (2, 3)
    );

    fleet.set_army_count(1);
    assert_eq!(
        (
            fleet.empty_transport_count(),
            fleet.loaded_transport_count()
        ),
        (4, 1)
    );

    // Shrinking the total keeps loaded transports first.
    fleet.set_army_count(3);
    fleet.set_troop_transport_count(4);
    assert_eq!(
        (
            fleet.empty_transport_count(),
            fleet.loaded_transport_count()
        ),
        (1, 3)
    );
    fleet.set_troop_transport_count(2);
    assert_eq!(
        (
            fleet.empty_transport_count(),
            fleet.loaded_transport_count()
        ),
        (0, 2)
    );
}

#[test]
fn max_speed_is_the_slowest_ship_type_present() {
    let cases: [(fn(&mut Fleet), u8); 9] = [
        (|_| {}, 7),
        (|f| f.set_scout_count(1), 6),
        (|f| f.set_destroyer_count(1), 6),
        (|f| f.set_empty_transport_count(1), 5),
        (|f| f.set_loaded_transport_count(1), 5),
        (|f| f.set_cruiser_count(1), 5),
        (|f| f.set_battleship_count(1), 4),
        (|f| f.set_etac_count(1), 3),
        (
            |f| {
                f.set_scout_count(1);
                f.set_battleship_count(1);
                f.set_loaded_transport_count(1);
            },
            4,
        ),
    ];
    for (idx, (compose, expected)) in cases.into_iter().enumerate() {
        let mut fleet = Fleet::default();
        fleet.set_current_speed(7);
        compose(&mut fleet);
        fleet.recompute_max_speed_from_composition();
        assert_eq!(fleet.max_speed(), expected, "case {idx}");
        assert_eq!(fleet.current_speed(), expected, "case {idx}");
    }
}

/// Appends empire-4 filler fleets until the next record would get id `count`.
fn pad_fleets_to(game_data: &mut CoreGameData, count: usize) {
    let template = game_data.fleets[15].clone();
    while game_data.fleets.len() + 1 < count {
        let mut filler = template.clone();
        let id = (game_data.fleets.len() + 1) as u16;
        filler.set_fleet_id_word_raw(id);
        filler.set_local_slot_word_raw(id);
        filler.set_previous_fleet_id(0);
        filler.set_next_fleet_link_word_raw(0);
        game_data.fleets.push(filler);
    }
}

#[test]
fn join_merges_into_a_host_whose_number_exceeds_one_byte() {
    // 0x23 is the host's fleet number (its 0x00 word), stored as a word; the
    // host's record id (300) differs from its number (299).
    let mut game_data = load_fixture("ecmaint-post");
    game_data.players[0].set_player_mode_raw(0x00);
    pad_fleets_to(&mut game_data, 300);

    let mut host = game_data.fleets[0].clone();
    host.set_fleet_id_word_raw(300);
    host.set_local_slot_word_raw(299);
    host.set_previous_fleet_id(0);
    host.set_next_fleet_link_word_raw(0);
    host.set_standing_order_kind(Order::HoldPosition);
    host.set_battleship_count(2);
    game_data.fleets.push(host);
    assert_eq!(game_data.fleets.len(), 300);

    let joiner = &mut game_data.fleets[1];
    joiner.set_standing_order_kind(Order::JoinAnotherFleet);
    joiner.set_mission_target(1, 299);
    assert_eq!(&joiner.to_bytes()[0x23..0x25], &[0x2b, 0x01]);
    let joiner_cruisers = joiner.cruiser_count();

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
        .find(|fleet| fleet.owner_empire_raw() == 1 && fleet.local_slot_word_raw() == 299)
        .expect("host survives");
    assert_eq!(host.battleship_count(), 2);
    assert_eq!(host.cruiser_count(), 1 + joiner_cruisers);
}

#[test]
fn fleet_chain_links_are_words() {
    // t038: ECGAME and ECMAINT read and write 0x03, 0x05, and 0x07 as words.
    let mut bytes = [0u8; 54];
    bytes[0x03..0x09].copy_from_slice(&[0x2c, 0x01, 0x2d, 0x01, 0x2b, 0x01]);
    let mut fleet = Fleet::from_raw(bytes);
    assert_eq!(fleet.next_fleet_id(), 300);
    assert_eq!(fleet.fleet_id(), 301);
    assert_eq!(fleet.previous_fleet_id(), 299);
    fleet.set_previous_fleet_id(0x0203);
    assert_eq!(&fleet.to_bytes()[0x07..0x09], &[0x03, 0x02]);

    // The invade pre-fixture's byte 0x08 (an old tool's "invasion armies")
    // is the previous-fleet word's high byte.
    let fixture = load_fixture("ecmaint-invade-pre");
    assert_eq!(fixture.fleets[2].previous_fleet_id(), 0x6402);
}

#[test]
fn detached_fleets_link_to_predecessors_above_one_byte() {
    let mut game_data = load_fixture("ecmaint-post");
    pad_fleets_to(&mut game_data, 300);
    let one_destroyer = FleetDetachSelection {
        destroyers: 1,
        ..FleetDetachSelection::default()
    };
    game_data.fleets[2].set_destroyer_count(3);
    let first = game_data
        .detach_ships_to_new_fleet(1, 3, one_destroyer, None, 6)
        .expect("first detach");
    let second = game_data
        .detach_ships_to_new_fleet(1, 3, one_destroyer, None, 6)
        .expect("second detach");
    let first = &game_data.fleets[first.new_fleet_record_index_1_based - 1];
    let second = &game_data.fleets[second.new_fleet_record_index_1_based - 1];
    assert_eq!(first.fleet_id(), 300);
    assert_eq!(second.fleet_id(), 301);
    assert_eq!(second.previous_fleet_id(), 300);
    assert_eq!(first.next_fleet_id(), 301);
}
