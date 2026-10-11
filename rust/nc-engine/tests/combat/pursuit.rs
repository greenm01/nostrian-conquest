//! Pursuit across ticks and years: an empire that has had its one exchange
//! with an escaping fleet gets no second attack on it while the escape
//! lasts, and the escape outlives the year.

use nc_data::{
    Coords, CoreGameData, DiplomacyOverride, EscapeEpisode, MaintenanceEvents, Order, Real48,
    recompute_max_speed, set_mission_target,
};
use nc_engine::run_maintenance_turn_with_context_and_seed;

use super::support::{DEEP_SPACE, Ships, diplomacy, empty_board, place_fleet};

/// Empire 1's home world in the fixture.
const HOME: [u8; 2] = [16, 13];

fn run(
    game_data: &mut CoreGameData,
    seed: u64,
    overrides: &[DiplomacyOverride],
) -> MaintenanceEvents {
    run_maintenance_turn_with_context_and_seed(game_data, seed, &[], overrides)
        .expect("maintenance should succeed")
}

/// Battles `empire` fought at `DEEP_SPACE`.
fn battles(events: &MaintenanceEvents, empire: u8) -> usize {
    events
        .fleet_battle_events
        .iter()
        .filter(|event| event.reporting_empire == empire && event.coords == DEEP_SPACE)
        .count()
}

/// Empire 1's fleet record 0, a cruiser with twenty scouts on ROE 1,
/// escaping from a battle in `DEEP_SPACE` that empire 2 has already
/// caught it in once. Stalled in transit (maximum speed 0), it cannot leave
/// the sector, so its escape lasts. Destroyers of empires 2 and 3 patrol
/// the sector on ROE 10; both empires are its enemies, and each other's
/// neutrals.
fn escaping_fleet_among_patrols() -> (CoreGameData, Vec<DiplomacyOverride>) {
    let mut game_data = empty_board(0, 0);
    place_fleet(
        &mut game_data,
        0,
        1,
        DEEP_SPACE,
        Order::SeekHome,
        HOME,
        Ships {
            ca: 1,
            sc: 20,
            ..Ships::default()
        },
        1,
    );
    let fleet = &mut game_data.fleets[0];
    set_mission_target(fleet, 1, 0);
    fleet.depth = Real48::ZERO;
    let mut escape = EscapeEpisode {
        sector: Coords::from_array(DEEP_SPACE),
        layer: 0,
        pursuers: 0,
    };
    escape.record_pursuit(2);
    fleet.escape = Some(escape);
    for (idx, owner) in [(4, 2), (8, 3)] {
        place_fleet(
            &mut game_data,
            idx,
            owner,
            DEEP_SPACE,
            Order::PatrolSector,
            DEEP_SPACE,
            Ships {
                dd: 2,
                ..Ships::default()
            },
            10,
        );
        let patrol = &mut game_data.fleets[idx];
        patrol.depth = Real48::ZERO;
        set_mission_target(patrol, 0, 0);
        recompute_max_speed(patrol);
    }
    (game_data, diplomacy(&[(1, 2), (1, 3)]))
}

#[test]
fn an_empire_that_caught_an_escaping_fleet_once_gets_no_second_attack_even_next_year() {
    let (scenario, overrides) = escaping_fleet_among_patrols();
    let mut caught_by_the_third = 0;
    for seed in 0..12u64 {
        let mut game_data = scenario.clone();
        let events = run(&mut game_data, seed, &overrides);
        // Empire 2 sights the fleet (a patrol sees it at 2.5% a week, and
        // at 30% more once it has fought it) but has had its exchange.
        assert_eq!(battles(&events, 2), 0, "seed {seed}");
        // Empire 3's first attack on the escaping fleet is its one
        // pursuit: one exchange, in which the fleet breaks off unpursued.
        let by_third = battles(&events, 3);
        assert!(by_third <= 1, "seed {seed}: {by_third} battles");
        let fleet = &game_data.fleets[0];
        assert_eq!(fleet.sector.to_array(), DEEP_SPACE, "seed {seed}");
        assert_eq!(fleet.order, Order::SeekHome, "seed {seed}");
        let escape = fleet.escape.expect("the escape persists");
        assert_eq!(escape.sector.to_array(), DEEP_SPACE);
        assert_eq!(escape.layer, 0);
        assert!(escape.pursued_by(2));
        assert_eq!(escape.pursued_by(3), by_third == 1, "seed {seed}");
        if by_third == 0 {
            continue;
        }
        caught_by_the_third += 1;

        // Next year the escape is still on, and now neither empire attacks.
        let events = run(&mut game_data, seed + 100, &overrides);
        assert_eq!(battles(&events, 2), 0, "seed {seed}, second year");
        assert_eq!(battles(&events, 3), 0, "seed {seed}, second year");
        let fleet = &game_data.fleets[0];
        assert_eq!(fleet.escape, Some(escape), "seed {seed}, second year");
        assert_eq!(fleet.order, Order::SeekHome);
    }
    assert!(caught_by_the_third > 0);
}
