//! Regression tests for fleet maintenance mechanics (Milestone 4 Phase 2)
//!
//! Validates that the Rust maintenance implementation matches the original ECMAINT
//! behavior on the fleet-scenario fixture pair.

mod common;

use nc_data::{
    BaseDat, BaseRecord, ColonizationResolvedEvent, CoreGameData, DiplomacyOverride,
    DiplomaticRelation, FleetOrderValidationError, GameStateBuilder, JoinMissionHostEvent, Mission,
    MissionAbortReason, MissionOutcome, MissionRetargetEvent, Order, PlanetIntelSource, Real48,
    SalvageFailureReason, SalvageResolvedEvent,
};
use nc_engine::run_maintenance_turn;
use std::path::Path;

/// Helper to load a fixture directory.
fn load_fixture(name: &str) -> CoreGameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
        .join("v1.5");
    CoreGameData::load(&dir).unwrap_or_else(|e| panic!("Failed to load fixture {}: {}", name, e))
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

    let target_world = &mut game_data.planets.records[target_idx];
    target_world.set_coords_raw(target_coords);
    target_world.set_owner_empire_slot_raw(2);
    target_world.set_ownership_status_raw(2);
    target_world.set_planet_name("Target");
    target_world.set_army_count_raw(10);
    target_world.set_ground_batteries_raw(4);

    let fleet = &mut game_data.fleets.records[0];
    fleet.set_battleship_count(ships.0);
    fleet.set_cruiser_count(ships.1);
    fleet.set_destroyer_count(ships.2);
    fleet.set_troop_transport_count(ships.3);
    fleet.set_army_count(ships.4);
    fleet.set_etac_count(ships.5);
    fleet.set_scout_count(ships.6 as u8);
    fleet.recompute_max_speed_from_composition();
    fleet.set_current_location_coords_raw([24, 25]);
    fleet.set_current_speed(3);
    fleet.set_standing_order_kind(order);
    fleet.set_standing_order_target_coords_raw(target_coords);
    fleet.set_mission_aux_bytes([1, 0]);
    fleet.set_at_rest_motion();

    (game_data, target_idx, target_coords)
}

fn mutual_enemy_overrides(left: u8, right: u8) -> [DiplomacyOverride; 2] {
    [
        DiplomacyOverride {
            from_empire_raw: left,
            to_empire_raw: right,
            relation: DiplomaticRelation::Enemy,
        },
        DiplomacyOverride {
            from_empire_raw: right,
            to_empire_raw: left,
            relation: DiplomaticRelation::Enemy,
        },
    ]
}

fn add_active_starbase(game_data: &mut CoreGameData, owner: u8, coords: [u8; 2]) {
    let mut base = BaseRecord::new_zeroed();
    base.set_local_slot_raw(1);
    base.set_active_flag_raw(1);
    base.set_base_id_raw(1);
    base.set_link_word_raw(0);
    base.set_chain_word_raw(1);
    base.set_coords_raw(coords);
    base.set_trailing_coords_raw(coords);
    base.set_owner_empire_raw(owner);
    game_data.bases.records.push(base);
}

#[test]
fn test_fleet_movement_and_colonization_fleets_dat() {
    // After one maintenance turn on the fleet pre-fixture, FLEETS.DAT should match
    // the post-fixture byte-for-byte.
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let post_data = load_fixture("ecmaint-fleet-post");

    run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert_eq!(
        game_data.fleets.to_bytes(),
        post_data.fleets.to_bytes(),
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
        game_data.planets.to_bytes(),
        post_data.planets.to_bytes(),
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
        game_data.player.to_bytes(),
        post_data.player.to_bytes(),
        "PLAYER.DAT should match post-fixture after 1 turn"
    );
}

#[test]
fn test_fleet_movement_arrival_state() {
    // Specific validation: fleet 0 should arrive at (15,13) with cleared speed and order.
    let mut game_data = load_fixture("ecmaint-fleet-pre");

    // Pre-state: fleet 0 at (16,13) with ColonizeWorld order, speed=3
    assert_eq!(
        game_data.fleets.records[0].current_location_coords_raw(),
        [16, 13],
        "Fleet 0 should start at (16,13)"
    );
    assert_eq!(
        game_data.fleets.records[0].current_speed(),
        3,
        "Fleet 0 should start with speed 3"
    );

    run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    // Post-state: fleet 0 should be at (15,13) with speed=0 and HoldPosition order
    assert_eq!(
        game_data.fleets.records[0].current_location_coords_raw(),
        [15, 13],
        "Fleet 0 should arrive at (15,13)"
    );
    assert_eq!(
        game_data.fleets.records[0].current_speed(),
        0,
        "Fleet 0 should have speed 0 after arrival"
    );
    assert_eq!(
        game_data.fleets.records[0].standing_order_code_raw(),
        0,
        "Fleet 0 order should be cleared to HoldPosition (0) after arrival"
    );
    assert_eq!(
        game_data.fleets.records[0].standing_order_kind(),
        Order::HoldPosition,
        "Fleet 0 should be HoldPosition after colonization arrival"
    );
}

#[test]
fn test_colonization_planet_state() {
    // Planet at (15,13) should be colonized by empire 1 after fleet 0 arrives.
    let mut game_data = load_fixture("ecmaint-fleet-pre");

    // Pre-state: planet 13 at (15,13) should be unowned
    let planet_13_pre = &game_data.planets.records[13];
    assert_eq!(
        planet_13_pre.coords_raw(),
        [15, 13],
        "Planet 13 should be at (15,13)"
    );
    assert_eq!(
        planet_13_pre.owner_empire_slot_raw(),
        0,
        "Planet 13 should be unowned pre-maint"
    );

    run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    // Post-state: planet 13 should be owned by empire 1
    let planet_13_post = &game_data.planets.records[13];
    assert_eq!(
        planet_13_post.owner_empire_slot_raw(),
        1,
        "Planet 13 should be owned by empire 1 after colonization"
    );
    assert_eq!(
        planet_13_post.ownership_status_raw(),
        2,
        "Planet 13 ownership_status should be 2 after colonization"
    );
    assert_eq!(
        planet_13_post.army_count_raw(),
        1,
        "Planet 13 should have 1 colonist army"
    );
    assert_eq!(
        planet_13_post.planet_name(),
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
            colonizer_empire_raw: 1,
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
        game_data.fleets.records[0].standing_order_kind(),
        Order::HoldPosition,
        "Fleet should be HoldPosition after colonization"
    );
    assert_eq!(
        game_data.fleets.records[0].current_speed(),
        0,
        "Fleet should have speed 0 after colonization"
    );
}

#[test]
fn test_colonization_emits_blocked_event_for_occupied_world() {
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let target = &mut game_data.planets.records[13];
    target.set_owner_empire_slot_raw(2);
    target.set_ownership_status_raw(2);
    target.set_planet_name("TargetPrime");
    target.set_army_count_raw(10);

    let events = run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert_eq!(events.colonization_events.len(), 1);
    assert!(matches!(
        events.colonization_events[0],
        ColonizationResolvedEvent::BlockedByOwner {
            fleet_idx: 0,
            planet_idx: 13,
            colonizer_empire_raw: 1,
            owner_empire_raw: 2,
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
            && event.viewer_empire_raw == 1
            && event.source == PlanetIntelSource::ColonizeBlockedByOwner
            && event.source_fleet_idx == Some(0)
    }));
    let target = &game_data.planets.records[13];
    assert_eq!(target.owner_empire_slot_raw(), 2);
    assert_eq!(target.planet_name(), "TargetPrime");
}

#[test]
fn test_scout_sector_arrival_emits_success_event() {
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let scout = &mut game_data.fleets.records[0];
    scout.set_standing_order_kind(Order::ScoutSector);
    scout.set_standing_order_target_coords_raw([15, 13]);
    scout.set_scout_count(1);
    // ECGAME writes the 0x22 world-target flag as 0 for sector orders.
    scout.set_mission_aux_bytes([0, 0]);
    scout.set_etac_count(0);

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
    assert_eq!(
        game_data.fleets.records[0].standing_order_kind(),
        Order::ScoutSector
    );
    assert_eq!(game_data.fleets.records[0].current_speed(), 0);
}

#[test]
fn test_blockading_foreign_world_escalates_to_enemy() {
    let mut game_data = load_fixture("ecmaint-post");
    let (planet_idx, coords) = game_data
        .planets
        .records
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner_empire_slot_raw() == 2)
        .map(|(idx, planet)| (idx, planet.coords_raw()))
        .expect("fixture should contain an empire 2 world");

    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw([coords[0].saturating_add(1), coords[1]]);
    fleet.set_standing_order_kind(Order::GuardBlockadeWorld);
    fleet.set_standing_order_target_coords_raw(coords);
    fleet.set_current_speed(3);
    fleet.raw[0x19] = 0x00;
    // The world's own fleets would attack the blockader in their system.
    common::hold_fire(&mut game_data, 2);

    let events = run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert!(
        events
            .diplomatic_escalation_events
            .iter()
            .any(|event| event.left_empire_raw == 1 && event.right_empire_raw == 2)
    );
    assert_eq!(
        game_data.player.records[0].diplomatic_relation_toward(2),
        Some(DiplomaticRelation::Enemy)
    );
    assert_eq!(
        game_data.player.records[1].diplomatic_relation_toward(1),
        Some(DiplomaticRelation::Enemy)
    );
    assert!(events.mission_events.iter().any(|event| {
        event.kind == Mission::GuardBlockadeWorld
            && event.owner_empire_raw == 1
            && event.planet_idx == Some(planet_idx)
    }));
}

