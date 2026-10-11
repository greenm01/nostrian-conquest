mod common;

use common::*;
use nc_compat::classic::{
    IPBM_RECORD_SIZE, encode_base, encode_ipbm, encode_planet, encode_setup, parse_bases_dat,
    parse_conquest_dat, parse_fleets_dat, parse_ipbm_dat, parse_planets_dat, parse_player_dat,
    parse_setup_dat, serialize_ipbm_dat,
};
use nc_compat::load_classic_core_files;
use nc_data::*;
use nc_data::{
    Coords, PlayerMode, ProductionItemKind, Real48, present_production_points, recompute_max_speed,
    set_armies_aboard, set_empire_name, set_fleet_at_rest, set_login_reviewables_present,
    set_mission_target, set_planet_name, set_player_handle, set_starbase_at_rest,
    set_troop_transports, stardock_slot_kind, stored_production_points, troop_transports,
};

#[test]
fn base_record_setters_can_recreate_known_valid_guard_starbase_record() {
    let mut record = Starbase::default();
    record.escort.0 = 0x0001;
    record.base_id.0 = 0x0001;
    record.owner.0 = 0x01;
    record.next.0 = 0x0000;
    record.record.0 = 0x0001;
    record.prev.0 = 0x0000;
    record.sector = Coords::new(0x10, 0x0D);
    set_starbase_at_rest(&mut record);
    record.moving = false;
    record.target = Coords::new(0x10, 0x0D);
    record.target_depth = 1;

    assert_eq!(record.escort.0, 0x0001);
    assert_eq!(record.base_id.0, 0x0001);
    assert_eq!(record.owner.0, 0x01);
    assert_eq!(record.next.0, 0x0000);
    assert_eq!(record.record.0, 0x0001);
    assert_eq!(record.prev.0, 0x0000);
    assert_eq!(record.sector.to_array(), [0x10, 0x0D]);
    assert_eq!(record.x_fraction, Real48::HALF);
    assert_eq!(record.y_fraction, Real48::HALF);
    assert_eq!(record.depth, Real48::ONE);
    assert!(!record.moving);
    assert_eq!(record.target.to_array(), [0x10, 0x0D]);
    assert_eq!(record.target_depth, 1);

    assert_eq!(
        encode_base(&record),
        [
            0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x10, 0x0D, 0x80,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x81, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x10, 0x0D, 0x01,
        ]
    );
}

#[test]
fn guard_starbase_related_accessors_expose_linkage_words() {
    let player = parse_player_dat(&read_fixture("PLAYER.DAT")).unwrap();
    assert_eq!(player[0].fleets_first.0, 1);
    assert_eq!(player[0].ipbms_first.0, 0);

    let fleet_bytes = read_ecmaint_starbase_pre_fixture("FLEETS.DAT");
    let fleets = parse_fleets_dat(&fleet_bytes).unwrap();
    let fleet = &fleets[0];
    assert_eq!(fleet.fleet_number.0, 1);
    assert_eq!(fleet.next.0, 2);
    assert_eq!(fleet.record.0, 1);
    assert_eq!(fleet.target_depth, 1);
    assert_eq!(fleet.mission_target, 1);

    let base_bytes = read_ecmaint_starbase_pre_fixture("BASES.DAT");
    let bases = parse_bases_dat(&base_bytes).unwrap();
    let base = &bases[0];
    assert_eq!(base.escort.0, fleet.fleet_number.0);
    assert_eq!(base.base_id.0, fleet.mission_target);
    assert_eq!(base.record.0, 1);
}

#[test]
fn set_starbase_destination_writes_what_ecgame_move_writes() {
    let mut data = GameStateBuilder::new()
        .with_player_count(4)
        .with_guard_starbase(1, 1, [16, 13], 1)
        .build_initialized_baseline()
        .expect("baseline should build");

    data.set_starbase_destination(1, 1, [2, 12])
        .expect("owned starbase destination should update");

    let base = &data.bases[0];
    assert_eq!(base.sector.to_array(), [16, 13]);
    assert_eq!(base.target.to_array(), [2, 12]);
    assert!(base.moving);
    assert_eq!(base.target_depth, 0, "no world of empire 1 at (2,12)");

    let home = data.planets[0].sector.to_array();
    assert_eq!(data.planets[0].owner.0, 1);
    data.set_starbase_destination(1, 1, home)
        .expect("owned starbase destination should update");
    assert_eq!(data.bases[0].target.to_array(), home);
    assert_eq!(data.bases[0].target_depth, 1, "own world: orbit");
}

#[test]
fn halt_starbase_writes_what_ecgame_halt_writes() {
    let mut data = GameStateBuilder::new()
        .with_player_count(4)
        .with_guard_starbase(1, 1, [16, 13], 1)
        .build_initialized_baseline()
        .expect("baseline should build");
    data.set_starbase_destination(1, 1, [2, 12]).unwrap();
    // Part way through the climb out of orbit.
    data.bases[0].depth = Real48::from_bytes([0x80, 0x00, 0x00, 0x00, 0x00, 0x40]);

    data.halt_starbase(1, 1)
        .expect("owned starbase halt should succeed");

    let base = &data.bases[0];
    assert_eq!(base.sector.to_array(), [16, 13]);
    assert_eq!(base.target.to_array(), [16, 13]);
    assert!(!base.moving);
    assert_eq!(base.target_depth, 1, "Round(0.75) = 1");
    assert_eq!(base.depth.to_bytes(), [0x80, 0x00, 0x00, 0x00, 0x00, 0x40]);
}

#[test]
fn fleet_owner_empire_accessor_round_trips() {
    let mut record = Fleet::default();
    record.owner.0 = 0x03;
    assert_eq!(record.owner.0, 0x03);
}

#[test]
fn fleet_recompute_max_speed_uses_slowest_member() {
    let mut record = Fleet::default();
    record.ships.destroyers = 10;
    record.ships.etacs = 1;
    record.speed = 6;
    recompute_max_speed(&mut record);
    assert_eq!(record.max_speed, 3);
    assert_eq!(record.speed, 3);
}

#[test]
fn player_fleet_chain_head_accessor_round_trips() {
    let mut record = Player::default();
    record.fleets_first.0 = 0x1234;
    assert_eq!(record.fleets_first.0, 0x1234);
}

#[test]
fn controlled_empire_name_setter_does_not_clobber_later_player_fields() {
    let mut record = Player {
        homeworld: 7,
        tax_rate: 50,
        ..Player::default()
    };
    record.ipbms_first.0 = 3;
    record.messages_first = 1;
    record.messages_last = 1;

    set_empire_name(&mut record, "Codex Dominion");

    assert_eq!(record.empire_name.trim().to_string(), "Codex Dominion");
    assert_eq!(record.homeworld, 7);
    assert_eq!(record.tax_rate, 50);
    assert_eq!(record.ipbms_first.0, 3);
    assert_eq!(record.messages_first, 1);
    assert_eq!(record.messages_last, 1);
}

#[test]
fn classic_login_reviewables_helper_mirrors_both_review_families() {
    let mut record = Player::default();

    set_login_reviewables_present(&mut record, true);
    assert_eq!(record.messages_first, 1);
    assert_eq!((record.messages_first >> 16) as u16, 0);
    assert_eq!(record.messages_last, 1);
    assert_eq!((record.messages_last >> 16) as u16, 0);
    assert!(record.messages_first != 0);
    assert!(record.messages_last != 0);
    assert!((record.messages_first != 0 || record.messages_last != 0));

    set_login_reviewables_present(&mut record, false);
    assert_eq!(record.messages_first, 0);
    assert_eq!((record.messages_first >> 16) as u16, 0);
    assert_eq!(record.messages_last, 0);
    assert_eq!((record.messages_last >> 16) as u16, 0);
    assert!(!(record.messages_first != 0 || record.messages_last != 0));
}

#[test]
fn rename_owned_planet_updates_name_for_matching_owner() {
    let mut planet = Planet::default();
    planet.owner.0 = 1;
    set_planet_name(&mut planet, "Not Named Yet");

    let mut data = CoreGameData {
        players: vec![Player::default()],
        planets: vec![planet],
        fleets: vec![],
        bases: vec![],
        ipbms: vec![],
        setup: Setup::default(),
        conquest: Conquest::default(),
    };
    data.players[0].mode = PlayerMode::Human;

    data.rename_owned_planet(1, 1, "New Horizon").unwrap();
    assert_eq!(data.planets[0].name.clone(), "New Horizon");
}

#[test]
fn rename_owned_planet_rejects_non_printable_ascii_names() {
    let mut data = GameStateBuilder::new()
        .with_player_count(2)
        .build_initialized_baseline()
        .expect("baseline should build");
    let planet_record_index_1_based = data
        .planets
        .iter()
        .position(|planet| planet.owner.0 == 1)
        .map(|idx| idx + 1)
        .unwrap();
    let before = data.planets[planet_record_index_1_based - 1].name.clone();

    for name in [
        "\u{1F600}\u{1F600}\u{1F600}\u{1F600}",
        "\u{C6}gir",
        "Tab\tWorld",
    ] {
        assert_eq!(
            data.rename_owned_planet(1, planet_record_index_1_based, name),
            Err(GameStateMutationError::InvalidPlanetName {
                planet_index_1_based: planet_record_index_1_based,
            })
        );
        assert_eq!(
            data.rename_player_homeworld(1, name),
            Err(GameStateMutationError::InvalidPlanetName {
                planet_index_1_based: planet_record_index_1_based,
            })
        );
    }
    assert_eq!(
        data.planets[planet_record_index_1_based - 1].name.clone(),
        before
    );
}

#[test]
fn set_planet_name_stores_classic_text_up_to_twenty_chars() {
    let mut planet = Planet::default();
    set_planet_name(&mut planet, "\u{1F600}\u{1F600}\u{1F600}\u{1F600}");
    assert_eq!(planet.name.clone(), "????");

    set_planet_name(&mut planet, "Fourteen Chars");
    assert_eq!(planet.name.clone(), "Fourteen Chars");

    set_planet_name(&mut planet, "Twenty-one characters");
    assert_eq!(planet.name.clone(), "Twenty-one character");
}

#[test]
fn join_player_normalizes_zero_tax_slots_to_seeded_opening_tax_rate() {
    let mut data = GameStateBuilder::new()
        .with_player_count(2)
        .build_joinable_new_game_baseline()
        .expect("joinable baseline should build");

    data.players[0].tax_rate = 0;
    data.planets[0].stored_goods = Real48::ZERO;

    data.join_player(1, "Codex Dominion")
        .expect("join player should succeed");

    assert_eq!(data.players[0].tax_rate, 50);
    assert_eq!(stored_production_points(&data.planets[0]), 50);
}

#[test]
fn join_player_rejects_already_joined_seat() {
    let mut data = GameStateBuilder::new()
        .with_player_count(2)
        .build_joinable_new_game_baseline()
        .expect("joinable baseline should build");

    data.join_player(1, "Codex Dominion")
        .expect("first join should succeed");

    assert_eq!(
        data.join_player(1, "Other Empire"),
        Err(GameStateMutationError::PlayerAlreadyJoined { index_1_based: 1 })
    );
}

