mod common;

use common::*;
use nc_compat::load_classic_core_files;
use nc_data::classic::{
    parse_bases_dat, parse_conquest_dat, parse_fleets_dat, parse_ipbm_dat, parse_planets_dat,
    parse_player_dat, parse_setup_dat, serialize_ipbm_dat,
};
use nc_data::*;

#[test]
fn base_record_setters_can_recreate_known_valid_guard_starbase_record() {
    let mut record = Starbase::default();
    record.set_escort_fleet_slot_raw(0x0001);
    record.set_base_id_raw(0x0001);
    record.set_owner_empire_raw(0x01);
    record.set_next_base_link_raw(0x0000);
    record.set_record_number_raw(0x0001);
    record.set_previous_base_link_raw(0x0000);
    record.set_coords_raw([0x10, 0x0D]);
    record.set_at_rest_motion();
    record.set_moving_flag_raw(0);
    record.set_target_coords_raw([0x10, 0x0D]);
    record.set_target_depth_raw(1);

    assert_eq!(record.escort_fleet_slot_raw(), 0x0001);
    assert_eq!(record.base_id_raw(), 0x0001);
    assert_eq!(record.owner_empire_raw(), 0x01);
    assert_eq!(record.next_base_link_raw(), 0x0000);
    assert_eq!(record.record_number_raw(), 0x0001);
    assert_eq!(record.previous_base_link_raw(), 0x0000);
    assert_eq!(record.coords_raw(), [0x10, 0x0D]);
    assert_eq!(record.x_fraction(), Real48::HALF);
    assert_eq!(record.y_fraction(), Real48::HALF);
    assert_eq!(record.depth(), Real48::ONE);
    assert_eq!(record.moving_flag_raw(), 0);
    assert_eq!(record.target_coords_raw(), [0x10, 0x0D]);
    assert_eq!(record.target_depth_raw(), 1);

    assert_eq!(
        record.to_bytes(),
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
    assert_eq!(player[0].fleet_chain_head_raw(), 1);
    assert_eq!(player[0].ipbm_chain_head_raw(), 0);

    let fleet_bytes = read_ecmaint_starbase_pre_fixture("FLEETS.DAT");
    let fleets = parse_fleets_dat(&fleet_bytes).unwrap();
    let fleet = &fleets[0];
    assert_eq!(fleet.local_slot_word_raw(), 1);
    assert_eq!(fleet.next_fleet_link_word_raw(), 2);
    assert_eq!(fleet.fleet_id_word_raw(), 1);
    assert_eq!(fleet.world_target_flag_raw(), 1);
    assert_eq!(fleet.guard_starbase_index_raw(), 1);

    let base_bytes = read_ecmaint_starbase_pre_fixture("BASES.DAT");
    let bases = parse_bases_dat(&base_bytes).unwrap();
    let base = &bases[0];
    assert_eq!(base.escort_fleet_slot_raw(), fleet.local_slot_word_raw());
    assert_eq!(base.base_id_raw(), fleet.guard_starbase_index_raw());
    assert_eq!(base.record_number_raw(), 1);
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
    assert_eq!(base.coords_raw(), [16, 13]);
    assert_eq!(base.target_coords_raw(), [2, 12]);
    assert_eq!(base.moving_flag_raw(), 1);
    assert_eq!(base.target_depth_raw(), 0, "no world of empire 1 at (2,12)");

    let home = data.planets[0].coords_raw();
    assert_eq!(data.planets[0].owner_empire_slot_raw(), 1);
    data.set_starbase_destination(1, 1, home)
        .expect("owned starbase destination should update");
    assert_eq!(data.bases[0].target_coords_raw(), home);
    assert_eq!(data.bases[0].target_depth_raw(), 1, "own world: orbit");
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
    data.bases[0].set_depth(Real48::from_bytes([0x80, 0x00, 0x00, 0x00, 0x00, 0x40]));

    data.halt_starbase(1, 1)
        .expect("owned starbase halt should succeed");

    let base = &data.bases[0];
    assert_eq!(base.coords_raw(), [16, 13]);
    assert_eq!(base.target_coords_raw(), [16, 13]);
    assert_eq!(base.moving_flag_raw(), 0);
    assert_eq!(base.target_depth_raw(), 1, "Round(0.75) = 1");
    assert_eq!(
        base.depth().to_bytes(),
        [0x80, 0x00, 0x00, 0x00, 0x00, 0x40]
    );
}

#[test]
fn fleet_owner_empire_accessor_round_trips() {
    let mut record = Fleet::default();
    record.set_owner_empire_raw(0x03);
    assert_eq!(record.owner_empire_raw(), 0x03);
}

#[test]
fn fleet_recompute_max_speed_uses_slowest_member() {
    let mut record = Fleet::default();
    record.set_destroyer_count(10);
    record.set_etac_count(1);
    record.set_current_speed(6);
    record.recompute_max_speed_from_composition();
    assert_eq!(record.max_speed(), 3);
    assert_eq!(record.current_speed(), 3);
}

#[test]
fn player_fleet_chain_head_accessor_round_trips() {
    let mut record = Player::default();
    record.set_fleet_chain_head_raw(0x1234);
    assert_eq!(record.fleet_chain_head_raw(), 0x1234);
}

#[test]
fn controlled_empire_name_setter_does_not_clobber_later_player_fields() {
    let mut record = Player::default();
    record.set_homeworld_planet_index_1_based_raw(7);
    record.set_tax_rate_raw(50);
    record.set_ipbm_chain_head_raw(3);
    record.set_classic_messages_pending_flag_raw(1);
    record.set_classic_reports_pending_flag_raw(1);

    record.set_controlled_empire_name_raw("Codex Dominion");

    assert_eq!(record.controlled_empire_name_summary(), "Codex Dominion");
    assert_eq!(record.homeworld_planet_index_1_based_raw(), 7);
    assert_eq!(record.tax_rate(), 50);
    assert_eq!(record.ipbm_chain_head_raw(), 3);
    assert_eq!(record.classic_messages_pending_flag_raw(), 1);
    assert_eq!(record.classic_reports_pending_flag_raw(), 1);
}

#[test]
fn classic_login_reviewables_helper_mirrors_both_review_families() {
    let mut record = Player::default();

    record.set_classic_login_reviewables_present(true);
    assert_eq!(record.classic_message_review_word_raw(), 1);
    assert_eq!(record.classic_message_review_carry_word_raw(), 0);
    assert_eq!(record.classic_results_review_word_raw(), 1);
    assert_eq!(record.classic_results_review_carry_word_raw(), 0);
    assert!(record.has_classic_messages_review_state());
    assert!(record.has_classic_results_review_state());
    assert!(record.has_any_classic_review_state());

    record.set_classic_login_reviewables_present(false);
    assert_eq!(record.classic_message_review_word_raw(), 0);
    assert_eq!(record.classic_message_review_carry_word_raw(), 0);
    assert_eq!(record.classic_results_review_word_raw(), 0);
    assert_eq!(record.classic_results_review_carry_word_raw(), 0);
    assert!(!record.has_any_classic_review_state());
}

#[test]
fn rename_owned_planet_updates_name_for_matching_owner() {
    let mut planet = Planet::default();
    planet.set_owner_empire_slot_raw(1);
    planet.set_planet_name("Not Named Yet");

    let mut data = CoreGameData {
        players: vec![Player::default()],
        planets: vec![planet],
        fleets: vec![],
        bases: vec![],
        ipbms: vec![],
        setup: Setup::default(),
        conquest: Conquest::default(),
    };
    data.players[0].set_owner_empire_raw(1);

    data.rename_owned_planet(1, 1, "New Horizon").unwrap();
    assert_eq!(data.planets[0].planet_name(), "New Horizon");
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
        .position(|planet| planet.owner_empire_slot_raw() == 1)
        .map(|idx| idx + 1)
        .unwrap();
    let before = data.planets[planet_record_index_1_based - 1].planet_name();

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
        data.planets[planet_record_index_1_based - 1].planet_name(),
        before
    );
}

