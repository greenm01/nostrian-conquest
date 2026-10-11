mod common;

use common::order_trace::assert_mission_event;
use nc_compat::load_classic_core_files;
use nc_data::{
    ContactReportSource, Coords, CoreGameData, DiplomacyOverride, EmpireId,
    EncounterDispositionEvent, Mission, MissionOutcome, Order, Planet, PlanetIntelSource, Real48,
    diplomatic_relation_toward, has_any_force, set_armies_aboard, set_fleet_at_rest,
    set_mission_target, set_starbase_at_rest, set_troop_transports, troop_transports,
};
use nc_engine::{run_maintenance_turn, run_maintenance_turn_with_context};
use std::path::Path;

fn load_fixture(name: &str) -> CoreGameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
        .join("v1.5");
    load_classic_core_files(&dir)
        .unwrap_or_else(|e| panic!("Failed to load fixture {}: {}", name, e))
}

fn configured_assault_state(order_code: u8) -> CoreGameData {
    let mut game_data = load_fixture("ecmaint-post");

    // Use the unowned world at (15,13) as a deterministic assault target.
    let target = &mut game_data.planets[13];
    *target = Planet {
        sector: Coords::new(15, 13),
        potential_production: 100,
        production: Real48::from_int(100),
        stored_goods: Real48::from_int(66),
        name: "TargetPrime".to_string(),
        armies: 10,
        ground_batteries: 4,
        years_under_owner: 2,
        owner: EmpireId(2),
        ..Planet::default()
    };

    // Reuse fleet 1 as the attacker and mark it as already-arrived this turn.
    let attacker = &mut game_data.fleets[0];
    attacker.sector = Coords::new(15, 13);
    attacker.order = Order::from_code(order_code);
    attacker.target = Coords::new(15, 13);
    attacker.speed = 3;
    // At rest in orbit: the strike test passes on the first tick.
    set_fleet_at_rest(attacker);
    attacker.rules_of_engagement = 10;
    attacker.ships.scouts = 0;
    attacker.ships.battleships = 0;
    attacker.ships.cruisers = 0;
    attacker.ships.destroyers = 1;
    set_troop_transports(attacker, 0);
    set_armies_aboard(attacker, 0);
    attacker.ships.etacs = 0;

    game_data
}

fn configured_invade_contact_state(attacker_roe: u8, hostile_roe: u8) -> CoreGameData {
    let mut game_data = load_fixture("ecmaint-post");
    let coords = [9, 13];
    let target = [15, 13];

    let attacker = &mut game_data.fleets[0];
    attacker.sector = Coords::from_array(coords);
    attacker.order = Order::InvadeWorld;
    attacker.target = Coords::from_array(target);
    attacker.speed = 0;
    attacker.ships.destroyers = 2;
    attacker.ships.cruisers = 4;
    attacker.ships.battleships = 1;
    attacker.ships.scouts = 0;
    set_troop_transports(attacker, 2);
    set_armies_aboard(attacker, 2);
    attacker.ships.etacs = 0;
    attacker.rules_of_engagement = attacker_roe;
    common::stall_in_place(attacker);

    let hostile = &mut game_data.fleets[4];
    hostile.sector = Coords::from_array(coords);
    hostile.order = Order::MoveOnly;
    hostile.target = Coords::new(8, 13);
    hostile.speed = 0;
    // Big enough to last into round 4, when ROE withdrawals start.
    hostile.ships.destroyers = 0;
    hostile.ships.cruisers = 8;
    hostile.ships.battleships = 0;
    hostile.ships.scouts = 0;
    set_troop_transports(hostile, 0);
    set_armies_aboard(hostile, 0);
    hostile.ships.etacs = 0;
    hostile.rules_of_engagement = hostile_roe;
    common::stall_in_place(hostile);

    game_data
}

fn configure_bombard_target_world(
    game_data: &mut CoreGameData,
    batteries: u8,
    armies: u8,
    stored_goods: u32,
    factories: u16,
) {
    let target = &mut game_data.planets[13];
    target.ground_batteries = u16::from(batteries);
    target.armies = u16::from(armies);
    target.stored_goods = Real48::from_f64(f64::from(stored_goods));
    target.production = Real48::from_f64(f64::from(factories));
    for slot in 0..nc_data::STARDOCK_SLOT_COUNT {
        target.stardock[slot] = 0;
    }
}

fn add_active_starbase(game_data: &mut CoreGameData, owner: u8, coords: [u8; 2]) {
    let mut base = nc_data::Starbase::default();
    base.base_id.0 = 1;
    base.record.0 = 1;
    base.sector = Coords::from_array(coords);
    base.target = Coords::from_array(coords);
    base.owner.0 = owner;
    // In orbit (depth 1), as commissioned starbases are.
    set_starbase_at_rest(&mut base);
    base.target_depth = 1;
    game_data.bases.push(base);
}

fn mutual_enemy_overrides(left: u8, right: u8) -> [DiplomacyOverride; 2] {
    [
        DiplomacyOverride {
            from_empire: left,
            to_empire: right,
            relation: nc_data::DiplomaticRelation::Enemy,
        },
        DiplomacyOverride {
            from_empire: right,
            to_empire: left,
            relation: nc_data::DiplomaticRelation::Enemy,
        },
    ]
}

fn configured_dominant_invalidated_invade_state() -> CoreGameData {
    let mut game_data = load_fixture("ecmaint-post");
    let coords = [15, 13];

    game_data.bases.clear();
    for fleet in &mut game_data.fleets {
        let current = fleet.sector.to_array();
        fleet.speed = 0;
        fleet.order = Order::HoldPosition;
        fleet.target = Coords::from_array(current);
        fleet.rules_of_engagement = 10;
        fleet.ships.destroyers = 0;
        fleet.ships.cruisers = 0;
        fleet.ships.battleships = 0;
        fleet.ships.scouts = 0;
        set_troop_transports(fleet, 0);
        set_armies_aboard(fleet, 0);
        fleet.ships.etacs = 0;
    }

    let target = &mut game_data.planets[13];
    *target = Planet {
        sector: Coords::from_array(coords),
        potential_production: 100,
        production: Real48::from_int(100),
        stored_goods: Real48::from_int(66),
        name: "TargetPrime".to_string(),
        armies: 10,
        ground_batteries: 4,
        years_under_owner: 2,
        owner: EmpireId(2),
        ..Planet::default()
    };

    let attacker = &mut game_data.fleets[0];
    attacker.sector = Coords::from_array(coords);
    attacker.order = Order::InvadeWorld;
    attacker.target = Coords::from_array(coords);
    attacker.speed = 3;
    // At rest in orbit: the strike test passes on the first tick.
    set_fleet_at_rest(attacker);
    attacker.ships.destroyers = 1;
    set_troop_transports(attacker, 1);
    set_armies_aboard(attacker, 1);

    let defender = &mut game_data.fleets[4];
    defender.sector = Coords::from_array(coords);
    defender.order = Order::HoldPosition;
    defender.target = Coords::from_array(coords);
    defender.speed = 0;
    defender.ships.destroyers = 1;

    game_data
}

