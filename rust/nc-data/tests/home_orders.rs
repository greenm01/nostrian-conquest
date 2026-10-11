//! Seek Home, Salvage, and the autopilot recall against classic ECMAINT.
//!
//! Each case names the sector, depth, speed, and ownership it was probed
//! with on the move fixture ([`common::move_scenario`]) and the numbers
//! classic gave (2026-10-10, t064).

mod common;

use common::move_scenario::*;
use nc_data::{
    Coords, CoreGameData, MaintenanceEvents, Mission, MissionOutcome, MissionRetargetEvent, Order,
    Real48, SalvageResolvedEvent, Starbase, set_starbase_at_rest, stored_production_points,
};

fn retargets(events: &MaintenanceEvents, mission: Mission) -> Vec<&MissionRetargetEvent> {
    events
        .mission_retarget_events
        .iter()
        .filter(|event| {
            matches!(
                event,
                MissionRetargetEvent::Retargeted { mission: m, .. }
                    | MissionRetargetEvent::Abandoned { mission: m, .. } if *m == mission
            )
        })
        .collect()
}

fn arrived_at(events: &MaintenanceEvents, mission: Mission, sector: [u8; 2]) -> bool {
    events.mission_events.iter().any(|event| {
        event.fleet_idx == 0
            && event.kind == mission
            && event.outcome == MissionOutcome::Succeeded
            && event.location_coords == Some(sector)
    })
}

// ── Seek Home ───────────────────────────────────────────────────────────

#[test]
fn seek_home_keeps_an_owned_target_although_another_world_is_nearer() {
    let mut game_data = scenario();
    give_world(&mut game_data, NEAR_WORLD, 1);
    place_fleet(
        &mut game_data,
        EMPTY_SECTOR,
        Real48::ZERO,
        Order::SeekHome,
        HOME,
        1,
    );

    let events = run(&mut game_data);

    assert_eq!(game_data.fleets[0].order, Order::SeekHome);
    assert_eq!(game_data.fleets[0].target.to_array(), HOME);
    assert!(retargets(&events, Mission::SeekHome).is_empty());
}

#[test]
fn seek_home_turns_for_the_nearest_world_in_the_tick_its_world_is_lost() {
    let mut game_data = scenario();
    give_world(&mut game_data, NEAR_WORLD, 1);
    give_world(&mut game_data, FAR_WORLD, 2);
    name_world(&mut game_data, NEAR_WORLD, "Haven");
    name_world(&mut game_data, FAR_WORLD, "Lost");
    place_fleet(
        &mut game_data,
        [13, 14],
        Real48::ZERO,
        Order::SeekHome,
        FAR_WORLD,
        3,
    );

    let events = run(&mut game_data);

    // It flies one tick toward the lost world, turns at stardate 1, and
    // ends the year over (12,12) by the tick-52 rule: 0.670 deep, where a
    // fleet that never flew toward (18,15) would end at 0.764.
    let [retarget] = retargets(&events, Mission::SeekHome)[..] else {
        panic!(
            "one retarget expected: {:?}",
            events.mission_retarget_events
        );
    };
    assert!(matches!(
        retarget,
        MissionRetargetEvent::Retargeted {
            previous_target_coords,
            new_target_coords,
            stardate_week: Some(1),
            ..
        } if *previous_target_coords == FAR_WORLD && *new_target_coords == NEAR_WORLD
    ));
    let fleet = &game_data.fleets[0];
    assert_eq!(fleet.sector.to_array(), NEAR_WORLD);
    assert_eq!(fleet.order, Order::HoldPosition);
    assert!(
        (depth(&game_data) - 0.67036).abs() < 1e-5,
        "{}",
        depth(&game_data)
    );
    let text = report_text(&game_data, &events);
    assert!(
        text.contains("Seek-Home mission report: Since we have lost control of planet \"Lost\" in System(18,15), we are changing course and heading for a different friendly world. Our new destination is planet \"Haven\" in System(12,12)."),
        "{text}"
    );
    // A fleet in deep space is headed by its sector, one in orbit by its
    // system.
    assert!(
        text.contains("From your 1st Fleet, located in Sector(13,14):"),
        "{text}"
    );
    assert!(text.contains("Stardate: 01/"), "{text}");
}

