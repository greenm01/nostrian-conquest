mod common;

use nc_data::{
    ContactReportSource, CoreGameData, DiplomacyOverride, EncounterDispositionEvent, Mission,
    MissionOutcome, Order, PlanetIntelSource,
};
use nc_engine::{
    run_maintenance_turn, run_maintenance_turn_with_context,
    run_maintenance_turn_with_context_and_seed,
};
use std::path::Path;

fn load_fixture(name: &str) -> CoreGameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
        .join("v1.5");
    CoreGameData::load(&dir).unwrap_or_else(|e| panic!("Failed to load fixture {}: {}", name, e))
}

fn configured_assault_state(order_code: u8) -> CoreGameData {
    let mut game_data = load_fixture("ecmaint-post");

    // Use the unowned world at (15,13) as a deterministic assault target.
    let target = &mut game_data.planets.records[13];
    target.set_as_owned_target_world(
        [15, 13],
        [0x64, 0x87],
        [0x00, 0x00, 0x00, 0x00, 0x48, 0x87],
        0x04,
        0x0b,
        *b"TargetPrimeet",
        [0x05, 0x1d, 0x0b, 0x11, 0x25, 0x1c, 0x05],
        10,
        4,
        2,
        2,
    );

    // Reuse fleet 1 as the attacker and mark it as already-arrived this turn.
    let attacker = &mut game_data.fleets.records[0];
    attacker.set_current_location_coords_raw([15, 13]);
    attacker.set_standing_order_code_raw(order_code);
    attacker.set_standing_order_target_coords_raw([15, 13]);
    attacker.set_current_speed(3);
    // At rest in orbit: the strike test passes on the first tick.
    attacker.set_at_rest_motion();
    attacker.set_rules_of_engagement(10);
    attacker.set_scout_count(0);
    attacker.set_battleship_count(0);
    attacker.set_cruiser_count(0);
    attacker.set_destroyer_count(1);
    attacker.set_troop_transport_count(0);
    attacker.set_army_count(0);
    attacker.set_etac_count(0);

    game_data
}

fn configured_invade_contact_state(attacker_roe: u8, hostile_roe: u8) -> CoreGameData {
    let mut game_data = load_fixture("ecmaint-post");
    let coords = [9, 13];
    let target = [15, 13];

    let attacker = &mut game_data.fleets.records[0];
    attacker.set_current_location_coords_raw(coords);
    attacker.set_standing_order_kind(Order::InvadeWorld);
    attacker.set_standing_order_target_coords_raw(target);
    attacker.set_current_speed(0);
    attacker.set_destroyer_count(2);
    attacker.set_cruiser_count(4);
    attacker.set_battleship_count(1);
    attacker.set_scout_count(0);
    attacker.set_troop_transport_count(2);
    attacker.set_army_count(2);
    attacker.set_etac_count(0);
    attacker.set_rules_of_engagement(attacker_roe);
    common::stall_in_place(attacker);

    let hostile = &mut game_data.fleets.records[4];
    hostile.set_current_location_coords_raw(coords);
    hostile.set_standing_order_kind(Order::MoveOnly);
    hostile.set_standing_order_target_coords_raw([8, 13]);
    hostile.set_current_speed(0);
    hostile.set_destroyer_count(0);
    hostile.set_cruiser_count(2);
    hostile.set_battleship_count(0);
    hostile.set_scout_count(0);
    hostile.set_troop_transport_count(0);
    hostile.set_army_count(0);
    hostile.set_etac_count(0);
    hostile.set_rules_of_engagement(hostile_roe);
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
    let target = &mut game_data.planets.records[13];
    target.set_ground_batteries_raw(batteries);
    target.set_army_count_raw(armies);
    target.set_stored_goods_raw(stored_goods);
    target.set_factories_word_raw(factories);
    for slot in 0..nc_data::STARDOCK_SLOT_COUNT {
        target.set_stardock_count_raw(slot, 0);
        target.set_stardock_kind_raw(slot, 0);
    }
}

