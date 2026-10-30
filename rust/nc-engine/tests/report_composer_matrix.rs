use nc_data::{
    AssaultReportEvent, BombardEvent, ContactReportSource, Coords, EncounterDispositionEvent,
    EncounterDispositionReason, FleetBattleEvent, FleetDestroyedEvent, FleetMergeEvent,
    GameStateBuilder, JoinMissionHostEvent, MaintenanceEvents, Mission, MissionEvent,
    MissionOutcome, MissionRetargetEvent, PlanetOwnershipChangeEvent, ScoutContactEvent,
    ShipLosses,
};
use nc_engine::{build_results_report_blocks, maint::FleetBattlePerspective};

fn viewer_report_texts(viewer_empire_id: u8, rows: &[nc_data::ReportBlockRow]) -> Vec<String> {
    rows.iter()
        .filter(|row| row.viewer_empire_id == viewer_empire_id)
        .map(|row| row.decoded_text.clone())
        .collect()
}

fn seeded_game_data() -> nc_data::CoreGameData {
    GameStateBuilder::new()
        .with_player_count(4)
        .with_year(3018)
        .build_initialized_baseline()
        .expect("baseline should build")
}

fn configure_fleet(
    game_data: &mut nc_data::CoreGameData,
    fleet_idx: usize,
    owner_empire: u8,
    fleet_number: u8,
    coords: [u8; 2],
) {
    let fleet = &mut game_data.fleets[fleet_idx];
    fleet.owner.0 = owner_empire;
    fleet.fleet_number.0 = fleet_number as u16;
    fleet.sector = Coords::from_array(coords);
}

fn assert_viewers_have_reports(
    game_data: &nc_data::CoreGameData,
    events: &MaintenanceEvents,
    viewers: &[u8],
) {
    let rows = build_results_report_blocks(game_data, events);
    for viewer in viewers {
        let texts = viewer_report_texts(*viewer, &rows);
        assert!(
            !texts.is_empty(),
            "viewer {viewer} should have at least one report: {texts:?}"
        );
    }
}

#[test]
fn fleet_destroyed_event_supersedes_generic_battle_report() {
    let mut game_data = seeded_game_data();
    configure_fleet(&mut game_data, 0, 1, 11, [6, 6]);

    let mut events = MaintenanceEvents::default();
    events.fleet_battle_events.push(FleetBattleEvent {
        reporting_empire: 1,
        reporting_fleet_number: Some(11),
        reporting_mission: Some(Mission::GuardBlockadeWorld),
        perspective: FleetBattlePerspective::Intercepted,
        coords: [6, 6],
        enemy_empires: vec![2],
        primary_enemy_fleet_number: Some(7),
        held_field: false,
        friendly_initial: ShipLosses {
            cruisers: 1,
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
            destroyers: 2,
            ..ShipLosses::default()
        },
        enemy_initial_starbases: 0,
        enemy_loaded_armies_initial: 0,
        enemy_losses: ShipLosses::default(),
        enemy_starbases_destroyed: 0,
        stardate_week: Some(1),
        surprise: false,
    });
    events.fleet_destroyed_events.push(FleetDestroyedEvent {
        reporting_empire: 1,
        fleet_number: 11,
        coords: [6, 6],
        was_intercepting: true,
        friendly_initial: ShipLosses {
            cruisers: 1,
            ..ShipLosses::default()
        },
        friendly_loaded_armies_initial: 0,
        enemy_initial: ShipLosses {
            destroyers: 2,
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
        primary_enemy_empire: Some(2),
        primary_enemy_fleet_number: Some(7),
        stardate_week: Some(1),
    });

    let texts = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events));
    assert_eq!(
        texts.len(),
        1,
        "destroyed fleet should suppress generic battle: {texts:?}"
    );
    let text = texts[0].split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(text.contains("We lost all contact with the"), "{text}");
    assert!(text.contains("shortly after it intercepted"), "{text}");
    assert!(text.contains("burnt flight recorder"), "{text}");
    assert!(!text.contains("Interception successful."), "{text}");
}

