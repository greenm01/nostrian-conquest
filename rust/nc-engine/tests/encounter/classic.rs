//! Whole maintenance years over many seeds, against the classic probe
//! numbers in the encounter spec. Rust matches classic's odds, not its
//! random sequence, so these check rates with binomial tolerances.

use nc_data::Order;

use super::support::{
    Fleet, NO_SHIPS, Ships, WORLD, add_starbase, assert_rate, attacks, board, board_with_world, ca,
    declared, first_contact, fought, put, within, years,
};

const SEEDS: std::ops::Range<u64> = 0..2000;

/// The spec's sighting probe: four sectors, one pair of fleets in each,
/// every fleet ROE 0 so nobody fights. Each line is (viewer, sector,
/// per-tick percent from the formulas; classic's measured percent).
#[test]
fn sighting_odds_match_classic() {
    let (a, b, c, d) = ([9, 10], [10, 14], [11, 16], [14, 9]);
    let mut scenario = board();
    let hold = |owner, at, in_system, ships| {
        Fleet::on_station(owner, at, in_system, Order::HoldPosition, ships)
    };
    put(&mut scenario, 0, hold(1, a, false, ca(1)));
    put(
        &mut scenario,
        4,
        Fleet::on_station(
            2,
            a,
            false,
            Order::PatrolSector,
            Ships { dd: 1, ..NO_SHIPS },
        )
        .max_speed(6),
    );
    put(&mut scenario, 1, hold(1, b, true, ca(1)));
    put(
        &mut scenario,
        5,
        Fleet::on_station(
            2,
            b,
            true,
            Order::GuardBlockadeWorld,
            Ships { bb: 1, ..NO_SHIPS },
        )
        .max_speed(4),
    );
    put(
        &mut scenario,
        2,
        Fleet::on_station(
            1,
            c,
            false,
            Order::PatrolSector,
            Ships {
                ca: 2,
                sc: 3,
                ..NO_SHIPS
            },
        )
        .max_speed(5),
    );
    put(
        &mut scenario,
        6,
        hold(
            2,
            c,
            false,
            Ships {
                dd: 1,
                sc: 2,
                ..NO_SHIPS
            },
        )
        .max_speed(6),
    );
    put(&mut scenario, 3, hold(1, d, true, ca(1)));
    put(
        &mut scenario,
        7,
        hold(
            2,
            d,
            true,
            Ships {
                et: 1,
                tt: 1,
                ..NO_SHIPS
            },
        ),
    );

    // (viewer, sector, formula percent, classic measured percent)
    let pairs: [(u8, [u8; 2], f64, f64); 8] = [
        (2, a, 1.053, 0.981), // Patrol sees 1 CA holding in deep space
        (1, a, 0.149, 0.098), // Hold sees a patrolling DD (x2)
        (2, b, 2.633, 2.696), // Guard (factor 5) sees 1 CA in orbit (x5)
        (1, b, 0.710, 0.625), // Hold sees a guarding BB in orbit (x5 x1.1)
        (1, c, 1.216, 1.240), // Patrol sees 1 DD + 2 SC holding
        (2, c, 0.413, 0.476), // Hold sees a patrol of 2 CA + 3 SC
        (2, d, 0.527, 0.561), // Hold sees 1 CA in orbit
        (1, d, 0.833, 0.721), // Hold sees 1 ETAC + 1 TT in orbit
    ];
    let mut contacts = [0usize; 8];
    let mut exposure = [0u32; 8];
    let mut runs = 0;
    for (_, events) in years(&scenario, &declared(&[]), SEEDS) {
        runs += 1;
        for (i, &(viewer, sector, _, _)) in pairs.iter().enumerate() {
            match first_contact(&events, viewer, sector) {
                Some(tick) => {
                    contacts[i] += 1;
                    exposure[i] += u32::from(tick);
                }
                None => exposure[i] += 52,
            }
        }
    }
    for (i, &(viewer, sector, percent, classic)) in pairs.iter().enumerate() {
        let measured = 100.0 * contacts[i] as f64 / f64::from(exposure[i]);
        eprintln!(
            "empire {viewer} at {sector:?}: formula {percent:.3}%  classic {classic:.3}%  rust {measured:.3}%  ({} of {runs} years)",
            contacts[i]
        );
        assert_rate(
            &format!("empire {viewer} sights at {sector:?}"),
            contacts[i],
            runs,
            within(percent, 52),
            4.0,
        );
    }
}

