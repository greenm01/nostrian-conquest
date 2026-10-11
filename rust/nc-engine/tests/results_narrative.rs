use nc_compat::load_classic_core_files;
use nc_data::{
    AssaultReportEvent, BombardEvent, ContactReportSource, Coords, EmpireEliminationCause,
    EmpireEliminationEvent, EncounterDispositionEvent, EncounterDispositionReason,
    FleetBattleEvent, FleetDestroyedEvent, FleetOrderValidationError, GameStateBuilder,
    GameVictoryNoticeEvent, InvalidPlayerStateEvent, MaintenanceEvents, Mission, MissionEvent,
    MissionOutcome, Planet, PlanetIntelEvent, PlanetIntelSource, PlanetOwnershipChangeEvent,
    Real48, ScoutContactEvent, ShipLosses, StarbaseAlert, StarbaseSightingEvent, set_planet_name,
};
use nc_engine::{
    apply_results_reviewable_flags, build_results_report_blocks, maint::FleetBattlePerspective,
    run_maintenance_turn,
};
use std::path::Path;

fn viewer_report_texts(viewer_empire_id: u8, rows: &[nc_data::ReportBlockRow]) -> Vec<String> {
    rows.iter()
        .filter(|row| row.viewer_empire_id == viewer_empire_id)
        .map(|row| row.decoded_text.clone())
        .collect()
}

fn seed_target_world(game_data: &mut nc_data::CoreGameData, coords: [u8; 2], name: &str) {
    let mut planet = Planet {
        sector: Coords::from_array(coords),
        ..Planet::default()
    };
    set_planet_name(&mut planet, name);
    planet.owner.0 = 1;
    planet.years_under_owner = 2;
    planet.potential_production = 100;
    game_data.planets[0] = planet;
}

fn load_fixture(name: &str) -> nc_data::CoreGameData {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(name)
        .join("v1.5");
    load_classic_core_files(&dir).unwrap_or_else(|e| panic!("Failed to load fixture {name}: {e}"))
}

#[test]
fn results_reports_contact_before_destroyed_fleet_notice() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3001)
        .build_initialized_baseline()
        .expect("baseline should build");
    let coords = [5, 13];
    seed_target_world(&mut game_data, coords, "Target");

    let mut events = MaintenanceEvents::default();
    events.scout_contact_events.push(ScoutContactEvent {
        viewer_empire_raw: 1,
        source: ContactReportSource::FleetMission(Mission::ScoutSector),
        reporting_fleet_number: Some(15),
        reporting_initial: ShipLosses {
            destroyers: 1,
            ..ShipLosses::default()
        },
        reporting_loaded_armies_initial: 0,
        coords,
        target_empire_raw: 3,
        target_fleet_number: Some(4),
        small_vessels: 2,
        medium_vessels: 2,
        large_vessels: 0,
        stardate_week: Some(2),
    });
    events.fleet_destroyed_events.push(FleetDestroyedEvent {
        reporting_empire_raw: 1,
        fleet_number: 15,
        coords,
        was_intercepting: true,
        friendly_initial: ShipLosses {
            destroyers: 1,
            ..ShipLosses::default()
        },
        friendly_loaded_armies_initial: 0,
        enemy_initial: ShipLosses {
            cruisers: 2,
            destroyers: 2,
            etacs: 2,
            ..ShipLosses::default()
        },
        enemy_initial_starbases: 0,
        enemy_loaded_armies_initial: 0,
        enemy_ground_batteries_initial: 0,
        enemy_ground_armies_initial: 0,
        enemy_losses: ShipLosses::default(),
        enemy_starbases_destroyed: 0,
        enemy_ground_battery_losses: 0,
        enemy_ground_army_losses: 0,
        primary_enemy_empire_raw: Some(3),
        primary_enemy_fleet_number: Some(4),
        stardate_week: Some(2),
    });

    let rows = build_results_report_blocks(&mut game_data, &events);
    let texts = viewer_report_texts(1, &rows);
    let contact_idx = texts
        .iter()
        .position(|text| text.contains("Sensor contact"))
        .expect("merged contact report should exist");
    let destroyed_idx = texts
        .iter()
        .position(|text| text.contains("ALERT: Fleet contact lost!"))
        .expect("lost-contact report should exist");
    assert!(
        contact_idx < destroyed_idx,
        "contact should precede lost-contact: {texts:?}"
    );
}

#[test]
fn results_reports_battle_before_bombard_aftermath() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3001)
        .build_initialized_baseline()
        .expect("baseline should build");
    let coords = [5, 13];
    seed_target_world(&mut game_data, coords, "Target");

    let mut events = MaintenanceEvents::default();
    events.scout_contact_events.push(ScoutContactEvent {
        viewer_empire_raw: 1,
        source: ContactReportSource::FleetMission(Mission::GuardBlockadeWorld),
        reporting_fleet_number: Some(7),
        reporting_initial: ShipLosses {
            cruisers: 2,
            destroyers: 1,
            ..ShipLosses::default()
        },
        reporting_loaded_armies_initial: 0,
        coords,
        target_empire_raw: 2,
        target_fleet_number: Some(9),
        small_vessels: 1,
        medium_vessels: 1,
        large_vessels: 0,
        stardate_week: Some(2),
    });
    events.fleet_battle_events.push(FleetBattleEvent {
        reporting_empire_raw: 1,
        reporting_fleet_number: Some(7),
        reporting_mission: Some(Mission::GuardBlockadeWorld),
        perspective: FleetBattlePerspective::Intercepted,
        coords,
        enemy_empires_raw: vec![2],
        primary_enemy_fleet_number: Some(9),
        held_field: true,
        friendly_initial: ShipLosses {
            cruisers: 2,
            destroyers: 1,
            ..ShipLosses::default()
        },
        friendly_initial_starbases: 0,
        friendly_loaded_armies_initial: 0,
        friendly_losses: ShipLosses::default(),
        friendly_starbases_lost: 0,
        enemy_initial: ShipLosses {
            destroyers: 1,
            ..ShipLosses::default()
        },
        enemy_initial_starbases: 0,
        enemy_loaded_armies_initial: 0,
        enemy_losses: ShipLosses {
            destroyers: 1,
            ..ShipLosses::default()
        },
        enemy_starbases_destroyed: 0,
        stardate_week: Some(2),
    });
    events.bombard_events.push(BombardEvent {
        planet_idx: 0,
        attacker_empire_raw: 2,
        attacker_fleet_number: Some(9),
        defender_empire_raw: 1,
        attacker_initial: ShipLosses {
            destroyers: 1,
            ..ShipLosses::default()
        },
        attacker_loaded_armies_initial: 0,
        defender_batteries_initial: 4,
        defender_armies_initial: 10,
        attacker_losses: ShipLosses::default(),
        defender_battery_losses: 2,
        defender_army_losses: 3,
        breakthrough: true,
        docked_losses: nc_data::EmpireUnitSummary::default(),
        stardock_items_destroyed: 0,
        stored_goods_destroyed: 0,
        factories_destroyed: 0,
        stardate_week: Some(2),
    });

    let rows = build_results_report_blocks(&mut game_data, &events);
    let texts = viewer_report_texts(1, &rows);
    let contact_idx = texts
        .iter()
        .position(|text| text.contains("Sensor contact"))
        .expect("merged contact report should exist");
    let battle_idx = texts
        .iter()
        .position(|text| text.contains("ALERT: Enemy fleet contact!"))
        .expect("battle report should exist");
    let bombard_idx = texts
        .iter()
        .position(|text| text.contains("ALERT: Orbital bombardment underway!"))
        .expect("bombard report should exist");
    assert!(
        contact_idx < battle_idx,
        "contact should precede battle: {texts:?}"
    );
    assert!(
        battle_idx < bombard_idx,
        "battle should precede bombard: {texts:?}"
    );
}

