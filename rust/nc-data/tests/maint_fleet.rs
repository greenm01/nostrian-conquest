//! Regression tests for fleet maintenance mechanics (Milestone 4 Phase 2)
//!
//! Validates that the Rust maintenance implementation matches the original ECMAINT
//! behavior on the fleet-scenario fixture pair.

mod common;

use nc_compat::classic::{serialize_fleets_dat, serialize_planets_dat, serialize_player_dat};
use nc_compat::load_classic_core_files;
use nc_data::{
    ColonizationResolvedEvent, Coords, CoreGameData, DiplomacyOverride, DiplomaticRelation,
    FleetOrderValidationError, GameStateBuilder, JoinMissionHostEvent, Mission, MissionAbortReason,
    MissionOutcome, MissionRetargetEvent, Order, PlanetIntelSource, PlayerMode, Real48,
    SalvageResolvedEvent, Starbase, diplomatic_relation_toward, recompute_max_speed,
    set_armies_aboard, set_fleet_at_rest, set_mission_target, set_planet_name,
    set_starbase_at_rest, set_troop_transports, stored_production_points,
};
use nc_engine::run_maintenance_turn;
use std::path::Path;

/// Helper to load a fixture directory.
fn load_fixture(name: &str) -> CoreGameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
        .join("v1.5");
    load_classic_core_files(&dir)
        .unwrap_or_else(|e| panic!("Failed to load fixture {}: {}", name, e))
}

fn configured_delayed_hostile_arrival_state(
    order: Order,
    ships: (u16, u16, u16, u16, u16, u16, u16),
) -> (CoreGameData, usize, [u8; 2]) {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");
    let target_coords = [25, 25];
    let target_idx = 4;

    let target_world = &mut game_data.planets[target_idx];
    target_world.sector = Coords::from_array(target_coords);
    target_world.owner.0 = 2;
    target_world.years_under_owner = 2;
    set_planet_name(target_world, "Target");
    target_world.armies = 10;
    target_world.ground_batteries = 4;

    let fleet = &mut game_data.fleets[0];
    fleet.ships.battleships = ships.0;
    fleet.ships.cruisers = ships.1;
    fleet.ships.destroyers = ships.2;
    set_troop_transports(fleet, ships.3);
    set_armies_aboard(fleet, ships.4);
    fleet.ships.etacs = ships.5;
    fleet.ships.scouts = ships.6;
    recompute_max_speed(fleet);
    fleet.sector = Coords::new(24, 25);
    fleet.speed = 3;
    fleet.order = order;
    fleet.target = Coords::from_array(target_coords);
    set_mission_target(fleet, 1, 0);
    set_fleet_at_rest(fleet);

    (game_data, target_idx, target_coords)
}

fn mutual_enemy_overrides(left: u8, right: u8) -> [DiplomacyOverride; 2] {
    [
        DiplomacyOverride {
            from_empire: left,
            to_empire: right,
            relation: DiplomaticRelation::Enemy,
        },
        DiplomacyOverride {
            from_empire: right,
            to_empire: left,
            relation: DiplomaticRelation::Enemy,
        },
    ]
}

/// A base at rest in deep space at the centre of `coords`: it neither moves
/// nor needs a friendly world.
fn deep_space_base(base_id: u16, owner: u8, coords: [u8; 2]) -> Starbase {
    let mut base = Starbase::default();
    base.base_id.0 = base_id;
    base.owner.0 = owner;
    base.sector = Coords::from_array(coords);
    base.x_fraction = Real48::HALF;
    base.y_fraction = Real48::HALF;
    base.target = Coords::from_array(coords);
    base
}

fn add_active_starbase(game_data: &mut CoreGameData, owner: u8, coords: [u8; 2]) {
    let mut base = Starbase::default();
    base.base_id.0 = 1;
    base.record.0 = 1;
    base.sector = Coords::from_array(coords);
    base.target = Coords::from_array(coords);
    base.owner.0 = owner;
    game_data.bases.push(base);
}

#[test]
fn test_fleet_movement_and_colonization_fleets_dat() {
    // After one maintenance turn on the fleet pre-fixture, FLEETS.DAT should match
    // the post-fixture byte-for-byte.
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let post_data = load_fixture("ecmaint-fleet-post");

    run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert_eq!(
        serialize_fleets_dat(&game_data.fleets),
        serialize_fleets_dat(&post_data.fleets),
        "FLEETS.DAT should match post-fixture after 1 turn"
    );
}

#[test]
fn test_fleet_movement_and_colonization_planets_dat() {
    // After one maintenance turn, PLANETS.DAT should match the post-fixture.
    // This validates planet colonization (fleet 0 ColonizeWorld to (15,13)).
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let post_data = load_fixture("ecmaint-fleet-post");

    run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert_eq!(
        serialize_planets_dat(&game_data.planets),
        serialize_planets_dat(&post_data.planets),
        "PLANETS.DAT should match post-fixture after 1 turn"
    );
}

#[test]
fn test_fleet_movement_and_colonization_player_dat() {
    // After one maintenance turn, PLAYER.DAT should match the post-fixture.
    // This validates planet count and economic updates after colonization.
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let post_data = load_fixture("ecmaint-fleet-post");

    run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert_eq!(
        serialize_player_dat(&game_data.players),
        serialize_player_dat(&post_data.players),
        "PLAYER.DAT should match post-fixture after 1 turn"
    );
}

#[test]
fn test_fleet_movement_arrival_state() {
    // Specific validation: fleet 0 should arrive at (15,13) with cleared speed and order.
    let mut game_data = load_fixture("ecmaint-fleet-pre");

    // Pre-state: fleet 0 at (16,13) with ColonizeWorld order, speed=3
    assert_eq!(
        game_data.fleets[0].sector.to_array(),
        [16, 13],
        "Fleet 0 should start at (16,13)"
    );
    assert_eq!(
        game_data.fleets[0].speed, 3,
        "Fleet 0 should start with speed 3"
    );

    run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    // Post-state: fleet 0 should be at (15,13) with speed=0 and HoldPosition order
    assert_eq!(
        game_data.fleets[0].sector.to_array(),
        [15, 13],
        "Fleet 0 should arrive at (15,13)"
    );
    assert_eq!(
        game_data.fleets[0].speed, 0,
        "Fleet 0 should have speed 0 after arrival"
    );
    assert_eq!(
        game_data.fleets[0].order.code(),
        0,
        "Fleet 0 order should be cleared to HoldPosition (0) after arrival"
    );
    assert_eq!(
        game_data.fleets[0].order,
        Order::HoldPosition,
        "Fleet 0 should be HoldPosition after colonization arrival"
    );
}

#[test]
fn test_colonization_planet_state() {
    // Planet at (15,13) should be colonized by empire 1 after fleet 0 arrives.
    let mut game_data = load_fixture("ecmaint-fleet-pre");

    // Pre-state: planet 13 at (15,13) should be unowned
    let planet_13_pre = &game_data.planets[13];
    assert_eq!(
        planet_13_pre.sector.to_array(),
        [15, 13],
        "Planet 13 should be at (15,13)"
    );
    assert_eq!(
        planet_13_pre.owner.0, 0,
        "Planet 13 should be unowned pre-maint"
    );

    run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    // Post-state: planet 13 should be owned by empire 1
    let planet_13_post = &game_data.planets[13];
    assert_eq!(
        planet_13_post.owner.0, 1,
        "Planet 13 should be owned by empire 1 after colonization"
    );
    assert_eq!(
        planet_13_post.years_under_owner, 2,
        "Planet 13 ownership_status should be 2 after colonization"
    );
    assert_eq!(
        planet_13_post.armies, 1,
        "Planet 13 should have 1 colonist army"
    );
    assert_eq!(
        planet_13_post.name.clone(),
        "Not Named Yet",
        "Planet 13 should be named 'Not Named Yet' after colonization"
    );
}

#[test]
fn test_colonization_emits_success_event() {
    let mut game_data = load_fixture("ecmaint-fleet-pre");

    let events = run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert_eq!(events.colonization_events.len(), 1);
    assert!(matches!(
        events.colonization_events[0],
        ColonizationResolvedEvent::Succeeded {
            fleet_idx: 0,
            planet_idx: 13,
            colonizer_empire: 1,
            ..
        }
    ));
    assert!(events.mission_events.iter().any(|event| {
        event.fleet_idx == 0
            && event.kind == Mission::ColonizeWorld
            && event.outcome == MissionOutcome::Succeeded
            && event.planet_idx == Some(13)
    }));
    // Fleet must be reset to HoldPosition with speed 0 after successful colonization.
    assert_eq!(
        game_data.fleets[0].order,
        Order::HoldPosition,
        "Fleet should be HoldPosition after colonization"
    );
    assert_eq!(
        game_data.fleets[0].speed, 0,
        "Fleet should have speed 0 after colonization"
    );
}

#[test]
fn test_colonization_emits_blocked_event_for_occupied_world() {
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let target = &mut game_data.planets[13];
    target.owner.0 = 2;
    target.years_under_owner = 2;
    set_planet_name(target, "TargetPrime");
    target.armies = 10;

    let events = run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert_eq!(events.colonization_events.len(), 1);
    assert!(matches!(
        events.colonization_events[0],
        ColonizationResolvedEvent::BlockedByOwner {
            fleet_idx: 0,
            planet_idx: 13,
            colonizer_empire: 1,
            owner_empire: 2,
            ..
        }
    ));
    assert!(events.mission_events.iter().any(|event| {
        event.fleet_idx == 0
            && event.kind == Mission::ColonizeWorld
            && event.outcome == MissionOutcome::Failed
            && event.planet_idx == Some(13)
    }));
    assert!(events.planet_intel_events.iter().any(|event| {
        event.planet_idx == 13
            && event.viewer_empire == 1
            && event.source == PlanetIntelSource::ColonizeBlockedByOwner
            && event.source_fleet_idx == Some(0)
    }));
    let target = &game_data.planets[13];
    assert_eq!(target.owner.0, 2);
    assert_eq!(target.name.clone(), "TargetPrime");
}

#[test]
fn test_scout_sector_arrival_emits_success_event() {
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let scout = &mut game_data.fleets[0];
    scout.order = Order::ScoutSector;
    scout.target = Coords::new(15, 13);
    scout.ships.scouts = 1;
    // ECGAME writes the 0x22 world-target flag as 0 for sector orders.
    set_mission_target(scout, 0, 0);
    scout.ships.etacs = 0;

    let events = run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert!(events.colonization_events.is_empty());
    assert!(events.mission_events.iter().any(|event| {
        event.fleet_idx == 0
            && event.kind == Mission::ScoutSector
            && (event.outcome == MissionOutcome::Succeeded
                || event.outcome == MissionOutcome::Arrived)
            && event.planet_idx.is_none()
    }));
    // Scout persists on station after arrival.
    assert_eq!(game_data.fleets[0].order, Order::ScoutSector);
    assert_eq!(game_data.fleets[0].speed, 0);
}

#[test]
fn test_blockading_foreign_world_escalates_to_enemy() {
    let mut game_data = load_fixture("ecmaint-post");
    let (planet_idx, coords) = game_data
        .planets
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner.0 == 2)
        .map(|(idx, planet)| (idx, planet.sector.to_array()))
        .expect("fixture should contain an empire 2 world");

    let fleet = &mut game_data.fleets[0];
    fleet.sector = Coords::new(coords[0].saturating_add(1), coords[1]);
    fleet.order = Order::GuardBlockadeWorld;
    fleet.target = Coords::from_array(coords);
    fleet.speed = 3;
    fleet.depth = Real48::ZERO;
    // The world's own fleets would attack the blockader in their system.
    common::hold_fire(&mut game_data, 2);

    let events = run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert!(
        events
            .diplomatic_escalation_events
            .iter()
            .any(|event| event.left_empire == 1 && event.right_empire == 2)
    );
    assert_eq!(
        diplomatic_relation_toward(&game_data.players[0], 2),
        Some(DiplomaticRelation::Enemy)
    );
    assert_eq!(
        diplomatic_relation_toward(&game_data.players[1], 1),
        Some(DiplomaticRelation::Enemy)
    );
    assert!(events.mission_events.iter().any(|event| {
        event.kind == Mission::GuardBlockadeWorld
            && event.owner_empire == 1
            && event.planet_idx == Some(planet_idx)
    }));
}