/// The spec's blockade probes at a world of empire 2: empire 1 blockades it
/// (Guard/Blockade, ROE 6, 3 CA) and empire 2's 2 CA leave for deep space at
/// speed 3, or hold. Classic fought in 0, 0, 1, 0, and 8 of 10 years.
#[test]
fn blockade_catches_only_declared_enemies_as_classic() {
    let blockade = Fleet::on_station(1, WORLD, true, Order::GuardBlockadeWorld, ca(3)).roe(6);
    let leaver = |roe| {
        Fleet::on_station(2, WORLD, true, Order::HoldPosition, ca(2))
            .roe(roe)
            .heading(Order::MoveOnly, [13, 13], false, 3)
    };
    let holder = Fleet::on_station(2, WORLD, true, Order::HoldPosition, ca(2));
    let rate = |visitor: Fleet, enemies: &[(u8, u8)]| {
        let mut scenario = board_with_world();
        put(&mut scenario, 0, blockade);
        put(&mut scenario, 4, visitor);
        let diplomacy = declared(enemies);
        let mut blockade_attacks = 0;
        let mut visitor_attacks = 0;
        for (_, events) in years(&scenario, &diplomacy, SEEDS) {
            blockade_attacks += usize::from(attacks(&events, 1, WORLD) > 0);
            visitor_attacks += usize::from(attacks(&events, 2, WORLD) > 0);
        }
        (blockade_attacks, visitor_attacks)
    };
    let years = SEEDS.count();

    // bd1, bd4: a neutral blockade ignores fleets leaving or holding.
    assert_eq!(rate(leaver(0), &[]), (0, 0));
    assert_eq!(rate(holder, &[]), (0, 0));

    // bd2: the blockade still ignores a leaver with ROE 6; the leaver, in
    // its own empire's system, may attack the blockade if it sights it
    // during the 8 or 9 ticks it is still in orbit.
    let (blockade, leaver_first) = rate(leaver(6), &[]);
    assert_eq!(blockade, 0);
    let visibility = 90f64.sqrt() * 5.0 * 1.1 / 52.0;
    eprintln!("bd2: leaver attacked in {leaver_first} of {years} years (classic 0 of 10)");
    assert!(leaver_first as f64 / years as f64 <= within(visibility, 9) + 0.03);

    // bd3: a blockade whose empire declared the leaver an enemy sights it at
    // 4.84% a tick while it climbs out, then needs an intercept.
    let (bd3, _) = rate(leaver(0), &[(1, 2)]);
    eprintln!("bd3: {bd3} of {years} years (classic 1 of 10)");
    assert!((0.15..=0.45).contains(&(bd3 as f64 / years as f64)));

    // bd5: a holding enemy is sighted at 3.73% a tick and cannot run.
    let (bd5, _) = rate(holder, &[(1, 2)]);
    let expected = within(60f64.sqrt() * 5.0 / 52.0 * 5.0, 52);
    eprintln!("bd5: {bd5} of {years} years, formula {expected:.3} (classic 8 of 10)");
    assert_rate("bd5", bd5, years, expected, 4.0);
}

/// A world owner's fleets attack a neutral fleet in the system, whatever its
/// order, at their own sighting rate (`Even though the alien fleet belongs
/// to a neutral empire...`).
#[test]
fn owner_attacks_a_neutral_fleet_in_its_system() {
    let visitor = Fleet::on_station(1, WORLD, true, Order::HoldPosition, ca(1));
    let visibility = 30f64.sqrt() * 5.0 / 52.0;
    for (order, factor) in [
        (Order::HoldPosition, 1.0),
        (Order::GuardBlockadeWorld, 5.0),
        (Order::PatrolSector, 10.0),
    ] {
        let mut scenario = board_with_world();
        put(&mut scenario, 0, visitor);
        put(
            &mut scenario,
            4,
            Fleet::on_station(2, WORLD, true, order, ca(1)).roe(6),
        );
        let attacked = years(&scenario, &declared(&[]), SEEDS)
            .filter(|(_, events)| attacks(events, 2, WORLD) > 0)
            .count();
        eprintln!("{order:?} defender: {attacked} of {} years", SEEDS.count());
        assert_rate(
            &format!("{order:?} defender"),
            attacked,
            SEEDS.count(),
            within(visibility * factor, 52),
            4.0,
        );
    }
    // The same fleet in the sector's deep space is not in the system.
    let mut scenario = board_with_world();
    put(
        &mut scenario,
        0,
        Fleet::on_station(1, WORLD, false, Order::HoldPosition, ca(1)),
    );
    put(
        &mut scenario,
        4,
        Fleet::on_station(2, WORLD, false, Order::PatrolSector, ca(1)).roe(10),
    );
    assert!(years(&scenario, &declared(&[]), 0..200).all(|(_, events)| !fought(&events, 2, WORLD)));
}

