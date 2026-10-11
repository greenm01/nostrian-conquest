mod common;

use common::*;
use nc_data::classic::{parse_fleets_dat, parse_planets_dat};
use nc_data::*;

#[test]
fn ecmaint_build_scenario_consumes_queue_and_changes_planet_state() {
    let pre_bytes = read_ecmaint_build_pre_fixture("PLANETS.DAT");
    let post_bytes = read_ecmaint_build_post_fixture("PLANETS.DAT");
    let post = parse_planets_dat(&post_bytes).unwrap();

    let post_record = &post[14];

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
    assert_eq!(post_record.stardock.destroyers, 3);
    assert_eq!(post_record.stardock.etacs, 1);
}

#[test]
fn ecmaint_fleet_scenario_consumes_order_and_updates_fleet_and_planet_state() {
    let pre_fleets = parse_fleets_dat(&read_ecmaint_fleet_pre_fixture("FLEETS.DAT")).unwrap();
    let post_fleets = parse_fleets_dat(&read_ecmaint_fleet_post_fixture("FLEETS.DAT")).unwrap();
    let post_planets = parse_planets_dat(&read_ecmaint_fleet_post_fixture("PLANETS.DAT")).unwrap();

    let pre_fleet = &pre_fleets[0];
    let post_fleet = &post_fleets[0];

    assert_eq!(pre_fleet.speed, 3);
    assert_eq!(pre_fleet.order, Order::ColonizeWorld);
    assert_eq!(pre_fleet.target.to_array()[0], 0x0f);

    assert_eq!(post_fleet.sector.to_array()[0], 0x0f);
    assert_eq!(
        post_fleet.depth.to_bytes(),
        [0x80, 0xb9, 0xff, 0xff, 0xff, 0x7f]
    );
    assert_eq!(post_fleet.order, Order::HoldPosition);
    assert_eq!(post_fleet.target.to_array()[0], 0x0f);

    let post_planet = &post_planets[13];
    assert_eq!(post_planet.armies, 1);
    assert_eq!(post_planet.years_under_owner, 2);
    assert_eq!(post_planet.owner.0, 1);
}

#[test]
fn ecmaint_bombard_scenario_arrival_preserves_attack_order() {
    let pre = parse_fleets_dat(&read_ecmaint_bombard_pre_fixture("FLEETS.DAT")).unwrap();
    let arrive = parse_fleets_dat(&read_ecmaint_bombard_arrive_fixture("FLEETS.DAT")).unwrap();

    let pre_fleet = &pre[2];
    let arrive_fleet = &arrive[2];

    assert_eq!(pre_fleet.speed, 0x03);
    assert_eq!(pre_fleet.order.to_raw(), 0x06);
    assert_eq!(pre_fleet.target.to_array(), [0x0f, 0x0d]);

    assert_eq!(arrive_fleet.speed, 0x03);
    assert_eq!(arrive_fleet.sector.to_array(), [0x0f, 0x0d]);
    assert_eq!(arrive_fleet.order.to_raw(), 0x06);
    assert_eq!(arrive_fleet.target.to_array(), [0x0f, 0x0d]);
}

// These preserved-fixture checks document classic ECMAINT outputs.
// They are oracle evidence, not the canonical Rust bombardment rule.
#[test]
fn ecmaint_bombard_scenario_second_pass_consumes_order_and_kills_attackers() {
    let arrive = parse_fleets_dat(&read_ecmaint_bombard_arrive_fixture("FLEETS.DAT")).unwrap();
    let post = parse_fleets_dat(&read_ecmaint_bombard_post_fixture("FLEETS.DAT")).unwrap();

    let arrive_fleet = &arrive[2];
    let post_fleet = &post[2];

    assert_eq!(arrive_fleet.speed, 0x03);
    assert_eq!(arrive_fleet.order.to_raw(), 0x06);
    assert_eq!(arrive_fleet.ships.cruisers, 0x03);
    assert_eq!(arrive_fleet.ships.destroyers, 0x05);

    assert_eq!(post_fleet.speed, 0x00);
    assert_eq!(post_fleet.order.to_raw(), 0x00);
    assert_eq!(post_fleet.sector.to_array(), [0x0f, 0x0d]);
    assert_eq!(post_fleet.ships.cruisers, 0x02);
    assert_eq!(post_fleet.ships.destroyers, 0x01);

    let arrive_planets =
        parse_planets_dat(&read_ecmaint_bombard_arrive_fixture("PLANETS.DAT")).unwrap();
    let post_planets =
        parse_planets_dat(&read_ecmaint_bombard_post_fixture("PLANETS.DAT")).unwrap();
    assert_eq!(arrive_planets[13], post_planets[13]);
}