#[test]
fn join_player_rejects_civil_disorder_seat_that_was_already_used() {
    let mut data = GameStateBuilder::new()
        .with_player_count(2)
        .build_joinable_new_game_baseline()
        .expect("joinable baseline should build");

    data.players[0].last_year_on = 3004;
    data.planets[0].owner.0 = 0;
    data.planets[0].years_under_owner = 0;
    data.players[0].planet_count = 0;

    assert!(!data.player_slot_is_open_for_first_join(1));
    assert_eq!(
        data.join_player(1, "Other Empire"),
        Err(GameStateMutationError::PlayerSlotNotJoinable { index_1_based: 1 })
    );
}

#[test]
fn join_player_rejects_non_printable_ascii_empire_names() {
    let mut data = GameStateBuilder::new()
        .with_player_count(2)
        .build_joinable_new_game_baseline()
        .expect("joinable baseline should build");

    for name in ["\u{C6}gir Dominion", "Em\u{2014}pire", "Tab\tEmpire"] {
        assert_eq!(
            data.join_player(1, name),
            Err(GameStateMutationError::InvalidEmpireName {
                player_index_1_based: 1,
            })
        );
    }
    assert!(data.player_slot_is_open_for_first_join(1));
}

#[test]
fn classic_text_setters_store_question_marks_for_other_characters() {
    let mut player = Player::default();
    set_empire_name(&mut player, "\u{C6}gir Dominion");
    assert_eq!(player.empire_name.trim().to_string(), "?gir Dominion");
    set_player_handle(&mut player, "Bj\u{F6}rn\u{1F600}");
    assert_eq!(player.handle.trim().to_string(), "Bj?rn?");

    let mut planet = Planet::default();
    set_planet_name(&mut planet, "\u{1F600}\u{1F600}\u{1F600}\u{1F600}");
    assert_eq!(planet.name.clone(), "????");
}

#[test]
fn stardock_slots_are_typed_u32_counts_at_classic_offsets() {
    let mut planet = Planet::default();

    planet.stardock.destroyers = 1;
    planet.stardock.scouts = 258;
    planet.stardock.starbases = 70_000;

    assert_eq!(planet.stardock.destroyers, 1);
    assert_eq!(planet.stardock.scouts, 258);
    assert_eq!(planet.stardock.starbases, 70_000);
    assert_eq!(stardock_slot_kind(&planet, 0).code(), 1);
    assert_eq!(stardock_slot_kind(&planet, 1).code(), 0);
    assert_eq!(stardock_slot_kind(&planet, 3).code(), 4);
    assert_eq!(stardock_slot_kind(&planet, 6).code(), 9);

    let bytes = encode_planet(&planet);
    assert_eq!(bytes[0x38..0x3C], [1, 0, 0, 0]);
    assert_eq!(bytes[0x44..0x48], [2, 1, 0, 0]);
    assert_eq!(bytes[0x50..0x54], 70_000u32.to_le_bytes());
}

#[test]
fn commission_ship_from_stardock_appends_fleet_and_clears_slot() {
    let player = Player {
        mode: PlayerMode::Human,
        ..Player::default()
    };

    let mut planet = Planet::default();
    planet.owner.0 = 1;
    planet.sector = Coords::new(6, 5);
    planet.stardock.destroyers = 2;

    let mut data = CoreGameData {
        players: vec![player],
        planets: vec![planet],
        fleets: vec![],
        bases: vec![],
        ipbms: vec![],
        setup: Setup::default(),
        conquest: Conquest::default(),
    };

    let result = data.commission_planet_stardock_slot(1, 1, 0).unwrap();
    assert_eq!(
        result,
        CommissionResult::Fleet {
            fleet_record_index_1_based: 1
        }
    );
    assert_eq!(stardock_slot_kind(&data.planets[0], 0).code(), 0);
    assert_eq!(data.planets[0].stardock.destroyers, 0);
    assert_eq!(data.players[0].fleets_first.0, 1);
    assert_eq!(data.fleets.len(), 1);
    let fleet = &data.fleets[0];
    assert_eq!(fleet.owner.0, 1);
    assert_eq!(fleet.record.0, 1);
    assert_eq!(fleet.fleet_number.0, 1);
    assert_eq!(fleet.ships.destroyers, 2);
    assert_eq!(fleet.sector.to_array(), [6, 5]);
    assert_eq!(fleet.order, Order::HoldPosition);
    assert_eq!(fleet.rules_of_engagement, 6);
}

#[test]
fn commission_support_only_fleet_starts_at_roe_zero() {
    let player = Player {
        mode: PlayerMode::Human,
        ..Player::default()
    };

    let mut planet = Planet::default();
    planet.owner.0 = 1;
    planet.sector = Coords::new(6, 5);
    planet.stardock.scouts = 2;

    let mut data = CoreGameData {
        players: vec![player],
        planets: vec![planet],
        fleets: vec![],
        bases: vec![],
        ipbms: vec![],
        setup: Setup::default(),
        conquest: Conquest::default(),
    };

    let result = data.commission_planet_stardock_slot(1, 1, 3).unwrap();
    assert_eq!(
        result,
        CommissionResult::Fleet {
            fleet_record_index_1_based: 1
        }
    );
    let fleet = &data.fleets[0];
    assert_eq!(fleet.ships.scouts, 2);
    assert_eq!(fleet.rules_of_engagement, 0);
}

#[test]
fn append_planet_build_order_still_uses_all_ten_queue_slots() {
    let player = Player {
        mode: PlayerMode::Human,
        ..Player::default()
    };

    let mut planet = Planet::default();
    planet.owner.0 = 1;
    for slot in 0..9 {
        planet.build_queue[slot].points_owed = 1;
        planet.build_queue[slot].kind = ProductionItemKind::Destroyer;
    }

    let mut data = CoreGameData {
        players: vec![player],
        planets: vec![planet],
        fleets: vec![],
        bases: vec![],
        ipbms: vec![],
        setup: Setup::default(),
        conquest: Conquest::default(),
    };

    data.append_planet_build_order(1, 15, 4).unwrap();

    assert_eq!(data.planets[0].build_queue[9].points_owed, 15);
    assert_eq!(data.planets[0].build_queue[9].kind.code(), 4);
}

#[test]
fn append_planet_build_order_merges_into_matching_kind_before_using_empty_slot() {
    let player = Player {
        mode: PlayerMode::Human,
        ..Player::default()
    };

    let mut planet = Planet::default();
    planet.owner.0 = 1;
    planet.build_queue[0].points_owed = 10;
    planet.build_queue[0].kind = ProductionItemKind::Destroyer;

    let mut data = CoreGameData {
        players: vec![player],
        planets: vec![planet],
        fleets: vec![],
        bases: vec![],
        ipbms: vec![],
        setup: Setup::default(),
        conquest: Conquest::default(),
    };

    data.append_planet_build_order(1, 15, 1).unwrap();

    assert_eq!(data.planets[0].build_queue[0].points_owed, 25);
    assert_eq!(data.planets[0].build_queue[0].kind.code(), 1);
    assert!((1..10).all(|slot| data.planets[0].build_queue[slot].points_owed == 0));
}

#[test]
fn append_planet_build_order_spills_same_kind_across_records_without_truncation() {
    let player = Player {
        mode: PlayerMode::Human,
        ..Player::default()
    };

    let mut planet = Planet::default();
    planet.owner.0 = 1;
    planet.build_queue[0].points_owed = 250;
    planet.build_queue[0].kind = ProductionItemKind::Destroyer;

    let mut data = CoreGameData {
        players: vec![player],
        planets: vec![planet],
        fleets: vec![],
        bases: vec![],
        ipbms: vec![],
        setup: Setup::default(),
        conquest: Conquest::default(),
    };

    data.append_planet_build_order(1, 20, 1).unwrap();

    assert_eq!(data.planets[0].build_queue[0].points_owed, u8::MAX);
    assert_eq!(data.planets[0].build_queue[0].kind.code(), 1);
    assert_eq!(data.planets[0].build_queue[1].points_owed, 15);
    assert_eq!(data.planets[0].build_queue[1].kind.code(), 1);
}

#[test]
fn append_planet_build_order_does_not_partially_write_when_capacity_is_exhausted() {
    let player = Player {
        mode: PlayerMode::Human,
        ..Player::default()
    };

    let mut planet = Planet::default();
    planet.owner.0 = 1;
    for slot in 0..10 {
        planet.build_queue[slot].points_owed = u8::MAX;
        planet.build_queue[slot].kind = ProductionItemKind::Destroyer;
    }

    let original = planet.clone();
    let mut data = CoreGameData {
        players: vec![player],
        planets: vec![planet],
        fleets: vec![],
        bases: vec![],
        ipbms: vec![],
        setup: Setup::default(),
        conquest: Conquest::default(),
    };

    let error = data.append_planet_build_order(1, 2, 8).unwrap_err();

    assert_eq!(
        error,
        GameStateMutationError::PlanetBuildQueueFull { index_1_based: 1 }
    );
    assert_eq!(data.planets[0], original);
}

#[test]
fn append_planet_build_order_rejects_non_multiple_ship_points() {
    let player = Player {
        mode: PlayerMode::Human,
        ..Player::default()
    };

    let mut planet = Planet::default();
    planet.owner.0 = 1;

    let mut data = CoreGameData {
        players: vec![player],
        planets: vec![planet],
        fleets: vec![],
        bases: vec![],
        ipbms: vec![],
        setup: Setup::default(),
        conquest: Conquest::default(),
    };

    let error = data.append_planet_build_order(1, 14, 4).unwrap_err();
    assert_eq!(
        error,
        GameStateMutationError::InvalidPlanetPlayerInput {
            planet_index_1_based: 1,
            reason: PlanetPlayerInputValidationError::InvalidBuildPointsForKind {
                kind: 4,
                points_remaining: 14,
            },
        }
    );
}

#[test]
fn append_planet_build_order_spills_starbases_one_per_queue_slot() {
    let player = Player {
        mode: PlayerMode::Human,
        ..Player::default()
    };

    let mut planet = Planet::default();
    planet.owner.0 = 1;

    let mut data = CoreGameData {
        players: vec![player],
        planets: vec![planet],
        fleets: vec![],
        bases: vec![],
        ipbms: vec![],
        setup: Setup::default(),
        conquest: Conquest::default(),
    };

    data.append_planet_build_order(1, 150, 9).unwrap();

    assert_eq!(data.planets[0].build_queue[0].points_owed, 50);
    assert_eq!(data.planets[0].build_queue[0].kind.code(), 9);
    assert_eq!(data.planets[0].build_queue[1].points_owed, 50);
    assert_eq!(data.planets[0].build_queue[1].kind.code(), 9);
    assert_eq!(data.planets[0].build_queue[2].points_owed, 50);
    assert_eq!(data.planets[0].build_queue[2].kind.code(), 9);
    assert!((3..10).all(|slot| data.planets[0].build_queue[slot].points_owed == 0));
}