#[test]
fn set_planet_name_stores_classic_text_up_to_twenty_chars() {
    let mut planet = Planet::default();
    planet.set_planet_name("\u{1F600}\u{1F600}\u{1F600}\u{1F600}");
    assert_eq!(planet.planet_name(), "????");

    planet.set_planet_name("Fourteen Chars");
    assert_eq!(planet.planet_name(), "Fourteen Chars");

    planet.set_planet_name("Twenty-one characters");
    assert_eq!(planet.planet_name(), "Twenty-one character");
}

#[test]
fn join_player_normalizes_zero_tax_slots_to_seeded_opening_tax_rate() {
    let mut data = GameStateBuilder::new()
        .with_player_count(2)
        .build_joinable_new_game_baseline()
        .expect("joinable baseline should build");

    data.players[0].set_tax_rate_raw(0);
    data.planets[0].set_stored_production_points(0);

    data.join_player(1, "Codex Dominion")
        .expect("join player should succeed");

    assert_eq!(data.players[0].tax_rate(), 50);
    assert_eq!(data.planets[0].stored_production_points(), 50);
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

    data.players[0].set_last_run_year_raw(3004);
    data.planets[0].set_owner_empire_slot_raw(0);
    data.planets[0].set_ownership_status_raw(0);
    data.players[0].set_planet_count_raw(0);

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
    player.set_controlled_empire_name_raw("\u{C6}gir Dominion");
    assert_eq!(player.controlled_empire_name_summary(), "?gir Dominion");
    player.set_assigned_player_handle_raw("Bj\u{F6}rn\u{1F600}");
    assert_eq!(player.assigned_player_handle_summary(), "Bj?rn?");

    let mut planet = Planet::default();
    planet.set_status_or_name_summary_raw("\u{1F600}\u{1F600}\u{1F600}\u{1F600}");
    assert_eq!(planet.planet_name(), "????");
}

#[test]
fn stardock_slots_are_typed_u32_counts_at_classic_offsets() {
    let mut planet = Planet::default();

    planet.set_stardock_count_raw(0, 1);
    planet.set_stardock_count_raw(3, 258);
    planet.set_stardock_count(6, 70_000);

    assert_eq!(planet.stardock_count_raw(0), 1);
    assert_eq!(planet.stardock_count_raw(3), 258);
    assert_eq!(planet.stardock_count(6), 70_000);
    assert_eq!(planet.stardock_kind_raw(0), 1);
    assert_eq!(planet.stardock_kind_raw(1), 0);
    assert_eq!(planet.stardock_kind_raw(3), 4);
    assert_eq!(planet.stardock_kind_raw(6), 9);

    let bytes = planet.to_bytes();
    assert_eq!(bytes[0x38..0x3C], [1, 0, 0, 0]);
    assert_eq!(bytes[0x44..0x48], [2, 1, 0, 0]);
    assert_eq!(bytes[0x50..0x54], 70_000u32.to_le_bytes());
}

#[test]
fn commission_ship_from_stardock_appends_fleet_and_clears_slot() {
    let mut player = Player::default();
    player.set_owner_empire_raw(1);

    let mut planet = Planet::default();
    planet.set_owner_empire_slot_raw(1);
    planet.set_coords_raw([6, 5]);
    planet.set_stardock_count_raw(0, 2);

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
    assert_eq!(data.planets[0].stardock_kind_raw(0), 0);
    assert_eq!(data.planets[0].stardock_count_raw(0), 0);
    assert_eq!(data.players[0].fleet_chain_head_raw(), 1);
    assert_eq!(data.fleets.len(), 1);
    let fleet = &data.fleets[0];
    assert_eq!(fleet.owner_empire_raw(), 1);
    assert_eq!(fleet.fleet_id_word_raw(), 1);
    assert_eq!(fleet.local_slot(), 1);
    assert_eq!(fleet.destroyer_count(), 2);
    assert_eq!(fleet.current_location_coords_raw(), [6, 5]);
    assert_eq!(fleet.standing_order_kind(), Order::HoldPosition);
    assert_eq!(fleet.rules_of_engagement(), 6);
}

#[test]
fn commission_support_only_fleet_starts_at_roe_zero() {
    let mut player = Player::default();
    player.set_owner_empire_raw(1);

    let mut planet = Planet::default();
    planet.set_owner_empire_slot_raw(1);
    planet.set_coords_raw([6, 5]);
    planet.set_stardock_count_raw(3, 2);

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
    assert_eq!(fleet.scout_count(), 2);
    assert_eq!(fleet.rules_of_engagement(), 0);
}

#[test]
fn append_planet_build_order_still_uses_all_ten_queue_slots() {
    let mut player = Player::default();
    player.set_owner_empire_raw(1);

    let mut planet = Planet::default();
    planet.set_owner_empire_slot_raw(1);
    for slot in 0..9 {
        planet.set_build_count_raw(slot, 1);
        planet.set_build_kind_raw(slot, 1);
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

    assert_eq!(data.planets[0].build_count_raw(9), 15);
    assert_eq!(data.planets[0].build_kind_raw(9), 4);
}

#[test]
fn append_planet_build_order_merges_into_matching_kind_before_using_empty_slot() {
    let mut player = Player::default();
    player.set_owner_empire_raw(1);

    let mut planet = Planet::default();
    planet.set_owner_empire_slot_raw(1);
    planet.set_build_count_raw(0, 10);
    planet.set_build_kind_raw(0, 1);

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

    assert_eq!(data.planets[0].build_count_raw(0), 25);
    assert_eq!(data.planets[0].build_kind_raw(0), 1);
    assert!((1..10).all(|slot| data.planets[0].build_count_raw(slot) == 0));
}

#[test]
fn append_planet_build_order_spills_same_kind_across_records_without_truncation() {
    let mut player = Player::default();
    player.set_owner_empire_raw(1);

    let mut planet = Planet::default();
    planet.set_owner_empire_slot_raw(1);
    planet.set_build_count_raw(0, 250);
    planet.set_build_kind_raw(0, 1);

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

    assert_eq!(data.planets[0].build_count_raw(0), u8::MAX);
    assert_eq!(data.planets[0].build_kind_raw(0), 1);
    assert_eq!(data.planets[0].build_count_raw(1), 15);
    assert_eq!(data.planets[0].build_kind_raw(1), 1);
}

#[test]
fn append_planet_build_order_does_not_partially_write_when_capacity_is_exhausted() {
    let mut player = Player::default();
    player.set_owner_empire_raw(1);

    let mut planet = Planet::default();
    planet.set_owner_empire_slot_raw(1);
    for slot in 0..10 {
        planet.set_build_count_raw(slot, u8::MAX);
        planet.set_build_kind_raw(slot, 1);
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
    let mut player = Player::default();
    player.set_owner_empire_raw(1);

    let mut planet = Planet::default();
    planet.set_owner_empire_slot_raw(1);

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
                kind_raw: 4,
                points_remaining_raw: 14,
            },
        }
    );
}