#[test]
fn destroyed_reporting_fleet_uses_telemetry_report_even_if_side_holds_field() {
    let game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3014)
        .build_initialized_baseline()
        .expect("baseline should build");

    let mut events = MaintenanceEvents::default();
    events.fleet_battle_events.push(FleetBattleEvent {
        reporting_empire_raw: 1,
        reporting_fleet_number: Some(15),
        reporting_mission: Some(Mission::GuardBlockadeWorld),
        perspective: nc_engine::maint::FleetBattlePerspective::Intercepted,
        coords: [9, 6],
        enemy_empires_raw: vec![2],
        primary_enemy_fleet_number: None,
        held_field: true,
        friendly_initial: ShipLosses {
            battleships: 1,
            ..ShipLosses::default()
        },
        friendly_initial_starbases: 0,
        friendly_loaded_armies_initial: 0,
        friendly_losses: ShipLosses {
            battleships: 1,
            ..ShipLosses::default()
        },
        friendly_starbases_lost: 0,
        enemy_initial: ShipLosses {
            cruisers: 2,
            ..ShipLosses::default()
        },
        enemy_initial_starbases: 0,
        enemy_loaded_armies_initial: 0,
        enemy_losses: ShipLosses {
            cruisers: 2,
            ..ShipLosses::default()
        },
        enemy_starbases_destroyed: 0,
        stardate_week: Some(2),
    });

    let rows = build_results_report_blocks(&game_data, &events);
    let text = viewer_report_texts(1, &rows).join(" ");
    assert!(text.contains("From your Fleet Command Center:"));
    assert!(text.contains("ALERT: Fleet contact lost!"));
    assert!(!text.contains("From your 15th Fleet"));
    assert!(!text.contains("Interception successful."));
}

#[test]
fn bombardment_defender_report_uses_first_person_loss_wording() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3001)
        .build_initialized_baseline()
        .expect("baseline should build");
    let coords = [5, 13];
    seed_target_world(&mut game_data, coords, "Target");

    let mut events = MaintenanceEvents::default();
    events.bombard_events.push(BombardEvent {
        planet_idx: 0,
        attacker_empire_raw: 2,
        attacker_fleet_number: Some(9),
        defender_empire_raw: 1,
        attacker_initial: ShipLosses {
            destroyers: 1,
            ..ShipLosses::default()
        },
        attacker_loaded_armies_initial: 0,
        defender_batteries_initial: 0,
        defender_armies_initial: 9,
        attacker_losses: ShipLosses::default(),
        defender_battery_losses: 0,
        defender_army_losses: 3,
        breakthrough: true,
        docked_losses: nc_data::EmpireUnitSummary::default(),
        stardock_items_destroyed: 2,
        stored_goods_destroyed: 0,
        factories_destroyed: 0,
        stardate_week: Some(3),
    });

    let rows = build_results_report_blocks(&game_data, &events);
    let bombard = viewer_report_texts(1, &rows).join(" ").replace('\n', " ");

    assert!(bombard.contains("ALERT: Orbital bombardment underway!"));
    assert!(bombard.contains("Attacker:"));
    assert!(bombard.contains("The 9th Fleet of"));
    assert!(bombard.contains("Attacking force:"));
    assert!(bombard.contains("1DD"));
    assert!(bombard.contains("Our defenses:"));
    assert!(bombard.contains("9 armies"));
    assert!(!bombard.contains("0 ground battery(ies)"));
    assert!(bombard.contains("Defensive losses: 3 armies."));
    assert!(bombard.contains("Local damage:"));
    assert!(bombard.contains("2 stardock items"));
    assert!(!bombard.contains("Bombardment also destroyed"));
}

#[test]
fn blitz_report_distinguishes_total_army_losses_from_transport_losses() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3014)
        .build_initialized_baseline()
        .expect("baseline should build");
    let coords = [8, 2];
    seed_target_world(&mut game_data, coords, "half");
    game_data.planets[0].ground_batteries = 2;
    game_data.planets[0].armies = 5;

    let mut events = MaintenanceEvents::default();
    events.assault_report_events.push(AssaultReportEvent {
        kind: Mission::BlitzWorld,
        attacker_fleet_number: Some(4),
        planet_idx: 0,
        attacker_empire_raw: 1,
        defender_empire_raw: 2,
        attacker_initial: ShipLosses {
            cruisers: 1,
            transports: 3,
            ..ShipLosses::default()
        },
        attacker_loaded_armies_initial: 3,
        defender_batteries_initial: 2,
        defender_armies_initial: 5,
        attacker_ship_losses: ShipLosses::default(),
        attacker_army_losses: 3,
        transport_army_losses: 0,
        defender_battery_losses: 2,
        defender_army_losses_softening: 0,
        defender_army_losses: 5,
        outcome: MissionOutcome::Succeeded,
        stardate_week: Some(3),
    });

    let rows = build_results_report_blocks(&game_data, &events);
    let blitz = viewer_report_texts(1, &rows).join(" ").replace('\n', " ");

    assert!(blitz.contains("From your 4th Fleet"));
    assert!(blitz.contains("Our forces:"));
    assert!(blitz.contains("1CA"));
    assert!(blitz.contains("Our losses:"));
    assert!(blitz.contains("3 armies"));
    assert!(blitz.contains("Enemy losses:"));
    assert!(blitz.contains("2 ground batteries and 5 armies"));
    assert!(blitz.contains("Transport losses:"));
    assert!(blitz.contains("none in destroyed transports"));
    assert!(!blitz.contains("No troops were lost during the landing."));
}

#[test]
fn blitz_report_includes_transport_and_ground_army_losses_in_total_losses() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3014)
        .build_initialized_baseline()
        .expect("baseline should build");
    let coords = [8, 2];
    seed_target_world(&mut game_data, coords, "half");
    game_data.planets[0].ground_batteries = 2;
    game_data.planets[0].armies = 5;

    let mut events = MaintenanceEvents::default();
    events.assault_report_events.push(AssaultReportEvent {
        kind: Mission::BlitzWorld,
        attacker_fleet_number: Some(4),
        planet_idx: 0,
        attacker_empire_raw: 1,
        defender_empire_raw: 2,
        attacker_initial: ShipLosses {
            cruisers: 1,
            transports: 2,
            ..ShipLosses::default()
        },
        attacker_loaded_armies_initial: 6,
        defender_batteries_initial: 2,
        defender_armies_initial: 5,
        attacker_ship_losses: ShipLosses::default(),
        attacker_army_losses: 3,
        transport_army_losses: 2,
        defender_battery_losses: 2,
        defender_army_losses_softening: 0,
        defender_army_losses: 5,
        outcome: MissionOutcome::Succeeded,
        stardate_week: Some(3),
    });

    let rows = build_results_report_blocks(&game_data, &events);
    let blitz = viewer_report_texts(1, &rows).join(" ").replace('\n', " ");

    assert!(blitz.contains("Our losses:"));
    assert!(blitz.contains("5 armies"));
    assert!(blitz.contains("Transport losses:"));
    assert!(blitz.contains("2 troop(s) lost in destroyed transports"));
}

#[test]
fn blitz_report_for_undefended_world_includes_attacker_force_and_no_battery_text() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3017)
        .build_initialized_baseline()
        .expect("baseline should build");
    let coords = [6, 7];
    seed_target_world(&mut game_data, coords, "dog");

    let mut events = MaintenanceEvents::default();
    events.assault_report_events.push(AssaultReportEvent {
        kind: Mission::BlitzWorld,
        attacker_fleet_number: Some(10),
        planet_idx: 0,
        attacker_empire_raw: 1,
        defender_empire_raw: 0,
        attacker_initial: ShipLosses {
            cruisers: 2,
            transports: 2,
            ..ShipLosses::default()
        },
        attacker_loaded_armies_initial: 2,
        defender_batteries_initial: 0,
        defender_armies_initial: 0,
        attacker_ship_losses: ShipLosses::default(),
        attacker_army_losses: 0,
        transport_army_losses: 0,
        defender_battery_losses: 0,
        defender_army_losses_softening: 0,
        defender_army_losses: 0,
        outcome: MissionOutcome::Succeeded,
        stardate_week: Some(3),
    });

    let rows = build_results_report_blocks(&game_data, &events);
    let blitz = viewer_report_texts(1, &rows).join(" ").replace('\n', " ");

    assert!(blitz.contains("ALERT: Blitz assault successful!"));
    assert!(blitz.contains("We have seized planet \"dog\" in a fast assault."));
    assert!(blitz.contains("Our forces:"));
    assert!(blitz.contains("2CA, 2TT*"));
    assert!(blitz.contains("World defenses:"));
    assert!(blitz.contains("undefended"));
    assert!(!blitz.contains("failed to suppress the defending batteries"));
    assert!(!blitz.contains("suppressed 0 ground batteries"));
    assert!(blitz.contains("Enemy losses:"));
    assert!(blitz.contains("none"));
}