#[test]
fn seek_home_to_an_empty_sector_turns_for_the_nearest_world() {
    let mut game_data = scenario();
    give_world(&mut game_data, NEAR_WORLD, 1);
    place_fleet(
        &mut game_data,
        [13, 14],
        Real48::ZERO,
        Order::SeekHome,
        EMPTY_SECTOR,
        3,
    );

    let events = run(&mut game_data);

    assert!(matches!(
        retargets(&events, Mission::SeekHome)[..],
        [MissionRetargetEvent::Retargeted {
            new_target_coords: NEAR_WORLD,
            stardate_week: Some(1),
            ..
        }]
    ));
    assert_eq!(game_data.fleets[0].sector.to_array(), NEAR_WORLD);
    // Classic: 0.75769, after a first tick toward the empty sector.
    assert!(
        (depth(&game_data) - 0.75769).abs() < 1e-5,
        "{}",
        depth(&game_data)
    );
    // The sector has no world, so classic names a blank planet.
    let text = report_text(&game_data, &events);
    assert!(
        text.contains(
            "Since we have lost control of planet \"\" in System(13,13), we are changing course"
        ),
        "{text}"
    );
}

#[test]
fn seek_home_with_no_world_left_holds_at_its_depth() {
    let mut game_data = scenario();
    give_world(&mut game_data, HOME, 2);
    name_world(&mut game_data, HOME, "Fallen");
    place_fleet(
        &mut game_data,
        EMPTY_SECTOR,
        Real48::from_f64(0.75),
        Order::SeekHome,
        HOME,
        3,
    );

    let events = run(&mut game_data);

    // One tick of the climb (3/52), then the hold: depth stays, the target
    // layer is the nearer one.
    let fleet = &game_data.fleets[0];
    assert_eq!(fleet.order, Order::HoldPosition);
    assert_eq!(fleet.speed, 0);
    assert_eq!(fleet.target.to_array(), EMPTY_SECTOR);
    assert_eq!(fleet.target_depth, 1);
    assert!(
        (depth(&game_data) - 0.69231).abs() < 1e-5,
        "{}",
        depth(&game_data)
    );
    assert!(matches!(
        retargets(&events, Mission::SeekHome)[..],
        [MissionRetargetEvent::Abandoned {
            previous_target_coords: HOME,
            stardate_week: Some(1),
            ..
        }]
    ));
    let text = report_text(&game_data, &events);
    assert!(
        text.contains("Seek-Home mission report: Since we have lost control of planet \"Fallen\" in System(16,13), we are holding our current position and awaiting new orders."),
        "{text}"
    );
    assert!(
        text.contains("From your 1st Fleet, located in System(13,13):"),
        "{text}"
    );
}

// ── Salvage ─────────────────────────────────────────────────────────────

#[test]
fn salvage_whose_world_is_lost_salvages_at_the_nearest_friendly_world() {
    let mut game_data = scenario();
    give_world(&mut game_data, HOME, 2);
    give_world(&mut game_data, NEAR_WORLD, 1);
    name_world(&mut game_data, NEAR_WORLD, "Haven");
    set_ships(&mut game_data.fleets[0], 0, 1, 0, 0, 1);
    place_fleet(
        &mut game_data,
        EMPTY_SECTOR,
        Real48::ZERO,
        Order::Salvage,
        HOME,
        3,
    );
    let stored_before = stored_production_points(planet_at(&mut game_data, NEAR_WORLD));

    let events = run(&mut game_data);

    assert!(matches!(
        retargets(&events, Mission::Salvage)[..],
        [MissionRetargetEvent::Retargeted {
            previous_target_coords: HOME,
            new_target_coords: NEAR_WORLD,
            stardate_week: Some(1),
            ..
        }]
    ));
    // A cruiser and an ETAC yield 8 and 10, in tick 44.
    assert!(matches!(
        events.salvage_events[..],
        [SalvageResolvedEvent::Succeeded {
            coords: NEAR_WORLD,
            recovered_points: 18,
            stardate_week: Some(44),
            ..
        }]
    ));
    assert!(
        game_data
            .fleets
            .iter()
            .all(|fleet| fleet.fleet_number.0 != 1 || fleet.owner.0 != 1)
    );
    assert_eq!(
        stored_production_points(planet_at(&mut game_data, NEAR_WORLD)),
        stored_before + 18
    );
    // The report says the fleet will salvage at the new world, not that it
    // seeks safety there (owner decision, t064).
    let text = report_text(&game_data, &events);
    assert!(
        text.contains("Salvage mission report: Since we no longer own the world in System(16,13), we are changing course. Until ordered otherwise, we are going to salvage our fleet at a friendly controlled world. Our new destination is planet \"Haven\" in System(12,12)."),
        "{text}"
    );
    assert!(!text.contains("seek to safety"), "{text}");
    assert!(
        text.contains("We have arrived at planet \"Haven\" in System(12,12) and have begun salvaging our fleet. We estimate that our fleet will yield 18 production point(s)."),
        "{text}"
    );
}