#[test]
fn append_planet_build_order_spills_starbases_one_per_queue_slot() {
    let mut player = Player::default();
    player.set_owner_empire_raw(1);

    let mut planet = Planet::default();
    planet.set_owner_empire_slot_raw(1);

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

    assert_eq!(data.planets[0].build_count_raw(0), 50);
    assert_eq!(data.planets[0].build_kind_raw(0), 9);
    assert_eq!(data.planets[0].build_count_raw(1), 50);
    assert_eq!(data.planets[0].build_kind_raw(1), 9);
    assert_eq!(data.planets[0].build_count_raw(2), 50);
    assert_eq!(data.planets[0].build_kind_raw(2), 9);
    assert!((3..10).all(|slot| data.planets[0].build_count_raw(slot) == 0));
}

#[test]
fn commission_ship_reuses_lowest_available_owned_fleet_number() {
    let mut player = Player::default();
    player.set_owner_empire_raw(1);
    player.set_fleet_chain_head_raw(1);

    let mut planet = Planet::default();
    planet.set_owner_empire_slot_raw(1);
    planet.set_coords_raw([6, 5]);
    planet.set_stardock_count_raw(0, 1);

    let mut fleet_a = Fleet::default();
    fleet_a.set_owner_empire_raw(1);
    fleet_a.set_fleet_id_word_raw(1);
    fleet_a.set_local_slot_word_raw(1);
    fleet_a.set_next_fleet_link_word_raw(2);

    let mut fleet_b = Fleet::default();
    fleet_b.set_owner_empire_raw(1);
    fleet_b.set_fleet_id_word_raw(2);
    fleet_b.set_local_slot_word_raw(3);
    fleet_b.set_previous_fleet_id(1);

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
    assert_eq!(data.players[0].fleet_chain_head_raw(), 1);
    assert_eq!(data.fleets[0].next_fleet_link_word_raw(), 2);
    let fleet = &data.fleets[1];
    assert_eq!(fleet.local_slot_word_raw(), 2);
    assert_eq!(fleet.destroyer_count(), 1);
    assert_eq!(fleet.fleet_id_word_raw(), 2);
    assert_eq!(fleet.previous_fleet_id(), 1);
    assert_eq!(fleet.next_fleet_link_word_raw(), 3);
    let moved = &data.fleets[2];
    assert_eq!(moved.local_slot_word_raw(), 3);
    assert_eq!(moved.fleet_id_word_raw(), 3);
    assert_eq!(moved.previous_fleet_id(), 2);
    assert_eq!(moved.next_fleet_link_word_raw(), 0);
}

#[test]
fn commission_ship_draft_can_split_selected_stardock_rows() {
    let mut player = Player::default();
    player.set_owner_empire_raw(1);

    let mut planet = Planet::default();
    planet.set_owner_empire_slot_raw(1);
    planet.set_coords_raw([6, 5]);
    planet.set_stardock_count_raw(0, 5);
    planet.set_stardock_count_raw(2, 3);

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
    assert_eq!(data.planets[0].stardock_count_raw(0), 3);
    assert_eq!(data.planets[0].stardock_count_raw(2), 2);
    let fleet = &data.fleets[0];
    assert_eq!(fleet.destroyer_count(), 2);
    assert_eq!(fleet.battleship_count(), 1);
    assert_eq!(fleet.current_location_coords_raw(), [6, 5]);
}

#[test]
fn commission_starbase_from_stardock_appends_base_and_clears_slot() {
    let mut player = Player::default();
    player.set_owner_empire_raw(1);

    let mut planet = Planet::default();
    planet.set_owner_empire_slot_raw(1);
    planet.set_coords_raw([6, 5]);
    planet.set_stardock_count_raw(6, 1);

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
    assert_eq!(data.planets[0].stardock_kind_raw(6), 0);
    assert_eq!(data.planets[0].stardock_count_raw(6), 0);
    assert_eq!(data.players[0].first_base_record_raw(), 1);
    assert_eq!(data.bases.len(), 1);
    let base = &data.bases[0];
    assert_eq!(base.base_id_raw(), 1);
    assert_eq!(base.coords_raw(), [6, 5]);
    assert_eq!(base.owner_empire_raw(), 1);
    // A new base as ECMAINT 2000:6642 creates one: no escort, at rest in
    // orbit, target = its own sector.
    assert_eq!(
        base.to_bytes(),
        [
            0, 0, 1, 0, 1, 0, 0, 1, 0, 0, 0, 6, 5, 0x80, 0, 0, 0, 0, 0, 0x80, 0, 0, 0, 0, 0, 0x81,
            0, 0, 0, 0, 0, 0, 6, 5, 1,
        ]
    );

    // A second base takes the next id and joins the end of the chain.
    data.planets[0].set_stardock_count_raw(6, 1);
    data.commission_planet_stardock_slot(1, 1, 6).unwrap();
    let [first, second] = &data.bases[..] else {
        panic!("expected two bases");
    };
    assert_eq!(first.next_base_link_raw(), 2);
    assert_eq!(second.base_id_raw(), 2);
    assert_eq!(second.record_number_raw(), 2);
    assert_eq!(second.previous_base_link_raw(), 1);
    assert_eq!(second.next_base_link_raw(), 0);
}

#[test]
fn commission_rejects_foreign_owned_planet() {
    let mut player = Player::default();
    player.set_owner_empire_raw(1);

    let mut planet = Planet::default();
    planet.set_owner_empire_slot_raw(2);
    planet.set_coords_raw([6, 5]);
    planet.set_stardock_count_raw(0, 2);

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
    let mut player = Player::default();
    player.set_owner_empire_raw(1);

    let mut planet_one = Planet::default();
    planet_one.set_owner_empire_slot_raw(1);
    planet_one.set_planet_name("Aurora Prime");
    planet_one.set_coords_raw([6, 5]);
    planet_one.set_stardock_count_raw(0, 2);
    planet_one.set_stardock_count_raw(3, 3);
    planet_one.set_stardock_count_raw(6, 1);

    let mut planet_two = Planet::default();
    planet_two.set_owner_empire_slot_raw(1);
    planet_two.set_planet_name("Bastion Reach");
    planet_two.set_coords_raw([7, 6]);
    planet_two.set_stardock_count_raw(1, 1);

    let mut planet_three = Planet::default();
    planet_three.set_owner_empire_slot_raw(1);
    planet_three.set_planet_name("Signal Keep");
    planet_three.set_coords_raw([8, 7]);
    planet_three.set_stardock_count_raw(6, 1);

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
    assert_eq!(data.planets[0].stardock_kind_raw(0), 0);
    assert_eq!(data.planets[0].stardock_kind_raw(1), 0);
    assert_eq!(data.planets[0].stardock_kind_raw(2), 0);
    assert_eq!(data.planets[0].stardock_count_raw(2), 0);
    assert_eq!(data.planets[1].stardock_kind_raw(0), 0);
    assert_eq!(data.planets[2].stardock_kind_raw(0), 0);
    assert_eq!(data.planets[2].stardock_count_raw(0), 0);
    assert_eq!(data.bases.len(), 2);
}