#[test]
fn invasion_report_includes_attacker_force_and_undefended_world_wording() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3017)
        .build_initialized_baseline()
        .expect("baseline should build");
    let coords = [6, 7];
    seed_target_world(&mut game_data, coords, "dog");

    let mut events = MaintenanceEvents::default();
    events.assault_report_events.push(AssaultReportEvent {
        kind: Mission::InvadeWorld,
        attacker_fleet_number: Some(10),
        planet_idx: 0,
        attacker_empire_raw: 1,
        defender_empire_raw: 0,
        attacker_initial: ShipLosses {
            battleships: 1,
            transports: 2,
            ..ShipLosses::default()
        },
        attacker_loaded_armies_initial: 2,
        defender_batteries_initial: 0,
        defender_armies_initial: 0,
        attacker_ship_losses: ShipLosses::default(),
        attacker_army_losses: 0,
        transport_army_losses: 0,
        defender_battery_losses: 0,
        defender_army_losses_softening: 0,
        defender_army_losses: 0,
        outcome: MissionOutcome::Succeeded,
        stardate_week: Some(3),
    });

    let rows = build_results_report_blocks(&game_data, &events);
    let invasion = viewer_report_texts(1, &rows).join(" ").replace('\n', " ");

    assert!(invasion.contains("ALERT: Planetary invasion successful!"));
    assert!(!invasion.contains("Our armies have captured planet \"dog\"."));
    assert!(invasion.contains("Our forces:"));
    assert!(invasion.contains("1BB, 2TT*"));
    assert!(invasion.contains("World defenses:"));
    assert!(invasion.contains("undefended"));
    assert!(invasion.contains("Enemy losses:"));
    assert!(invasion.contains("none"));
    assert!(!invasion.contains("Orbital softening losses:"));
    assert!(!invasion.contains("Ground battle losses:"));
}

#[test]
fn invasion_report_lists_ship_and_ground_army_losses() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3017)
        .build_initialized_baseline()
        .expect("baseline should build");
    let coords = [6, 7];
    seed_target_world(&mut game_data, coords, "red");

    let mut events = MaintenanceEvents::default();
    events.assault_report_events.push(AssaultReportEvent {
        kind: Mission::InvadeWorld,
        attacker_fleet_number: Some(8),
        planet_idx: 0,
        attacker_empire_raw: 1,
        defender_empire_raw: 2,
        attacker_initial: ShipLosses {
            battleships: 2,
            cruisers: 25,
            transports: 33,
            ..ShipLosses::default()
        },
        attacker_loaded_armies_initial: 33,
        defender_batteries_initial: 10,
        defender_armies_initial: 34,
        attacker_ship_losses: ShipLosses {
            cruisers: 15,
            ..ShipLosses::default()
        },
        attacker_army_losses: 7,
        transport_army_losses: 0,
        defender_battery_losses: 10,
        defender_army_losses_softening: 17,
        defender_army_losses: 34,
        outcome: MissionOutcome::Succeeded,
        stardate_week: Some(3),
    });

    let rows = build_results_report_blocks(&game_data, &events);
    let invasion = viewer_report_texts(1, &rows).join(" ").replace('\n', " ");

    assert!(invasion.contains("ALERT: Planetary invasion successful!"));
    assert!(!invasion.contains("Our armies have captured planet \"red\"."));
    assert!(invasion.contains("Our losses:"));
    assert!(invasion.contains("15CA and 7 armies"));
    assert!(invasion.contains("Enemy losses:"));
    assert!(invasion.contains("10 ground batteries and 34 armies"));
    assert!(invasion.contains("Enemy orbital softening losses:"));
    assert!(invasion.contains("Enemy orbital softening losses: 17 armies"));
    assert!(invasion.contains("17 armies"));
    assert!(invasion.contains("Enemy ground battle losses:"));
    assert!(invasion.contains("Enemy ground battle losses: 17 armies"));
}

#[test]
fn defender_receives_repulsed_invasion_report() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3017)
        .build_initialized_baseline()
        .expect("baseline should build");
    let coords = [6, 7];
    seed_target_world(&mut game_data, coords, "red");
    game_data.planets[0].owner.0 = 2;

    let mut events = MaintenanceEvents::default();
    events.assault_report_events.push(AssaultReportEvent {
        kind: Mission::InvadeWorld,
        attacker_fleet_number: Some(8),
        planet_idx: 0,
        attacker_empire_raw: 1,
        defender_empire_raw: 2,
        attacker_initial: ShipLosses {
            battleships: 2,
            transports: 12,
            ..ShipLosses::default()
        },
        attacker_loaded_armies_initial: 12,
        defender_batteries_initial: 4,
        defender_armies_initial: 16,
        attacker_ship_losses: ShipLosses {
            cruisers: 1,
            ..ShipLosses::default()
        },
        attacker_army_losses: 5,
        transport_army_losses: 0,
        defender_battery_losses: 4,
        defender_army_losses_softening: 8,
        defender_army_losses: 13,
        outcome: MissionOutcome::Failed,
        stardate_week: Some(3),
    });

    let invasion = viewer_report_texts(2, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");

    assert!(invasion.contains("ALERT: Enemy invasion repulsed!"));
    assert!(invasion.contains("Attacking force:"));
    assert!(invasion.contains("2BB, 12TT*"));
    assert!(invasion.contains("Our defenses:"));
    assert!(invasion.contains("4 ground batteries and 16 armies"));
    assert!(invasion.contains("The invaders were thrown back."));
    assert!(invasion.contains("Defensive losses: 4 ground batteries and 13 armies."));
    assert!(invasion.contains("Enemy losses:"));
    assert!(invasion.contains("1CA and 5 armies"));
    assert!(invasion.contains("Our orbital softening losses: 8 armies"));
    assert!(invasion.contains("Our ground battle losses: 5 armies"));
}

#[test]
fn defender_receives_blocked_invasion_report() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3017)
        .build_initialized_baseline()
        .expect("baseline should build");
    let coords = [6, 7];
    seed_target_world(&mut game_data, coords, "red");
    game_data.planets[0].owner.0 = 2;

    let mut events = MaintenanceEvents::default();
    events.assault_report_events.push(AssaultReportEvent {
        kind: Mission::InvadeWorld,
        attacker_fleet_number: Some(8),
        planet_idx: 0,
        attacker_empire_raw: 1,
        defender_empire_raw: 2,
        attacker_initial: ShipLosses {
            destroyers: 4,
            transports: 8,
            ..ShipLosses::default()
        },
        attacker_loaded_armies_initial: 8,
        defender_batteries_initial: 6,
        defender_armies_initial: 20,
        attacker_ship_losses: ShipLosses {
            destroyers: 1,
            ..ShipLosses::default()
        },
        attacker_army_losses: 8,
        transport_army_losses: 0,
        defender_battery_losses: 3,
        defender_army_losses_softening: 0,
        defender_army_losses: 0,
        outcome: MissionOutcome::Aborted,
        stardate_week: Some(3),
    });

    let invasion = viewer_report_texts(2, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");

    assert!(invasion.contains("ALERT: Enemy invasion blocked!"));
    assert!(invasion.contains("Our batteries prevented an enemy landing."));
    assert!(invasion.contains("Defensive losses: 3 ground batteries."));
    assert!(invasion.contains("Enemy losses:"));
    assert!(invasion.contains("1DD and 8 armies"));
    assert!(!invasion.contains("Our orbital softening losses:"));
    assert!(!invasion.contains("Our ground battle losses:"));
}

#[test]
fn ownership_change_report_uses_assault_context_for_defender() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3014)
        .build_initialized_baseline()
        .expect("baseline should build");
    let coords = [8, 2];
    seed_target_world(&mut game_data, coords, "half");
    game_data.planets[0].owner.0 = 2;
    game_data.planets[0].ground_batteries = 2;
    game_data.planets[0].armies = 5;

    let mut events = MaintenanceEvents::default();
    events.assault_report_events.push(AssaultReportEvent {
        kind: Mission::BlitzWorld,
        attacker_fleet_number: Some(4),
        planet_idx: 0,
        attacker_empire_raw: 1,
        defender_empire_raw: 2,
        attacker_initial: ShipLosses {
            cruisers: 1,
            ..ShipLosses::default()
        },
        attacker_loaded_armies_initial: 3,
        defender_batteries_initial: 2,
        defender_armies_initial: 5,
        attacker_ship_losses: ShipLosses::default(),
        attacker_army_losses: 3,
        transport_army_losses: 0,
        defender_battery_losses: 2,
        defender_army_losses_softening: 0,
        defender_army_losses: 5,
        outcome: MissionOutcome::Succeeded,
        stardate_week: Some(3),
    });
    events
        .ownership_change_events
        .push(PlanetOwnershipChangeEvent {
            planet_idx: 0,
            reporting_empire_raw: 2,
            previous_owner_empire_raw: 2,
            new_owner_empire_raw: 1,
            stardate_week: Some(3),
        });

    let rows = build_results_report_blocks(&game_data, &events);
    let text = viewer_report_texts(2, &rows).join(" ").replace('\n', " ");
    assert!(text.contains("ALERT: Planet lost to enemy blitz!"));
    assert!(!text.contains("Planet seized in a blitz assault by"));
    assert!(!text.contains("captured by \"Player1\", (Empire #1) from"));
    assert!(text.contains("Attacking force:"));
    assert!(text.contains("1CA"));
    assert!(text.contains("Our defenses:"));
    assert!(text.contains("2 ground batteries and 5 armies"));
    assert!(text.contains("All planetary defenses were destroyed."));
    assert!(text.contains("Enemy losses:"));
    assert!(text.contains("3 armies"));
    assert!(!text.contains("Orbital softening losses:"));
    assert!(!text.contains("Ground battle losses:"));
}