#[test]
fn roe_retreat_and_abort_merge_into_one_report() {
    let mut game_data = seeded_game_data();
    configure_fleet(&mut game_data, 0, 1, 12, [6, 6]);

    let mut events = MaintenanceEvents::default();
    events.mission_events.push(MissionEvent {
        fleet_idx: 0,
        owner_empire: 1,
        kind: Mission::InvadeWorld,
        outcome: MissionOutcome::Aborted,
        abort_reason: None,
        planet_idx: None,
        location_coords: Some([6, 6]),
        target_coords: Some([6, 6]),
        stardate_week: Some(1),
    });
    events
        .encounter_disposition_events
        .push(EncounterDispositionEvent::Retreated {
            fleet_idx: 0,
            owner_empire: 1,
            mission: Some(Mission::InvadeWorld),
            coords: [6, 6],
            friendly_initial: ShipLosses {
                cruisers: 1,
                transports: 2,
                ..ShipLosses::default()
            },
            friendly_loaded_armies_initial: 2,
            target_empire: 2,
            target_fleet_number: Some(3),
            enemy_initial: ShipLosses {
                cruisers: 2,
                ..ShipLosses::default()
            },
            retreat_target_coords: [5, 5],
            resumed: false,
            losses_sustained: ShipLosses {
                destroyers: 1,
                ..ShipLosses::default()
            },
            enemy_losses_inflicted: ShipLosses::default(),
            reason: EncounterDispositionReason::RoeWithdrawal,
            stardate_week: Some(1),
        });

    let texts = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events));
    let joined = texts.join(" ").replace('\n', " ");
    assert_eq!(texts.len(), 1, "ROE-backed abort should merge: {texts:?}");
    assert!(joined.contains("Invasion mission report"));
    assert!(joined.contains("In accordance with our ROE"));
    assert!(joined.contains("abort the invasion"));
    assert!(joined.contains("We had CA1 TT*2."));
}

#[test]
fn rendezvous_absorbing_report_uses_compact_oxford_fleet_list() {
    let mut game_data = seeded_game_data();
    configure_fleet(&mut game_data, 0, 1, 10, [12, 12]);

    let mut events = MaintenanceEvents::default();
    for absorbed in [11, 5, 7, 6] {
        events.fleet_merge_events.push(FleetMergeEvent {
            fleet_idx: 0,
            owner_empire: 1,
            kind: Mission::RendezvousSector,
            host_fleet_id: 1,
            absorbed_fleet_id: u16::from(absorbed),
            host_fleet_number: 10,
            absorbed_fleet_number: absorbed,
            coords: [12, 12],
            survivor_side: true,
            stardate_week: Some(1),
        });
    }

    let texts = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events));
    let joined = texts.join(" ").replace('\n', " ");
    assert!(joined.contains("Rendezvous mission report"));
    assert!(
        joined.contains("absorbing fleets 5, 6, 7, and 11."),
        "{joined}"
    );
    assert!(!joined.contains("the 5th Fleet"), "{joined}");
    assert!(!joined.contains("the 11th Fleet"), "{joined}");
}

#[test]
fn unknown_join_host_never_renders_zero_fleet_number() {
    let mut game_data = seeded_game_data();
    configure_fleet(&mut game_data, 0, 1, 10, [3, 9]);

    let mut events = MaintenanceEvents::default();
    events
        .join_host_events
        .push(JoinMissionHostEvent::HostDestroyed {
            fleet_idx: 0,
            owner_empire: 1,
            destroyed_host_fleet_number: None,
            coords: [3, 9],
        });

    let text = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");
    assert!(text.contains("From your Fleet Command Center:"));
    assert!(text.contains("Lost hosts: Fleet 10 lost their host and is holding position."));
    assert!(!text.contains("(0th Fleet)"));
}