fn add_active_starbase(game_data: &mut CoreGameData, owner: u8, coords: [u8; 2]) {
    let mut base = nc_data::BaseRecord::new_zeroed();
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

fn mutual_enemy_overrides(left: u8, right: u8) -> [DiplomacyOverride; 2] {
    [
        DiplomacyOverride {
            from_empire_raw: left,
            to_empire_raw: right,
            relation: nc_data::DiplomaticRelation::Enemy,
        },
        DiplomacyOverride {
            from_empire_raw: right,
            to_empire_raw: left,
            relation: nc_data::DiplomaticRelation::Enemy,
        },
    ]
}

fn configured_dominant_invalidated_invade_state() -> CoreGameData {
    let mut game_data = load_fixture("ecmaint-post");
    let coords = [15, 13];

    game_data.bases.records.clear();
    for fleet in &mut game_data.fleets.records {
        let current = fleet.current_location_coords_raw();
        fleet.set_current_speed(0);
        fleet.set_standing_order_kind(Order::HoldPosition);
        fleet.set_standing_order_target_coords_raw(current);
        fleet.set_rules_of_engagement(10);
        fleet.set_destroyer_count(0);
        fleet.set_cruiser_count(0);
        fleet.set_battleship_count(0);
        fleet.set_scout_count(0);
        fleet.set_troop_transport_count(0);
        fleet.set_army_count(0);
        fleet.set_etac_count(0);
    }

    let target = &mut game_data.planets.records[13];
    target.set_as_owned_target_world(
        coords,
        [0x64, 0x87],
        [0x00, 0x00, 0x00, 0x00, 0x48, 0x87],
        0x04,
        0x0b,
        *b"TargetPrimeet",
        [0x05, 0x1d, 0x0b, 0x11, 0x25, 0x1c, 0x05],
        10,
        4,
        2,
        2,
    );

    let attacker = &mut game_data.fleets.records[0];
    attacker.set_current_location_coords_raw(coords);
    attacker.set_standing_order_kind(Order::InvadeWorld);
    attacker.set_standing_order_target_coords_raw(coords);
    attacker.set_current_speed(3);
    // At rest in orbit: the strike test passes on the first tick.
    attacker.set_at_rest_motion();
    attacker.set_destroyer_count(1);
    attacker.set_troop_transport_count(1);
    attacker.set_army_count(1);

    let defender = &mut game_data.fleets.records[4];
    defender.set_current_location_coords_raw(coords);
    defender.set_standing_order_kind(Order::HoldPosition);
    defender.set_standing_order_target_coords_raw(coords);
    defender.set_current_speed(0);
    defender.set_destroyer_count(1);

    game_data
}

#[test]
fn canonical_bombardment_consumes_order_and_devastates_target() {
    let mut game_data = load_fixture("ecmaint-bombard-arrive");

    let pre_target = game_data.planets.records[13].clone();
    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(events.bombard_events.len(), 1);
    assert_eq!(events.bombard_events[0].planet_idx, 13);
    assert_eq!(events.bombard_events[0].attacker_empire_raw, 1);
    assert!(
        events.planet_intel_events.is_empty(),
        "bombardment alone should not refresh classic planet-database intel"
    );
    assert!(events.colonization_events.is_empty());

    let attacker = &game_data.fleets.records[2];
    assert_eq!(attacker.current_location_coords_raw(), [15, 13]);
    // Bombardment persists: fleet keeps BombardWorld order for next turn.
    assert_eq!(
        attacker.standing_order_code_raw(),
        nc_data::Order::BombardWorld.to_raw()
    );
    // It keeps its pre-strike speed and depth, as ECMAINT leaves a bombard.
    assert_eq!(attacker.current_speed(), 3);
    assert_eq!(attacker.cruiser_count(), 0);
    assert_eq!(attacker.destroyer_count(), 0);

    let post_target = &game_data.planets.records[13];
    assert_eq!(
        post_target.owner_empire_slot_raw(),
        pre_target.owner_empire_slot_raw()
    );
    // Three-round bombardment: batteries absorb suppression fire in rounds 1-2.
    // Armies are only hit if batteries reach 0 before round 3 (breakthrough).
    assert!(
        post_target.ground_batteries_raw() <= pre_target.ground_batteries_raw(),
        "batteries should be damaged or destroyed"
    );
    if post_target.ground_batteries_raw() == 0 {
        // Breakthrough occurred: armies may have been reduced.
        assert!(post_target.army_count_raw() <= pre_target.army_count_raw());
    } else {
        // No breakthrough: armies are shielded.
        assert_eq!(post_target.army_count_raw(), pre_target.army_count_raw());
    }
    assert_eq!(
        game_data.player.records[0].diplomatic_relation_toward(2),
        Some(nc_data::DiplomaticRelation::Enemy)
    );
    assert_eq!(
        game_data.player.records[1].diplomatic_relation_toward(1),
        Some(nc_data::DiplomaticRelation::Enemy)
    );
}

#[test]
fn bombardment_armies_without_batteries_do_not_inflict_orbital_losses() {
    let mut game_data = configured_assault_state(Order::BombardWorld.to_raw());
    configure_bombard_target_world(&mut game_data, 0, 6, 0, 0);

    let attacker = &mut game_data.fleets.records[0];
    attacker.set_destroyer_count(0);
    attacker.set_cruiser_count(1);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(events.bombard_events.len(), 1);
    let bombard = &events.bombard_events[0];
    assert_eq!(bombard.defender_batteries_initial, 0);
    assert_eq!(bombard.defender_armies_initial, 6);
    assert_eq!(bombard.attacker_losses, nc_data::ShipLosses::default());

    let attacker = &game_data.fleets.records[0];
    assert_eq!(attacker.cruiser_count(), 1);
    assert_eq!(attacker.destroyer_count(), 0);

    let target = &game_data.planets.records[13];
    assert_eq!(target.ground_batteries_raw(), 0);
    assert!(target.army_count_raw() < 6);
}

#[test]
fn bombardment_batteries_still_inflict_orbital_losses() {
    let mut game_data = configured_assault_state(Order::BombardWorld.to_raw());
    configure_bombard_target_world(&mut game_data, 4, 0, 0, 0);

    let attacker = &mut game_data.fleets.records[0];
    attacker.set_destroyer_count(0);
    attacker.set_cruiser_count(1);

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

    let attacker = &game_data.fleets.records[0];
    assert_eq!(attacker.cruiser_count(), 0);
}

#[test]
fn foreign_contact_without_enemy_declaration_reports_but_does_not_force_battle() {
    let mut game_data = load_fixture("ecmaint-fleet-battle-pre");
    game_data.fleets.records[0].set_standing_order_kind(Order::ScoutSector);
    game_data.fleets.records[0].set_scout_count(1);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(events.fleet_battle_events.len(), 0);
    assert!(events.fleet_destroyed_events.is_empty());
    assert!(events.scout_contact_events.iter().any(|event| {
        event.viewer_empire_raw == 1
            && event.source == ContactReportSource::FleetMission(Mission::ScoutSector)
            && event.coords == [10, 10]
            && event.target_empire_raw == 2
    }));
    assert!(events.mission_events.iter().all(|event| {
        !(event.kind == Mission::ScoutSector && event.outcome == MissionOutcome::Aborted)
    }));
}

#[test]
fn declared_enemy_fleet_battle_clears_destroyed_scout_fleet_without_garbage_counts() {
    let mut game_data = load_fixture("ecmaint-fleet-battle-pre");
    game_data.fleets.records[0].set_standing_order_kind(Order::ScoutSector);
    game_data.fleets.records[0].set_scout_count(1);
    let diplomacy = mutual_enemy_overrides(1, 2);

    let events = run_maintenance_turn_with_context(&mut game_data, &[], &diplomacy)
        .expect("maintenance should succeed");

    let loser_one = &game_data.fleets.records[0];
    assert_eq!(loser_one.current_location_coords_raw(), [10, 10]);
    assert_eq!(loser_one.standing_order_code_raw(), 0);
    assert_eq!(loser_one.rules_of_engagement(), 0);
    assert_eq!(loser_one.battleship_count(), 0);
    assert_eq!(loser_one.cruiser_count(), 0);
    assert_eq!(loser_one.destroyer_count(), 0);
    assert_eq!(loser_one.troop_transport_count(), 0);
    assert_eq!(loser_one.scout_count(), 0);

    assert!(events.mission_events.iter().any(|event| {
        event.fleet_idx == 0
            && event.kind == Mission::ScoutSector
            && event.outcome == MissionOutcome::Aborted
            && event.location_coords == Some([10, 10])
    }));
    assert!(events.fleet_battle_events.iter().any(|event| {
        event.coords == [10, 10]
            && event.enemy_empires_raw.contains(&2)
            && event.reporting_empire_raw == 1
    }));
    assert!(events.scout_contact_events.iter().any(|event| {
        event.viewer_empire_raw == 1
            && event.source == ContactReportSource::FleetMission(Mission::ScoutSector)
            && event.coords == [10, 10]
            && event.target_empire_raw == 2
    }));
    assert!(events.scout_contact_events.iter().any(|event| {
        matches!(
            event.source,
            ContactReportSource::Fleet(_) | ContactReportSource::FleetMission(Mission::MoveOnly)
        ) && event.coords == [10, 10]
            && event.viewer_empire_raw != 1
    }));

    let survivor = &game_data.fleets.records[6];
    assert_eq!(survivor.current_location_coords_raw(), [10, 10]);
    assert_eq!(survivor.battleship_count(), 1);
    // The fixture's byte 0x24 = 10 is the mission id's high byte; the fleet
    // has no scouts.
    assert_eq!(survivor.scout_count(), 0);
    assert_eq!(survivor.etac_count(), 1);

    for fleet in &game_data.fleets.records {
        assert!(fleet.battleship_count() <= 100);
        assert!(fleet.cruiser_count() <= 100);
        assert!(fleet.destroyer_count() <= 100);
        assert!(fleet.troop_transport_count() <= 100);
    }
}

#[test]
fn dominant_fleet_with_invalidated_invade_order_holds_and_skips_stale_assault() {
    let mut game_data = configured_dominant_invalidated_invade_state();
    let diplomacy = mutual_enemy_overrides(1, 2);

    let events = run_maintenance_turn_with_context(&mut game_data, &[], &diplomacy)
        .expect("maintenance should succeed");

    let attacker = &game_data.fleets.records[0];
    assert_eq!(attacker.current_location_coords_raw(), [15, 13]);
    assert_eq!(attacker.standing_order_kind(), Order::HoldPosition);
    assert_eq!(attacker.current_speed(), 0);

    assert_eq!(game_data.planets.records[13].owner_empire_slot_raw(), 2);
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
        event.reporting_empire_raw == 1 && event.reporting_mission == Some(Mission::InvadeWorld)
    }));
}