#[test]
fn commission_ship_reuses_lowest_available_owned_fleet_number() {
    let mut player = Player {
        mode: PlayerMode::Human,
        ..Player::default()
    };
    player.fleets_first.0 = 1;

    let mut planet = Planet::default();
    planet.owner.0 = 1;
    planet.sector = Coords::new(6, 5);
    planet.stardock.destroyers = 1;

    let mut fleet_a = Fleet::default();
    fleet_a.owner.0 = 1;
    fleet_a.record.0 = 1;
    fleet_a.fleet_number.0 = 1;
    fleet_a.next.0 = 2;

    let mut fleet_b = Fleet::default();
    fleet_b.owner.0 = 1;
    fleet_b.record.0 = 2;
    fleet_b.fleet_number.0 = 3;
    fleet_b.prev.0 = 1;

    let mut data = CoreGameData {
        players: vec![player],
        planets: vec![planet],
        fleets: vec![fleet_a, fleet_b],
        bases: vec![],
        ipbms: vec![],
        setup: Setup::default(),
        conquest: Conquest::default(),
    };

    // ECGAME appends fleet #2 as record 3 (id 3), then sorts the chain by
    // fleet number: #2 moves into record 2, #3 into record 3, and the
    // next/id/previous words stay with their records.
    let result = data.commission_planet_stardock_slot(1, 1, 0).unwrap();
    assert_eq!(
        result,
        CommissionResult::Fleet {
            fleet_record_index_1_based: 2
        }
    );
    assert_eq!(data.players[0].fleets_first.0, 1);
    assert_eq!(data.fleets[0].next.0, 2);
    let fleet = &data.fleets[1];
    assert_eq!(fleet.fleet_number.0, 2);
    assert_eq!(fleet.ships.destroyers, 1);
    assert_eq!(fleet.record.0, 2);
    assert_eq!(fleet.prev.0, 1);
    assert_eq!(fleet.next.0, 3);
    let moved = &data.fleets[2];
    assert_eq!(moved.fleet_number.0, 3);
    assert_eq!(moved.record.0, 3);
    assert_eq!(moved.prev.0, 2);
    assert_eq!(moved.next.0, 0);
}

#[test]
fn commission_ship_draft_can_split_selected_stardock_rows() {
    let player = Player {
        mode: PlayerMode::Human,
        ..Player::default()
    };

    let mut planet = Planet::default();
    planet.owner.0 = 1;
    planet.sector = Coords::new(6, 5);
    planet.stardock.destroyers = 5;
    planet.stardock.battleships = 3;

    let mut data = CoreGameData {
        players: vec![player],
        planets: vec![planet],
        fleets: vec![],
        bases: vec![],
        ipbms: vec![],
        setup: Setup::default(),
        conquest: Conquest::default(),
    };

    let result = data
        .commission_planet_stardock_slots_with_draft(
            1,
            1,
            &[0, 2],
            CommissionFleetDraft {
                destroyers: 2,
                battleships: 1,
                ..CommissionFleetDraft::default()
            },
        )
        .unwrap();
    assert_eq!(
        result,
        CommissionResult::Fleet {
            fleet_record_index_1_based: 1
        }
    );
    assert_eq!(data.planets[0].stardock.destroyers, 3);
    assert_eq!(data.planets[0].stardock.battleships, 2);
    let fleet = &data.fleets[0];
    assert_eq!(fleet.ships.destroyers, 2);
    assert_eq!(fleet.ships.battleships, 1);
    assert_eq!(fleet.sector.to_array(), [6, 5]);
}

#[test]
fn commission_starbase_from_stardock_appends_base_and_clears_slot() {
    let player = Player {
        mode: PlayerMode::Human,
        ..Player::default()
    };

    let mut planet = Planet::default();
    planet.owner.0 = 1;
    planet.sector = Coords::new(6, 5);
    planet.stardock.starbases = 1;

    let mut data = CoreGameData {
        players: vec![player],
        planets: vec![planet],
        fleets: vec![],
        bases: vec![],
        ipbms: vec![],
        setup: Setup::default(),
        conquest: Conquest::default(),
    };

    let result = data.commission_planet_stardock_slot(1, 1, 6).unwrap();
    assert_eq!(
        result,
        CommissionResult::Starbase {
            base_record_index_1_based: 1
        }
    );
    assert_eq!(stardock_slot_kind(&data.planets[0], 6).code(), 0);
    assert_eq!(data.planets[0].stardock.starbases, 0);
    assert_eq!(data.players[0].bases_first.0, 1);
    assert_eq!(data.bases.len(), 1);
    let base = &data.bases[0];
    assert_eq!(base.base_id.0, 1);
    assert_eq!(base.sector.to_array(), [6, 5]);
    assert_eq!(base.owner.0, 1);
    // A new base as ECMAINT 2000:6642 creates one: no escort, at rest in
    // orbit, target = its own sector.
    assert_eq!(
        encode_base(base),
        [
            0, 0, 1, 0, 1, 0, 0, 1, 0, 0, 0, 6, 5, 0x80, 0, 0, 0, 0, 0, 0x80, 0, 0, 0, 0, 0, 0x81,
            0, 0, 0, 0, 0, 0, 6, 5, 1,
        ]
    );

    // A second base takes the next id and joins the end of the chain.
    data.planets[0].stardock.starbases = 1;
    data.commission_planet_stardock_slot(1, 1, 6).unwrap();
    let [first, second] = &data.bases[..] else {
        panic!("expected two bases");
    };
    assert_eq!(first.next.0, 2);
    assert_eq!(second.base_id.0, 2);
    assert_eq!(second.record.0, 2);
    assert_eq!(second.prev.0, 1);
    assert_eq!(second.next.0, 0);
}

#[test]
fn commission_rejects_foreign_owned_planet() {
    let player = Player {
        mode: PlayerMode::Human,
        ..Player::default()
    };

    let mut planet = Planet::default();
    planet.owner.0 = 2;
    planet.sector = Coords::new(6, 5);
    planet.stardock.destroyers = 2;

    let mut data = CoreGameData {
        players: vec![player],
        planets: vec![planet],
        fleets: vec![],
        bases: vec![],
        ipbms: vec![],
        setup: Setup::default(),
        conquest: Conquest::default(),
    };

    let err = data.commission_planet_stardock_slot(1, 1, 0).unwrap_err();
    assert!(matches!(
        err,
        GameStateMutationError::PlanetOwnershipMismatch {
            player_index_1_based: 1,
            planet_index_1_based: 1,
        }
    ));
}

#[test]
fn auto_commission_all_stardock_units_creates_fleets_and_bases() {
    let player = Player {
        mode: PlayerMode::Human,
        ..Player::default()
    };

    let mut planet_one = Planet::default();
    planet_one.owner.0 = 1;
    set_planet_name(&mut planet_one, "Aurora Prime");
    planet_one.sector = Coords::new(6, 5);
    planet_one.stardock.destroyers = 2;
    planet_one.stardock.scouts = 3;
    planet_one.stardock.starbases = 1;

    let mut planet_two = Planet::default();
    planet_two.owner.0 = 1;
    set_planet_name(&mut planet_two, "Bastion Reach");
    planet_two.sector = Coords::new(7, 6);
    planet_two.stardock.cruisers = 1;

    let mut planet_three = Planet::default();
    planet_three.owner.0 = 1;
    set_planet_name(&mut planet_three, "Signal Keep");
    planet_three.sector = Coords::new(8, 7);
    planet_three.stardock.starbases = 1;

    let mut data = CoreGameData {
        players: vec![player],
        planets: vec![planet_one, planet_two, planet_three],
        fleets: vec![],
        bases: vec![],
        ipbms: vec![],
        setup: Setup::default(),
        conquest: Conquest::default(),
    };

    let report = data.auto_commission_all_stardock_units(1).unwrap();
    assert_eq!(
        report,
        AutoCommissionReport {
            ships_commissioned: 6,
            starbases_commissioned: 2,
            planets_used: 3,
            fleets_created: 2,
            entries: vec![
                AutoCommissionEntry::Fleet(AutoCommissionFleetEntry {
                    fleet_number: 1,
                    planet_name: "Aurora Prime".to_string(),
                    coords: [6, 5],
                    destroyers: 2,
                    cruisers: 0,
                    battleships: 0,
                    scouts: 3,
                    transports: 0,
                    etacs: 0,
                }),
                AutoCommissionEntry::Starbase(AutoCommissionStarbaseEntry {
                    starbase_number: 1,
                    planet_name: "Aurora Prime".to_string(),
                    coords: [6, 5],
                }),
                AutoCommissionEntry::Fleet(AutoCommissionFleetEntry {
                    fleet_number: 2,
                    planet_name: "Bastion Reach".to_string(),
                    coords: [7, 6],
                    destroyers: 0,
                    cruisers: 1,
                    battleships: 0,
                    scouts: 0,
                    transports: 0,
                    etacs: 0,
                }),
                AutoCommissionEntry::Starbase(AutoCommissionStarbaseEntry {
                    starbase_number: 2,
                    planet_name: "Signal Keep".to_string(),
                    coords: [8, 7],
                }),
            ],
        }
    );
    assert_eq!(data.fleets.len(), 2);
    assert_eq!(stardock_slot_kind(&data.planets[0], 0).code(), 0);
    assert_eq!(stardock_slot_kind(&data.planets[0], 1).code(), 0);
    assert_eq!(stardock_slot_kind(&data.planets[0], 2).code(), 0);
    assert_eq!(data.planets[0].stardock.battleships, 0);
    assert_eq!(stardock_slot_kind(&data.planets[1], 0).code(), 0);
    assert_eq!(stardock_slot_kind(&data.planets[2], 0).code(), 0);
    assert_eq!(data.planets[2].stardock.destroyers, 0);
    assert_eq!(data.bases.len(), 2);
}

#[test]
fn load_planet_armies_onto_fleet_moves_armies_into_transports() {
    let player = Player {
        mode: PlayerMode::Human,
        ..Player::default()
    };

    let mut planet = Planet::default();
    planet.owner.0 = 1;
    planet.sector = Coords::new(6, 5);
    planet.armies = 10;

    let mut fleet = Fleet::default();
    fleet.owner.0 = 1;
    fleet.sector = Coords::new(6, 5);
    set_troop_transports(&mut fleet, 4);
    set_armies_aboard(&mut fleet, 1);

    let mut data = CoreGameData {
        players: vec![player],
        planets: vec![planet],
        fleets: vec![fleet],
        bases: vec![],
        ipbms: vec![],
        setup: Setup::default(),
        conquest: Conquest::default(),
    };

    data.load_planet_armies_onto_fleet(1, 1, 1, 3).unwrap();
    assert_eq!(data.planets[0].armies, 7);
    assert_eq!(data.fleets[0].ships.loaded_transports, 4);
    // Each loaded army moves one transport from 0x2c (empty) to 0x2e (loaded).
    assert_eq!(data.fleets[0].ships.empty_transports, 0);
    assert_eq!(data.fleets[0].ships.loaded_transports, 4);
    assert_eq!(troop_transports(&data.fleets[0]), 4);
}

#[test]
fn unload_fleet_armies_to_planet_moves_armies_back_to_planet() {
    let player = Player {
        mode: PlayerMode::Human,
        ..Player::default()
    };

    let mut planet = Planet::default();
    planet.owner.0 = 1;
    planet.sector = Coords::new(6, 5);
    planet.armies = 4;

    let mut fleet = Fleet::default();
    fleet.owner.0 = 1;
    fleet.sector = Coords::new(6, 5);
    set_troop_transports(&mut fleet, 4);
    set_armies_aboard(&mut fleet, 3);

    let mut data = CoreGameData {
        players: vec![player],
        planets: vec![planet],
        fleets: vec![fleet],
        bases: vec![],
        ipbms: vec![],
        setup: Setup::default(),
        conquest: Conquest::default(),
    };

    data.unload_fleet_armies_to_planet(1, 1, 1, 2).unwrap();
    assert_eq!(data.planets[0].armies, 6);
    assert_eq!(data.fleets[0].ships.loaded_transports, 1);
    // Each unloaded army moves one transport from 0x2e back to 0x2c.
    assert_eq!(data.fleets[0].ships.empty_transports, 3);
    assert_eq!(data.fleets[0].ships.loaded_transports, 1);
    assert_eq!(troop_transports(&data.fleets[0]), 4);
}