#[test]
fn known_join_host_renders_destroyed_host_fleet_number() {
    let mut game_data = seeded_game_data();
    configure_fleet(&mut game_data, 0, 1, 10, [3, 9]);

    let mut events = MaintenanceEvents::default();
    events
        .join_host_events
        .push(JoinMissionHostEvent::HostDestroyed {
            fleet_idx: 0,
            owner_empire: 1,
            destroyed_host_fleet_number: Some(7),
            coords: [3, 9],
        });

    let text = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");
    assert!(text.contains("From your Fleet Command Center:"));
    assert!(text.contains("Lost hosts: Fleet 10 lost host Fleet 7 and is holding position."));
    assert!(!text.contains("(0th Fleet)"));
}

#[test]
fn join_summary_retarget_uses_stored_reporting_fleet_number_and_omits_sector() {
    let mut game_data = seeded_game_data();
    configure_fleet(&mut game_data, 0, 1, 99, [6, 6]);

    let mut events = MaintenanceEvents::default();
    events
        .mission_retarget_events
        .push(MissionRetargetEvent::Retargeted {
            fleet_idx: 0,
            reporting_fleet_number: Some(11),
            owner_empire: 1,
            mission: Mission::JoinAnotherFleet,
            current_coords: [6, 6],
            in_system: false,
            previous_target_coords: [4, 4],
            new_target_coords: [8, 8],
            stardate_week: None,
        });

    let text = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");
    assert!(text.contains("From your Fleet Command Center:"));
    assert!(text.contains("Retargeted to follow host: Fleet 11."));
    assert!(!text.contains("Fleet 99"));
    assert!(!text.contains("Sector(8,8)"));
}

#[test]
fn join_summary_contains_single_end_of_transmission_footer() {
    let mut game_data = seeded_game_data();
    configure_fleet(&mut game_data, 0, 1, 3, [6, 6]);
    configure_fleet(&mut game_data, 1, 1, 8, [6, 6]);
    configure_fleet(&mut game_data, 2, 1, 11, [6, 6]);
    configure_fleet(&mut game_data, 3, 1, 13, [6, 6]);

    let mut events = MaintenanceEvents::default();
    events.fleet_merge_events.push(FleetMergeEvent {
        fleet_idx: 1,
        owner_empire: 1,
        kind: Mission::JoinAnotherFleet,
        host_fleet_id: 1,
        absorbed_fleet_id: 2,
        coords: [6, 6],
        host_fleet_number: 3,
        absorbed_fleet_number: 8,
        survivor_side: false,
        stardate_week: Some(1),
    });
    events
        .mission_retarget_events
        .push(MissionRetargetEvent::Retargeted {
            fleet_idx: 2,
            reporting_fleet_number: Some(11),
            owner_empire: 1,
            mission: Mission::JoinAnotherFleet,
            current_coords: [6, 6],
            in_system: false,
            previous_target_coords: [4, 4],
            new_target_coords: [8, 8],
            stardate_week: None,
        });
    events
        .join_host_events
        .push(JoinMissionHostEvent::HostDestroyed {
            fleet_idx: 3,
            owner_empire: 1,
            destroyed_host_fleet_number: Some(2),
            coords: [6, 6],
        });

    let texts = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events));
    let joined = texts.join("\n");
    assert!(joined.contains("Join mission summary"));
    assert_eq!(
        joined.matches("<end of transmission>").count(),
        1,
        "{joined}"
    );
}

#[test]
fn destroyed_fleet_telemetry_reports_starbase_only_opponent() {
    let mut game_data = seeded_game_data();
    configure_fleet(&mut game_data, 0, 1, 20, [11, 8]);

    let mut events = MaintenanceEvents::default();
    events.fleet_destroyed_events.push(FleetDestroyedEvent {
        reporting_empire: 1,
        fleet_number: 20,
        coords: [11, 8],
        was_intercepting: true,
        friendly_initial: ShipLosses {
            cruisers: 1,
            ..ShipLosses::default()
        },
        friendly_loaded_armies_initial: 0,
        enemy_initial: ShipLosses::default(),
        enemy_initial_starbases: 1,
        enemy_loaded_armies_initial: 0,
        enemy_ground_batteries_initial: 0,
        enemy_ground_armies_initial: 0,
        enemy_losses: ShipLosses::default(),
        enemy_starbases_destroyed: 0,
        enemy_ground_battery_losses: 0,
        enemy_ground_army_losses: 0,
        primary_enemy_empire: Some(2),
        primary_enemy_fleet_number: None,
        stardate_week: Some(2),
    });

    let text = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");
    assert!(text.contains("SB1"));
    assert!(!text.contains("no ships"));
}

