use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use nc_data::{
    AssaultReportEvent, BombardEvent, CampaignStore, EmpireUnitSummary, FleetBattleEvent,
    GameStateBuilder, MaintenanceEvents, Mission, MissionOutcome, PlayerWarStatsState, ShipLosses,
    apply_maintenance_events_to_player_war_stats, default_player_activity_states,
};

static TEMP_DIR_SEQ: AtomicU64 = AtomicU64::new(0);

fn temp_dir(prefix: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "{prefix}-{}-{}-{}",
        std::process::id(),
        TEMP_DIR_SEQ.fetch_add(1, Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time ok")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("create temp dir");
    root
}

#[test]
fn maintenance_events_accumulate_lifetime_war_stats() {
    let mut states = vec![
        PlayerWarStatsState::for_player(1),
        PlayerWarStatsState::for_player(2),
    ];
    let mut events = MaintenanceEvents::default();
    events
        .colonization_events
        .push(nc_data::ColonizationResolvedEvent::Succeeded {
            fleet_idx: 0,
            colonizer_empire: 1,
            planet_idx: 0,
            stardate_week: Some(10),
        });
    events
        .ownership_change_events
        .push(nc_data::PlanetOwnershipChangeEvent {
            planet_idx: 0,
            reporting_empire: 2,
            previous_owner_empire: 2,
            new_owner_empire: 1,
            stardate_week: Some(12),
        });
    events.bombard_events.push(BombardEvent {
        planet_idx: 0,
        attacker_empire: 1,
        attacker_fleet_number: Some(1),
        defender_empire: 2,
        attacker_initial: ShipLosses::default(),
        attacker_loaded_armies_initial: 0,
        defender_batteries_initial: 3,
        defender_armies_initial: 4,
        attacker_losses: ShipLosses {
            destroyers: 1,
            ..ShipLosses::default()
        },
        defender_battery_losses: 2,
        defender_army_losses: 3,
        breakthrough: true,
        docked_losses: EmpireUnitSummary {
            transports: 2,
            ..EmpireUnitSummary::default()
        },
        stardock_items_destroyed: 2,
        stored_goods_destroyed: 0,
        factories_destroyed: 0,
        stardate_week: Some(14),
    });
    events.assault_report_events.push(AssaultReportEvent {
        kind: Mission::InvadeWorld,
        attacker_fleet_number: Some(2),
        planet_idx: 0,
        attacker_empire: 1,
        defender_empire: 2,
        attacker_initial: ShipLosses::default(),
        attacker_loaded_armies_initial: 3,
        defender_batteries_initial: 1,
        defender_armies_initial: 2,
        attacker_ship_losses: ShipLosses {
            transports: 1,
            ..ShipLosses::default()
        },
        attacker_army_losses: 3,
        transport_army_losses: 0,
        defender_battery_losses: 1,
        defender_army_losses_softening: 1,
        defender_army_losses: 2,
        inherited: None,
        outcome: MissionOutcome::Failed,
        stardate_week: Some(16),
    });
    events.fleet_battle_events.push(FleetBattleEvent {
        reporting_empire: 1,
        reporting_fleet_number: Some(3),
        reporting_mission: Some(Mission::PatrolSector),
        perspective: nc_data::maintenance_types::FleetBattlePerspective::Attacked,
        coords: [8, 8],
        enemy_empires: vec![2],
        primary_enemy_fleet_number: Some(4),
        held_field: true,
        friendly_initial: ShipLosses::default(),
        friendly_initial_starbases: 1,
        friendly_loaded_armies_initial: 0,
        friendly_losses: ShipLosses {
            cruisers: 1,
            ..ShipLosses::default()
        },
        friendly_starbases_lost: 1,
        enemy_initial: ShipLosses::default(),
        enemy_initial_starbases: 1,
        enemy_loaded_armies_initial: 0,
        enemy_losses: ShipLosses {
            battleships: 2,
            ..ShipLosses::default()
        },
        enemy_starbases_destroyed: 1,
        stardate_week: Some(18),
    });

    apply_maintenance_events_to_player_war_stats(&mut states, &events);

    assert_eq!(states[0].colonies_established, 1);
    assert_eq!(states[0].worlds_taken, 1);
    assert_eq!(states[0].bombardments_launched, 1);
    assert_eq!(states[0].invade_attempts, 1);
    assert_eq!(states[0].invade_successes, 0);
    assert_eq!(states[0].invade_failures(), 1);
    assert_eq!(states[0].enemy_units_destroyed.transports, 2);
    assert_eq!(states[0].enemy_units_destroyed.armies, 5);
    assert_eq!(states[0].enemy_units_destroyed.ground_batteries, 3);
    assert_eq!(states[0].enemy_units_destroyed.battleships, 2);
    assert_eq!(states[0].enemy_units_destroyed.starbases, 1);
    assert_eq!(states[0].units_lost.destroyers, 1);
    assert_eq!(states[0].units_lost.transports, 1);
    assert_eq!(states[0].units_lost.cruisers, 1);
    assert_eq!(states[0].units_lost.armies, 3);
    assert_eq!(states[0].units_lost.starbases, 1);

    assert_eq!(states[1].worlds_lost, 1);
    assert_eq!(states[1].bombardments_suffered, 1);
    assert_eq!(states[1].attacks_repelled, 1);
    assert_eq!(states[1].units_lost.transports, 2);
    assert_eq!(states[1].units_lost.armies, 5);
    assert_eq!(states[1].units_lost.ground_batteries, 3);
    assert_eq!(states[1].enemy_units_destroyed.destroyers, 1);
    assert_eq!(states[1].enemy_units_destroyed.transports, 1);
    assert_eq!(states[1].enemy_units_destroyed.armies, 3);
}

#[test]
fn runtime_store_persists_war_stats_rows() {
    let root = temp_dir("nc-data-war-stats");
    let store = CampaignStore::open_default_in_dir(&root).expect("open store");
    let game_data = GameStateBuilder::new()
        .with_player_count(4)
        .build_initialized_baseline()
        .expect("baseline");
    store
        .save_runtime_state_structured(
            game_data.conquest.game_year,
            &game_data,
            &BTreeSet::new(),
            &[],
            &[],
        )
        .expect("initial save");

    let initial = store
        .latest_player_war_stats(game_data.conquest.player_count)
        .expect("load initial war stats");
    assert_eq!(
        initial,
        vec![
            PlayerWarStatsState::for_player(1),
            PlayerWarStatsState::for_player(2),
            PlayerWarStatsState::for_player(3),
            PlayerWarStatsState::for_player(4),
        ]
    );

    let mut override_states = initial.clone();
    override_states[0].worlds_taken = 3;
    override_states[0].enemy_units_destroyed.destroyers = 4;
    override_states[0].enemy_units_destroyed.starbases = 1;
    override_states[1].worlds_lost = 2;
    override_states[1].units_lost.transports = 5;
    override_states[1].attacks_repelled = 1;

    let intel = vec![BTreeMap::new(); game_data.conquest.player_count as usize];
    let activity = default_player_activity_states(game_data.conquest.player_count);
    store
        .save_runtime_state_structured_with_intel_activity_and_war_stats(
            game_data.conquest.game_year,
            &game_data,
            &BTreeSet::new(),
            &[],
            &[],
            &intel,
            &activity,
            &override_states,
        )
        .expect("save war stats override");

    let reloaded = store
        .latest_player_war_stats(game_data.conquest.player_count)
        .expect("reload war stats");
    assert_eq!(reloaded, override_states);

    let _ = fs::remove_dir_all(root);
}