#[test]
fn canonical_bombardment_consumes_order_and_devastates_target() {
    let mut game_data = load_fixture("ecmaint-bombard-arrive");

    let pre_target = game_data.planets[13].clone();
    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(events.bombard_events.len(), 1);
    assert_eq!(events.bombard_events[0].planet_idx, 13);
    assert_eq!(events.bombard_events[0].attacker_empire, 1);
    assert!(
        events.planet_intel_events.is_empty(),
        "bombardment alone should not refresh classic planet-database intel"
    );
    assert!(events.colonization_events.is_empty());

    let attacker = &game_data.fleets[2];
    assert_eq!(attacker.sector.to_array(), [15, 13]);
    // Bombardment persists: fleet keeps BombardWorld order for next turn.
    assert_eq!(attacker.order.code(), nc_data::Order::BombardWorld.code());
    // It stops in orbit to bombard again next year.
    assert_eq!(attacker.speed, 0);
    assert_eq!(attacker.depth, nc_data::Real48::ONE);
    assert_eq!(attacker.ships.cruisers, 0);
    assert_eq!(attacker.ships.destroyers, 0);

    let post_target = &game_data.planets[13];
    assert_eq!(post_target.owner.0, pre_target.owner.0);
    // Three-round bombardment: batteries absorb suppression fire in rounds 1-2.
    // Armies are only hit if batteries reach 0 before round 3 (breakthrough).
    assert!(
        u8::try_from(post_target.ground_batteries).unwrap_or(u8::MAX)
            <= u8::try_from(pre_target.ground_batteries).unwrap_or(u8::MAX),
        "batteries should be damaged or destroyed"
    );
    if post_target.ground_batteries == 0 {
        // Breakthrough occurred: armies may have been reduced.
        assert!(
            u8::try_from(post_target.armies).unwrap_or(u8::MAX)
                <= u8::try_from(pre_target.armies).unwrap_or(u8::MAX)
        );
    } else {
        // No breakthrough: armies are shielded.
        assert_eq!(
            u8::try_from(post_target.armies).unwrap_or(u8::MAX),
            u8::try_from(pre_target.armies).unwrap_or(u8::MAX)
        );
    }
    assert_eq!(
        diplomatic_relation_toward(&game_data.players[0], 2),
        Some(nc_data::DiplomaticRelation::Enemy)
    );
    assert_eq!(
        diplomatic_relation_toward(&game_data.players[1], 1),
        Some(nc_data::DiplomaticRelation::Enemy)
    );
}

#[test]
fn bombardment_armies_without_batteries_do_not_inflict_orbital_losses() {
    let mut game_data = configured_assault_state(Order::BombardWorld.code());
    configure_bombard_target_world(&mut game_data, 0, 6, 0, 0);

    let attacker = &mut game_data.fleets[0];
    attacker.ships.destroyers = 0;
    attacker.ships.cruisers = 1;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(events.bombard_events.len(), 1);
    let bombard = &events.bombard_events[0];
    assert_eq!(bombard.defender_batteries_initial, 0);
    assert_eq!(bombard.defender_armies_initial, 6);
    assert_eq!(bombard.attacker_losses, nc_data::ShipLosses::default());

    let attacker = &game_data.fleets[0];
    assert_eq!(attacker.ships.cruisers, 1);
    assert_eq!(attacker.ships.destroyers, 0);

    let target = &game_data.planets[13];
    assert_eq!(target.ground_batteries, 0);
    assert!(u8::try_from(target.armies).unwrap_or(u8::MAX) < 6);
}

#[test]
fn bombardment_batteries_still_inflict_orbital_losses() {
    let mut game_data = configured_assault_state(Order::BombardWorld.code());
    configure_bombard_target_world(&mut game_data, 4, 0, 0, 0);

    let attacker = &mut game_data.fleets[0];
    attacker.ships.destroyers = 0;
    attacker.ships.cruisers = 1;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(events.bombard_events.len(), 1);
    let bombard = &events.bombard_events[0];
    assert_eq!(bombard.defender_batteries_initial, 4);
    assert_eq!(bombard.defender_armies_initial, 0);
    assert!(
        bombard.attacker_losses.cruisers > 0
            || bombard.attacker_losses.destroyers > 0
            || bombard.attacker_losses.battleships > 0,
        "ground batteries should still inflict orbital losses"
    );

    let attacker = &game_data.fleets[0];
    assert_eq!(attacker.ships.cruisers, 0);
}

#[test]
fn foreign_contact_without_enemy_declaration_reports_but_does_not_force_battle() {
    let mut game_data = load_fixture("ecmaint-fleet-battle-pre");
    game_data.fleets[0].order = Order::ScoutSector;
    game_data.fleets[0].ships.scouts = 1;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(events.fleet_battle_events.len(), 0);
    assert!(events.fleet_destroyed_events.is_empty());
    assert!(events.scout_contact_events.iter().any(|event| {
        event.viewer_empire == 1
            && event.source == ContactReportSource::FleetMission(Mission::ScoutSector)
            && event.coords == [10, 10]
            && event.target_empire == 2
    }));
    assert!(events.mission_events.iter().all(|event| {
        !(event.kind == Mission::ScoutSector && event.outcome == MissionOutcome::Aborted)
    }));
}

