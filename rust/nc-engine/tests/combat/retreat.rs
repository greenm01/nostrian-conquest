//! Where a fleet goes when it leaves a battle, and how fast.

use nc_data::{
    CoreGameData, EncounterDispositionEvent, MaintenanceEvents, MissionAbortReason, MissionOutcome,
    Order, Real48,
};
use nc_engine::run_maintenance_turn_with_context_and_seed;

use super::support::{
    DEEP_SPACE, SYSTEM, Ships, diplomacy, empty_board, mark_arrived, place_fleet,
};

/// Empire 1's only world in the fixture, at (16,13).
const HOME_PLANET_IDX: usize = 14;

fn run(game_data: &mut CoreGameData, seed: u64) -> MaintenanceEvents {
    run_maintenance_turn_with_context_and_seed(game_data, seed, &[], &diplomacy(&[(1, 2)]))
        .expect("maintenance should succeed")
}

/// Put fleet record `idx` at the centre of its sector in deep space, its
/// order targeting that layer, and rate it by its ships.
fn at_sector_centre_in_deep_space(game_data: &mut CoreGameData, idx: usize) {
    let fleet = &mut game_data.fleets.records[idx];
    fleet.set_x_fraction(Real48::HALF);
    fleet.set_y_fraction(Real48::HALF);
    fleet.set_depth(Real48::ZERO);
    fleet.set_mission_target(0, 0);
    fleet.recompute_max_speed_from_composition();
}

/// Fleet 0 withdrew from the battle at `coords` this year.
fn withdrew(events: &MaintenanceEvents, coords: [u8; 2]) -> bool {
    events.encounter_disposition_events.iter().any(|event| {
        matches!(
            event,
            EncounterDispositionEvent::Retreated { fleet_idx: 0, coords: at, .. } if *at == coords
        )
    })
}

/// Empire 1's fleet record 0 with `ships` bombards `SYSTEM`, held by
/// `batteries`, and takes their fire. Returns the fleet after the year.
fn bombards_batteries(ships: Ships, batteries: u8, seed: u64) -> nc_data::FleetRecord {
    let mut game_data = empty_board(0, batteries);
    place_fleet(
        &mut game_data,
        0,
        1,
        SYSTEM,
        Order::BombardWorld,
        SYSTEM,
        ships,
        10,
    );
    mark_arrived(&mut game_data, 0);
    game_data.fleets.records[0].recompute_max_speed_from_composition();
    run(&mut game_data, seed);
    game_data.fleets.records[0].clone()
}

#[test]
fn survivors_move_at_the_speed_of_the_ships_they_have_left() {
    // Battery fire falls on the combat line first, so ten batteries sink the
    // battleship in round 1; the bombardment then ends with the scouts
    // alive, and they move at their own speed.
    for seed in 0..4u64 {
        let fleet = bombards_batteries(
            Ships {
                bb: 1,
                sc: 150,
                ..Ships::default()
            },
            10,
            seed,
        );
        assert_eq!(fleet.battleship_count(), 0, "seed {seed}");
        assert!(fleet.scout_count() > 0, "seed {seed}");
        assert_eq!(fleet.max_speed(), 6, "seed {seed}");
        assert!(fleet.current_speed() <= 6, "seed {seed}");
    }
}

#[test]
fn losing_the_faster_hull_keeps_the_slower_hull_speed() {
    // A patrol of empire 1 on ROE 0 meets two destroyers of empire 2 on
    // ROE 10 and breaks off in round 4; its destroyer dies first.
    let mut lost_the_destroyer = 0;
    for seed in 0..16u64 {
        let mut game_data = empty_board(0, 0);
        place_fleet(
            &mut game_data,
            0,
            1,
            DEEP_SPACE,
            Order::PatrolSector,
            DEEP_SPACE,
            Ships {
                bb: 1,
                dd: 1,
                ..Ships::default()
            },
            0,
        );
        game_data.fleets.records[0].recompute_max_speed_from_composition();
        place_fleet(
            &mut game_data,
            4,
            2,
            DEEP_SPACE,
            Order::MoveOnly,
            [9, 8],
            Ships {
                dd: 2,
                ..Ships::default()
            },
            10,
        );
        run(&mut game_data, seed);
        let fleet = &game_data.fleets.records[0];
        if fleet.battleship_count() == 0 {
            continue;
        }
        assert_eq!(fleet.max_speed(), 4, "seed {seed}");
        assert!(fleet.current_speed() <= 4, "seed {seed}");
        if fleet.destroyer_count() == 0 {
            lost_the_destroyer += 1;
        }
    }
    assert!(lost_the_destroyer > 0);
}