#[test]
fn test_scout_system_arrival_emits_success_event() {
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let scout = &mut game_data.fleets[0];
    scout.order = Order::ScoutSolarSystem;
    scout.target = Coords::new(15, 13);
    scout.ships.scouts = 1;
    scout.ships.etacs = 0;
    // Cruiser speed settles in orbit one sector away within the year.
    recompute_max_speed(scout);
    scout.speed = scout.max_speed;

    let events = run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert!(events.colonization_events.is_empty());
    assert_eq!(events.planet_intel_events.len(), 1);
    assert_eq!(events.planet_intel_events[0].planet_idx, 13);
    assert_eq!(events.planet_intel_events[0].viewer_empire, 1);
    assert_eq!(
        events.planet_intel_events[0].source,
        PlanetIntelSource::ScoutSolarSystem
    );
    assert_eq!(events.planet_intel_events[0].source_fleet_idx, Some(0));
    assert!(events.planet_intel_events[0].observed_snapshot.is_some());
    assert!(events.mission_events.iter().any(|event| {
        event.fleet_idx == 0
            && event.kind == Mission::ScoutSolarSystem
            && (event.outcome == MissionOutcome::Succeeded
                || event.outcome == MissionOutcome::Arrived)
            && event.planet_idx == Some(13)
    }));
    assert_eq!(game_data.fleets[0].sector.to_array(), [15, 13]);
    // Scout persists on station after arrival.
    assert_eq!(game_data.fleets[0].order, Order::ScoutSolarSystem);
}

#[test]
fn test_view_world_arrival_emits_success_and_intel_event() {
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let viewer = &mut game_data.fleets[0];
    viewer.order = Order::ViewWorld;
    viewer.target = Coords::new(15, 13);
    viewer.ships.scouts = 0;
    viewer.ships.etacs = 0;
    // Cruiser speed settles in orbit one sector away within the year.
    recompute_max_speed(viewer);
    viewer.speed = viewer.max_speed;

    let events = run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert!(events.colonization_events.is_empty());
    assert_eq!(events.planet_intel_events.len(), 1);
    assert_eq!(events.planet_intel_events[0].planet_idx, 13);
    assert_eq!(events.planet_intel_events[0].viewer_empire, 1);
    assert_eq!(
        events.planet_intel_events[0].source,
        PlanetIntelSource::ViewWorld
    );
    assert_eq!(events.planet_intel_events[0].source_fleet_idx, Some(0));
    assert!(events.planet_intel_events[0].observed_snapshot.is_some());
    assert!(events.mission_events.iter().any(|event| {
        event.fleet_idx == 0
            && event.kind == Mission::ViewWorld
            && event.outcome == MissionOutcome::Succeeded
            && event.planet_idx == Some(13)
    }));
    assert_eq!(game_data.fleets[0].order, Order::HoldPosition);
    assert_eq!(game_data.fleets[0].speed, 0);
    assert_eq!(
        game_data.fleets[0].depth,
        Real48::ZERO,
        "ECMAINT's View World action lifts the fleet to depth 0"
    );
    assert!(
        events.mission_events.iter().any(|event| {
            event.fleet_idx == 0
                && event.kind == Mission::MoveOnly
                && event.outcome == MissionOutcome::Succeeded
        }),
        "the Move out of the system resolves in the same year and reports"
    );
}

#[test]
fn test_view_world_fleet_caught_by_an_enemy_commits_no_intel() {
    let (mut game_data, target_idx, target_coords) =
        configured_delayed_hostile_arrival_state(Order::ViewWorld, (0, 0, 1, 0, 0, 0, 0));
    game_data.planets[target_idx].owner.0 = 2;
    game_data.planets[target_idx].years_under_owner = 2;

    let enemy = &mut game_data.fleets[1];
    enemy.owner.0 = 2;
    enemy.record.0 = 9;
    enemy.sector = Coords::from_array(target_coords);
    enemy.order = Order::HoldPosition;
    enemy.target = Coords::from_array(target_coords);
    enemy.speed = 0;
    enemy.ships.battleships = 8;
    enemy.ships.cruisers = 8;
    enemy.ships.destroyers = 8;
    enemy.ships.scouts = 0;
    set_troop_transports(enemy, 0);
    set_armies_aboard(enemy, 0);
    enemy.ships.etacs = 0;
    enemy.rules_of_engagement = 10;

    // A year where the enemy caught the viewer in the system.
    let (_, events) = common::first_year(&game_data, &mutual_enemy_overrides(1, 2), |_, events| {
        events
            .fleet_battle_events
            .iter()
            .any(|event| event.reporting_empire == 1)
    });

    assert!(
        !events.planet_intel_events.iter().any(|event| {
            event.viewer_empire == 1 && event.source == PlanetIntelSource::ViewWorld
        }),
        "aborted ViewWorld should not commit intel"
    );
    assert!(
        events.mission_events.iter().any(|event| {
            event.fleet_idx == 0
                && event.kind == Mission::ViewWorld
                && event.outcome == MissionOutcome::Aborted
        }),
        "the battle aborts the ViewWorld mission"
    );
}

#[test]
fn test_scout_system_arrival_still_emits_intel_with_enemy_fleet_present() {
    let (mut game_data, target_idx, target_coords) =
        configured_delayed_hostile_arrival_state(Order::ScoutSolarSystem, (0, 0, 1, 0, 0, 0, 1));
    // Fast enough to settle in orbit within the year (1 + 1 + 1 sectors).
    game_data.fleets[0].speed = 6;
    game_data.planets[target_idx].owner.0 = 2;
    game_data.planets[target_idx].years_under_owner = 2;

    let enemy = &mut game_data.fleets[1];
    enemy.owner.0 = 2;
    enemy.record.0 = 9;
    enemy.sector = Coords::from_array(target_coords);
    enemy.order = Order::HoldPosition;
    enemy.target = Coords::from_array(target_coords);
    enemy.speed = 0;
    enemy.ships.battleships = 8;
    enemy.ships.cruisers = 8;
    enemy.ships.destroyers = 8;
    enemy.ships.scouts = 0;
    set_troop_transports(enemy, 0);
    set_armies_aboard(enemy, 0);
    enemy.ships.etacs = 0;
    enemy.rules_of_engagement = 10;

    // A year where the scout reached orbit before the enemy caught it.
    let (_, events) = common::first_year(&game_data, &mutual_enemy_overrides(1, 2), |_, events| {
        events
            .fleet_destroyed_events
            .iter()
            .all(|event| event.reporting_empire != 1)
            && events
                .mission_events
                .iter()
                .any(|event| event.fleet_idx == 0 && event.kind == Mission::ScoutSolarSystem)
    });

    assert!(
        events.planet_intel_events.iter().any(|event| {
            event.viewer_empire == 1 && event.source == PlanetIntelSource::ScoutSolarSystem
        }),
        "arrival ScoutSolarSystem should still emit intel before any later hostility logic"
    );
    assert!(
        events.mission_events.iter().any(|event| {
            event.fleet_idx == 0
                && event.kind == Mission::ScoutSolarSystem
                && event.outcome == MissionOutcome::Succeeded
        }),
        "arrival ScoutSolarSystem should still emit success on the arrival tick"
    );
    assert!(
        events
            .fleet_destroyed_events
            .iter()
            .all(|event| event.reporting_empire != 1),
        "the arriving scout should not be destroyed on the same tick"
    );
}

#[test]
fn test_seek_home_arrival_emits_success_event() {
    let mut game_data = load_fixture("ecmaint-post");
    let target_coords = game_data
        .planets
        .iter()
        .find(|planet| planet.owner.0 == 1)
        .map(|planet| planet.sector.to_array())
        .expect("fixture should contain an owned world");
    let fleet = &mut game_data.fleets[0];
    fleet.sector = Coords::new(target_coords[0].saturating_add(1), target_coords[1]);
    fleet.order = Order::SeekHome;
    fleet.target = Coords::from_array(target_coords);
    fleet.speed = 3;
    fleet.x_fraction = Real48::HALF;
    fleet.depth = Real48::ZERO;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert!(events.mission_events.iter().any(|event| {
        event.fleet_idx == 0
            && event.kind == Mission::SeekHome
            && event.outcome == MissionOutcome::Succeeded
            && event.location_coords == Some(target_coords)
    }));
}

#[test]
fn test_rendezvous_arrival_emits_waiting_event() {
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let fleet = &mut game_data.fleets[0];
    fleet.order = Order::RendezvousSector;
    fleet.target = Coords::new(15, 13);
    // ECGAME writes the 0x22 world-target flag as 0 for sector orders.
    set_mission_target(fleet, 0, 0);

    let events = run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert_eq!(game_data.fleets[0].order, Order::RendezvousSector);
    assert_eq!(game_data.fleets[0].target.to_array(), [15, 13]);

    assert!(events.mission_events.iter().any(|event| {
        event.fleet_idx == 0
            && event.kind == Mission::RendezvousSector
            && event.outcome == MissionOutcome::Arrived
            && event.location_coords == Some([15, 13])
    }));
}

#[test]
fn test_join_merge_emits_merge_event() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.players[0].mode = PlayerMode::Rogue;
    let coords = game_data.fleets[0].sector.to_array();
    game_data.fleets[1].sector = Coords::from_array(coords);
    game_data.fleets[1].order = Order::JoinAnotherFleet;

    let events = run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert!(events.fleet_merge_events.iter().any(|event| {
        event.fleet_idx == 1
            && event.kind == Mission::JoinAnotherFleet
            && event.owner_empire == 1
            && !event.survivor_side
    }));
}

#[test]
fn test_merges_that_would_overflow_scout_count_are_refused() {
    // Scout counts are stored in a word; merging two 40000-scout fleets must
    // not silently drop ships. Covers mission joins, rendezvous, and rogue
    // co-location consolidation.
    for (consolidation_flag, order) in [
        (0x00, Order::JoinAnotherFleet),
        (0x00, Order::RendezvousSector),
        (0xff, Order::HoldPosition),
    ] {
        let mut game_data = load_fixture("ecmaint-post");
        game_data.players[0].mode = PlayerMode::from_code(consolidation_flag);
        let coords = game_data.fleets[0].sector.to_array();
        let host_number = game_data.fleets[0].fleet_number.0;
        for idx in [0, 1] {
            let fleet = &mut game_data.fleets[idx];
            fleet.sector = Coords::from_array(coords);
            fleet.order = order;
            fleet.target = Coords::from_array(coords);
            fleet.speed = 0;
            fleet.ships.scouts = 40000;
        }
        if order == Order::JoinAnotherFleet {
            game_data.fleets[0].order = Order::HoldPosition;
            game_data.fleets[1].mission_target = host_number;
        }
        let empire_scouts = |game_data: &CoreGameData| -> u32 {
            game_data
                .fleets
                .iter()
                .filter(|fleet| fleet.owner.0 == 1)
                .map(|fleet| u32::from(fleet.ships.scouts))
                .sum()
        };
        let scouts_before = empire_scouts(&game_data);

        let events = run_maintenance_turn(&mut game_data).expect("Maintenance failed");

        assert_eq!(empire_scouts(&game_data), scouts_before, "{order:?}");
        assert!(
            events.fleet_merge_events.is_empty(),
            "{order:?}: {:?}",
            events.fleet_merge_events
        );
    }
}

