mod common;

use common::production::{
    commissioned_starbase, configured_conquest, owned_homeworld_seed, owned_planet,
    player_with_empire_name, single_planet_game, zeroed_conquest, zeroed_setup,
};
use nc_data::{
    BaseDat, CoreGameData, EmpireProductionRankingSort, FleetDat, IpbmDat, PlanetDat, PlayerDat,
    Real48, starbase_growth_bonus_percent,
};
use nc_engine::{build_seeded_initialized_game, build_seeded_new_game, run_maintenance_turn};

#[test]
fn real48_reads_current_known_homeworld_values() {
    let fifty = Real48::from_bytes([0x86, 0x00, 0x00, 0x00, 0x00, 0x48]).to_f64();
    let hundred = Real48::from_bytes([0x87, 0x00, 0x00, 0x00, 0x00, 0x48]).to_f64();

    assert_eq!(fifty, 50.0);
    assert_eq!(hundred, 100.0);
}

#[test]
fn real48_round_trips_common_production_values() {
    for points in [0.0, 1.0, 25.0, 50.0, 75.0, 100.0] {
        assert_eq!(Real48::from_f64(points).to_f64(), points);
    }
}

#[test]
fn empire_economy_helpers_use_classic_production_terms() {
    let mut player1 = player_with_empire_name("Alpha", 50, 0);
    let mut player2 = player_with_empire_name("Beta", 60, 20);
    player1.set_owner_empire_raw(1);
    player2.set_owner_empire_raw(1);

    let game = CoreGameData {
        player: PlayerDat {
            records: vec![player1, player2],
        },
        planets: PlanetDat {
            records: vec![
                owned_planet(1, 100, 100.0, 20, 10, 4),
                owned_planet(1, 50, 25.0, 15, 5, 1),
                owned_planet(2, 200, 200.0, 25, 8, 2),
            ],
        },
        fleets: FleetDat { records: vec![] },
        bases: BaseDat { records: vec![] },
        ipbm: IpbmDat { records: vec![] },
        setup: zeroed_setup(),
        conquest: zeroed_conquest(),
    };

    let economy = game.empire_economy_summary(1);
    assert_eq!(economy.owned_planets, 2);
    assert_eq!(economy.present_production, 125);
    assert_eq!(economy.potential_production, 150);
    assert_eq!(economy.total_available_points, 62);
    assert!((economy.efficiency_percent - 83.333).abs() < 0.01);
    assert_eq!(economy.rank_by_planets, 1);
    assert_eq!(economy.rank_by_present_production, 2);

    let rankings = game.empire_production_ranking_rows(EmpireProductionRankingSort::Production);
    assert_eq!(rankings[0].empire_name, "Beta");
    assert_eq!(rankings[0].current_production, 200);
    assert_eq!(rankings[1].empire_name, "Alpha");
    assert_eq!(rankings[1].current_production, 125);
}

#[test]
fn total_available_points_matches_first_turn_tax_budget() {
    let mut player = player_with_empire_name("Alpha", 50, 0);
    player.set_owner_empire_raw(1);

    let game = CoreGameData {
        player: PlayerDat {
            records: vec![player],
        },
        planets: PlanetDat {
            records: vec![owned_homeworld_seed(1, 100, 100.0, 10, 4)],
        },
        fleets: FleetDat { records: vec![] },
        bases: BaseDat { records: vec![] },
        ipbm: IpbmDat { records: vec![] },
        setup: zeroed_setup(),
        conquest: zeroed_conquest(),
    };

    assert_eq!(game.empire_present_production(1), 100);
    assert_eq!(game.empire_available_production_points(1), 50);
}

#[test]
fn named_homeworld_present_production_reads_its_stored_value() {
    let mut planet = owned_homeworld_seed(1, 100, 100.0, 10, 4);
    planet.set_planet_name("Dust Bowl");
    planet.set_present_production(50.0);
    assert_eq!(planet.present_production_points(), Some(50));
}

#[test]
fn naming_a_homeworld_seed_keeps_its_full_production() {
    let mut planet = owned_homeworld_seed(1, 100, 100.0, 10, 4);
    planet.set_planet_name("Codex Prime");
    assert_eq!(planet.present_production_points(), Some(100));
}

#[test]
fn joined_homeworld_high_tax_penalty_compounds_each_turn() {
    let mut game = build_seeded_initialized_game(4, 3000, 1515).expect("seeded game should build");
    game.player.records[1].set_tax_rate_raw(100);

    run_maintenance_turn(&mut game).expect("maintenance should succeed");
    assert_eq!(
        game.planets.records[1].present_production_points(),
        Some(93)
    );
    run_maintenance_turn(&mut game).expect("maintenance should succeed");
    assert_eq!(
        game.planets.records[1].present_production_points(),
        Some(87)
    );
}