#[test]
fn unload_fleet_armies_to_planet_rejects_planet_overflow() {
    let player = Player {
        mode: PlayerMode::Human,
        ..Player::default()
    };

    let mut planet = Planet::default();
    planet.owner.0 = 1;
    planet.sector = Coords::new(6, 5);
    planet.armies = 255;

    let mut fleet = Fleet::default();
    fleet.owner.0 = 1;
    fleet.sector = Coords::new(6, 5);
    set_troop_transports(&mut fleet, 4);
    set_armies_aboard(&mut fleet, 3);

    let mut data = CoreGameData {
        players: vec![player],
        planets: vec![planet],
        fleets: vec![fleet],
        bases: vec![],
        ipbms: vec![],
        setup: Setup::default(),
        conquest: Conquest::default(),
    };

    let err = data.unload_fleet_armies_to_planet(1, 1, 1, 1).unwrap_err();
    assert_eq!(
        err,
        nc_data::GameStateMutationError::PlanetArmyCapacityExceeded {
            planet_index_1_based: 1,
            requested: 1,
            available: 0,
        }
    );
}

#[test]
fn ipbm_record_setters_round_trip_chain_fields() {
    let mut record = Ipbm {
        ipbm_number: 0x1234,
        ..Ipbm::default()
    };
    record.owner.0 = 0x02;
    record.next.0 = 0x4567;
    record.record.0 = 0x89ab;

    assert_eq!(record.ipbm_number, 0x1234);
    assert_eq!(record.owner.0, 0x02);
    assert_eq!(record.next.0, 0x4567);
    assert_eq!(record.record.0, 0x89ab);
    let bytes = encode_ipbm(&record);
    assert_eq!(&bytes[..7], &[0x34, 0x12, 0x02, 0x67, 0x45, 0xab, 0x89]);
}

#[test]
fn ipbm_record_setters_round_trip_position_and_target() {
    let record = Ipbm {
        sector: Coords::new(0x11, 0x22),
        x_fraction: Real48::from_bytes([1, 2, 3, 4, 5, 6]),
        y_fraction: Real48::from_bytes([7, 8, 9, 10, 11, 12]),
        depth: Real48::from_bytes([13, 14, 15, 16, 17, 18]),
        target: Coords::new(0xAA, 0xBB),
        target_depth: 0xCC,
        ..Ipbm::default()
    };

    assert_eq!(record.sector.to_array(), [0x11, 0x22]);
    assert_eq!(record.target.to_array(), [0xAA, 0xBB]);
    assert_eq!(record.target_depth, 0xCC);
    let bytes = encode_ipbm(&record);
    assert_eq!(&bytes[0x09..0x0B], &[0x11, 0x22]);
    assert_eq!(&bytes[0x0B..0x11], &[1, 2, 3, 4, 5, 6]);
    assert_eq!(&bytes[0x11..0x17], &[7, 8, 9, 10, 11, 12]);
    assert_eq!(&bytes[0x17..0x1D], &[13, 14, 15, 16, 17, 18]);
    assert_eq!(&bytes[0x1D..0x20], &[0xAA, 0xBB, 0xCC]);
}

#[test]
fn can_set_maintenance_schedule_from_enabled_days() {
    let mut post_maint = parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap();
    post_maint.maintenance_days.0 = [true, false, true, false, true, false, true];
    // ECUTIL writes 0 or 1; ECMAINT tests only for nonzero.
    assert_eq!(
        post_maint.maintenance_days.0.map(u8::from),
        [0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01]
    );
    assert_eq!(
        post_maint.maintenance_days.0,
        [true, false, true, false, true, false, true]
    );
}

#[test]
fn can_toggle_snoop_enabled() {
    let mut setup = parse_setup_dat(&read_fixture("SETUP.DAT")).unwrap();
    assert!(setup.snoop);
    setup.snoop = false;
    assert!(!setup.snoop);
    assert_eq!(encode_setup(&setup)[512], 0);
}

#[test]
fn can_set_other_setup_program_fields() {
    let mut setup = parse_setup_dat(&read_fixture("SETUP.DAT")).unwrap();
    setup.com_hw_flow_control = [false; 4];
    setup.max_key_gap_minutes = 15;
    setup.remote_timeout = false;
    setup.local_timeout = true;
    setup.minimum_time_minutes = 69;
    setup.purge_after_turns = 10;
    setup.autopilot_after_turns = 3;

    assert_eq!(setup.max_key_gap_minutes, 15);
    assert!(!setup.remote_timeout);
    assert!(setup.local_timeout);
    assert_eq!(setup.minimum_time_minutes, 69);
    assert_eq!(setup.purge_after_turns, 10);
    assert_eq!(setup.autopilot_after_turns, 3);
    assert_eq!(setup.com_hw_flow_control.first().copied(), Some(false));
    assert_eq!(setup.com_hw_flow_control.get(1).copied(), Some(false));
    assert_eq!(setup.com_hw_flow_control.get(2).copied(), Some(false));
    assert_eq!(setup.com_hw_flow_control.get(3).copied(), Some(false));
}

#[test]
fn can_set_purge_after_turns() {
    let mut setup = parse_setup_dat(&read_fixture("SETUP.DAT")).unwrap();
    assert_eq!(setup.purge_after_turns, 0);
    setup.purge_after_turns = 1;
    assert_eq!(setup.purge_after_turns, 1);
    assert_eq!(encode_setup(&setup)[518], 1);
}

#[test]
fn planet_status_prefix_setter_stores_the_name() {
    let mut record = Planet::default();
    set_planet_name(&mut record, "Unowned");

    assert_eq!(record.name.chars().count() as u8, 7);
    assert_eq!(record.name.trim().to_string(), "Unowned");
    assert_eq!(&encode_planet(&record)[0x0F..0x17], b"\x07Unowned");
}

#[test]
fn core_game_data_current_known_count_helpers_follow_player1_and_records() {
    let mut base1 = Starbase::default();
    base1.owner.0 = 1;
    base1.base_id.0 = 1;
    let mut base2 = Starbase::default();
    base2.owner.0 = 1;
    base2.base_id.0 = 2;
    let mut base3 = Starbase::default();
    base3.owner.0 = 2;
    base3.base_id.0 = 1;
    let mut data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: vec![base1, base2, base3],
        ipbms: vec![Ipbm::default(), Ipbm::default(), Ipbm::default()],
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };
    assert_eq!(data.player1_starbase_count_current_known(), 0);
    assert_eq!(
        data.player_owned_planet_counts_current_known(),
        vec![1, 1, 1, 1]
    );
    assert_eq!(
        data.player_homeworld_seed_coords_current_known(),
        vec![Some([16, 13]), Some([4, 13]), Some([6, 5]), Some([13, 5])]
    );
    assert_eq!(data.player1_owned_base_record_count_current_known(), 2);
    assert_eq!(
        data.player_owned_base_record_counts_current_known(),
        vec![2, 1, 0, 0]
    );
    assert_eq!(data.player1_ipbm_count_current_known(), 0);
    let initial_errors = data.current_known_core_state_errors();
    assert_eq!(initial_errors.len(), 4);
    assert!(
        initial_errors
            .contains(&"PLAYER[1]-owned BASES.DAT record count expected 0, got 2".to_string())
    );
    assert!(initial_errors.contains(&"IPBM.DAT record count expected 0, got 3".to_string()));
    assert!(
        initial_errors
            .contains(&"BASES.DAT expected empty auxiliary baseline, got 3 records".to_string())
    );
    assert!(
        initial_errors
            .contains(&"IPBM.DAT expected empty auxiliary baseline, got 3 records".to_string())
    );

    let player2_starbase_before = data.players[1].bases_first.0;
    let player3_starbase_before = data.players[2].bases_first.0;
    let player4_starbase_before = data.players[3].bases_first.0;
    data.sync_player1_current_known_counts();

    // Player 1's chain now runs from record 1 (Starbase 1) to record 2.
    assert_eq!(data.players[0].bases_first.0, 1);
    assert_eq!(data.bases[0].next.0, 2);
    assert_eq!(data.bases[1].prev.0, 1);
    assert_eq!(data.players[1].bases_first.0, player2_starbase_before);
    assert_eq!(data.players[2].bases_first.0, player3_starbase_before);
    assert_eq!(data.players[3].bases_first.0, player4_starbase_before);
    assert_eq!(data.players[0].ipbms_first.0, 3);
    assert_eq!(data.player1_starbase_count_current_known(), 2);
    assert_eq!(
        data.player_owned_planet_counts_current_known(),
        vec![1, 1, 1, 1]
    );
    assert_eq!(data.player1_owned_base_record_count_current_known(), 2);
    assert_eq!(
        data.player_owned_base_record_counts_current_known(),
        vec![2, 1, 0, 0]
    );
    assert_eq!(data.player1_ipbm_count_current_known(), 3);
    let post_sync_errors = data.current_known_core_state_errors();
    assert_eq!(
        post_sync_errors,
        vec![
            "BASES.DAT expected empty auxiliary baseline, got 3 records".to_string(),
            "IPBM.DAT expected empty auxiliary baseline, got 3 records".to_string(),
        ]
    );
}

#[test]
fn core_game_data_initialized_fleet_block_helpers_match_known_fixtures() {
    let data = CoreGameData {
        players: parse_player_dat(&read_initialized_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_initialized_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_initialized_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_initialized_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_initialized_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_initialized_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_initialized_fixture("CONQUEST.DAT")).unwrap(),
    };

    assert!(data.looks_like_initialized_fleet_blocks_current_known());
    assert_eq!(
        data.current_known_initialized_fleet_block_head_ids(),
        vec![1, 5, 9, 13]
    );
    assert!(
        data.current_known_initialized_fleet_block_errors()
            .is_empty()
    );
    assert!(
        data.current_known_initialized_fleet_payload_errors()
            .is_empty()
    );
    assert!(
        data.current_known_initialized_fleet_mission_errors()
            .is_empty()
    );
    assert!(data.current_known_homeworld_seed_errors().is_empty());
    assert!(
        data.current_known_initialized_planet_ownership_errors()
            .is_empty()
    );
    assert!(
        data.current_known_homeworld_seed_payload_errors()
            .is_empty()
    );
    assert!(
        data.current_known_unowned_planet_payload_errors()
            .is_empty()
    );
    assert!(data.current_known_empty_auxiliary_state_errors().is_empty());
    assert!(
        data.current_known_initialized_homeworld_alignment_errors()
            .is_empty()
    );
    assert!(data.current_known_setup_baseline_errors().is_empty());
    assert!(data.current_known_conquest_baseline_errors().is_empty());
}

#[test]
fn core_game_data_initialized_fleet_block_errors_catch_broken_local_chain() {
    let mut data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.fleets[1].next.0 = 9;

    assert_eq!(
        data.current_known_initialized_fleet_block_errors(),
        vec!["FLEET[2].next_fleet_id expected 3, got 9".to_string()]
    );
}