#[test]
fn test_join_order_refreshes_target_to_moving_host_each_turn() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.players[0].mode = PlayerMode::CivilDisorder;

    let host = &mut game_data.fleets[0];
    host.sector = Coords::new(10, 10);
    host.order = Order::MoveOnly;
    host.target = Coords::new(14, 10);
    host.speed = 3;
    host.x_fraction = Real48::HALF;
    host.depth = Real48::HALF;

    let host_number = game_data.fleets[0].fleet_number.0;
    let joiner = &mut game_data.fleets[1];
    joiner.sector = Coords::new(4, 10);
    joiner.order = Order::JoinAnotherFleet;
    joiner.mission_target = host_number;
    joiner.target = Coords::new(10, 10);
    joiner.speed = 3;
    joiner.x_fraction = Real48::HALF;
    joiner.depth = Real48::HALF;

    run_maintenance_turn(&mut game_data).expect("first maintenance turn should succeed");

    let host_after_first = game_data.fleets[0].sector.to_array();
    assert_eq!(host_after_first, [12, 10]);
    assert_eq!(
        game_data.fleets[1].target.to_array(),
        [10, 10],
        "joiner should chase the host's turn-start position during the first turn"
    );

    run_maintenance_turn(&mut game_data).expect("second maintenance turn should succeed");

    assert_eq!(
        game_data.fleets[1].target.to_array(),
        host_after_first,
        "joiner should refresh to the host's latest location on the following turn"
    );
    assert_eq!(
        game_data.fleets[1].mission_target, host_number,
        "host identity should remain unchanged while the host survives"
    );
    assert_eq!(game_data.fleets[1].order, Order::JoinAnotherFleet);
}

#[test]
fn test_join_order_survives_arrival_at_previous_host_position() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.players[0].mode = PlayerMode::CivilDisorder;

    let host = &mut game_data.fleets[0];
    host.sector = Coords::new(10, 10);
    host.order = Order::MoveOnly;
    host.target = Coords::new(14, 10);
    host.speed = 3;
    host.x_fraction = Real48::HALF;
    host.depth = Real48::HALF;

    let host_number = game_data.fleets[0].fleet_number.0;
    let joiner = &mut game_data.fleets[1];
    joiner.sector = Coords::new(9, 10);
    joiner.order = Order::JoinAnotherFleet;
    joiner.mission_target = host_number;
    joiner.target = Coords::new(10, 10);
    joiner.speed = 3;
    joiner.x_fraction = Real48::HALF;
    joiner.depth = Real48::HALF;

    run_maintenance_turn(&mut game_data).expect("first maintenance turn should succeed");

    let host_after_first = game_data.fleets[0].sector.to_array();
    assert_eq!(host_after_first, [12, 10]);
    assert_eq!(game_data.fleets[1].sector.to_array(), [10, 10]);
    assert_eq!(game_data.fleets[1].order, Order::JoinAnotherFleet);
    assert_eq!(
        game_data.fleets[1].speed, 3,
        "joiner should keep chasing speed after arriving at the host's previous position"
    );

    run_maintenance_turn(&mut game_data).expect("second maintenance turn should succeed");

    assert_eq!(game_data.fleets[1].order, Order::JoinAnotherFleet);
    assert_eq!(game_data.fleets[1].target.to_array(), host_after_first);
    assert_eq!(
        game_data.fleets[1].speed, 3,
        "joiner speed should remain armed for continued pursuit on the following turn"
    );
}

#[test]
fn test_join_order_abandons_mission_when_host_is_destroyed() {
    let mut game_data = load_fixture("ecmaint-post");

    let host_number = game_data.fleets[0].fleet_number.0;
    game_data.fleets[0].ships.destroyers = 0;
    game_data.fleets[0].ships.cruisers = 0;
    game_data.fleets[0].ships.battleships = 0;
    game_data.fleets[0].ships.scouts = 0;
    set_troop_transports(&mut game_data.fleets[0], 0);
    game_data.fleets[0].ships.etacs = 0;

    let joiner_coords = [7, 9];
    let joiner = &mut game_data.fleets[1];
    joiner.sector = Coords::from_array(joiner_coords);
    joiner.order = Order::JoinAnotherFleet;
    joiner.mission_target = host_number;
    joiner.target = Coords::new(10, 10);
    joiner.speed = 3;

    let fleet_count_before = game_data.fleets.len();
    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    // The empty host is culled at turn start; lost-host join missions now
    // resolve immediately instead of burning a movement step first.
    assert_eq!(game_data.fleets.len(), fleet_count_before - 1);
    assert_eq!(game_data.fleets[0].order, Order::HoldPosition);
    assert_eq!(game_data.fleets[0].speed, 0);
    let abandoned_coords = game_data.fleets[0].sector.to_array();
    assert_eq!(abandoned_coords, joiner_coords);
    assert_eq!(game_data.fleets[0].target.to_array(), abandoned_coords);
    assert_eq!(game_data.fleets[0].mission_target, 0);
    assert!(events.join_host_events.iter().any(|event| {
        matches!(
            event,
            JoinMissionHostEvent::HostDestroyed {
                fleet_idx,
                destroyed_host_fleet_number: Some(1),
                coords,
                ..
            } if *fleet_idx == 0
                && *coords == abandoned_coords
        )
    }));
}

#[test]
fn test_joiner_retargets_when_host_merges_into_surviving_fleet() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.players[0].mode = PlayerMode::CivilDisorder;

    let survivor_coords = [10, 10];
    let survivor_number = game_data.fleets[0].fleet_number.0;
    let absorbed_host_number = game_data.fleets[1].fleet_number.0;

    {
        let survivor = &mut game_data.fleets[0];
        survivor.sector = Coords::from_array(survivor_coords);
        survivor.order = Order::HoldPosition;
        survivor.target = Coords::from_array(survivor_coords);
        survivor.speed = 0;
    }
    {
        let host = &mut game_data.fleets[1];
        host.sector = Coords::from_array(survivor_coords);
        host.order = Order::JoinAnotherFleet;
        host.mission_target = survivor_number;
        host.target = Coords::from_array(survivor_coords);
        host.speed = 0;
    }
    {
        let joiner = &mut game_data.fleets[2];
        joiner.sector = Coords::new(5, 10);
        joiner.order = Order::JoinAnotherFleet;
        joiner.mission_target = absorbed_host_number;
        joiner.target = Coords::from_array(survivor_coords);
        joiner.speed = 3;
    }

    let fleet_count_before = game_data.fleets.len();
    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(game_data.fleets.len(), fleet_count_before - 1);
    let joiner = &game_data.fleets[1];
    assert_eq!(joiner.order, Order::JoinAnotherFleet);
    assert_eq!(joiner.mission_target, survivor_number);
    assert_eq!(joiner.target.to_array(), survivor_coords);
    assert!(events.join_host_events.iter().any(|event| {
        matches!(
            event,
            JoinMissionHostEvent::Retargeted {
                fleet_idx,
                previous_host_fleet_number: Some(2),
                new_host_fleet_number: Some(1),
                ..
            } if *fleet_idx == 1
        )
    }));
    assert!(!events.invalid_player_state_events.iter().any(|event| {
        matches!(
            event,
            nc_data::InvalidPlayerStateEvent::FleetMission {
                reason: FleetOrderValidationError::InvalidJoinHost,
                ..
            }
        )
    }));
}

#[test]
fn test_joiner_merges_same_turn_after_host_merges_into_surviving_fleet() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.players[0].mode = PlayerMode::CivilDisorder;

    let coords = [10, 10];
    let survivor_number = game_data.fleets[0].fleet_number.0;
    let absorbed_host_number = game_data.fleets[1].fleet_number.0;

    {
        let survivor = &mut game_data.fleets[0];
        survivor.sector = Coords::from_array(coords);
        survivor.order = Order::HoldPosition;
        survivor.target = Coords::from_array(coords);
        survivor.speed = 0;
    }
    {
        let host = &mut game_data.fleets[1];
        host.sector = Coords::from_array(coords);
        host.order = Order::JoinAnotherFleet;
        host.mission_target = survivor_number;
        host.target = Coords::from_array(coords);
        host.speed = 0;
    }
    {
        let joiner = &mut game_data.fleets[2];
        joiner.sector = Coords::from_array(coords);
        joiner.order = Order::JoinAnotherFleet;
        joiner.mission_target = absorbed_host_number;
        joiner.target = Coords::from_array(coords);
        joiner.speed = 0;
    }

    let fleet_count_before = game_data.fleets.len();
    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(game_data.fleets.len(), fleet_count_before - 2);
    assert!(events.fleet_merge_events.iter().any(|event| {
        event.kind == Mission::JoinAnotherFleet
            && !event.survivor_side
            && event.absorbed_fleet_number == 2
            && event.host_fleet_number == 1
    }));
    assert!(events.fleet_merge_events.iter().any(|event| {
        event.kind == Mission::JoinAnotherFleet
            && !event.survivor_side
            && event.absorbed_fleet_number == 3
            && event.host_fleet_number == 1
    }));
    assert!(!events.invalid_player_state_events.iter().any(|event| {
        matches!(
            event,
            nc_data::InvalidPlayerStateEvent::FleetMission {
                reason: FleetOrderValidationError::InvalidJoinHost,
                ..
            }
        )
    }));
}

#[test]
fn test_seek_home_retargets_to_next_owned_planet_when_target_is_lost() {
    let mut game_data = load_fixture("ecmaint-post");
    let original_target = game_data.planets[0].sector.to_array();
    game_data.planets[0].owner.0 = 2;
    let fallback_target = game_data
        .planets
        .iter()
        .find(|planet| planet.owner.0 == 1)
        .map(|planet| planet.sector.to_array())
        .expect("fixture should still have another owned planet");

    let fleet = &mut game_data.fleets[1];
    fleet.sector = Coords::new(9, 9);
    fleet.order = Order::SeekHome;
    fleet.target = Coords::from_array(original_target);
    fleet.speed = 3;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(game_data.fleets[1].target.to_array(), fallback_target);
    assert_eq!(game_data.fleets[1].speed, 3);
    assert!(events.mission_retarget_events.iter().any(|event| {
        matches!(
            event,
            MissionRetargetEvent::Retargeted {
                fleet_idx,
                reporting_fleet_number,
                mission,
                current_coords,
                previous_target_coords,
                new_target_coords,
                ..
            } if *fleet_idx == 1
                && *reporting_fleet_number == Some(game_data.fleets[1].fleet_number.0 as u8)
                && *mission == Mission::SeekHome
                && *current_coords == [9, 9]
                && *previous_target_coords == original_target
                && *new_target_coords == fallback_target
        )
    }));
}

#[test]
fn test_guard_starbase_retargets_to_live_base_coords() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.bases = vec![deep_space_base(3, 1, [12, 8])];

    let fleet = &mut game_data.fleets[1];
    fleet.sector = Coords::new(4, 8);
    fleet.order = Order::GuardStarbase;
    fleet.target = Coords::new(9, 8);
    fleet.speed = 3;
    set_mission_target(fleet, 1, 3);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(game_data.fleets[1].target.to_array(), [12, 8]);
    assert_eq!(game_data.fleets[1].speed, 3);
    // ECMAINT keeps the 0x22 world-target flag on guard-starbase fleets.
    assert_eq!(
        [
            game_data.fleets[1].target_depth,
            game_data.fleets[1].mission_target as u8
        ],
        [1, 3]
    );
    assert!(events.mission_retarget_events.iter().any(|event| {
        matches!(
            event,
            MissionRetargetEvent::Retargeted {
                fleet_idx,
                reporting_fleet_number,
                mission,
                current_coords,
                previous_target_coords,
                new_target_coords,
                ..
            } if *fleet_idx == 1
                && *reporting_fleet_number == Some(game_data.fleets[1].fleet_number.0 as u8)
                && *mission == Mission::GuardStarbase
                && *current_coords == [4, 8]
                && *previous_target_coords == [9, 8]
                && *new_target_coords == [12, 8]
        )
    }));
}