#[test]
fn defender_receives_repulsed_blitz_report() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3014)
        .build_initialized_baseline()
        .expect("baseline should build");
    let coords = [8, 2];
    seed_target_world(&mut game_data, coords, "half");
    game_data.planets[0].owner.0 = 2;
    game_data.planets[0].ground_batteries = 2;
    game_data.planets[0].armies = 5;

    let mut events = MaintenanceEvents::default();
    events.assault_report_events.push(AssaultReportEvent {
        kind: Mission::BlitzWorld,
        attacker_fleet_number: Some(4),
        planet_idx: 0,
        attacker_empire_raw: 1,
        defender_empire_raw: 2,
        attacker_initial: ShipLosses {
            cruisers: 1,
            transports: 6,
            ..ShipLosses::default()
        },
        attacker_loaded_armies_initial: 6,
        defender_batteries_initial: 2,
        defender_armies_initial: 5,
        attacker_ship_losses: ShipLosses::default(),
        attacker_army_losses: 4,
        transport_army_losses: 2,
        defender_battery_losses: 1,
        defender_army_losses_softening: 0,
        defender_army_losses: 2,
        outcome: MissionOutcome::Failed,
        stardate_week: Some(3),
    });

    let text = viewer_report_texts(2, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");
    assert!(text.contains("ALERT: Enemy blitz repulsed!"));
    assert!(text.contains("Attacking force:"));
    assert!(text.contains("1CA, 6TT*"));
    assert!(text.contains("Cover fire:"));
    assert!(text.contains("briefly suppressed"));
    assert!(text.contains("The blitz assault was repulsed."));
    assert!(text.contains("Defensive losses: 1 ground battery and 2 armies."));
    assert!(text.contains("Enemy losses:"));
    assert!(text.contains("6 armies"));
}

#[test]
fn bombardment_attacker_report_uses_first_person_fleet_and_undefended_wording() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3026)
        .build_initialized_baseline()
        .expect("baseline should build");
    let coords = [1, 9];
    seed_target_world(&mut game_data, coords, "biggy");

    let mut events = MaintenanceEvents::default();
    events.bombard_events.push(BombardEvent {
        planet_idx: 0,
        attacker_empire_raw: 1,
        attacker_fleet_number: Some(10),
        defender_empire_raw: 2,
        attacker_initial: ShipLosses {
            battleships: 10,
            cruisers: 11,
            transports: 21,
            ..ShipLosses::default()
        },
        attacker_loaded_armies_initial: 14,
        defender_batteries_initial: 0,
        defender_armies_initial: 0,
        attacker_losses: ShipLosses::default(),
        defender_battery_losses: 0,
        defender_army_losses: 0,
        breakthrough: true,
        docked_losses: nc_data::EmpireUnitSummary::default(),
        stardock_items_destroyed: 0,
        stored_goods_destroyed: 25,
        factories_destroyed: 336,
        stardate_week: Some(3),
    });
    events.mission_events.push(MissionEvent {
        fleet_idx: 0,
        owner_empire_raw: 1,
        kind: Mission::BombardWorld,
        outcome: MissionOutcome::Succeeded,
        abort_reason: None,
        planet_idx: Some(0),
        location_coords: Some(coords),
        target_coords: Some(coords),
        stardate_week: Some(3),
    });

    let text = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");
    assert!(text.contains("ALERT: Orbital bombardment underway!"));
    assert!(text.contains("Our forces:       10BB, 11CA, 14TT*, 7TT"));
    assert!(text.contains("World defenses:   undefended"));
    assert!(text.contains("Bombing damage: 336 points of industry destroyed."));
    assert!(text.contains("Bombing damage: 25 stored production destroyed."));
    assert!(!text.contains("336 factories"));
    assert!(!text.contains("We were unable to inflict any ground losses."));
    assert!(
        !text
            .contains("We broke through planetary defenses and struck the world's infrastructure.")
    );
    assert!(!text.contains("0 ground battery(ies)"));
    assert!(!text.contains("0 armies"));
}

#[test]
fn bombardment_report_ignores_destroyed_escort_when_bomber_survives() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3026)
        .build_initialized_baseline()
        .expect("baseline should build");
    let coords = [1, 9];
    seed_target_world(&mut game_data, coords, "biggy");
    game_data.fleets[0].owner.0 = 1;
    game_data.fleets[0].fleet_number.0 = 10;
    game_data.fleets[0].sector = Coords::from_array(coords);

    let mut events = MaintenanceEvents::default();
    events.mission_events.push(MissionEvent {
        fleet_idx: 0,
        owner_empire_raw: 1,
        kind: Mission::BombardWorld,
        outcome: MissionOutcome::Succeeded,
        abort_reason: None,
        planet_idx: Some(0),
        location_coords: Some(coords),
        target_coords: Some(coords),
        stardate_week: Some(3),
    });
    let escort_destroyed = FleetDestroyedEvent {
        reporting_empire_raw: 1,
        fleet_number: 11,
        coords,
        was_intercepting: false,
        friendly_initial: ShipLosses {
            destroyers: 1,
            ..ShipLosses::default()
        },
        friendly_loaded_armies_initial: 0,
        enemy_initial: ShipLosses::default(),
        enemy_initial_starbases: 0,
        enemy_loaded_armies_initial: 0,
        enemy_ground_batteries_initial: 2,
        enemy_ground_armies_initial: 0,
        enemy_losses: ShipLosses::default(),
        enemy_starbases_destroyed: 0,
        enemy_ground_battery_losses: 0,
        enemy_ground_army_losses: 0,
        primary_enemy_empire_raw: Some(2),
        primary_enemy_fleet_number: None,
        stardate_week: Some(3),
    };
    events.fleet_destroyed_events.push(escort_destroyed.clone());

    let text = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");
    assert!(!text.contains("destroyed the bombardment force"), "{text}");
    assert!(text.contains("We are maintaining bombardment position"));

    events.fleet_destroyed_events.push(FleetDestroyedEvent {
        fleet_number: 10,
        ..escort_destroyed
    });
    let text = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");
    assert!(text.contains("Hostile return fire destroyed the bombardment force."));
}

#[test]
fn bombardment_defender_report_uses_no_defenses_for_zero_counts() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3026)
        .build_initialized_baseline()
        .expect("baseline should build");
    let coords = [1, 9];
    seed_target_world(&mut game_data, coords, "biggy");

    let mut events = MaintenanceEvents::default();
    events.bombard_events.push(BombardEvent {
        planet_idx: 0,
        attacker_empire_raw: 2,
        attacker_fleet_number: Some(10),
        defender_empire_raw: 1,
        attacker_initial: ShipLosses {
            battleships: 10,
            cruisers: 11,
            transports: 21,
            ..ShipLosses::default()
        },
        attacker_loaded_armies_initial: 14,
        defender_batteries_initial: 0,
        defender_armies_initial: 0,
        attacker_losses: ShipLosses::default(),
        defender_battery_losses: 0,
        defender_army_losses: 0,
        breakthrough: true,
        docked_losses: nc_data::EmpireUnitSummary::default(),
        stardock_items_destroyed: 0,
        stored_goods_destroyed: 25,
        factories_destroyed: 336,
        stardate_week: Some(3),
    });

    let text = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");
    assert!(text.contains("Attacking force:"));
    assert!(text.contains("10BB, 11CA, 14TT*, 7TT"));
    assert!(text.contains("Our defenses:"));
    assert!(text.contains("none"));
    assert!(text.contains("Local damage: 336 points of industry destroyed."));
    assert!(text.contains("Local damage: 25 stored production destroyed."));
    assert!(!text.contains("336 factories"));
    assert!(!text.contains("appeared to contain"));
    assert!(!text.contains("0 ground battery(ies)"));
    assert!(!text.contains("0 army(ies)"));
    assert!(!text.contains("We lost 0"));
}