#[test]
fn hostile_contact_with_roe_zero_allows_clean_sensor_retreat() {
    let mut game_data = load_fixture("ecmaint-post");
    let coords = [15, 13];

    let patrol = &mut game_data.fleets.records[0];
    patrol.set_current_location_coords_raw(coords);
    patrol.set_standing_order_kind(Order::PatrolSector);
    patrol.set_destroyer_count(1);
    patrol.set_cruiser_count(0);
    patrol.set_battleship_count(0);
    patrol.set_scout_count(0);
    patrol.set_troop_transport_count(0);
    patrol.set_etac_count(0);
    patrol.set_rules_of_engagement(0);
    common::stall_in_place(patrol);

    let hostile = &mut game_data.fleets.records[4];
    hostile.set_current_location_coords_raw(coords);
    hostile.set_standing_order_kind(Order::MoveOnly);
    hostile.set_destroyer_count(1);
    hostile.set_cruiser_count(0);
    hostile.set_battleship_count(0);
    hostile.set_scout_count(0);
    hostile.set_troop_transport_count(0);
    hostile.set_etac_count(0);
    hostile.set_rules_of_engagement(0);
    common::stall_in_place(hostile);
    let diplomacy = mutual_enemy_overrides(1, 2);
    let events = run_maintenance_turn_with_context(&mut game_data, &[], &diplomacy)
        .expect("maintenance should succeed");

    assert!(
        events.fleet_battle_events.is_empty(),
        "ROE-zero transit contact should abort before combat when the patrol already holds the sector"
    );
    assert!(events.encounter_disposition_events.iter().any(|event| {
        matches!(
            event,
            EncounterDispositionEvent::Retreated {
                owner_empire_raw,
                mission: Some(Mission::MoveOnly),
                coords: event_coords,
                target_empire_raw,
                target_fleet_number: _,
                reason: nc_data::EncounterDispositionReason::RoeWithdrawal,
                ..
            } if *owner_empire_raw == 2
                && *event_coords == coords
                && *target_empire_raw == 1
        )
    }));
    assert_eq!(
        game_data.fleets.records[0].standing_order_kind(),
        Order::PatrolSector
    );
    assert_eq!(game_data.fleets.records[0].current_speed(), 0);
    assert_eq!(
        game_data.fleets.records[4].standing_order_kind(),
        Order::SeekHome
    );
    // The retreat runs at full speed (0 for this stalled contact).
    assert_eq!(
        game_data.fleets.records[4].current_speed(),
        game_data.fleets.records[4].max_speed()
    );
}