#[test]
fn test_guard_starbase_abandons_when_linked_base_is_missing() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.bases = vec![];

    let fleet = &mut game_data.fleets[1];
    fleet.sector = Coords::new(4, 8);
    fleet.order = Order::GuardStarbase;
    fleet.target = Coords::new(9, 8);
    fleet.speed = 3;
    set_mission_target(fleet, 1, 3);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(game_data.fleets[1].order, Order::HoldPosition);
    assert_eq!(game_data.fleets[1].speed, 0);
    assert_eq!(
        game_data.fleets[1].target.to_array(),
        game_data.fleets[1].sector.to_array()
    );
    assert!(events.mission_retarget_events.iter().any(|event| {
        matches!(
            event,
            MissionRetargetEvent::Abandoned {
                fleet_idx,
                reporting_fleet_number,
                mission,
                previous_target_coords,
                ..
            } if *fleet_idx == 1
                && *reporting_fleet_number == Some(game_data.fleets[1].fleet_number.0 as u8)
                && *mission == Mission::GuardStarbase
                && *previous_target_coords == [9, 8]
        )
    }));
}

#[test]
fn test_guard_starbase_persists_after_arrival() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.bases = vec![deep_space_base(3, 1, [11, 8])];

    let fleet = &mut game_data.fleets[1];
    fleet.sector = Coords::new(10, 8);
    fleet.order = Order::GuardStarbase;
    fleet.target = Coords::new(11, 8);
    fleet.speed = 3;
    set_mission_target(fleet, 1, 3);
    fleet.x_fraction = Real48::HALF;
    fleet.depth = Real48::HALF;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(game_data.fleets[1].sector.to_array(), [11, 8]);
    assert_eq!(game_data.fleets[1].order, Order::GuardStarbase);
    assert_eq!(game_data.fleets[1].speed, 0);
    assert_eq!(game_data.fleets[1].target.to_array(), [11, 8]);
    // It arrives on entering the base's sector and layer and takes the
    // base's depth and layer (0x22 = 0, t040); it becomes the base's escort.
    assert_eq!(
        [
            game_data.fleets[1].target_depth,
            game_data.fleets[1].mission_target as u8
        ],
        [0, 3]
    );
    assert!(game_data.fleets[1].depth.is_zero());
    assert_eq!(
        game_data.bases[0].escort.0,
        game_data.fleets[1].fleet_number.0
    );
    assert!(events.mission_events.iter().any(|event| {
        event.fleet_idx == 1
            && event.kind == Mission::GuardStarbase
            && event.outcome == MissionOutcome::Arrived
            && event.location_coords == Some([11, 8])
    }));
}

#[test]
fn test_guard_starbase_with_zero_index_holds_as_classic() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.bases = vec![deep_space_base(3, 1, [11, 8])];

    let fleet = &mut game_data.fleets[1];
    fleet.sector = Coords::new(11, 8);
    fleet.order = Order::GuardStarbase;
    fleet.target = Coords::new(11, 8);
    fleet.speed = 0;
    set_mission_target(fleet, 0, 0);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    // ECMAINT finds no Starbase 0 and holds the fleet (`0b5d:0cf6`), even
    // with a base at its target.
    assert_eq!(game_data.fleets[1].order, Order::HoldPosition);
    assert_eq!(game_data.fleets[1].speed, 0);
    assert_eq!(
        [
            game_data.fleets[1].target_depth,
            game_data.fleets[1].mission_target as u8
        ],
        [0, 0]
    );
    assert!(events.mission_retarget_events.iter().any(|event| {
        matches!(
            event,
            MissionRetargetEvent::Abandoned {
                fleet_idx,
                mission,
                ..
            } if *fleet_idx == 1 && *mission == Mission::GuardStarbase
        )
    }));
}

#[test]
fn test_guard_starbase_with_zero_index_abandons_when_target_base_is_missing() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.bases = vec![];

    let fleet = &mut game_data.fleets[1];
    fleet.sector = Coords::new(11, 8);
    fleet.order = Order::GuardStarbase;
    fleet.target = Coords::new(11, 8);
    fleet.speed = 0;
    set_mission_target(fleet, 0, 0);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(game_data.fleets[1].order, Order::HoldPosition);
    assert_eq!(game_data.fleets[1].speed, 0);
    assert_eq!(
        [
            game_data.fleets[1].target_depth,
            game_data.fleets[1].mission_target as u8
        ],
        [0, 0]
    );
    assert!(events.mission_retarget_events.iter().any(|event| {
        matches!(
            event,
            MissionRetargetEvent::Abandoned {
                fleet_idx,
                reporting_fleet_number,
                mission,
                previous_target_coords,
                ..
            } if *fleet_idx == 1
                && *reporting_fleet_number == Some(game_data.fleets[1].fleet_number.0 as u8)
                && *mission == Mission::GuardStarbase
                && *previous_target_coords == [11, 8]
        )
    }));
}

#[test]
fn test_guard_blockade_world_persists_after_isolated_arrival() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");
    let target_coords = [11, 8];

    let target_world = &mut game_data.planets[4];
    target_world.sector = Coords::from_array(target_coords);
    target_world.owner.0 = 2;
    target_world.years_under_owner = 2;
    set_planet_name(target_world, "Target");

    let fleet = &mut game_data.fleets[0];
    fleet.sector = Coords::new(10, 8);
    fleet.order = Order::GuardBlockadeWorld;
    fleet.target = Coords::from_array(target_coords);
    fleet.speed = 3;
    fleet.x_fraction = Real48::HALF;
    fleet.depth = Real48::HALF;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(game_data.fleets[0].sector.to_array(), target_coords);
    assert_eq!(game_data.fleets[0].order, Order::GuardBlockadeWorld);
    assert_eq!(game_data.fleets[0].speed, 0);
    assert_eq!(game_data.fleets[0].target.to_array(), target_coords);
    assert!(events.mission_events.iter().any(|event| {
        event.fleet_idx == 0
            && event.kind == Mission::GuardBlockadeWorld
            && event.outcome == MissionOutcome::Arrived
            && event.location_coords == Some(target_coords)
    }));
}

#[test]
fn test_patrol_sector_persists_after_arrival() {
    let mut game_data = load_fixture("ecmaint-post");
    let fleet = &mut game_data.fleets[1];
    fleet.sector = Coords::new(10, 10);
    fleet.order = Order::PatrolSector;
    fleet.target = Coords::new(11, 10);
    fleet.speed = 3;
    fleet.x_fraction = Real48::HALF;
    fleet.depth = Real48::HALF;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(game_data.fleets[1].sector.to_array(), [11, 10]);
    assert_eq!(game_data.fleets[1].order, Order::PatrolSector);
    assert_eq!(game_data.fleets[1].speed, 0);
    assert!(events.mission_events.iter().any(|event| {
        event.fleet_idx == 1
            && event.kind == Mission::PatrolSector
            && event.outcome == MissionOutcome::Arrived
    }));
}

#[test]
/// Classic ECMAINT completes the move the year the fleet rounds into the
/// target sector, with its exact path still short of the target point.
fn test_move_only_diagonal_completes_when_it_rounds_into_the_target() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    let fleet = &mut game_data.fleets[0];
    fleet.ships.battleships = 0;
    fleet.ships.cruisers = 0;
    fleet.ships.destroyers = 0;
    set_troop_transports(fleet, 0);
    fleet.ships.etacs = 0;
    fleet.ships.scouts = 1;
    recompute_max_speed(fleet);
    fleet.sector = Coords::new(10, 10);
    fleet.speed = 3;
    fleet.order = Order::MoveOnly;
    fleet.target = Coords::new(16, 16);
    set_mission_target(fleet, 0, 0);
    set_fleet_at_rest(fleet);

    run_maintenance_turn(&mut game_data).expect("turn 1 should succeed");
    run_maintenance_turn(&mut game_data).expect("turn 2 should succeed");
    let turn3_events = run_maintenance_turn(&mut game_data).expect("turn 3 should succeed");

    let fleet = &game_data.fleets[0];
    assert_eq!(fleet.sector.to_array(), [16, 16]);
    assert_eq!(fleet.order, Order::HoldPosition);
    assert_eq!(fleet.speed, 0);
    assert!(turn3_events.mission_events.iter().any(|event| {
        event.fleet_idx == 0
            && event.kind == Mission::MoveOnly
            && event.outcome == MissionOutcome::Succeeded
            && event.location_coords == Some([16, 16])
    }));
}

#[test]
fn test_move_only_horizontal_clears_to_hold_on_exact_arrival() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    let fleet = &mut game_data.fleets[0];
    fleet.ships.battleships = 0;
    fleet.ships.cruisers = 0;
    fleet.ships.destroyers = 0;
    set_troop_transports(fleet, 0);
    fleet.ships.etacs = 0;
    fleet.ships.scouts = 1;
    recompute_max_speed(fleet);
    fleet.sector = Coords::new(10, 10);
    set_mission_target(fleet, 1, 0);
    game_data
        .set_fleet_order(1, 3, Order::MoveOnly.code(), [16, 10], None, None)
        .expect("move-only order should apply");
    game_data.fleets[0].depth = Real48::ONE;

    run_maintenance_turn(&mut game_data).expect("turn 1 should succeed");
    run_maintenance_turn(&mut game_data).expect("turn 2 should succeed");
    let turn3_events = run_maintenance_turn(&mut game_data).expect("turn 3 should succeed");

    let fleet = &game_data.fleets[0];
    assert_eq!(fleet.sector.to_array(), [16, 10]);
    assert_eq!(fleet.order, Order::HoldPosition);
    assert_eq!(fleet.speed, 0);
    assert!(turn3_events.mission_events.iter().any(|event| {
        event.fleet_idx == 0
            && event.kind == Mission::MoveOnly
            && event.outcome == MissionOutcome::Succeeded
            && event.location_coords == Some([16, 10])
    }));
}

#[test]
fn test_delayed_hostile_orders_keep_speed_and_order_just_short_of_orbit() {
    let cases = [
        (
            "bombard",
            Order::BombardWorld,
            Mission::BombardWorld,
            (0, 3, 5, 0, 0, 0, 0),
        ),
        (
            "invade",
            Order::InvadeWorld,
            Mission::InvadeWorld,
            (0, 1, 0, 10, 10, 0, 0),
        ),
        (
            "blitz",
            Order::BlitzWorld,
            Mission::BlitzWorld,
            (100, 50, 50, 50, 50, 0, 0),
        ),
    ];

    for (name, order, mission, ships) in cases {
        let (mut game_data, target_idx, target_coords) =
            configured_delayed_hostile_arrival_state(order, ships);

        let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");
        let fleet = &game_data.fleets[0];

        assert_eq!(fleet.sector.to_array(), target_coords, "{name} coords");
        assert_eq!(fleet.order, order, "{name} order");
        // Speed 3 from rest one sector out ends the year just short of
        // orbit (depth 1 - epsilon), still under way; it strikes next year.
        assert_eq!(fleet.speed, 3, "{name} arrival speed");
        assert_eq!(fleet.target.to_array(), target_coords, "{name} target");
        assert_eq!(
            fleet.depth.to_bytes(),
            [0x80, 0xb9, 0xff, 0xff, 0xff, 0x7f],
            "{name} depth"
        );
        assert!(events.mission_events.iter().any(|event| {
            event.fleet_idx == 0
                && event.kind == mission
                && event.outcome == MissionOutcome::Arrived
                && event.planet_idx == Some(target_idx)
                && event.location_coords == Some(target_coords)
        }));
    }
}

#[test]
fn test_join_merge_occurs_without_combat_merge_flag() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.players[0].mode = PlayerMode::CivilDisorder;
    let host_coords = game_data.fleets[0].sector.to_array();
    let host_number = game_data.fleets[0].fleet_number.0;

    let joiner = &mut game_data.fleets[1];
    joiner.sector = Coords::from_array(host_coords);
    joiner.order = Order::JoinAnotherFleet;
    joiner.mission_target = host_number;
    joiner.target = Coords::from_array(host_coords);

    let fleet_count_before = game_data.fleets.len();
    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(game_data.fleets.len(), fleet_count_before - 1);
    assert!(
        events
            .fleet_merge_events
            .iter()
            .any(|event| { event.kind == Mission::JoinAnotherFleet && !event.survivor_side })
    );
}