#[test]
fn core_game_data_initialized_fleet_payload_errors_catch_broken_slot_pattern() {
    let mut data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.fleets[2].max_speed = 3;

    assert_eq!(
        data.current_known_initialized_fleet_payload_errors(),
        vec!["FLEET[3].max_speed expected 6, got 3".to_string()]
    );
}

#[test]
fn core_game_data_initialized_fleet_payload_errors_catch_missing_tuple_markers() {
    let mut data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.fleets[0].x_fraction = Real48::ZERO;

    assert_eq!(
        data.current_known_initialized_fleet_payload_errors(),
        vec!["FLEET[1].x_fraction expected 0.5, got [0, 0, 0, 0, 0, 0]".to_string()]
    );
}

#[test]
fn core_game_data_initialized_fleet_mission_errors_catch_wrong_order_code() {
    let mut data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.fleets[0].order = crate::Order::GuardStarbase;

    assert_eq!(
        data.current_known_initialized_fleet_mission_errors(),
        vec!["FLEET[1].standing_order expected 5 for initialized baseline, got 4".to_string()]
    );
}

#[test]
fn core_game_data_initialized_fleet_mission_errors_catch_wrong_aux_bytes() {
    let mut data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    set_mission_target(&mut data.fleets[0], 0, 1);

    assert_eq!(
        data.current_known_initialized_fleet_mission_errors(),
        vec![
            "FLEET[1].mission_aux expected [1, 0] for initialized baseline, got [0, 1]".to_string()
        ]
    );
}

#[test]
fn core_game_data_owner_range_errors_catch_invalid_planet_and_base_owners() {
    let mut data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: vec![Starbase::default()],
        ipbms: parse_ipbm_dat(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.planets[0].owner.0 = 9;
    data.bases[0].base_id.0 = 1;
    data.bases[0].owner.0 = 0;

    assert_eq!(
        data.current_known_planet_owner_slot_errors(),
        vec!["PLANET[1].owner_empire_slot expected <= 4, got 9".to_string()]
    );
    assert_eq!(
        data.current_known_base_owner_empire_errors(),
        vec!["BASES[1].owner_empire expected 1..=4, got 0".to_string()]
    );

    data.bases[0].base_id.0 = 0;
    assert!(data.current_known_base_owner_empire_errors().is_empty());
}

#[test]
fn core_game_data_homeworld_alignment_errors_catch_misaligned_fleet_block() {
    let mut data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    let [_, target_y] = data.fleets[4].target.to_array();
    data.fleets[4].target = Coords::new(9, target_y);

    assert_eq!(
        data.current_known_initialized_homeworld_alignment_errors(),
        vec!["FLEET block 2 target expected homeworld seed [4, 13], got [9, 13]".to_string()]
    );
}

#[test]
fn core_game_data_initialized_planet_ownership_errors_catch_non_homeworld_owner() {
    let mut data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.planets[0].owner.0 = 2;

    assert_eq!(
        data.current_known_initialized_planet_ownership_errors(),
        vec![
            "PLANET[1] expected unowned non-homeworld baseline, got owner 2".to_string(),
            "PLAYER[2] owned_planet_count expected 1, got 2".to_string(),
        ]
    );
}

#[test]
fn core_game_data_homeworld_seed_payload_errors_catch_changed_army_marker() {
    let mut data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.planets[14].ground_batteries = 1;

    assert_eq!(
        data.current_known_homeworld_seed_payload_errors(),
        vec!["PLANET[15].ground_batteries_raw expected 4 for homeworld seed, got 1".to_string()]
    );
}

#[test]
fn core_game_data_homeworld_seed_payload_errors_catch_changed_stored_goods() {
    let mut data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.planets[14].stored_goods = Real48::from_f64(34.75);

    assert_eq!(
        data.current_known_homeworld_seed_payload_errors(),
        vec!["PLANET[15].stored_goods expected 35 for homeworld seed, got 34.75".to_string()]
    );
}

#[test]
fn core_game_data_unowned_planet_payload_errors_catch_owned_marker() {
    let mut data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.planets[0].ground_batteries = 1;

    assert_eq!(
        data.current_known_unowned_planet_payload_errors(),
        vec!["PLANET[1].ground_batteries_raw expected 0 for unowned baseline, got 1".to_string()]
    );
}

#[test]
fn core_game_data_unowned_planet_payload_errors_catch_nonzero_stored_goods() {
    let mut data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.planets[0].stored_goods = Real48::from_f64(1.0);

    assert_eq!(
        data.current_known_unowned_planet_payload_errors(),
        vec!["PLANET[1].stored_goods expected 0 for unowned baseline, got 1".to_string()]
    );
}

#[test]
fn core_game_data_empty_auxiliary_state_errors_catch_starbase_record() {
    let mut data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.bases.push(Starbase::default());

    assert_eq!(
        data.current_known_empty_auxiliary_state_errors(),
        vec!["BASES.DAT expected empty auxiliary baseline, got 1 records".to_string()]
    );
}

#[test]
fn core_game_data_setup_baseline_errors_catch_changed_timeout_flag() {
    let mut data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.setup.remote_timeout = false;

    assert_eq!(
        data.current_known_setup_baseline_errors(),
        vec!["SETUP.DAT.remote_timeout expected enabled in baseline".to_string()]
    );
}

#[test]
fn core_game_data_conquest_baseline_errors_catch_changed_year() {
    let mut data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.conquest.game_year = 2999;

    assert_eq!(
        data.current_known_conquest_baseline_errors(),
        vec!["CONQUEST.DAT.game_year expected 3000 or 3001 for preserved initialized/post-maint baseline, got 2999".to_string()]
    );
}

#[test]
fn core_game_data_sync_current_known_baseline_controls_and_counts_repairs_mutated_fields() {
    let mut base1 = Starbase::default();
    base1.owner.0 = 1;
    base1.base_id.0 = 1;
    let mut data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: vec![base1],
        ipbms: vec![Ipbm::default()],
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.players[0].bases_first.0 = 9;
    data.players[0].ipbms_first.0 = 9;
    data.setup.version_tag = *b"BAD!!";
    data.setup.remote_timeout = false;
    data.conquest.game_year = 2999;
    data.conquest.player_count = 9;
    data.conquest.maintenance_days.0 = [false; 7];

    data.sync_current_known_baseline_controls_and_counts();

    assert_eq!(data.players[0].bases_first.0, 1);
    assert_eq!(data.players[0].ipbms_first.0, 1);
    assert!(data.current_known_setup_baseline_errors().is_empty());
    assert!(data.current_known_conquest_baseline_errors().is_empty());
}

#[test]
fn core_game_data_sync_current_known_initialized_fleet_baseline_repairs_mutated_fleets() {
    let mut data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.fleets.clear();
    data.fleets.push(Fleet::default());

    data.sync_current_known_initialized_fleet_baseline();

    assert!(data.looks_like_initialized_fleet_blocks_current_known());
    assert!(
        data.current_known_initialized_fleet_payload_errors()
            .is_empty()
    );
    assert!(
        data.current_known_initialized_fleet_mission_errors()
            .is_empty()
    );
    assert!(
        data.current_known_initialized_homeworld_alignment_errors()
            .is_empty()
    );
}

#[test]
fn core_game_data_sync_current_known_initialized_planet_payloads_repairs_mutated_planets() {
    let mut data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.planets[14].stored_goods = Real48::from_f64(3.0);
    set_planet_name(&mut data.planets[0], "Broken");
    data.planets[0].armies = 9;

    data.sync_current_known_initialized_planet_payloads();

    assert!(
        data.current_known_homeworld_seed_payload_errors()
            .is_empty()
    );
    assert!(
        data.current_known_unowned_planet_payload_errors()
            .is_empty()
    );
}

#[test]
fn core_game_data_sync_current_known_initialized_post_maint_baseline_repairs_combined_state() {
    let mut data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.players[0].bases_first.0 = 7;
    data.players[0].ipbms_first.0 = 4;
    data.setup.version_tag = *b"BAD!!";
    data.bases.push(Starbase::default());
    data.ipbms.push(Ipbm::default());
    data.fleets.clear();
    data.fleets.push(Fleet::default());
    data.planets[14].stored_goods = Real48::from_f64(3.0);
    set_planet_name(&mut data.planets[0], "Broken");

    data.sync_current_known_initialized_post_maint_baseline();

    assert!(data.current_known_setup_baseline_errors().is_empty());
    assert!(data.current_known_conquest_baseline_errors().is_empty());
    assert!(
        data.current_known_initialized_fleet_block_errors()
            .is_empty()
    );
    assert!(
        data.current_known_initialized_fleet_payload_errors()
            .is_empty()
    );
    assert!(
        data.current_known_initialized_fleet_mission_errors()
            .is_empty()
    );
    assert!(
        data.current_known_initialized_homeworld_alignment_errors()
            .is_empty()
    );
    assert!(
        data.current_known_homeworld_seed_payload_errors()
            .is_empty()
    );
    assert!(
        data.current_known_unowned_planet_payload_errors()
            .is_empty()
    );
    assert!(data.current_known_empty_auxiliary_state_errors().is_empty());
}

#[test]
fn core_game_data_can_apply_current_known_scenario_mutations() {
    let mut data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    let aux = data
        .set_fleet_order(1, 0x03, 0x0C, [0x0F, 0x0D], None, None)
        .unwrap();
    assert_eq!(data.fleets[0].speed, 0x03);
    assert_eq!(data.fleets[0].order.code(), 0x0C);
    assert_eq!(data.fleets[0].target.to_array(), [0x0F, 0x0D]);
    assert_eq!(
        aux,
        [
            data.fleets[0].target_depth,
            data.fleets[0].mission_target as u8
        ]
    );

    data.set_planet_build(15, 0x03, 0x01).unwrap();
    assert_eq!(data.planets[14].build_queue[0].points_owed, 0x03);
    assert_eq!(data.planets[14].build_queue[0].kind.code(), 0x01);

    data.clear_planet_build_queue(15).unwrap();
    assert!((0..10).all(|slot| data.planets[14].build_queue[slot].points_owed == 0));
    assert!((0..10).all(|slot| data.planets[14].build_queue[slot].kind.code() == 0));

    data.replace_planet_build_queue_with_single_order(15, 0x05, 0x08)
        .unwrap();
    assert_eq!(data.planets[14].build_queue[0].points_owed, 0x05);
    assert_eq!(data.planets[14].build_queue[0].kind.code(), 0x08);
    assert!((1..10).all(|slot| data.planets[14].build_queue[slot].points_owed == 0));
    assert!((1..10).all(|slot| data.planets[14].build_queue[slot].kind.code() == 0));

    data.set_guard_starbase(1, 1, [0x10, 0x0D], 1, 1).unwrap();
    assert_eq!(data.players[0].bases_first.0, 1);
    assert_eq!(data.fleets[0].order.code(), 0x04);
    assert_eq!(data.fleets[0].target.to_array(), [0x10, 0x0D]);
    assert_eq!(
        [
            data.fleets[0].target_depth,
            data.fleets[0].mission_target as u8
        ],
        [0x01, 0x01]
    );
    assert_eq!(data.bases.len(), 1);
    assert_eq!(data.bases[0].escort.0, data.fleets[0].fleet_number.0);
    assert_eq!(data.bases[0].record.0, data.fleets[0].record.0);
    assert_eq!(data.bases[0].sector.to_array(), [0x10, 0x0D]);

    data.set_ipbm_zero_records(2);
    assert_eq!(data.players[0].ipbms_first.0, 2);
    assert_eq!(data.ipbms.len(), 2);
    assert_eq!(serialize_ipbm_dat(&data.ipbms).len(), 2 * IPBM_RECORD_SIZE);

    data.set_ipbm_record_prefix(2, 0x1234, 0x02, 0x4567, 0x89ab)
        .unwrap();
    let ipbm = &data.ipbms[1];
    assert_eq!(ipbm.ipbm_number, 0x1234);
    assert_eq!(ipbm.owner.0, 0x02);
    assert_eq!(ipbm.next.0, 0x4567);
    assert_eq!(ipbm.record.0, 0x89ab);
}

#[test]
fn set_fleet_order_rejects_speed_above_current_maximum() {
    let mut data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    let max_speed = data.fleets[0].max_speed;
    let error = data
        .set_fleet_order(
            1,
            max_speed.saturating_add(1),
            Order::MoveOnly.code(),
            [0x0F, 0x0D],
            None,
            None,
        )
        .unwrap_err();

    assert_eq!(
        error,
        GameStateMutationError::InvalidFleetSpeed {
            fleet_index_1_based: 1,
            requested: max_speed.saturating_add(1),
            max: max_speed,
        }
    );
}

#[test]
fn set_fleet_order_rejects_duplicate_friendly_colonize_target() {
    let mut data = GameStateBuilder::new()
        .with_player_count(4)
        .build_initialized_baseline()
        .expect("baseline should build");
    let target = data
        .planets
        .iter()
        .find(|planet| planet.owner.0 == 0)
        .map(|planet| planet.sector.to_array())
        .expect("baseline should have an unowned planet");
    let first_speed = data.fleets[0].speed;
    let second_speed = data.fleets[1].speed;
    data.fleets[0].ships.etacs = 1;
    data.fleets[1].ships.etacs = 1;

    data.set_fleet_order(
        1,
        first_speed,
        Order::ColonizeWorld.code(),
        target,
        None,
        None,
    )
    .expect("first colonize order should succeed");

    let error = data
        .set_fleet_order(
            2,
            second_speed,
            Order::ColonizeWorld.code(),
            target,
            None,
            None,
        )
        .expect_err("second colonize order should be rejected");

    assert_eq!(
        error,
        GameStateMutationError::InvalidFleetOrder {
            fleet_index_1_based: 2,
            reason: FleetOrderValidationError::DuplicateFriendlyColonizeTarget {
                target,
                conflicting_fleet_record_index_1_based: 1,
            },
        }
    );
}

fn configure_on_station_hostile_order_state() -> (CoreGameData, [u8; 2]) {
    let mut data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");
    let target = [25, 25];
    let target_world = &mut data.planets[4];
    target_world.sector = Coords::from_array(target);
    target_world.owner.0 = 2;
    target_world.years_under_owner = 2;
    set_planet_name(target_world, "Target");
    target_world.armies = 10;
    target_world.ground_batteries = 4;

    let fleet = &mut data.fleets[0];
    fleet.sector = Coords::from_array(target);
    fleet.ships.battleships = 1;
    fleet.ships.cruisers = 0;
    fleet.ships.destroyers = 1;
    set_troop_transports(fleet, 2);
    set_armies_aboard(fleet, 6);
    fleet.ships.scouts = 0;
    fleet.ships.etacs = 0;
    recompute_max_speed(fleet);
    fleet.speed = fleet.max_speed;
    fleet.depth = Real48::ONE;

    (data, target)
}

#[test]
fn set_fleet_order_keeps_on_station_hostile_fleet_motion() {
    for order in [Order::BombardWorld, Order::InvadeWorld, Order::BlitzWorld] {
        let (mut data, target) = configure_on_station_hostile_order_state();

        data.set_fleet_order(
            1,
            data.fleets[0].max_speed,
            order.code(),
            target,
            None,
            None,
        )
        .expect("hostile order should be accepted");

        let fleet = &data.fleets[0];
        assert_eq!(fleet.order, order);
        assert_eq!(fleet.target.to_array(), target);
        assert_eq!(fleet.sector.to_array(), target);
        // Like ECGAME, the order keeps the fleet's stored motion: at rest in
        // orbit, so its strike test passes on the next maintenance's first tick.
        assert_eq!(fleet.speed, fleet.max_speed);
        assert_eq!(
            fleet.depth,
            Real48::ONE,
            "{order:?} keeps its at-rest depth"
        );
    }
}

#[test]
fn scorch_planet_surface_clears_economy_queue_and_stardock_immediately() {
    let mut data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    let planet = &mut data.planets[14];
    planet.production = Real48::from_f64(42.0);
    planet.potential_production = 77;
    planet.stored_goods = Real48::from_f64(1234.0);
    planet.build_queue[0].points_owed = 5;
    planet.build_queue[0].kind = ProductionItemKind::Battleship;
    planet.stardock.transports = 2;
    planet.armies = 9;
    planet.ground_batteries = 4;

    data.scorch_planet_surface(15).unwrap();

    let planet = &data.planets[14];
    assert_eq!(present_production_points(planet), 0);
    assert_eq!(u16::from(planet.potential_production), 0);
    assert_eq!(stored_production_points(planet), 0);
    assert!((0..10).all(|slot| planet.build_queue[slot].points_owed == 0));
    assert!((0..10).all(|slot| planet.build_queue[slot].kind.code() == 0));
    assert!((0..STARDOCK_SLOT_COUNT).all(|slot| planet.stardock[slot] == 0));
    assert!((0..STARDOCK_SLOT_COUNT).all(|slot| stardock_slot_kind(planet, slot).code() == 0));
    assert_eq!(planet.armies, 9);
    assert_eq!(planet.ground_batteries, 4);
}

#[test]
fn set_fleet_rules_of_engagement_rejects_non_combat_value() {
    let mut data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };
    let fleet = &mut data.fleets[0];
    fleet.ships.destroyers = 0;
    fleet.ships.cruisers = 0;
    fleet.ships.battleships = 0;
    fleet.ships.scouts = 1;
    set_troop_transports(fleet, 0);
    set_armies_aboard(fleet, 0);
    fleet.ships.etacs = 0;

    let error = data.set_fleet_rules_of_engagement(1, 1, 6).unwrap_err();
    assert_eq!(
        error,
        GameStateMutationError::InvalidFleetPlayerInput {
            fleet_index_1_based: 1,
            reason: FleetPlayerInputValidationError::NonCombatFleetMustUseZeroRoe { roe: 6 },
        }
    );
}