#[test]
fn maintenance_recomputes_player_production_from_present_production() {
    let mut game = build_seeded_new_game(4, 3000, 1515).expect("seeded game should build");
    run_maintenance_turn(&mut game).expect("maintenance should succeed");
    assert_eq!(game.player.records[0].raw[0x52], 100);
}

#[test]
fn maintenance_adds_tax_revenue_and_grows_planets_faster_under_lower_tax() {
    let mut low_tax = player_with_empire_name("Alpha", 25, 0);
    low_tax.set_owner_empire_raw(1);
    let mut high_tax = low_tax.clone();
    high_tax.set_tax_rate_raw(80);

    let colony = owned_planet(1, 100, 25.0, 0, 1, 0);

    let mut low_game = CoreGameData {
        player: PlayerDat {
            records: vec![low_tax],
        },
        planets: PlanetDat {
            records: vec![colony.clone()],
        },
        fleets: FleetDat { records: vec![] },
        bases: BaseDat { records: vec![] },
        ipbm: IpbmDat { records: vec![] },
        setup: zeroed_setup(),
        conquest: configured_conquest(1),
    };
    let mut high_game = CoreGameData {
        player: PlayerDat {
            records: vec![high_tax],
        },
        planets: PlanetDat {
            records: vec![colony],
        },
        fleets: FleetDat { records: vec![] },
        bases: BaseDat { records: vec![] },
        ipbm: IpbmDat { records: vec![] },
        setup: zeroed_setup(),
        conquest: configured_conquest(1),
    };

    run_maintenance_turn(&mut low_game).expect("maintenance should succeed");
    run_maintenance_turn(&mut high_game).expect("maintenance should succeed");

    let low_planet = &low_game.planets.records[0];
    let high_planet = &high_game.planets.records[0];
    assert_eq!(low_planet.stored_production_points(), 10);
    assert_eq!(high_planet.stored_production_points(), 22);
    assert!(
        low_planet.present_production_points().unwrap()
            > high_planet.present_production_points().unwrap()
    );
}

#[test]
fn maintenance_starbase_growth_bonus_accelerates_planet_development() {
    let mut player = player_with_empire_name("Alpha", 50, 0);
    player.set_owner_empire_raw(1);

    let colony = owned_planet(1, 100, 50.0, 0, 3, 1);
    let mut with_base = CoreGameData {
        player: PlayerDat {
            records: vec![player.clone()],
        },
        planets: PlanetDat {
            records: vec![colony.clone()],
        },
        fleets: FleetDat { records: vec![] },
        bases: BaseDat {
            records: vec![commissioned_starbase(1, [0, 0])],
        },
        ipbm: IpbmDat { records: vec![] },
        setup: zeroed_setup(),
        conquest: configured_conquest(1),
    };
    with_base.planets.records[0].set_coords_raw([0, 0]);

    let mut without_base = CoreGameData {
        player: PlayerDat {
            records: vec![player],
        },
        planets: PlanetDat {
            records: vec![colony],
        },
        fleets: FleetDat { records: vec![] },
        bases: BaseDat { records: vec![] },
        ipbm: IpbmDat { records: vec![] },
        setup: zeroed_setup(),
        conquest: configured_conquest(1),
    };

    run_maintenance_turn(&mut with_base).expect("maintenance should succeed");
    run_maintenance_turn(&mut without_base).expect("maintenance should succeed");

    assert!(
        with_base.planets.records[0]
            .present_production_points()
            .unwrap()
            > without_base.planets.records[0]
                .present_production_points()
                .unwrap()
    );
}

#[test]
fn maintenance_high_tax_above_65_can_reduce_present_production() {
    let mut player = player_with_empire_name("Alpha", 80, 0);
    player.set_owner_empire_raw(1);

    let colony = owned_planet(1, 100, 25.0, 0, 1, 0);
    let mut game = CoreGameData {
        player: PlayerDat {
            records: vec![player],
        },
        planets: PlanetDat {
            records: vec![colony],
        },
        fleets: FleetDat { records: vec![] },
        bases: BaseDat { records: vec![] },
        ipbm: IpbmDat { records: vec![] },
        setup: zeroed_setup(),
        conquest: configured_conquest(1),
    };

    run_maintenance_turn(&mut game).expect("maintenance should succeed");

    let planet = &game.planets.records[0];
    assert_eq!(planet.stored_production_points(), 22);
    assert_eq!(planet.present_production_points().unwrap(), 28);
}