#[test]
fn scout_system_report_uses_estimated_production_not_present_production() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3026)
        .build_initialized_baseline()
        .expect("baseline should build");
    let coords = [6, 14];
    seed_target_world(&mut game_data, coords, "spyglass");
    let planet = &mut game_data.planets[0];
    planet.production = Real48::from_f64(73.0);
    planet.stored_goods = Real48::from_f64(41.0);
    planet.armies = 7;
    planet.ground_batteries = 2;

    let fleet = &mut game_data.fleets[0];
    fleet.owner.0 = 1;
    fleet.fleet_number.0 = 4;
    fleet.sector = Coords::from_array(coords);

    let mut events = MaintenanceEvents::default();
    events.planet_intel_events.push(PlanetIntelEvent {
        planet_idx: 0,
        viewer_empire_raw: 1,
        source: PlanetIntelSource::ScoutSolarSystem,
        source_fleet_idx: Some(0),
        observed_snapshot: None,
        stardate_week: Some(4),
    });
    events.mission_events.push(MissionEvent {
        fleet_idx: 0,
        owner_empire_raw: 1,
        kind: Mission::ScoutSolarSystem,
        outcome: MissionOutcome::Succeeded,
        abort_reason: None,
        planet_idx: Some(0),
        location_coords: Some(coords),
        target_coords: Some(coords),
        stardate_week: Some(4),
    });

    let text = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");
    assert!(text.contains("Potential production: 100 points"));
    assert!(text.contains("Estimated production: 73 points"));
    assert!(!text.contains("Estimated present production"));
}

#[test]
fn scout_system_report_aligns_its_fact_rows() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3026)
        .build_initialized_baseline()
        .expect("baseline should build");
    let coords = [6, 14];
    seed_target_world(&mut game_data, coords, "spyglass");
    game_data.planets[0].armies = 7;

    let fleet = &mut game_data.fleets[0];
    fleet.owner.0 = 1;
    fleet.fleet_number.0 = 4;
    fleet.sector = Coords::from_array(coords);

    let mut events = MaintenanceEvents::default();
    events.planet_intel_events.push(PlanetIntelEvent {
        planet_idx: 0,
        viewer_empire_raw: 1,
        source: PlanetIntelSource::ScoutSolarSystem,
        source_fleet_idx: Some(0),
        observed_snapshot: None,
        stardate_week: Some(4),
    });
    events.mission_events.push(MissionEvent {
        fleet_idx: 0,
        owner_empire_raw: 1,
        kind: Mission::ScoutSolarSystem,
        outcome: MissionOutcome::Succeeded,
        abort_reason: None,
        planet_idx: Some(0),
        location_coords: Some(coords),
        target_coords: Some(coords),
        stardate_week: Some(4),
    });

    let text = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events)).join("\n");
    let lines: Vec<&str> = text.lines().collect();
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("Owned by:         ")),
        "{text}"
    );
    assert!(lines.contains(&"Number of armies: 7"), "{text}");
    assert!(
        lines.iter().all(|line| !line.starts_with("  ")
            && line.len() <= 72
            && line.bytes().all(|byte| (0x20..0x7f).contains(&byte))),
        "{text}"
    );
}

#[test]
fn results_reports_named_hostile_fleet_with_empire_local_slot() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3001)
        .build_initialized_baseline()
        .expect("baseline should build");

    let rows = build_results_report_blocks(
        &mut game_data,
        &MaintenanceEvents {
            fleet_battle_events: vec![FleetBattleEvent {
                reporting_empire_raw: 1,
                reporting_fleet_number: Some(3),
                reporting_mission: Some(Mission::PatrolSector),
                perspective: FleetBattlePerspective::Attacked,
                coords: [8, 8],
                enemy_empires_raw: vec![2],
                primary_enemy_fleet_number: Some(2),
                held_field: false,
                friendly_initial: ShipLosses {
                    destroyers: 1,
                    ..ShipLosses::default()
                },
                friendly_initial_starbases: 0,
                friendly_loaded_armies_initial: 0,
                friendly_losses: ShipLosses {
                    destroyers: 1,
                    ..ShipLosses::default()
                },
                friendly_starbases_lost: 0,
                enemy_initial: ShipLosses {
                    cruisers: 1,
                    ..ShipLosses::default()
                },
                enemy_initial_starbases: 0,
                enemy_loaded_armies_initial: 0,
                enemy_losses: ShipLosses::default(),
                enemy_starbases_destroyed: 0,
                stardate_week: Some(2),
            }],
            ..MaintenanceEvents::default()
        },
    );

    let texts = viewer_report_texts(1, &rows);
    assert!(
        texts
            .iter()
            .any(|text| text.contains("2nd Fleet") && !text.contains("5th Fleet")),
        "hostile fleet references should use empire-local fleet numbers: {texts:?}"
    );
}

#[test]
fn results_report_invalid_capability_loss_as_aborted_seek_home_mission() {
    let mut game_data = load_fixture("ecmaint-fleet-pre");
    game_data.fleets[0].ships.etacs = 0;

    let events = run_maintenance_turn(&mut game_data).expect("maintenance should succeed");
    let rows = build_results_report_blocks(&mut game_data, &events);
    let texts = viewer_report_texts(1, &rows);

    assert!(texts.iter().any(|text| {
        text.contains("colonize world mission") && text.contains("lacks the required ETAC")
    }));
}

#[test]
fn results_merge_roe_retreat_into_invasion_abort_report() {
    let game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3016)
        .build_initialized_baseline()
        .expect("baseline should build");

    let mut events = MaintenanceEvents::default();
    events.mission_events.push(MissionEvent {
        fleet_idx: 0,
        owner_empire_raw: 1,
        kind: Mission::InvadeWorld,
        outcome: MissionOutcome::Aborted,
        abort_reason: None,
        planet_idx: Some(0),
        location_coords: Some([15, 13]),
        target_coords: Some([15, 13]),
        stardate_week: Some(2),
    });
    events
        .encounter_disposition_events
        .push(EncounterDispositionEvent::Retreated {
            fleet_idx: 0,
            owner_empire_raw: 1,
            mission: Some(Mission::InvadeWorld),
            coords: [15, 13],
            friendly_initial: ShipLosses {
                cruisers: 1,
                transports: 2,
                ..ShipLosses::default()
            },
            friendly_loaded_armies_initial: 2,
            target_empire_raw: 2,
            target_fleet_number: Some(4),
            enemy_initial: ShipLosses {
                cruisers: 2,
                transports: 3,
                ..ShipLosses::default()
            },
            retreat_target_coords: [16, 13],
            losses_sustained: ShipLosses {
                destroyers: 1,
                ..ShipLosses::default()
            },
            enemy_losses_inflicted: ShipLosses::default(),
            reason: EncounterDispositionReason::RoeWithdrawal,
            stardate_week: Some(2),
        });

    let rows = build_results_report_blocks(&game_data, &events);
    let texts = viewer_report_texts(1, &rows);
    let joined = texts.join(" ").replace('\n', " ");

    assert_eq!(
        texts
            .iter()
            .filter(|text| text.contains("Invasion mission report"))
            .count(),
        1,
        "expected one merged invasion abort report: {texts:?}"
    );
    assert!(joined.contains("In accordance with our ROE, we withdrew"));
    assert!(
        joined.contains("This forced us to abort the invasion before the landing could begin.")
    );
    assert!(joined.contains("We had 1CA, 2TT*."));
    assert!(joined.contains("The alien force contained 2CA, 3TT."));
    assert!(!joined.contains("Hostile action stripped us of our invasion capability"));
}

