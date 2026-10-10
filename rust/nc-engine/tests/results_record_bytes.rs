use nc_data::{
    BombardEvent, ContactReportSource, GameStateBuilder, MaintenanceEvents, Mission, PlanetRecord,
    ScoutContactEvent, ShipLosses,
};
use nc_engine::{build_results_dat, build_results_report_blocks};

const RECORD_SIZE: usize = 84;
const TEXT_SIZE: usize = 72;

fn seeded_game_data() -> nc_data::CoreGameData {
    GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3031)
        .build_initialized_baseline()
        .expect("baseline should build")
}

fn seed_world_with_raw_name(game_data: &mut nc_data::CoreGameData, name: &[u8]) {
    let mut planet = PlanetRecord::new_zeroed();
    planet.set_coords_raw([9, 6]);
    let mut buffer = [0u8; 13];
    buffer[..name.len()].copy_from_slice(name);
    planet.set_planet_name_buffer(name.len() as u8, &buffer);
    planet.set_owner_empire_slot_raw(1);
    planet.set_ownership_status_raw(2);
    game_data.planets.records[0] = planet;
}

fn defender_bombard_events() -> MaintenanceEvents {
    let mut events = MaintenanceEvents::default();
    events.bombard_events.push(BombardEvent {
        planet_idx: 0,
        attacker_empire_raw: 2,
        attacker_fleet_number: Some(10),
        defender_empire_raw: 1,
        attacker_initial: ShipLosses {
            battleships: 26,
            cruisers: 58,
            transports: 68,
            ..ShipLosses::default()
        },
        attacker_loaded_armies_initial: 0,
        defender_batteries_initial: 0,
        defender_armies_initial: 3,
        attacker_losses: ShipLosses::default(),
        defender_battery_losses: 0,
        defender_army_losses: 3,
        breakthrough: true,
        docked_losses: nc_data::EmpireUnitSummary::default(),
        stardock_items_destroyed: 7,
        stored_goods_destroyed: 42,
        factories_destroyed: 10497,
        stardate_week: Some(3),
    });
    events
}

fn patrol_contact_events() -> MaintenanceEvents {
    let mut events = MaintenanceEvents::default();
    events.scout_contact_events.push(ScoutContactEvent {
        viewer_empire_raw: 1,
        source: ContactReportSource::FleetMission(Mission::PatrolSector),
        reporting_fleet_number: Some(13),
        reporting_initial: ShipLosses {
            destroyers: 1,
            ..ShipLosses::default()
        },
        reporting_loaded_armies_initial: 0,
        coords: [24, 14],
        target_empire_raw: 2,
        target_fleet_number: Some(5),
        small_vessels: 2,
        medium_vessels: 0,
        large_vessels: 0,
        stardate_week: Some(52),
    });
    events
}

/// Every record must carry a Pascal `String[72]` length byte <= 72 and only
/// printable ASCII in its declared text.
fn assert_classic_records(bytes: &[u8]) -> Vec<String> {
    assert!(!bytes.is_empty(), "expected RESULTS.DAT records");
    assert_eq!(bytes.len() % RECORD_SIZE, 0);
    bytes
        .chunks_exact(RECORD_SIZE)
        .map(|record| {
            let len = record[1] as usize;
            assert!(len <= TEXT_SIZE, "record length byte {len} exceeds 72");
            let text = &record[2..2 + len];
            assert!(
                text.iter().all(|b| *b == b' ' || b.is_ascii_graphic()),
                "record text is not printable ASCII: {text:?}"
            );
            String::from_utf8(text.to_vec()).unwrap()
        })
        .collect()
}

#[test]
fn utf8_planet_name_report_stays_within_classic_record_bytes() {
    let mut game_data = seeded_game_data();
    seed_world_with_raw_name(&mut game_data, "ÆÆÆÆÆÆ".as_bytes());

    let lines = assert_classic_records(&build_results_dat(&game_data, &defender_bombard_events()));
    assert!(lines[0].starts_with("From planet \"??????\" in System(9,6):  "));
    assert!(lines[0].ends_with("  Stardate: 03/3031"));
    assert_eq!(lines[0].len(), TEXT_SIZE);
}

#[test]
fn classic_high_byte_planet_name_report_does_not_panic() {
    let mut game_data = seeded_game_data();
    seed_world_with_raw_name(&mut game_data, &[0x92; 13]);

    let lines = assert_classic_records(&build_results_dat(&game_data, &defender_bombard_events()));
    assert!(lines[0].starts_with("From planet \"?"));
    assert!(lines[0].ends_with("Stardate: 03/3031"));
    assert_eq!(lines[0].len(), TEXT_SIZE);
}

#[test]
fn decoded_report_text_matches_classic_record_bytes() {
    let mut game_data = seeded_game_data();
    seed_world_with_raw_name(&mut game_data, "ÆÆÆÆÆÆ".as_bytes());
    let events = defender_bombard_events();

    let lines = assert_classic_records(&build_results_dat(&game_data, &events));
    let rows = build_results_report_blocks(&game_data, &events);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].decoded_text, lines.join("\n"));
}

#[test]
fn patrol_contact_report_uses_ascii_dash_within_record_bytes() {
    let game_data = seeded_game_data();

    let lines = assert_classic_records(&build_results_dat(&game_data, &patrol_contact_events()));
    let text = lines.join(" ");
    assert!(
        text.contains("Patrol mission report: Sensor contact -- detected and identified"),
        "{text}"
    );
}

#[test]
fn structured_report_rows_keep_label_column_alignment() {
    let mut game_data = seeded_game_data();
    seed_world_with_raw_name(&mut game_data, b"biggy");

    let lines = assert_classic_records(&build_results_dat(&game_data, &defender_bombard_events()));
    let attacker = lines
        .iter()
        .find(|line| line.starts_with("Attacker:"))
        .expect("attacker row");
    assert_eq!(&attacker[..18], "Attacker:         ");
    let force_idx = lines
        .iter()
        .position(|line| line.starts_with("Attacking force:"))
        .expect("attacking force row");
    assert_eq!(&lines[force_idx][..18], "Attacking force:  ");
    assert!(lines[force_idx][18..].starts_with(|ch: char| ch.is_ascii_digit()));
}