#[test]
fn declared_enemy_fleet_battle_clears_destroyed_scout_fleet_without_garbage_counts() {
    let mut game_data = load_fixture("ecmaint-fleet-battle-pre");
    game_data.fleets[0].order = Order::ScoutSector;
    game_data.fleets[0].ships.scouts = 1;
    let diplomacy = mutual_enemy_overrides(1, 2);

    let (game_data, events) = common::first_year(&game_data, &diplomacy, |year, _| {
        !has_any_force(&year.fleets[0])
    });

    let loser_one = &game_data.fleets[0];
    assert_eq!(loser_one.sector.to_array(), [10, 10]);
    assert_eq!(loser_one.order.code(), 0);
    assert_eq!(loser_one.rules_of_engagement, 0);
    assert_eq!(loser_one.ships.battleships, 0);
    assert_eq!(loser_one.ships.cruisers, 0);
    assert_eq!(loser_one.ships.destroyers, 0);
    assert_eq!(troop_transports(loser_one), 0);
    assert_eq!(loser_one.ships.scouts, 0);

    assert!(events.mission_events.iter().any(|event| {
        event.fleet_idx == 0
            && event.kind == Mission::ScoutSector
            && event.outcome == MissionOutcome::Aborted
            && event.location_coords == Some([10, 10])
    }));
    assert!(events.fleet_battle_events.iter().any(|event| {
        event.coords == [10, 10] && event.enemy_empires.contains(&2) && event.reporting_empire == 1
    }));
    // Whoever attacked sighted the other first.
    assert!(
        events
            .scout_contact_events
            .iter()
            .any(|event| { event.coords == [10, 10] && matches!(event.viewer_empire, 1 | 2) })
    );

    let survivor = &game_data.fleets[6];
    assert_eq!(survivor.sector.to_array(), [10, 10]);
    assert_eq!(survivor.ships.battleships, 1);
    // The fixture's byte 0x24 = 10 is the mission id's high byte; the fleet
    // has no scouts.
    assert_eq!(survivor.ships.scouts, 0);
    assert_eq!(survivor.ships.etacs, 1);

    for fleet in &game_data.fleets {
        assert!(fleet.ships.battleships <= 100);
        assert!(fleet.ships.cruisers <= 100);
        assert!(fleet.ships.destroyers <= 100);
        assert!(troop_transports(fleet) <= 100);
    }
}

#[test]
fn dominant_fleet_with_invalidated_invade_order_holds_and_skips_stale_assault() {
    let game_data = configured_dominant_invalidated_invade_state();
    let diplomacy = mutual_enemy_overrides(1, 2);

    let (game_data, events) = common::first_year(&game_data, &diplomacy, |_, events| {
        events.fleet_battle_events.iter().any(|event| {
            event.reporting_empire == 1 && event.held_field && event.friendly_losses.destroyers == 1
        })
    });

    // The battle comes mid-year: the aborted fleet seeks home at once and may
    // reach it (16,13) before the year ends.
    let attacker = &game_data.fleets[0];
    assert_ne!(attacker.order, Order::InvadeWorld);
    assert!(matches!(
        attacker.order,
        Order::SeekHome | Order::HoldPosition
    ));
    assert_eq!(attacker.target.to_array(), [16, 13]);

    assert_eq!(game_data.planets[13].owner.0, 2);
    assert!(
        events.assault_report_events.is_empty(),
        "fleet battle should invalidate the stale invade before the assault phase runs"
    );
    assert!(events.ownership_change_events.is_empty());
    assert_eq!(
        events
            .mission_events
            .iter()
            .filter(|event| {
                event.fleet_idx == 0
                    && event.kind == Mission::InvadeWorld
                    && event.outcome == MissionOutcome::Aborted
            })
            .count(),
        1,
    );
    assert!(events.fleet_battle_events.iter().any(|event| {
        event.reporting_empire == 1 && event.reporting_mission == Some(Mission::InvadeWorld)
    }));
}

#[test]
fn roe_zero_enemies_identify_and_avoid_each_other() {
    let mut game_data = load_fixture("ecmaint-post");
    let coords = [15, 13];

    let patrol = &mut game_data.fleets[0];
    patrol.sector = Coords::from_array(coords);
    patrol.order = Order::PatrolSector;
    patrol.ships.destroyers = 1;
    patrol.ships.cruisers = 0;
    patrol.ships.battleships = 0;
    patrol.ships.scouts = 0;
    set_troop_transports(patrol, 0);
    patrol.ships.etacs = 0;
    patrol.rules_of_engagement = 0;
    common::stall_in_place(patrol);

    let hostile = &mut game_data.fleets[4];
    hostile.sector = Coords::from_array(coords);
    hostile.order = Order::MoveOnly;
    hostile.ships.destroyers = 1;
    hostile.ships.cruisers = 0;
    hostile.ships.battleships = 0;
    hostile.ships.scouts = 0;
    set_troop_transports(hostile, 0);
    hostile.ships.etacs = 0;
    hostile.rules_of_engagement = 0;
    common::stall_in_place(hostile);
    let diplomacy = mutual_enemy_overrides(1, 2);

    // ROE 0 never attacks. The patrol identifies the enemy and reports that
    // its ROE has it avoid the fight; both keep their orders.
    let (game_data, events) = common::first_year(&game_data, &diplomacy, |_, events| {
        !events.encounter_disposition_events.is_empty()
    });
    assert!(events.fleet_battle_events.is_empty());
    assert!(events.encounter_disposition_events.iter().any(|event| {
        matches!(
            event,
            EncounterDispositionEvent::NoEngagement {
                owner_empire: 1,
                mission: Some(Mission::PatrolSector),
                coords: event_coords,
                target_empire: 2,
                reason: nc_data::EncounterDispositionReason::RoeDeclined,
                ..
            } if *event_coords == coords
        )
    }));
    assert_eq!(game_data.fleets[0].order, Order::PatrolSector);
    assert_eq!(game_data.fleets[4].order, Order::MoveOnly);
}

/// A year in which someone withdrew under ROE; a failed intercept alone
/// is also a disposition event, but not the one these tests want.
fn retreated(events: &nc_data::MaintenanceEvents) -> bool {
    events
        .encounter_disposition_events
        .iter()
        .any(|event| matches!(event, EncounterDispositionEvent::Retreated { .. }))
}