#[test]
fn enemy_roe_withdrawal_does_not_abort_invader_mission() {
    let mut game_data = configured_invade_contact_state(10, 0);
    let diplomacy = mutual_enemy_overrides(1, 2);

    let events = run_maintenance_turn_with_context(&mut game_data, &[], &diplomacy)
        .expect("maintenance should succeed");

    assert!(
        events.encounter_disposition_events.iter().any(|event| {
            matches!(
                event,
                EncounterDispositionEvent::Retreated {
                    owner_empire_raw,
                    mission: Some(Mission::MoveOnly),
                    reason: nc_data::EncounterDispositionReason::RoeWithdrawal,
                    ..
                } if *owner_empire_raw == 2
            )
        }),
        "{events:?}"
    );
    assert!(!events.encounter_disposition_events.iter().any(|event| {
        matches!(
            event,
            EncounterDispositionEvent::Retreated {
                owner_empire_raw,
                mission: Some(Mission::InvadeWorld),
                reason: nc_data::EncounterDispositionReason::RoeWithdrawal,
                ..
            } if *owner_empire_raw == 1
        )
    }));
    assert!(!events.mission_events.iter().any(|event| {
        event.owner_empire_raw == 1
            && event.kind == Mission::InvadeWorld
            && event.outcome == MissionOutcome::Aborted
    }));

    let attacker = &game_data.fleets.records[0];
    assert_eq!(attacker.standing_order_kind(), Order::InvadeWorld);
    assert_eq!(attacker.standing_order_target_coords_raw(), [15, 13]);
    assert_eq!(attacker.current_location_coords_raw(), [9, 13]);
    assert_eq!(attacker.current_speed(), 0);

    let hostile = &game_data.fleets.records[4];
    assert_eq!(hostile.standing_order_kind(), Order::SeekHome);
    // The retreat runs at full speed (0 for this stalled contact).
    assert_eq!(hostile.current_speed(), hostile.max_speed());
}

#[test]
fn friendly_roe_withdrawal_aborts_invade_mission_and_records_abort_event() {
    let mut game_data = configured_invade_contact_state(0, 10);
    let diplomacy = mutual_enemy_overrides(1, 2);

    let events = run_maintenance_turn_with_context(&mut game_data, &[], &diplomacy)
        .expect("maintenance should succeed");

    assert!(events.encounter_disposition_events.iter().any(|event| {
        matches!(
            event,
            EncounterDispositionEvent::Retreated {
                owner_empire_raw,
                mission: Some(Mission::InvadeWorld),
                reason: nc_data::EncounterDispositionReason::RoeWithdrawal,
                ..
            } if *owner_empire_raw == 1
        )
    }));
    assert!(events.mission_events.iter().any(|event| {
        event.owner_empire_raw == 1
            && event.kind == Mission::InvadeWorld
            && event.outcome == MissionOutcome::Aborted
    }));

    let attacker = &game_data.fleets.records[0];
    assert_eq!(attacker.standing_order_kind(), Order::SeekHome);
    // The retreat runs at full speed (0 for this stalled contact).
    assert_eq!(attacker.current_speed(), attacker.max_speed());
}

#[test]
fn neutral_bombard_arrival_at_defended_world_forces_local_battle_and_escalation() {
    let mut game_data = load_fixture("ecmaint-post");
    let target = [15, 13];

    let target_world = &mut game_data.planets.records[13];
    target_world.set_as_owned_target_world(
        target,
        [0x64, 0x87],
        [0x00, 0x00, 0x00, 0x00, 0x48, 0x87],
        0x04,
        0x0b,
        *b"TargetPrimeet",
        [0x05, 0x1d, 0x0b, 0x11, 0x25, 0x1c, 0x05],
        10,
        4,
        2,
        2,
    );

    let attacker = &mut game_data.fleets.records[0];
    attacker.set_owner_empire_raw(1);
    attacker.set_current_location_coords_raw([14, 13]);
    attacker.set_standing_order_kind(Order::BombardWorld);
    attacker.set_standing_order_target_coords_raw(target);
    attacker.set_current_speed(3);
    attacker.raw[0x19] = 0x00;
    attacker.set_destroyer_count(1);
    attacker.set_cruiser_count(0);
    attacker.set_battleship_count(0);
    attacker.set_scout_count(0);
    attacker.set_troop_transport_count(0);
    attacker.set_army_count(0);
    attacker.set_etac_count(0);
    attacker.set_rules_of_engagement(0);

    let defender = &mut game_data.fleets.records[4];
    defender.set_owner_empire_raw(2);
    defender.set_current_location_coords_raw(target);
    defender.set_standing_order_kind(Order::GuardBlockadeWorld);
    defender.set_standing_order_target_coords_raw(target);
    defender.set_current_speed(0);
    defender.set_destroyer_count(1);
    defender.set_cruiser_count(0);
    defender.set_battleship_count(0);
    defender.set_scout_count(0);
    defender.set_troop_transport_count(0);
    defender.set_army_count(0);
    defender.set_etac_count(0);
    defender.set_rules_of_engagement(0);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert!(
        !events.fleet_battle_events.is_empty(),
        "defended-world assault posture should force a local battle"
    );
    assert!(
        events.bombard_events.is_empty(),
        "arrival turn should contest orbit before bombardment executes"
    );
    assert_eq!(
        game_data.player.records[0].diplomatic_relation_toward(2),
        Some(nc_data::DiplomaticRelation::Enemy)
    );
    assert_eq!(
        game_data.player.records[1].diplomatic_relation_toward(1),
        Some(nc_data::DiplomaticRelation::Enemy)
    );
}

