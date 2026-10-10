mod common;

use common::*;
use nc_data::*;

#[test]
fn ecmaint_build_scenario_consumes_queue_and_changes_planet_state() {
    let pre_bytes = read_ecmaint_build_pre_fixture("PLANETS.DAT");
    let post_bytes = read_ecmaint_build_post_fixture("PLANETS.DAT");
    let post = PlanetDat::parse(&post_bytes).unwrap();

    let post_record = &post.records[14];

    // Classic queues ten u16 counts by kind (destroyers at 0x24, ETACs at
    // 0x2E). The model holds Rust's build slots until t056 translates, so
    // read classic's words from the file.
    let queue_word = |bytes: &[u8], kind: usize| {
        let at = 14 * PLANET_RECORD_SIZE + 0x22 + 2 * kind;
        u16::from_le_bytes([bytes[at], bytes[at + 1]])
    };
    assert_eq!(queue_word(&pre_bytes, 1), 3);
    assert_eq!(queue_word(&pre_bytes, 6), 1);
    assert!((1..=10).all(|kind| queue_word(&post_bytes, kind) == 0));
    assert_eq!(post_record.planet.stardock.destroyers, 3);
    assert_eq!(post_record.planet.stardock.etacs, 1);
}

#[test]
fn ecmaint_fleet_scenario_consumes_order_and_updates_fleet_and_planet_state() {
    let pre_fleets = FleetDat::parse(&read_ecmaint_fleet_pre_fixture("FLEETS.DAT")).unwrap();
    let post_fleets = FleetDat::parse(&read_ecmaint_fleet_post_fixture("FLEETS.DAT")).unwrap();
    let post_planets = PlanetDat::parse(&read_ecmaint_fleet_post_fixture("PLANETS.DAT")).unwrap();

    let pre_fleet = &pre_fleets.records[0];
    let post_fleet = &post_fleets.records[0];

    assert_eq!(pre_fleet.current_speed(), 3);
    assert_eq!(pre_fleet.standing_order_kind(), Order::ColonizeWorld);
    assert_eq!(pre_fleet.standing_order_target_coords_raw()[0], 0x0f);

    assert_eq!(post_fleet.current_location_coords_raw()[0], 0x0f);
    assert_eq!(
        post_fleet.depth().to_bytes(),
        [0x80, 0xb9, 0xff, 0xff, 0xff, 0x7f]
    );
    assert_eq!(post_fleet.standing_order_kind(), Order::HoldPosition);
    assert_eq!(post_fleet.standing_order_target_coords_raw()[0], 0x0f);

    let post_planet = &post_planets.records[13];
    assert_eq!(post_planet.army_count_raw(), 1);
    assert_eq!(post_planet.ownership_status_raw(), 2);
    assert_eq!(post_planet.owner_empire_slot_raw(), 1);
}

#[test]
fn ecmaint_bombard_scenario_arrival_preserves_attack_order() {
    let pre = FleetDat::parse(&read_ecmaint_bombard_pre_fixture("FLEETS.DAT")).unwrap();
    let arrive = FleetDat::parse(&read_ecmaint_bombard_arrive_fixture("FLEETS.DAT")).unwrap();

    let pre_fleet = &pre.records[2];
    let arrive_fleet = &arrive.records[2];

    assert_eq!(pre_fleet.current_speed(), 0x03);
    assert_eq!(pre_fleet.standing_order_code_raw(), 0x06);
    assert_eq!(pre_fleet.standing_order_target_coords_raw(), [0x0f, 0x0d]);

    assert_eq!(arrive_fleet.current_speed(), 0x03);
    assert_eq!(arrive_fleet.current_location_coords_raw(), [0x0f, 0x0d]);
    assert_eq!(arrive_fleet.standing_order_code_raw(), 0x06);
    assert_eq!(
        arrive_fleet.standing_order_target_coords_raw(),
        [0x0f, 0x0d]
    );
}