#[test]
fn load_planet_armies_onto_fleet_moves_armies_into_transports() {
    let mut player = Player::default();
    player.set_owner_empire_raw(1);

    let mut planet = Planet::default();
    planet.set_owner_empire_slot_raw(1);
    planet.set_coords_raw([6, 5]);
    planet.set_army_count_raw(10);

    let mut fleet = Fleet::default();
    fleet.set_owner_empire_raw(1);
    fleet.set_current_location_coords_raw([6, 5]);
    fleet.set_troop_transport_count(4);
    fleet.set_army_count(1);

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
    assert_eq!(data.planets[0].army_count_raw(), 7);
    assert_eq!(data.fleets[0].army_count(), 4);
    // Each loaded army moves one transport from 0x2c (empty) to 0x2e (loaded).
    assert_eq!(data.fleets[0].empty_transport_count(), 0);
    assert_eq!(data.fleets[0].loaded_transport_count(), 4);
    assert_eq!(data.fleets[0].troop_transport_count(), 4);
}

#[test]
fn unload_fleet_armies_to_planet_moves_armies_back_to_planet() {
    let mut player = Player::default();
    player.set_owner_empire_raw(1);

    let mut planet = Planet::default();
    planet.set_owner_empire_slot_raw(1);
    planet.set_coords_raw([6, 5]);
    planet.set_army_count_raw(4);

    let mut fleet = Fleet::default();
    fleet.set_owner_empire_raw(1);
    fleet.set_current_location_coords_raw([6, 5]);
    fleet.set_troop_transport_count(4);
    fleet.set_army_count(3);

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
    assert_eq!(data.planets[0].army_count_raw(), 6);
    assert_eq!(data.fleets[0].army_count(), 1);
    // Each unloaded army moves one transport from 0x2e back to 0x2c.
    assert_eq!(data.fleets[0].empty_transport_count(), 3);
    assert_eq!(data.fleets[0].loaded_transport_count(), 1);
    assert_eq!(data.fleets[0].troop_transport_count(), 4);
}

#[test]
fn unload_fleet_armies_to_planet_rejects_planet_overflow() {
    let mut player = Player::default();
    player.set_owner_empire_raw(1);

    let mut planet = Planet::default();
    planet.set_owner_empire_slot_raw(1);
    planet.set_coords_raw([6, 5]);
    planet.set_army_count_raw(255);

    let mut fleet = Fleet::default();
    fleet.set_owner_empire_raw(1);
    fleet.set_current_location_coords_raw([6, 5]);
    fleet.set_troop_transport_count(4);
    fleet.set_army_count(3);

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
    let mut record = Ipbm::default();
    record.set_ipbm_number_raw(0x1234);
    record.set_owner_empire_raw(0x02);
    record.set_next_ipbm_link_raw(0x4567);
    record.set_record_number_raw(0x89ab);

    assert_eq!(record.ipbm_number_raw(), 0x1234);
    assert_eq!(record.owner_empire_raw(), 0x02);
    assert_eq!(record.next_ipbm_link_raw(), 0x4567);
    assert_eq!(record.record_number_raw(), 0x89ab);
    let bytes = record.to_bytes();
    assert_eq!(&bytes[..7], &[0x34, 0x12, 0x02, 0x67, 0x45, 0xab, 0x89]);
}

#[test]
fn ipbm_record_setters_round_trip_position_and_target() {
    let mut record = Ipbm::default();
    record.set_coords_raw([0x11, 0x22]);
    record.set_x_fraction(Real48::from_bytes([1, 2, 3, 4, 5, 6]));
    record.set_y_fraction(Real48::from_bytes([7, 8, 9, 10, 11, 12]));
    record.set_depth(Real48::from_bytes([13, 14, 15, 16, 17, 18]));
    record.set_target_coords_raw([0xAA, 0xBB]);
    record.set_target_depth_raw(0xCC);

    assert_eq!(record.coords_raw(), [0x11, 0x22]);
    assert_eq!(record.target_coords_raw(), [0xAA, 0xBB]);
    assert_eq!(record.target_depth_raw(), 0xCC);
    let bytes = record.to_bytes();
    assert_eq!(&bytes[0x09..0x0B], &[0x11, 0x22]);
    assert_eq!(&bytes[0x0B..0x11], &[1, 2, 3, 4, 5, 6]);
    assert_eq!(&bytes[0x11..0x17], &[7, 8, 9, 10, 11, 12]);
    assert_eq!(&bytes[0x17..0x1D], &[13, 14, 15, 16, 17, 18]);
    assert_eq!(&bytes[0x1D..0x20], &[0xAA, 0xBB, 0xCC]);
}

#[test]
fn can_set_maintenance_schedule_from_enabled_days() {
    let mut post_maint = parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap();
    post_maint.set_maintenance_schedule_enabled([true, false, true, false, true, false, true]);
    // ECUTIL writes 0 or 1; ECMAINT tests only for nonzero.
    assert_eq!(
        post_maint.maintenance_schedule_bytes(),
        [0x01, 0x00, 0x01, 0x00, 0x01, 0x00, 0x01]
    );
    assert_eq!(
        post_maint.maintenance_schedule_enabled(),
        [true, false, true, false, true, false, true]
    );
}

#[test]
fn can_toggle_snoop_enabled() {
    let mut setup = parse_setup_dat(&read_fixture("SETUP.DAT")).unwrap();
    assert!(setup.snoop_enabled());
    setup.set_snoop_enabled(false);
    assert!(!setup.snoop_enabled());
    assert_eq!(setup.to_bytes()[512], 0);
}

#[test]
fn can_set_other_setup_program_fields() {
    let mut setup = parse_setup_dat(&read_fixture("SETUP.DAT")).unwrap();
    assert!(setup.set_com_hardware_flow_control_enabled(0, false));
    assert!(setup.set_com_hardware_flow_control_enabled(1, false));
    assert!(setup.set_com_hardware_flow_control_enabled(2, false));
    assert!(setup.set_com_hardware_flow_control_enabled(3, false));
    setup.set_max_time_between_keys_minutes_raw(15);
    setup.set_remote_timeout_enabled(false);
    setup.set_local_timeout_enabled(true);
    setup.set_minimum_time_granted_minutes_raw(69);
    setup.set_purge_after_turns_raw(10);
    setup.set_autopilot_inactive_turns_raw(3);

    assert_eq!(setup.max_time_between_keys_minutes_raw(), 15);
    assert!(!setup.remote_timeout_enabled());
    assert!(setup.local_timeout_enabled());
    assert_eq!(setup.minimum_time_granted_minutes_raw(), 69);
    assert_eq!(setup.purge_after_turns_raw(), 10);
    assert_eq!(setup.autopilot_inactive_turns_raw(), 3);
    assert_eq!(setup.com_hardware_flow_control_enabled(0), Some(false));
    assert_eq!(setup.com_hardware_flow_control_enabled(1), Some(false));
    assert_eq!(setup.com_hardware_flow_control_enabled(2), Some(false));
    assert_eq!(setup.com_hardware_flow_control_enabled(3), Some(false));
}

#[test]
fn can_set_purge_after_turns_raw() {
    let mut setup = parse_setup_dat(&read_fixture("SETUP.DAT")).unwrap();
    assert_eq!(setup.purge_after_turns_raw(), 0);
    setup.set_purge_after_turns_raw(1);
    assert_eq!(setup.purge_after_turns_raw(), 1);
    assert_eq!(setup.to_bytes()[518], 1);
}