#[test]
fn enemy_transit_fleets_can_fight_in_deep_space() {
    let mut game_data = load_fixture("ecmaint-post");
    let coords = [8, 8];

    let left = &mut game_data.fleets.records[0];
    left.set_current_location_coords_raw(coords);
    left.set_standing_order_kind(Order::MoveOnly);
    left.set_standing_order_target_coords_raw([9, 8]);
    left.set_destroyer_count(1);
    left.set_cruiser_count(0);
    left.set_battleship_count(0);
    left.set_scout_count(0);
    left.set_troop_transport_count(0);
    left.set_etac_count(0);
    left.set_rules_of_engagement(10);
    common::stall_in_place(left);

    let right = &mut game_data.fleets.records[4];
    right.set_current_location_coords_raw(coords);
    right.set_standing_order_kind(Order::MoveOnly);
    right.set_standing_order_target_coords_raw([7, 8]);
    right.set_destroyer_count(1);
    right.set_cruiser_count(0);
    right.set_battleship_count(0);
    right.set_scout_count(0);
    right.set_troop_transport_count(0);
    right.set_etac_count(0);
    right.set_rules_of_engagement(10);
    common::stall_in_place(right);

    let diplomacy = mutual_enemy_overrides(1, 2);
    let events = run_maintenance_turn_with_context(&mut game_data, &[], &diplomacy)
        .expect("maintenance should succeed");

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

    let defender = &mut game_data.fleets.records[0];
    defender.set_current_location_coords_raw(coords);
    defender.set_standing_order_kind(Order::GuardStarbase);
    defender.set_standing_order_target_coords_raw(coords);
    defender.set_cruiser_count(1);
    defender.set_destroyer_count(0);
    defender.set_battleship_count(0);
    defender.set_troop_transport_count(0);
    defender.set_army_count(0);
    defender.set_scout_count(0);
    defender.set_etac_count(0);
    defender.set_rules_of_engagement(10);
    defender.set_mission_aux_bytes([1, 1]);

    let transit = &mut game_data.fleets.records[4];
    transit.set_current_location_coords_raw(coords);
    transit.set_standing_order_kind(Order::MoveOnly);
    transit.set_standing_order_target_coords_raw([18, 13]);
    transit.set_destroyer_count(1);
    transit.set_cruiser_count(0);
    transit.set_battleship_count(0);
    transit.set_troop_transport_count(0);
    transit.set_army_count(0);
    transit.set_scout_count(0);
    transit.set_etac_count(0);
    transit.set_rules_of_engagement(10);
    common::stall_in_place(transit);

    let diplomacy = mutual_enemy_overrides(1, 2);
    let events = run_maintenance_turn_with_context(&mut game_data, &[], &diplomacy)
        .expect("maintenance should succeed");

    assert!(
        events
            .fleet_battle_events
            .iter()
            .any(|event| event.reporting_empire_raw == 1 && event.enemy_empires_raw == [2]),
        "declared enemies are intercepted wherever encountered, including a guarded system"
    );
    assert!(events.scout_contact_events.iter().any(|event| {
        event.viewer_empire_raw == 1 && event.target_empire_raw == 2 && event.coords == coords
    }));
}