#[test]
fn test_join_order_written_as_ecgame_writes_it_moves_toward_host_and_merges() {
    // ECGAME stores Join as 0x22 = 1 (world/fleet target flag) and 0x23 = host id.
    let mut game_data = load_fixture("ecmaint-post");
    game_data.players[0].mode = PlayerMode::CivilDisorder;

    let host = &mut game_data.fleets[2];
    host.sector = Coords::new(10, 10);
    host.order = Order::HoldPosition;
    host.target = Coords::new(10, 10);
    host.speed = 0;
    let host_number = game_data.fleets[2].fleet_number.0 as u8;
    assert_ne!(
        host_number, 1,
        "host id must differ from the 0x22 world-target flag"
    );

    let joiner = &mut game_data.fleets[1];
    joiner.sector = Coords::new(7, 10);
    joiner.order = Order::JoinAnotherFleet;
    joiner.target = Coords::new(10, 10);
    set_mission_target(joiner, 1, u16::from(host_number));
    joiner.speed = 3;
    joiner.x_fraction = Real48::HALF;
    joiner.depth = Real48::HALF;
    assert_eq!(joiner.mission_target, u16::from(host_number));

    let fleet_count_before = game_data.fleets.len();
    run_maintenance_turn(&mut game_data).expect("first maintenance turn should succeed");

    let joiner = &game_data.fleets[1];
    assert_eq!(joiner.order, Order::JoinAnotherFleet);
    assert_eq!(
        [joiner.target_depth, joiner.mission_target as u8],
        [1, host_number]
    );
    assert_ne!(
        joiner.sector.to_array(),
        [7, 10],
        "joiner should move toward the host on the first turn"
    );

    let events =
        run_maintenance_turn(&mut game_data).expect("second maintenance turn should succeed");

    assert_eq!(
        game_data.fleets.len(),
        fleet_count_before - 1,
        "joiner should merge into the host once it arrives"
    );
    assert!(
        events
            .fleet_merge_events
            .iter()
            .any(|event| { event.kind == Mission::JoinAnotherFleet && !event.survivor_side })
    );
}

#[test]
fn test_join_merge_occurs_when_joiner_has_hidden_exact_transit_in_host_sector() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.players[0].mode = PlayerMode::CivilDisorder;
    let host_coords = game_data.fleets[0].sector.to_array();
    let host_number = game_data.fleets[0].fleet_number.0;

    let joiner = &mut game_data.fleets[1];
    joiner.sector = Coords::from_array(host_coords);
    joiner.order = Order::JoinAnotherFleet;
    joiner.mission_target = host_number;
    joiner.target = Coords::from_array(host_coords);
    joiner.speed = 3;
    set_in_transit_west_of_centre(joiner);

    let fleet_count_before = game_data.fleets.len();
    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(game_data.fleets.len(), fleet_count_before - 1);
    assert!(events.fleet_merge_events.iter().any(|event| {
        event.kind == Mission::JoinAnotherFleet
            && !event.survivor_side
            && event.fleet_idx == 1
            && event.coords == host_coords
    }));
}

#[test]
fn test_move_only_arrival_completes_when_hidden_exact_transit_reaches_target_sector() {
    let mut game_data = load_fixture("ecmaint-post");
    let target_coords = [20, 20];

    let fleet = &mut game_data.fleets[1];
    fleet.sector = Coords::from_array(target_coords);
    fleet.order = Order::MoveOnly;
    fleet.target = Coords::from_array(target_coords);
    fleet.speed = 3;
    set_in_transit_west_of_centre(fleet);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let fleet = &game_data.fleets[1];
    assert_eq!(fleet.sector.to_array(), target_coords);
    assert_eq!(fleet.order, Order::HoldPosition);
    assert_eq!(fleet.speed, 0);
    assert!(events.mission_events.iter().any(|event| {
        event.fleet_idx == 1
            && event.kind == Mission::MoveOnly
            && event.outcome == MissionOutcome::Succeeded
            && event.location_coords == Some(target_coords)
    }));
}

#[test]
fn test_rendezvous_merge_occurs_without_combat_merge_flag() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.players[0].mode = PlayerMode::CivilDisorder;
    let coords = game_data.fleets[0].sector.to_array();
    game_data.fleets[0].order = Order::RendezvousSector;
    game_data.fleets[0].target = Coords::from_array(coords);
    game_data.fleets[1].sector = Coords::from_array(coords);
    game_data.fleets[1].order = Order::RendezvousSector;
    game_data.fleets[1].target = Coords::from_array(coords);

    let fleet_count_before = game_data.fleets.len();
    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(game_data.fleets.len(), fleet_count_before - 1);
    assert!(
        events
            .fleet_merge_events
            .iter()
            .any(|event| { event.kind == Mission::RendezvousSector && !event.survivor_side })
    );
}

#[test]
fn test_rendezvous_merge_occurs_when_arrival_completes_from_hidden_exact_transit() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.players[0].mode = PlayerMode::CivilDisorder;
    let coords = game_data.fleets[0].sector.to_array();

    game_data.fleets[0].order = Order::RendezvousSector;
    game_data.fleets[0].target = Coords::from_array(coords);

    let joiner = &mut game_data.fleets[1];
    joiner.sector = Coords::from_array(coords);
    joiner.order = Order::RendezvousSector;
    joiner.target = Coords::from_array(coords);
    joiner.speed = 3;
    set_in_transit_west_of_centre(joiner);

    let fleet_count_before = game_data.fleets.len();
    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(game_data.fleets.len(), fleet_count_before - 1);
    assert!(events.fleet_merge_events.iter().any(|event| {
        event.kind == Mission::RendezvousSector
            && !event.survivor_side
            && event.fleet_idx == 1
            && event.coords == coords
    }));
}

#[test]
fn test_rendezvous_merge_occurs_same_turn_when_fleet_arrives_at_assigned_sector() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.players[0].mode = PlayerMode::CivilDisorder;
    let coords = game_data.fleets[0].sector.to_array();

    game_data.fleets[0].order = Order::RendezvousSector;
    game_data.fleets[0].target = Coords::from_array(coords);

    let joiner = &mut game_data.fleets[1];
    joiner.sector = Coords::new(coords[0].saturating_sub(1), coords[1]);
    joiner.order = Order::RendezvousSector;
    joiner.target = Coords::from_array(coords);
    joiner.speed = 3;
    joiner.x_fraction = Real48::HALF;
    joiner.depth = Real48::HALF;

    let fleet_count_before = game_data.fleets.len();
    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(game_data.fleets.len(), fleet_count_before - 1);
    assert!(events.fleet_merge_events.iter().any(|event| {
        event.kind == Mission::RendezvousSector
            && !event.survivor_side
            && event.fleet_idx == 1
            && event.coords == coords
    }));
}

#[test]
fn test_rendezvous_host_selection_uses_lowest_structural_fleet_id() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.players[0].mode = PlayerMode::CivilDisorder;
    let coords = game_data.fleets[0].sector.to_array();

    game_data.fleets[0].fleet_number.0 = 9;
    game_data.fleets[0].order = Order::RendezvousSector;
    game_data.fleets[0].target = Coords::from_array(coords);
    let survivor_id = game_data.fleets[0].record.0;

    game_data.fleets[1].fleet_number.0 = 1;
    game_data.fleets[1].sector = Coords::from_array(coords);
    game_data.fleets[1].order = Order::RendezvousSector;
    game_data.fleets[1].target = Coords::from_array(coords);

    run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let survivor = game_data
        .fleets
        .iter()
        .find(|fleet| fleet.record.0 == survivor_id)
        .expect("lowest structural fleet id should remain the rendezvous host");
    assert_eq!(survivor.fleet_number.0, 9);
    assert_eq!(survivor.order, Order::RendezvousSector);
    assert_eq!(survivor.target.to_array(), coords);
}

#[test]
fn test_rendezvous_does_not_merge_before_reaching_its_assigned_sector() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.players[0].mode = PlayerMode::CivilDisorder;
    let coords = game_data.fleets[0].sector.to_array();

    game_data.fleets[0].order = Order::RendezvousSector;
    game_data.fleets[0].target = Coords::from_array(coords);
    game_data.fleets[1].sector = Coords::from_array(coords);
    game_data.fleets[1].order = Order::RendezvousSector;
    game_data.fleets[1].target = Coords::new(coords[0] + 1, coords[1]);
    game_data.fleets[1].speed = 0;

    let fleet_count_before = game_data.fleets.len();
    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(game_data.fleets.len(), fleet_count_before);
    assert!(
        events
            .fleet_merge_events
            .iter()
            .all(|event| { event.kind != Mission::RendezvousSector })
    );
}

#[test]
fn test_salvage_converts_fleet_value_into_owned_planet_production_and_removes_fleet() {
    let mut game_data = load_fixture("ecmaint-post");
    let (planet_idx, target_coords) = game_data
        .planets
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner.0 == 1)
        .map(|(idx, planet)| (idx, planet.sector.to_array()))
        .expect("fixture should contain an owned planet");
    let start_coords = if target_coords[0] > 1 {
        [target_coords[0] - 1, target_coords[1]]
    } else {
        [target_coords[0] + 1, target_coords[1]]
    };
    let stored_before = stored_production_points(&game_data.planets[planet_idx]);
    let fleet_count_before = game_data.fleets.len();

    let fleet = &mut game_data.fleets[0];
    fleet.sector = Coords::from_array(start_coords);
    fleet.order = Order::Salvage;
    fleet.target = Coords::from_array(target_coords);
    fleet.speed = 3;
    fleet.ships.destroyers = 1;
    fleet.ships.cruisers = 1;
    fleet.ships.battleships = 0;
    fleet.ships.scouts = 0;
    set_troop_transports(fleet, 0);
    set_armies_aboard(fleet, 0);
    fleet.ships.etacs = 0;
    fleet.x_fraction = Real48::HALF;
    fleet.depth = Real48::ZERO;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(game_data.fleets.len(), fleet_count_before - 1);
    // A destroyer yields 3 points and a cruiser 8: each ship's half cost,
    // rounded up (`0b5d:4d65`).
    assert_eq!(
        stored_production_points(&game_data.planets[planet_idx]),
        stored_before + 11,
    );
    assert!(events.salvage_events.iter().any(|event| {
        matches!(
            event,
            SalvageResolvedEvent::Succeeded {
                owner_empire,
                planet_idx: event_planet_idx,
                coords,
                recovered_points,
                ..
            } if *owner_empire == 1
                && *event_planet_idx == planet_idx
                && *coords == target_coords
                && *recovered_points == 11
        )
    }));
}

#[test]
fn test_empty_fleet_is_culled_at_turn_start() {
    // A fleet with no ships is culled at the very start of the maintenance
    // turn, before movement or salvage resolution runs.  The fleet must be
    // removed, no SalvageResolvedEvent must fire, and the target planet's
    // treasury must be untouched.
    let mut game_data = load_fixture("ecmaint-post");
    let (planet_idx, target_coords) = game_data
        .planets
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner.0 == 1)
        .map(|(idx, planet)| (idx, planet.sector.to_array()))
        .expect("fixture should contain an owned planet");
    let start_coords = if target_coords[0] > 1 {
        [target_coords[0] - 1, target_coords[1]]
    } else {
        [target_coords[0] + 1, target_coords[1]]
    };
    let stored_before = stored_production_points(&game_data.planets[planet_idx]);
    let fleet_count_before = game_data.fleets.len();

    let fleet = &mut game_data.fleets[0];
    fleet.sector = Coords::from_array(start_coords);
    fleet.order = Order::Salvage;
    fleet.target = Coords::from_array(target_coords);
    fleet.speed = 3;
    // Zero out all ship counts — this fleet is an empty shell.
    fleet.ships.destroyers = 0;
    fleet.ships.cruisers = 0;
    fleet.ships.battleships = 0;
    fleet.ships.scouts = 0;
    set_troop_transports(fleet, 0);
    set_armies_aboard(fleet, 0);
    fleet.ships.etacs = 0;
    fleet.x_fraction = Real48::HALF;
    fleet.depth = Real48::ZERO;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    // Fleet removed by the start-of-turn cull.
    assert_eq!(
        game_data.fleets.len(),
        fleet_count_before - 1,
        "empty fleet must be culled at turn start"
    );
    // Salvage never ran — no salvage event emitted.
    assert!(
        events.salvage_events.is_empty(),
        "no SalvageResolvedEvent should fire when the fleet is culled before movement"
    );
    // Planet treasury must be unchanged.
    assert_eq!(
        stored_production_points(&game_data.planets[planet_idx]),
        stored_before,
        "planet treasury must not change when the empty fleet is culled before salvage"
    );
}