#[test]
fn no_engagement_report_includes_reporting_fleet_composition() {
    let mut game_data = seeded_game_data();
    configure_fleet(&mut game_data, 0, 1, 8, [9, 5]);

    let mut events = MaintenanceEvents::default();
    events
        .encounter_disposition_events
        .push(EncounterDispositionEvent::NoEngagement {
            fleet_idx: 0,
            owner_empire: 1,
            mission: Some(Mission::ScoutSector),
            coords: [9, 5],
            friendly_initial: ShipLosses {
                cruisers: 1,
                destroyers: 1,
                ..ShipLosses::default()
            },
            friendly_loaded_armies_initial: 0,
            target_empire: 2,
            target_fleet_number: Some(4),
            small_vessels: 1,
            medium_vessels: 2,
            large_vessels: 1,
            reason: EncounterDispositionReason::RoeDeclined,
            stardate_week: Some(2),
        });

    let text = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");
    assert!(text.contains("We have CA1 DD1."));
    assert!(text.contains("Their fleet contains"));
}

#[test]
fn fleet_contact_report_includes_reporting_fleet_composition() {
    let mut game_data = seeded_game_data();
    configure_fleet(&mut game_data, 0, 1, 13, [24, 14]);

    let mut events = MaintenanceEvents::default();
    events.scout_contact_events.push(ScoutContactEvent {
        viewer_empire: 1,
        source: ContactReportSource::FleetMission(Mission::ScoutSector),
        reporting_fleet_number: Some(13),
        reporting_initial: ShipLosses {
            scouts: 1,
            ..ShipLosses::default()
        },
        reporting_loaded_armies_initial: 0,
        coords: [24, 14],
        target_empire: 2,
        target_fleet_number: Some(5),
        small_vessels: 2,
        medium_vessels: 0,
        large_vessels: 0,
        stardate_week: Some(52),
    });

    let text = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");
    assert!(text.contains("We have SC1."));
    assert!(text.contains("Their fleet contains"));
}

#[test]
fn join_host_retarget_summary_lists_joiner_without_host_merge_prose() {
    let mut game_data = seeded_game_data();
    configure_fleet(&mut game_data, 0, 1, 12, [2, 7]);

    let mut events = MaintenanceEvents::default();
    events
        .join_host_events
        .push(JoinMissionHostEvent::Retargeted {
            fleet_idx: 0,
            owner_empire: 1,
            previous_host_fleet_number: Some(14),
            new_host_fleet_number: Some(2),
            coords: [2, 7],
        });

    let text = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");
    assert!(text.contains("From your Fleet Command Center:"));
    assert!(text.contains("Retargeted to follow host: Fleet 12."));
    assert!(!text.contains("14th Fleet"));
    assert!(!text.contains("2nd Fleet"));
}

#[test]
fn join_summary_combines_completed_retargeted_and_lost_hosts() {
    let mut game_data = seeded_game_data();
    configure_fleet(&mut game_data, 0, 1, 3, [5, 5]);
    configure_fleet(&mut game_data, 1, 1, 8, [5, 5]);
    configure_fleet(&mut game_data, 2, 1, 11, [6, 6]);
    configure_fleet(&mut game_data, 3, 1, 13, [7, 7]);

    let mut events = MaintenanceEvents::default();
    events.fleet_merge_events.push(FleetMergeEvent {
        fleet_idx: 1,
        owner_empire: 1,
        kind: Mission::JoinAnotherFleet,
        host_fleet_id: 1,
        absorbed_fleet_id: 2,
        host_fleet_number: 3,
        absorbed_fleet_number: 8,
        coords: [5, 5],
        survivor_side: false,
        stardate_week: Some(2),
    });
    events
        .mission_retarget_events
        .push(MissionRetargetEvent::Retargeted {
            fleet_idx: 2,
            reporting_fleet_number: Some(11),
            owner_empire: 1,
            mission: Mission::JoinAnotherFleet,
            current_coords: [6, 6],
            in_system: false,
            previous_target_coords: [4, 4],
            new_target_coords: [8, 8],
            stardate_week: None,
        });
    events
        .join_host_events
        .push(JoinMissionHostEvent::HostDestroyed {
            fleet_idx: 3,
            owner_empire: 1,
            destroyed_host_fleet_number: Some(2),
            coords: [7, 7],
        });

    let texts = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events));
    assert_eq!(texts.len(), 1, "{texts:?}");
    let text = &texts[0];
    assert!(text.contains("From your Fleet Command Center:"));
    assert!(text.contains("Join mission summary"));
    assert!(text.contains("Completed joins: Fleet 8 merged into Fleet 3."));
    assert!(text.contains("Retargeted to follow host: Fleet 11."));
    assert!(text.contains("Lost hosts: Fleet 13 lost host Fleet 2 and is holding position."));
}