/// A toothless guard: one destroyer, which dies in round 1, and scouts
/// enough to outlast three rounds of fire, so a force on ROE 0 reaches its
/// round-4 withdrawal with its ships.
fn toothless() -> Ships {
    Ships {
        dd: 1,
        sc: 300,
        ..Ships::default()
    }
}

/// Six battleships of empire 1 bombarding `SYSTEM` on ROE 0 meet its
/// toothless guard and withdraw in round 4.
fn bombardment_withdraws(game_data: &mut CoreGameData, seed: u64) -> MaintenanceEvents {
    place_fleet(
        game_data,
        0,
        1,
        SYSTEM,
        Order::BombardWorld,
        SYSTEM,
        Ships {
            bb: 6,
            ..Ships::default()
        },
        0,
    );
    mark_arrived(game_data, 0);
    game_data.fleets.records[0].recompute_max_speed_from_composition();
    place_fleet(
        game_data,
        4,
        2,
        SYSTEM,
        Order::GuardBlockadeWorld,
        SYSTEM,
        toothless(),
        10,
    );
    mark_arrived(game_data, 4);
    run(game_data, seed)
}

/// Empire 1's fleet record 0, a destroyer escorting a loaded transport to
/// invade `SYSTEM` on ROE 10, meets the world's guard, a lone destroyer on
/// ROE 10. When the destroyers sink each other the invasion force holds the
/// field without the warship its mission needs and aborts.
fn invasion_loses_its_escort(game_data: &mut CoreGameData, seed: u64) -> MaintenanceEvents {
    place_fleet(
        game_data,
        0,
        1,
        SYSTEM,
        Order::InvadeWorld,
        SYSTEM,
        Ships {
            dd: 1,
            tt: 1,
            armies: 1,
            ..Ships::default()
        },
        10,
    );
    // The invader starts its dive from the sector's deep space, so the guard
    // in orbit has the descent ticks to catch it before it strikes (strikes
    // resolve in their tick since t052).
    {
        let fleet = &mut game_data.fleets.records[0];
        fleet.set_mission_target(1, fleet.mission_id_raw());
        fleet.set_at_rest_motion();
        fleet.set_depth(Real48::ZERO);
        fleet.recompute_max_speed_from_composition();
        fleet.set_current_speed(fleet.max_speed());
    }
    place_fleet(
        game_data,
        4,
        2,
        SYSTEM,
        Order::GuardBlockadeWorld,
        SYSTEM,
        Ships {
            dd: 1,
            ..Ships::default()
        },
        10,
    );
    mark_arrived(game_data, 4);
    run(game_data, seed)
}

#[test]
fn retreat_from_an_enemy_orbit_seeks_the_home_world_and_climbs_out_first() {
    const HOME: [u8; 2] = [3, 13];
    let mut retreated = 0;
    for seed in 0..40u64 {
        let mut game_data = empty_board(0, 0);
        game_data.planets.records[HOME_PLANET_IDX].set_coords_raw(HOME);
        let events = invasion_loses_its_escort(&mut game_data, seed);
        let fleet = &game_data.fleets.records[0];
        if fleet.destroyer_count() > 0 || fleet.troop_transport_count() == 0 {
            continue;
        }
        // Strikes resolve in their tick (t052). When the year's unit order
        // lets the invader strike before the guard sights it, the invasion
        // resolves first and there is no mission left to abort; only the
        // seeds where the guard caught the invader first test the retreat.
        if !events.assault_report_events.is_empty() {
            continue;
        }
        retreated += 1;
        assert_eq!(fleet.standing_order_kind(), Order::SeekHome, "seed {seed}");
        assert_eq!(fleet.standing_order_target_coords_raw(), HOME);
        assert_eq!(
            fleet.world_target_flag_raw(),
            1,
            "Seek Home targets the world"
        );
        assert_eq!(fleet.rules_of_engagement(), 10, "ROE is left alone");
        assert_eq!(
            fleet.max_speed(),
            5,
            "the transport's speed, not the destroyer's"
        );
        assert_eq!(fleet.current_speed(), 5);
        // Twelve sectors from home, the fleet is still under way at year
        // end: out of the enemy's orbit and flying in deep space.
        assert_ne!(fleet.current_location_coords_raw(), SYSTEM, "seed {seed}");
        assert_eq!(fleet.depth(), Real48::ZERO, "seed {seed}");
        assert!(
            events.mission_events.iter().any(|event| {
                event.fleet_idx == 0
                    && event.outcome == MissionOutcome::Aborted
                    && event.abort_reason == Some(MissionAbortReason::CapabilityLost)
                    && event.target_coords == Some(HOME)
            }),
            "the abort names the home world: {:?}",
            events.mission_events
        );
    }
    assert!(retreated > 0);
}