#[test]
fn set_fleet_rules_of_engagement_allows_mixed_fleet_value() {
    let mut data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };
    let fleet = &mut data.fleets[0];
    fleet.ships.destroyers = 1;
    fleet.ships.cruisers = 0;
    fleet.ships.battleships = 0;
    fleet.ships.scouts = 1;
    set_troop_transports(fleet, 0);
    set_armies_aboard(fleet, 0);
    fleet.ships.etacs = 0;

    data.set_fleet_rules_of_engagement(1, 1, 6).unwrap();

    assert_eq!(data.fleets[0].rules_of_engagement, 6);
}

#[test]
fn set_stored_diplomatic_relation_rejects_self_target() {
    let mut data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    let error = data
        .set_stored_diplomatic_relation(1, 1, DiplomaticRelation::Enemy)
        .unwrap_err();
    assert_eq!(
        error,
        GameStateMutationError::InvalidDiplomacyInput {
            player_index_1_based: 1,
            reason: PlayerDiplomacyValidationError::SelfTarget { empire: 1 },
        }
    );
}

#[test]
fn clear_planet_build_orders_by_kind_removes_matching_slots_only() {
    let mut data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };
    let planet = &mut data.planets[0];
    planet.build_queue[0].points_owed = 4;
    planet.build_queue[0].kind = ProductionItemKind::Destroyer;
    planet.build_queue[1].points_owed = 7;
    planet.build_queue[1].kind = ProductionItemKind::Destroyer;
    planet.build_queue[2].points_owed = 5;
    planet.build_queue[2].kind = ProductionItemKind::GroundBattery;

    let cleared = data
        .clear_planet_build_orders_by_kind(1, ProductionItemKind::Destroyer)
        .unwrap();

    assert_eq!(cleared, 2);
    assert_eq!(data.planets[0].build_queue[0].points_owed, 0);
    assert_eq!(data.planets[0].build_queue[0].kind.code(), 0);
    assert_eq!(data.planets[0].build_queue[1].points_owed, 0);
    assert_eq!(data.planets[0].build_queue[1].kind.code(), 0);
    assert_eq!(data.planets[0].build_queue[2].points_owed, 5);
    assert_eq!(
        ProductionItemKind::from_code(data.planets[0].build_queue[2].kind.code()),
        ProductionItemKind::GroundBattery
    );
}

#[test]
fn set_join_fleet_order_targets_host_fleet_and_records_host_id() {
    let mut data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    let host_coords = data.fleets[0].sector.to_array();
    let host_fleet_id = data.fleets[0].record.0;

    data.set_join_fleet_order(1, 2, 1).unwrap();

    assert_eq!(data.fleets[1].order, crate::Order::JoinAnotherFleet);
    assert_eq!(data.fleets[1].target.to_array(), host_coords);
    assert_eq!(data.fleets[1].mission_target, host_fleet_id);
}

#[test]
fn set_fleet_order_writes_mission_aux_bytes_as_ecgame_does() {
    let mut data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };
    data.set_guard_starbase(1, 1, [16, 13], 1, 1).unwrap();
    let base = data.bases[0].clone();
    let host_id = data.fleets[0].record.0 as u8;
    let owner = data.fleets[1].owner.0;
    let planet_coords = |predicate: &dyn Fn(u8) -> bool| {
        data.planets
            .iter()
            .find(|planet| predicate(planet.owner.0))
            .map(|planet| planet.sector.to_array())
            .expect("fixture planet")
    };
    let owned = planet_coords(&|planet_owner| planet_owner == owner);
    let foreign = planet_coords(&|planet_owner| planet_owner != 0 && planet_owner != owner);
    let sector = [1, 1];
    {
        let fleet = &mut data.fleets[1];
        fleet.ships.scouts = 1;
        fleet.ships.destroyers = 1;
        set_troop_transports(fleet, 1);
        set_armies_aboard(fleet, 1);
        fleet.ships.etacs = 1;
    }

    let cases = [
        (Order::HoldPosition, sector, None, [1, 0]),
        (Order::MoveOnly, sector, None, [0, 0]),
        (Order::SeekHome, owned, None, [1, 0]),
        (Order::PatrolSector, sector, None, [0, 0]),
        (
            Order::GuardStarbase,
            base.sector.to_array(),
            Some(base.base_id.0 as u8),
            [1, base.base_id.0 as u8],
        ),
        (Order::GuardBlockadeWorld, owned, None, [1, 0]),
        (Order::BombardWorld, foreign, None, [1, 0]),
        (Order::InvadeWorld, foreign, None, [1, 0]),
        (Order::BlitzWorld, foreign, None, [1, 0]),
        (Order::ViewWorld, foreign, None, [1, 0]),
        (Order::ScoutSector, sector, None, [0, 0]),
        (Order::ScoutSolarSystem, foreign, None, [1, 0]),
        (Order::ColonizeWorld, foreign, None, [1, 0]),
        (Order::JoinAnotherFleet, sector, Some(host_id), [1, host_id]),
        (Order::RendezvousSector, sector, None, [0, 0]),
        (Order::Salvage, owned, None, [1, 0]),
    ];
    assert_eq!(cases.len(), 16);
    for (order, target, aux1, expected) in cases {
        set_mission_target(&mut data.fleets[1], 0xfe, 0xfe);
        let aux = data
            .set_fleet_order(2, 1, order.code(), target, None, aux1)
            .unwrap_or_else(|err| panic!("{order:?}: {err}"));
        assert_eq!(aux, expected, "{order:?}");
        assert_eq!(
            [
                data.fleets[1].target_depth,
                data.fleets[1].mission_target as u8
            ],
            expected,
            "{order:?}"
        );
        assert_eq!(order.targets_world(), expected[0] == 1, "{order:?}");
    }
}

