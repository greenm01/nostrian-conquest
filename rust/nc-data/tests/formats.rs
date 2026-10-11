mod common;

use nc_compat::classic::{ClassicFile, INITIALIZED_FLEET_RECORD_COUNT, without_ignored_bytes};
use nc_compat::{load_classic_core_files, save_classic_core_files};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use common::*;
use nc_compat::classic::{
    encode_planet, parse_conquest_dat, parse_fleets_dat, parse_planets_dat, parse_player_dat,
    parse_setup_dat, serialize_conquest_dat, serialize_fleets_dat, serialize_planets_dat,
    serialize_player_dat, serialize_setup_dat,
};
use nc_data::*;
use nc_data::{
    has_any_force, is_named_homeworld_seed, order_summary, ownership_summary, set_armies_aboard,
    set_troop_transports, ship_composition_table_summary, stardock_slot_kind,
};

#[test]
fn round_trip_player_dat() {
    let bytes = read_fixture("PLAYER.DAT");
    let parsed = parse_player_dat(&bytes).unwrap();
    // Name tails and diplomacy slots past the player count are not carried.
    assert_eq!(
        without_ignored_bytes(ClassicFile::Player, &serialize_player_dat(&parsed)),
        without_ignored_bytes(ClassicFile::Player, &bytes)
    );
}

#[test]
fn round_trip_planets_dat() {
    let bytes = read_fixture("PLANETS.DAT");
    let parsed = parse_planets_dat(&bytes).unwrap();
    // The stale name tails are not carried; everything classic reads is.
    assert_eq!(
        without_ignored_bytes(ClassicFile::Planets, &serialize_planets_dat(&parsed)),
        without_ignored_bytes(ClassicFile::Planets, &bytes)
    );
}