#[test]
fn join_summary_uses_compact_fleet_lists_for_multi_fleet_sections() {
    let mut game_data = seeded_game_data();
    configure_fleet(&mut game_data, 0, 1, 3, [7, 7]);
    configure_fleet(&mut game_data, 1, 1, 5, [7, 7]);
    configure_fleet(&mut game_data, 2, 1, 6, [7, 7]);
    configure_fleet(&mut game_data, 3, 1, 7, [7, 7]);
    configure_fleet(&mut game_data, 4, 1, 8, [7, 7]);
    configure_fleet(&mut game_data, 5, 1, 9, [7, 7]);
    configure_fleet(&mut game_data, 6, 1, 10, [7, 7]);
    configure_fleet(&mut game_data, 7, 1, 11, [7, 7]);
    configure_fleet(&mut game_data, 8, 1, 12, [7, 7]);

    let mut events = MaintenanceEvents::default();
    for absorbed in [11, 5, 7] {
        events.fleet_merge_events.push(FleetMergeEvent {
            fleet_idx: (absorbed - 4) as usize,
            owner_empire: 1,
            kind: Mission::JoinAnotherFleet,
            host_fleet_id: 1,
            absorbed_fleet_id: u16::from(absorbed),
            host_fleet_number: 3,
            absorbed_fleet_number: absorbed,
            coords: [7, 7],
            survivor_side: false,
            stardate_week: Some(1),
        });
    }
    for fleet_number in [10, 6, 4] {
        events
            .mission_retarget_events
            .push(MissionRetargetEvent::Retargeted {
                fleet_idx: 0,
                reporting_fleet_number: Some(fleet_number),
                owner_empire: 1,
                mission: Mission::JoinAnotherFleet,
                current_coords: [7, 7],
                in_system: false,
                previous_target_coords: [4, 4],
                new_target_coords: [8, 8],
                stardate_week: None,
            });
    }
    for fleet_number in [12, 9, 8] {
        events
            .join_host_events
            .push(JoinMissionHostEvent::HostDestroyed {
                fleet_idx: (fleet_number - 4) as usize,
                owner_empire: 1,
                destroyed_host_fleet_number: Some(2),
                coords: [7, 7],
            });
    }

    let text = viewer_report_texts(1, &build_results_report_blocks(&game_data, &events))
        .join(" ")
        .replace('\n', " ");
    assert!(
        text.contains("Completed joins: Fleets 5, 7, and 11 merged into Fleet 3."),
        "{text}"
    );
    assert!(
        text.contains("Retargeted to follow host: Fleets 4, 6, and 10."),
        "{text}"
    );
    assert!(
        text.contains(
            "Lost hosts: Fleets 8, 9, and 12 lost host Fleet 2 and are holding position."
        ),
        "{text}"
    );
    assert!(!text.contains("Fleets 5, 7 and 11"), "{text}");
}