#[test]
fn enemy_roe_withdrawal_does_not_abort_invader_mission() {
    let game_data = configured_invade_contact_state(10, 0);
    let diplomacy = mutual_enemy_overrides(1, 2);

    // A year where the hostile withdrew and survived, and the invader kept
    // its loaded transports (losing them aborts the mission for another
    // reason).
    let (game_data, events) = common::first_year(&game_data, &diplomacy, |year, events| {
        retreated(events)
            && year.fleets[0].ships.loaded_transports == 2
            && has_any_force(&year.fleets[4])
    });

    assert!(
        events.encounter_disposition_events.iter().any(|event| {
            matches!(
                event,
                EncounterDispositionEvent::Retreated {
                    owner_empire,
                    mission: Some(Mission::MoveOnly),
                    reason: nc_data::EncounterDispositionReason::RoeWithdrawal,
                    ..
                } if *owner_empire == 2
            )
        }),
        "{events:?}"
    );
    assert!(!events.encounter_disposition_events.iter().any(|event| {
        matches!(
            event,
            EncounterDispositionEvent::Retreated {
                owner_empire,
                mission: Some(Mission::InvadeWorld),
                reason: nc_data::EncounterDispositionReason::RoeWithdrawal,
                ..
            } if *owner_empire == 1
        )
    }));
    assert!(!events.mission_events.iter().any(|event| {
        event.owner_empire == 1
            && event.kind == Mission::InvadeWorld
            && event.outcome == MissionOutcome::Aborted
    }));

    let attacker = &game_data.fleets[0];
    assert_eq!(attacker.order, Order::InvadeWorld);
    assert_eq!(attacker.target.to_array(), [15, 13]);
    // Re-rated by its losses, the invader carries on toward its target at
    // the full speed of the ships it has left (0 when this stalled contact
    // lost none), and is still short of it at year end.
    let [x, y] = attacker.sector.to_array();
    assert!((9..15).contains(&x) && y == 13, "{:?}", [x, y]);
    assert_eq!(attacker.speed, attacker.max_speed);

    let hostile = &game_data.fleets[4];
    assert_eq!(hostile.order, Order::SeekHome);
    // The retreat runs at the full speed of the ships it has left (0 when
    // this stalled contact lost none).
    assert_eq!(hostile.speed, hostile.max_speed);
}

#[test]
fn friendly_roe_withdrawal_aborts_invade_mission_and_records_abort_event() {
    let game_data = configured_invade_contact_state(0, 10);
    let diplomacy = mutual_enemy_overrides(1, 2);

    let (game_data, events) =
        common::first_year(&game_data, &diplomacy, |_, events| retreated(events));

    assert!(events.encounter_disposition_events.iter().any(|event| {
        matches!(
            event,
            EncounterDispositionEvent::Retreated {
                owner_empire,
                mission: Some(Mission::InvadeWorld),
                reason: nc_data::EncounterDispositionReason::RoeWithdrawal,
                ..
            } if *owner_empire == 1
        )
    }));
    assert!(events.mission_events.iter().any(|event| {
        event.owner_empire == 1
            && event.kind == Mission::InvadeWorld
            && event.outcome == MissionOutcome::Aborted
    }));

    let attacker = &game_data.fleets[0];
    assert_eq!(attacker.order, Order::SeekHome);
    // The retreat runs at full speed (0 for this stalled contact).
    assert_eq!(attacker.speed, attacker.max_speed);
}

#[test]
fn owners_guard_attacks_a_neutral_bombard_arrival() {
    let mut game_data = load_fixture("ecmaint-post");
    let target = [15, 13];

    let target_world = &mut game_data.planets[13];
    *target_world = Planet {
        sector: Coords::from_array(target),
        potential_production: 100,
        production: Real48::from_int(100),
        stored_goods: Real48::from_int(66),
        name: "TargetPrime".to_string(),
        armies: 10,
        ground_batteries: 4,
        years_under_owner: 2,
        owner: EmpireId(2),
        ..Planet::default()
    };

    let attacker = &mut game_data.fleets[0];
    attacker.owner.0 = 1;
    attacker.sector = Coords::new(14, 13);
    attacker.order = Order::BombardWorld;
    attacker.target = Coords::from_array(target);
    attacker.speed = 3;
    attacker.depth = Real48::ZERO;
    attacker.ships.destroyers = 1;
    attacker.ships.cruisers = 0;
    attacker.ships.battleships = 0;
    attacker.ships.scouts = 0;
    set_troop_transports(attacker, 0);
    set_armies_aboard(attacker, 0);
    attacker.ships.etacs = 0;
    attacker.rules_of_engagement = 0;

    let defender = &mut game_data.fleets[4];
    defender.owner.0 = 2;
    defender.sector = Coords::from_array(target);
    defender.order = Order::GuardBlockadeWorld;
    defender.target = Coords::from_array(target);
    defender.speed = 0;
    defender.ships.destroyers = 1;
    defender.ships.cruisers = 0;
    defender.ships.battleships = 0;
    defender.ships.scouts = 0;
    set_troop_transports(defender, 0);
    set_armies_aboard(defender, 0);
    defender.ships.etacs = 0;
    defender.rules_of_engagement = 6;

    // The guard sees the bombarder in its system, twenty times easier to see
    // at its target, and attacks it although the two empires are neutral.
    let (_, events) = common::first_year(&game_data, &[], common::battle);

    assert!(
        !events.fleet_battle_events.is_empty(),
        "defended-world assault posture should force a local battle"
    );
    assert!(events.fleet_battle_events.iter().any(|event| {
        event.reporting_empire == 2
            && event.perspective == nc_data::maintenance_types::FleetBattlePerspective::Intercepted
    }));
}

#[test]
fn enemy_transit_fleets_can_fight_in_deep_space() {
    let mut game_data = load_fixture("ecmaint-post");
    let coords = [8, 8];

    let left = &mut game_data.fleets[0];
    left.sector = Coords::from_array(coords);
    left.order = Order::MoveOnly;
    left.target = Coords::new(9, 8);
    left.ships.destroyers = 1;
    left.ships.cruisers = 0;
    left.ships.battleships = 0;
    left.ships.scouts = 0;
    set_troop_transports(left, 0);
    left.ships.etacs = 0;
    left.rules_of_engagement = 10;
    common::stall_in_place(left);

    let right = &mut game_data.fleets[4];
    right.sector = Coords::from_array(coords);
    right.order = Order::MoveOnly;
    right.target = Coords::new(7, 8);
    right.ships.destroyers = 1;
    right.ships.cruisers = 0;
    right.ships.battleships = 0;
    right.ships.scouts = 0;
    set_troop_transports(right, 0);
    right.ships.etacs = 0;
    right.rules_of_engagement = 10;
    common::stall_in_place(right);

    let diplomacy = mutual_enemy_overrides(1, 2);
    let (_, events) = common::first_year(&game_data, &diplomacy, common::battle);

    assert!(
        !events.fleet_battle_events.is_empty(),
        "enemy transit fleets in deep space should still fight"
    );
}