#[test]
fn salvage_refills_a_zero_speed_when_it_turns() {
    // A fleet at rest in orbit of its lost world has nothing to arrive at
    // that the dispatcher would refill for; the retarget does.
    let mut game_data = scenario();
    give_world(&mut game_data, HOME, 2);
    give_world(&mut game_data, NEAR_WORLD, 1);
    place_fleet(&mut game_data, HOME, Real48::ONE, Order::Salvage, HOME, 0);

    run(&mut game_data);

    let fleet = &game_data.fleets[0];
    assert_eq!(fleet.order, Order::Salvage);
    assert_eq!(fleet.target.to_array(), NEAR_WORLD);
    assert_eq!(fleet.speed, fleet.max_speed);
}

#[test]
fn salvage_with_no_world_left_holds_at_its_depth() {
    // Classic snaps a salvage fleet's depth to the nearer layer (0.692 to
    // 1.0, 0.135 to 0.0); NC leaves it, as Seek Home does.
    for (start_depth, speed, held_depth, target_depth) in
        [(0.75, 3, 0.69231, 1), (0.25, 6, 0.13462, 0)]
    {
        let mut game_data = scenario();
        give_world(&mut game_data, HOME, 2);
        set_ships(&mut game_data.fleets[0], 0, 0, 1, 0, 0);
        place_fleet(
            &mut game_data,
            EMPTY_SECTOR,
            Real48::from_f64(start_depth),
            Order::Salvage,
            HOME,
            speed,
        );

        let events = run(&mut game_data);

        let fleet = &game_data.fleets[0];
        assert_eq!(fleet.order, Order::HoldPosition);
        assert_eq!(fleet.speed, 0);
        assert_eq!(fleet.target.to_array(), EMPTY_SECTOR);
        assert_eq!(fleet.target_depth, target_depth);
        assert!(
            (depth(&game_data) - held_depth).abs() < 1e-5,
            "{}",
            depth(&game_data)
        );
        assert!(matches!(
            retargets(&events, Mission::Salvage)[..],
            [MissionRetargetEvent::Abandoned {
                stardate_week: Some(1),
                ..
            }]
        ));
        assert!(events.salvage_events.is_empty());
        let text = report_text(&game_data, &events);
        assert!(
            text.contains("Salvage mission report: Since we no longer own the world in System(16,13), we are aborting our salvage mission. Until ordered otherwise, we are going to hold our current position and await new orders."),
            "{text}"
        );
    }
}

// ── Autopilot recall ────────────────────────────────────────────────────

/// The autopilot probes: empire 1 owns (16,13) at production 5, (12,11) and
/// (12,12) at 2, and (2,16) at 0.
fn autopilot_scenario() -> CoreGameData {
    let mut game_data = scenario();
    game_data.players[0].autopilot = true;
    for (sector, production) in [
        (HOME, 5),
        (OTHER_NEAR_WORLD, 2),
        (NEAR_WORLD, 2),
        (REMOTE_WORLD, 0),
    ] {
        give_world(&mut game_data, sector, 1);
        set_production(&mut game_data, sector, production);
    }
    game_data
}

#[test]
fn autopilot_recall_weighs_distance_by_production() {
    // (12,12) is 2.0 away at production 2, (16,13) 4.16 away at production 5.
    let mut game_data = autopilot_scenario();
    place_fleet(
        &mut game_data,
        [13, 12],
        Real48::ZERO,
        Order::HoldPosition,
        [13, 12],
        0,
    );

    let events = run(&mut game_data);

    assert!(arrived_at(&events, Mission::SeekHome, HOME));
}

#[test]
fn autopilot_recall_breaks_a_tie_for_the_earlier_world() {
    // 1.5 from both (12,11) and (12,12) at equal production.
    let mut game_data = autopilot_scenario();
    place_fleet(
        &mut game_data,
        NEAR_WORLD,
        Real48::ZERO,
        Order::HoldPosition,
        NEAR_WORLD,
        0,
    );
    game_data.fleets[0].y_fraction = Real48::ZERO;

    let events = run(&mut game_data);

    assert!(arrived_at(&events, Mission::SeekHome, OTHER_NEAR_WORLD));
}

#[test]
fn autopilot_recall_takes_a_world_without_production() {
    // 1.0 from a world of production 0, which counts as 1.
    let mut game_data = autopilot_scenario();
    place_fleet(
        &mut game_data,
        REMOTE_WORLD,
        Real48::ZERO,
        Order::HoldPosition,
        REMOTE_WORLD,
        0,
    );

    let events = run(&mut game_data);

    assert!(arrived_at(&events, Mission::SeekHome, REMOTE_WORLD));
}

#[test]
fn autopilot_warships_in_orbit_guard_the_world_with_roe_6() {
    let mut game_data = autopilot_scenario();
    place_fleet(
        &mut game_data,
        NEAR_WORLD,
        Real48::ONE,
        Order::HoldPosition,
        NEAR_WORLD,
        0,
    );
    game_data.fleets[0].rules_of_engagement = 0;

    run(&mut game_data);

    let fleet = &game_data.fleets[0];
    assert_eq!(fleet.order, Order::GuardBlockadeWorld);
    assert_eq!(fleet.target.to_array(), NEAR_WORLD);
    assert_eq!(fleet.speed, 0);
    assert_eq!(fleet.rules_of_engagement, 6);
}