/// Units meet only in the same depth layer: an enemy in deep space is never
/// even sighted from orbit.
#[test]
fn enemies_in_different_layers_never_meet() {
    let mut scenario = board_with_world();
    put(
        &mut scenario,
        0,
        Fleet::on_station(1, WORLD, false, Order::PatrolSector, ca(5)).roe(10),
    );
    put(
        &mut scenario,
        4,
        Fleet::on_station(2, WORLD, true, Order::PatrolSector, ca(5)).roe(10),
    );
    for (_, events) in years(&scenario, &declared(&[(1, 2), (2, 1)]), 0..200) {
        assert!(events.scout_contact_events.is_empty());
        assert!(events.fleet_battle_events.is_empty());
    }
}

/// Starbases scan at factor 25 in orbit and 30 in deep space (classic
/// 13.14% and 3.21% a tick), report, and never start a fight.
#[test]
fn starbases_sight_at_classic_odds_and_never_attack() {
    let sector = [9, 10];
    for (in_system, factor, classic) in [(true, 25.0, 13.14), (false, 30.0, 3.21)] {
        let mut scenario = board();
        add_starbase(&mut scenario, 1, 1, sector, in_system);
        put(
            &mut scenario,
            4,
            Fleet::on_station(2, sector, in_system, Order::HoldPosition, ca(1)),
        );
        let visibility = 30f64.sqrt() * if in_system { 5.0 } else { 1.0 } / 52.0;
        let percent = visibility * factor;
        let mut contacts = 0;
        let mut exposure = 0u32;
        for (_, events) in years(&scenario, &declared(&[(1, 2)]), SEEDS) {
            assert!(events.fleet_battle_events.is_empty());
            match first_contact(&events, 1, sector) {
                Some(tick) => {
                    contacts += 1;
                    exposure += u32::from(tick);
                }
                None => exposure += 52,
            }
        }
        eprintln!(
            "starbase in system {in_system}: formula {percent:.2}% classic {classic:.2}% rust {:.2}%",
            100.0 * contacts as f64 / f64::from(exposure)
        );
        assert_rate(
            "starbase sighting",
            contacts,
            SEEDS.count(),
            within(percent, 52),
            4.0,
        );
    }
}

/// Awareness belongs to the empire: two patrols never both report first
/// contact with the same fleet. It resets when that fleet changes sector,
/// so a fleet crossing three patrolled sectors is contacted in each.
#[test]
fn awareness_is_shared_by_the_empire_and_reset_by_a_sector_change() {
    let here = [9, 10];
    let mut scenario = board();
    put(
        &mut scenario,
        0,
        Fleet::on_station(1, here, false, Order::PatrolSector, ca(1)),
    );
    put(
        &mut scenario,
        1,
        Fleet::on_station(1, here, false, Order::PatrolSector, ca(1)),
    );
    put(
        &mut scenario,
        4,
        Fleet::on_station(2, here, false, Order::HoldPosition, ca(3)),
    );
    for (_, events) in years(&scenario, &declared(&[]), 0..300) {
        let reports = events
            .scout_contact_events
            .iter()
            .filter(|event| event.viewer_empire_raw == 1)
            .count();
        assert!(reports <= 1, "{reports} first contacts");
    }

    let mut scenario = board();
    let row = 10;
    put(
        &mut scenario,
        0,
        Fleet::on_station(1, [3, row], false, Order::HoldPosition, ca(20))
            .max_speed(5)
            .heading(Order::MoveOnly, [12, row], false, 3),
    );
    for (idx, x) in [(4, 3), (5, 4), (6, 5)] {
        put(
            &mut scenario,
            idx,
            Fleet::on_station(2, [x, row], false, Order::PatrolSector, ca(1)),
        );
    }
    let mut multi = 0;
    for (_, events) in years(&scenario, &declared(&[]), 0..300) {
        let sectors = [3u8, 4, 5]
            .iter()
            .filter(|&&x| first_contact(&events, 2, [x, row]).is_some())
            .count();
        multi += usize::from(sectors >= 2);
    }
    assert!(
        multi > 100,
        "re-contacted in later sectors in {multi} of 300 years"
    );
}