#[test]
fn mirrored_contact_events_cover_both_viewers() {
    let mut game_data = seeded_game_data();
    configure_fleet(&mut game_data, 0, 1, 13, [24, 14]);
    configure_fleet(&mut game_data, 1, 2, 5, [24, 14]);

    let mut events = MaintenanceEvents::default();
    events.scout_contact_events.push(ScoutContactEvent {
        viewer_empire: 1,
        source: ContactReportSource::FleetMission(Mission::ScoutSector),
        reporting_fleet_number: Some(13),
        reporting_initial: ShipLosses {
            scouts: 1,
            ..ShipLosses::default()
        },
        reporting_loaded_armies_initial: 0,
        coords: [24, 14],
        target_empire: 2,
        target_fleet_number: Some(5),
        small_vessels: 2,
        medium_vessels: 0,
        large_vessels: 0,
        stardate_week: Some(52),
    });
    events.scout_contact_events.push(ScoutContactEvent {
        viewer_empire: 2,
        source: ContactReportSource::Fleet(5),
        reporting_fleet_number: Some(5),
        reporting_initial: ShipLosses {
            destroyers: 2,
            ..ShipLosses::default()
        },
        reporting_loaded_armies_initial: 0,
        coords: [24, 14],
        target_empire: 1,
        target_fleet_number: Some(13),
        small_vessels: 1,
        medium_vessels: 0,
        large_vessels: 0,
        stardate_week: Some(52),
    });

    assert_viewers_have_reports(&game_data, &events, &[1, 2]);
}

#[test]
fn fleet_battle_reports_cover_both_viewers() {
    let game_data = seeded_game_data();
    let mut events = MaintenanceEvents::default();
    events.fleet_battle_events.push(FleetBattleEvent {
        reporting_empire: 1,
        reporting_fleet_number: Some(11),
        reporting_mission: Some(Mission::GuardBlockadeWorld),
        perspective: FleetBattlePerspective::Intercepted,
        coords: [6, 6],
        enemy_empires: vec![2],
        primary_enemy_fleet_number: Some(7),
        held_field: true,
        friendly_initial: ShipLosses {
            cruisers: 2,
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
            destroyers: 2,
            ..ShipLosses::default()
        },
        enemy_initial_starbases: 0,
        enemy_loaded_armies_initial: 0,
        enemy_losses: ShipLosses {
            destroyers: 2,
            ..ShipLosses::default()
        },
        enemy_starbases_destroyed: 0,
        stardate_week: Some(1),
        surprise: false,
    });
    events.fleet_battle_events.push(FleetBattleEvent {
        reporting_empire: 2,
        reporting_fleet_number: Some(7),
        reporting_mission: Some(Mission::MoveOnly),
        perspective: FleetBattlePerspective::Attacked,
        coords: [6, 6],
        enemy_empires: vec![1],
        primary_enemy_fleet_number: Some(11),
        held_field: false,
        friendly_initial: ShipLosses {
            destroyers: 2,
            ..ShipLosses::default()
        },
        friendly_initial_starbases: 0,
        friendly_loaded_armies_initial: 0,
        friendly_losses: ShipLosses {
            destroyers: 2,
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
            cruisers: 1,
            ..ShipLosses::default()
        },
        enemy_starbases_destroyed: 0,
        stardate_week: Some(1),
        surprise: false,
    });

    assert_viewers_have_reports(&game_data, &events, &[1, 2]);
}