#[test]
fn committed_patrol_engagement_can_resolve_before_any_roe_withdrawal() {
    let mut game_data = load_fixture("ecmaint-post");
    let coords = [15, 13];

    let fleet = &mut game_data.fleets.records[0];
    fleet.set_current_location_coords_raw(coords);
    fleet.set_standing_order_kind(Order::PatrolSector);
    fleet.set_destroyer_count(0);
    fleet.set_cruiser_count(8);
    fleet.set_battleship_count(0);
    fleet.set_scout_count(0);
    fleet.set_troop_transport_count(0);
    fleet.set_etac_count(0);
    fleet.set_rules_of_engagement(6);
    common::stall_in_place(fleet);

    let hostile = &mut game_data.fleets.records[4];
    hostile.set_current_location_coords_raw(coords);
    hostile.set_standing_order_kind(Order::MoveOnly);
    hostile.set_destroyer_count(0);
    hostile.set_cruiser_count(8);
    hostile.set_battleship_count(0);
    hostile.set_scout_count(0);
    hostile.set_troop_transport_count(0);
    hostile.set_etac_count(0);
    hostile.set_rules_of_engagement(10);
    common::stall_in_place(hostile);

    let diplomacy = mutual_enemy_overrides(1, 2);
    let events = run_maintenance_turn_with_context_and_seed(&mut game_data, 5, &[], &diplomacy)
        .expect("maintenance should succeed");

    assert_eq!(game_data.fleets.records[0].cruiser_count(), 1);
    assert_eq!(
        game_data.fleets.records[0].standing_order_kind(),
        Order::PatrolSector
    );
    assert_eq!(game_data.fleets.records[0].current_speed(), 0);
    assert_eq!(
        game_data.fleets.records[0].standing_order_target_coords_raw(),
        [16, 13]
    );
    assert!(events.fleet_battle_events.iter().any(|event| {
        event.reporting_empire_raw == 1
            && event.friendly_losses.cruisers == 7
            && event.enemy_losses.cruisers == 8
            && event.held_field
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

    let left = &mut game_data.fleets.records[0];
    left.set_current_location_coords_raw(coords);
    left.set_standing_order_kind(Order::MoveOnly);
    left.set_standing_order_target_coords_raw([9, 8]);
    left.set_destroyer_count(1);
    left.set_cruiser_count(0);
    left.set_battleship_count(0);
    left.set_scout_count(0);
    left.set_troop_transport_count(0);
    left.set_etac_count(0);
    left.set_rules_of_engagement(10);
    common::stall_in_place(left);

    let right = &mut game_data.fleets.records[4];
    right.set_current_location_coords_raw(coords);
    right.set_standing_order_kind(Order::MoveOnly);
    right.set_standing_order_target_coords_raw([7, 8]);
    right.set_local_slot_word_raw(2);
    right.set_destroyer_count(3);
    right.set_cruiser_count(0);
    right.set_battleship_count(0);
    right.set_scout_count(0);
    right.set_troop_transport_count(0);
    right.set_etac_count(0);
    right.set_rules_of_engagement(10);
    common::stall_in_place(right);
    let hostile_fleet_number = right.local_slot_word_raw() as u8;

    let diplomacy = mutual_enemy_overrides(1, 2);
    let events = run_maintenance_turn_with_context(&mut game_data, &[], &diplomacy)
        .expect("maintenance should succeed");

    assert!(events.scout_contact_events.iter().any(|event| {
        event.viewer_empire_raw == 1
            && event.target_empire_raw == 2
            && event.target_fleet_number == Some(hostile_fleet_number)
    }));
    assert!(events.fleet_battle_events.iter().any(|event| {
        event.reporting_empire_raw == 1
            && event.primary_enemy_fleet_number == Some(hostile_fleet_number)
    }));
}

#[test]
fn committed_fire_can_reach_auxiliaries_after_the_combat_line_collapses() {
    let coords = [8, 8];
    let mut game_data = load_fixture("ecmaint-post");

    for fleet in &mut game_data.fleets.records {
        let current = fleet.current_location_coords_raw();
        fleet.set_current_speed(0);
        fleet.set_standing_order_kind(Order::HoldPosition);
        fleet.set_standing_order_target_coords_raw(current);
        fleet.set_rules_of_engagement(10);
        fleet.set_destroyer_count(0);
        fleet.set_cruiser_count(0);
        fleet.set_battleship_count(0);
        fleet.set_scout_count(0);
        fleet.set_troop_transport_count(0);
        fleet.set_army_count(0);
        fleet.set_etac_count(0);
    }

    let screened = &mut game_data.fleets.records[0];
    screened.set_current_location_coords_raw(coords);
    screened.set_standing_order_kind(Order::PatrolSector);
    screened.set_standing_order_target_coords_raw(coords);
    screened.set_cruiser_count(1);
    screened.set_troop_transport_count(1);
    screened.set_army_count(1);
    screened.set_rules_of_engagement(0);

    let attacker = &mut game_data.fleets.records[4];
    attacker.set_current_location_coords_raw(coords);
    attacker.set_standing_order_kind(Order::HoldPosition);
    attacker.set_standing_order_target_coords_raw(coords);
    attacker.set_destroyer_count(3);
    attacker.set_cruiser_count(1);
    attacker.set_battleship_count(0);
    attacker.set_rules_of_engagement(10);

    let diplomacy = mutual_enemy_overrides(1, 2);
    let events = run_maintenance_turn_with_context_and_seed(&mut game_data, 5, &[], &diplomacy)
        .expect("maintenance should succeed");

    let screened = &game_data.fleets.records[0];
    assert_eq!(
        screened.cruiser_count(),
        0,
        "ordinary fire should strip the combat line first"
    );
    assert_eq!(
        screened.troop_transport_count(),
        0,
        "once the cruiser screen collapses during the committed rounds, auxiliaries can be lost"
    );
    assert_eq!(screened.army_count(), 0);
    assert!(events.fleet_battle_events.iter().any(|event| {
        event.reporting_empire_raw == 1
            && event.friendly_losses.cruisers == 1
            && event.friendly_losses.transports == 1
    }));
    assert!(events.encounter_disposition_events.is_empty());
}

#[test]
fn canonical_three_empire_open_space_battle_resolves_deterministically() {
    fn configured_three_way_battle() -> (CoreGameData, Vec<DiplomacyOverride>) {
        let mut game_data = load_fixture("ecmaint-post");

        let fleet_a = &mut game_data.fleets.records[0];
        fleet_a.set_current_location_coords_raw([15, 13]);
        fleet_a.set_standing_order_kind(Order::HoldPosition);
        fleet_a.set_destroyer_count(1);
        fleet_a.set_cruiser_count(0);
        fleet_a.set_battleship_count(0);
        fleet_a.set_troop_transport_count(0);
        fleet_a.set_army_count(0);
        fleet_a.set_scout_count(0);
        fleet_a.set_etac_count(0);
        fleet_a.set_rules_of_engagement(6);

        let fleet_b = &mut game_data.fleets.records[4];
        fleet_b.set_current_location_coords_raw([15, 13]);
        fleet_b.set_standing_order_kind(Order::HoldPosition);
        fleet_b.set_destroyer_count(1);
        fleet_b.set_cruiser_count(0);
        fleet_b.set_battleship_count(0);
        fleet_b.set_troop_transport_count(0);
        fleet_b.set_army_count(0);
        fleet_b.set_scout_count(0);
        fleet_b.set_etac_count(0);
        fleet_b.set_rules_of_engagement(6);

        let fleet_c = &mut game_data.fleets.records[8];
        fleet_c.set_current_location_coords_raw([15, 13]);
        fleet_c.set_standing_order_kind(Order::HoldPosition);
        fleet_c.set_destroyer_count(0);
        fleet_c.set_cruiser_count(0);
        fleet_c.set_battleship_count(1);
        fleet_c.set_troop_transport_count(0);
        fleet_c.set_army_count(0);
        fleet_c.set_scout_count(0);
        fleet_c.set_etac_count(0);
        fleet_c.set_rules_of_engagement(6);

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
            first.fleets.records[idx].raw,
            second.fleets.records[idx].raw,
            "seeded combat should resolve reproducibly for fleet record {}",
            idx + 1
        );
    }
}

#[test]
fn canonical_starbase_defender_repels_orbital_attacker() {
    let mut game_data = load_fixture("ecmaint-post");
    add_active_starbase(&mut game_data, 1, [16, 13]);

    let defender = &mut game_data.fleets.records[0];
    defender.set_current_location_coords_raw([16, 13]);
    defender.set_standing_order_kind(Order::GuardStarbase);
    defender.set_standing_order_target_coords_raw([16, 13]);
    defender.set_cruiser_count(1);
    defender.set_destroyer_count(0);
    defender.set_battleship_count(0);
    defender.set_troop_transport_count(0);
    defender.set_army_count(0);
    defender.set_scout_count(0);
    defender.set_etac_count(0);
    defender.set_rules_of_engagement(6);

    // Blockading the world enters its solar system: a hostile intrusion.
    // (A fleet merely holding in the sector's deep space would not be.)
    let attacker = &mut game_data.fleets.records[4];
    attacker.set_current_location_coords_raw([16, 13]);
    attacker.set_standing_order_kind(Order::GuardBlockadeWorld);
    attacker.set_standing_order_target_coords_raw([16, 13]);
    attacker.set_cruiser_count(1);
    attacker.set_destroyer_count(0);
    attacker.set_battleship_count(0);
    attacker.set_troop_transport_count(0);
    attacker.set_army_count(0);
    attacker.set_scout_count(0);
    attacker.set_etac_count(0);
    attacker.set_rules_of_engagement(6);

    run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let attacker = &game_data.fleets.records[4];
    assert_eq!(attacker.cruiser_count(), 0);
    assert_eq!(attacker.standing_order_code_raw(), 0);
    assert_eq!(attacker.rules_of_engagement(), 0);

    assert_eq!(game_data.bases.records.len(), 1);
    assert_eq!(game_data.bases.records[0].owner_empire_raw(), 1);
    assert_eq!(game_data.bases.records[0].coords_raw(), [16, 13]);
}

#[test]
fn starbase_only_planet_defender_cannot_concede_the_field() {
    let mut game_data = configured_assault_state(Order::BombardWorld.to_raw());
    let coords = [15, 13];

    for fleet in game_data.fleets.records.iter_mut().skip(1) {
        let current = fleet.current_location_coords_raw();
        fleet.set_current_speed(0);
        fleet.set_standing_order_kind(Order::HoldPosition);
        fleet.set_standing_order_target_coords_raw(current);
        fleet.set_destroyer_count(0);
        fleet.set_cruiser_count(0);
        fleet.set_battleship_count(0);
        fleet.set_scout_count(0);
        fleet.set_troop_transport_count(0);
        fleet.set_army_count(0);
        fleet.set_etac_count(0);
    }

    let attacker = &mut game_data.fleets.records[0];
    attacker.set_destroyer_count(1);
    attacker.set_cruiser_count(0);
    attacker.set_battleship_count(0);
    attacker.set_rules_of_engagement(10);

    add_active_starbase(&mut game_data, 2, coords);

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    assert_eq!(game_data.bases.records.len(), 1);
    assert_eq!(game_data.bases.records[0].owner_empire_raw(), 2);
    assert!(
        events
            .fleet_battle_events
            .iter()
            .any(|event| event.reporting_empire_raw == 1 && !event.held_field),
        "attacker should not hold the field while the defending starbase survives"
    );
}

#[test]
fn canonical_invade_against_standing_batteries_retries_next_year() {
    let mut game_data = configured_assault_state(7);

    {
        let attacker = &mut game_data.fleets.records[0];
        attacker.set_troop_transport_count(2);
        attacker.set_army_count(2);
    }
    {
        let target = &mut game_data.planets.records[13];
        target.set_army_count_raw(10);
        target.set_ground_batteries_raw(4);
    }

    run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let attacker = &game_data.fleets.records[0];
    let target = &game_data.planets.records[13];
    // The batteries held, so no troops landed: a failed attack keeps the
    // order and speed to strike again next year (the battery return fire
    // destroys this small force, which the next turn culls).
    assert_eq!(attacker.standing_order_code_raw(), 7);
    assert_eq!(attacker.current_speed(), 3);
    assert_eq!(target.owner_empire_slot_raw(), 2);
    assert_eq!(target.ownership_status_raw(), 2);
    assert_eq!(target.army_count_raw(), 10);
    // A lone destroyer's suppression fire cannot pay a battery's 20-hit DS.
    assert_eq!(target.ground_batteries_raw(), 4);
}

#[test]
fn canonical_invade_softening_caps_at_half_of_starting_armies() {
    let mut game_data = configured_assault_state(7);

    {
        let attacker = &mut game_data.fleets.records[0];
        attacker.set_battleship_count(20);
        attacker.set_cruiser_count(20);
        attacker.set_destroyer_count(20);
        attacker.set_troop_transport_count(80);
        attacker.set_army_count(80);
    }
    {
        let target = &mut game_data.planets.records[13];
        target.set_army_count_raw(62);
        target.set_ground_batteries_raw(0);
    }

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let target = &game_data.planets.records[13];
    let assault = &events.assault_report_events[0];
    assert_eq!(target.owner_empire_slot_raw(), 1);
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
        let attacker = &mut game_data.fleets.records[0];
        attacker.set_battleship_count(20);
        attacker.set_cruiser_count(20);
        attacker.set_destroyer_count(20);
        attacker.set_troop_transport_count(20);
        attacker.set_army_count(20);
    }
    {
        let target = &mut game_data.planets.records[13];
        target.set_army_count_raw(5);
        target.set_ground_batteries_raw(0);
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
        let attacker = &mut game_data.fleets.records[0];
        attacker.set_troop_transport_count(40);
        attacker.set_army_count(40);
    }
    {
        let target = &mut game_data.planets.records[13];
        target.set_army_count_raw(3);
        target.set_ground_batteries_raw(2);
    }

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let attacker = &game_data.fleets.records[0];
    let target = &game_data.planets.records[13];
    assert_eq!(attacker.standing_order_code_raw(), 0);
    assert_eq!(attacker.current_speed(), 0);
    assert_eq!(attacker.army_count(), 0);
    assert_eq!(target.owner_empire_slot_raw(), 1);
    assert_eq!(target.ownership_status_raw(), 2);
    // The lone destroyer's cover fire cannot pay a battery's 20-hit DS, so
    // both batteries survive and fire 180 AS on the landing. Each transport
    // costs 20 hits, so 15 sink; 25 armies land, win, and pass the batteries
    // to the victor.
    assert_eq!(target.ground_batteries_raw(), 2);
    assert_eq!(target.army_count_raw(), 22);
    assert_eq!(events.ownership_change_events.len(), 1);
    assert_eq!(events.assault_report_events.len(), 1);
    assert_eq!(events.assault_report_events[0].attacker_army_losses, 3);
    assert_eq!(events.assault_report_events[0].transport_army_losses, 15);
    assert_eq!(events.assault_report_events[0].defender_battery_losses, 0);
    assert_eq!(events.assault_report_events[0].defender_army_losses, 3);
    assert_eq!(events.ownership_change_events[0].planet_idx, 13);
    assert_eq!(
        events.ownership_change_events[0].previous_owner_empire_raw,
        2
    );
    assert_eq!(events.ownership_change_events[0].new_owner_empire_raw, 1);
    assert!(
        events
            .planet_intel_events
            .iter()
            .any(|event| event.planet_idx == 13 && event.viewer_empire_raw == 1)
    );
    assert!(
        events
            .planet_intel_events
            .iter()
            .any(|event| event.planet_idx == 13 && event.viewer_empire_raw == 2)
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
        let attacker = &mut game_data.fleets.records[0];
        attacker.set_troop_transport_count(4);
        attacker.set_army_count(4);
    }
    {
        let target = &mut game_data.planets.records[13];
        target.set_army_count_raw(10);
        target.set_ground_batteries_raw(2);
    }

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");

    let attacker = &game_data.fleets.records[0];
    let target = &game_data.planets.records[13];
    assert_eq!(attacker.standing_order_code_raw(), 0);
    assert_eq!(attacker.current_speed(), 0);
    assert_eq!(attacker.army_count(), 0);
    assert_eq!(target.owner_empire_slot_raw(), 2);
    assert_eq!(target.ownership_status_raw(), 2);
    assert_eq!(target.army_count_raw(), 10);
    // Cover fire from one destroyer cannot pay a battery's 20-hit DS, and the
    // two surviving batteries sink all four transports before they land.
    assert_eq!(target.ground_batteries_raw(), 2);
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
}

#[test]
fn guard_blockade_does_not_intercept_transit_fleet() {
    // The ROE-2 transit fleet's sensor check sees the guard and breaks off
    // cleanly before any fire is exchanged.
    let mut game_data = load_fixture("ecmaint-post");
    let coords = [8, 8]; // Deep space, not at a planet

    // Guard fleet - empire 1 on Guard/Blockade mission
    let guard = &mut game_data.fleets.records[0];
    guard.set_current_location_coords_raw(coords);
    guard.set_standing_order_kind(Order::GuardBlockadeWorld);
    guard.set_standing_order_target_coords_raw(coords);
    guard.set_destroyer_count(2); // 2 AS
    guard.set_cruiser_count(0);
    guard.set_battleship_count(0);
    guard.set_rules_of_engagement(10); // Always fight

    // Transit fleet - empire 2 just moving through with a cautious ROE
    let transit = &mut game_data.fleets.records[4];
    transit.set_current_location_coords_raw(coords);
    transit.set_standing_order_kind(Order::MoveOnly);
    transit.set_standing_order_target_coords_raw([9, 8]);
    transit.set_destroyer_count(0);
    transit.set_cruiser_count(1); // 3 AS
    transit.set_battleship_count(0);
    transit.set_rules_of_engagement(2); // ROE=2 requires 3:1

    let diplomacy = mutual_enemy_overrides(1, 2);
    let events = run_maintenance_turn_with_context(&mut game_data, &[], &diplomacy)
        .expect("maintenance should succeed");

    // The transit fleet withdrew before contact, so no battle was fought.
    assert!(
        events.fleet_battle_events.is_empty(),
        "cautious transit fleet should break off before the guard engages"
    );

    // No pursuit fire or engagement events
    let pursuit_events: Vec<_> = events
        .encounter_disposition_events
        .iter()
        .filter(|e| matches!(e, EncounterDispositionEvent::PursuitFire { .. }))
        .collect();
    assert!(
        pursuit_events.is_empty(),
        "no pursuit fire for transit fleet"
    );
}