#[test]
fn test_salvage_whose_world_is_not_ours_salvages_at_a_friendly_world() {
    // ECMAINT checks a Salvage fleet's target world every tick; one that is
    // not the empire's turns for the nearest friendly world and keeps the
    // Salvage order (`0b5d:4a44`).
    let mut game_data = load_fixture("ecmaint-post");
    let (planet_idx, home_coords) = game_data
        .planets
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner.0 == 1)
        .map(|(idx, planet)| (idx, planet.sector.to_array()))
        .expect("fixture should contain an owned planet");
    let foreign_coords = game_data
        .planets
        .iter()
        .find(|planet| planet.owner.0 == 2)
        .map(|planet| planet.sector.to_array())
        .expect("fixture should contain a foreign planet");
    let start_coords = if home_coords[0] > 1 {
        [home_coords[0] - 1, home_coords[1]]
    } else {
        [home_coords[0] + 1, home_coords[1]]
    };
    let stored_before = stored_production_points(&game_data.planets[planet_idx]);
    let fleet_count_before = game_data.fleets.len();

    let fleet = &mut game_data.fleets[0];
    fleet.sector = Coords::from_array(start_coords);
    fleet.order = Order::Salvage;
    fleet.target = Coords::from_array(foreign_coords);
    fleet.speed = 3;
    fleet.ships.destroyers = 1;
    fleet.ships.cruisers = 0;
    fleet.ships.battleships = 0;
    fleet.ships.scouts = 0;
    set_troop_transports(fleet, 0);
    set_armies_aboard(fleet, 0);
    fleet.ships.etacs = 0;
    fleet.x_fraction = Real48::HALF;
    fleet.depth = Real48::ZERO;
    let fleet_number = fleet.fleet_number.0 as u8;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(game_data.fleets.len(), fleet_count_before - 1);
    assert_eq!(
        stored_production_points(&game_data.planets[planet_idx]),
        stored_before + 3,
    );
    assert!(events.mission_retarget_events.iter().any(|event| {
        matches!(
            event,
            MissionRetargetEvent::Retargeted {
                reporting_fleet_number,
                mission,
                previous_target_coords,
                new_target_coords,
                stardate_week,
                ..
            } if *reporting_fleet_number == Some(fleet_number)
                && *mission == Mission::Salvage
                && *previous_target_coords == foreign_coords
                && *new_target_coords == home_coords
                && *stardate_week == Some(1)
        )
    }));
    assert!(events.salvage_events.iter().any(|event| {
        matches!(
            event,
            SalvageResolvedEvent::Succeeded {
                owner_empire,
                planet_idx: event_planet_idx,
                coords,
                recovered_points,
                ..
            } if *owner_empire == 1
                && *event_planet_idx == planet_idx
                && *coords == home_coords
                && *recovered_points == 3
        )
    }));
}

#[test]
fn test_rendezvous_merge_emits_survivor_absorption_event() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.players[0].mode = PlayerMode::Rogue;
    let coords = game_data.fleets[0].sector.to_array();
    game_data.fleets[0].order = Order::RendezvousSector;
    game_data.fleets[1].sector = Coords::from_array(coords);
    game_data.fleets[1].order = Order::RendezvousSector;

    let survivor_id = game_data.fleets[0].record.0;
    let absorbed_id = game_data.fleets[1].record.0;

    let events = run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert!(events.fleet_merge_events.iter().any(|event| {
        event.kind == Mission::RendezvousSector
            && event.survivor_side
            && event.host_fleet_id == survivor_id
            && event.absorbed_fleet_id == absorbed_id
    }));
}

#[test]
fn test_rendezvous_survivor_remains_on_rendezvous_after_merge() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.players[0].mode = PlayerMode::CivilDisorder;
    let coords = game_data.fleets[0].sector.to_array();
    game_data.fleets[0].order = Order::RendezvousSector;
    game_data.fleets[0].target = Coords::from_array(coords);
    game_data.fleets[1].sector = Coords::from_array(coords);
    game_data.fleets[1].order = Order::RendezvousSector;
    game_data.fleets[1].target = Coords::from_array(coords);

    let survivor_id = game_data.fleets[0].record.0;

    run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    let survivor = game_data
        .fleets
        .iter()
        .find(|fleet| fleet.record.0 == survivor_id)
        .expect("survivor fleet should remain present");
    assert_eq!(survivor.order, Order::RendezvousSector);
    assert_eq!(survivor.target.to_array(), coords);
}

#[test]
fn test_rendezvous_merge_recomputes_survivor_speed_to_slowest_member() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.players[0].mode = PlayerMode::Rogue;
    let coords = game_data.fleets[0].sector.to_array();

    game_data.fleets[0].order = Order::RendezvousSector;
    game_data.fleets[0].ships.destroyers = 1;
    game_data.fleets[0].ships.cruisers = 0;
    game_data.fleets[0].ships.battleships = 0;
    game_data.fleets[0].ships.scouts = 0;
    set_troop_transports(&mut game_data.fleets[0], 0);
    game_data.fleets[0].ships.etacs = 0;
    game_data.fleets[0].max_speed = 6;

    game_data.fleets[1].sector = Coords::from_array(coords);
    game_data.fleets[1].order = Order::RendezvousSector;
    game_data.fleets[1].ships.destroyers = 0;
    game_data.fleets[1].ships.cruisers = 0;
    game_data.fleets[1].ships.battleships = 0;
    game_data.fleets[1].ships.scouts = 0;
    set_troop_transports(&mut game_data.fleets[1], 0);
    game_data.fleets[1].ships.etacs = 1;
    game_data.fleets[1].max_speed = 3;

    run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert_eq!(game_data.fleets[0].max_speed, 3);
}

#[test]
fn test_merge_preserves_surviving_local_fleet_numbers() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.players[0].mode = PlayerMode::Rogue;
    let coords = game_data.fleets[0].sector.to_array();
    game_data.fleets[2].sector = Coords::new(1, 1);
    game_data.fleets[3].sector = Coords::new(2, 2);
    game_data.fleets[1].sector = Coords::from_array(coords);
    game_data.fleets[1].order = Order::JoinAnotherFleet;

    run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    let player1_local_slots = game_data
        .fleets
        .iter()
        .filter(|fleet| fleet.owner.0 == 1)
        .map(|fleet| fleet.fleet_number.0)
        .collect::<Vec<_>>();
    assert_eq!(player1_local_slots, vec![1, 3, 4]);

    let player2_local_slots = game_data
        .fleets
        .iter()
        .filter(|fleet| fleet.owner.0 == 2)
        .map(|fleet| fleet.fleet_number.0)
        .collect::<Vec<_>>();
    assert_eq!(player2_local_slots, vec![1, 2, 3, 4]);
}

// ---------------------------------------------------------------------------
// Group A — One-shot on-station
// ---------------------------------------------------------------------------

#[test]
fn test_view_world_on_station_reverts_to_hold_position() {
    // Fleet already at target coords from a prior turn (on-station path).
    // ViewWorld is one-shot: the observation fires AND the order resets to HoldPosition.
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let planet_coords = game_data.planets[13].sector.to_array();
    let viewer = &mut game_data.fleets[0];
    viewer.sector = Coords::from_array(planet_coords);
    viewer.order = Order::ViewWorld;
    viewer.target = Coords::from_array(planet_coords);
    viewer.speed = 0;
    viewer.ships.scouts = 0;
    viewer.ships.etacs = 0;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    // Observation must fire.
    assert!(
        events.planet_intel_events.iter().any(|e| {
            e.planet_idx == 13 && e.viewer_empire == 1 && e.source == PlanetIntelSource::ViewWorld
        }),
        "on-station ViewWorld must emit a planet intel event"
    );
    assert!(
        events.mission_events.iter().any(|e| {
            e.fleet_idx == 0
                && e.kind == Mission::ViewWorld
                && e.outcome == MissionOutcome::Succeeded
        }),
        "on-station ViewWorld must emit a Succeeded mission event"
    );

    // ViewWorld is one-shot: order must revert to HoldPosition.
    assert_eq!(
        game_data.fleets[0].order,
        Order::HoldPosition,
        "on-station ViewWorld must reset to HoldPosition after firing"
    );
    assert_eq!(game_data.fleets[0].speed, 0);
    assert_eq!(
        game_data.fleets[0].depth,
        Real48::ZERO,
        "ECMAINT's View World action lifts the fleet to depth 0"
    );
}

#[test]
fn test_colonize_world_on_station_colonizes_and_resets() {
    // Fleet already at its target planet from a prior turn (on-station path).
    // ColonizeWorld is one-shot: the colonization must fire AND the order must
    // reset to HoldPosition with speed 0 — even though should_move is false.
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let planet_coords = game_data.planets[13].sector.to_array(); // unowned planet at (15,13)
    let colonizer = &mut game_data.fleets[0];
    colonizer.sector = Coords::from_array(planet_coords);
    colonizer.order = Order::ColonizeWorld;
    colonizer.target = Coords::from_array(planet_coords);
    colonizer.speed = 3; // speed > 0 to confirm the gate is the blocker
    colonizer.ships.etacs = 3;
    colonizer.ships.scouts = 0;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    // Planet must be colonized.
    assert_eq!(
        game_data.planets[13].owner.0, 1,
        "on-station ColonizeWorld must colonize the planet"
    );
    assert_eq!(
        game_data.planets[13].years_under_owner, 2,
        "on-station ColonizeWorld must set ownership_status to 2"
    );

    // Fleet must be reset to HoldPosition with speed 0.
    assert_eq!(
        game_data.fleets[0].order,
        Order::HoldPosition,
        "on-station ColonizeWorld must reset to HoldPosition after firing"
    );
    assert_eq!(
        game_data.fleets[0].speed, 0,
        "on-station ColonizeWorld must set speed to 0 after firing"
    );

    // Events must be emitted.
    assert!(
        events.colonization_events.iter().any(|e| matches!(
            e,
            ColonizationResolvedEvent::Succeeded {
                fleet_idx: 0,
                planet_idx: 13,
                colonizer_empire: 1,
                ..
            }
        )),
        "on-station ColonizeWorld must emit a Succeeded colonization event"
    );
    assert!(
        events.mission_events.iter().any(|e| {
            e.fleet_idx == 0
                && e.kind == Mission::ColonizeWorld
                && e.outcome == MissionOutcome::Succeeded
        }),
        "on-station ColonizeWorld must emit a Succeeded mission event"
    );
}

// ---------------------------------------------------------------------------
// Group B — Persistent observation on-station
// ---------------------------------------------------------------------------

#[test]
fn test_scout_sector_on_station_fires_report_and_persists() {
    // ScoutSector fleet already at target from a prior turn.
    // Must emit a mission event and keep Order::ScoutSector.
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let planet_coords = game_data.planets[13].sector.to_array();
    let scout = &mut game_data.fleets[0];
    scout.sector = Coords::from_array(planet_coords);
    scout.order = Order::ScoutSector;
    scout.target = Coords::from_array(planet_coords);
    scout.speed = 0;
    scout.ships.scouts = 1;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert!(
        events.mission_events.iter().any(|e| {
            e.fleet_idx == 0
                && e.kind == Mission::ScoutSector
                && e.outcome == MissionOutcome::Succeeded
        }),
        "on-station ScoutSector must emit a Succeeded mission event"
    );
    assert_eq!(
        game_data.fleets[0].order,
        Order::ScoutSector,
        "ScoutSector must persist on station after firing"
    );
    assert_eq!(game_data.fleets[0].speed, 0);
}