#[test]
fn fleet_destruction_events_still_cover_both_viewers() {
    let game_data = seeded_game_data();
    let mut events = MaintenanceEvents::default();
    events.fleet_destroyed_events.push(FleetDestroyedEvent {
        reporting_empire: 1,
        fleet_number: 11,
        coords: [6, 6],
        was_intercepting: true,
        friendly_initial: ShipLosses {
            cruisers: 1,
            ..ShipLosses::default()
        },
        friendly_loaded_armies_initial: 0,
        enemy_initial: ShipLosses {
            destroyers: 2,
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
        primary_enemy_empire: Some(2),
        primary_enemy_fleet_number: Some(7),
        stardate_week: Some(1),
    });
    events.fleet_battle_events.push(FleetBattleEvent {
        reporting_empire: 2,
        reporting_fleet_number: Some(7),
        reporting_mission: Some(Mission::MoveOnly),
        perspective: FleetBattlePerspective::Attacked,
        coords: [6, 6],
        enemy_empires: vec![1],
        primary_enemy_fleet_number: Some(11),
        held_field: true,
        friendly_initial: ShipLosses {
            destroyers: 2,
            ..ShipLosses::default()
        },
        friendly_initial_starbases: 0,
        friendly_loaded_armies_initial: 0,
        friendly_losses: ShipLosses::default(),
        friendly_starbases_lost: 0,
        enemy_initial: ShipLosses {
            cruisers: 1,
            ..ShipLosses::default()
        },
        enemy_initial_starbases: 0,
        enemy_loaded_armies_initial: 0,
        enemy_losses: ShipLosses {
            cruisers: 1,
            ..ShipLosses::default()
        },
        enemy_starbases_destroyed: 0,
        stardate_week: Some(1),
        surprise: false,
    });

    assert_viewers_have_reports(&game_data, &events, &[1, 2]);
}

#[test]
fn bombardment_reports_cover_attacker_and_defender() {
    let game_data = seeded_game_data();
    let mut events = MaintenanceEvents::default();
    events.bombard_events.push(BombardEvent {
        planet_idx: 0,
        attacker_empire: 1,
        attacker_fleet_number: Some(10),
        defender_empire: 2,
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
        defender_batteries_crippled: 0,
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
        owner_empire: 1,
        kind: Mission::BombardWorld,
        outcome: MissionOutcome::Succeeded,
        abort_reason: None,
        planet_idx: Some(0),
        location_coords: Some([1, 9]),
        target_coords: Some([1, 9]),
        stardate_week: Some(3),
    });

    assert_viewers_have_reports(&game_data, &events, &[1, 2]);
}

#[test]
fn invasion_reports_cover_both_viewers_for_success_failure_and_abort() {
    let game_data = seeded_game_data();

    let mut success = MaintenanceEvents::default();
    success.assault_report_events.push(AssaultReportEvent {
        kind: Mission::InvadeWorld,
        attacker_fleet_number: Some(8),
        planet_idx: 0,
        attacker_empire: 1,
        defender_empire: 2,
        attacker_initial: ShipLosses {
            battleships: 2,
            transports: 12,
            ..ShipLosses::default()
        },
        attacker_loaded_armies_initial: 12,
        defender_batteries_initial: 4,
        defender_armies_initial: 16,
        attacker_ship_losses: ShipLosses::default(),
        attacker_army_losses: 4,
        transport_army_losses: 0,
        defender_battery_losses: 4,
        defender_army_losses_softening: 8,
        defender_army_losses: 16,
        inherited: None,
        outcome: MissionOutcome::Succeeded,
        stardate_week: Some(3),
    });
    success
        .ownership_change_events
        .push(PlanetOwnershipChangeEvent {
            planet_idx: 0,
            reporting_empire: 2,
            previous_owner_empire: 2,
            new_owner_empire: 1,
            stardate_week: Some(3),
        });
    assert_viewers_have_reports(&game_data, &success, &[1, 2]);

    let mut failed = MaintenanceEvents::default();
    failed.assault_report_events.push(AssaultReportEvent {
        kind: Mission::InvadeWorld,
        attacker_fleet_number: Some(8),
        planet_idx: 0,
        attacker_empire: 1,
        defender_empire: 2,
        attacker_initial: ShipLosses {
            destroyers: 4,
            transports: 8,
            ..ShipLosses::default()
        },
        attacker_loaded_armies_initial: 8,
        defender_batteries_initial: 6,
        defender_armies_initial: 20,
        attacker_ship_losses: ShipLosses::default(),
        attacker_army_losses: 6,
        transport_army_losses: 0,
        defender_battery_losses: 6,
        defender_army_losses_softening: 10,
        defender_army_losses: 16,
        inherited: None,
        outcome: MissionOutcome::Failed,
        stardate_week: Some(3),
    });
    assert_viewers_have_reports(&game_data, &failed, &[1, 2]);

    let mut aborted = MaintenanceEvents::default();
    aborted.assault_report_events.push(AssaultReportEvent {
        kind: Mission::InvadeWorld,
        attacker_fleet_number: Some(8),
        planet_idx: 0,
        attacker_empire: 1,
        defender_empire: 2,
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
        inherited: None,
        outcome: MissionOutcome::Aborted,
        stardate_week: Some(3),
    });
    assert_viewers_have_reports(&game_data, &aborted, &[1, 2]);
}

#[test]
fn blitz_reports_cover_both_viewers_for_success_and_failure() {
    let game_data = seeded_game_data();

    let mut success = MaintenanceEvents::default();
    success.assault_report_events.push(AssaultReportEvent {
        kind: Mission::BlitzWorld,
        attacker_fleet_number: Some(4),
        planet_idx: 0,
        attacker_empire: 1,
        defender_empire: 2,
        attacker_initial: ShipLosses {
            cruisers: 1,
            transports: 6,
            ..ShipLosses::default()
        },
        attacker_loaded_armies_initial: 6,
        defender_batteries_initial: 2,
        defender_armies_initial: 5,
        attacker_ship_losses: ShipLosses::default(),
        attacker_army_losses: 3,
        transport_army_losses: 0,
        defender_battery_losses: 2,
        defender_army_losses_softening: 0,
        defender_army_losses: 5,
        inherited: None,
        outcome: MissionOutcome::Succeeded,
        stardate_week: Some(3),
    });
    success
        .ownership_change_events
        .push(PlanetOwnershipChangeEvent {
            planet_idx: 0,
            reporting_empire: 2,
            previous_owner_empire: 2,
            new_owner_empire: 1,
            stardate_week: Some(3),
        });
    assert_viewers_have_reports(&game_data, &success, &[1, 2]);

    let mut failed = MaintenanceEvents::default();
    failed.assault_report_events.push(AssaultReportEvent {
        kind: Mission::BlitzWorld,
        attacker_fleet_number: Some(4),
        planet_idx: 0,
        attacker_empire: 1,
        defender_empire: 2,
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
        inherited: None,
        outcome: MissionOutcome::Failed,
        stardate_week: Some(3),
    });
    assert_viewers_have_reports(&game_data, &failed, &[1, 2]);
}

#[test]
fn starbase_only_defense_still_covers_both_viewers() {
    let game_data = seeded_game_data();
    let mut events = MaintenanceEvents::default();
    events.fleet_battle_events.push(FleetBattleEvent {
        reporting_empire: 1,
        reporting_fleet_number: Some(12),
        reporting_mission: Some(Mission::MoveOnly),
        perspective: FleetBattlePerspective::Attacked,
        coords: [9, 6],
        enemy_empires: vec![2],
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
        surprise: false,
    });
    events
        .starbase_destroyed_events
        .push(nc_data::StarbaseDestroyedEvent {
            reporting_empire: 2,
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
            primary_enemy_empire: Some(1),
            primary_enemy_fleet_number: Some(12),
            stardate_week: Some(2),
        });

    assert_viewers_have_reports(&game_data, &events, &[1, 2]);
}

#[test]
fn no_engagement_composer_path_still_emits_a_viewer_report() {
    let mut game_data = seeded_game_data();
    configure_fleet(&mut game_data, 0, 1, 8, [9, 5]);

    let mut events = MaintenanceEvents::default();
    events
        .encounter_disposition_events
        .push(EncounterDispositionEvent::NoEngagement {
            fleet_idx: 0,
            owner_empire: 1,
            mission: Some(Mission::ScoutSector),
            coords: [9, 5],
            friendly_initial: ShipLosses {
                cruisers: 1,
                destroyers: 1,
                ..ShipLosses::default()
            },
            friendly_loaded_armies_initial: 0,
            target_empire: 2,
            target_fleet_number: Some(4),
            small_vessels: 1,
            medium_vessels: 2,
            large_vessels: 1,
            reason: EncounterDispositionReason::RoeDeclined,
            stardate_week: Some(2),
        });

    assert_viewers_have_reports(&game_data, &events, &[1]);
}