// These preserved-fixture checks document classic ECMAINT outputs.
// They are oracle evidence, not the canonical Rust bombardment rule.
#[test]
fn ecmaint_bombard_scenario_second_pass_consumes_order_and_kills_attackers() {
    let arrive = FleetDat::parse(&read_ecmaint_bombard_arrive_fixture("FLEETS.DAT")).unwrap();
    let post = FleetDat::parse(&read_ecmaint_bombard_post_fixture("FLEETS.DAT")).unwrap();

    let arrive_fleet = &arrive.records[2];
    let post_fleet = &post.records[2];

    assert_eq!(arrive_fleet.current_speed(), 0x03);
    assert_eq!(arrive_fleet.standing_order_code_raw(), 0x06);
    assert_eq!(arrive_fleet.cruiser_count(), 0x03);
    assert_eq!(arrive_fleet.destroyer_count(), 0x05);

    assert_eq!(post_fleet.current_speed(), 0x00);
    assert_eq!(post_fleet.standing_order_code_raw(), 0x00);
    assert_eq!(post_fleet.current_location_coords_raw(), [0x0f, 0x0d]);
    assert_eq!(post_fleet.cruiser_count(), 0x02);
    assert_eq!(post_fleet.destroyer_count(), 0x01);

    let arrive_planets =
        PlanetDat::parse(&read_ecmaint_bombard_arrive_fixture("PLANETS.DAT")).unwrap();
    let post_planets = PlanetDat::parse(&read_ecmaint_bombard_post_fixture("PLANETS.DAT")).unwrap();
    assert_eq!(arrive_planets.records[13], post_planets.records[13]);
}

#[test]
fn ecmaint_bombard_zero_army_target_changes_planet_without_attacker_losses() {
    let pre = FleetDat::parse(&read_ecmaint_bombard_army0_pre_fixture("FLEETS.DAT")).unwrap();
    let post = FleetDat::parse(&read_ecmaint_bombard_army0_post_fixture("FLEETS.DAT")).unwrap();

    let pre_fleet = &pre.records[2];
    let post_fleet = &post.records[2];

    assert_eq!(pre_fleet.standing_order_code_raw(), 0x06);
    assert_eq!(pre_fleet.cruiser_count(), 0x03);
    assert_eq!(pre_fleet.destroyer_count(), 0x05);

    assert_eq!(post_fleet.current_speed(), 0x00);
    assert_eq!(post_fleet.standing_order_code_raw(), 0x00);
    assert_eq!(post_fleet.current_location_coords_raw(), [0x0f, 0x0d]);
    assert_eq!(post_fleet.cruiser_count(), 0x03);
    assert_eq!(post_fleet.destroyer_count(), 0x05);

    let pre_planets =
        PlanetDat::parse(&read_ecmaint_bombard_army0_pre_fixture("PLANETS.DAT")).unwrap();
    let post_planets =
        PlanetDat::parse(&read_ecmaint_bombard_army0_post_fixture("PLANETS.DAT")).unwrap();
    let pre_target = &pre_planets.records[13];
    let post_target = &post_planets.records[13];

    assert_eq!(pre_target.ground_batteries_raw(), 0);
    assert_eq!(post_target.ground_batteries_raw(), 0);
    assert_eq!(pre_target.owner_empire_slot_raw(), 2);
    assert_eq!(post_target.owner_empire_slot_raw(), 2);
    assert_eq!(pre_target.army_count_raw(), 0x8e);
    assert_eq!(post_target.army_count_raw(), 0x8a);
    assert_ne!(pre_target, post_target);

    assert_eq!(
        read_ecmaint_bombard_army0_post_fixture("MESSAGES.DAT"),
        Vec::<u8>::new()
    );
    assert_eq!(
        read_ecmaint_bombard_army0_post_fixture("RESULTS.DAT"),
        Vec::<u8>::new()
    );
}

#[test]
fn ecmaint_bombard_zero_army_zero_dev_target_changes_damage_pattern() {
    let pre = FleetDat::parse(&read_ecmaint_bombard_army0_dev0_pre_fixture("FLEETS.DAT")).unwrap();
    let post =
        FleetDat::parse(&read_ecmaint_bombard_army0_dev0_post_fixture("FLEETS.DAT")).unwrap();

    let pre_fleet = &pre.records[2];
    let post_fleet = &post.records[2];

    assert_eq!(pre_fleet.standing_order_code_raw(), 0x06);
    assert_eq!(post_fleet.standing_order_code_raw(), 0x00);
    assert_eq!(post_fleet.current_speed(), 0x00);
    assert_eq!(post_fleet.current_location_coords_raw(), [0x0f, 0x0d]);
    assert_eq!(post_fleet.cruiser_count(), 0x03);
    assert_eq!(post_fleet.destroyer_count(), 0x05);

    let pre_planets =
        PlanetDat::parse(&read_ecmaint_bombard_army0_dev0_pre_fixture("PLANETS.DAT")).unwrap();
    let post_planets =
        PlanetDat::parse(&read_ecmaint_bombard_army0_dev0_post_fixture("PLANETS.DAT")).unwrap();
    let pre_target = &pre_planets.records[13];
    let post_target = &post_planets.records[13];

    assert_eq!(pre_target.army_count_raw(), 0x00);
    assert_eq!(post_target.army_count_raw(), 0x00);
    assert_eq!(pre_target.ground_batteries_raw(), 0x00);
    assert_eq!(post_target.ground_batteries_raw(), 0x00);
    assert_eq!(pre_target.owner_empire_slot_raw(), 2);
    assert_eq!(post_target.owner_empire_slot_raw(), 2);
    assert_ne!(pre_target, post_target);

    assert_eq!(
        read_ecmaint_bombard_army0_dev0_post_fixture("MESSAGES.DAT"),
        Vec::<u8>::new()
    );
    assert_eq!(
        read_ecmaint_bombard_army0_dev0_post_fixture("RESULTS.DAT"),
        Vec::<u8>::new()
    );
}