#[test]
fn set_fleet_order_accepts_guard_starbase_using_submitted_aux_bytes() {
    let mut data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.set_guard_starbase(1, 1, [16, 13], 1, 1).unwrap();
    let base = data.bases[0].clone();
    let current_speed = data.fleets[1].speed;
    data.fleets[1].order = Order::HoldPosition;
    data.fleets[1].target = Coords::new(0, 0);
    set_mission_target(&mut data.fleets[1], 0, 0);

    data.set_fleet_order(
        2,
        current_speed,
        Order::GuardStarbase.code(),
        base.sector.to_array(),
        Some(1),
        Some(base.base_id.0 as u8),
    )
    .unwrap();

    assert_eq!(data.fleets[1].order, Order::GuardStarbase);
    assert_eq!(data.fleets[1].target.to_array(), base.sector.to_array());
    assert_eq!(
        [
            data.fleets[1].target_depth,
            data.fleets[1].mission_target as u8
        ],
        [1, base.base_id.0 as u8]
    );
}

#[test]
fn set_fleet_order_accepts_join_another_fleet_using_submitted_aux_bytes() {
    let mut data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    let host = data.fleets[0].clone();
    let current_speed = data.fleets[1].speed;
    data.fleets[1].order = Order::HoldPosition;
    data.fleets[1].target = Coords::new(0, 0);
    set_mission_target(&mut data.fleets[1], 0, 0);

    data.set_fleet_order(
        2,
        current_speed,
        Order::JoinAnotherFleet.code(),
        host.sector.to_array(),
        None,
        Some(host.record.0 as u8),
    )
    .unwrap();

    assert_eq!(data.fleets[1].order, Order::JoinAnotherFleet);
    assert_eq!(data.fleets[1].target.to_array(), host.sector.to_array());
    assert_eq!(data.fleets[1].mission_target, host.record.0);
}

#[test]
fn core_game_data_current_known_validation_helpers_match_known_fixtures() {
    let fleet_data = CoreGameData {
        players: parse_player_dat(&read_ecmaint_fleet_pre_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_ecmaint_fleet_pre_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_ecmaint_fleet_pre_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_ecmaint_fleet_pre_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_ecmaint_fleet_pre_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_ecmaint_fleet_pre_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_ecmaint_fleet_pre_fixture("CONQUEST.DAT")).unwrap(),
    };
    assert!(
        fleet_data
            .fleet_order_errors_current_known(1, 0x03, 0x0C, [0x0F, 0x0D], None, None)
            .is_empty()
    );

    let build_data = CoreGameData {
        players: parse_player_dat(&read_ecmaint_build_pre_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_ecmaint_build_pre_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_ecmaint_build_pre_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_ecmaint_build_pre_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_ecmaint_build_pre_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_ecmaint_build_pre_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_ecmaint_build_pre_fixture("CONQUEST.DAT")).unwrap(),
    };
    assert!(
        build_data
            .planet_build_errors_current_known(15, 0x03, 0x01)
            .is_empty()
    );

    let starbase_data = CoreGameData {
        players: parse_player_dat(&read_ecmaint_starbase_pre_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_ecmaint_starbase_pre_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_ecmaint_starbase_pre_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_ecmaint_starbase_pre_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_ecmaint_starbase_pre_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_ecmaint_starbase_pre_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_ecmaint_starbase_pre_fixture("CONQUEST.DAT")).unwrap(),
    };
    assert!(
        starbase_data
            .guard_starbase_onebase_errors_current_known()
            .is_empty()
    );
    assert_eq!(
        starbase_data.current_known_compliance_status(),
        CurrentKnownComplianceStatus {
            fleet_order: false,
            planet_build: false,
            guard_starbase: true,
            ipbm: true,
        }
    );
    assert_eq!(
        starbase_data.current_known_key_word_summary(),
        CurrentKnownKeyWordSummary {
            player_first_base_record: 1,
            player_ipbm_count: 0,
            fleet1_local_slot: Some(1),
            fleet1_id: Some(1),
            fleet1_guard_index: Some(1),
            fleet1_target: Some([0x10, 0x0D]),
            base1_escort_fleet_slot: Some(1),
            base1_id: Some(1),
            base1_record_number: Some(1),
            base1_coords: Some([0x10, 0x0D]),
            ipbm_record_count: 0,
        }
    );
    assert_eq!(
        starbase_data
            .guard_starbase_linkage_summary_current_known(1, 1)
            .unwrap(),
        CurrentKnownGuardStarbaseLinkageSummary {
            player_record_index_1_based: 1,
            fleet_record_index_1_based: 1,
            player_starbase_count: 1,
            fleet_order: 0x04,
            fleet_local_slot: 1,
            fleet_id: 1,
            guard_index: 1,
            target_coords: [0x10, 0x0D],
            selected_base_present: true,
            selected_base_escort_fleet_slot: Some(1),
            selected_base_id: Some(1),
            selected_base_record_number: Some(1),
            selected_base_coords: Some([0x10, 0x0D]),
            selected_base_target_coords: Some([0x10, 0x0D]),
            selected_base_owner_empire: Some(1),
        }
    );
    assert!(
        starbase_data
            .guard_starbase_linkage_errors_current_known(1, 1)
            .is_empty()
    );
    assert_eq!(
        starbase_data.guarding_fleet_record_indexes_current_known(),
        vec![1]
    );
    assert_eq!(
        starbase_data
            .guard_starbase_linkage_summaries_for_guarding_fleets_current_known(1)
            .len(),
        1
    );
    assert!(
        starbase_data
            .guard_starbase_linkage_errors_for_guarding_fleets_current_known(1)
            .is_empty()
    );

    let post_data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: parse_bases_dat(&read_post_maint_fixture("BASES.DAT")).unwrap(),
        ipbms: parse_ipbm_dat(&read_post_maint_fixture("IPBM.DAT")).unwrap(),
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };
    assert!(post_data.ipbm_count_errors_current_known().is_empty());
    assert!(
        post_data
            .guarding_fleet_record_indexes_current_known()
            .is_empty()
    );
    assert!(
        post_data
            .guard_starbase_linkage_summaries_for_guarding_fleets_current_known(1)
            .is_empty()
    );
    assert_eq!(
        post_data.guard_starbase_linkage_errors_for_guarding_fleets_current_known(1),
        vec!["no guarding fleets found".to_string()]
    );
    assert!(
        post_data
            .guard_starbase_linkage_errors_current_known(1, 1)
            .iter()
            .any(|error| error.contains("guard starbase index expected non-zero"))
    );
}

#[test]
fn detach_ships_creates_new_fleet_and_preserves_donor_order() {
    let mut player = Player {
        mode: PlayerMode::Human,
        ..Player::default()
    };
    player.fleets_first.0 = 1;

    let mut donor = Fleet::default();
    donor.owner.0 = 1;
    donor.fleet_number.0 = 1;
    donor.record.0 = 1;
    donor.sector = Coords::new(24, 14);
    set_fleet_at_rest(&mut donor);
    donor.order = Order::GuardBlockadeWorld;
    donor.target = Coords::new(25, 12);
    donor.ships.destroyers = 2;
    donor.ships.cruisers = 1;
    set_troop_transports(&mut donor, 2);
    set_armies_aboard(&mut donor, 1);
    donor.ships.etacs = 1;
    donor.rules_of_engagement = 6;
    recompute_max_speed(&mut donor);
    donor.speed = 5;

    let mut data = CoreGameData {
        players: vec![player],
        planets: vec![],
        fleets: vec![donor],
        bases: vec![],
        ipbms: vec![],
        setup: Setup::default(),
        conquest: Conquest::default(),
    };

    let result = data
        .detach_ships_to_new_fleet(
            1,
            1,
            FleetDetachSelection {
                destroyers: 1,
                full_transports: 1,
                etacs: 1,
                ..FleetDetachSelection::default()
            },
            None,
            4,
        )
        .unwrap();

    assert_eq!(
        result,
        FleetDetachResult {
            donor_fleet_record_index_1_based: 1,
            new_fleet_record_index_1_based: 2,
        }
    );
    let donor = &data.fleets[0];
    assert_eq!(donor.ships.destroyers, 1);
    assert_eq!(donor.ships.cruisers, 1);
    assert_eq!(troop_transports(donor), 1);
    assert_eq!(donor.ships.loaded_transports, 0);
    assert_eq!(donor.ships.etacs, 0);
    assert_eq!(donor.order, Order::GuardBlockadeWorld);
    assert_eq!(donor.target.to_array(), [25, 12]);

    let detached = &data.fleets[1];
    assert_eq!(detached.fleet_number.0, 2);
    assert_eq!(detached.record.0, 2);
    assert_eq!(detached.sector.to_array(), [24, 14]);
    assert_eq!(detached.ships.destroyers, 1);
    assert_eq!(troop_transports(detached), 1);
    assert_eq!(detached.ships.loaded_transports, 1);
    assert_eq!(detached.ships.etacs, 1);
    assert_eq!(detached.rules_of_engagement, 4);
    assert_eq!(detached.order, Order::HoldPosition);
    assert_eq!(detached.speed, 0);
    assert_eq!(data.players[0].fleets_first.0, 1);
}

#[test]
fn detach_ships_clamps_support_only_new_fleet_to_roe_zero() {
    let mut player = Player {
        mode: PlayerMode::Human,
        ..Player::default()
    };
    player.fleets_first.0 = 1;

    let mut donor = Fleet::default();
    donor.owner.0 = 1;
    donor.fleet_number.0 = 1;
    donor.record.0 = 1;
    donor.sector = Coords::new(24, 14);
    donor.ships.destroyers = 1;
    set_troop_transports(&mut donor, 1);
    set_armies_aboard(&mut donor, 1);
    donor.ships.scouts = 1;
    recompute_max_speed(&mut donor);
    donor.speed = 3;
    donor.rules_of_engagement = 6;

    let mut data = CoreGameData {
        players: vec![player],
        planets: vec![],
        fleets: vec![donor],
        bases: vec![],
        ipbms: vec![],
        setup: Setup::default(),
        conquest: Conquest::default(),
    };

    let result = data
        .detach_ships_to_new_fleet(
            1,
            1,
            FleetDetachSelection {
                scouts: 1,
                ..FleetDetachSelection::default()
            },
            None,
            6,
        )
        .unwrap();

    assert_eq!(result.new_fleet_record_index_1_based, 2);
    assert_eq!(data.fleets[0].rules_of_engagement, 6);
    assert_eq!(data.fleets[1].ships.scouts, 1);
    assert_eq!(data.fleets[1].rules_of_engagement, 0);
}