#[test]
fn anchored_guard_intercepts_declared_enemy_transit_contact() {
    let mut game_data = load_fixture("ecmaint-post");
    let coords = [16, 13];
    add_active_starbase(&mut game_data, 1, coords);

    let defender = &mut game_data.fleets[0];
    defender.sector = Coords::from_array(coords);
    defender.order = Order::GuardStarbase;
    defender.target = Coords::from_array(coords);
    defender.ships.cruisers = 1;
    defender.ships.destroyers = 0;
    defender.ships.battleships = 0;
    set_troop_transports(defender, 0);
    set_armies_aboard(defender, 0);
    defender.ships.scouts = 0;
    defender.ships.etacs = 0;
    defender.rules_of_engagement = 10;
    set_mission_target(defender, 1, 1);

    let transit = &mut game_data.fleets[4];
    transit.sector = Coords::from_array(coords);
    transit.order = Order::MoveOnly;
    transit.target = Coords::new(18, 13);
    transit.ships.destroyers = 1;
    transit.ships.cruisers = 0;
    transit.ships.battleships = 0;
    set_troop_transports(transit, 0);
    set_armies_aboard(transit, 0);
    transit.ships.scouts = 0;
    transit.ships.etacs = 0;
    transit.rules_of_engagement = 10;
    common::stall_in_place(transit);

    let diplomacy = mutual_enemy_overrides(1, 2);
    let (_, events) = common::first_year(&game_data, &diplomacy, common::battle);

    assert!(
        events
            .fleet_battle_events
            .iter()
            .any(|event| event.reporting_empire == 1 && event.enemy_empires == [2]),
        "declared enemies are intercepted wherever encountered, including a guarded system"
    );
    assert!(events.scout_contact_events.iter().any(|event| {
        event.viewer_empire == 1 && event.target_empire == 2 && event.coords == coords
    }));
}

#[test]
fn committed_patrol_engagement_can_resolve_before_any_roe_withdrawal() {
    let mut game_data = load_fixture("ecmaint-post");
    let coords = [15, 13];

    let fleet = &mut game_data.fleets[0];
    fleet.sector = Coords::from_array(coords);
    fleet.order = Order::PatrolSector;
    fleet.ships.destroyers = 0;
    fleet.ships.cruisers = 8;
    fleet.ships.battleships = 0;
    fleet.ships.scouts = 0;
    set_troop_transports(fleet, 0);
    fleet.ships.etacs = 0;
    fleet.rules_of_engagement = 6;
    common::stall_in_place(fleet);

    let hostile = &mut game_data.fleets[4];
    hostile.sector = Coords::from_array(coords);
    hostile.order = Order::MoveOnly;
    hostile.ships.destroyers = 0;
    hostile.ships.cruisers = 8;
    hostile.ships.battleships = 0;
    hostile.ships.scouts = 0;
    set_troop_transports(hostile, 0);
    hostile.ships.etacs = 0;
    hostile.rules_of_engagement = 10;
    common::stall_in_place(hostile);

    let diplomacy = mutual_enemy_overrides(1, 2);
    let (game_data, events) = common::first_year(&game_data, &diplomacy, |_, events| {
        events.encounter_disposition_events.is_empty()
            && events.fleet_battle_events.iter().any(|event| {
                event.reporting_empire == 1 && event.held_field && event.enemy_losses.cruisers == 8
            })
    });

    assert!(game_data.fleets[0].ships.cruisers > 0);
    assert_eq!(game_data.fleets[0].order, Order::PatrolSector);
    assert_eq!(game_data.fleets[0].speed, 0);
    assert_eq!(game_data.fleets[0].target.to_array(), [16, 13]);
    assert!(events.fleet_battle_events.iter().any(|event| {
        event.reporting_empire == 1 && event.enemy_losses.cruisers == 8 && event.held_field
    }));
    assert!(
        events.encounter_disposition_events.is_empty(),
        "the battle resolves during the committed rounds with no later ROE retreat"
    );
}

#[test]
fn combat_reports_enemy_fleet_by_empire_local_slot() {
    let mut game_data = load_fixture("ecmaint-post");
    let coords = [8, 8];

    let left = &mut game_data.fleets[0];
    left.sector = Coords::from_array(coords);
    left.order = Order::MoveOnly;
    left.target = Coords::new(9, 8);
    left.ships.destroyers = 1;
    left.ships.cruisers = 0;
    left.ships.battleships = 0;
    left.ships.scouts = 0;
    set_troop_transports(left, 0);
    left.ships.etacs = 0;
    left.rules_of_engagement = 10;
    common::stall_in_place(left);

    let right = &mut game_data.fleets[4];
    right.sector = Coords::from_array(coords);
    right.order = Order::MoveOnly;
    right.target = Coords::new(7, 8);
    right.fleet_number.0 = 2;
    right.ships.destroyers = 3;
    right.ships.cruisers = 0;
    right.ships.battleships = 0;
    right.ships.scouts = 0;
    set_troop_transports(right, 0);
    right.ships.etacs = 0;
    right.rules_of_engagement = 10;
    common::stall_in_place(right);
    let hostile_fleet_number = right.fleet_number.0 as u8;

    let diplomacy = mutual_enemy_overrides(1, 2);
    let events = run_maintenance_turn_with_context(&mut game_data, &[], &diplomacy)
        .expect("maintenance should succeed");

    assert!(events.scout_contact_events.iter().any(|event| {
        event.viewer_empire == 1
            && event.target_empire == 2
            && event.target_fleet_number == Some(hostile_fleet_number)
    }));
    assert!(events.fleet_battle_events.iter().any(|event| {
        event.reporting_empire == 1
            && event.primary_enemy_fleet_number == Some(hostile_fleet_number)
    }));
}