#[test]
fn starbase_growth_bonus_tapers_out_by_tax_65() {
    assert_eq!(starbase_growth_bonus_percent(25), 50);
    assert_eq!(starbase_growth_bonus_percent(50), 50);
    assert_eq!(starbase_growth_bonus_percent(60), 16);
    assert_eq!(starbase_growth_bonus_percent(64), 3);
    assert_eq!(starbase_growth_bonus_percent(65), 0);
    assert_eq!(starbase_growth_bonus_percent(70), 0);
}

#[test]
fn maintenance_starbases_do_not_shift_the_high_tax_penalty_threshold() {
    let mut player = player_with_empire_name("Alpha", 70, 0);
    player.set_owner_empire_raw(1);

    let colony = owned_planet(1, 100, 50.0, 0, 3, 1);
    let mut with_base = CoreGameData {
        player: PlayerDat {
            records: vec![player.clone()],
        },
        planets: PlanetDat {
            records: vec![colony.clone()],
        },
        fleets: FleetDat { records: vec![] },
        bases: BaseDat {
            records: vec![commissioned_starbase(1, [0, 0])],
        },
        ipbm: IpbmDat { records: vec![] },
        setup: zeroed_setup(),
        conquest: configured_conquest(1),
    };
    with_base.planets.records[0].set_coords_raw([0, 0]);

    let mut without_base = CoreGameData {
        player: PlayerDat {
            records: vec![player],
        },
        planets: PlanetDat {
            records: vec![colony],
        },
        fleets: FleetDat { records: vec![] },
        bases: BaseDat { records: vec![] },
        ipbm: IpbmDat { records: vec![] },
        setup: zeroed_setup(),
        conquest: configured_conquest(1),
    };

    run_maintenance_turn(&mut with_base).expect("maintenance should succeed");
    run_maintenance_turn(&mut without_base).expect("maintenance should succeed");

    assert_eq!(with_base.planets.records[0].stored_production_points(), 37);
    assert_eq!(
        without_base.planets.records[0].stored_production_points(),
        37
    );
    assert_eq!(
        with_base.planets.records[0]
            .present_production_points()
            .unwrap(),
        53
    );
    assert_eq!(
        with_base.planets.records[0]
            .present_production_points()
            .unwrap(),
        without_base.planets.records[0]
            .present_production_points()
            .unwrap()
    );
}

#[test]
fn maintenance_starbases_still_help_at_moderate_tax_rates() {
    let mut player = player_with_empire_name("Alpha", 50, 0);
    player.set_owner_empire_raw(1);

    let colony = owned_planet(1, 100, 50.0, 0, 3, 1);
    let mut with_base = CoreGameData {
        player: PlayerDat {
            records: vec![player.clone()],
        },
        planets: PlanetDat {
            records: vec![colony.clone()],
        },
        fleets: FleetDat { records: vec![] },
        bases: BaseDat {
            records: vec![commissioned_starbase(1, [0, 0])],
        },
        ipbm: IpbmDat { records: vec![] },
        setup: zeroed_setup(),
        conquest: configured_conquest(1),
    };
    with_base.planets.records[0].set_coords_raw([0, 0]);

    let mut without_base = CoreGameData {
        player: PlayerDat {
            records: vec![player],
        },
        planets: PlanetDat {
            records: vec![colony],
        },
        fleets: FleetDat { records: vec![] },
        bases: BaseDat { records: vec![] },
        ipbm: IpbmDat { records: vec![] },
        setup: zeroed_setup(),
        conquest: configured_conquest(1),
    };

    run_maintenance_turn(&mut with_base).expect("maintenance should succeed");
    run_maintenance_turn(&mut without_base).expect("maintenance should succeed");

    assert_eq!(with_base.planets.records[0].stored_production_points(), 30);
    assert_eq!(
        without_base.planets.records[0].stored_production_points(),
        28
    );
    assert_eq!(
        without_base.planets.records[0]
            .present_production_points()
            .unwrap(),
        57
    );
    assert_eq!(
        with_base.planets.records[0]
            .present_production_points()
            .unwrap(),
        61
    );
}

#[test]
fn maintenance_exact_ship_quantities_arrive_in_stardock() {
    let mut player = player_with_empire_name("Alpha", 50, 0);
    player.set_owner_empire_raw(1);

    let mut planet = owned_planet(1, 100, 100.0, 100, 1, 0);
    planet.set_build_count_raw(0, 10);
    planet.set_build_kind_raw(0, 1);
    planet.set_build_count_raw(1, 40);
    planet.set_build_kind_raw(1, 6);

    let mut game = single_planet_game(player, planet);
    run_maintenance_turn(&mut game).expect("maintenance should succeed");

    let planet = &game.planets.records[0];
    assert_eq!(planet.build_count_raw(0), 0);
    assert_eq!(planet.build_kind_raw(0), 0);
    assert_eq!(planet.build_count_raw(1), 0);
    assert_eq!(planet.build_kind_raw(1), 0);
    assert_eq!(planet.stardock_kind_raw(0), 1);
    assert_eq!(planet.stardock_count_raw(0), 2);
    assert_eq!(planet.stardock_kind_raw(5), 6);
    assert_eq!(planet.stardock_count_raw(5), 2);
}