#[test]
fn roe_retreat_without_losses_uses_natural_wording() {
    let game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3019)
        .build_initialized_baseline()
        .expect("baseline should build");

    let mut events = MaintenanceEvents::default();
    events
        .encounter_disposition_events
        .push(EncounterDispositionEvent::Retreated {
            fleet_idx: 0,
            owner_empire_raw: 1,
            mission: Some(Mission::MoveOnly),
            coords: [11, 12],
            friendly_initial: ShipLosses {
                transports: 9,
                ..ShipLosses::default()
            },
            friendly_loaded_armies_initial: 9,
            target_empire_raw: 2,
            target_fleet_number: Some(14),
            enemy_initial: ShipLosses {
                battleships: 1,
                cruisers: 1,
                ..ShipLosses::default()
            },
            retreat_target_coords: [11, 11],
            losses_sustained: ShipLosses::default(),
            enemy_losses_inflicted: ShipLosses::default(),
            reason: EncounterDispositionReason::RoeWithdrawal,
            stardate_week: Some(2),
        });

    let text = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");
    assert!(text.contains("In accordance with our ROE, we withdrew"));
    assert!(text.contains("without suffering losses."));
    assert!(!text.contains("suffering losses of no ship losses"));
}

#[test]
fn victorious_invasion_report_says_enemy_fled_without_roe_leak() {
    let game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3016)
        .build_initialized_baseline()
        .expect("baseline should build");

    let mut events = MaintenanceEvents::default();
    events.fleet_battle_events.push(FleetBattleEvent {
        reporting_empire_raw: 1,
        reporting_fleet_number: Some(12),
        reporting_mission: Some(Mission::InvadeWorld),
        perspective: FleetBattlePerspective::Intercepted,
        coords: [9, 13],
        enemy_empires_raw: vec![2],
        primary_enemy_fleet_number: Some(8),
        held_field: true,
        friendly_initial: ShipLosses {
            battleships: 5,
            cruisers: 32,
            destroyers: 2,
            transports: 39,
            ..ShipLosses::default()
        },
        friendly_initial_starbases: 0,
        friendly_loaded_armies_initial: 6,
        friendly_losses: ShipLosses::default(),
        friendly_starbases_lost: 0,
        enemy_initial: ShipLosses {
            cruisers: 2,
            ..ShipLosses::default()
        },
        enemy_initial_starbases: 0,
        enemy_loaded_armies_initial: 0,
        enemy_losses: ShipLosses::default(),
        enemy_starbases_destroyed: 0,
        stardate_week: Some(2),
    });
    events
        .encounter_disposition_events
        .push(EncounterDispositionEvent::Retreated {
            fleet_idx: 4,
            owner_empire_raw: 2,
            mission: Some(Mission::MoveOnly),
            coords: [9, 13],
            friendly_initial: ShipLosses {
                cruisers: 2,
                ..ShipLosses::default()
            },
            friendly_loaded_armies_initial: 0,
            target_empire_raw: 1,
            target_fleet_number: Some(12),
            enemy_initial: ShipLosses {
                battleships: 5,
                cruisers: 32,
                destroyers: 2,
                transports: 39,
                ..ShipLosses::default()
            },
            retreat_target_coords: [5, 13],
            losses_sustained: ShipLosses::default(),
            enemy_losses_inflicted: ShipLosses::default(),
            reason: EncounterDispositionReason::RoeWithdrawal,
            stardate_week: Some(2),
        });

    let text = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");
    assert!(text.contains("ALERT: Enemy fleet contact!"));
    assert!(text.contains("The enemy fled the field."));
    assert!(!text.contains("In accordance with our ROE"));
}

#[test]
fn starbase_only_defender_report_uses_command_center_source() {
    let game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3016)
        .build_initialized_baseline()
        .expect("baseline should build");

    let mut events = MaintenanceEvents::default();
    events.fleet_battle_events.push(FleetBattleEvent {
        reporting_empire_raw: 2,
        reporting_fleet_number: None,
        reporting_mission: None,
        perspective: FleetBattlePerspective::Attacked,
        coords: [9, 6],
        enemy_empires_raw: vec![1],
        primary_enemy_fleet_number: Some(12),
        held_field: false,
        friendly_initial: ShipLosses::default(),
        friendly_initial_starbases: 1,
        friendly_loaded_armies_initial: 0,
        friendly_losses: ShipLosses::default(),
        friendly_starbases_lost: 0,
        enemy_initial: ShipLosses {
            battleships: 1,
            cruisers: 3,
            transports: 14,
            ..ShipLosses::default()
        },
        enemy_initial_starbases: 0,
        enemy_loaded_armies_initial: 11,
        enemy_losses: ShipLosses::default(),
        enemy_starbases_destroyed: 0,
        stardate_week: Some(2),
    });

    let rows = build_results_report_blocks(&game_data, &events);
    let text = viewer_report_texts(2, &rows).join(" ");
    assert!(text.contains("From your Fleet Command Center:"));
    assert!(text.contains("Our defenses:"));
    assert!(text.contains("1SB"));
    assert!(!text.contains("From your fleet"));

    assert!(!text.contains("Our force contained no ships."));
}

#[test]
fn attacker_report_mentions_destroyed_lone_starbase() {
    let game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3016)
        .build_initialized_baseline()
        .expect("baseline should build");

    let mut events = MaintenanceEvents::default();
    events.fleet_battle_events.push(FleetBattleEvent {
        reporting_empire_raw: 1,
        reporting_fleet_number: Some(12),
        reporting_mission: Some(Mission::BombardWorld),
        perspective: FleetBattlePerspective::Intercepted,
        coords: [9, 6],
        enemy_empires_raw: vec![2],
        primary_enemy_fleet_number: None,
        held_field: true,
        friendly_initial: ShipLosses {
            battleships: 1,
            cruisers: 3,
            transports: 14,
            ..ShipLosses::default()
        },
        friendly_initial_starbases: 0,
        friendly_loaded_armies_initial: 11,
        friendly_losses: ShipLosses::default(),
        friendly_starbases_lost: 0,
        enemy_initial: ShipLosses::default(),
        enemy_initial_starbases: 1,
        enemy_loaded_armies_initial: 0,
        enemy_losses: ShipLosses::default(),
        enemy_starbases_destroyed: 1,
        stardate_week: Some(2),
    });

    let rows = build_results_report_blocks(&game_data, &events);
    let text = viewer_report_texts(1, &rows).join(" ");
    assert!(text.contains("Alien forces:"));
    assert!(text.contains("1SB"));
    assert!(text.contains("The aliens were completely destroyed."));

    assert!(!text.contains("We were unable to inflict any losses."));
}

#[test]
fn victorious_fleet_report_says_enemy_fled_without_roe_leak() {
    let game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3016)
        .build_initialized_baseline()
        .expect("baseline should build");

    let mut events = MaintenanceEvents::default();
    events.fleet_battle_events.push(FleetBattleEvent {
        reporting_empire_raw: 1,
        reporting_fleet_number: Some(8),
        reporting_mission: Some(Mission::PatrolSector),
        perspective: FleetBattlePerspective::Intercepted,
        coords: [9, 6],
        enemy_empires_raw: vec![2],
        primary_enemy_fleet_number: Some(12),
        held_field: true,
        friendly_initial: ShipLosses {
            cruisers: 2,
            ..ShipLosses::default()
        },
        friendly_initial_starbases: 0,
        friendly_loaded_armies_initial: 0,
        friendly_losses: ShipLosses::default(),
        friendly_starbases_lost: 0,
        enemy_initial: ShipLosses {
            cruisers: 2,
            ..ShipLosses::default()
        },
        enemy_initial_starbases: 0,
        enemy_loaded_armies_initial: 0,
        enemy_losses: ShipLosses::default(),
        enemy_starbases_destroyed: 0,
        stardate_week: Some(2),
    });
    events
        .encounter_disposition_events
        .push(EncounterDispositionEvent::Retreated {
            fleet_idx: 4,
            owner_empire_raw: 2,
            mission: Some(Mission::MoveOnly),
            coords: [9, 6],
            friendly_initial: ShipLosses {
                cruisers: 2,
                ..ShipLosses::default()
            },
            friendly_loaded_armies_initial: 0,
            target_empire_raw: 1,
            target_fleet_number: Some(8),
            enemy_initial: ShipLosses {
                cruisers: 2,
                ..ShipLosses::default()
            },
            retreat_target_coords: [10, 6],
            losses_sustained: ShipLosses::default(),
            enemy_losses_inflicted: ShipLosses::default(),
            reason: EncounterDispositionReason::RoeWithdrawal,
            stardate_week: Some(2),
        });

    let text = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");
    assert!(text.contains("The enemy fled the field."));
    assert!(!text.contains("We held the field."));
    assert!(!text.contains("In accordance with our ROE"));
}