#[test]
fn committed_fire_can_reach_auxiliaries_after_the_combat_line_collapses() {
    let coords = [8, 8];
    let mut game_data = load_fixture("ecmaint-post");

    for fleet in &mut game_data.fleets {
        let current = fleet.sector.to_array();
        fleet.speed = 0;
        fleet.order = Order::HoldPosition;
        fleet.target = Coords::from_array(current);
        fleet.rules_of_engagement = 10;
        fleet.ships.destroyers = 0;
        fleet.ships.cruisers = 0;
        fleet.ships.battleships = 0;
        fleet.ships.scouts = 0;
        set_troop_transports(fleet, 0);
        set_armies_aboard(fleet, 0);
        fleet.ships.etacs = 0;
    }

    let screened = &mut game_data.fleets[0];
    screened.sector = Coords::from_array(coords);
    screened.order = Order::PatrolSector;
    screened.target = Coords::from_array(coords);
    screened.ships.cruisers = 1;
    set_troop_transports(screened, 1);
    set_armies_aboard(screened, 1);
    screened.rules_of_engagement = 0;

    let attacker = &mut game_data.fleets[4];
    attacker.sector = Coords::from_array(coords);
    attacker.order = Order::HoldPosition;
    attacker.target = Coords::from_array(coords);
    attacker.ships.destroyers = 3;
    attacker.ships.cruisers = 1;
    attacker.ships.battleships = 0;
    attacker.rules_of_engagement = 10;

    let diplomacy = mutual_enemy_overrides(1, 2);
    let (game_data, events) = common::first_year(&game_data, &diplomacy, |year, _| {
        troop_transports(&year.fleets[0]) == 0
    });

    let screened = &game_data.fleets[0];
    assert_eq!(
        screened.ships.cruisers, 0,
        "ordinary fire should strip the combat line first"
    );
    assert_eq!(
        troop_transports(screened),
        0,
        "once the cruiser screen collapses during the committed rounds, auxiliaries can be lost"
    );
    assert_eq!(screened.ships.loaded_transports, 0);
    assert!(events.fleet_battle_events.iter().any(|event| {
        event.reporting_empire == 1
            && event.friendly_losses.cruisers == 1
            && event.friendly_losses.transports == 1
    }));
    assert!(events.encounter_disposition_events.is_empty());
}