#[test]
fn test_scout_system_arrival_emits_success_event() {
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let scout = &mut game_data.fleets.records[0];
    scout.set_standing_order_kind(Order::ScoutSolarSystem);
    scout.set_standing_order_target_coords_raw([15, 13]);
    scout.set_scout_count(1);
    scout.set_etac_count(0);
    // Cruiser speed settles in orbit one sector away within the year.
    scout.recompute_max_speed_from_composition();
    scout.set_current_speed(scout.max_speed());

    let events = run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert!(events.colonization_events.is_empty());
    assert_eq!(events.planet_intel_events.len(), 1);
    assert_eq!(events.planet_intel_events[0].planet_idx, 13);
    assert_eq!(events.planet_intel_events[0].viewer_empire_raw, 1);
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
    assert_eq!(
        game_data.fleets.records[0].current_location_coords_raw(),
        [15, 13]
    );
    // Scout persists on station after arrival.
    assert_eq!(
        game_data.fleets.records[0].standing_order_kind(),
        Order::ScoutSolarSystem
    );
}

#[test]
fn test_view_world_arrival_emits_success_and_intel_event() {
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let viewer = &mut game_data.fleets.records[0];
    viewer.set_standing_order_kind(Order::ViewWorld);
    viewer.set_standing_order_target_coords_raw([15, 13]);
    viewer.set_scout_count(0);
    viewer.set_etac_count(0);
    // Cruiser speed settles in orbit one sector away within the year.
    viewer.recompute_max_speed_from_composition();
    viewer.set_current_speed(viewer.max_speed());

    let events = run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert!(events.colonization_events.is_empty());
    assert_eq!(events.planet_intel_events.len(), 1);
    assert_eq!(events.planet_intel_events[0].planet_idx, 13);
    assert_eq!(events.planet_intel_events[0].viewer_empire_raw, 1);
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
    assert_eq!(
        game_data.fleets.records[0].standing_order_kind(),
        Order::HoldPosition
    );
    assert_eq!(game_data.fleets.records[0].current_speed(), 0);
    assert_eq!(
        game_data.fleets.records[0].tuple_c_payload_raw(),
        [0, 0, 0, 0, 0],
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
    game_data.planets.records[target_idx].set_owner_empire_slot_raw(2);
    game_data.planets.records[target_idx].set_ownership_status_raw(2);

    let enemy = &mut game_data.fleets.records[1];
    enemy.set_owner_empire_raw(2);
    enemy.set_fleet_id_word_raw(9);
    enemy.set_current_location_coords_raw(target_coords);
    enemy.set_standing_order_kind(Order::HoldPosition);
    enemy.set_standing_order_target_coords_raw(target_coords);
    enemy.set_current_speed(0);
    enemy.set_battleship_count(8);
    enemy.set_cruiser_count(8);
    enemy.set_destroyer_count(8);
    enemy.set_scout_count(0);
    enemy.set_troop_transport_count(0);
    enemy.set_army_count(0);
    enemy.set_etac_count(0);
    enemy.set_rules_of_engagement(10);

    // A year where the enemy caught the viewer in the system.
    let (_, events) = common::first_year(&game_data, &mutual_enemy_overrides(1, 2), |_, events| {
        events
            .fleet_battle_events
            .iter()
            .any(|event| event.reporting_empire_raw == 1)
    });

    assert!(
        !events.planet_intel_events.iter().any(|event| {
            event.viewer_empire_raw == 1 && event.source == PlanetIntelSource::ViewWorld
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
    game_data.fleets.records[0].set_current_speed(6);
    game_data.planets.records[target_idx].set_owner_empire_slot_raw(2);
    game_data.planets.records[target_idx].set_ownership_status_raw(2);

    let enemy = &mut game_data.fleets.records[1];
    enemy.set_owner_empire_raw(2);
    enemy.set_fleet_id_word_raw(9);
    enemy.set_current_location_coords_raw(target_coords);
    enemy.set_standing_order_kind(Order::HoldPosition);
    enemy.set_standing_order_target_coords_raw(target_coords);
    enemy.set_current_speed(0);
    enemy.set_battleship_count(8);
    enemy.set_cruiser_count(8);
    enemy.set_destroyer_count(8);
    enemy.set_scout_count(0);
    enemy.set_troop_transport_count(0);
    enemy.set_army_count(0);
    enemy.set_etac_count(0);
    enemy.set_rules_of_engagement(10);

    // A year where the scout reached orbit before the enemy caught it.
    let (_, events) = common::first_year(&game_data, &mutual_enemy_overrides(1, 2), |_, events| {
        events
            .fleet_destroyed_events
            .iter()
            .all(|event| event.reporting_empire_raw != 1)
            && events
                .mission_events
                .iter()
                .any(|event| event.fleet_idx == 0 && event.kind == Mission::ScoutSolarSystem)
    });

    assert!(
        events.planet_intel_events.iter().any(|event| {
            event.viewer_empire_raw == 1 && event.source == PlanetIntelSource::ScoutSolarSystem
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
            .all(|event| event.reporting_empire_raw != 1),
        "the arriving scout should not be destroyed on the same tick"
    );
}

#[test]
fn test_seek_home_arrival_emits_success_event() {
    let mut game_data = load_fixture("ecmaint-post");
    let target_coords = game_data
        .planets
        .records
        .iter()
        .find(|planet| planet.owner_empire_slot_raw() == 1)
        .map(|planet| planet.coords_raw())
        .expect("fixture should contain an owned world");
    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw([target_coords[0].saturating_add(1), target_coords[1]]);
    fleet.set_standing_order_kind(Order::SeekHome);
    fleet.set_standing_order_target_coords_raw(target_coords);
    fleet.set_current_speed(3);
    fleet.raw[0x0d] = 0x80;
    fleet.raw[0x0f] = 0;
    fleet.raw[0x19] = 0x00;

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
    let fleet = &mut game_data.fleets.records[0];
    fleet.set_standing_order_kind(Order::RendezvousSector);
    fleet.set_standing_order_target_coords_raw([15, 13]);
    // ECGAME writes the 0x22 world-target flag as 0 for sector orders.
    fleet.set_mission_aux_bytes([0, 0]);

    let events = run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert_eq!(
        game_data.fleets.records[0].standing_order_kind(),
        Order::RendezvousSector
    );
    assert_eq!(
        game_data.fleets.records[0].standing_order_target_coords_raw(),
        [15, 13]
    );

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
    game_data.player.records[0].raw[0x00] = 0xff;
    let coords = game_data.fleets.records[0].current_location_coords_raw();
    game_data.fleets.records[1].set_current_location_coords_raw(coords);
    game_data.fleets.records[1].set_standing_order_kind(Order::JoinAnotherFleet);

    let events = run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert!(events.fleet_merge_events.iter().any(|event| {
        event.fleet_idx == 1
            && event.kind == Mission::JoinAnotherFleet
            && event.owner_empire_raw == 1
            && !event.survivor_side
    }));
}

#[test]
fn test_merges_that_would_overflow_scout_count_are_refused() {
    // Scout counts are stored in one byte; merging two 200-scout fleets must
    // not silently drop 145 scouts. Covers mission joins, rendezvous, and
    // rogue co-location consolidation.
    for (consolidation_flag, order) in [
        (0x00, Order::JoinAnotherFleet),
        (0x00, Order::RendezvousSector),
        (0xff, Order::HoldPosition),
    ] {
        let mut game_data = load_fixture("ecmaint-post");
        game_data.player.records[0].raw[0x00] = consolidation_flag;
        let coords = game_data.fleets.records[0].current_location_coords_raw();
        let host_id = game_data.fleets.records[0].fleet_id();
        for idx in [0, 1] {
            let fleet = &mut game_data.fleets.records[idx];
            fleet.set_current_location_coords_raw(coords);
            fleet.set_standing_order_kind(order);
            fleet.set_standing_order_target_coords_raw(coords);
            fleet.set_current_speed(0);
            fleet.set_scout_count(200);
        }
        if order == Order::JoinAnotherFleet {
            game_data.fleets.records[0].set_standing_order_kind(Order::HoldPosition);
            game_data.fleets.records[1].set_join_host_fleet_id_raw(host_id);
        }
        let empire_scouts = |game_data: &CoreGameData| -> u32 {
            game_data
                .fleets
                .records
                .iter()
                .filter(|fleet| fleet.owner_empire_raw() == 1)
                .map(|fleet| u32::from(fleet.scout_count()))
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
    game_data.player.records[0].raw[0x00] = 0x00;

    let host = &mut game_data.fleets.records[0];
    host.set_current_location_coords_raw([10, 10]);
    host.set_standing_order_kind(Order::MoveOnly);
    host.set_standing_order_target_coords_raw([14, 10]);
    host.set_current_speed(3);
    host.raw[0x0d] = 0x80;
    host.raw[0x0f] = 0;
    host.raw[0x19] = 0x80;

    let host_id = game_data.fleets.records[0].fleet_id();
    let joiner = &mut game_data.fleets.records[1];
    joiner.set_current_location_coords_raw([4, 10]);
    joiner.set_standing_order_kind(Order::JoinAnotherFleet);
    joiner.set_join_host_fleet_id_raw(host_id);
    joiner.set_standing_order_target_coords_raw([10, 10]);
    joiner.set_current_speed(3);
    joiner.raw[0x0d] = 0x80;
    joiner.raw[0x0f] = 0;
    joiner.raw[0x19] = 0x80;

    run_maintenance_turn(&mut game_data).expect("first maintenance turn should succeed");

    let host_after_first = game_data.fleets.records[0].current_location_coords_raw();
    assert_eq!(host_after_first, [12, 10]);
    assert_eq!(
        game_data.fleets.records[1].standing_order_target_coords_raw(),
        [10, 10],
        "joiner should chase the host's turn-start position during the first turn"
    );

    run_maintenance_turn(&mut game_data).expect("second maintenance turn should succeed");

    assert_eq!(
        game_data.fleets.records[1].standing_order_target_coords_raw(),
        host_after_first,
        "joiner should refresh to the host's latest location on the following turn"
    );
    assert_eq!(
        game_data.fleets.records[1].join_host_fleet_id_raw(),
        host_id,
        "host identity should remain unchanged while the host survives"
    );
    assert_eq!(
        game_data.fleets.records[1].standing_order_kind(),
        Order::JoinAnotherFleet
    );
}

#[test]
fn test_join_order_survives_arrival_at_previous_host_position() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.player.records[0].raw[0x00] = 0x00;

    let host = &mut game_data.fleets.records[0];
    host.set_current_location_coords_raw([10, 10]);
    host.set_standing_order_kind(Order::MoveOnly);
    host.set_standing_order_target_coords_raw([14, 10]);
    host.set_current_speed(3);
    host.raw[0x0d] = 0x80;
    host.raw[0x0f] = 0;
    host.raw[0x19] = 0x80;

    let host_id = game_data.fleets.records[0].fleet_id();
    let joiner = &mut game_data.fleets.records[1];
    joiner.set_current_location_coords_raw([9, 10]);
    joiner.set_standing_order_kind(Order::JoinAnotherFleet);
    joiner.set_join_host_fleet_id_raw(host_id);
    joiner.set_standing_order_target_coords_raw([10, 10]);
    joiner.set_current_speed(3);
    joiner.raw[0x0d] = 0x80;
    joiner.raw[0x0f] = 0;
    joiner.raw[0x19] = 0x80;

    run_maintenance_turn(&mut game_data).expect("first maintenance turn should succeed");

    let host_after_first = game_data.fleets.records[0].current_location_coords_raw();
    assert_eq!(host_after_first, [12, 10]);
    assert_eq!(
        game_data.fleets.records[1].current_location_coords_raw(),
        [10, 10]
    );
    assert_eq!(
        game_data.fleets.records[1].standing_order_kind(),
        Order::JoinAnotherFleet
    );
    assert_eq!(
        game_data.fleets.records[1].current_speed(),
        3,
        "joiner should keep chasing speed after arriving at the host's previous position"
    );

    run_maintenance_turn(&mut game_data).expect("second maintenance turn should succeed");

    assert_eq!(
        game_data.fleets.records[1].standing_order_kind(),
        Order::JoinAnotherFleet
    );
    assert_eq!(
        game_data.fleets.records[1].standing_order_target_coords_raw(),
        host_after_first
    );
    assert_eq!(
        game_data.fleets.records[1].current_speed(),
        3,
        "joiner speed should remain armed for continued pursuit on the following turn"
    );
}

#[test]
fn test_join_order_abandons_mission_when_host_is_destroyed() {
    let mut game_data = load_fixture("ecmaint-post");

    let host_id = game_data.fleets.records[0].fleet_id();
    game_data.fleets.records[0].set_destroyer_count(0);
    game_data.fleets.records[0].set_cruiser_count(0);
    game_data.fleets.records[0].set_battleship_count(0);
    game_data.fleets.records[0].set_scout_count(0);
    game_data.fleets.records[0].set_troop_transport_count(0);
    game_data.fleets.records[0].set_etac_count(0);

    let joiner_coords = [7, 9];
    let joiner = &mut game_data.fleets.records[1];
    joiner.set_current_location_coords_raw(joiner_coords);
    joiner.set_standing_order_kind(Order::JoinAnotherFleet);
    joiner.set_join_host_fleet_id_raw(host_id);
    joiner.set_standing_order_target_coords_raw([10, 10]);
    joiner.set_current_speed(3);

    let fleet_count_before = game_data.fleets.records.len();
    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    // The empty host is culled at turn start; lost-host join missions now
    // resolve immediately instead of burning a movement step first.
    assert_eq!(game_data.fleets.records.len(), fleet_count_before - 1);
    assert_eq!(
        game_data.fleets.records[0].standing_order_kind(),
        Order::HoldPosition
    );
    assert_eq!(game_data.fleets.records[0].current_speed(), 0);
    let abandoned_coords = game_data.fleets.records[0].current_location_coords_raw();
    assert_eq!(abandoned_coords, joiner_coords);
    assert_eq!(
        game_data.fleets.records[0].standing_order_target_coords_raw(),
        abandoned_coords
    );
    assert_eq!(game_data.fleets.records[0].join_host_fleet_id_raw(), 0);
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
    game_data.player.records[0].raw[0x00] = 0x00;

    let survivor_coords = [10, 10];
    let survivor_id = game_data.fleets.records[0].fleet_id();
    let absorbed_host_id = game_data.fleets.records[1].fleet_id();

    {
        let survivor = &mut game_data.fleets.records[0];
        survivor.set_current_location_coords_raw(survivor_coords);
        survivor.set_standing_order_kind(Order::HoldPosition);
        survivor.set_standing_order_target_coords_raw(survivor_coords);
        survivor.set_current_speed(0);
    }
    {
        let host = &mut game_data.fleets.records[1];
        host.set_current_location_coords_raw(survivor_coords);
        host.set_standing_order_kind(Order::JoinAnotherFleet);
        host.set_join_host_fleet_id_raw(survivor_id);
        host.set_standing_order_target_coords_raw(survivor_coords);
        host.set_current_speed(0);
    }
    {
        let joiner = &mut game_data.fleets.records[2];
        joiner.set_current_location_coords_raw([5, 10]);
        joiner.set_standing_order_kind(Order::JoinAnotherFleet);
        joiner.set_join_host_fleet_id_raw(absorbed_host_id);
        joiner.set_standing_order_target_coords_raw(survivor_coords);
        joiner.set_current_speed(3);
    }

    let fleet_count_before = game_data.fleets.records.len();
    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(game_data.fleets.records.len(), fleet_count_before - 1);
    let joiner = &game_data.fleets.records[1];
    assert_eq!(joiner.standing_order_kind(), Order::JoinAnotherFleet);
    assert_eq!(joiner.join_host_fleet_id_raw(), survivor_id);
    assert_eq!(joiner.standing_order_target_coords_raw(), survivor_coords);
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
    game_data.player.records[0].raw[0x00] = 0x00;

    let coords = [10, 10];
    let survivor_id = game_data.fleets.records[0].fleet_id();
    let absorbed_host_id = game_data.fleets.records[1].fleet_id();

    {
        let survivor = &mut game_data.fleets.records[0];
        survivor.set_current_location_coords_raw(coords);
        survivor.set_standing_order_kind(Order::HoldPosition);
        survivor.set_standing_order_target_coords_raw(coords);
        survivor.set_current_speed(0);
    }
    {
        let host = &mut game_data.fleets.records[1];
        host.set_current_location_coords_raw(coords);
        host.set_standing_order_kind(Order::JoinAnotherFleet);
        host.set_join_host_fleet_id_raw(survivor_id);
        host.set_standing_order_target_coords_raw(coords);
        host.set_current_speed(0);
    }
    {
        let joiner = &mut game_data.fleets.records[2];
        joiner.set_current_location_coords_raw(coords);
        joiner.set_standing_order_kind(Order::JoinAnotherFleet);
        joiner.set_join_host_fleet_id_raw(absorbed_host_id);
        joiner.set_standing_order_target_coords_raw(coords);
        joiner.set_current_speed(0);
    }

    let fleet_count_before = game_data.fleets.records.len();
    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(game_data.fleets.records.len(), fleet_count_before - 2);
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
    let original_target = game_data.planets.records[0].coords_raw();
    game_data.planets.records[0].set_owner_empire_slot_raw(2);
    let fallback_target = game_data
        .planets
        .records
        .iter()
        .find(|planet| planet.owner_empire_slot_raw() == 1)
        .map(|planet| planet.coords_raw())
        .expect("fixture should still have another owned planet");

    let fleet = &mut game_data.fleets.records[1];
    fleet.set_current_location_coords_raw([9, 9]);
    fleet.set_standing_order_kind(Order::SeekHome);
    fleet.set_standing_order_target_coords_raw(original_target);
    fleet.set_current_speed(3);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(
        game_data.fleets.records[1].standing_order_target_coords_raw(),
        fallback_target
    );
    assert_eq!(game_data.fleets.records[1].current_speed(), 3);
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
                && *reporting_fleet_number == Some(game_data.fleets.records[1].local_slot_word_raw() as u8)
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
    let mut base = BaseRecord::new_zeroed();
    base.set_base_id_raw(3);
    base.set_owner_empire_raw(1);
    base.set_coords_raw([12, 8]);
    game_data.bases = BaseDat {
        records: vec![base],
    };

    let fleet = &mut game_data.fleets.records[1];
    fleet.set_current_location_coords_raw([4, 8]);
    fleet.set_standing_order_kind(Order::GuardStarbase);
    fleet.set_standing_order_target_coords_raw([9, 8]);
    fleet.set_current_speed(3);
    fleet.set_mission_aux_bytes([1, 3]);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(
        game_data.fleets.records[1].standing_order_target_coords_raw(),
        [12, 8]
    );
    assert_eq!(game_data.fleets.records[1].current_speed(), 3);
    // ECMAINT keeps the 0x22 world-target flag on guard-starbase fleets.
    assert_eq!(game_data.fleets.records[1].mission_aux_bytes(), [1, 3]);
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
                && *reporting_fleet_number == Some(game_data.fleets.records[1].local_slot_word_raw() as u8)
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
    game_data.bases = BaseDat { records: vec![] };

    let fleet = &mut game_data.fleets.records[1];
    fleet.set_current_location_coords_raw([4, 8]);
    fleet.set_standing_order_kind(Order::GuardStarbase);
    fleet.set_standing_order_target_coords_raw([9, 8]);
    fleet.set_current_speed(3);
    fleet.set_mission_aux_bytes([1, 3]);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(
        game_data.fleets.records[1].standing_order_kind(),
        Order::HoldPosition
    );
    assert_eq!(game_data.fleets.records[1].current_speed(), 0);
    assert_eq!(
        game_data.fleets.records[1].standing_order_target_coords_raw(),
        game_data.fleets.records[1].current_location_coords_raw()
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
                && *reporting_fleet_number == Some(game_data.fleets.records[1].local_slot_word_raw() as u8)
                && *mission == Mission::GuardStarbase
                && *previous_target_coords == [9, 8]
        )
    }));
}

#[test]
fn test_guard_starbase_persists_after_arrival() {
    let mut game_data = load_fixture("ecmaint-post");
    let mut base = BaseRecord::new_zeroed();
    base.set_active_flag_raw(1);
    base.set_base_id_raw(3);
    base.set_owner_empire_raw(1);
    base.set_coords_raw([11, 8]);
    game_data.bases = BaseDat {
        records: vec![base],
    };

    let fleet = &mut game_data.fleets.records[1];
    fleet.set_current_location_coords_raw([10, 8]);
    fleet.set_standing_order_kind(Order::GuardStarbase);
    fleet.set_standing_order_target_coords_raw([11, 8]);
    fleet.set_current_speed(3);
    fleet.set_mission_aux_bytes([1, 3]);
    fleet.raw[0x0d] = 0x80;
    fleet.raw[0x0f] = 0;
    fleet.raw[0x19] = 0x80;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(
        game_data.fleets.records[1].current_location_coords_raw(),
        [11, 8]
    );
    assert_eq!(
        game_data.fleets.records[1].standing_order_kind(),
        Order::GuardStarbase
    );
    assert_eq!(game_data.fleets.records[1].current_speed(), 0);
    assert_eq!(
        game_data.fleets.records[1].standing_order_target_coords_raw(),
        [11, 8]
    );
    assert_eq!(game_data.fleets.records[1].mission_aux_bytes(), [1, 3]);
    // Settled in orbit at the sector centre, as ECMAINT's guard action leaves it.
    let fleet = &game_data.fleets.records[1];
    assert_eq!(fleet.x_fraction(), Real48::HALF);
    assert_eq!(fleet.y_fraction(), Real48::HALF);
    assert_eq!(fleet.depth().to_bytes(), [0x81, 0, 0, 0, 0, 0]);
    assert!(events.mission_events.iter().any(|event| {
        event.fleet_idx == 1
            && event.kind == Mission::GuardStarbase
            && event.outcome == MissionOutcome::Arrived
            && event.location_coords == Some([11, 8])
    }));
}

#[test]
fn test_guard_starbase_with_zero_index_stays_armed_when_target_base_exists() {
    let mut game_data = load_fixture("ecmaint-post");
    let mut base = BaseRecord::new_zeroed();
    base.set_active_flag_raw(1);
    base.set_base_id_raw(3);
    base.set_owner_empire_raw(1);
    base.set_coords_raw([11, 8]);
    game_data.bases = BaseDat {
        records: vec![base],
    };

    let fleet = &mut game_data.fleets.records[1];
    fleet.set_current_location_coords_raw([11, 8]);
    fleet.set_standing_order_kind(Order::GuardStarbase);
    fleet.set_standing_order_target_coords_raw([11, 8]);
    fleet.set_current_speed(0);
    fleet.set_mission_aux_bytes([0, 0]);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(
        game_data.fleets.records[1].standing_order_kind(),
        Order::GuardStarbase
    );
    assert_eq!(game_data.fleets.records[1].current_speed(), 0);
    assert_eq!(game_data.fleets.records[1].mission_aux_bytes(), [0, 0]);
    assert!(!events.mission_retarget_events.iter().any(|event| {
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
    game_data.bases = BaseDat { records: vec![] };

    let fleet = &mut game_data.fleets.records[1];
    fleet.set_current_location_coords_raw([11, 8]);
    fleet.set_standing_order_kind(Order::GuardStarbase);
    fleet.set_standing_order_target_coords_raw([11, 8]);
    fleet.set_current_speed(0);
    fleet.set_mission_aux_bytes([0, 0]);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(
        game_data.fleets.records[1].standing_order_kind(),
        Order::HoldPosition
    );
    assert_eq!(game_data.fleets.records[1].current_speed(), 0);
    assert_eq!(game_data.fleets.records[1].mission_aux_bytes(), [0, 0]);
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
                && *reporting_fleet_number == Some(game_data.fleets.records[1].local_slot_word_raw() as u8)
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

    let target_world = &mut game_data.planets.records[4];
    target_world.set_coords_raw(target_coords);
    target_world.set_owner_empire_slot_raw(2);
    target_world.set_ownership_status_raw(2);
    target_world.set_planet_name("Target");

    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw([10, 8]);
    fleet.set_standing_order_kind(Order::GuardBlockadeWorld);
    fleet.set_standing_order_target_coords_raw(target_coords);
    fleet.set_current_speed(3);
    fleet.raw[0x0d] = 0x80;
    fleet.raw[0x0f] = 0;
    fleet.raw[0x19] = 0x80;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(
        game_data.fleets.records[0].current_location_coords_raw(),
        target_coords
    );
    assert_eq!(
        game_data.fleets.records[0].standing_order_kind(),
        Order::GuardBlockadeWorld
    );
    assert_eq!(game_data.fleets.records[0].current_speed(), 0);
    assert_eq!(
        game_data.fleets.records[0].standing_order_target_coords_raw(),
        target_coords
    );
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
    let fleet = &mut game_data.fleets.records[1];
    fleet.set_current_location_coords_raw([10, 10]);
    fleet.set_standing_order_kind(Order::PatrolSector);
    fleet.set_standing_order_target_coords_raw([11, 10]);
    fleet.set_current_speed(3);
    fleet.raw[0x0d] = 0x80;
    fleet.raw[0x0f] = 0;
    fleet.raw[0x19] = 0x80;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(
        game_data.fleets.records[1].current_location_coords_raw(),
        [11, 10]
    );
    assert_eq!(
        game_data.fleets.records[1].standing_order_kind(),
        Order::PatrolSector
    );
    assert_eq!(game_data.fleets.records[1].current_speed(), 0);
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

    let fleet = &mut game_data.fleets.records[0];
    fleet.set_battleship_count(0);
    fleet.set_cruiser_count(0);
    fleet.set_destroyer_count(0);
    fleet.set_troop_transport_count(0);
    fleet.set_etac_count(0);
    fleet.set_scout_count(1);
    fleet.recompute_max_speed_from_composition();
    fleet.set_current_location_coords_raw([10, 10]);
    fleet.set_current_speed(3);
    fleet.set_standing_order_kind(Order::MoveOnly);
    fleet.set_standing_order_target_coords_raw([16, 16]);
    fleet.set_mission_aux_bytes([0, 0]);
    fleet.set_at_rest_motion();

    run_maintenance_turn(&mut game_data).expect("turn 1 should succeed");
    run_maintenance_turn(&mut game_data).expect("turn 2 should succeed");
    let turn3_events = run_maintenance_turn(&mut game_data).expect("turn 3 should succeed");

    let fleet = &game_data.fleets.records[0];
    assert_eq!(fleet.current_location_coords_raw(), [16, 16]);
    assert_eq!(fleet.standing_order_kind(), Order::HoldPosition);
    assert_eq!(fleet.current_speed(), 0);
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

    let fleet = &mut game_data.fleets.records[0];
    fleet.set_battleship_count(0);
    fleet.set_cruiser_count(0);
    fleet.set_destroyer_count(0);
    fleet.set_troop_transport_count(0);
    fleet.set_etac_count(0);
    fleet.set_scout_count(1);
    fleet.recompute_max_speed_from_composition();
    fleet.set_current_location_coords_raw([10, 10]);
    fleet.set_mission_aux_bytes([1, 0]);
    game_data
        .set_fleet_order(1, 3, Order::MoveOnly.to_raw(), [16, 10], None, None)
        .expect("move-only order should apply");
    game_data.fleets.records[0].raw[0x19] = 0x81;

    run_maintenance_turn(&mut game_data).expect("turn 1 should succeed");
    run_maintenance_turn(&mut game_data).expect("turn 2 should succeed");
    let turn3_events = run_maintenance_turn(&mut game_data).expect("turn 3 should succeed");

    let fleet = &game_data.fleets.records[0];
    assert_eq!(fleet.current_location_coords_raw(), [16, 10]);
    assert_eq!(fleet.standing_order_kind(), Order::HoldPosition);
    assert_eq!(fleet.current_speed(), 0);
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
        let fleet = &game_data.fleets.records[0];

        assert_eq!(
            fleet.current_location_coords_raw(),
            target_coords,
            "{name} coords"
        );
        assert_eq!(fleet.standing_order_kind(), order, "{name} order");
        // Speed 3 from rest one sector out ends the year just short of
        // orbit (depth 1 - epsilon), still under way; it strikes next year.
        assert_eq!(fleet.current_speed(), 3, "{name} arrival speed");
        assert_eq!(
            fleet.standing_order_target_coords_raw(),
            target_coords,
            "{name} target"
        );
        assert_eq!(fleet.raw[0x19], 0x80, "{name} raw[0x19]");
        assert_eq!(fleet.raw[0x1a], 0xb9, "{name} raw[0x1a]");
        assert_eq!(fleet.raw[0x1b], 0xff, "{name} raw[0x1b]");
        assert_eq!(fleet.raw[0x1c], 0xff, "{name} raw[0x1c]");
        assert_eq!(fleet.raw[0x1d], 0xff, "{name} raw[0x1d]");
        assert_eq!(fleet.raw[0x1e], 0x7f, "{name} raw[0x1e]");
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
    game_data.player.records[0].raw[0x00] = 0x00;
    let host_coords = game_data.fleets.records[0].current_location_coords_raw();
    let host_id = game_data.fleets.records[0].fleet_id();

    let joiner = &mut game_data.fleets.records[1];
    joiner.set_current_location_coords_raw(host_coords);
    joiner.set_standing_order_kind(Order::JoinAnotherFleet);
    joiner.set_join_host_fleet_id_raw(host_id);
    joiner.set_standing_order_target_coords_raw(host_coords);

    let fleet_count_before = game_data.fleets.records.len();
    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(game_data.fleets.records.len(), fleet_count_before - 1);
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
    game_data.player.records[0].raw[0x00] = 0x00;

    let host = &mut game_data.fleets.records[2];
    host.set_current_location_coords_raw([10, 10]);
    host.set_standing_order_kind(Order::HoldPosition);
    host.set_standing_order_target_coords_raw([10, 10]);
    host.set_current_speed(0);
    let host_id = game_data.fleets.records[2].fleet_id();
    assert_ne!(
        host_id, 1,
        "host id must differ from the 0x22 world-target flag"
    );

    let joiner = &mut game_data.fleets.records[1];
    joiner.set_current_location_coords_raw([7, 10]);
    joiner.set_standing_order_kind(Order::JoinAnotherFleet);
    joiner.set_standing_order_target_coords_raw([10, 10]);
    joiner.set_mission_aux_bytes([1, host_id]);
    joiner.set_current_speed(3);
    joiner.raw[0x0d] = 0x80;
    joiner.raw[0x0f] = 0;
    joiner.raw[0x19] = 0x80;
    assert_eq!(joiner.join_host_fleet_id_raw(), host_id);

    let fleet_count_before = game_data.fleets.records.len();
    run_maintenance_turn(&mut game_data).expect("first maintenance turn should succeed");

    let joiner = &game_data.fleets.records[1];
    assert_eq!(joiner.standing_order_kind(), Order::JoinAnotherFleet);
    assert_eq!(joiner.mission_aux_bytes(), [1, host_id]);
    assert_ne!(
        joiner.current_location_coords_raw(),
        [7, 10],
        "joiner should move toward the host on the first turn"
    );

    let events =
        run_maintenance_turn(&mut game_data).expect("second maintenance turn should succeed");

    assert_eq!(
        game_data.fleets.records.len(),
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
    game_data.player.records[0].raw[0x00] = 0x00;
    let host_coords = game_data.fleets.records[0].current_location_coords_raw();
    let host_id = game_data.fleets.records[0].fleet_id();

    let joiner = &mut game_data.fleets.records[1];
    joiner.set_current_location_coords_raw(host_coords);
    joiner.set_standing_order_kind(Order::JoinAnotherFleet);
    joiner.set_join_host_fleet_id_raw(host_id);
    joiner.set_standing_order_target_coords_raw(host_coords);
    joiner.set_current_speed(3);
    set_in_transit_west_of_centre(joiner);

    let fleet_count_before = game_data.fleets.records.len();
    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(game_data.fleets.records.len(), fleet_count_before - 1);
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

    let fleet = &mut game_data.fleets.records[1];
    fleet.set_current_location_coords_raw(target_coords);
    fleet.set_standing_order_kind(Order::MoveOnly);
    fleet.set_standing_order_target_coords_raw(target_coords);
    fleet.set_current_speed(3);
    set_in_transit_west_of_centre(fleet);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let fleet = &game_data.fleets.records[1];
    assert_eq!(fleet.current_location_coords_raw(), target_coords);
    assert_eq!(fleet.standing_order_kind(), Order::HoldPosition);
    assert_eq!(fleet.current_speed(), 0);
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
    game_data.player.records[0].raw[0x00] = 0x00;
    let coords = game_data.fleets.records[0].current_location_coords_raw();
    game_data.fleets.records[0].set_standing_order_kind(Order::RendezvousSector);
    game_data.fleets.records[0].set_standing_order_target_coords_raw(coords);
    game_data.fleets.records[1].set_current_location_coords_raw(coords);
    game_data.fleets.records[1].set_standing_order_kind(Order::RendezvousSector);
    game_data.fleets.records[1].set_standing_order_target_coords_raw(coords);

    let fleet_count_before = game_data.fleets.records.len();
    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(game_data.fleets.records.len(), fleet_count_before - 1);
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
    game_data.player.records[0].raw[0x00] = 0x00;
    let coords = game_data.fleets.records[0].current_location_coords_raw();

    game_data.fleets.records[0].set_standing_order_kind(Order::RendezvousSector);
    game_data.fleets.records[0].set_standing_order_target_coords_raw(coords);

    let joiner = &mut game_data.fleets.records[1];
    joiner.set_current_location_coords_raw(coords);
    joiner.set_standing_order_kind(Order::RendezvousSector);
    joiner.set_standing_order_target_coords_raw(coords);
    joiner.set_current_speed(3);
    set_in_transit_west_of_centre(joiner);

    let fleet_count_before = game_data.fleets.records.len();
    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(game_data.fleets.records.len(), fleet_count_before - 1);
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
    game_data.player.records[0].raw[0x00] = 0x00;
    let coords = game_data.fleets.records[0].current_location_coords_raw();

    game_data.fleets.records[0].set_standing_order_kind(Order::RendezvousSector);
    game_data.fleets.records[0].set_standing_order_target_coords_raw(coords);

    let joiner = &mut game_data.fleets.records[1];
    joiner.set_current_location_coords_raw([coords[0].saturating_sub(1), coords[1]]);
    joiner.set_standing_order_kind(Order::RendezvousSector);
    joiner.set_standing_order_target_coords_raw(coords);
    joiner.set_current_speed(3);
    joiner.raw[0x0d] = 0x80;
    joiner.raw[0x0f] = 0;
    joiner.raw[0x19] = 0x80;

    let fleet_count_before = game_data.fleets.records.len();
    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(game_data.fleets.records.len(), fleet_count_before - 1);
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
    game_data.player.records[0].raw[0x00] = 0x00;
    let coords = game_data.fleets.records[0].current_location_coords_raw();

    game_data.fleets.records[0].set_local_slot_word_raw(9);
    game_data.fleets.records[0].set_standing_order_kind(Order::RendezvousSector);
    game_data.fleets.records[0].set_standing_order_target_coords_raw(coords);
    let survivor_id = game_data.fleets.records[0].fleet_id();

    game_data.fleets.records[1].set_local_slot_word_raw(1);
    game_data.fleets.records[1].set_current_location_coords_raw(coords);
    game_data.fleets.records[1].set_standing_order_kind(Order::RendezvousSector);
    game_data.fleets.records[1].set_standing_order_target_coords_raw(coords);

    run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let survivor = game_data
        .fleets
        .records
        .iter()
        .find(|fleet| fleet.fleet_id() == survivor_id)
        .expect("lowest structural fleet id should remain the rendezvous host");
    assert_eq!(survivor.local_slot_word_raw(), 9);
    assert_eq!(survivor.standing_order_kind(), Order::RendezvousSector);
    assert_eq!(survivor.standing_order_target_coords_raw(), coords);
}

#[test]
fn test_rendezvous_does_not_merge_before_reaching_its_assigned_sector() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.player.records[0].raw[0x00] = 0x00;
    let coords = game_data.fleets.records[0].current_location_coords_raw();

    game_data.fleets.records[0].set_standing_order_kind(Order::RendezvousSector);
    game_data.fleets.records[0].set_standing_order_target_coords_raw(coords);
    game_data.fleets.records[1].set_current_location_coords_raw(coords);
    game_data.fleets.records[1].set_standing_order_kind(Order::RendezvousSector);
    game_data.fleets.records[1].set_standing_order_target_coords_raw([coords[0] + 1, coords[1]]);
    game_data.fleets.records[1].set_current_speed(0);

    let fleet_count_before = game_data.fleets.records.len();
    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(game_data.fleets.records.len(), fleet_count_before);
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
        .records
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner_empire_slot_raw() == 1)
        .map(|(idx, planet)| (idx, planet.coords_raw()))
        .expect("fixture should contain an owned planet");
    let start_coords = if target_coords[0] > 1 {
        [target_coords[0] - 1, target_coords[1]]
    } else {
        [target_coords[0] + 1, target_coords[1]]
    };
    let stored_before = game_data.planets.records[planet_idx].stored_production_points();
    let fleet_count_before = game_data.fleets.records.len();

    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw(start_coords);
    fleet.set_standing_order_kind(Order::Salvage);
    fleet.set_standing_order_target_coords_raw(target_coords);
    fleet.set_current_speed(3);
    fleet.set_destroyer_count(1);
    fleet.set_cruiser_count(1);
    fleet.set_battleship_count(0);
    fleet.set_scout_count(0);
    fleet.set_troop_transport_count(0);
    fleet.set_army_count(0);
    fleet.set_etac_count(0);
    fleet.raw[0x0d] = 0x80;
    fleet.raw[0x0f] = 0;
    fleet.raw[0x19] = 0x00;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(game_data.fleets.records.len(), fleet_count_before - 1);
    assert_eq!(
        game_data.planets.records[planet_idx].stored_production_points(),
        stored_before + 10,
    );
    assert!(events.salvage_events.iter().any(|event| {
        matches!(
            event,
            SalvageResolvedEvent::Succeeded {
                owner_empire_raw,
                planet_idx: event_planet_idx,
                coords,
                recovered_points,
                ..
            } if *owner_empire_raw == 1
                && *event_planet_idx == planet_idx
                && *coords == target_coords
                && *recovered_points == 10
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
        .records
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner_empire_slot_raw() == 1)
        .map(|(idx, planet)| (idx, planet.coords_raw()))
        .expect("fixture should contain an owned planet");
    let start_coords = if target_coords[0] > 1 {
        [target_coords[0] - 1, target_coords[1]]
    } else {
        [target_coords[0] + 1, target_coords[1]]
    };
    let stored_before = game_data.planets.records[planet_idx].stored_production_points();
    let fleet_count_before = game_data.fleets.records.len();

    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw(start_coords);
    fleet.set_standing_order_kind(Order::Salvage);
    fleet.set_standing_order_target_coords_raw(target_coords);
    fleet.set_current_speed(3);
    // Zero out all ship counts — this fleet is an empty shell.
    fleet.set_destroyer_count(0);
    fleet.set_cruiser_count(0);
    fleet.set_battleship_count(0);
    fleet.set_scout_count(0);
    fleet.set_troop_transport_count(0);
    fleet.set_army_count(0);
    fleet.set_etac_count(0);
    fleet.raw[0x0d] = 0x80;
    fleet.raw[0x0f] = 0;
    fleet.raw[0x19] = 0x00;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    // Fleet removed by the start-of-turn cull.
    assert_eq!(
        game_data.fleets.records.len(),
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
        game_data.planets.records[planet_idx].stored_production_points(),
        stored_before,
        "planet treasury must not change when the empty fleet is culled before salvage"
    );
}

#[test]
fn test_salvage_fails_at_foreign_planet_without_removing_fleet() {
    let mut game_data = load_fixture("ecmaint-post");
    let (planet_idx, target_coords) = game_data
        .planets
        .records
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner_empire_slot_raw() == 2)
        .map(|(idx, planet)| (idx, planet.coords_raw()))
        .expect("fixture should contain a foreign planet");
    let start_coords = if target_coords[0] > 1 {
        [target_coords[0] - 1, target_coords[1]]
    } else {
        [target_coords[0] + 1, target_coords[1]]
    };
    let fleet_count_before = game_data.fleets.records.len();

    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw(start_coords);
    fleet.set_standing_order_kind(Order::Salvage);
    fleet.set_standing_order_target_coords_raw(target_coords);
    fleet.set_current_speed(3);
    fleet.raw[0x0d] = 0x80;
    fleet.raw[0x0f] = 0;
    fleet.raw[0x19] = 0x00;
    // The world's own fleets would attack the salvager in their system.
    common::hold_fire(&mut game_data, 2);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(game_data.fleets.records.len(), fleet_count_before);
    assert_eq!(
        game_data.fleets.records[0].standing_order_kind(),
        Order::HoldPosition
    );
    assert!(events.salvage_events.iter().any(|event| {
        matches!(
            event,
            SalvageResolvedEvent::Failed {
                owner_empire_raw,
                planet_idx: Some(idx),
                coords,
                reason,
                ..
            } if *owner_empire_raw == 1
                && *idx == planet_idx
                && *coords == target_coords
                && *reason == SalvageFailureReason::PlanetNotOwned
        )
    }));
}

#[test]
fn test_rendezvous_merge_emits_survivor_absorption_event() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.player.records[0].raw[0x00] = 0xff;
    let coords = game_data.fleets.records[0].current_location_coords_raw();
    game_data.fleets.records[0].set_standing_order_kind(Order::RendezvousSector);
    game_data.fleets.records[1].set_current_location_coords_raw(coords);
    game_data.fleets.records[1].set_standing_order_kind(Order::RendezvousSector);

    let survivor_id = game_data.fleets.records[0].fleet_id();
    let absorbed_id = game_data.fleets.records[1].fleet_id();

    let events = run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert!(events.fleet_merge_events.iter().any(|event| {
        event.kind == Mission::RendezvousSector
            && event.survivor_side
            && event.host_fleet_id_raw == survivor_id
            && event.absorbed_fleet_id_raw == absorbed_id
    }));
}

#[test]
fn test_rendezvous_survivor_remains_on_rendezvous_after_merge() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.player.records[0].raw[0x00] = 0x00;
    let coords = game_data.fleets.records[0].current_location_coords_raw();
    game_data.fleets.records[0].set_standing_order_kind(Order::RendezvousSector);
    game_data.fleets.records[0].set_standing_order_target_coords_raw(coords);
    game_data.fleets.records[1].set_current_location_coords_raw(coords);
    game_data.fleets.records[1].set_standing_order_kind(Order::RendezvousSector);
    game_data.fleets.records[1].set_standing_order_target_coords_raw(coords);

    let survivor_id = game_data.fleets.records[0].fleet_id();

    run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    let survivor = game_data
        .fleets
        .records
        .iter()
        .find(|fleet| fleet.fleet_id() == survivor_id)
        .expect("survivor fleet should remain present");
    assert_eq!(survivor.standing_order_kind(), Order::RendezvousSector);
    assert_eq!(survivor.standing_order_target_coords_raw(), coords);
}

#[test]
fn test_rendezvous_merge_recomputes_survivor_speed_to_slowest_member() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.player.records[0].raw[0x00] = 0xff;
    let coords = game_data.fleets.records[0].current_location_coords_raw();

    game_data.fleets.records[0].set_standing_order_kind(Order::RendezvousSector);
    game_data.fleets.records[0].set_destroyer_count(1);
    game_data.fleets.records[0].set_cruiser_count(0);
    game_data.fleets.records[0].set_battleship_count(0);
    game_data.fleets.records[0].set_scout_count(0);
    game_data.fleets.records[0].set_troop_transport_count(0);
    game_data.fleets.records[0].set_etac_count(0);
    game_data.fleets.records[0].set_max_speed(6);

    game_data.fleets.records[1].set_current_location_coords_raw(coords);
    game_data.fleets.records[1].set_standing_order_kind(Order::RendezvousSector);
    game_data.fleets.records[1].set_destroyer_count(0);
    game_data.fleets.records[1].set_cruiser_count(0);
    game_data.fleets.records[1].set_battleship_count(0);
    game_data.fleets.records[1].set_scout_count(0);
    game_data.fleets.records[1].set_troop_transport_count(0);
    game_data.fleets.records[1].set_etac_count(1);
    game_data.fleets.records[1].set_max_speed(3);

    run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    assert_eq!(game_data.fleets.records[0].max_speed(), 3);
}

#[test]
fn test_merge_preserves_surviving_local_fleet_numbers() {
    let mut game_data = load_fixture("ecmaint-post");
    game_data.player.records[0].raw[0x00] = 0xff;
    let coords = game_data.fleets.records[0].current_location_coords_raw();
    game_data.fleets.records[2].set_current_location_coords_raw([1, 1]);
    game_data.fleets.records[3].set_current_location_coords_raw([2, 2]);
    game_data.fleets.records[1].set_current_location_coords_raw(coords);
    game_data.fleets.records[1].set_standing_order_kind(Order::JoinAnotherFleet);

    run_maintenance_turn(&mut game_data).expect("Maintenance failed");

    let player1_local_slots = game_data
        .fleets
        .records
        .iter()
        .filter(|fleet| fleet.owner_empire_raw() == 1)
        .map(|fleet| fleet.local_slot_word_raw())
        .collect::<Vec<_>>();
    assert_eq!(player1_local_slots, vec![1, 3, 4]);

    let player2_local_slots = game_data
        .fleets
        .records
        .iter()
        .filter(|fleet| fleet.owner_empire_raw() == 2)
        .map(|fleet| fleet.local_slot_word_raw())
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
    let planet_coords = game_data.planets.records[13].coords_raw();
    let viewer = &mut game_data.fleets.records[0];
    viewer.set_current_location_coords_raw(planet_coords);
    viewer.set_standing_order_kind(Order::ViewWorld);
    viewer.set_standing_order_target_coords_raw(planet_coords);
    viewer.set_current_speed(0);
    viewer.set_scout_count(0);
    viewer.set_etac_count(0);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    // Observation must fire.
    assert!(
        events.planet_intel_events.iter().any(|e| {
            e.planet_idx == 13
                && e.viewer_empire_raw == 1
                && e.source == PlanetIntelSource::ViewWorld
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
        game_data.fleets.records[0].standing_order_kind(),
        Order::HoldPosition,
        "on-station ViewWorld must reset to HoldPosition after firing"
    );
    assert_eq!(game_data.fleets.records[0].current_speed(), 0);
    assert_eq!(
        game_data.fleets.records[0].tuple_c_payload_raw(),
        [0, 0, 0, 0, 0],
        "ECMAINT's View World action lifts the fleet to depth 0"
    );
}

#[test]
fn test_colonize_world_on_station_colonizes_and_resets() {
    // Fleet already at its target planet from a prior turn (on-station path).
    // ColonizeWorld is one-shot: the colonization must fire AND the order must
    // reset to HoldPosition with speed 0 — even though should_move is false.
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let planet_coords = game_data.planets.records[13].coords_raw(); // unowned planet at (15,13)
    let colonizer = &mut game_data.fleets.records[0];
    colonizer.set_current_location_coords_raw(planet_coords);
    colonizer.set_standing_order_kind(Order::ColonizeWorld);
    colonizer.set_standing_order_target_coords_raw(planet_coords);
    colonizer.set_current_speed(3); // speed > 0 to confirm the gate is the blocker
    colonizer.set_etac_count(3);
    colonizer.set_scout_count(0);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    // Planet must be colonized.
    assert_eq!(
        game_data.planets.records[13].owner_empire_slot_raw(),
        1,
        "on-station ColonizeWorld must colonize the planet"
    );
    assert_eq!(
        game_data.planets.records[13].ownership_status_raw(),
        2,
        "on-station ColonizeWorld must set ownership_status to 2"
    );

    // Fleet must be reset to HoldPosition with speed 0.
    assert_eq!(
        game_data.fleets.records[0].standing_order_kind(),
        Order::HoldPosition,
        "on-station ColonizeWorld must reset to HoldPosition after firing"
    );
    assert_eq!(
        game_data.fleets.records[0].current_speed(),
        0,
        "on-station ColonizeWorld must set speed to 0 after firing"
    );

    // Events must be emitted.
    assert!(
        events.colonization_events.iter().any(|e| matches!(
            e,
            ColonizationResolvedEvent::Succeeded {
                fleet_idx: 0,
                planet_idx: 13,
                colonizer_empire_raw: 1,
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
    let planet_coords = game_data.planets.records[13].coords_raw();
    let scout = &mut game_data.fleets.records[0];
    scout.set_current_location_coords_raw(planet_coords);
    scout.set_standing_order_kind(Order::ScoutSector);
    scout.set_standing_order_target_coords_raw(planet_coords);
    scout.set_current_speed(0);
    scout.set_scout_count(1);

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
        game_data.fleets.records[0].standing_order_kind(),
        Order::ScoutSector,
        "ScoutSector must persist on station after firing"
    );
    assert_eq!(game_data.fleets.records[0].current_speed(), 0);
}

#[test]
fn test_scout_system_on_station_fires_intel_and_persists() {
    // ScoutSolarSystem fleet already at target from a prior turn.
    // Must emit a planet intel event and a mission event, and keep Order::ScoutSolarSystem.
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let planet_coords = game_data.planets.records[13].coords_raw();
    let scout = &mut game_data.fleets.records[0];
    scout.set_current_location_coords_raw(planet_coords);
    scout.set_standing_order_kind(Order::ScoutSolarSystem);
    scout.set_standing_order_target_coords_raw(planet_coords);
    scout.set_current_speed(0);
    scout.set_scout_count(1);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert!(
        events.planet_intel_events.iter().any(|e| {
            e.planet_idx == 13
                && e.viewer_empire_raw == 1
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
        game_data.fleets.records[0].standing_order_kind(),
        Order::ScoutSolarSystem,
        "ScoutSolarSystem must persist on station after firing"
    );
    assert_eq!(game_data.fleets.records[0].current_speed(), 0);
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
    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw(target_coords);
    fleet.set_standing_order_kind(Order::PatrolSector);
    fleet.set_standing_order_target_coords_raw(target_coords);
    fleet.set_current_speed(0);

    run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(
        game_data.fleets.records[0].standing_order_kind(),
        Order::PatrolSector,
        "PatrolSector must persist on station"
    );
    assert_eq!(game_data.fleets.records[0].current_speed(), 0);
}

#[test]
fn test_guard_blockade_world_on_station_persists() {
    // GuardBlockadeWorld fleet already at target from a prior turn.
    // No on-station observation fires; order must simply persist.
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    let target_coords = game_data.planets.records[13].coords_raw();
    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw(target_coords);
    fleet.set_standing_order_kind(Order::GuardBlockadeWorld);
    fleet.set_standing_order_target_coords_raw(target_coords);
    fleet.set_current_speed(0);

    run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert_eq!(
        game_data.fleets.records[0].standing_order_kind(),
        Order::GuardBlockadeWorld,
        "GuardBlockadeWorld must persist on station"
    );
    assert_eq!(game_data.fleets.records[0].current_speed(), 0);
}

// ---------------------------------------------------------------------------
// Group D — Hostile order turn-2 execution (shape only)
// ---------------------------------------------------------------------------

/// Build a game state where a hostile fleet is already on-station and ready to
/// execute (transit_ready_flag_raw == 0x80).  The target planet at [25,25] is
/// owned by empire 2 with 10 armies and 4 ground batteries.
fn hostile_on_station_ready(
    order: Order,
    ships: (u16, u16, u16, u16, u16, u8),
) -> (CoreGameData, usize, [u8; 2]) {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3000)
        .build_initialized_baseline()
        .expect("baseline should build");

    let target_coords = [25, 25];
    let target_idx = 4;

    let target_world = &mut game_data.planets.records[target_idx];
    target_world.set_coords_raw(target_coords);
    target_world.set_owner_empire_slot_raw(2);
    target_world.set_ownership_status_raw(2);
    target_world.set_planet_name("Target");
    target_world.set_army_count_raw(10);
    target_world.set_ground_batteries_raw(4);

    // (battleships, cruisers, destroyers, transports, armies, scouts)
    let fleet = &mut game_data.fleets.records[0];
    fleet.set_battleship_count(ships.0);
    fleet.set_cruiser_count(ships.1);
    fleet.set_destroyer_count(ships.2);
    fleet.set_troop_transport_count(ships.3);
    fleet.set_army_count(ships.4);
    fleet.set_scout_count(ships.5);
    fleet.set_etac_count(0);
    fleet.recompute_max_speed_from_composition();
    fleet.set_current_location_coords_raw(target_coords);
    fleet.set_standing_order_kind(order);
    fleet.set_standing_order_target_coords_raw(target_coords);
    // At rest in orbit (depth 1.0): the strike test passes on the first tick.
    fleet.set_current_speed(0);
    fleet.set_at_rest_motion();

    (game_data, target_idx, target_coords)
}

fn hostile_mixed_ready_exact_transit_state(
    order: Order,
    ships: (u16, u16, u16, u16, u16, u16, u16),
) -> (CoreGameData, usize, [u8; 2]) {
    let (mut game_data, target_idx, target_coords) =
        configured_delayed_hostile_arrival_state(order, ships);
    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw(target_coords);
    fleet.set_current_speed(3);
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
            .any(|e| e.planet_idx == target_idx && e.attacker_empire_raw == 1),
        "BombardEvent must reference the target planet and attacker empire"
    );
    assert_eq!(
        game_data.fleets.records[0].standing_order_kind(),
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
                && e.attacker_empire_raw == 1),
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
                && e.attacker_empire_raw == 1),
        "AssaultReportEvent must reference BlitzWorld, the target planet, and attacker empire"
    );
}

#[test]
fn test_tt_only_blitz_executes_without_combat_ships() {
    let (mut game_data, target_idx, target_coords) =
        hostile_on_station_ready(Order::BlitzWorld, (0, 0, 0, 12, 12, 0));
    game_data.planets.records[target_idx].set_ground_batteries_raw(0);
    game_data.planets.records[target_idx].set_army_count_raw(3);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    assert!(
        events.assault_report_events.iter().any(|event| {
            event.kind == Mission::BlitzWorld
                && event.planet_idx == target_idx
                && event.attacker_empire_raw == 1
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
        game_data.fleets.records[0].standing_order_kind(),
        Order::HoldPosition,
        "resolved blitz should clear back to HoldPosition"
    );
    assert_eq!(
        game_data.fleets.records[0].current_location_coords_raw(),
        target_coords,
        "fleet should stay on station after the blitz resolves"
    );
}

#[test]
fn test_tt_only_blitz_ignores_enemy_support_only_orbit() {
    let (mut game_data, target_idx, _) =
        hostile_on_station_ready(Order::BlitzWorld, (0, 0, 0, 12, 12, 0));
    game_data.planets.records[target_idx].set_ground_batteries_raw(0);
    game_data.planets.records[target_idx].set_army_count_raw(3);

    let enemy = &mut game_data.fleets.records[1];
    enemy.set_owner_empire_raw(2);
    enemy.set_current_location_coords_raw(game_data.planets.records[target_idx].coords_raw());
    enemy.set_standing_order_kind(Order::HoldPosition);
    enemy.set_current_speed(0);
    enemy.set_troop_transport_count(2);
    enemy.set_army_count(0);
    enemy.set_destroyer_count(0);
    enemy.set_cruiser_count(0);
    enemy.set_battleship_count(0);
    enemy.set_scout_count(0);
    enemy.set_etac_count(0);
    enemy.set_rules_of_engagement(0);

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
    game_data.planets.records[target_idx].set_ground_batteries_raw(0);
    game_data.planets.records[target_idx].set_army_count_raw(3);
    add_active_starbase(&mut game_data, 2, target_coords);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");
    let abort = events
        .mission_events
        .iter()
        .find(|event| event.kind == Mission::BlitzWorld && event.outcome == MissionOutcome::Aborted)
        .expect("blocked TT*-only blitz should emit an aborted mission event");

    assert!(
        matches!(
            abort.abort_reason,
            None | Some(MissionAbortReason::OrbitBlocked)
        ),
        "blocked TT*-only blitz should abort cleanly whether the block happens in fleet combat or assault resolution"
    );
    assert!(
        events.assault_report_events.is_empty(),
        "blocked TT*-only blitz should not resolve a ground assault"
    );
    assert_eq!(
        game_data.fleets.records[0].standing_order_kind(),
        Order::HoldPosition,
        "blocked blitz should clear the hostile order"
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
            let fleet = &mut game_data.fleets.records[0];
            fleet.set_current_location_coords_raw(target_coords);
            fleet.set_current_speed(fleet.max_speed());
        }
        game_data
            .set_fleet_order(
                1,
                game_data.fleets.records[0].max_speed(),
                order.to_raw(),
                target_coords,
                None,
                None,
            )
            .expect("on-station hostile order should arm");

        // The order keeps the fleet's stored motion: at rest in orbit
        // (depth 1.0), so its strike test passes on the first tick.
        let fleet = &game_data.fleets.records[0];
        assert_eq!(fleet.current_speed(), fleet.max_speed());
        assert_eq!(fleet.depth(), Real48::ONE);
        assert_eq!(fleet.current_location_coords_raw(), target_coords);
        assert_eq!(fleet.standing_order_kind(), order);

        let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

        match mission {
            Mission::BombardWorld => assert!(
                events
                    .bombard_events
                    .iter()
                    .any(|e| e.planet_idx == target_idx && e.attacker_empire_raw == 1),
                "BombardWorld should execute on the next maintenance tick"
            ),
            Mission::InvadeWorld | Mission::BlitzWorld => assert!(
                events.assault_report_events.iter().any(|e| {
                    e.kind == mission && e.planet_idx == target_idx && e.attacker_empire_raw == 1
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

        let fleet = &mut game_data.fleets.records[0];
        fleet.set_current_location_coords_raw([15, 25]);
        fleet.set_current_speed(0);
        fleet.set_tuple_c_payload_raw([0x80, 0xb9, 0xff, 0xff, 0xff]);
        let pre_batteries = game_data.planets.records[target_idx].ground_batteries_raw();
        let pre_armies = game_data.planets.records[target_idx].army_count_raw();

        let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");
        let fleet = &game_data.fleets.records[0];

        assert_eq!(
            fleet.standing_order_kind(),
            order,
            "{name} order should persist"
        );
        assert_eq!(
            fleet.standing_order_target_coords_raw(),
            target_coords,
            "{name} target"
        );
        assert_eq!(
            fleet.current_speed(),
            fleet.max_speed().max(1),
            "{name} speed should rearm"
        );
        assert_eq!(
            fleet.depth(),
            Real48::ZERO,
            "{name} should climb out and resume travel"
        );
        assert_ne!(
            fleet.current_location_coords_raw(),
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
            game_data.planets.records[target_idx].ground_batteries_raw(),
            pre_batteries,
            "{name} should not change target batteries while off target"
        );
        assert_eq!(
            game_data.planets.records[target_idx].army_count_raw(),
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
            game_data.fleets.records[0].current_location_coords_raw(),
            target_coords,
            "{name} coords"
        );
    }
}

#[test]
fn test_join_succeeds_when_fleet_id_remapped_by_prior_salvage_removal() {
    // Regression: apply_fleet_removal_remap previously left join_host_fleet_id_raw
    // stale after a salvage removal shifted fleet IDs.  When fleet 1 (fleet_id=2)
    // is removed by salvage during movement, fleet 2 compresses from fleet_id=3 to
    // fleet_id=2.  Without the fix, the joiner's stored host_id=3 no longer
    // resolves and the join silently fails.  With the fix the remap also updates
    // join_host_fleet_id_raw and the join succeeds on the same turn.
    let mut game_data = load_fixture("ecmaint-post");
    // Disable early consolidation so it cannot merge or remove fleets before
    // the salvage-triggered remap this test is exercising.
    game_data.player.records[0].raw[0x00] = 0x00;

    // Find a planet owned by player 1 for the salvage fleet to target.
    let (_, planet_coords) = game_data
        .planets
        .records
        .iter()
        .enumerate()
        .find(|(_, planet)| planet.owner_empire_slot_raw() == 1)
        .map(|(idx, planet)| (idx, planet.coords_raw()))
        .expect("fixture should contain an owned planet for player 1");

    // Fleet 2 (fleet_id=3) is the host — stationary at a fixed location.
    let host_coords = [10u8, 10u8];
    let host_id = game_data.fleets.records[2].fleet_id(); // invariant: == 3
    {
        let host = &mut game_data.fleets.records[2];
        host.set_current_location_coords_raw(host_coords);
        host.set_standing_order_kind(Order::HoldPosition);
        host.set_current_speed(0);
        host.raw[0x0d] = 0x00;
        host.raw[0x0f] = 0;
        host.raw[0x19] = 0x00;
    }

    // Fleet 0 (fleet_id=1) is the joiner — co-located with the host, storing
    // the host's current fleet_id (3) in join_host_fleet_id_raw.
    {
        let joiner = &mut game_data.fleets.records[0];
        joiner.set_current_location_coords_raw(host_coords);
        joiner.set_standing_order_kind(Order::JoinAnotherFleet);
        joiner.set_join_host_fleet_id_raw(host_id);
        joiner.set_standing_order_target_coords_raw(host_coords);
        joiner.set_current_speed(0);
    }

    // Fleet 1 (fleet_id=2) is the salvage fleet — one sector from the planet,
    // carrying ships so recovered_points > 0 and the fleet is removed on arrival.
    let salvage_start = if planet_coords[0] > 1 {
        [planet_coords[0] - 1, planet_coords[1]]
    } else {
        [planet_coords[0] + 1, planet_coords[1]]
    };
    {
        let salvager = &mut game_data.fleets.records[1];
        salvager.set_current_location_coords_raw(salvage_start);
        salvager.set_standing_order_kind(Order::Salvage);
        salvager.set_standing_order_target_coords_raw(planet_coords);
        salvager.set_current_speed(3);
        salvager.set_destroyer_count(1);
        salvager.set_cruiser_count(0);
        salvager.set_battleship_count(0);
        salvager.set_scout_count(0);
        salvager.set_troop_transport_count(0);
        salvager.set_army_count(0);
        salvager.set_etac_count(0);
        salvager.raw[0x0d] = 0x80;
        salvager.raw[0x0f] = 0;
        salvager.raw[0x19] = 0x00;
    }

    let fleet_count_before = game_data.fleets.records.len();
    let events = run_maintenance_turn(&mut game_data).expect("maintenance turn should succeed");

    // Salvage removes fleet 1 (-1); join merges fleet 0 into the host (-1).
    assert_eq!(
        game_data.fleets.records.len(),
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
fn set_in_transit_west_of_centre(fleet: &mut nc_data::FleetRecord) {
    fleet.set_x_fraction(Real48::from_int(1) / Real48::from_int(10));
    fleet.set_y_fraction(Real48::HALF);
    fleet.set_depth(Real48::ZERO);
}