#[test]
fn test_scout_system_on_station_fires_intel_and_persists() {
    // ScoutSolarSystem fleet already at target from a prior turn.
    // Must emit a planet intel event and a mission event, and keep Order::ScoutSolarSystem.
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let planet_coords = game_data.planets[13].sector.to_array();
    let scout = &mut game_data.fleets[0];
    scout.sector = Coords::from_array(planet_coords);
    scout.order = Order::ScoutSolarSystem;
    scout.target = Coords::from_array(planet_coords);
    scout.speed = 0;
    scout.ships.scouts = 1;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert!(
        events.planet_intel_events.iter().any(|e| {
            e.planet_idx == 13
                && e.viewer_empire == 1
                && e.source == PlanetIntelSource::ScoutSolarSystem
        }),
        "on-station ScoutSolarSystem must emit a planet intel event"
    );
    assert!(
        events.mission_events.iter().any(|e| {
            e.fleet_idx == 0
                && e.kind == Mission::ScoutSolarSystem
                && e.outcome == MissionOutcome::Succeeded
        }),
        "on-station ScoutSolarSystem must emit a Succeeded mission event"
    );
    assert_eq!(
        game_data.fleets[0].order,
        Order::ScoutSolarSystem,
        "ScoutSolarSystem must persist on station after firing"
    );
    assert_eq!(game_data.fleets[0].speed, 0);
}

// ---------------------------------------------------------------------------
// Group C — Persistent guard/patrol on-station (no observation loop arm)
// ---------------------------------------------------------------------------

#[test]
fn test_patrol_sector_on_station_persists() {
    // PatrolSector fleet already at target from a prior turn.
    // No on-station observation fires; order must simply persist.
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let target_coords = [15, 10];
    let fleet = &mut game_data.fleets[0];
    fleet.sector = Coords::from_array(target_coords);
    fleet.order = Order::PatrolSector;
    fleet.target = Coords::from_array(target_coords);
    fleet.speed = 0;

    run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(
        game_data.fleets[0].order,
        Order::PatrolSector,
        "PatrolSector must persist on station"
    );
    assert_eq!(game_data.fleets[0].speed, 0);
}

#[test]
fn test_guard_blockade_world_on_station_persists() {
    // GuardBlockadeWorld fleet already at target from a prior turn.
    // No on-station observation fires; order must simply persist.
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let target_coords = game_data.planets[13].sector.to_array();
    let fleet = &mut game_data.fleets[0];
    fleet.sector = Coords::from_array(target_coords);
    fleet.order = Order::GuardBlockadeWorld;
    fleet.target = Coords::from_array(target_coords);
    fleet.speed = 0;

    run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(
        game_data.fleets[0].order,
        Order::GuardBlockadeWorld,
        "GuardBlockadeWorld must persist on station"
    );
    assert_eq!(game_data.fleets[0].speed, 0);
}

// ---------------------------------------------------------------------------
// Group D — Hostile order turn-2 execution (shape only)
// ---------------------------------------------------------------------------

/// Build a game state where a hostile fleet is already on-station and ready to
/// execute (transit_ready_flag == 0x80).  The target planet at [25,25] is
/// owned by empire 2 with 10 armies and 4 ground batteries.
fn hostile_on_station_ready(
    order: Order,
    ships: (u16, u16, u16, u16, u16, u16),
) -> (CoreGameData, usize, [u8; 2]) {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    let target_coords = [25, 25];
    let target_idx = 4;

    let target_world = &mut game_data.planets[target_idx];
    target_world.sector = Coords::from_array(target_coords);
    target_world.owner.0 = 2;
    target_world.years_under_owner = 2;
    set_planet_name(target_world, "Target");
    target_world.armies = 10;
    target_world.ground_batteries = 4;

    // (battleships, cruisers, destroyers, transports, armies, scouts)
    let fleet = &mut game_data.fleets[0];
    fleet.ships.battleships = ships.0;
    fleet.ships.cruisers = ships.1;
    fleet.ships.destroyers = ships.2;
    set_troop_transports(fleet, ships.3);
    set_armies_aboard(fleet, ships.4);
    fleet.ships.scouts = ships.5;
    fleet.ships.etacs = 0;
    recompute_max_speed(fleet);
    fleet.sector = Coords::from_array(target_coords);
    fleet.order = order;
    fleet.target = Coords::from_array(target_coords);
    // At rest in orbit (depth 1.0): the strike test passes on the first tick.
    fleet.speed = 0;
    set_fleet_at_rest(fleet);

    (game_data, target_idx, target_coords)
}

fn hostile_mixed_ready_exact_transit_state(
    order: Order,
    ships: (u16, u16, u16, u16, u16, u16, u16),
) -> (CoreGameData, usize, [u8; 2]) {
    let (mut game_data, target_idx, target_coords) =
        configured_delayed_hostile_arrival_state(order, ships);
    let fleet = &mut game_data.fleets[0];
    fleet.sector = Coords::from_array(target_coords);
    fleet.speed = 3;
    set_in_transit_west_of_centre(fleet);
    (game_data, target_idx, target_coords)
}

#[test]
fn test_bombard_world_executes_on_second_turn() {
    // A BombardWorld fleet that arrived last turn (transit_ready_flag == 0x80)
    // must fire a BombardEvent this turn and keep Order::BombardWorld.
    let (mut game_data, target_idx, _) =
        hostile_on_station_ready(Order::BombardWorld, (0, 0, 1, 0, 0, 0));

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert!(
        !events.bombard_events.is_empty(),
        "BombardWorld should fire a bombardment event on the second turn"
    );
    assert!(
        events
            .bombard_events
            .iter()
            .any(|e| e.planet_idx == target_idx && e.attacker_empire == 1),
        "BombardEvent must reference the target planet and attacker empire"
    );
    assert_eq!(
        game_data.fleets[0].order,
        Order::BombardWorld,
        "BombardWorld fleet must keep its order after executing"
    );
}

#[test]
fn test_invade_world_executes_on_second_turn() {
    // An InvadeWorld fleet that arrived last turn must fire an AssaultReportEvent.
    // InvadeWorld requires combat ships + loaded transports; use 1 destroyer + 1 transport + 1 army.
    let (mut game_data, target_idx, _) =
        hostile_on_station_ready(Order::InvadeWorld, (0, 0, 1, 1, 1, 0));

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert!(
        !events.assault_report_events.is_empty(),
        "InvadeWorld should fire an assault report event on the second turn"
    );
    assert!(
        events
            .assault_report_events
            .iter()
            .any(|e| e.kind == Mission::InvadeWorld
                && e.planet_idx == target_idx
                && e.attacker_empire == 1),
        "AssaultReportEvent must reference InvadeWorld, the target planet, and attacker empire"
    );
}

#[test]
fn test_blitz_world_executes_on_second_turn() {
    // A BlitzWorld fleet that arrived last turn must fire an AssaultReportEvent.
    // BlitzWorld needs loaded transports + orbital presence to achieve supremacy;
    // use 1 destroyer + 1 transport + 1 army.
    let (mut game_data, target_idx, _) =
        hostile_on_station_ready(Order::BlitzWorld, (0, 0, 1, 1, 1, 0));

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert!(
        !events.assault_report_events.is_empty(),
        "BlitzWorld should fire an assault report event on the second turn"
    );
    assert!(
        events
            .assault_report_events
            .iter()
            .any(|e| e.kind == Mission::BlitzWorld
                && e.planet_idx == target_idx
                && e.attacker_empire == 1),
        "AssaultReportEvent must reference BlitzWorld, the target planet, and attacker empire"
    );
}

#[test]
fn test_tt_only_blitz_executes_without_combat_ships() {
    let (mut game_data, target_idx, target_coords) =
        hostile_on_station_ready(Order::BlitzWorld, (0, 0, 0, 12, 12, 0));
    game_data.planets[target_idx].ground_batteries = 0;
    game_data.planets[target_idx].armies = 3;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert!(
        events.assault_report_events.iter().any(|event| {
            event.kind == Mission::BlitzWorld
                && event.planet_idx == target_idx
                && event.attacker_empire == 1
                && event.outcome != MissionOutcome::Aborted
        }),
        "loaded TT*-only blitz should resolve as a real assault"
    );
    assert!(
        !events.mission_events.iter().any(|event| {
            event.kind == Mission::BlitzWorld && event.outcome == MissionOutcome::Aborted
        }),
        "TT*-only blitz should not be aborted just because it has no combat ships"
    );
    assert_eq!(
        game_data.fleets[0].order,
        Order::HoldPosition,
        "resolved blitz should clear back to HoldPosition"
    );
    assert_eq!(
        game_data.fleets[0].sector.to_array(),
        target_coords,
        "fleet should stay on station after the blitz resolves"
    );
}

#[test]
fn test_tt_only_blitz_ignores_enemy_support_only_orbit() {
    let (mut game_data, target_idx, _) =
        hostile_on_station_ready(Order::BlitzWorld, (0, 0, 0, 12, 12, 0));
    game_data.planets[target_idx].ground_batteries = 0;
    game_data.planets[target_idx].armies = 3;

    let enemy = &mut game_data.fleets[1];
    enemy.owner.0 = 2;
    enemy.sector = game_data.planets[target_idx].sector;
    enemy.order = Order::HoldPosition;
    enemy.speed = 0;
    set_troop_transports(enemy, 2);
    set_armies_aboard(enemy, 0);
    enemy.ships.destroyers = 0;
    enemy.ships.cruisers = 0;
    enemy.ships.battleships = 0;
    enemy.ships.scouts = 0;
    enemy.ships.etacs = 0;
    enemy.rules_of_engagement = 0;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert!(
        events.assault_report_events.iter().any(|event| {
            event.kind == Mission::BlitzWorld
                && event.planet_idx == target_idx
                && event.outcome != MissionOutcome::Aborted
        }),
        "support-only enemy orbit should not block a TT*-only blitz"
    );
}

#[test]
fn test_tt_only_blitz_blocked_by_starbase_clears_order() {
    let (mut game_data, target_idx, target_coords) =
        hostile_on_station_ready(Order::BlitzWorld, (0, 0, 0, 12, 12, 0));
    game_data.planets[target_idx].ground_batteries = 0;
    game_data.planets[target_idx].armies = 3;
    add_active_starbase(&mut game_data, 2, target_coords);
    // The orbital gate (t054) is a layer-1 matter: the base blocks from
    // orbit, not from the deep space the helper leaves it in.
    let base = game_data.bases.last_mut().expect("the base was added");
    set_starbase_at_rest(base);
    base.target_depth = 1;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");
    let abort = events
        .mission_events
        .iter()
        .find(|event| event.kind == Mission::BlitzWorld && event.outcome == MissionOutcome::Aborted)
        .expect("blocked TT*-only blitz should emit an aborted mission event");

    assert_eq!(
        abort.abort_reason,
        Some(MissionAbortReason::OrbitBlocked),
        "transports alone cannot fight the base: the orbit is blocked"
    );
    assert!(
        events.fleet_battle_events.is_empty(),
        "nothing to fight the base with: no shot is fired"
    );
    assert!(
        events.assault_report_events.is_empty(),
        "blocked TT*-only blitz should not resolve a ground assault"
    );
    // A fleet that cannot get past the orbit seeks home (t054), as it does
    // when a sighted base turns it back.
    assert_eq!(
        game_data.fleets[0].order,
        Order::SeekHome,
        "blocked blitz should abort to Seek Home"
    );
}