#[test]
fn ecmaint_bombard_one_army_target_causes_partial_attacker_losses() {
    let pre = FleetDat::parse(&read_ecmaint_bombard_army1_pre_fixture("FLEETS.DAT")).unwrap();
    let post = FleetDat::parse(&read_ecmaint_bombard_army1_post_fixture("FLEETS.DAT")).unwrap();

    let pre_fleet = &pre.records[2];
    let post_fleet = &post.records[2];

    assert_eq!(pre_fleet.standing_order_code_raw(), 0x06);
    assert_eq!(pre_fleet.cruiser_count(), 0x03);
    assert_eq!(pre_fleet.destroyer_count(), 0x05);

    assert_eq!(post_fleet.current_speed(), 0x00);
    assert_eq!(post_fleet.standing_order_code_raw(), 0x00);
    assert_eq!(post_fleet.current_location_coords_raw(), [0x0f, 0x0d]);
    assert_eq!(post_fleet.cruiser_count(), 0x02);
    assert_eq!(post_fleet.destroyer_count(), 0x02);

    let pre_planets =
        PlanetDat::parse(&read_ecmaint_bombard_army1_pre_fixture("PLANETS.DAT")).unwrap();
    let post_planets =
        PlanetDat::parse(&read_ecmaint_bombard_army1_post_fixture("PLANETS.DAT")).unwrap();
    let pre_target = &pre_planets.records[13];
    let post_target = &post_planets.records[13];

    assert_eq!(pre_target.ground_batteries_raw(), 1);
    assert_eq!(post_target.ground_batteries_raw(), 0);
    assert_eq!(pre_target.owner_empire_slot_raw(), 2);
    assert_eq!(post_target.owner_empire_slot_raw(), 2);
    assert_eq!(pre_target.army_count_raw(), 0x8e);
    assert_eq!(post_target.army_count_raw(), 0x8d);
    assert_ne!(pre_target, post_target);

    assert_eq!(
        read_ecmaint_bombard_army1_post_fixture("MESSAGES.DAT"),
        Vec::<u8>::new()
    );
    assert_eq!(
        read_ecmaint_bombard_army1_post_fixture("RESULTS.DAT"),
        Vec::<u8>::new()
    );
}

#[test]
fn ecmaint_bombard_one_army_zero_dev_target_changes_loss_profile() {
    let pre = FleetDat::parse(&read_ecmaint_bombard_army1_dev0_pre_fixture("FLEETS.DAT")).unwrap();
    let post =
        FleetDat::parse(&read_ecmaint_bombard_army1_dev0_post_fixture("FLEETS.DAT")).unwrap();

    let pre_fleet = &pre.records[2];
    let post_fleet = &post.records[2];

    assert_eq!(pre_fleet.standing_order_code_raw(), 0x06);
    assert_eq!(pre_fleet.cruiser_count(), 0x03);
    assert_eq!(pre_fleet.destroyer_count(), 0x05);

    assert_eq!(post_fleet.current_speed(), 0x00);
    assert_eq!(post_fleet.standing_order_code_raw(), 0x00);
    assert_eq!(post_fleet.current_location_coords_raw(), [0x0f, 0x0d]);
    assert_eq!(post_fleet.cruiser_count(), 0x02);
    assert_eq!(post_fleet.destroyer_count(), 0x04);

    let pre_planets =
        PlanetDat::parse(&read_ecmaint_bombard_army1_dev0_pre_fixture("PLANETS.DAT")).unwrap();
    let post_planets =
        PlanetDat::parse(&read_ecmaint_bombard_army1_dev0_post_fixture("PLANETS.DAT")).unwrap();
    let pre_target = &pre_planets.records[13];
    let post_target = &post_planets.records[13];

    assert_eq!(pre_target.ground_batteries_raw(), 1);
    assert_eq!(post_target.ground_batteries_raw(), 0);
    assert_eq!(pre_target.army_count_raw(), 0x00);
    assert_eq!(post_target.army_count_raw(), 0x00);
    assert_eq!(pre_target.owner_empire_slot_raw(), 2);
    assert_eq!(post_target.owner_empire_slot_raw(), 2);
    assert_ne!(pre_target, post_target);

    assert_eq!(
        read_ecmaint_bombard_army1_dev0_post_fixture("MESSAGES.DAT"),
        Vec::<u8>::new()
    );
    assert_eq!(
        read_ecmaint_bombard_army1_dev0_post_fixture("RESULTS.DAT"),
        Vec::<u8>::new()
    );
}