#[test]
fn canonical_three_empire_open_space_battle_resolves_deterministically() {
    fn configured_three_way_battle() -> (CoreGameData, Vec<DiplomacyOverride>) {
        let mut game_data = load_fixture("ecmaint-post");

        let fleet_a = &mut game_data.fleets[0];
        fleet_a.sector = Coords::new(15, 13);
        fleet_a.order = Order::HoldPosition;
        fleet_a.ships.destroyers = 1;
        fleet_a.ships.cruisers = 0;
        fleet_a.ships.battleships = 0;
        set_troop_transports(fleet_a, 0);
        set_armies_aboard(fleet_a, 0);
        fleet_a.ships.scouts = 0;
        fleet_a.ships.etacs = 0;
        fleet_a.rules_of_engagement = 6;

        let fleet_b = &mut game_data.fleets[4];
        fleet_b.sector = Coords::new(15, 13);
        fleet_b.order = Order::HoldPosition;
        fleet_b.ships.destroyers = 1;
        fleet_b.ships.cruisers = 0;
        fleet_b.ships.battleships = 0;
        set_troop_transports(fleet_b, 0);
        set_armies_aboard(fleet_b, 0);
        fleet_b.ships.scouts = 0;
        fleet_b.ships.etacs = 0;
        fleet_b.rules_of_engagement = 6;

        let fleet_c = &mut game_data.fleets[8];
        fleet_c.sector = Coords::new(15, 13);
        fleet_c.order = Order::HoldPosition;
        fleet_c.ships.destroyers = 0;
        fleet_c.ships.cruisers = 0;
        fleet_c.ships.battleships = 1;
        set_troop_transports(fleet_c, 0);
        set_armies_aboard(fleet_c, 0);
        fleet_c.ships.scouts = 0;
        fleet_c.ships.etacs = 0;
        fleet_c.rules_of_engagement = 6;

        let diplomacy = [
            mutual_enemy_overrides(1, 2),
            mutual_enemy_overrides(1, 3),
            mutual_enemy_overrides(2, 3),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        (game_data, diplomacy)
    }

    let (mut first, diplomacy) = configured_three_way_battle();
    let (mut second, _) = configured_three_way_battle();

    run_maintenance_turn_with_context(&mut first, &[], &diplomacy)
        .expect("first maintenance should succeed");
    run_maintenance_turn_with_context(&mut second, &[], &diplomacy)
        .expect("second maintenance should succeed");

    for idx in [0usize, 4, 8] {
        assert_eq!(
            first.fleets[idx],
            second.fleets[idx],
            "seeded combat should resolve reproducibly for fleet record {}",
            idx + 1
        );
    }
}

#[test]
fn canonical_starbase_defender_repels_orbital_attacker() {
    let mut game_data = load_fixture("ecmaint-post");
    add_active_starbase(&mut game_data, 1, [16, 13]);

    let defender = &mut game_data.fleets[0];
    defender.sector = Coords::new(16, 13);
    defender.order = Order::GuardStarbase;
    defender.target = Coords::new(16, 13);
    defender.ships.cruisers = 1;
    defender.ships.destroyers = 0;
    defender.ships.battleships = 0;
    set_troop_transports(defender, 0);
    set_armies_aboard(defender, 0);
    defender.ships.scouts = 0;
    defender.ships.etacs = 0;
    defender.rules_of_engagement = 6;

    // Blockading the world enters its solar system: a hostile intrusion.
    // (A fleet merely holding in the sector's deep space would not be.)
    let attacker = &mut game_data.fleets[4];
    attacker.sector = Coords::new(16, 13);
    attacker.order = Order::GuardBlockadeWorld;
    attacker.target = Coords::new(16, 13);
    attacker.ships.cruisers = 1;
    attacker.ships.destroyers = 0;
    attacker.ships.battleships = 0;
    set_troop_transports(attacker, 0);
    set_armies_aboard(attacker, 0);
    attacker.ships.scouts = 0;
    attacker.ships.etacs = 0;
    attacker.rules_of_engagement = 6;

    let (game_data, _) = common::first_year(&game_data, &[], common::battle);

    let attacker = &game_data.fleets[4];
    assert_eq!(attacker.ships.cruisers, 0);
    assert_eq!(attacker.order.code(), 0);
    assert_eq!(attacker.rules_of_engagement, 0);

    assert_eq!(game_data.bases.len(), 1);
    assert_eq!(game_data.bases[0].owner.0, 1);
    assert_eq!(game_data.bases[0].sector.to_array(), [16, 13]);
}

#[test]
fn starbase_only_planet_defender_cannot_concede_the_field() {
    let mut game_data = configured_assault_state(Order::BombardWorld.code());
    let coords = [15, 13];

    for fleet in game_data.fleets.iter_mut().skip(1) {
        let current = fleet.sector.to_array();
        fleet.speed = 0;
        fleet.order = Order::HoldPosition;
        fleet.target = Coords::from_array(current);
        fleet.ships.destroyers = 0;
        fleet.ships.cruisers = 0;
        fleet.ships.battleships = 0;
        fleet.ships.scouts = 0;
        set_troop_transports(fleet, 0);
        set_armies_aboard(fleet, 0);
        fleet.ships.etacs = 0;
    }

    let attacker = &mut game_data.fleets[0];
    attacker.ships.destroyers = 1;
    attacker.ships.cruisers = 0;
    attacker.ships.battleships = 0;
    attacker.rules_of_engagement = 10;

    add_active_starbase(&mut game_data, 2, coords);

    let (game_data, events) = common::first_year(&game_data, &[], common::battle);

    assert_eq!(game_data.bases.len(), 1);
    assert_eq!(game_data.bases[0].owner.0, 2);
    assert!(
        events
            .fleet_battle_events
            .iter()
            .any(|event| event.reporting_empire == 1 && !event.held_field),
        "attacker should not hold the field while the defending starbase survives"
    );
}

#[test]
fn canonical_invade_against_standing_batteries_aborts_without_retry() {
    let mut game_data = configured_assault_state(7);

    {
        let attacker = &mut game_data.fleets[0];
        set_troop_transports(attacker, 2);
        set_armies_aboard(attacker, 2);
    }
    {
        let target = &mut game_data.planets[13];
        target.armies = 10;
        target.ground_batteries = 4;
    }

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_mission_event(&events, 0, Mission::InvadeWorld, MissionOutcome::Aborted);
    let attacker = &game_data.fleets[0];
    let target = &game_data.planets[13];
    // The batteries held, so no troops landed, and their return fire
    // destroys this small force. The invasion is over: the order is Hold.
    assert!(!has_any_force(attacker));
    assert_eq!(attacker.order, Order::HoldPosition);
    assert_eq!(attacker.speed, 0);
    assert_eq!(target.owner.0, 2);
    assert_eq!(target.years_under_owner, 2);
    assert_eq!(target.armies, 10);
    // A lone destroyer's suppression fire cannot pay a battery's 20-hit DS.
    assert_eq!(target.ground_batteries, 4);

    // No retry the next year.
    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");
    assert!(events.assault_report_events.is_empty());
}

#[test]
fn failed_invasion_holds_in_orbit_without_retry() {
    let mut game_data = configured_assault_state(7);

    {
        let attacker = &mut game_data.fleets[0];
        attacker.ships.battleships = 5;
        set_troop_transports(attacker, 2);
        set_armies_aboard(attacker, 2);
    }
    {
        let target = &mut game_data.planets[13];
        target.armies = 10;
        target.ground_batteries = 5;
    }

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    // The batteries fall, the two armies land and lose, and the surviving
    // ships hold in orbit.
    assert_mission_event(&events, 0, Mission::InvadeWorld, MissionOutcome::Failed);
    let attacker = &game_data.fleets[0];
    assert!(attacker.ships.battleships > 0);
    assert_eq!(attacker.order, Order::HoldPosition);
    assert_eq!(attacker.sector.to_array(), [15, 13]);
    assert_eq!(attacker.speed, 0);
    assert_eq!(attacker.depth, nc_data::Real48::ONE);
    assert_eq!(game_data.planets[13].owner.0, 2);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");
    assert!(events.assault_report_events.is_empty());
    assert_eq!(game_data.fleets[0].order, Order::HoldPosition);
}

#[test]
fn canonical_invade_softening_caps_at_half_of_starting_armies() {
    let mut game_data = configured_assault_state(7);

    {
        let attacker = &mut game_data.fleets[0];
        attacker.ships.battleships = 20;
        attacker.ships.cruisers = 20;
        attacker.ships.destroyers = 20;
        set_troop_transports(attacker, 80);
        set_armies_aboard(attacker, 80);
    }
    {
        let target = &mut game_data.planets[13];
        target.armies = 62;
        target.ground_batteries = 0;
    }

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let target = &game_data.planets[13];
    let assault = &events.assault_report_events[0];
    assert_eq!(target.owner.0, 1);
    assert_eq!(assault.kind, Mission::InvadeWorld);
    assert_eq!(assault.outcome, MissionOutcome::Succeeded);
    assert_eq!(assault.defender_armies_initial, 62);
    assert_eq!(assault.defender_army_losses_softening, 31);
    assert_eq!(assault.defender_army_losses, 62);
}

#[test]
fn canonical_invade_softening_leaves_majority_on_odd_army_counts() {
    let mut game_data = configured_assault_state(7);

    {
        let attacker = &mut game_data.fleets[0];
        attacker.ships.battleships = 20;
        attacker.ships.cruisers = 20;
        attacker.ships.destroyers = 20;
        set_troop_transports(attacker, 20);
        set_armies_aboard(attacker, 20);
    }
    {
        let target = &mut game_data.planets[13];
        target.armies = 5;
        target.ground_batteries = 0;
    }

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let assault = &events.assault_report_events[0];
    assert_eq!(assault.kind, Mission::InvadeWorld);
    assert_eq!(assault.outcome, MissionOutcome::Succeeded);
    assert_eq!(assault.defender_armies_initial, 5);
    assert_eq!(assault.defender_army_losses_softening, 2);
    assert_eq!(assault.defender_army_losses, 5);
}

#[test]
fn canonical_blitz_success_transfers_surviving_batteries() {
    let mut game_data = configured_assault_state(8);

    {
        let attacker = &mut game_data.fleets[0];
        set_troop_transports(attacker, 40);
        set_armies_aboard(attacker, 40);
    }
    {
        let target = &mut game_data.planets[13];
        target.armies = 3;
        target.ground_batteries = 2;
    }

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let attacker = &game_data.fleets[0];
    let target = &game_data.planets[13];
    assert_eq!(attacker.order.code(), 0);
    assert_eq!(attacker.speed, 0);
    assert_eq!(attacker.ships.loaded_transports, 0);
    assert_eq!(target.owner.0, 1);
    // Just captured (ECMAINT 0xFA41 writes 0); the next pass steps it.
    assert_eq!(target.years_under_owner, 0);
    // The lone destroyer's cover fire cannot pay a battery's 20-hit DS, so
    // both batteries survive and fire 180 AS on the landing. Each transport
    // costs 20 hits, so 15 sink; 25 armies land, win, and pass the batteries
    // to the victor. The three defenders fire first on this seed and kill one
    // attacker before the rest wipe them out.
    assert_eq!(target.ground_batteries, 2);
    assert_eq!(target.armies, 24);
    assert_eq!(events.ownership_change_events.len(), 1);
    assert_eq!(events.assault_report_events.len(), 1);
    assert_eq!(events.assault_report_events[0].attacker_army_losses, 1);
    assert_eq!(events.assault_report_events[0].transport_army_losses, 15);
    assert_eq!(events.assault_report_events[0].defender_battery_losses, 0);
    assert_eq!(events.assault_report_events[0].defender_army_losses, 3);
    assert_eq!(events.ownership_change_events[0].planet_idx, 13);
    assert_eq!(events.ownership_change_events[0].previous_owner_empire, 2);
    assert_eq!(events.ownership_change_events[0].new_owner_empire, 1);
    assert!(
        events
            .planet_intel_events
            .iter()
            .any(|event| event.planet_idx == 13 && event.viewer_empire == 1)
    );
    assert!(
        events
            .planet_intel_events
            .iter()
            .any(|event| event.planet_idx == 13 && event.viewer_empire == 2)
    );
    assert!(
        events
            .planet_intel_events
            .iter()
            .all(|event| event.source == PlanetIntelSource::AssaultSuccess)
    );
    assert!(events.colonization_events.is_empty());
}

#[test]
fn canonical_blitz_failure_leaves_defender_in_control() {
    let mut game_data = configured_assault_state(8);

    {
        let attacker = &mut game_data.fleets[0];
        set_troop_transports(attacker, 4);
        set_armies_aboard(attacker, 4);
    }
    {
        let target = &mut game_data.planets[13];
        target.armies = 10;
        target.ground_batteries = 2;
    }

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let attacker = &game_data.fleets[0];
    let target = &game_data.planets[13];
    assert_eq!(attacker.order.code(), 0);
    assert_eq!(attacker.speed, 0);
    assert_eq!(attacker.ships.loaded_transports, 0);
    assert_eq!(target.owner.0, 2);
    assert_eq!(target.years_under_owner, 2);
    assert_eq!(target.armies, 10);
    // Cover fire from one destroyer cannot pay a battery's 20-hit DS, and the
    // two surviving batteries sink all four transports before they land.
    assert_eq!(target.ground_batteries, 2);
    assert_eq!(events.assault_report_events.len(), 1);
    assert_eq!(events.assault_report_events[0].attacker_army_losses, 0);
    assert_eq!(events.assault_report_events[0].transport_army_losses, 4);
    assert_eq!(events.assault_report_events[0].defender_battery_losses, 0);
    assert_eq!(events.assault_report_events[0].defender_army_losses, 0);
    assert!(
        events
            .planet_intel_events
            .iter()
            .all(|event| event.source == PlanetIntelSource::AssaultFailure)
    );

    // The blitz is over; nothing strikes again.
    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");
    assert!(events.assault_report_events.is_empty());
    assert!(
        game_data
            .fleets
            .iter()
            .all(|fleet| fleet.order != Order::BlitzWorld)
    );
}

#[test]
fn bombardment_repeats_each_year_until_the_order_changes() {
    let mut game_data = configured_assault_state(6);
    {
        let attacker = &mut game_data.fleets[0];
        attacker.ships.battleships = 20;
        attacker.ships.destroyers = 0;
    }
    game_data.planets[13].ground_batteries = 1;

    for year in 1..=2 {
        let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");
        assert_eq!(events.bombard_events.len(), 1, "year {year} bombards");
        let attacker = &game_data.fleets[0];
        assert_eq!(attacker.order, Order::BombardWorld);
        assert_eq!(attacker.sector.to_array(), [15, 13]);
        assert_eq!(attacker.speed, 0);
        assert_eq!(attacker.depth, nc_data::Real48::ONE);
    }

    game_data.fleets[0].order = Order::HoldPosition;
    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");
    assert!(events.bombard_events.is_empty());
}

#[test]
fn hostile_order_given_in_orbit_strikes_at_the_next_maintenance() {
    let mut game_data = configured_assault_state(0);
    {
        let attacker = &mut game_data.fleets[0];
        attacker.speed = 0;
        attacker.ships.battleships = 20;
        set_troop_transports(attacker, 40);
        set_armies_aboard(attacker, 40);
    }
    {
        let target = &mut game_data.planets[13];
        target.armies = 3;
        target.ground_batteries = 0;
    }
    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");
    assert!(events.assault_report_events.is_empty());

    game_data.fleets[0].order = Order::InvadeWorld;
    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");
    assert_mission_event(&events, 0, Mission::InvadeWorld, MissionOutcome::Succeeded);
    assert_eq!(game_data.planets[13].owner.0, 1);
    let attacker = &game_data.fleets[0];
    assert_eq!(attacker.order, Order::HoldPosition);
    assert_eq!(attacker.speed, 0);
}

#[test]
fn bombarding_fleet_destroyed_by_batteries_stays_destroyed() {
    let mut game_data = configured_assault_state(6);
    game_data.planets[13].ground_batteries = 20;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");
    assert_eq!(events.bombard_events.len(), 1);
    assert!(!events.fleet_destroyed_events.is_empty());
    assert!(
        game_data
            .fleets
            .first()
            .is_none_or(|fleet| fleet.owner.0 != 1 || !has_any_force(fleet))
    );

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");
    assert!(events.bombard_events.is_empty());
    assert!(
        game_data
            .fleets
            .iter()
            .all(|fleet| fleet.order != Order::BombardWorld || fleet.owner.0 != 1)
    );
}