#[test]
fn detach_ships_requires_valid_post_split_donor_speed() {
    let mut player = Player {
        mode: PlayerMode::Human,
        ..Player::default()
    };
    player.fleets_first.0 = 1;

    let mut donor = Fleet::default();
    donor.owner.0 = 1;
    donor.fleet_number.0 = 1;
    donor.record.0 = 1;
    donor.ships.destroyers = 1;
    donor.ships.cruisers = 1;
    recompute_max_speed(&mut donor);
    donor.speed = 6;

    let mut data = CoreGameData {
        players: vec![player],
        planets: vec![],
        fleets: vec![donor],
        bases: vec![],
        ipbms: vec![],
        setup: Setup::default(),
        conquest: Conquest::default(),
    };

    let error = data
        .detach_ships_to_new_fleet(
            1,
            1,
            FleetDetachSelection {
                destroyers: 1,
                ..FleetDetachSelection::default()
            },
            Some(6),
            6,
        )
        .unwrap_err();

    assert_eq!(
        error,
        GameStateMutationError::InvalidFleetSpeed {
            fleet_index_1_based: 1,
            requested: 6,
            max: 5,
        }
    );
}

#[test]
fn detach_ships_invalidated_donor_colonize_order_falls_back_to_local_hold() {
    let mut player = Player {
        mode: PlayerMode::Human,
        ..Player::default()
    };
    player.fleets_first.0 = 1;

    let mut donor = Fleet::default();
    donor.owner.0 = 1;
    donor.fleet_number.0 = 1;
    donor.record.0 = 1;
    donor.sector = Coords::new(24, 14);
    donor.order = Order::ColonizeWorld;
    donor.target = Coords::new(25, 12);
    set_mission_target(&mut donor, 7, 9);
    donor.ships.cruisers = 1;
    donor.ships.etacs = 1;
    recompute_max_speed(&mut donor);
    donor.speed = 3;

    let mut data = CoreGameData {
        players: vec![player],
        planets: vec![],
        fleets: vec![donor],
        bases: vec![],
        ipbms: vec![],
        setup: Setup::default(),
        conquest: Conquest::default(),
    };

    data.detach_ships_to_new_fleet(
        1,
        1,
        FleetDetachSelection {
            etacs: 1,
            ..FleetDetachSelection::default()
        },
        None,
        6,
    )
    .unwrap();

    let donor = &data.fleets[0];
    assert_eq!(donor.ships.etacs, 0);
    assert_eq!(donor.order, Order::HoldPosition);
    assert_eq!(donor.speed, 0);
    assert_eq!(donor.target.to_array(), [24, 14]);
    assert_eq!([donor.target_depth, donor.mission_target as u8], [0, 0]);
}

#[test]
fn transfer_ships_between_fleets_moves_counts_and_preserves_both_fleets() {
    let mut player = Player {
        mode: PlayerMode::Human,
        ..Player::default()
    };
    player.fleets_first.0 = 1;

    let mut donor = Fleet::default();
    donor.owner.0 = 1;
    donor.fleet_number.0 = 1;
    donor.record.0 = 1;
    donor.sector = Coords::new(24, 14);
    donor.ships.destroyers = 2;
    set_troop_transports(&mut donor, 2);
    set_armies_aboard(&mut donor, 1);
    donor.ships.etacs = 1;
    recompute_max_speed(&mut donor);
    donor.speed = 5;

    let mut host = Fleet::default();
    host.owner.0 = 1;
    host.fleet_number.0 = 2;
    host.record.0 = 2;
    host.sector = Coords::new(24, 14);
    host.ships.cruisers = 1;
    recompute_max_speed(&mut host);
    host.speed = 5;

    let mut data = CoreGameData {
        players: vec![player],
        planets: vec![],
        fleets: vec![donor, host],
        bases: vec![],
        ipbms: vec![],
        setup: Setup::default(),
        conquest: Conquest::default(),
    };

    let result = data
        .transfer_ships_between_fleets(
            1,
            1,
            2,
            FleetDetachSelection {
                destroyers: 1,
                full_transports: 1,
                ..FleetDetachSelection::default()
            },
        )
        .unwrap();

    assert_eq!(result.donor_fleet_record_index_1_based, 1);
    assert_eq!(result.host_fleet_record_index_1_based, 2);

    let donor = &data.fleets[0];
    assert_eq!(donor.ships.destroyers, 1);
    assert_eq!(troop_transports(donor), 1);
    assert_eq!(donor.ships.loaded_transports, 0);

    let host = &data.fleets[1];
    assert_eq!(host.ships.cruisers, 1);
    assert_eq!(host.ships.destroyers, 1);
    assert_eq!(troop_transports(host), 1);
    assert_eq!(host.ships.loaded_transports, 1);
}

#[test]
fn transfer_ships_invalidated_donor_scout_order_falls_back_to_local_hold() {
    let mut player = Player {
        mode: PlayerMode::Human,
        ..Player::default()
    };
    player.fleets_first.0 = 1;

    let mut donor = Fleet::default();
    donor.owner.0 = 1;
    donor.fleet_number.0 = 1;
    donor.record.0 = 1;
    donor.sector = Coords::new(24, 14);
    donor.order = Order::ScoutSector;
    donor.target = Coords::new(25, 12);
    donor.ships.scouts = 1;
    donor.ships.destroyers = 1;
    recompute_max_speed(&mut donor);
    donor.speed = 6;

    let mut host = Fleet::default();
    host.owner.0 = 1;
    host.fleet_number.0 = 2;
    host.record.0 = 2;
    host.sector = Coords::new(24, 14);
    host.ships.cruisers = 1;
    recompute_max_speed(&mut host);
    host.speed = 5;

    let mut data = CoreGameData {
        players: vec![player],
        planets: vec![],
        fleets: vec![donor, host],
        bases: vec![],
        ipbms: vec![],
        setup: Setup::default(),
        conquest: Conquest::default(),
    };

    data.transfer_ships_between_fleets(
        1,
        1,
        2,
        FleetDetachSelection {
            scouts: 1,
            ..FleetDetachSelection::default()
        },
    )
    .unwrap();

    let donor = &data.fleets[0];
    assert_eq!(donor.ships.scouts, 0);
    assert_eq!(donor.order, Order::HoldPosition);
    assert_eq!(donor.speed, 0);
    assert_eq!(donor.target.to_array(), [24, 14]);
}

#[test]
fn transfer_ships_clamps_support_only_resulting_fleet_to_roe_zero() {
    let mut player = Player {
        mode: PlayerMode::Human,
        ..Player::default()
    };
    player.fleets_first.0 = 1;

    let mut donor = Fleet::default();
    donor.owner.0 = 1;
    donor.fleet_number.0 = 1;
    donor.record.0 = 1;
    donor.sector = Coords::new(24, 14);
    donor.ships.destroyers = 1;
    donor.ships.scouts = 1;
    recompute_max_speed(&mut donor);
    donor.speed = 3;
    donor.rules_of_engagement = 6;

    let mut host = Fleet::default();
    host.owner.0 = 1;
    host.fleet_number.0 = 2;
    host.record.0 = 2;
    host.sector = Coords::new(24, 14);
    host.ships.destroyers = 1;
    recompute_max_speed(&mut host);
    host.speed = 6;
    host.rules_of_engagement = 6;

    let mut data = CoreGameData {
        players: vec![player],
        planets: vec![],
        fleets: vec![donor, host],
        bases: vec![],
        ipbms: vec![],
        setup: Setup::default(),
        conquest: Conquest::default(),
    };

    data.transfer_ships_between_fleets(
        1,
        1,
        2,
        FleetDetachSelection {
            destroyers: 1,
            ..FleetDetachSelection::default()
        },
    )
    .unwrap();

    assert_eq!(data.fleets[0].ships.scouts, 1);
    assert_eq!(data.fleets[0].rules_of_engagement, 0);
    assert_eq!(data.fleets[1].ships.destroyers, 2);
    assert_eq!(data.fleets[1].rules_of_engagement, 6);
}

#[test]
fn unload_armies_invalidated_invade_order_falls_back_to_local_hold() {
    let player = Player {
        mode: PlayerMode::Human,
        ..Player::default()
    };

    let mut planet = Planet::default();
    planet.owner.0 = 1;
    planet.sector = Coords::new(24, 14);
    planet.armies = 0;

    let mut fleet = Fleet::default();
    fleet.owner.0 = 1;
    fleet.fleet_number.0 = 1;
    fleet.record.0 = 1;
    fleet.sector = Coords::new(24, 14);
    fleet.order = Order::InvadeWorld;
    fleet.target = Coords::new(25, 12);
    fleet.ships.destroyers = 1;
    set_troop_transports(&mut fleet, 1);
    set_armies_aboard(&mut fleet, 1);
    recompute_max_speed(&mut fleet);
    fleet.speed = 5;

    let mut data = CoreGameData {
        players: vec![player],
        planets: vec![planet],
        fleets: vec![fleet],
        bases: vec![],
        ipbms: vec![],
        setup: Setup::default(),
        conquest: Conquest::default(),
    };

    data.unload_fleet_armies_to_planet(1, 1, 1, 1).unwrap();

    let fleet = &data.fleets[0];
    assert_eq!(fleet.ships.loaded_transports, 0);
    assert_eq!(fleet.order, Order::HoldPosition);
    assert_eq!(fleet.speed, 0);
    assert_eq!(fleet.target.to_array(), [24, 14]);
}

#[test]
fn append_planet_build_order_rejects_non_multiple_surface_points() {
    let player = Player {
        mode: PlayerMode::Human,
        ..Player::default()
    };

    let mut planet = Planet::default();
    planet.owner.0 = 1;

    let mut data = CoreGameData {
        players: vec![player],
        planets: vec![planet],
        fleets: vec![],
        bases: vec![],
        ipbms: vec![],
        setup: Setup::default(),
        conquest: Conquest::default(),
    };

    for (points, kind) in [(1, 7), (21, 7), (1, 8), (3, 8)] {
        let error = data.append_planet_build_order(1, points, kind).unwrap_err();
        assert_eq!(
            error,
            GameStateMutationError::InvalidPlanetPlayerInput {
                planet_index_1_based: 1,
                reason: PlanetPlayerInputValidationError::InvalidBuildPointsForKind {
                    kind,
                    points_remaining: points,
                },
            }
        );
    }
    assert_eq!(data.planets[0].build_queue[0].points_owed, 0);

    data.append_planet_build_order(1, 40, 7).unwrap();
    data.append_planet_build_order(1, 4, 8).unwrap();
}

#[test]
fn renumbering_a_fleet_retargets_the_join_orders_that_name_it() {
    // ECGAME's renumbering (`0x15b35`) rewrites the 0x23 word of the empire's
    // Join orders that name the old number.
    let mut data =
        load_classic_core_files(&repo_root().join("fixtures/ecmaint-post/v1.5")).unwrap();
    data.set_join_fleet_order(1, 2, 3).expect("join order");
    data.set_join_fleet_order(2, 6, 7)
        .expect("empire 2 join order");
    assert_eq!(data.fleets[1].mission_target, 3);
    assert_eq!(data.fleets[5].mission_target, 3);

    data.set_fleet_local_slot(1, 3, 12)
        .expect("renumber fleet 3");

    assert_eq!(data.fleets[2].fleet_number.0, 12);
    assert_eq!(data.fleets[1].mission_target, 12);
    // Empire 2's join names its own fleet 3, which kept its number.
    assert_eq!(data.fleets[5].mission_target, 3);
}