#[test]
fn ecmaint_bombard_byte_0e_increases_defender_damage_profile() {
    let pre = FleetDat::parse(&read_ecmaint_bombard_army1_dev0_e0c_pre_fixture(
        "FLEETS.DAT",
    ))
    .unwrap();
    let post = FleetDat::parse(&read_ecmaint_bombard_army1_dev0_e0c_post_fixture(
        "FLEETS.DAT",
    ))
    .unwrap();

    let pre_fleet = &pre.records[2];
    let post_fleet = &post.records[2];

    assert_eq!(pre_fleet.standing_order_code_raw(), 0x06);
    assert_eq!(pre_fleet.cruiser_count(), 0x03);
    assert_eq!(pre_fleet.destroyer_count(), 0x05);

    assert_eq!(post_fleet.current_speed(), 0x00);
    assert_eq!(post_fleet.standing_order_code_raw(), 0x00);
    assert_eq!(post_fleet.current_location_coords_raw(), [0x0f, 0x0d]);
    assert_eq!(post_fleet.cruiser_count(), 0x03);
    assert_eq!(post_fleet.destroyer_count(), 0x01);

    let pre_planets = PlanetDat::parse(&read_ecmaint_bombard_army1_dev0_e0c_pre_fixture(
        "PLANETS.DAT",
    ))
    .unwrap();
    let post_planets = PlanetDat::parse(&read_ecmaint_bombard_army1_dev0_e0c_post_fixture(
        "PLANETS.DAT",
    ))
    .unwrap();
    let pre_target = &pre_planets.records[13];
    let post_target = &post_planets.records[13];

    assert_eq!(pre_target.planet.stored_goods.to_bytes()[5], 0x0c);
    assert_eq!(pre_target.ground_batteries_raw(), 1);
    assert_eq!(pre_target.army_count_raw(), 0x00);
    assert_eq!(post_target.ground_batteries_raw(), 0);
    assert_eq!(post_target.army_count_raw(), 0x00);
    assert_eq!(post_target.planet.stored_goods.to_bytes()[5], 0x54);
    assert_eq!(post_target.owner_empire_slot_raw(), 2);
    assert_ne!(pre_target, post_target);

    assert_eq!(
        read_ecmaint_bombard_army1_dev0_e0c_post_fixture("MESSAGES.DAT"),
        Vec::<u8>::new()
    );
    assert_eq!(
        read_ecmaint_bombard_army1_dev0_e0c_post_fixture("RESULTS.DAT"),
        Vec::<u8>::new()
    );
}