#[test]
fn maintenance_starbases_arrive_in_the_starbase_stardock_slot() {
    let mut player = player_with_empire_name("Alpha", 50, 0);
    player.set_owner_empire_raw(1);

    let mut planet = owned_planet(1, 100, 100.0, 100, 1, 0);
    planet.set_build_count_raw(0, 50);
    planet.set_build_kind_raw(0, 9);
    planet.set_build_count_raw(1, 50);
    planet.set_build_kind_raw(1, 9);
    planet.set_build_count_raw(2, 50);
    planet.set_build_kind_raw(2, 9);

    let mut game = single_planet_game(player, planet);
    run_maintenance_turn(&mut game).expect("maintenance should succeed");

    let planet = &game.planets.records[0];
    assert_eq!(planet.stardock_kind_raw(6), 9);
    assert_eq!(planet.stardock_count_raw(6), 2);
    assert_eq!(planet.build_count_raw(2), 50);
    assert_eq!(planet.build_kind_raw(2), 9);
}

#[test]
fn maintenance_army_build_completes_even_when_stardock_is_full() {
    let mut player = player_with_empire_name("Alpha", 50, 0);
    player.set_owner_empire_raw(1);

    let mut planet = owned_planet(1, 100, 100.0, 100, 1, 0);
    planet.set_build_count_raw(0, 100);
    planet.set_build_kind_raw(0, 8);
    for slot in 0..nc_data::STARDOCK_SLOT_COUNT {
        planet.set_stardock_count_raw(slot, 1);
    }

    let mut game = single_planet_game(player, planet);
    run_maintenance_turn(&mut game).expect("maintenance should succeed");

    let planet = &game.planets.records[0];
    assert_eq!(planet.build_count_raw(0), 0);
    assert_eq!(planet.build_kind_raw(0), 0);
    assert_eq!(planet.army_count_raw(), 51);
}

#[test]
fn maintenance_ground_battery_build_completes_even_when_stardock_is_full() {
    let mut player = player_with_empire_name("Alpha", 50, 0);
    player.set_owner_empire_raw(1);

    let mut planet = owned_planet(1, 100, 100.0, 100, 1, 0);
    planet.set_build_count_raw(0, 100);
    planet.set_build_kind_raw(0, 7);
    for slot in 0..nc_data::STARDOCK_SLOT_COUNT {
        planet.set_stardock_count_raw(slot, 1);
    }

    let mut game = single_planet_game(player, planet);
    run_maintenance_turn(&mut game).expect("maintenance should succeed");

    let planet = &game.planets.records[0];
    assert_eq!(planet.build_count_raw(0), 0);
    assert_eq!(planet.build_kind_raw(0), 0);
    assert_eq!(planet.ground_batteries_raw(), 5);
}

#[test]
fn maintenance_army_build_waits_at_planet_cap() {
    let mut player = player_with_empire_name("Alpha", 50, 0);
    player.set_owner_empire_raw(1);

    let mut planet = owned_planet(1, 100, 100.0, 100, u8::MAX, 0);
    planet.set_build_count_raw(0, 100);
    planet.set_build_kind_raw(0, 8);

    let mut game = single_planet_game(player, planet);
    run_maintenance_turn(&mut game).expect("maintenance should succeed");

    let planet = &game.planets.records[0];
    assert_eq!(planet.build_count_raw(0), 100);
    assert_eq!(planet.build_kind_raw(0), 8);
    assert_eq!(planet.army_count_raw(), u8::MAX);
}

#[test]
fn maintenance_ground_battery_build_waits_at_planet_cap() {
    let mut player = player_with_empire_name("Alpha", 50, 0);
    player.set_owner_empire_raw(1);

    let mut planet = owned_planet(1, 100, 100.0, 100, 0, u8::MAX);
    planet.set_build_count_raw(0, 100);
    planet.set_build_kind_raw(0, 7);

    let mut game = single_planet_game(player, planet);
    run_maintenance_turn(&mut game).expect("maintenance should succeed");

    let planet = &game.planets.records[0];
    assert_eq!(planet.build_count_raw(0), 100);
    assert_eq!(planet.build_kind_raw(0), 7);
    assert_eq!(planet.ground_batteries_raw(), u8::MAX);
}