#[test]
fn ecmaint_bombard_zero_army_target_changes_planet_without_attacker_losses() {
    let pre = parse_fleets_dat(&read_ecmaint_bombard_army0_pre_fixture("FLEETS.DAT")).unwrap();
    let post = parse_fleets_dat(&read_ecmaint_bombard_army0_post_fixture("FLEETS.DAT")).unwrap();

    let pre_fleet = &pre[2];
    let post_fleet = &post[2];

    assert_eq!(pre_fleet.order.to_raw(), 0x06);
    assert_eq!(pre_fleet.ships.cruisers, 0x03);
    assert_eq!(pre_fleet.ships.destroyers, 0x05);

    assert_eq!(post_fleet.speed, 0x00);
    assert_eq!(post_fleet.order.to_raw(), 0x00);
    assert_eq!(post_fleet.sector.to_array(), [0x0f, 0x0d]);
    assert_eq!(post_fleet.ships.cruisers, 0x03);
    assert_eq!(post_fleet.ships.destroyers, 0x05);

    let pre_planets =
        parse_planets_dat(&read_ecmaint_bombard_army0_pre_fixture("PLANETS.DAT")).unwrap();
    let post_planets =
        parse_planets_dat(&read_ecmaint_bombard_army0_post_fixture("PLANETS.DAT")).unwrap();
    let pre_target = &pre_planets[13];
    let post_target = &post_planets[13];

    assert_eq!(pre_target.ground_batteries, 0);
    assert_eq!(post_target.ground_batteries, 0);
    assert_eq!(pre_target.owner.0, 2);
    assert_eq!(post_target.owner.0, 2);
    assert_eq!(pre_target.armies, 0x8e);
    assert_eq!(post_target.armies, 0x8a);
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
    let pre = parse_fleets_dat(&read_ecmaint_bombard_army0_dev0_pre_fixture("FLEETS.DAT")).unwrap();
    let post =
        parse_fleets_dat(&read_ecmaint_bombard_army0_dev0_post_fixture("FLEETS.DAT")).unwrap();

    let pre_fleet = &pre[2];
    let post_fleet = &post[2];

    assert_eq!(pre_fleet.order.to_raw(), 0x06);
    assert_eq!(post_fleet.order.to_raw(), 0x00);
    assert_eq!(post_fleet.speed, 0x00);
    assert_eq!(post_fleet.sector.to_array(), [0x0f, 0x0d]);
    assert_eq!(post_fleet.ships.cruisers, 0x03);
    assert_eq!(post_fleet.ships.destroyers, 0x05);

    let pre_planets =
        parse_planets_dat(&read_ecmaint_bombard_army0_dev0_pre_fixture("PLANETS.DAT")).unwrap();
    let post_planets =
        parse_planets_dat(&read_ecmaint_bombard_army0_dev0_post_fixture("PLANETS.DAT")).unwrap();
    let pre_target = &pre_planets[13];
    let post_target = &post_planets[13];

    assert_eq!(pre_target.armies, 0x00);
    assert_eq!(post_target.armies, 0x00);
    assert_eq!(pre_target.ground_batteries, 0x00);
    assert_eq!(post_target.ground_batteries, 0x00);
    assert_eq!(pre_target.owner.0, 2);
    assert_eq!(post_target.owner.0, 2);
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
    let pre = parse_fleets_dat(&read_ecmaint_bombard_army1_pre_fixture("FLEETS.DAT")).unwrap();
    let post = parse_fleets_dat(&read_ecmaint_bombard_army1_post_fixture("FLEETS.DAT")).unwrap();

    let pre_fleet = &pre[2];
    let post_fleet = &post[2];

    assert_eq!(pre_fleet.order.to_raw(), 0x06);
    assert_eq!(pre_fleet.ships.cruisers, 0x03);
    assert_eq!(pre_fleet.ships.destroyers, 0x05);

    assert_eq!(post_fleet.speed, 0x00);
    assert_eq!(post_fleet.order.to_raw(), 0x00);
    assert_eq!(post_fleet.sector.to_array(), [0x0f, 0x0d]);
    assert_eq!(post_fleet.ships.cruisers, 0x02);
    assert_eq!(post_fleet.ships.destroyers, 0x02);

    let pre_planets =
        parse_planets_dat(&read_ecmaint_bombard_army1_pre_fixture("PLANETS.DAT")).unwrap();
    let post_planets =
        parse_planets_dat(&read_ecmaint_bombard_army1_post_fixture("PLANETS.DAT")).unwrap();
    let pre_target = &pre_planets[13];
    let post_target = &post_planets[13];

    assert_eq!(pre_target.ground_batteries, 1);
    assert_eq!(post_target.ground_batteries, 0);
    assert_eq!(pre_target.owner.0, 2);
    assert_eq!(post_target.owner.0, 2);
    assert_eq!(pre_target.armies, 0x8e);
    assert_eq!(post_target.armies, 0x8d);
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
    let pre = parse_fleets_dat(&read_ecmaint_bombard_army1_dev0_pre_fixture("FLEETS.DAT")).unwrap();
    let post =
        parse_fleets_dat(&read_ecmaint_bombard_army1_dev0_post_fixture("FLEETS.DAT")).unwrap();

    let pre_fleet = &pre[2];
    let post_fleet = &post[2];

    assert_eq!(pre_fleet.order.to_raw(), 0x06);
    assert_eq!(pre_fleet.ships.cruisers, 0x03);
    assert_eq!(pre_fleet.ships.destroyers, 0x05);

    assert_eq!(post_fleet.speed, 0x00);
    assert_eq!(post_fleet.order.to_raw(), 0x00);
    assert_eq!(post_fleet.sector.to_array(), [0x0f, 0x0d]);
    assert_eq!(post_fleet.ships.cruisers, 0x02);
    assert_eq!(post_fleet.ships.destroyers, 0x04);

    let pre_planets =
        parse_planets_dat(&read_ecmaint_bombard_army1_dev0_pre_fixture("PLANETS.DAT")).unwrap();
    let post_planets =
        parse_planets_dat(&read_ecmaint_bombard_army1_dev0_post_fixture("PLANETS.DAT")).unwrap();
    let pre_target = &pre_planets[13];
    let post_target = &post_planets[13];

    assert_eq!(pre_target.ground_batteries, 1);
    assert_eq!(post_target.ground_batteries, 0);
    assert_eq!(pre_target.armies, 0x00);
    assert_eq!(post_target.armies, 0x00);
    assert_eq!(pre_target.owner.0, 2);
    assert_eq!(post_target.owner.0, 2);
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
    let pre = parse_fleets_dat(&read_ecmaint_bombard_army1_dev0_e0c_pre_fixture(
        "FLEETS.DAT",
    ))
    .unwrap();
    let post = parse_fleets_dat(&read_ecmaint_bombard_army1_dev0_e0c_post_fixture(
        "FLEETS.DAT",
    ))
    .unwrap();

    let pre_fleet = &pre[2];
    let post_fleet = &post[2];

    assert_eq!(pre_fleet.order.to_raw(), 0x06);
    assert_eq!(pre_fleet.ships.cruisers, 0x03);
    assert_eq!(pre_fleet.ships.destroyers, 0x05);

    assert_eq!(post_fleet.speed, 0x00);
    assert_eq!(post_fleet.order.to_raw(), 0x00);
    assert_eq!(post_fleet.sector.to_array(), [0x0f, 0x0d]);
    assert_eq!(post_fleet.ships.cruisers, 0x03);
    assert_eq!(post_fleet.ships.destroyers, 0x01);

    let pre_planets = parse_planets_dat(&read_ecmaint_bombard_army1_dev0_e0c_pre_fixture(
        "PLANETS.DAT",
    ))
    .unwrap();
    let post_planets = parse_planets_dat(&read_ecmaint_bombard_army1_dev0_e0c_post_fixture(
        "PLANETS.DAT",
    ))
    .unwrap();
    let pre_target = &pre_planets[13];
    let post_target = &post_planets[13];

    assert_eq!(pre_target.stored_goods.to_bytes()[5], 0x0c);
    assert_eq!(pre_target.ground_batteries, 1);
    assert_eq!(pre_target.armies, 0x00);
    assert_eq!(post_target.ground_batteries, 0);
    assert_eq!(post_target.armies, 0x00);
    assert_eq!(post_target.stored_goods.to_bytes()[5], 0x54);
    assert_eq!(post_target.owner.0, 2);
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
    let pre = parse_fleets_dat(&read_ecmaint_bombard_army1_dev0_b08_pre_fixture(
        "FLEETS.DAT",
    ))
    .unwrap();
    let post = parse_fleets_dat(&read_ecmaint_bombard_army1_dev0_b08_post_fixture(
        "FLEETS.DAT",
    ))
    .unwrap();

    let pre_fleet = &pre[2];
    let post_fleet = &post[2];

    assert_eq!(pre_fleet.order.to_raw(), 0x06);
    assert_eq!(pre_fleet.ships.cruisers, 0x03);
    assert_eq!(pre_fleet.ships.destroyers, 0x05);

    assert_eq!(post_fleet.speed, 0x00);
    assert_eq!(post_fleet.order.to_raw(), 0x00);
    assert_eq!(post_fleet.sector.to_array(), [0x0f, 0x0d]);
    assert_eq!(post_fleet.ships.cruisers, 0x01);
    assert_eq!(post_fleet.ships.destroyers, 0x03);

    let pre_planets = parse_planets_dat(&read_ecmaint_bombard_army1_dev0_b08_pre_fixture(
        "PLANETS.DAT",
    ))
    .unwrap();
    let post_planets = parse_planets_dat(&read_ecmaint_bombard_army1_dev0_b08_post_fixture(
        "PLANETS.DAT",
    ))
    .unwrap();
    let pre_target = &pre_planets[13];
    let post_target = &post_planets[13];

    assert_eq!(pre_target.production.to_bytes()[5], 0x00);
    assert_eq!(pre_target.ground_batteries, 1);
    assert_eq!(pre_target.armies, 0x00);
    assert_eq!(post_target.ground_batteries, 0);
    assert_eq!(post_target.armies, 0x00);
    assert_eq!(post_target.owner.0, 2);
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
    let pre = parse_fleets_dat(&read_ecmaint_bombard_army1_dev0_b09_pre_fixture(
        "FLEETS.DAT",
    ))
    .unwrap();
    let post = parse_fleets_dat(&read_ecmaint_bombard_army1_dev0_b09_post_fixture(
        "FLEETS.DAT",
    ))
    .unwrap();

    let pre_fleet = &pre[2];
    let post_fleet = &post[2];

    assert_eq!(pre_fleet.order.to_raw(), 0x06);
    assert_eq!(pre_fleet.ships.cruisers, 0x03);
    assert_eq!(pre_fleet.ships.destroyers, 0x05);

    assert_eq!(post_fleet.speed, 0x00);
    assert_eq!(post_fleet.order.to_raw(), 0x00);
    assert_eq!(post_fleet.sector.to_array(), [0x0f, 0x0d]);
    assert_eq!(post_fleet.ships.cruisers, 0x01);
    assert_eq!(post_fleet.ships.destroyers, 0x05);

    let pre_planets = parse_planets_dat(&read_ecmaint_bombard_army1_dev0_b09_pre_fixture(
        "PLANETS.DAT",
    ))
    .unwrap();
    let post_planets = parse_planets_dat(&read_ecmaint_bombard_army1_dev0_b09_post_fixture(
        "PLANETS.DAT",
    ))
    .unwrap();
    let pre_target = &pre_planets[13];
    let post_target = &post_planets[13];

    assert_eq!(pre_target.stored_goods.to_bytes()[0], 0x00);
    assert_eq!(pre_target.ground_batteries, 1);
    assert_eq!(pre_target.armies, 0x00);
    assert_eq!(post_target.ground_batteries, 0);
    assert_eq!(post_target.armies, 0x00);
    assert_eq!(post_target.owner.0, 2);
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
        parse_planets_dat(&read_ecmaint_bombard_heavy_pre_fixture("PLANETS.DAT")).unwrap();
    let post_planets =
        parse_planets_dat(&read_ecmaint_bombard_heavy_post_fixture("PLANETS.DAT")).unwrap();
    let pre_target = &pre_planets[13];
    let post_target = &post_planets[13];

    assert_eq!(pre_target.ground_batteries, 15);
    assert_eq!(pre_target.armies, 0x8E);
    assert_ne!(pre_target, post_target);

    let results = read_ecmaint_bombard_heavy_post_fixture("RESULTS.DAT");
    assert!(
        !results.is_empty(),
        "RESULTS.DAT should contain the bombardment report"
    );
}