#[test]
fn ecmaint_bombard_byte_08_changes_defender_loss_profile() {
    let pre = FleetDat::parse(&read_ecmaint_bombard_army1_dev0_b08_pre_fixture(
        "FLEETS.DAT",
    ))
    .unwrap();
    let post = FleetDat::parse(&read_ecmaint_bombard_army1_dev0_b08_post_fixture(
        "FLEETS.DAT",
    ))
    .unwrap();

    let pre_fleet = &pre.records[2];
    let post_fleet = &post.records[2];

    assert_eq!(pre_fleet.standing_order_code_raw(), 0x06);
    assert_eq!(pre_fleet.cruiser_count(), 0x03);
    assert_eq!(pre_fleet.destroyer_count(), 0x05);

    assert_eq!(post_fleet.current_speed(), 0x00);
    assert_eq!(post_fleet.standing_order_code_raw(), 0x00);
    assert_eq!(post_fleet.current_location_coords_raw(), [0x0f, 0x0d]);
    assert_eq!(post_fleet.cruiser_count(), 0x01);
    assert_eq!(post_fleet.destroyer_count(), 0x03);

    let pre_planets = PlanetDat::parse(&read_ecmaint_bombard_army1_dev0_b08_pre_fixture(
        "PLANETS.DAT",
    ))
    .unwrap();
    let post_planets = PlanetDat::parse(&read_ecmaint_bombard_army1_dev0_b08_post_fixture(
        "PLANETS.DAT",
    ))
    .unwrap();
    let pre_target = &pre_planets.records[13];
    let post_target = &post_planets.records[13];

    assert_eq!(pre_target.planet.production.to_bytes()[5], 0x00);
    assert_eq!(pre_target.ground_batteries_raw(), 1);
    assert_eq!(pre_target.army_count_raw(), 0x00);
    assert_eq!(post_target.ground_batteries_raw(), 0);
    assert_eq!(post_target.army_count_raw(), 0x00);
    assert_eq!(post_target.owner_empire_slot_raw(), 2);
    assert_ne!(pre_target, post_target);

    assert_eq!(
        read_ecmaint_bombard_army1_dev0_b08_post_fixture("MESSAGES.DAT"),
        Vec::<u8>::new()
    );
    assert_eq!(
        read_ecmaint_bombard_army1_dev0_b08_post_fixture("RESULTS.DAT"),
        Vec::<u8>::new()
    );
}

#[test]
fn ecmaint_bombard_byte_09_changes_attacker_loss_profile() {
    let pre = FleetDat::parse(&read_ecmaint_bombard_army1_dev0_b09_pre_fixture(
        "FLEETS.DAT",
    ))
    .unwrap();
    let post = FleetDat::parse(&read_ecmaint_bombard_army1_dev0_b09_post_fixture(
        "FLEETS.DAT",
    ))
    .unwrap();

    let pre_fleet = &pre.records[2];
    let post_fleet = &post.records[2];

    assert_eq!(pre_fleet.standing_order_code_raw(), 0x06);
    assert_eq!(pre_fleet.cruiser_count(), 0x03);
    assert_eq!(pre_fleet.destroyer_count(), 0x05);

    assert_eq!(post_fleet.current_speed(), 0x00);
    assert_eq!(post_fleet.standing_order_code_raw(), 0x00);
    assert_eq!(post_fleet.current_location_coords_raw(), [0x0f, 0x0d]);
    assert_eq!(post_fleet.cruiser_count(), 0x01);
    assert_eq!(post_fleet.destroyer_count(), 0x05);

    let pre_planets = PlanetDat::parse(&read_ecmaint_bombard_army1_dev0_b09_pre_fixture(
        "PLANETS.DAT",
    ))
    .unwrap();
    let post_planets = PlanetDat::parse(&read_ecmaint_bombard_army1_dev0_b09_post_fixture(
        "PLANETS.DAT",
    ))
    .unwrap();
    let pre_target = &pre_planets.records[13];
    let post_target = &post_planets.records[13];

    assert_eq!(pre_target.planet.stored_goods.to_bytes()[0], 0x00);
    assert_eq!(pre_target.ground_batteries_raw(), 1);
    assert_eq!(pre_target.army_count_raw(), 0x00);
    assert_eq!(post_target.ground_batteries_raw(), 0);
    assert_eq!(post_target.army_count_raw(), 0x00);
    assert_eq!(post_target.owner_empire_slot_raw(), 2);
    assert_ne!(pre_target, post_target);

    assert_eq!(
        read_ecmaint_bombard_army1_dev0_b09_post_fixture("MESSAGES.DAT"),
        Vec::<u8>::new()
    );
    assert_eq!(
        read_ecmaint_bombard_army1_dev0_b09_post_fixture("RESULTS.DAT"),
        Vec::<u8>::new()
    );
}

#[test]
fn ecmaint_bombard_heavy_generates_combat_report() {
    let pre_planets =
        PlanetDat::parse(&read_ecmaint_bombard_heavy_pre_fixture("PLANETS.DAT")).unwrap();
    let post_planets =
        PlanetDat::parse(&read_ecmaint_bombard_heavy_post_fixture("PLANETS.DAT")).unwrap();
    let pre_target = &pre_planets.records[13];
    let post_target = &post_planets.records[13];

    assert_eq!(pre_target.ground_batteries_raw(), 15);
    assert_eq!(pre_target.army_count_raw(), 0x8E);
    assert_ne!(pre_target, post_target);

    let results = read_ecmaint_bombard_heavy_post_fixture("RESULTS.DAT");
    assert!(
        !results.is_empty(),
        "RESULTS.DAT should contain the bombardment report"
    );
}