/// An attacker loses the tick's movement: a speed-1 mover that attacks
/// ends the year `attacks / 52` short (classic `mvlost.py`).
#[test]
fn an_attacker_loses_the_ticks_movement() {
    let row = 10;
    let mover = Fleet::on_station(1, [3, row], false, Order::HoldPosition, ca(5))
        .roe(6)
        .max_speed(5)
        .heading(Order::MoveOnly, [12, row], false, 1);
    let x_of = |game_data: &nc_data::CoreGameData| {
        let fleet = &game_data.fleets.records[0];
        f64::from(fleet.current_location_coords_raw()[0]) + fleet.x_fraction().to_f64()
    };
    let mut alone = board();
    put(&mut alone, 0, mover);
    let (reference, _) = years(&alone, &declared(&[(1, 2)]), 0..1).next().unwrap();
    let reference = x_of(&reference);

    let mut scenario = alone.clone();
    put(
        &mut scenario,
        4,
        Fleet::on_station(
            2,
            [3, row],
            false,
            Order::HoldPosition,
            Ships { et: 50, ..NO_SHIPS },
        ),
    );
    let mut attacked_years = 0;
    let mut total = 0;
    for (game_data, events) in years(&scenario, &declared(&[(1, 2)]), 0..100) {
        let attacked = attacks(&events, 1, [3, row]);
        attacked_years += usize::from(attacked > 0);
        total += attacked;
        let shortfall = (reference - x_of(&game_data)) * 52.0;
        assert!(
            (shortfall - attacked as f64).abs() < 1e-6,
            "{attacked} attacks, shortfall {shortfall}"
        );
    }
    eprintln!("{attacked_years} of 100 years with attacks, {total} attacks");
    // The mover spends 26 ticks in the sector and sights the ETACs at 0.91%
    // a tick: 21% of years. Rust's battle drives the ETACs off, so unlike
    // classic's there are no repeat attacks (battle internals are t036).
    assert!(attacked_years > 8);
}

/// A starbase and its guard fleet fight as one side: attacking either
/// brings in the other, and nobody else joins.
#[test]
fn a_starbase_and_its_guard_fight_together() {
    let sector = [9, 10];
    let mut scenario = board();
    add_starbase(&mut scenario, 2, 1, sector, true);
    put(
        &mut scenario,
        4,
        Fleet::on_station(2, sector, true, Order::GuardStarbase, ca(2)).guarding(1),
    );
    // A bystander of empire 3 in the same orbit never joins.
    put(
        &mut scenario,
        8,
        Fleet::on_station(3, sector, true, Order::HoldPosition, ca(4)),
    );
    put(
        &mut scenario,
        0,
        Fleet::on_station(
            1,
            sector,
            true,
            Order::PatrolSector,
            Ships { bb: 5, ..NO_SHIPS },
        )
        .roe(10),
    );
    let mut battles = 0;
    for (_, events) in years(&scenario, &declared(&[(1, 2)]), 0..200) {
        for event in &events.fleet_battle_events {
            assert_ne!(event.reporting_empire_raw, 3);
            if event.reporting_empire_raw == 2 {
                battles += 1;
                assert_eq!(event.friendly_initial_starbases, 1);
                assert_eq!(event.friendly_initial.cruisers, 2);
                assert_eq!(event.enemy_empires_raw, vec![1]);
            }
        }
    }
    assert!(battles > 150, "{battles} battles in 200 years");
}