#[test]
fn victorious_fleet_report_uses_total_destruction_phrase() {
    let game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3025)
        .build_initialized_baseline()
        .expect("baseline should build");

    let mut events = MaintenanceEvents::default();
    events.fleet_battle_events.push(FleetBattleEvent {
        reporting_empire_raw: 1,
        reporting_fleet_number: Some(6),
        reporting_mission: Some(Mission::BombardWorld),
        perspective: FleetBattlePerspective::Intercepted,
        coords: [11, 4],
        enemy_empires_raw: vec![2],
        primary_enemy_fleet_number: Some(9),
        held_field: true,
        friendly_initial: ShipLosses {
            battleships: 2,
            cruisers: 6,
            destroyers: 4,
            ..ShipLosses::default()
        },
        friendly_initial_starbases: 0,
        friendly_loaded_armies_initial: 0,
        friendly_losses: ShipLosses {
            cruisers: 1,
            ..ShipLosses::default()
        },
        friendly_starbases_lost: 0,
        enemy_initial: ShipLosses {
            battleships: 1,
            cruisers: 3,
            transports: 2,
            ..ShipLosses::default()
        },
        enemy_initial_starbases: 0,
        enemy_loaded_armies_initial: 2,
        enemy_losses: ShipLosses {
            battleships: 1,
            cruisers: 3,
            transports: 2,
            ..ShipLosses::default()
        },
        enemy_starbases_destroyed: 0,
        stardate_week: Some(3),
    });

    let text = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");
    assert!(text.contains("The aliens were completely destroyed."));
    assert!(
        !text.contains(
            "We inflicted losses of 1 battleship, 3 cruisers and 2 troop transport ships."
        )
    );
}

#[test]
fn victorious_starbase_report_says_enemy_fled_without_roe_leak() {
    let game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3016)
        .build_initialized_baseline()
        .expect("baseline should build");

    let mut events = MaintenanceEvents::default();
    events.fleet_battle_events.push(FleetBattleEvent {
        reporting_empire_raw: 2,
        reporting_fleet_number: None,
        reporting_mission: None,
        perspective: FleetBattlePerspective::Attacked,
        coords: [9, 6],
        enemy_empires_raw: vec![1],
        primary_enemy_fleet_number: Some(12),
        held_field: true,
        friendly_initial: ShipLosses::default(),
        friendly_initial_starbases: 1,
        friendly_loaded_armies_initial: 0,
        friendly_losses: ShipLosses::default(),
        friendly_starbases_lost: 0,
        enemy_initial: ShipLosses {
            battleships: 1,
            ..ShipLosses::default()
        },
        enemy_initial_starbases: 0,
        enemy_loaded_armies_initial: 0,
        enemy_losses: ShipLosses::default(),
        enemy_starbases_destroyed: 0,
        stardate_week: Some(2),
    });
    events
        .encounter_disposition_events
        .push(EncounterDispositionEvent::Retreated {
            fleet_idx: 0,
            owner_empire_raw: 1,
            mission: Some(Mission::BombardWorld),
            coords: [9, 6],
            friendly_initial: ShipLosses {
                battleships: 1,
                ..ShipLosses::default()
            },
            friendly_loaded_armies_initial: 0,
            target_empire_raw: 2,
            target_fleet_number: None,
            enemy_initial: ShipLosses::default(),
            retreat_target_coords: [8, 6],
            losses_sustained: ShipLosses::default(),
            enemy_losses_inflicted: ShipLosses::default(),
            reason: EncounterDispositionReason::RoeWithdrawal,
            stardate_week: Some(2),
        });

    let text = viewer_report_texts(2, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");
    assert!(text.contains("The enemy fled the field."));
    assert!(!text.contains("We held the field."));
    assert!(!text.contains("In accordance with our ROE"));
}

#[test]
fn destroyed_starbase_only_defender_emits_only_telemetry_report() {
    let game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3016)
        .build_initialized_baseline()
        .expect("baseline should build");

    let mut events = MaintenanceEvents::default();
    events.fleet_battle_events.push(FleetBattleEvent {
        reporting_empire_raw: 2,
        reporting_fleet_number: None,
        reporting_mission: None,
        perspective: FleetBattlePerspective::Attacked,
        coords: [9, 6],
        enemy_empires_raw: vec![1],
        primary_enemy_fleet_number: Some(12),
        held_field: false,
        friendly_initial: ShipLosses::default(),
        friendly_initial_starbases: 1,
        friendly_loaded_armies_initial: 0,
        friendly_losses: ShipLosses::default(),
        friendly_starbases_lost: 1,
        enemy_initial: ShipLosses {
            battleships: 1,
            cruisers: 3,
            transports: 14,
            ..ShipLosses::default()
        },
        enemy_initial_starbases: 0,
        enemy_loaded_armies_initial: 11,
        enemy_losses: ShipLosses::default(),
        enemy_starbases_destroyed: 0,
        stardate_week: Some(2),
    });
    events
        .starbase_destroyed_events
        .push(nc_data::StarbaseDestroyedEvent {
            reporting_empire_raw: 2,
            starbase_id: 4,
            coords: [9, 6],
            enemy_initial: ShipLosses {
                battleships: 1,
                cruisers: 3,
                transports: 14,
                ..ShipLosses::default()
            },
            enemy_loaded_armies_initial: 11,
            enemy_losses: ShipLosses::default(),
            primary_enemy_empire_raw: Some(1),
            primary_enemy_fleet_number: Some(12),
            stardate_week: Some(2),
        });

    let rows = build_results_report_blocks(&game_data, &events);
    let texts = viewer_report_texts(2, &rows);
    assert_eq!(
        texts.len(),
        1,
        "starbase-only defense should not emit duplicate battle + telemetry reports: {texts:?}"
    );
    let before_forces = texts[0]
        .split("Alien forces:")
        .next()
        .expect("expected Alien forces section");
    assert!(before_forces.ends_with("\n\n"), "{:?}", texts[0]);
    assert!(texts[0].contains("Last contact:     destroyed by"));
    assert!(texts[0].contains("ALERT: Starbase contact lost!"));
    assert!(texts[0].contains("Alien forces:     1BB, 3CA, 11TT*, 3TT"));
}

#[test]
fn attacker_battle_report_explicitly_mentions_enemy_starbase_destruction() {
    let game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3016)
        .build_initialized_baseline()
        .expect("baseline should build");

    let mut events = MaintenanceEvents::default();
    events.fleet_battle_events.push(FleetBattleEvent {
        reporting_empire_raw: 1,
        reporting_fleet_number: Some(12),
        reporting_mission: Some(Mission::MoveOnly),
        perspective: FleetBattlePerspective::Attacked,
        coords: [9, 6],
        enemy_empires_raw: vec![2],
        primary_enemy_fleet_number: None,
        held_field: true,
        friendly_initial: ShipLosses {
            battleships: 1,
            cruisers: 3,
            transports: 14,
            ..ShipLosses::default()
        },
        friendly_initial_starbases: 0,
        friendly_loaded_armies_initial: 11,
        friendly_losses: ShipLosses::default(),
        friendly_starbases_lost: 0,
        enemy_initial: ShipLosses::default(),
        enemy_initial_starbases: 1,
        enemy_loaded_armies_initial: 0,
        enemy_losses: ShipLosses::default(),
        enemy_starbases_destroyed: 1,
        stardate_week: Some(2),
    });

    let text = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");
    assert!(text.contains("Enemy starbase destroyed."));
    assert!(text.contains("The aliens were completely destroyed."));
}

#[test]
fn results_reports_a_starbase_sighting_of_an_alien_starbase() {
    let game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3024)
        .build_initialized_baseline()
        .expect("baseline should build");
    let mut events = MaintenanceEvents::default();
    for (alerting, week) in [(true, 2), (false, 3)] {
        events.starbase_sighting_events.push(StarbaseSightingEvent {
            viewer_empire_raw: 1,
            starbase_id: 5,
            coords: [9, 13],
            in_system: false,
            target_empire_raw: 2,
            target_starbase_id: 3,
            alerting,
            stardate_week: Some(week),
        });
    }

    let texts = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events))
        .into_iter()
        .map(|text| text.replace('\n', " "))
        .collect::<Vec<_>>();
    assert_eq!(texts.len(), 2, "{texts:?}");
    assert!(
        texts[0].contains("From Starbase 5, located in Sector(9,13):"),
        "{}",
        texts[0]
    );
    assert!(
        texts[0].contains(
            "We have located and identified an alien starbase in Sector(9,13). It is Starbase 3 of Empire #2. We are alerting all fleets in the area."
        ),
        "{}",
        texts[0]
    );
    assert!(
        texts[1].contains("Continuing with scanning assignment..."),
        "{}",
        texts[1]
    );
    assert!(!texts[1].contains("alerting"), "{}", texts[1]);
}