#[test]
fn planet_status_prefix_setter_stores_the_name() {
    let mut record = Planet::default();
    record.set_status_or_name_prefix_raw("Unowned");

    assert_eq!(record.string_len(), 7);
    assert_eq!(record.status_or_name_summary(), "Unowned");
    assert_eq!(&record.to_bytes()[0x0F..0x17], b"\x07Unowned");
}

#[test]
fn core_game_data_current_known_count_helpers_follow_player1_and_records() {
    let mut base1 = Starbase::default();
    base1.set_owner_empire_raw(1);
    base1.set_base_id_raw(1);
    let mut base2 = Starbase::default();
    base2.set_owner_empire_raw(1);
    base2.set_base_id_raw(2);
    let mut base3 = Starbase::default();
    base3.set_owner_empire_raw(2);
    base3.set_base_id_raw(1);
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

    let player2_starbase_before = data.players[1].first_base_record_raw();
    let player3_starbase_before = data.players[2].first_base_record_raw();
    let player4_starbase_before = data.players[3].first_base_record_raw();
    data.sync_player1_current_known_counts();

    // Player 1's chain now runs from record 1 (Starbase 1) to record 2.
    assert_eq!(data.players[0].first_base_record_raw(), 1);
    assert_eq!(data.bases[0].next_base_link_raw(), 2);
    assert_eq!(data.bases[1].previous_base_link_raw(), 1);
    assert_eq!(
        data.players[1].first_base_record_raw(),
        player2_starbase_before
    );
    assert_eq!(
        data.players[2].first_base_record_raw(),
        player3_starbase_before
    );
    assert_eq!(
        data.players[3].first_base_record_raw(),
        player4_starbase_before
    );
    assert_eq!(data.players[0].ipbm_chain_head_raw(), 3);
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

    data.fleets[1].set_next_fleet_link_word_raw(9);

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

    data.fleets[2].set_max_speed(3);

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

    data.fleets[0].set_x_fraction(Real48::ZERO);

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

    data.fleets[0].set_standing_order_kind(crate::Order::GuardStarbase);

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

    data.fleets[0].set_mission_aux_bytes([0, 1]);

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

    data.planets[0].set_owner_empire_slot_raw(9);
    data.bases[0].set_base_id_raw(1);
    data.bases[0].set_owner_empire_raw(0);

    assert_eq!(
        data.current_known_planet_owner_slot_errors(),
        vec!["PLANET[1].owner_empire_slot expected <= 4, got 9".to_string()]
    );
    assert_eq!(
        data.current_known_base_owner_empire_errors(),
        vec!["BASES[1].owner_empire expected 1..=4, got 0".to_string()]
    );

    data.bases[0].set_base_id_raw(0);
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

    let [_, target_y] = data.fleets[4].standing_order_target_coords_raw();
    data.fleets[4].set_standing_order_target_coords_raw([9, target_y]);

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

    data.planets[0].set_owner_empire_slot_raw(2);

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

    data.planets[14].set_ground_batteries_raw(1);

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

    data.planets[14].set_stored_goods(34.75);

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

    data.planets[0].set_ground_batteries_raw(1);

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

    data.planets[0].set_stored_goods(1.0);

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

    data.setup.set_remote_timeout_enabled(false);

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

    data.conquest.set_game_year(2999);

    assert_eq!(
        data.current_known_conquest_baseline_errors(),
        vec!["CONQUEST.DAT.game_year expected 3000 or 3001 for preserved initialized/post-maint baseline, got 2999".to_string()]
    );
}

