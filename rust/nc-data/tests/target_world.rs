//! World orders against their target world's owner, as classic ECMAINT
//! checks it in the year's ticks (stardate audit, 2026-10-31, t220).

mod common;

use common::move_scenario::*;
use nc_data::{
    ColonizationResolvedEvent, MaintenanceEvents, Mission, MissionRetargetEvent, Order, Real48,
};

/// The week and follow-on order of fleet 1's target-world event.
fn owned_event(events: &MaintenanceEvents, mission: Mission) -> (Option<u8>, Order) {
    events
        .mission_retarget_events
        .iter()
        .find_map(|event| match *event {
            MissionRetargetEvent::TargetWorldOwned {
                fleet_idx: 0,
                mission: m,
                follow_on,
                stardate_week,
                ..
            } if m == mission => Some((stardate_week, follow_on)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no {mission:?} event: {:?}", events.mission_retarget_events))
}

#[test]
fn an_assault_on_our_own_world_guards_it_instead() {
    let mut game_data = scenario();
    give_world(&mut game_data, NEAR_WORLD, 1);
    place_fleet(
        &mut game_data,
        EMPTY_SECTOR,
        Real48::ZERO,
        Order::BombardWorld,
        NEAR_WORLD,
        3,
    );
    game_data.fleets[0].rules_of_engagement = 6;

    let events = run(&mut game_data);

    assert_eq!(
        owned_event(&events, Mission::BombardWorld),
        (Some(1), Order::GuardBlockadeWorld)
    );
    // It flies on to the world and takes up its guard there.
    assert_eq!(game_data.fleets[0].sector.to_array(), NEAR_WORLD);
    assert_eq!(game_data.fleets[0].order, Order::GuardBlockadeWorld);
    let text = report_text(&game_data, &events);
    assert!(
        text.contains("Bombardment mission report: Since we now own the world in System(12,12), we are aborting our mission to bombard it. Until ordered otherwise, we are taking the initiative to guard this new world."),
        "{text}"
    );
}

#[test]
fn an_assault_without_roe_moves_to_our_own_world() {
    let mut game_data = scenario();
    give_world(&mut game_data, NEAR_WORLD, 1);
    place_fleet(
        &mut game_data,
        EMPTY_SECTOR,
        Real48::ZERO,
        Order::BombardWorld,
        NEAR_WORLD,
        3,
    );
    game_data.fleets[0].rules_of_engagement = 0;

    let events = run(&mut game_data);

    assert_eq!(
        owned_event(&events, Mission::BombardWorld),
        (Some(1), Order::MoveOnly)
    );
    let text = report_text(&game_data, &events);
    assert!(
        text.contains("we are aborting our mission to bombard it. Until ordered otherwise, we are simply going to move to this new world."),
        "{text}"
    );
    // The Move ends in orbit, so it reports from the system.
    assert!(text.contains("located in System(12,12):"), "{text}");
    assert!(!text.contains("located in Sector(12,12):"), "{text}");
}

#[test]
fn viewing_our_own_world_moves_to_its_sector() {
    let mut game_data = scenario();
    give_world(&mut game_data, NEAR_WORLD, 1);
    place_fleet(
        &mut game_data,
        EMPTY_SECTOR,
        Real48::ZERO,
        Order::ViewWorld,
        NEAR_WORLD,
        3,
    );

    let events = run(&mut game_data);

    assert_eq!(
        owned_event(&events, Mission::ViewWorld),
        (Some(1), Order::MoveOnly)
    );
    let fleet = &game_data.fleets[0];
    assert_eq!(fleet.sector.to_array(), NEAR_WORLD);
    assert_eq!(fleet.target_depth, 0);
    assert!(fleet.depth.is_zero(), "it stays in deep space");
    let text = report_text(&game_data, &events);
    assert!(
        text.contains("Viewing mission report: Since we now own the world in System(12,12), we are aborting our viewing mission. Until ordered otherwise, we are simply going to proceed to Sector(12,12) and then await new orders."),
        "{text}"
    );
}

#[test]
fn colonizing_our_own_world_holds_before_moving() {
    let mut game_data = scenario();
    give_world(&mut game_data, NEAR_WORLD, 1);
    place_fleet(
        &mut game_data,
        EMPTY_SECTOR,
        Real48::ONE,
        Order::ColonizeWorld,
        NEAR_WORLD,
        3,
    );
    set_ships(&mut game_data.fleets[0], 0, 1, 0, 0, 1);

    let events = run(&mut game_data);

    assert_eq!(
        owned_event(&events, Mission::ColonizeWorld),
        (Some(1), Order::HoldPosition)
    );
    let fleet = &game_data.fleets[0];
    assert_eq!(fleet.sector.to_array(), EMPTY_SECTOR);
    assert_eq!(
        fleet.depth,
        Real48::ONE,
        "classic holds without its first step"
    );
    assert_eq!(fleet.speed, 0);
    let text = report_text(&game_data, &events);
    assert!(
        text.contains("Colonization mission report: Since we now own the world in System(12,12), we are holding our current position and are awaiting new orders."),
        "{text}"
    );
}

#[test]
fn colonizing_an_occupied_world_turns_back_on_entering_its_system() {
    let mut game_data = scenario();
    give_world(&mut game_data, NEAR_WORLD, 2);
    place_fleet(
        &mut game_data,
        EMPTY_SECTOR,
        Real48::ZERO,
        Order::ColonizeWorld,
        NEAR_WORLD,
        3,
    );
    set_ships(&mut game_data.fleets[0], 0, 1, 0, 0, 1);

    let events = run(&mut game_data);

    let week = events
        .colonization_events
        .iter()
        .find_map(|event| match *event {
            ColonizationResolvedEvent::BlockedByOwner { stardate_week, .. } => stardate_week,
            _ => None,
        })
        .expect("the attempt is blocked");
    assert!(
        week < 52,
        "blocked in the week it enters the system, not at year end: {week}"
    );
    // It climbs back out to the sector and reports that Move's arrival.
    let fleet = &game_data.fleets[0];
    assert_eq!(fleet.sector.to_array(), NEAR_WORLD);
    assert!(fleet.depth.is_zero());
    let text = report_text(&game_data, &events);
    assert!(
        text.contains("aliens are already living on the world found within!"),
        "{text}"
    );
    assert!(text.contains("located in Sector(12,12):"), "{text}");
    assert!(
        text.contains("Move mission report: We have arrived"),
        "{text}"
    );
}

#[test]
fn scouting_our_own_system_holds_on_arrival() {
    let mut game_data = scenario();
    give_world(&mut game_data, NEAR_WORLD, 1);
    place_fleet(
        &mut game_data,
        EMPTY_SECTOR,
        Real48::ZERO,
        Order::ScoutSolarSystem,
        NEAR_WORLD,
        3,
    );
    set_ships(&mut game_data.fleets[0], 0, 0, 0, 1, 0);

    let events = run(&mut game_data);

    let (week, follow_on) = owned_event(&events, Mission::ScoutSolarSystem);
    assert!(
        week.is_some_and(|week| week > 1),
        "on arrival, not at the start: {week:?}"
    );
    assert_eq!(follow_on, Order::HoldPosition);
    assert_eq!(game_data.fleets[0].order, Order::HoldPosition);
    let text = report_text(&game_data, &events);
    assert!(
        text.contains("Scouting mission report: Since we now control System(12,12), we are aborting our scouting mission. Until ordered otherwise, we will be simply holding our position and awaiting new orders."),
        "{text}"
    );
}

#[test]
fn viewing_an_owned_world_names_it() {
    let mut game_data = scenario();
    give_world(&mut game_data, NEAR_WORLD, 2);
    name_world(&mut game_data, NEAR_WORLD, "Bastion");
    place_fleet(
        &mut game_data,
        EMPTY_SECTOR,
        Real48::ZERO,
        Order::ViewWorld,
        NEAR_WORLD,
        3,
    );

    let events = run(&mut game_data);

    let text = report_text(&game_data, &events);
    assert!(
        text.contains("have completed a long range viewing analysis of planet \"Bastion\". The world is owned by"),
        "{text}"
    );
}