#[test]
fn results_reports_starbase_contact_before_destroyed_starbase_notice() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3024)
        .build_initialized_baseline()
        .expect("baseline should build");
    let coords = [9, 13];
    seed_target_world(&mut game_data, coords, "Target");

    let mut events = MaintenanceEvents::default();
    events.scout_contact_events.push(ScoutContactEvent {
        viewer_empire_raw: 1,
        source: ContactReportSource::Starbase {
            id: 5,
            alert: StarbaseAlert::Enemy,
        },
        reporting_fleet_number: None,
        reporting_initial: ShipLosses::default(),
        reporting_loaded_armies_initial: 0,
        coords,
        target_empire_raw: 2,
        target_fleet_number: Some(7),
        small_vessels: 31,
        medium_vessels: 19,
        large_vessels: 2,
        stardate_week: Some(2),
    });
    events
        .starbase_destroyed_events
        .push(nc_data::StarbaseDestroyedEvent {
            reporting_empire_raw: 1,
            starbase_id: 5,
            coords,
            enemy_initial: ShipLosses {
                battleships: 2,
                cruisers: 19,
                destroyers: 5,
                scouts: 1,
                transports: 25,
                ..ShipLosses::default()
            },
            enemy_loaded_armies_initial: 9,
            enemy_losses: ShipLosses::default(),
            primary_enemy_empire_raw: Some(2),
            primary_enemy_fleet_number: Some(7),
            stardate_week: Some(2),
        });

    let texts = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events));
    assert_eq!(
        texts.len(),
        2,
        "expected contact and destroyed-starbase reports: {texts:?}"
    );
    assert!(texts[0].contains("We have located and identified an alien fleet"));
    assert!(texts[1].contains("ALERT: Starbase contact lost!"));
}

/// A starbase's sighting report ends with what the base makes of the fleet,
/// in classic's words.
#[test]
fn starbase_sighting_report_ends_with_the_bases_alert() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3024)
        .build_initialized_baseline()
        .expect("baseline should build");
    let coords = [9, 13];
    seed_target_world(&mut game_data, coords, "Target");
    for (alert, ending) in [
        (
            StarbaseAlert::HeadedAtOurWorld,
            "The aliens appear to be headed at our planet Target! We are alerting all fleets in the area.",
        ),
        (
            StarbaseAlert::Enemy,
            "We are alerting all fleets in the area.",
        ),
        (
            StarbaseAlert::Scanning,
            "Continuing with scanning assignment...",
        ),
    ] {
        let mut events = MaintenanceEvents::default();
        events.scout_contact_events.push(ScoutContactEvent {
            viewer_empire_raw: 1,
            source: ContactReportSource::Starbase { id: 5, alert },
            reporting_fleet_number: None,
            reporting_initial: ShipLosses::default(),
            reporting_loaded_armies_initial: 0,
            coords,
            target_empire_raw: 2,
            target_fleet_number: Some(7),
            small_vessels: 0,
            medium_vessels: 1,
            large_vessels: 0,
            stardate_week: Some(3),
        });
        let text = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events))
            .join(" ")
            .replace('\n', " ");
        assert!(
            text.contains("We have located and identified an alien fleet in System(9,13)."),
            "{alert:?}: {text}"
        );
        assert!(text.contains(ending), "{alert:?}: {text}");
        if alert != StarbaseAlert::HeadedAtOurWorld {
            assert!(!text.contains("headed at our planet"), "{alert:?}: {text}");
        }
    }
}

/// A fleet that decided to attack but could not catch its target says so,
/// in classic's words, instead of the ROE avoidance line.
#[test]
fn unable_to_intercept_report_uses_classic_wording() {
    let game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3024)
        .build_initialized_baseline()
        .expect("baseline should build");
    let mut events = MaintenanceEvents::default();
    events
        .encounter_disposition_events
        .push(EncounterDispositionEvent::NoEngagement {
            fleet_idx: 0,
            owner_empire_raw: 1,
            mission: Some(Mission::GuardBlockadeWorld),
            coords: [15, 13],
            friendly_initial: ShipLosses {
                cruisers: 3,
                ..ShipLosses::default()
            },
            friendly_loaded_armies_initial: 0,
            target_empire_raw: 2,
            target_fleet_number: Some(1),
            small_vessels: 0,
            medium_vessels: 2,
            large_vessels: 0,
            reason: EncounterDispositionReason::InterceptFailed,
            stardate_week: Some(24),
        });
    let text = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");
    assert!(
        text.contains("We have located and identified the alien fleet in System(15,13)"),
        "{text}"
    );
    assert!(text.contains("Their fleet contains 2 medium vessel(s) of unknown type. We are unable to intercept the enemy."), "{text}");
    assert!(!text.contains("avoiding"), "{text}");
}

#[test]
fn results_projection_is_pure_until_reviewable_flags_are_applied() {
    let mut game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3001)
        .build_initialized_baseline()
        .expect("baseline should build");
    let coords = [5, 13];
    seed_target_world(&mut game_data, coords, "Target");

    let mut events = MaintenanceEvents::default();
    events.scout_contact_events.push(ScoutContactEvent {
        viewer_empire_raw: 1,
        source: ContactReportSource::FleetMission(Mission::ScoutSector),
        reporting_fleet_number: Some(15),
        reporting_initial: ShipLosses {
            scouts: 1,
            ..ShipLosses::default()
        },
        reporting_loaded_armies_initial: 0,
        coords,
        target_empire_raw: 3,
        target_fleet_number: Some(4),
        small_vessels: 2,
        medium_vessels: 0,
        large_vessels: 0,
        stardate_week: Some(2),
    });

    let rows = build_results_report_blocks(&game_data, &events);
    assert!(
        game_data
            .players
            .iter()
            .all(|player| !(player.messages_last != 0)),
        "report projection should not mutate reviewable flags"
    );

    apply_results_reviewable_flags(&mut game_data, &rows);
    assert!(game_data.players[0].messages_last != 0);
    assert!(
        game_data
            .players
            .iter()
            .skip(1)
            .all(|player| !(player.messages_last != 0))
    );
}

#[test]
fn invalid_fleet_mission_report_tolerates_removed_fleet_index() {
    let game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3025)
        .build_initialized_baseline()
        .expect("baseline should build");

    let mut events = MaintenanceEvents::default();
    events
        .invalid_player_state_events
        .push(InvalidPlayerStateEvent::FleetMission {
            fleet_idx: 99,
            owner_empire_raw: 1,
            order_code_raw: nc_data::Order::InvadeWorld.code(),
            coords: [9, 9],
            reason: FleetOrderValidationError::MissingLoadedTroopTransports,
        });

    let text = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");
    assert!(text.contains("Hostile action forced us to abort the invade world mission"));
    assert!(text.contains("holding position and awaiting orders"));
}

#[test]
fn defeated_empire_report_uses_first_person_voice() {
    let game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3063)
        .build_initialized_baseline()
        .expect("baseline should build");

    let mut events = MaintenanceEvents::default();
    events
        .empire_elimination_events
        .push(EmpireEliminationEvent {
            defeated_empire_raw: 2,
            victor_empire_raw: Some(1),
            cause: EmpireEliminationCause::LastPlanetLost,
            planet_idx: None,
            coords: Some([7, 5]),
            stardate_week: Some(52),
        });

    let text = viewer_report_texts(2, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");
    assert!(text.contains("ALERT: Empire defeated! We have been defeated."));
    assert!(text.contains(
        "Our last world has been lost at System(7,5), and no recovery force remains able to restore our empire."
    ));
    assert!(!text.contains("\"Player2\" has been defeated."));
}

#[test]
fn victory_declared_report_uses_first_person_for_winner() {
    let game_data = GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3063)
        .build_initialized_baseline()
        .expect("baseline should build");

    let mut events = MaintenanceEvents::default();
    events
        .game_victory_notice_events
        .push(GameVictoryNoticeEvent {
            recipient_empire_raw: 1,
            winner_empire_raw: 1,
            stardate_week: Some(52),
        });

    let text = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");
    assert!(text.contains("ALERT: Victory declared! We have been recognized as Emperor."));
    assert!(!text.contains("\"Player1\", (Empire #1) has been recognized as Emperor."));
}