#[test]
fn test_on_station_hostile_orders_assigned_via_mutator_execute_next_turn() {
    let cases = [
        (Order::BombardWorld, Mission::BombardWorld),
        (Order::InvadeWorld, Mission::InvadeWorld),
        (Order::BlitzWorld, Mission::BlitzWorld),
    ];

    for (order, mission) in cases {
        let ships = match order {
            Order::BombardWorld => (0, 0, 1, 0, 0, 0, 0),
            Order::InvadeWorld => (0, 0, 1, 2, 6, 0, 0),
            Order::BlitzWorld => (0, 0, 1, 2, 6, 0, 0),
            _ => unreachable!(),
        };
        let (mut game_data, target_idx, target_coords) =
            configured_delayed_hostile_arrival_state(order, ships);

        {
            let fleet = &mut game_data.fleets[0];
            fleet.sector = Coords::from_array(target_coords);
            fleet.speed = fleet.max_speed;
        }
        game_data
            .set_fleet_order(
                1,
                game_data.fleets[0].max_speed,
                order.code(),
                target_coords,
                None,
                None,
            )
            .expect("on-station hostile order should arm");

        // The order keeps the fleet's stored motion: at rest in orbit
        // (depth 1.0), so its strike test passes on the first tick.
        let fleet = &game_data.fleets[0];
        assert_eq!(fleet.speed, fleet.max_speed);
        assert_eq!(fleet.depth, Real48::ONE);
        assert_eq!(fleet.sector.to_array(), target_coords);
        assert_eq!(fleet.order, order);

        let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

        match mission {
            Mission::BombardWorld => assert!(
                events
                    .bombard_events
                    .iter()
                    .any(|e| e.planet_idx == target_idx && e.attacker_empire == 1),
                "BombardWorld should execute on the next maintenance tick"
            ),
            Mission::InvadeWorld | Mission::BlitzWorld => assert!(
                events.assault_report_events.iter().any(|e| {
                    e.kind == mission && e.planet_idx == target_idx && e.attacker_empire == 1
                }),
                "{mission:?} should execute on the next maintenance tick"
            ),
            _ => unreachable!(),
        }
    }
}

#[test]
fn test_stale_off_target_hostile_orders_resume_travel_instead_of_executing() {
    let cases = [
        ("bombard", Order::BombardWorld, Mission::BombardWorld),
        ("invade", Order::InvadeWorld, Mission::InvadeWorld),
        ("blitz", Order::BlitzWorld, Mission::BlitzWorld),
    ];

    for (name, order, mission) in cases {
        let ships = match order {
            Order::BombardWorld => (0, 0, 1, 0, 0, 0, 0),
            Order::InvadeWorld => (0, 0, 1, 2, 6, 0, 0),
            Order::BlitzWorld => (0, 0, 1, 4, 8, 0, 0),
            _ => unreachable!(),
        };
        let (mut game_data, target_idx, target_coords) =
            configured_delayed_hostile_arrival_state(order, ships);

        let fleet = &mut game_data.fleets[0];
        fleet.sector = Coords::new(15, 25);
        fleet.speed = 0;
        let mut depth = fleet.depth.to_bytes();
        depth[..5].copy_from_slice(&[0x80, 0xb9, 0xff, 0xff, 0xff]);
        fleet.depth = Real48::from_bytes(depth);
        let pre_batteries =
            u8::try_from(game_data.planets[target_idx].ground_batteries).unwrap_or(u8::MAX);
        let pre_armies = u8::try_from(game_data.planets[target_idx].armies).unwrap_or(u8::MAX);

        let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");
        let fleet = &game_data.fleets[0];

        assert_eq!(fleet.order, order, "{name} order should persist");
        assert_eq!(fleet.target.to_array(), target_coords, "{name} target");
        assert_eq!(
            fleet.speed,
            fleet.max_speed.max(1),
            "{name} speed should rearm"
        );
        assert_eq!(
            fleet.depth,
            Real48::ZERO,
            "{name} should climb out and resume travel"
        );
        assert_ne!(
            fleet.sector.to_array(),
            target_coords,
            "{name} should not teleport to the target"
        );
        assert!(
            events
                .mission_events
                .iter()
                .all(|event| !(event.fleet_idx == 0 && event.kind == mission)),
            "{name} should not emit hostile mission events while still off target"
        );
        assert!(
            events.bombard_events.is_empty(),
            "{name} should not emit bombardment execution while off target"
        );
        assert!(
            events.assault_report_events.is_empty(),
            "{name} should not emit assault execution while off target"
        );
        assert_eq!(
            u8::try_from(game_data.planets[target_idx].ground_batteries).unwrap_or(u8::MAX),
            pre_batteries,
            "{name} should not change target batteries while off target"
        );
        assert_eq!(
            u8::try_from(game_data.planets[target_idx].armies).unwrap_or(u8::MAX),
            pre_armies,
            "{name} should not change target armies while off target"
        );
    }
}

#[test]
fn test_hostile_orders_strike_in_the_year_they_settle_in_orbit() {
    // ECMAINT runs the attack on the tick the fleet's depth reaches 1.0, in
    // the same year, not the year after it reaches the target sector.
    let cases = [
        ("bombard", Order::BombardWorld, (0, 3, 5, 0, 0, 0, 0)),
        ("invade", Order::InvadeWorld, (0, 1, 0, 10, 10, 0, 0)),
        ("blitz", Order::BlitzWorld, (100, 50, 50, 50, 50, 0, 0)),
    ];

    for (name, order, ships) in cases {
        // In the target sector, 0.4 from the centre, in deep space: 1.4 of
        // the year's 3 sectors of travel reach orbit.
        let (mut game_data, _target_idx, target_coords) =
            hostile_mixed_ready_exact_transit_state(order, ships);

        let first_turn =
            run_maintenance_turn(&mut game_data).expect("first maintenance turn should succeed");

        match order {
            Order::BombardWorld => assert!(
                !first_turn.bombard_events.is_empty(),
                "{name} should strike in the year it reaches orbit"
            ),
            Order::InvadeWorld | Order::BlitzWorld => assert!(
                !first_turn.assault_report_events.is_empty(),
                "{name} should strike in the year it reaches orbit"
            ),
            _ => unreachable!(),
        }
        assert_eq!(
            game_data.fleets[0].sector.to_array(),
            target_coords,
            "{name} coords"
        );
    }
}

#[test]
fn test_join_succeeds_when_fleet_id_remapped_by_prior_salvage_removal() {
    // When fleet 1 (fleet_id=2) is removed by salvage during movement, fleet 2
    // compresses from fleet_id=3 to fleet_id=2 but keeps fleet number 3. The
    // joiner names its host by that number, so the join still resolves on the
    // same turn.
    let mut game_data = load_fixture("ecmaint-post");
    // Disable early consolidation so it cannot merge or remove fleets before
    // the salvage-triggered remap this test is exercising.
    game_data.players[0].mode = PlayerMode::CivilDisorder;

    // Find a planet owned by player 1 for the salvage fleet to target.
    let (_, planet_coords) = game_data
        .planets
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner.0 == 1)
        .map(|(idx, planet)| (idx, planet.sector.to_array()))
        .expect("fixture should contain an owned planet for player 1");

    // Fleet 2 (fleet_id=3) is the host — stationary at a fixed location.
    let host_coords = [10u8, 10u8];
    let host_number = game_data.fleets[2].fleet_number.0; // invariant: == 3
    {
        let host = &mut game_data.fleets[2];
        host.sector = Coords::from_array(host_coords);
        host.order = Order::HoldPosition;
        host.speed = 0;
        host.x_fraction = Real48::ZERO;
        host.depth = Real48::ZERO;
    }

    // Fleet 0 (fleet_id=1) is the joiner — co-located with the host, storing
    // the host's current fleet_id (3) in join_host_fleet_id.
    {
        let joiner = &mut game_data.fleets[0];
        joiner.sector = Coords::from_array(host_coords);
        joiner.order = Order::JoinAnotherFleet;
        joiner.mission_target = host_number;
        joiner.target = Coords::from_array(host_coords);
        joiner.speed = 0;
    }

    // Fleet 1 (fleet_id=2) is the salvage fleet — one sector from the planet,
    // carrying ships so recovered_points > 0 and the fleet is removed on arrival.
    let salvage_start = if planet_coords[0] > 1 {
        [planet_coords[0] - 1, planet_coords[1]]
    } else {
        [planet_coords[0] + 1, planet_coords[1]]
    };
    {
        let salvager = &mut game_data.fleets[1];
        salvager.sector = Coords::from_array(salvage_start);
        salvager.order = Order::Salvage;
        salvager.target = Coords::from_array(planet_coords);
        salvager.speed = 3;
        salvager.ships.destroyers = 1;
        salvager.ships.cruisers = 0;
        salvager.ships.battleships = 0;
        salvager.ships.scouts = 0;
        set_troop_transports(salvager, 0);
        set_armies_aboard(salvager, 0);
        salvager.ships.etacs = 0;
        salvager.x_fraction = Real48::HALF;
        salvager.depth = Real48::ZERO;
    }

    let fleet_count_before = game_data.fleets.len();
    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    // Salvage removes fleet 1 (-1); join merges fleet 0 into the host (-1).
    assert_eq!(
        game_data.fleets.len(),
        fleet_count_before - 2,
        "salvage removal and join merge must each reduce the fleet count by one"
    );
    assert!(
        events
            .fleet_merge_events
            .iter()
            .any(|event| event.kind == Mission::JoinAnotherFleet && !event.survivor_side),
        "JoinAnotherFleet merge event must be emitted — join must not silently fail after remap"
    );
}

/// In deep space inside its current sector, 0.4 west of the centre.
fn set_in_transit_west_of_centre(fleet: &mut nc_data::Fleet) {
    fleet.x_fraction = Real48::from_int(1) / Real48::from_int(10);
    fleet.y_fraction = Real48::HALF;
    fleet.depth = Real48::ZERO;
}

#[test]
fn test_join_host_is_named_by_fleet_number_not_record_id() {
    // ECMAINT's merge (`2895:4a6b`) matches a joiner's 0x23 word against the
    // host's fleet number (0x00). Empire 2's fleets are records 5..8 with
    // numbers 1..4, so record 6 (fleet 2) joining with 0x23 = 1 merges into
    // record 5 (fleet 1); with 0x23 = 5 (the record id) ECMAINT leaves the
    // files untouched.
    let mut game_data = load_fixture("ecmaint-post");
    for player in &mut game_data.players {
        player.mode = PlayerMode::CivilDisorder;
    }
    let host = &game_data.fleets[4];
    assert_eq!((host.owner.0, host.fleet_number.0), (2, 1));
    assert_eq!(host.record.0, 5);
    let host_cruisers = host.ships.cruisers;
    let empire_1_fleet_1_cruisers = game_data.fleets[0].ships.cruisers;
    let joiner = &mut game_data.fleets[5];
    assert_eq!((joiner.owner.0, joiner.fleet_number.0), (2, 2));
    let joiner_cruisers = joiner.ships.cruisers;
    joiner.order = Order::JoinAnotherFleet;
    set_mission_target(joiner, 1, 1);
    joiner.speed = joiner.max_speed;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert!(events.fleet_merge_events.iter().any(|event| {
        event.kind == Mission::JoinAnotherFleet
            && event.owner_empire == 2
            && event.host_fleet_number == 1
            && event.absorbed_fleet_number == 2
    }));
    let host = game_data
        .fleets
        .iter()
        .find(|fleet| fleet.owner.0 == 2 && fleet.fleet_number.0 == 1)
        .expect("empire 2 fleet 1 survives");
    assert_eq!(host.ships.cruisers, host_cruisers + joiner_cruisers);
    assert!(
        !game_data
            .fleets
            .iter()
            .any(|fleet| fleet.owner.0 == 2 && fleet.fleet_number.0 == 2)
    );
    // Empire 1's fleet 1 (record id 1) is untouched.
    assert_eq!(
        game_data.fleets[0].ships.cruisers,
        empire_1_fleet_1_cruisers
    );
}