#[test]
fn initialized_planets_expose_named_homeworld_seeds() {
    let bytes = read_initialized_fixture("PLANETS.DAT");
    let parsed = parse_planets_dat(&bytes).unwrap();
    let seeds = parsed
        .iter()
        .enumerate()
        .filter(|(_, record)| is_named_homeworld_seed(record))
        .map(|(idx, record)| {
            (
                idx + 1,
                record.sector.to_array(),
                u16::from(record.potential_production),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        seeds,
        vec![
            (5, [6, 5], 100),
            (6, [13, 5], 100),
            (13, [4, 13], 100),
            (15, [16, 13], 100),
        ]
    );
}

#[test]
fn planet_tail_fields_expose_owner_slot_army_and_battery_fields() {
    let init = parse_planets_dat(&read_initialized_fixture("PLANETS.DAT")).unwrap();
    assert_eq!(init[12].owner.0, 2);
    assert_eq!(init[12].years_under_owner, 2);
    assert_eq!(init[12].armies, 10);
    assert_eq!(init[12].ground_batteries, 4);

    assert_eq!(init[14].owner.0, 1);
    assert_eq!(init[14].years_under_owner, 2);
    assert_eq!(init[14].armies, 10);
    assert_eq!(init[14].ground_batteries, 4);

    let fleet_post = parse_planets_dat(&read_ecmaint_fleet_post_fixture("PLANETS.DAT")).unwrap();
    assert_eq!(fleet_post[13].owner.0, 1);
    assert_eq!(fleet_post[13].years_under_owner, 2);
    assert_eq!(fleet_post[13].armies, 1);
    assert_eq!(fleet_post[13].ground_batteries, 0);
}

#[test]
fn planet_stardock_accessors_cover_eight_typed_u32_slots() {
    let mut planet = Planet::default();
    planet.stardock.cruisers = 1;
    planet.stardock.ipbms = 300;

    assert_eq!(planet.stardock.cruisers, 1);
    assert_eq!(stardock_slot_kind(&planet, 1).code(), 2);
    assert_eq!(planet.stardock.ipbms, 300);
    let bytes = encode_planet(&planet);
    assert_eq!(bytes[0x3C], 1);
    assert_eq!(bytes[0x54], 44);
    assert_eq!(bytes[0x55], 1);
}

#[test]
fn round_trip_setup_dat() {
    let bytes = read_fixture("SETUP.DAT");
    let parsed = parse_setup_dat(&bytes).unwrap();
    assert_eq!(serialize_setup_dat(&parsed), bytes);
    assert_eq!(&parsed.version_tag, b"EC151");
    assert_eq!(parsed.com_irq, [4, 3, 4, 3]);
    assert_eq!(parsed.com_hw_flow_control, [true; 4]);
    assert_eq!(parsed.com_irq.first().copied(), Some(4));
    assert_eq!(parsed.com_irq.get(1).copied(), Some(3));
    assert_eq!(parsed.com_irq.get(2).copied(), Some(4));
    assert_eq!(parsed.com_irq.get(3).copied(), Some(3));
    assert_eq!(parsed.com_hw_flow_control.first().copied(), Some(true));
    assert_eq!(parsed.com_hw_flow_control.get(1).copied(), Some(true));
    assert_eq!(parsed.com_hw_flow_control.get(2).copied(), Some(true));
    assert_eq!(parsed.com_hw_flow_control.get(3).copied(), Some(true));
    assert!(parsed.snoop);
    assert_eq!(parsed.max_key_gap_minutes, 10);
    assert!(parsed.remote_timeout);
    assert!(!parsed.local_timeout);
    assert_eq!(parsed.minimum_time_minutes, 0);
    assert_eq!(parsed.purge_after_turns, 0);
    assert_eq!(parsed.autopilot_after_turns, 0);
}

#[test]
fn round_trip_conquest_dat() {
    let bytes = read_fixture("CONQUEST.DAT");
    let parsed = parse_conquest_dat(&bytes).unwrap();
    // Ranking slots past the player count hold ECMAINT's garbage; the
    // model does not carry them.
    assert_eq!(
        without_ignored_bytes(ClassicFile::Conquest, &serialize_conquest_dat(&parsed)),
        without_ignored_bytes(ClassicFile::Conquest, &bytes)
    );
    assert_eq!(parsed.game_year, 3022);
    assert_eq!(parsed.player_count, 4);
    assert_eq!(parsed.previous_production[..4], [100; 4]);
    assert_eq!(parsed.previous_planet_count[..4], [1; 4]);
}

#[test]
fn player_tax_rate_matches_current_notes() {
    let bytes = read_fixture("PLAYER.DAT");
    let parsed = parse_player_dat(&bytes).unwrap();
    assert_eq!(parsed[0].tax_rate, 65);
}

#[test]
fn f3_owner_fixture_exposes_rogue_and_player_controlled_empire_summaries() {
    let bytes = read_f3_owner_fixture("PLAYER.DAT");
    let parsed = parse_player_dat(&bytes).unwrap();

    assert_eq!(parsed[0].mode.code(), 0xff);
    assert_eq!(parsed[0].empire_name.chars().count() as u8, 6);
    assert_eq!(parsed[0].empire_name.trim().to_string(), "Rogues");
    assert_eq!(ownership_summary(&parsed[0]), "rogue label='Rogues'");

    assert_eq!(parsed[1].mode.code(), 1);
    assert_eq!(parsed[1].handle.trim().to_string(), "FOO");
    assert_eq!(parsed[1].empire_name.chars().count() as u8, 3);
    assert_eq!(parsed[1].empire_name.trim().to_string(), "foo");
    assert_eq!(
        ownership_summary(&parsed[1]),
        "player handle='FOO' empire='foo'"
    );
}

#[test]
fn shipped_fleets_dat_uses_a_variable_record_count() {
    let bytes = read_fixture("FLEETS.DAT");
    let parsed = parse_fleets_dat(&bytes).unwrap();
    assert_eq!(parsed.len(), 13);
}

#[test]
fn round_trip_initialized_fleets_dat() {
    let bytes = read_initialized_fixture("FLEETS.DAT");
    let parsed = parse_fleets_dat(&bytes).unwrap();
    assert_eq!(serialize_fleets_dat(&parsed), bytes);
    assert_eq!(parsed.len(), INITIALIZED_FLEET_RECORD_COUNT);
    assert_eq!(parsed[0].record.0, 1);
    assert_eq!(parsed[0].fleet_number.0, 1);
    assert_eq!(parsed[0].next.0, 2);
    assert_eq!(parsed[0].prev.0, 0);
    assert_eq!(parsed[0].max_speed, 3);
    assert_eq!(parsed[0].rules_of_engagement, 6);
    assert_eq!(parsed[0].ships.cruisers, 1);
    assert_eq!(parsed[0].ships.destroyers, 0);
    assert_eq!(parsed[0].ships.etacs, 1);
    assert_eq!(parsed[0].order.code(), 5);
    assert_eq!(parsed[0].order, Order::GuardBlockadeWorld);
    assert_eq!(parsed[0].target.to_array(), [16, 13]);
    assert_eq!(
        order_summary(&parsed[0]),
        "Guard/blockade world in System (16,13)"
    );
    assert_eq!(ship_composition_table_summary(&parsed[0]), "CA1 ETAC1");

    assert_eq!(parsed[2].record.0, 3);
    assert_eq!(parsed[2].fleet_number.0, 3);
    assert_eq!(parsed[2].next.0, 4);
    assert_eq!(parsed[2].prev.0, 2);
    assert_eq!(parsed[2].max_speed, 6);
    assert_eq!(parsed[2].rules_of_engagement, 6);
    assert_eq!(parsed[2].ships.cruisers, 0);
    assert_eq!(parsed[2].ships.destroyers, 1);
    assert_eq!(parsed[2].ships.etacs, 0);
    assert_eq!(parsed[2].order.code(), 5);
    assert_eq!(parsed[2].order, Order::GuardBlockadeWorld);
    assert_eq!(parsed[2].target.to_array(), [16, 13]);
    assert_eq!(
        order_summary(&parsed[2]),
        "Guard/blockade world in System (16,13)"
    );
    assert_eq!(ship_composition_table_summary(&parsed[2]), "DD1");
}

#[test]
fn fleet_has_any_force_tracks_ship_transport_army_and_etac_presence() {
    let mut record = Fleet::default();
    assert!(!has_any_force(&record));

    set_armies_aboard(&mut record, 1);
    assert!(has_any_force(&record));
    set_armies_aboard(&mut record, 0);

    set_troop_transports(&mut record, 1);
    assert!(has_any_force(&record));
    set_troop_transports(&mut record, 0);

    record.ships.etacs = 1;
    assert!(has_any_force(&record));
}

#[test]
#[should_panic(expected = "empty fleet record is not a fleet")]
fn empty_fleet_record_panics_when_rendering_ship_composition() {
    let mut record = parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap()[0].clone();
    record.ships.scouts = 0;
    record.ships.battleships = 0;
    record.ships.cruisers = 0;
    record.ships.destroyers = 0;
    set_troop_transports(&mut record, 0);
    set_armies_aboard(&mut record, 0);
    record.ships.etacs = 0;
    let _ = ship_composition_table_summary(&record);
}

#[test]
fn fleet_table_summary_uses_tt_star_for_loaded_transports_only() {
    let mut record = parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap()[0].clone();
    record.ships.scouts = 0;
    record.ships.battleships = 0;
    record.ships.cruisers = 0;
    record.ships.destroyers = 0;
    record.ships.etacs = 0;
    set_troop_transports(&mut record, 2);
    set_armies_aboard(&mut record, 0);
    assert_eq!(ship_composition_table_summary(&record), "TT2");

    set_armies_aboard(&mut record, 2);
    assert_eq!(ship_composition_table_summary(&record), "TT*2");
    assert!(!ship_composition_table_summary(&record).contains("TT2"));
}

#[test]
fn fleet_list_summary_always_shows_the_total_even_for_one_ship() {
    let mut record = parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap()[0].clone();
    record.ships.scouts = 2;
    record.ships.battleships = 1;
    record.ships.cruisers = 0;
    record.ships.destroyers = 4;
    set_troop_transports(&mut record, 0);
    set_armies_aboard(&mut record, 0);
    record.ships.etacs = 1;

    assert_eq!(ship_composition_table_summary(&record), "SC2 BB1 DD4 ETAC1");
}

#[test]
fn fleet_list_summary_splits_loaded_and_empty_transports() {
    let mut record = parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap()[0].clone();
    record.ships.scouts = 0;
    record.ships.battleships = 0;
    record.ships.cruisers = 0;
    record.ships.destroyers = 0;
    set_troop_transports(&mut record, 5);
    set_armies_aboard(&mut record, 2);
    record.ships.etacs = 0;

    assert_eq!(ship_composition_table_summary(&record), "TT*2 TT3");
}

#[test]
fn post_maintenance_matches_init_for_core_state_but_not_global_summaries() {
    assert_eq!(
        read_initialized_fixture("PLAYER.DAT"),
        read_post_maint_fixture("PLAYER.DAT")
    );
    assert_eq!(
        read_initialized_fixture("PLANETS.DAT"),
        read_post_maint_fixture("PLANETS.DAT")
    );
    assert_eq!(
        read_initialized_fixture("FLEETS.DAT"),
        read_post_maint_fixture("FLEETS.DAT")
    );
    assert_eq!(
        read_initialized_fixture("SETUP.DAT"),
        read_post_maint_fixture("SETUP.DAT")
    );

    assert_ne!(
        read_initialized_fixture("CONQUEST.DAT"),
        read_post_maint_fixture("CONQUEST.DAT")
    );
    assert_ne!(
        read_initialized_fixture("DATABASE.DAT"),
        read_post_maint_fixture("DATABASE.DAT")
    );
}

#[test]
fn preserved_conquest_year_progression_matches_docs() {
    let original = parse_conquest_dat(&read_fixture("CONQUEST.DAT")).unwrap();
    let initialized = parse_conquest_dat(&read_initialized_fixture("CONQUEST.DAT")).unwrap();
    let post_maint = parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap();

    assert_eq!(initialized.game_year, 3000);
    assert_eq!(post_maint.game_year, 3001);
    assert_eq!(original.game_year, 3022);
    assert_eq!(initialized.player_count, 4);
    assert_eq!(post_maint.player_count, 4);
    assert_eq!(original.player_count, 4);
}

#[test]
fn post_maintenance_fixture_exposes_known_schedule_bytes() {
    let post_maint = parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap();
    assert_eq!(post_maint.maintenance_days.0.map(u8::from), [0x01; 7]);
}

#[test]
fn core_game_data_round_trips_post_maintenance_directory() {
    let source_dir = repo_root().join("fixtures/ecmaint-post/v1.5");
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("nc-data-core-game-{unique}"));
    fs::create_dir_all(&temp_dir).unwrap();

    for name in [
        "PLAYER.DAT",
        "PLANETS.DAT",
        "FLEETS.DAT",
        "BASES.DAT",
        "IPBM.DAT",
        "SETUP.DAT",
        "CONQUEST.DAT",
    ] {
        fs::copy(source_dir.join(name), temp_dir.join(name)).unwrap();
    }

    let parsed = load_classic_core_files(&temp_dir).unwrap();
    save_classic_core_files(&temp_dir, &parsed).unwrap();

    for name in [
        "PLAYER.DAT",
        "PLANETS.DAT",
        "FLEETS.DAT",
        "BASES.DAT",
        "IPBM.DAT",
        "SETUP.DAT",
        "CONQUEST.DAT",
    ] {
        let file = ClassicFile::from_file_name(name).expect("a core classic file");
        // Bytes no classic program reads are not carried.
        assert_eq!(
            without_ignored_bytes(file, &fs::read(temp_dir.join(name)).unwrap()),
            without_ignored_bytes(file, &fs::read(source_dir.join(name)).unwrap()),
            "{name} should round-trip through CoreGameData unchanged"
        );
    }

    fs::remove_dir_all(temp_dir).unwrap();
}