#[test]
fn retreat_with_no_owned_world_holds_in_place() {
    for seed in 0..4u64 {
        let mut game_data = empty_board(0, 0);
        game_data.planets.records[HOME_PLANET_IDX].set_owner_empire_slot_raw(3);
        let events = bombardment_withdraws(&mut game_data, seed);
        assert!(withdrew(&events, SYSTEM), "seed {seed}");
        let fleet = &game_data.fleets.records[0];
        assert_eq!(
            fleet.standing_order_kind(),
            Order::HoldPosition,
            "seed {seed}"
        );
        assert_eq!(fleet.current_speed(), 0);
        assert_eq!(fleet.battleship_count(), 6);
        assert_eq!(fleet.current_location_coords_raw(), SYSTEM);
        assert_eq!(fleet.depth(), Real48::ONE, "it stays in orbit, seed {seed}");
    }
}

#[test]
fn retreat_picks_home_by_travel_distance_not_manhattan_distance() {
    // From (8,8), (14,8) is six sectors away by the Manhattan count and 6.0
    // as the fleet flies; (12,12) is eight by Manhattan and 5.66 as it flies.
    // (14,8) also comes first in PLANETS.DAT, so a tie would pick it too.
    const BY_MANHATTAN: [u8; 2] = [14, 8];
    const BY_TRAVEL: [u8; 2] = [12, 12];
    let mut retreated = 0;
    for seed in 0..8u64 {
        let mut game_data = empty_board(0, 0);
        game_data.planets.records[HOME_PLANET_IDX].set_owner_empire_slot_raw(3);
        game_data.planets.records[7].set_coords_raw(BY_MANHATTAN);
        game_data.planets.records[7].set_owner_empire_slot_raw(1);
        assert_eq!(game_data.planets.records[11].coords_raw(), BY_TRAVEL);
        game_data.planets.records[11].set_owner_empire_slot_raw(1);
        place_fleet(
            &mut game_data,
            0,
            1,
            DEEP_SPACE,
            Order::PatrolSector,
            DEEP_SPACE,
            Ships {
                bb: 6,
                ..Ships::default()
            },
            0,
        );
        at_sector_centre_in_deep_space(&mut game_data, 0);
        place_fleet(
            &mut game_data,
            4,
            2,
            DEEP_SPACE,
            Order::MoveOnly,
            [9, 8],
            toothless(),
            10,
        );
        at_sector_centre_in_deep_space(&mut game_data, 4);
        game_data.fleets.records[4].set_max_speed(0);
        let events = run(&mut game_data, seed);
        if !withdrew(&events, DEEP_SPACE) {
            // The passing fleet did not attack this year.
            continue;
        }
        retreated += 1;
        let fleet = &game_data.fleets.records[0];
        assert_eq!(fleet.standing_order_kind(), Order::SeekHome, "seed {seed}");
        assert_eq!(fleet.standing_order_target_coords_raw(), BY_TRAVEL);
        assert_eq!(fleet.world_target_flag_raw(), 1);
        assert_eq!(fleet.current_speed(), 4);
        assert!(
            events
                .encounter_disposition_events
                .iter()
                .any(|event| matches!(
                    event,
                    EncounterDispositionEvent::Retreated {
                        fleet_idx: 0,
                        retreat_target_coords: BY_TRAVEL,
                        ..
                    }
                )),
            "the retreat report names the world the fleet flies to"
        );
    }
    assert!(retreated > 0);
}