#[test]
fn core_game_data_sync_current_known_baseline_controls_and_counts_repairs_mutated_fields() {
    let mut base1 = Starbase::default();
    base1.set_owner_empire_raw(1);
    base1.set_base_id_raw(1);
    let mut data = CoreGameData {
        players: parse_player_dat(&read_post_maint_fixture("PLAYER.DAT")).unwrap(),
        planets: parse_planets_dat(&read_post_maint_fixture("PLANETS.DAT")).unwrap(),
        fleets: parse_fleets_dat(&read_post_maint_fixture("FLEETS.DAT")).unwrap(),
        bases: vec![base1],
        ipbms: vec![Ipbm::default()],
        setup: parse_setup_dat(&read_post_maint_fixture("SETUP.DAT")).unwrap(),
        conquest: parse_conquest_dat(&read_post_maint_fixture("CONQUEST.DAT")).unwrap(),
    };

    data.players[0].set_first_base_record_raw(9);
    data.players[0].set_ipbm_chain_head_raw(9);
    data.setup.set_version_tag(b"BAD!!");
    data.setup.set_remote_timeout_enabled(false);
    data.conquest.set_game_year(2999);
    data.conquest.set_player_count(9);
    data.conquest.set_maintenance_schedule_enabled([false; 7]);

    data.sync_current_known_baseline_controls_and_counts();

    assert_eq!(data.players[0].first_base_record_raw(), 1);
    assert_eq!(data.players[0].ipbm_chain_head_raw(), 1);
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

    data.planets[14].set_stored_goods(3.0);
    data.planets[0].set_status_or_name_summary_raw("Broken");
    data.planets[0].set_army_count_raw(9);

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

    data.players[0].set_first_base_record_raw(7);
    data.players[0].set_ipbm_chain_head_raw(4);
    data.setup.set_version_tag(b"BAD!!");
    data.bases.push(Starbase::default());
    data.ipbms.push(Ipbm::default());
    data.fleets.clear();
    data.fleets.push(Fleet::default());
    data.planets[14].set_stored_goods(3.0);
    data.planets[0].set_status_or_name_summary_raw("Broken");

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
    assert_eq!(data.fleets[0].current_speed(), 0x03);
    assert_eq!(data.fleets[0].standing_order_code_raw(), 0x0C);
    assert_eq!(
        data.fleets[0].standing_order_target_coords_raw(),
        [0x0F, 0x0D]
    );
    assert_eq!(aux, data.fleets[0].mission_aux_bytes());

    data.set_planet_build(15, 0x03, 0x01).unwrap();
    assert_eq!(data.planets[14].build_count_raw(0), 0x03);
    assert_eq!(data.planets[14].build_kind_raw(0), 0x01);

    data.clear_planet_build_queue(15).unwrap();
    assert!((0..10).all(|slot| data.planets[14].build_count_raw(slot) == 0));
    assert!((0..10).all(|slot| data.planets[14].build_kind_raw(slot) == 0));

    data.replace_planet_build_queue_with_single_order(15, 0x05, 0x08)
        .unwrap();
    assert_eq!(data.planets[14].build_count_raw(0), 0x05);
    assert_eq!(data.planets[14].build_kind_raw(0), 0x08);
    assert!((1..10).all(|slot| data.planets[14].build_count_raw(slot) == 0));
    assert!((1..10).all(|slot| data.planets[14].build_kind_raw(slot) == 0));

    data.set_guard_starbase(1, 1, [0x10, 0x0D], 1, 1).unwrap();
    assert_eq!(data.players[0].first_base_record_raw(), 1);
    assert_eq!(data.fleets[0].standing_order_code_raw(), 0x04);
    assert_eq!(
        data.fleets[0].standing_order_target_coords_raw(),
        [0x10, 0x0D]
    );
    assert_eq!(data.fleets[0].mission_aux_bytes(), [0x01, 0x01]);
    assert_eq!(data.bases.len(), 1);
    assert_eq!(
        data.bases[0].escort_fleet_slot_raw(),
        data.fleets[0].local_slot_word_raw()
    );
    assert_eq!(
        data.bases[0].record_number_raw(),
        data.fleets[0].fleet_id_word_raw()
    );
    assert_eq!(data.bases[0].coords_raw(), [0x10, 0x0D]);

    data.set_ipbm_zero_records(2);
    assert_eq!(data.players[0].ipbm_chain_head_raw(), 2);
    assert_eq!(data.ipbms.len(), 2);
    assert_eq!(serialize_ipbm_dat(&data.ipbms).len(), 2 * IPBM_RECORD_SIZE);

    data.set_ipbm_record_prefix(2, 0x1234, 0x02, 0x4567, 0x89ab)
        .unwrap();
    let ipbm = &data.ipbms[1];
    assert_eq!(ipbm.ipbm_number_raw(), 0x1234);
    assert_eq!(ipbm.owner_empire_raw(), 0x02);
    assert_eq!(ipbm.next_ipbm_link_raw(), 0x4567);
    assert_eq!(ipbm.record_number_raw(), 0x89ab);
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

    let max_speed = data.fleets[0].max_speed();
    let error = data
        .set_fleet_order(
            1,
            max_speed.saturating_add(1),
            Order::MoveOnly.to_raw(),
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
        .find(|planet| planet.owner_empire_slot_raw() == 0)
        .map(|planet| planet.coords_raw())
        .expect("baseline should have an unowned planet");
    let first_speed = data.fleets[0].current_speed();
    let second_speed = data.fleets[1].current_speed();
    data.fleets[0].set_etac_count(1);
    data.fleets[1].set_etac_count(1);

    data.set_fleet_order(
        1,
        first_speed,
        Order::ColonizeWorld.to_raw(),
        target,
        None,
        None,
    )
    .expect("first colonize order should succeed");

    let error = data
        .set_fleet_order(
            2,
            second_speed,
            Order::ColonizeWorld.to_raw(),
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
    target_world.set_coords_raw(target);
    target_world.set_owner_empire_slot_raw(2);
    target_world.set_ownership_status_raw(2);
    target_world.set_planet_name("Target");
    target_world.set_army_count_raw(10);
    target_world.set_ground_batteries_raw(4);

    let fleet = &mut data.fleets[0];
    fleet.set_current_location_coords_raw(target);
    fleet.set_battleship_count(1);
    fleet.set_cruiser_count(0);
    fleet.set_destroyer_count(1);
    fleet.set_troop_transport_count(2);
    fleet.set_army_count(6);
    fleet.set_scout_count(0);
    fleet.set_etac_count(0);
    fleet.recompute_max_speed_from_composition();
    fleet.set_current_speed(fleet.max_speed());
    fleet.set_depth(Real48::ONE);

    (data, target)
}

#[test]
fn set_fleet_order_keeps_on_station_hostile_fleet_motion() {
    for order in [Order::BombardWorld, Order::InvadeWorld, Order::BlitzWorld] {
        let (mut data, target) = configure_on_station_hostile_order_state();

        data.set_fleet_order(
            1,
            data.fleets[0].max_speed(),
            order.to_raw(),
            target,
            None,
            None,
        )
        .expect("hostile order should be accepted");

        let fleet = &data.fleets[0];
        assert_eq!(fleet.standing_order_kind(), order);
        assert_eq!(fleet.standing_order_target_coords_raw(), target);
        assert_eq!(fleet.current_location_coords_raw(), target);
        // Like ECGAME, the order keeps the fleet's stored motion: at rest in
        // orbit, so its strike test passes on the next maintenance's first tick.
        assert_eq!(fleet.current_speed(), fleet.max_speed());
        assert_eq!(
            fleet.depth(),
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
    let _ = planet.set_present_production_points(42);
    planet.set_potential_production_points(77);
    planet.set_stored_production_points(1234);
    planet.set_build_count_raw(0, 5);
    planet.set_build_kind_raw(0, 3);
    planet.set_stardock_count_raw(4, 2);
    planet.set_army_count_raw(9);
    planet.set_ground_batteries_raw(4);

    data.scorch_planet_surface(15).unwrap();

    let planet = &data.planets[14];
    assert_eq!(planet.present_production_points(), Some(0));
    assert_eq!(planet.potential_production_points(), 0);
    assert_eq!(planet.stored_production_points(), 0);
    assert!((0..10).all(|slot| planet.build_count_raw(slot) == 0));
    assert!((0..10).all(|slot| planet.build_kind_raw(slot) == 0));
    assert!((0..STARDOCK_SLOT_COUNT).all(|slot| planet.stardock_count_raw(slot) == 0));
    assert!((0..STARDOCK_SLOT_COUNT).all(|slot| planet.stardock_kind_raw(slot) == 0));
    assert_eq!(planet.army_count_raw(), 9);
    assert_eq!(planet.ground_batteries_raw(), 4);
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
    fleet.set_destroyer_count(0);
    fleet.set_cruiser_count(0);
    fleet.set_battleship_count(0);
    fleet.set_scout_count(1);
    fleet.set_troop_transport_count(0);
    fleet.set_army_count(0);
    fleet.set_etac_count(0);

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
    fleet.set_destroyer_count(1);
    fleet.set_cruiser_count(0);
    fleet.set_battleship_count(0);
    fleet.set_scout_count(1);
    fleet.set_troop_transport_count(0);
    fleet.set_army_count(0);
    fleet.set_etac_count(0);

    data.set_fleet_rules_of_engagement(1, 1, 6).unwrap();

    assert_eq!(data.fleets[0].rules_of_engagement(), 6);
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
            reason: PlayerDiplomacyValidationError::SelfTarget { empire_raw: 1 },
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
    planet.set_build_count_raw(0, 4);
    planet.set_build_kind_raw(0, 1);
    planet.set_build_count_raw(1, 7);
    planet.set_build_kind_raw(1, 1);
    planet.set_build_count_raw(2, 5);
    planet.set_build_kind_raw(2, 7);

    let cleared = data
        .clear_planet_build_orders_by_kind(1, ProductionItemKind::Destroyer)
        .unwrap();

    assert_eq!(cleared, 2);
    assert_eq!(data.planets[0].build_count_raw(0), 0);
    assert_eq!(data.planets[0].build_kind_raw(0), 0);
    assert_eq!(data.planets[0].build_count_raw(1), 0);
    assert_eq!(data.planets[0].build_kind_raw(1), 0);
    assert_eq!(data.planets[0].build_count_raw(2), 5);
    assert_eq!(
        ProductionItemKind::from_raw(data.planets[0].build_kind_raw(2)),
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

    let host_coords = data.fleets[0].current_location_coords_raw();
    let host_fleet_id = data.fleets[0].fleet_id_word_raw();

    data.set_join_fleet_order(1, 2, 1).unwrap();

    assert_eq!(
        data.fleets[1].standing_order_kind(),
        crate::Order::JoinAnotherFleet
    );
    assert_eq!(
        data.fleets[1].standing_order_target_coords_raw(),
        host_coords
    );
    assert_eq!(data.fleets[1].join_host_fleet_id_raw(), host_fleet_id);
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
    let host_id = data.fleets[0].fleet_id() as u8;
    let owner = data.fleets[1].owner_empire_raw();
    let planet_coords = |predicate: &dyn Fn(u8) -> bool| {
        data.planets
            .iter()
            .find(|planet| predicate(planet.owner_empire_slot_raw()))
            .map(|planet| planet.coords_raw())
            .expect("fixture planet")
    };
    let owned = planet_coords(&|planet_owner| planet_owner == owner);
    let foreign = planet_coords(&|planet_owner| planet_owner != 0 && planet_owner != owner);
    let sector = [1, 1];
    {
        let fleet = &mut data.fleets[1];
        fleet.set_scout_count(1);
        fleet.set_destroyer_count(1);
        fleet.set_troop_transport_count(1);
        fleet.set_army_count(1);
        fleet.set_etac_count(1);
    }

    let cases = [
        (Order::HoldPosition, sector, None, [1, 0]),
        (Order::MoveOnly, sector, None, [0, 0]),
        (Order::SeekHome, owned, None, [1, 0]),
        (Order::PatrolSector, sector, None, [0, 0]),
        (
            Order::GuardStarbase,
            base.coords_raw(),
            Some(base.base_id_raw() as u8),
            [1, base.base_id_raw() as u8],
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
        data.fleets[1].set_mission_aux_bytes([0xfe, 0xfe]);
        let aux = data
            .set_fleet_order(2, 1, order.to_raw(), target, None, aux1)
            .unwrap_or_else(|err| panic!("{order:?}: {err}"));
        assert_eq!(aux, expected, "{order:?}");
        assert_eq!(data.fleets[1].mission_aux_bytes(), expected, "{order:?}");
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
    let current_speed = data.fleets[1].current_speed();
    data.fleets[1].set_standing_order_kind(Order::HoldPosition);
    data.fleets[1].set_standing_order_target_coords_raw([0, 0]);
    data.fleets[1].set_mission_aux_bytes([0, 0]);

    data.set_fleet_order(
        2,
        current_speed,
        Order::GuardStarbase.to_raw(),
        base.coords_raw(),
        Some(1),
        Some(base.base_id_raw() as u8),
    )
    .unwrap();

    assert_eq!(data.fleets[1].standing_order_kind(), Order::GuardStarbase);
    assert_eq!(
        data.fleets[1].standing_order_target_coords_raw(),
        base.coords_raw()
    );
    assert_eq!(
        data.fleets[1].mission_aux_bytes(),
        [1, base.base_id_raw() as u8]
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
    let current_speed = data.fleets[1].current_speed();
    data.fleets[1].set_standing_order_kind(Order::HoldPosition);
    data.fleets[1].set_standing_order_target_coords_raw([0, 0]);
    data.fleets[1].set_mission_aux_bytes([0, 0]);

    data.set_fleet_order(
        2,
        current_speed,
        Order::JoinAnotherFleet.to_raw(),
        host.current_location_coords_raw(),
        None,
        Some(host.fleet_id() as u8),
    )
    .unwrap();

    assert_eq!(
        data.fleets[1].standing_order_kind(),
        Order::JoinAnotherFleet
    );
    assert_eq!(
        data.fleets[1].standing_order_target_coords_raw(),
        host.current_location_coords_raw()
    );
    assert_eq!(
        data.fleets[1].join_host_fleet_id_raw(),
        host.fleet_id_word_raw()
    );
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
    assert!(
        post_data
            .ipbm_count_length_errors_current_known()
            .is_empty()
    );
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
    let mut player = Player::default();
    player.set_owner_empire_raw(1);
    player.set_fleet_chain_head_raw(1);

    let mut donor = Fleet::default();
    donor.set_owner_empire_raw(1);
    donor.set_local_slot_word_raw(1);
    donor.set_fleet_id_word_raw(1);
    donor.set_current_location_coords_raw([24, 14]);
    donor.set_at_rest_motion();
    donor.set_standing_order_kind(Order::GuardBlockadeWorld);
    donor.set_standing_order_target_coords_raw([25, 12]);
    donor.set_destroyer_count(2);
    donor.set_cruiser_count(1);
    donor.set_troop_transport_count(2);
    donor.set_army_count(1);
    donor.set_etac_count(1);
    donor.set_rules_of_engagement(6);
    donor.recompute_max_speed_from_composition();
    donor.set_current_speed(5);

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
    assert_eq!(donor.destroyer_count(), 1);
    assert_eq!(donor.cruiser_count(), 1);
    assert_eq!(donor.troop_transport_count(), 1);
    assert_eq!(donor.army_count(), 0);
    assert_eq!(donor.etac_count(), 0);
    assert_eq!(donor.standing_order_kind(), Order::GuardBlockadeWorld);
    assert_eq!(donor.standing_order_target_coords_raw(), [25, 12]);

    let detached = &data.fleets[1];
    assert_eq!(detached.local_slot_word_raw(), 2);
    assert_eq!(detached.fleet_id_word_raw(), 2);
    assert_eq!(detached.current_location_coords_raw(), [24, 14]);
    assert_eq!(detached.destroyer_count(), 1);
    assert_eq!(detached.troop_transport_count(), 1);
    assert_eq!(detached.army_count(), 1);
    assert_eq!(detached.etac_count(), 1);
    assert_eq!(detached.rules_of_engagement(), 4);
    assert_eq!(detached.standing_order_kind(), Order::HoldPosition);
    assert_eq!(detached.current_speed(), 0);
    assert_eq!(data.players[0].fleet_chain_head_raw(), 1);
}

#[test]
fn detach_ships_clamps_support_only_new_fleet_to_roe_zero() {
    let mut player = Player::default();
    player.set_owner_empire_raw(1);
    player.set_fleet_chain_head_raw(1);

    let mut donor = Fleet::default();
    donor.set_owner_empire_raw(1);
    donor.set_local_slot_word_raw(1);
    donor.set_fleet_id_word_raw(1);
    donor.set_current_location_coords_raw([24, 14]);
    donor.set_destroyer_count(1);
    donor.set_troop_transport_count(1);
    donor.set_army_count(1);
    donor.set_scout_count(1);
    donor.recompute_max_speed_from_composition();
    donor.set_current_speed(3);
    donor.set_rules_of_engagement(6);

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
    assert_eq!(data.fleets[0].rules_of_engagement(), 6);
    assert_eq!(data.fleets[1].scout_count(), 1);
    assert_eq!(data.fleets[1].rules_of_engagement(), 0);
}

#[test]
fn detach_ships_requires_valid_post_split_donor_speed() {
    let mut player = Player::default();
    player.set_owner_empire_raw(1);
    player.set_fleet_chain_head_raw(1);

    let mut donor = Fleet::default();
    donor.set_owner_empire_raw(1);
    donor.set_local_slot_word_raw(1);
    donor.set_fleet_id_word_raw(1);
    donor.set_destroyer_count(1);
    donor.set_cruiser_count(1);
    donor.recompute_max_speed_from_composition();
    donor.set_current_speed(6);

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
    let mut player = Player::default();
    player.set_owner_empire_raw(1);
    player.set_fleet_chain_head_raw(1);

    let mut donor = Fleet::default();
    donor.set_owner_empire_raw(1);
    donor.set_local_slot_word_raw(1);
    donor.set_fleet_id_word_raw(1);
    donor.set_current_location_coords_raw([24, 14]);
    donor.set_standing_order_kind(Order::ColonizeWorld);
    donor.set_standing_order_target_coords_raw([25, 12]);
    donor.set_mission_aux_bytes([7, 9]);
    donor.set_cruiser_count(1);
    donor.set_etac_count(1);
    donor.recompute_max_speed_from_composition();
    donor.set_current_speed(3);

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
    assert_eq!(donor.etac_count(), 0);
    assert_eq!(donor.standing_order_kind(), Order::HoldPosition);
    assert_eq!(donor.current_speed(), 0);
    assert_eq!(donor.standing_order_target_coords_raw(), [24, 14]);
    assert_eq!(donor.mission_aux_bytes(), [0, 0]);
}

#[test]
fn transfer_ships_between_fleets_moves_counts_and_preserves_both_fleets() {
    let mut player = Player::default();
    player.set_owner_empire_raw(1);
    player.set_fleet_chain_head_raw(1);

    let mut donor = Fleet::default();
    donor.set_owner_empire_raw(1);
    donor.set_local_slot_word_raw(1);
    donor.set_fleet_id_word_raw(1);
    donor.set_current_location_coords_raw([24, 14]);
    donor.set_destroyer_count(2);
    donor.set_troop_transport_count(2);
    donor.set_army_count(1);
    donor.set_etac_count(1);
    donor.recompute_max_speed_from_composition();
    donor.set_current_speed(5);

    let mut host = Fleet::default();
    host.set_owner_empire_raw(1);
    host.set_local_slot_word_raw(2);
    host.set_fleet_id_word_raw(2);
    host.set_current_location_coords_raw([24, 14]);
    host.set_cruiser_count(1);
    host.recompute_max_speed_from_composition();
    host.set_current_speed(5);

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
    assert_eq!(donor.destroyer_count(), 1);
    assert_eq!(donor.troop_transport_count(), 1);
    assert_eq!(donor.army_count(), 0);

    let host = &data.fleets[1];
    assert_eq!(host.cruiser_count(), 1);
    assert_eq!(host.destroyer_count(), 1);
    assert_eq!(host.troop_transport_count(), 1);
    assert_eq!(host.army_count(), 1);
}

#[test]
fn transfer_ships_invalidated_donor_scout_order_falls_back_to_local_hold() {
    let mut player = Player::default();
    player.set_owner_empire_raw(1);
    player.set_fleet_chain_head_raw(1);

    let mut donor = Fleet::default();
    donor.set_owner_empire_raw(1);
    donor.set_local_slot_word_raw(1);
    donor.set_fleet_id_word_raw(1);
    donor.set_current_location_coords_raw([24, 14]);
    donor.set_standing_order_kind(Order::ScoutSector);
    donor.set_standing_order_target_coords_raw([25, 12]);
    donor.set_scout_count(1);
    donor.set_destroyer_count(1);
    donor.recompute_max_speed_from_composition();
    donor.set_current_speed(6);

    let mut host = Fleet::default();
    host.set_owner_empire_raw(1);
    host.set_local_slot_word_raw(2);
    host.set_fleet_id_word_raw(2);
    host.set_current_location_coords_raw([24, 14]);
    host.set_cruiser_count(1);
    host.recompute_max_speed_from_composition();
    host.set_current_speed(5);

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
    assert_eq!(donor.scout_count(), 0);
    assert_eq!(donor.standing_order_kind(), Order::HoldPosition);
    assert_eq!(donor.current_speed(), 0);
    assert_eq!(donor.standing_order_target_coords_raw(), [24, 14]);
}

#[test]
fn transfer_ships_clamps_support_only_resulting_fleet_to_roe_zero() {
    let mut player = Player::default();
    player.set_owner_empire_raw(1);
    player.set_fleet_chain_head_raw(1);

    let mut donor = Fleet::default();
    donor.set_owner_empire_raw(1);
    donor.set_local_slot_word_raw(1);
    donor.set_fleet_id_word_raw(1);
    donor.set_current_location_coords_raw([24, 14]);
    donor.set_destroyer_count(1);
    donor.set_scout_count(1);
    donor.recompute_max_speed_from_composition();
    donor.set_current_speed(3);
    donor.set_rules_of_engagement(6);

    let mut host = Fleet::default();
    host.set_owner_empire_raw(1);
    host.set_local_slot_word_raw(2);
    host.set_fleet_id_word_raw(2);
    host.set_current_location_coords_raw([24, 14]);
    host.set_destroyer_count(1);
    host.recompute_max_speed_from_composition();
    host.set_current_speed(6);
    host.set_rules_of_engagement(6);

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

    assert_eq!(data.fleets[0].scout_count(), 1);
    assert_eq!(data.fleets[0].rules_of_engagement(), 0);
    assert_eq!(data.fleets[1].destroyer_count(), 2);
    assert_eq!(data.fleets[1].rules_of_engagement(), 6);
}

#[test]
fn unload_armies_invalidated_invade_order_falls_back_to_local_hold() {
    let mut player = Player::default();
    player.set_owner_empire_raw(1);

    let mut planet = Planet::default();
    planet.set_owner_empire_slot_raw(1);
    planet.set_coords_raw([24, 14]);
    planet.set_army_count_raw(0);

    let mut fleet = Fleet::default();
    fleet.set_owner_empire_raw(1);
    fleet.set_local_slot_word_raw(1);
    fleet.set_fleet_id_word_raw(1);
    fleet.set_current_location_coords_raw([24, 14]);
    fleet.set_standing_order_kind(Order::InvadeWorld);
    fleet.set_standing_order_target_coords_raw([25, 12]);
    fleet.set_destroyer_count(1);
    fleet.set_troop_transport_count(1);
    fleet.set_army_count(1);
    fleet.recompute_max_speed_from_composition();
    fleet.set_current_speed(5);

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
    assert_eq!(fleet.army_count(), 0);
    assert_eq!(fleet.standing_order_kind(), Order::HoldPosition);
    assert_eq!(fleet.current_speed(), 0);
    assert_eq!(fleet.standing_order_target_coords_raw(), [24, 14]);
}

#[test]
fn append_planet_build_order_rejects_non_multiple_surface_points() {
    let mut player = Player::default();
    player.set_owner_empire_raw(1);

    let mut planet = Planet::default();
    planet.set_owner_empire_slot_raw(1);

    let mut data = CoreGameData {
        players: vec![player],
        planets: vec![planet],
        fleets: vec![],
        bases: vec![],
        ipbms: vec![],
        setup: Setup::default(),
        conquest: Conquest::default(),
    };

    for (points, kind_raw) in [(1, 7), (21, 7), (1, 8), (3, 8)] {
        let error = data
            .append_planet_build_order(1, points, kind_raw)
            .unwrap_err();
        assert_eq!(
            error,
            GameStateMutationError::InvalidPlanetPlayerInput {
                planet_index_1_based: 1,
                reason: PlanetPlayerInputValidationError::InvalidBuildPointsForKind {
                    kind_raw,
                    points_remaining_raw: points,
                },
            }
        );
    }
    assert_eq!(data.planets[0].build_count_raw(0), 0);

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
    assert_eq!(data.fleets[1].join_host_fleet_id_raw(), 3);
    assert_eq!(data.fleets[5].join_host_fleet_id_raw(), 3);

    data.set_fleet_local_slot(1, 3, 12)
        .expect("renumber fleet 3");

    assert_eq!(data.fleets[2].local_slot_word_raw(), 12);
    assert_eq!(data.fleets[1].join_host_fleet_id_raw(), 12);
    // Empire 2's join names its own fleet 3, which kept its number.
    assert_eq!(data.fleets[5].join_host_fleet_id_raw(), 3);
}