#[test]
fn autopilot_warships_guard_an_unescorted_starbase_in_orbit() {
    let mut game_data = autopilot_scenario();
    let mut base = Starbase::default();
    base.base_id.0 = 1;
    base.owner.0 = 1;
    base.record.0 = 1;
    base.sector = Coords::from_array(NEAR_WORLD);
    set_starbase_at_rest(&mut base);
    base.target = base.sector;
    base.target_depth = 1;
    game_data.bases = vec![base];
    game_data.players[0].bases_first.0 = 1;
    place_fleet(
        &mut game_data,
        NEAR_WORLD,
        Real48::ONE,
        Order::HoldPosition,
        NEAR_WORLD,
        0,
    );
    game_data.fleets[0].rules_of_engagement = 0;

    run(&mut game_data);

    let fleet = &game_data.fleets[0];
    assert_eq!(fleet.order, Order::GuardStarbase);
    assert_eq!(fleet.mission_target, 1);
    assert_eq!(game_data.bases[0].escort.0, fleet.fleet_number.0);
}

#[test]
fn autopilot_guard_blockade_fleets_at_rest_also_take_an_unescorted_starbase() {
    // Classic runs the starbase step on every fleet on Guard/Blockade at
    // rest, not only on those the pass re-ordered; this one keeps its ROE 3.
    let mut game_data = autopilot_scenario();
    add_base(&mut game_data, 1, NEAR_WORLD, Real48::ONE, 1, 0);
    place_fleet(
        &mut game_data,
        NEAR_WORLD,
        Real48::ONE,
        Order::GuardBlockadeWorld,
        NEAR_WORLD,
        0,
    );
    game_data.fleets[0].rules_of_engagement = 3;

    run(&mut game_data);

    let fleet = &game_data.fleets[0];
    assert_eq!(fleet.order, Order::GuardStarbase);
    assert_eq!(fleet.mission_target, 1);
    assert_eq!(fleet.rules_of_engagement, 3);
    assert_eq!(game_data.bases[0].escort.0, fleet.fleet_number.0);
}

#[test]
fn autopilot_leaves_unarmed_fleets_in_orbit_and_other_orders_alone() {
    let mut game_data = autopilot_scenario();
    // Fleet 1: an ETAC alone in orbit stays on Hold.
    set_ships(&mut game_data.fleets[0], 0, 0, 0, 0, 1);
    place_fleet(
        &mut game_data,
        NEAR_WORLD,
        Real48::ONE,
        Order::HoldPosition,
        NEAR_WORLD,
        0,
    );
    game_data.fleets[0].rules_of_engagement = 0;
    // Fleets 2 and 3: a Move and a Seek Home order stay as they are.
    for (idx, order, target) in [
        (1, Order::MoveOnly, [8, 13]),
        (2, Order::SeekHome, OTHER_NEAR_WORLD),
    ] {
        let fleet = &mut game_data.fleets[idx];
        fleet.sector = Coords::from_array(EMPTY_SECTOR);
        fleet.depth = Real48::ZERO;
        fleet.order = order;
        fleet.target = Coords::from_array(target);
        fleet.target_depth = u8::from(order == Order::SeekHome);
        fleet.speed = 1;
    }

    run(&mut game_data);

    assert_eq!(game_data.fleets[0].order, Order::HoldPosition);
    assert_eq!(game_data.fleets[1].order, Order::MoveOnly);
    assert_eq!(game_data.fleets[1].target.to_array(), [8, 13]);
    assert_eq!(game_data.fleets[2].order, Order::SeekHome);
    assert_eq!(game_data.fleets[2].target.to_array(), OTHER_NEAR_WORLD);
}

#[test]
fn a_human_guard_starbase_fleet_that_lost_its_warships_holds_and_lets_go_of_its_base() {
    // The pre-maintenance check takes the fleet off its order (it has no
    // other world to seek, so it holds), and the base lets go of it.
    let mut game_data = scenario();
    set_ships(&mut game_data.fleets[0], 0, 0, 0, 1, 0);
    place_fleet(
        &mut game_data,
        HOME,
        Real48::ONE,
        Order::GuardStarbase,
        HOME,
        0,
    );
    game_data.fleets[0].mission_target = 1;
    let escort = game_data.fleets[0].fleet_number.0;
    add_base(&mut game_data, 1, HOME, Real48::ONE, 1, escort);

    run(&mut game_data);

    assert_eq!(game_data.fleets[0].order, Order::HoldPosition);
    assert_eq!(game_data.bases[0].escort.0, 0);
}
