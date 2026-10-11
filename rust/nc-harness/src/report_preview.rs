use nc_data::{
    Coords, Fleet, Planet, PlayerMode, Real48, ReportBlockRow, StarbaseAlert, recompute_max_speed,
    set_armies_aboard, set_empire_name, set_planet_name, set_player_handle, set_troop_transports,
};
use nc_engine::{
    AssaultReportEvent, BombardEvent, ContactReportSource, EncounterDispositionEvent,
    EncounterDispositionReason, FleetBattleEvent, FleetDestroyedEvent, GameRng, MaintenanceEvents,
    Mission, MissionEvent, MissionOutcome, Order, PlanetOwnershipChangeEvent, ScoutContactEvent,
    ShipLosses, build_results_report_blocks, build_seeded_initialized_game,
    maint::FleetBattlePerspective,
};

use crate::error::HarnessError;

const PREVIEW_PLAYER_COUNT: u8 = 4;
const PREVIEW_RNG_TAG: u64 = 0xEC15_5250_5456_5746;
const ATTACKER_EMPIRE: u8 = 1;
const DEFENDER_EMPIRE: u8 = 2;
const ATTACKER_FLEET_IDX: usize = 0;
const ATTACKER_FLEET_NUMBER: u8 = 7;
const DEFENDER_FLEET_NUMBER: u8 = 9;
const TARGET_PLANET_IDX: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewFamilyStatus {
    Implemented,
    Stub,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReportPreviewFamily {
    pub key: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub status: PreviewFamilyStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportPreviewQuery {
    pub family: String,
    pub seed: u64,
    pub samples: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewerReportSet {
    pub role: &'static str,
    pub empire_raw: u8,
    pub reports: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportPreviewCase {
    pub sample_index: usize,
    pub variant_label: String,
    pub asset_summary: Vec<String>,
    pub event_summary: Vec<String>,
    pub viewer_reports: Vec<ViewerReportSet>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportPreviewFamilyRun {
    pub family: ReportPreviewFamily,
    pub cases: Vec<ReportPreviewCase>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportPreviewRun {
    pub query: ReportPreviewQuery,
    pub family_runs: Vec<ReportPreviewFamilyRun>,
    pub requested_stub_family: Option<ReportPreviewFamily>,
    pub skipped_stub_families: Vec<ReportPreviewFamily>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ViewerRole {
    role: &'static str,
    empire_raw: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct ForceSpec {
    battleships: u16,
    cruisers: u16,
    destroyers: u16,
    scouts: u16,
    transports: u16,
    loaded_armies: u16,
    etacs: u16,
}

#[derive(Debug)]
struct PreviewScenario {
    variant_label: String,
    game_data: nc_data::CoreGameData,
    events: MaintenanceEvents,
    viewers: Vec<ViewerRole>,
    asset_summary: Vec<String>,
    event_summary: Vec<String>,
}

/// Registry of report-preview families.
///
/// Implemented families have synthetic generators wired below. Stub families are
/// intentionally discoverable now so the CLI can advertise the remaining
/// `build_results_report_blocks` coverage without pretending the generator
/// exists yet.
const REGISTERED_FAMILIES: &[ReportPreviewFamily] = &[
    ReportPreviewFamily {
        key: "bombard",
        category: "combat",
        description: "Bombardment attacker and defender wording.",
        status: PreviewFamilyStatus::Implemented,
    },
    ReportPreviewFamily {
        key: "invade",
        category: "combat",
        description: "Invasion attacker and defender/capture wording.",
        status: PreviewFamilyStatus::Implemented,
    },
    ReportPreviewFamily {
        key: "blitz",
        category: "combat",
        description: "Blitz attacker and defender/capture wording.",
        status: PreviewFamilyStatus::Implemented,
    },
    ReportPreviewFamily {
        key: "fleet-battle",
        category: "combat",
        description: "Battle reports from both sides of the same engagement.",
        status: PreviewFamilyStatus::Implemented,
    },
    ReportPreviewFamily {
        key: "fleet-destroyed",
        category: "combat",
        description: "Destroyed-fleet telemetry and surviving-side battle report.",
        status: PreviewFamilyStatus::Implemented,
    },
    ReportPreviewFamily {
        key: "scout-contact",
        category: "intel",
        description: "Scout/contact-family reports with visible force summaries.",
        status: PreviewFamilyStatus::Implemented,
    },
    ReportPreviewFamily {
        key: "encounter-retreated",
        category: "combat",
        description: "ROE retreat disposition reports.",
        status: PreviewFamilyStatus::Implemented,
    },
    ReportPreviewFamily {
        key: "encounter-no-engagement",
        category: "combat",
        description: "ROE no-engagement disposition reports.",
        status: PreviewFamilyStatus::Implemented,
    },
    ReportPreviewFamily {
        key: "encounter-pursuit-fire",
        category: "combat",
        description: "ROE pursuit-fire withdrawal reports.",
        status: PreviewFamilyStatus::Implemented,
    },
    ReportPreviewFamily {
        key: "ownership-change",
        category: "combat",
        description: "Capture notices paired with attacker mission success.",
        status: PreviewFamilyStatus::Implemented,
    },
    // Stub registry for the remaining results-composer families.
    ReportPreviewFamily {
        key: "move",
        category: "ops",
        description: "Move mission reports.",
        status: PreviewFamilyStatus::Stub,
    },
    ReportPreviewFamily {
        key: "seek-home",
        category: "ops",
        description: "Seek-home mission reports.",
        status: PreviewFamilyStatus::Stub,
    },
    ReportPreviewFamily {
        key: "patrol",
        category: "ops",
        description: "Patrol mission reports.",
        status: PreviewFamilyStatus::Stub,
    },
    ReportPreviewFamily {
        key: "view-world",
        category: "intel",
        description: "Viewing mission reports.",
        status: PreviewFamilyStatus::Stub,
    },
    ReportPreviewFamily {
        key: "guard-starbase",
        category: "ops",
        description: "Guard-starbase mission reports.",
        status: PreviewFamilyStatus::Stub,
    },
    ReportPreviewFamily {
        key: "guard-blockade",
        category: "ops",
        description: "Guard/blockade mission reports.",
        status: PreviewFamilyStatus::Stub,
    },
    ReportPreviewFamily {
        key: "scout-sector",
        category: "intel",
        description: "Scout-sector mission reports.",
        status: PreviewFamilyStatus::Stub,
    },
    ReportPreviewFamily {
        key: "scout-system",
        category: "intel",
        description: "Scout-solar-system mission reports.",
        status: PreviewFamilyStatus::Stub,
    },
    ReportPreviewFamily {
        key: "colonization",
        category: "ops",
        description: "Colonization outcome reports.",
        status: PreviewFamilyStatus::Stub,
    },
    ReportPreviewFamily {
        key: "salvage",
        category: "ops",
        description: "Salvage outcome reports.",
        status: PreviewFamilyStatus::Stub,
    },
    ReportPreviewFamily {
        key: "starbase-destroyed",
        category: "combat",
        description: "Destroyed-starbase telemetry reports.",
        status: PreviewFamilyStatus::Stub,
    },
    ReportPreviewFamily {
        key: "civil-disorder",
        category: "admin",
        description: "Civil disorder notices.",
        status: PreviewFamilyStatus::Stub,
    },
    ReportPreviewFamily {
        key: "campaign-outlook",
        category: "admin",
        description: "Campaign outlook reports.",
        status: PreviewFamilyStatus::Stub,
    },
    ReportPreviewFamily {
        key: "campaign-outcome",
        category: "admin",
        description: "Campaign outcome reports.",
        status: PreviewFamilyStatus::Stub,
    },
    ReportPreviewFamily {
        key: "invalid-player-state",
        category: "admin",
        description: "Sanitization and invalid-player-state reports.",
        status: PreviewFamilyStatus::Stub,
    },
    ReportPreviewFamily {
        key: "fleet-defection",
        category: "admin",
        description: "Fleet defection notices.",
        status: PreviewFamilyStatus::Stub,
    },
    ReportPreviewFamily {
        key: "fleet-merge",
        category: "ops",
        description: "Join/rendezvous merge reports.",
        status: PreviewFamilyStatus::Stub,
    },
    ReportPreviewFamily {
        key: "join-host",
        category: "ops",
        description: "Join-host retarget and destruction reports.",
        status: PreviewFamilyStatus::Stub,
    },
    ReportPreviewFamily {
        key: "mission-retarget",
        category: "ops",
        description: "Mission retarget and abandonment reports.",
        status: PreviewFamilyStatus::Stub,
    },
];

pub fn list_report_preview_families() -> Vec<ReportPreviewFamily> {
    REGISTERED_FAMILIES.to_vec()
}

pub fn run_report_preview(query: &ReportPreviewQuery) -> Result<ReportPreviewRun, HarnessError> {
    if query.samples == 0 {
        return Err(HarnessError::Validation(
            "report preview requires --samples >= 1".to_string(),
        ));
    }

    let normalized = query.family.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Err(HarnessError::Validation(
            "report preview requires --family <name|all>".to_string(),
        ));
    }

    if normalized == "all" {
        let mut family_runs = Vec::new();
        let mut skipped_stub_families = Vec::new();
        for family in REGISTERED_FAMILIES {
            match family.status {
                PreviewFamilyStatus::Implemented => {
                    family_runs.push(build_family_run(*family, query.seed, query.samples)?);
                }
                PreviewFamilyStatus::Stub => skipped_stub_families.push(*family),
            }
        }
        return Ok(ReportPreviewRun {
            query: query.clone(),
            family_runs,
            requested_stub_family: None,
            skipped_stub_families,
        });
    }

    let Some(family) = REGISTERED_FAMILIES
        .iter()
        .find(|family| family.key == normalized)
    else {
        return Err(HarnessError::Validation(format!(
            "unknown report preview family: {}",
            query.family
        )));
    };

    if matches!(family.status, PreviewFamilyStatus::Stub) {
        return Ok(ReportPreviewRun {
            query: query.clone(),
            family_runs: Vec::new(),
            requested_stub_family: Some(*family),
            skipped_stub_families: Vec::new(),
        });
    }

    Ok(ReportPreviewRun {
        query: query.clone(),
        family_runs: vec![build_family_run(*family, query.seed, query.samples)?],
        requested_stub_family: None,
        skipped_stub_families: Vec::new(),
    })
}

fn build_family_run(
    family: ReportPreviewFamily,
    seed: u64,
    samples: usize,
) -> Result<ReportPreviewFamilyRun, HarnessError> {
    let mut cases = Vec::with_capacity(samples);
    for sample_index in 0..samples {
        let scenario = build_preview_scenario(family.key, seed, sample_index)?;
        let rows = build_results_report_blocks(&scenario.game_data, &scenario.events);
        let viewer_reports = scenario
            .viewers
            .iter()
            .map(|viewer| ViewerReportSet {
                role: viewer.role,
                empire_raw: viewer.empire_raw,
                reports: viewer_report_texts(viewer.empire_raw, &rows),
            })
            .collect();
        cases.push(ReportPreviewCase {
            sample_index,
            variant_label: scenario.variant_label,
            asset_summary: scenario.asset_summary,
            event_summary: scenario.event_summary,
            viewer_reports,
        });
    }
    Ok(ReportPreviewFamilyRun { family, cases })
}

fn viewer_report_texts(viewer_empire_id: u8, rows: &[ReportBlockRow]) -> Vec<String> {
    rows.iter()
        .filter(|row| !row.recipient_deleted && row.is_visible_to_viewer(viewer_empire_id))
        .map(|row| row.decoded_text.clone())
        .collect()
}

fn build_preview_scenario(
    family_key: &str,
    seed: u64,
    sample_index: usize,
) -> Result<PreviewScenario, HarnessError> {
    match family_key {
        "bombard" => build_bombard_preview(seed, sample_index),
        "invade" => build_invade_preview(seed, sample_index),
        "blitz" => build_blitz_preview(seed, sample_index),
        "fleet-battle" => build_fleet_battle_preview(seed, sample_index),
        "fleet-destroyed" => build_fleet_destroyed_preview(seed, sample_index),
        "scout-contact" => build_scout_contact_preview(seed, sample_index),
        "encounter-retreated" => build_encounter_retreated_preview(seed, sample_index),
        "encounter-no-engagement" => build_encounter_no_engagement_preview(seed, sample_index),
        "encounter-pursuit-fire" => build_encounter_pursuit_fire_preview(seed, sample_index),
        "ownership-change" => build_ownership_change_preview(seed, sample_index),
        other => Err(HarnessError::Validation(format!(
            "report preview family is not implemented: {other}"
        ))),
    }
}

fn build_bombard_preview(seed: u64, sample_index: usize) -> Result<PreviewScenario, HarnessError> {
    let mut world = PreviewWorld::new("bombard", seed, sample_index)?;
    let mut rng = family_rng("bombard", seed, sample_index);
    let attacker_force = sample_bombard_force(&mut rng);
    let variant = match sample_index % 3 {
        0 => BombardVariant::DefendedBreakthrough,
        1 => BombardVariant::UndefendedInfrastructure,
        _ => BombardVariant::DefendedNoBreakthrough,
    };
    let (planet_name, defense_armies, defense_batteries, army_losses, battery_losses, breakthrough) =
        match variant {
            BombardVariant::DefendedBreakthrough => ("Harrow", 8, 3, 3, 2, true),
            BombardVariant::UndefendedInfrastructure => ("Relay", 0, 0, 0, 0, true),
            BombardVariant::DefendedNoBreakthrough => ("Rampart", 9, 4, 0, 1, false),
        };
    let stored_goods_destroyed = if breakthrough {
        u32::from(rng.range_u8(6, 24))
    } else {
        0
    };
    let factories_destroyed = if breakthrough {
        rng.range_u8(8, 42) as u16
    } else {
        0
    };
    let stardock_items_destroyed =
        if breakthrough && variant != BombardVariant::UndefendedInfrastructure {
            u32::from(rng.range_u8(1, 3))
        } else {
            0
        };
    configure_target_planet(
        &mut world.game_data,
        planet_name,
        world.target_coords,
        DEFENDER_EMPIRE,
        defense_armies,
        defense_batteries,
    );
    configure_fleet(
        &mut world.game_data.fleets[ATTACKER_FLEET_IDX],
        ATTACKER_EMPIRE,
        ATTACKER_FLEET_NUMBER,
        world.target_coords,
        attacker_force,
        Order::BombardWorld,
        world.target_coords,
    );
    let attacker_losses = if variant == BombardVariant::UndefendedInfrastructure {
        ShipLosses::default()
    } else {
        ShipLosses {
            destroyers: u32::from(rng.range_u8(0, 2)),
            cruisers: u32::from(rng.range_u8(0, 1)),
            battleships: u32::from(rng.range_u8(0, 1)),
            ..ShipLosses::default()
        }
    };
    let week = Some(sample_week(sample_index));
    world.events.bombard_events.push(BombardEvent {
        planet_idx: TARGET_PLANET_IDX,
        attacker_empire_raw: ATTACKER_EMPIRE,
        attacker_fleet_number: Some(ATTACKER_FLEET_NUMBER),
        defender_empire_raw: DEFENDER_EMPIRE,
        attacker_initial: attacker_force.to_ship_losses(),
        attacker_loaded_armies_initial: attacker_force.loaded_armies(),
        defender_batteries_initial: defense_batteries,
        defender_armies_initial: defense_armies,
        attacker_losses,
        defender_battery_losses: battery_losses,
        defender_army_losses: army_losses,
        breakthrough,
        docked_losses: nc_data::EmpireUnitSummary::default(),
        stardock_items_destroyed,
        stored_goods_destroyed,
        factories_destroyed,
        stardate_week: week,
    });
    world.events.mission_events.push(MissionEvent {
        fleet_idx: ATTACKER_FLEET_IDX,
        owner_empire_raw: ATTACKER_EMPIRE,
        kind: Mission::BombardWorld,
        outcome: MissionOutcome::Succeeded,
        abort_reason: None,
        planet_idx: Some(TARGET_PLANET_IDX),
        location_coords: Some(world.target_coords),
        target_coords: Some(world.target_coords),
        stardate_week: week,
    });
    let coords = world.target_coords;
    world.finish(
        variant.label(),
        vec![
            format!(
                "attacker {} at System({},{})",
                describe_force(attacker_force),
                coords[0],
                coords[1]
            ),
            format!(
                "defender world \"{planet_name}\" defenses: {}",
                describe_ground_forces(defense_batteries, defense_armies)
            ),
        ],
    )
}

fn build_invade_preview(seed: u64, sample_index: usize) -> Result<PreviewScenario, HarnessError> {
    build_assault_preview(seed, sample_index, Mission::InvadeWorld)
}

fn build_blitz_preview(seed: u64, sample_index: usize) -> Result<PreviewScenario, HarnessError> {
    build_assault_preview(seed, sample_index, Mission::BlitzWorld)
}

fn build_assault_preview(
    seed: u64,
    sample_index: usize,
    mission: Mission,
) -> Result<PreviewScenario, HarnessError> {
    let family_key = match mission {
        Mission::InvadeWorld => "invade",
        Mission::BlitzWorld => "blitz",
        _ => unreachable!(),
    };
    let mut world = PreviewWorld::new(family_key, seed, sample_index)?;
    let mut rng = family_rng(family_key, seed, sample_index);
    let attacker_force = sample_assault_force(&mut rng);
    let variant = match sample_index % 3 {
        0 => AssaultVariant::Succeeded,
        1 => AssaultVariant::Failed,
        _ => AssaultVariant::Aborted,
    };
    let defense_armies = rng.range_u8(4, 11);
    let defense_batteries = rng.range_u8(1, 4);
    let target_name = if mission == Mission::InvadeWorld {
        "Redoubt"
    } else {
        "Cutlass"
    };
    configure_target_planet(
        &mut world.game_data,
        target_name,
        world.target_coords,
        DEFENDER_EMPIRE,
        defense_armies,
        defense_batteries,
    );
    configure_fleet(
        &mut world.game_data.fleets[ATTACKER_FLEET_IDX],
        ATTACKER_EMPIRE,
        ATTACKER_FLEET_NUMBER,
        world.target_coords,
        attacker_force,
        mission_to_order(mission),
        world.target_coords,
    );
    let outcome = variant.outcome();
    let week = Some(sample_week(sample_index));
    let attacker_ship_losses = ShipLosses {
        destroyers: u32::from(rng.range_u8(0, 2)),
        cruisers: u32::from(rng.range_u8(0, 1)),
        battleships: u32::from(rng.range_u8(0, 1)),
        transports: u32::from(rng.range_u8(0, 1)).min(attacker_force.transports as u32),
        ..ShipLosses::default()
    };
    let attacker_army_losses =
        u32::from(rng.range_u8(0, attacker_force.loaded_armies.min(5) as u8));
    let defender_battery_losses = match variant {
        AssaultVariant::Succeeded => defense_batteries,
        AssaultVariant::Failed => defense_batteries.saturating_sub(1),
        AssaultVariant::Aborted => defense_batteries.min(1),
    };
    let defender_army_losses = match variant {
        AssaultVariant::Succeeded => defense_armies,
        AssaultVariant::Failed => defense_armies.saturating_sub(rng.range_u8(0, 2)),
        AssaultVariant::Aborted => defense_armies.min(1),
    };
    world.events.assault_report_events.push(AssaultReportEvent {
        kind: mission,
        attacker_fleet_number: Some(ATTACKER_FLEET_NUMBER),
        planet_idx: TARGET_PLANET_IDX,
        attacker_empire_raw: ATTACKER_EMPIRE,
        defender_empire_raw: DEFENDER_EMPIRE,
        attacker_initial: attacker_force.to_ship_losses(),
        attacker_loaded_armies_initial: attacker_force.loaded_armies(),
        defender_batteries_initial: defense_batteries,
        defender_armies_initial: defense_armies,
        attacker_ship_losses,
        attacker_army_losses,
        transport_army_losses: attacker_ship_losses.transports * 2,
        defender_battery_losses,
        defender_army_losses_softening: if mission == Mission::InvadeWorld {
            defender_army_losses / 2
        } else {
            0
        },
        defender_army_losses,
        outcome,
        stardate_week: week,
    });
    world.events.mission_events.push(MissionEvent {
        fleet_idx: ATTACKER_FLEET_IDX,
        owner_empire_raw: ATTACKER_EMPIRE,
        kind: mission,
        outcome,
        abort_reason: None,
        planet_idx: Some(TARGET_PLANET_IDX),
        location_coords: Some(world.target_coords),
        target_coords: Some(world.target_coords),
        stardate_week: week,
    });
    if matches!(variant, AssaultVariant::Succeeded) {
        world
            .events
            .ownership_change_events
            .push(PlanetOwnershipChangeEvent {
                planet_idx: TARGET_PLANET_IDX,
                reporting_empire_raw: DEFENDER_EMPIRE,
                previous_owner_empire_raw: DEFENDER_EMPIRE,
                new_owner_empire_raw: ATTACKER_EMPIRE,
                stardate_week: week,
            });
    }
    let coords = world.target_coords;
    world.finish(
        variant.label(mission),
        vec![
            format!(
                "attacker {} at System({},{})",
                describe_force(attacker_force),
                coords[0],
                coords[1]
            ),
            format!(
                "defender world \"{target_name}\" defenses: {}",
                describe_ground_forces(defense_batteries, defense_armies)
            ),
        ],
    )
}

fn build_fleet_battle_preview(
    seed: u64,
    sample_index: usize,
) -> Result<PreviewScenario, HarnessError> {
    let mut world = PreviewWorld::new("fleet-battle", seed, sample_index)?;
    let mut rng = family_rng("fleet-battle", seed, sample_index);
    let attacker_force = sample_battle_force(&mut rng);
    let defender_force = sample_battle_force(&mut rng);
    let held_field = sample_index % 2 == 0;
    let coords = world.target_coords;
    let week = Some(sample_week(sample_index));
    let attacker_losses = ShipLosses {
        destroyers: u32::from(rng.range_u8(0, 2)),
        cruisers: u32::from(rng.range_u8(0, 1)),
        transports: u32::from(rng.range_u8(0, 1)),
        ..ShipLosses::default()
    };
    let defender_losses = ShipLosses {
        destroyers: u32::from(rng.range_u8(0, 2)),
        cruisers: u32::from(rng.range_u8(0, 1)),
        battleships: u32::from(rng.range_u8(0, 1)),
        ..ShipLosses::default()
    };
    world.events.fleet_battle_events.push(FleetBattleEvent {
        reporting_empire_raw: ATTACKER_EMPIRE,
        reporting_fleet_number: Some(ATTACKER_FLEET_NUMBER),
        reporting_mission: Some(Mission::GuardBlockadeWorld),
        perspective: if held_field {
            FleetBattlePerspective::Intercepted
        } else {
            FleetBattlePerspective::Attacked
        },
        coords,
        enemy_empires_raw: vec![DEFENDER_EMPIRE],
        primary_enemy_fleet_number: Some(DEFENDER_FLEET_NUMBER),
        held_field,
        friendly_initial: attacker_force.to_ship_losses(),
        friendly_initial_starbases: 0,
        friendly_loaded_armies_initial: attacker_force.loaded_armies(),
        friendly_losses: attacker_losses,
        friendly_starbases_lost: 0,
        enemy_initial: defender_force.to_ship_losses(),
        enemy_initial_starbases: 0,
        enemy_loaded_armies_initial: defender_force.loaded_armies(),
        enemy_losses: defender_losses,
        enemy_starbases_destroyed: 0,
        stardate_week: week,
    });
    world.events.fleet_battle_events.push(FleetBattleEvent {
        reporting_empire_raw: DEFENDER_EMPIRE,
        reporting_fleet_number: Some(DEFENDER_FLEET_NUMBER),
        reporting_mission: Some(Mission::GuardBlockadeWorld),
        perspective: if held_field {
            FleetBattlePerspective::Attacked
        } else {
            FleetBattlePerspective::Intercepted
        },
        coords,
        enemy_empires_raw: vec![ATTACKER_EMPIRE],
        primary_enemy_fleet_number: Some(ATTACKER_FLEET_NUMBER),
        held_field: !held_field,
        friendly_initial: defender_force.to_ship_losses(),
        friendly_initial_starbases: 0,
        friendly_loaded_armies_initial: defender_force.loaded_armies(),
        friendly_losses: defender_losses,
        friendly_starbases_lost: 0,
        enemy_initial: attacker_force.to_ship_losses(),
        enemy_initial_starbases: 0,
        enemy_loaded_armies_initial: attacker_force.loaded_armies(),
        enemy_losses: attacker_losses,
        enemy_starbases_destroyed: 0,
        stardate_week: week,
    });
    world.finish(
        if held_field {
            "intercepted-held-field".to_string()
        } else {
            "ambushed-lost-field".to_string()
        },
        vec![
            format!("attacker fleet: {}", describe_force(attacker_force)),
            format!("defender fleet: {}", describe_force(defender_force)),
        ],
    )
}

fn build_fleet_destroyed_preview(
    seed: u64,
    sample_index: usize,
) -> Result<PreviewScenario, HarnessError> {
    let mut world = PreviewWorld::new("fleet-destroyed", seed, sample_index)?;
    let mut rng = family_rng("fleet-destroyed", seed, sample_index);
    let attacker_force = sample_battle_force(&mut rng);
    let defender_force = sample_destroyed_fleet_force(&mut rng);
    let coords = world.target_coords;
    let week = Some(sample_week(sample_index));
    let enemy_starbases = if sample_index % 3 == 2 { 1 } else { 0 };
    let enemy_losses = if enemy_starbases == 0 {
        ShipLosses {
            destroyers: u32::from(rng.range_u8(0, 1)),
            ..ShipLosses::default()
        }
    } else {
        ShipLosses::default()
    };
    world
        .events
        .fleet_destroyed_events
        .push(FleetDestroyedEvent {
            reporting_empire_raw: DEFENDER_EMPIRE,
            fleet_number: DEFENDER_FLEET_NUMBER,
            coords,
            was_intercepting: false,
            friendly_initial: defender_force.to_ship_losses(),
            friendly_loaded_armies_initial: defender_force.loaded_armies(),
            enemy_initial: attacker_force.to_ship_losses(),
            enemy_initial_starbases: enemy_starbases,
            enemy_loaded_armies_initial: attacker_force.loaded_armies(),
            enemy_ground_batteries_initial: 0,
            enemy_ground_armies_initial: 0,
            enemy_losses,
            enemy_starbases_destroyed: 0,
            enemy_ground_battery_losses: 0,
            enemy_ground_army_losses: 0,
            primary_enemy_empire_raw: Some(ATTACKER_EMPIRE),
            primary_enemy_fleet_number: Some(ATTACKER_FLEET_NUMBER),
            stardate_week: week,
        });
    world.events.fleet_battle_events.push(FleetBattleEvent {
        reporting_empire_raw: ATTACKER_EMPIRE,
        reporting_fleet_number: Some(ATTACKER_FLEET_NUMBER),
        reporting_mission: Some(Mission::GuardBlockadeWorld),
        perspective: FleetBattlePerspective::Intercepted,
        coords,
        enemy_empires_raw: vec![DEFENDER_EMPIRE],
        primary_enemy_fleet_number: Some(DEFENDER_FLEET_NUMBER),
        held_field: true,
        friendly_initial: attacker_force.to_ship_losses(),
        friendly_initial_starbases: 0,
        friendly_loaded_armies_initial: attacker_force.loaded_armies(),
        friendly_losses: ShipLosses::default(),
        friendly_starbases_lost: 0,
        enemy_initial: defender_force.to_ship_losses(),
        enemy_initial_starbases: 0,
        enemy_loaded_armies_initial: defender_force.loaded_armies(),
        enemy_losses: defender_force.to_ship_losses(),
        enemy_starbases_destroyed: 0,
        stardate_week: week,
    });
    world.finish(
        if enemy_starbases > 0 {
            "destroyed-by-starbase-supported-force".to_string()
        } else {
            "destroyed-by-fleet".to_string()
        },
        vec![
            format!("surviving fleet: {}", describe_force(attacker_force)),
            format!("lost fleet: {}", describe_force(defender_force)),
        ],
    )
}

fn build_scout_contact_preview(
    seed: u64,
    sample_index: usize,
) -> Result<PreviewScenario, HarnessError> {
    let mut world = PreviewWorld::new("scout-contact", seed, sample_index)?;
    let mut rng = family_rng("scout-contact", seed, sample_index);
    let viewer_force = sample_contact_force(&mut rng);
    let source = match sample_index % 3 {
        0 => ContactReportSource::FleetMission(Mission::ScoutSector),
        1 => ContactReportSource::Fleet(ATTACKER_FLEET_NUMBER),
        _ => ContactReportSource::Starbase {
            id: 2,
            alert: StarbaseAlert::Enemy,
        },
    };
    let small = u32::from(rng.range_u8(1, 4));
    let medium = u32::from(rng.range_u8(0, 3));
    let large = u32::from(rng.range_u8(0, 2));
    world.events.scout_contact_events.push(ScoutContactEvent {
        viewer_empire_raw: ATTACKER_EMPIRE,
        source,
        reporting_fleet_number: Some(ATTACKER_FLEET_NUMBER),
        reporting_initial: viewer_force.to_ship_losses(),
        reporting_loaded_armies_initial: viewer_force.loaded_armies(),
        coords: world.target_coords,
        target_empire_raw: DEFENDER_EMPIRE,
        target_fleet_number: Some(DEFENDER_FLEET_NUMBER),
        small_vessels: small,
        medium_vessels: medium,
        large_vessels: large,
        stardate_week: Some(sample_week(sample_index)),
    });
    world.viewers = vec![
        ViewerRole {
            role: "viewer",
            empire_raw: ATTACKER_EMPIRE,
        },
        ViewerRole {
            role: "target",
            empire_raw: DEFENDER_EMPIRE,
        },
    ];
    world.finish(
        match source {
            ContactReportSource::FleetMission(_) => "fleet-mission-contact".to_string(),
            ContactReportSource::Fleet(_) => "fleet-contact".to_string(),
            ContactReportSource::Starbase { .. } => "starbase-contact".to_string(),
        },
        vec![
            format!("viewer force: {}", describe_force(viewer_force)),
            format!("detected contact mix: {small} small, {medium} medium, {large} large"),
        ],
    )
}

fn build_encounter_retreated_preview(
    seed: u64,
    sample_index: usize,
) -> Result<PreviewScenario, HarnessError> {
    build_encounter_preview(seed, sample_index, EncounterPreviewKind::Retreated)
}

fn build_encounter_no_engagement_preview(
    seed: u64,
    sample_index: usize,
) -> Result<PreviewScenario, HarnessError> {
    build_encounter_preview(seed, sample_index, EncounterPreviewKind::NoEngagement)
}

fn build_encounter_pursuit_fire_preview(
    seed: u64,
    sample_index: usize,
) -> Result<PreviewScenario, HarnessError> {
    build_encounter_preview(seed, sample_index, EncounterPreviewKind::PursuitFire)
}

fn build_encounter_preview(
    seed: u64,
    sample_index: usize,
    kind: EncounterPreviewKind,
) -> Result<PreviewScenario, HarnessError> {
    let family_key = kind.family_key();
    let mut world = PreviewWorld::new(family_key, seed, sample_index)?;
    let mut rng = family_rng(family_key, seed, sample_index);
    let friendly = sample_contact_force(&mut rng);
    let enemy = sample_battle_force(&mut rng);
    configure_fleet(
        &mut world.game_data.fleets[ATTACKER_FLEET_IDX],
        ATTACKER_EMPIRE,
        ATTACKER_FLEET_NUMBER,
        world.target_coords,
        friendly,
        Order::ScoutSector,
        world.target_coords,
    );
    let reason = if sample_index % 2 == 0 {
        EncounterDispositionReason::RoeWithdrawal
    } else {
        EncounterDispositionReason::RoeDeclined
    };
    let week = Some(sample_week(sample_index));
    match kind {
        EncounterPreviewKind::NoEngagement => {
            world.events.encounter_disposition_events.push(
                EncounterDispositionEvent::NoEngagement {
                    fleet_idx: ATTACKER_FLEET_IDX,
                    owner_empire_raw: ATTACKER_EMPIRE,
                    mission: Some(Mission::ScoutSector),
                    coords: world.target_coords,
                    friendly_initial: friendly.to_ship_losses(),
                    friendly_loaded_armies_initial: friendly.loaded_armies(),
                    target_empire_raw: DEFENDER_EMPIRE,
                    target_fleet_number: Some(DEFENDER_FLEET_NUMBER),
                    small_vessels: u32::from(rng.range_u8(1, 4)),
                    medium_vessels: u32::from(rng.range_u8(0, 2)),
                    large_vessels: u32::from(rng.range_u8(0, 2)),
                    reason,
                    stardate_week: week,
                },
            );
        }
        EncounterPreviewKind::Retreated => {
            world
                .events
                .encounter_disposition_events
                .push(EncounterDispositionEvent::Retreated {
                    fleet_idx: ATTACKER_FLEET_IDX,
                    owner_empire_raw: ATTACKER_EMPIRE,
                    mission: Some(Mission::InvadeWorld),
                    coords: world.target_coords,
                    friendly_initial: friendly.to_ship_losses(),
                    friendly_loaded_armies_initial: friendly.loaded_armies(),
                    target_empire_raw: DEFENDER_EMPIRE,
                    target_fleet_number: Some(DEFENDER_FLEET_NUMBER),
                    enemy_initial: enemy.to_ship_losses(),
                    retreat_target_coords: retreat_coords(world.target_coords),
                    losses_sustained: ShipLosses {
                        destroyers: u32::from(rng.range_u8(0, 1)),
                        transports: u32::from(rng.range_u8(0, 1)),
                        ..ShipLosses::default()
                    },
                    enemy_losses_inflicted: ShipLosses {
                        destroyers: u32::from(rng.range_u8(0, 1)),
                        ..ShipLosses::default()
                    },
                    reason,
                    stardate_week: week,
                });
            world.events.mission_events.push(MissionEvent {
                fleet_idx: ATTACKER_FLEET_IDX,
                owner_empire_raw: ATTACKER_EMPIRE,
                kind: Mission::InvadeWorld,
                outcome: MissionOutcome::Aborted,
                abort_reason: None,
                planet_idx: Some(TARGET_PLANET_IDX),
                location_coords: Some(world.target_coords),
                target_coords: Some(world.target_coords),
                stardate_week: week,
            });
        }
        EncounterPreviewKind::PursuitFire => {
            world.events.encounter_disposition_events.push(
                EncounterDispositionEvent::PursuitFire {
                    fleet_idx: ATTACKER_FLEET_IDX,
                    owner_empire_raw: ATTACKER_EMPIRE,
                    mission: Some(Mission::BombardWorld),
                    coords: world.target_coords,
                    friendly_initial: friendly.to_ship_losses(),
                    friendly_loaded_armies_initial: friendly.loaded_armies(),
                    target_empire_raw: DEFENDER_EMPIRE,
                    target_fleet_number: Some(DEFENDER_FLEET_NUMBER),
                    enemy_initial: enemy.to_ship_losses(),
                    retreat_target_coords: retreat_coords(world.target_coords),
                    losses_sustained: ShipLosses {
                        cruisers: u32::from(rng.range_u8(0, 1)),
                        destroyers: u32::from(rng.range_u8(0, 2)),
                        ..ShipLosses::default()
                    },
                    enemy_losses_inflicted: ShipLosses {
                        destroyers: u32::from(rng.range_u8(0, 1)),
                        ..ShipLosses::default()
                    },
                    reason,
                    stardate_week: week,
                },
            );
        }
    }
    world.viewers = vec![
        ViewerRole {
            role: "reporting-fleet",
            empire_raw: ATTACKER_EMPIRE,
        },
        ViewerRole {
            role: "target",
            empire_raw: DEFENDER_EMPIRE,
        },
    ];
    world.finish(
        kind.variant_label(reason),
        vec![
            format!("reporting force: {}", describe_force(friendly)),
            format!("contacted force: {}", describe_force(enemy)),
        ],
    )
}

fn build_ownership_change_preview(
    seed: u64,
    sample_index: usize,
) -> Result<PreviewScenario, HarnessError> {
    let mission = if sample_index % 2 == 0 {
        Mission::InvadeWorld
    } else {
        Mission::BlitzWorld
    };
    let mut world = PreviewWorld::new("ownership-change", seed, sample_index)?;
    let mut rng = family_rng("ownership-change", seed, sample_index);
    let attacker_force = sample_assault_force(&mut rng);
    let defense_armies = rng.range_u8(4, 9);
    let defense_batteries = rng.range_u8(1, 3);
    configure_target_planet(
        &mut world.game_data,
        "Farside",
        world.target_coords,
        DEFENDER_EMPIRE,
        defense_armies,
        defense_batteries,
    );
    configure_fleet(
        &mut world.game_data.fleets[ATTACKER_FLEET_IDX],
        ATTACKER_EMPIRE,
        ATTACKER_FLEET_NUMBER,
        world.target_coords,
        attacker_force,
        mission_to_order(mission),
        world.target_coords,
    );
    let week = Some(sample_week(sample_index));
    world.events.assault_report_events.push(AssaultReportEvent {
        kind: mission,
        attacker_fleet_number: Some(ATTACKER_FLEET_NUMBER),
        planet_idx: TARGET_PLANET_IDX,
        attacker_empire_raw: ATTACKER_EMPIRE,
        defender_empire_raw: DEFENDER_EMPIRE,
        attacker_initial: attacker_force.to_ship_losses(),
        attacker_loaded_armies_initial: attacker_force.loaded_armies(),
        defender_batteries_initial: defense_batteries,
        defender_armies_initial: defense_armies,
        attacker_ship_losses: ShipLosses {
            destroyers: u32::from(rng.range_u8(0, 1)),
            transports: u32::from(rng.range_u8(0, 1)),
            ..ShipLosses::default()
        },
        attacker_army_losses: u32::from(rng.range_u8(0, 2)),
        transport_army_losses: 0,
        defender_battery_losses: defense_batteries,
        defender_army_losses_softening: if mission == Mission::InvadeWorld {
            defense_armies / 2
        } else {
            0
        },
        defender_army_losses: defense_armies,
        outcome: MissionOutcome::Succeeded,
        stardate_week: week,
    });
    world
        .events
        .ownership_change_events
        .push(PlanetOwnershipChangeEvent {
            planet_idx: TARGET_PLANET_IDX,
            reporting_empire_raw: DEFENDER_EMPIRE,
            previous_owner_empire_raw: DEFENDER_EMPIRE,
            new_owner_empire_raw: ATTACKER_EMPIRE,
            stardate_week: week,
        });
    world.events.mission_events.push(MissionEvent {
        fleet_idx: ATTACKER_FLEET_IDX,
        owner_empire_raw: ATTACKER_EMPIRE,
        kind: mission,
        outcome: MissionOutcome::Succeeded,
        abort_reason: None,
        planet_idx: Some(TARGET_PLANET_IDX),
        location_coords: Some(world.target_coords),
        target_coords: Some(world.target_coords),
        stardate_week: week,
    });
    world.finish(
        format!("{}-capture", mission_label(mission)),
        vec![
            format!("capturing force: {}", describe_force(attacker_force)),
            format!(
                "captured world defenses: {}",
                describe_ground_forces(defense_batteries, defense_armies)
            ),
        ],
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BombardVariant {
    DefendedBreakthrough,
    UndefendedInfrastructure,
    DefendedNoBreakthrough,
}

impl BombardVariant {
    fn label(self) -> String {
        match self {
            Self::DefendedBreakthrough => "defended-breakthrough".to_string(),
            Self::UndefendedInfrastructure => "undefended-infrastructure".to_string(),
            Self::DefendedNoBreakthrough => "defended-no-breakthrough".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AssaultVariant {
    Succeeded,
    Failed,
    Aborted,
}

impl AssaultVariant {
    fn outcome(self) -> MissionOutcome {
        match self {
            Self::Succeeded => MissionOutcome::Succeeded,
            Self::Failed => MissionOutcome::Failed,
            Self::Aborted => MissionOutcome::Aborted,
        }
    }

    fn label(self, mission: Mission) -> String {
        format!("{}-{}", mission_label(mission), self.outcome_label())
    }

    fn outcome_label(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Aborted => "aborted",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EncounterPreviewKind {
    NoEngagement,
    Retreated,
    PursuitFire,
}

impl EncounterPreviewKind {
    fn family_key(self) -> &'static str {
        match self {
            Self::NoEngagement => "encounter-no-engagement",
            Self::Retreated => "encounter-retreated",
            Self::PursuitFire => "encounter-pursuit-fire",
        }
    }

    fn variant_label(self, reason: EncounterDispositionReason) -> String {
        let reason = match reason {
            EncounterDispositionReason::RoeDeclined => "roe-declined",
            EncounterDispositionReason::RoeWithdrawal => "roe-withdrawal",
            EncounterDispositionReason::InterceptFailed => "intercept-failed",
        };
        match self {
            Self::NoEngagement => format!("no-engagement-{reason}"),
            Self::Retreated => format!("retreated-{reason}"),
            Self::PursuitFire => format!("pursuit-fire-{reason}"),
        }
    }
}

struct PreviewWorld {
    game_data: nc_data::CoreGameData,
    events: MaintenanceEvents,
    viewers: Vec<ViewerRole>,
    target_coords: [u8; 2],
}

impl PreviewWorld {
    fn new(family_key: &str, seed: u64, sample_index: usize) -> Result<Self, HarnessError> {
        let target_coords = target_coords_for(seed, family_key, sample_index);
        let year = 3001 + (sample_index as u16 % 15);
        let mut game_data = build_seeded_initialized_game(PREVIEW_PLAYER_COUNT, year, seed)
            .map_err(HarnessError::Mutation)?;
        configure_named_player(&mut game_data, ATTACKER_EMPIRE, "p1", "Aurora League");
        configure_named_player(&mut game_data, DEFENDER_EMPIRE, "p2", "Helios Crown");
        Ok(Self {
            game_data,
            events: MaintenanceEvents::default(),
            viewers: vec![
                ViewerRole {
                    role: "attacker",
                    empire_raw: ATTACKER_EMPIRE,
                },
                ViewerRole {
                    role: "defender",
                    empire_raw: DEFENDER_EMPIRE,
                },
            ],
            target_coords,
        })
    }

    fn finish(
        self,
        variant_label: String,
        asset_summary: Vec<String>,
    ) -> Result<PreviewScenario, HarnessError> {
        Ok(PreviewScenario {
            variant_label,
            event_summary: summarize_events(&self.events),
            game_data: self.game_data,
            events: self.events,
            viewers: self.viewers,
            asset_summary,
        })
    }
}

impl ForceSpec {
    fn to_ship_losses(self) -> ShipLosses {
        ShipLosses {
            battleships: self.battleships.into(),
            cruisers: self.cruisers.into(),
            destroyers: self.destroyers.into(),
            scouts: self.scouts.into(),
            transports: self.transports.into(),
            etacs: self.etacs.into(),
        }
    }

    fn loaded_armies(self) -> u32 {
        self.loaded_armies.into()
    }
}

fn summarize_events(events: &MaintenanceEvents) -> Vec<String> {
    vec![
        format!("bombard_events={}", events.bombard_events.len()),
        format!(
            "assault_report_events={}",
            events.assault_report_events.len()
        ),
        format!("fleet_battle_events={}", events.fleet_battle_events.len()),
        format!(
            "fleet_destroyed_events={}",
            events.fleet_destroyed_events.len()
        ),
        format!("scout_contact_events={}", events.scout_contact_events.len()),
        format!(
            "encounter_disposition_events={}",
            events.encounter_disposition_events.len()
        ),
        format!(
            "ownership_change_events={}",
            events.ownership_change_events.len()
        ),
        format!("mission_events={}", events.mission_events.len()),
    ]
}

fn target_coords_for(seed: u64, family_key: &str, sample_index: usize) -> [u8; 2] {
    let mut rng = family_rng(family_key, seed, sample_index);
    [rng.range_u8(4, 32), rng.range_u8(4, 32)]
}

fn family_rng(family_key: &str, seed: u64, sample_index: usize) -> GameRng {
    GameRng::from_context(
        seed,
        PREVIEW_RNG_TAG,
        &[hash_label(family_key), sample_index as u64],
    )
}

fn hash_label(label: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in label.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}

fn configure_named_player(
    game_data: &mut nc_data::CoreGameData,
    empire_raw: u8,
    handle: &str,
    empire_name: &str,
) {
    if let Some(player) = game_data.players.get_mut((empire_raw - 1) as usize) {
        player.mode = PlayerMode::from_raw(empire_raw);
        set_player_handle(player, handle);
        set_empire_name(player, empire_name);
    }
}

fn configure_target_planet(
    game_data: &mut nc_data::CoreGameData,
    name: &str,
    coords: [u8; 2],
    owner_empire_raw: u8,
    armies: u8,
    batteries: u8,
) {
    let planet = &mut game_data.planets[TARGET_PLANET_IDX];
    *planet = Planet::default();
    planet.sector = Coords::from_array(coords);
    set_planet_name(planet, name);
    planet.owner.0 = owner_empire_raw;
    planet.years_under_owner = if owner_empire_raw == 0 { 0 } else { 2 };
    planet.ground_batteries = u16::from(batteries);
    planet.armies = u16::from(armies);
    planet.potential_production = 120;
    planet.production = Real48::from_f64(95.0);
    planet.stored_goods = Real48::from_f64(45.0);
}

fn configure_fleet(
    fleet: &mut Fleet,
    owner_empire_raw: u8,
    fleet_number: u8,
    coords: [u8; 2],
    force: ForceSpec,
    order: Order,
    target: [u8; 2],
) {
    *fleet = Fleet::default();
    fleet.owner.0 = owner_empire_raw;
    fleet.fleet_number.0 = fleet_number.into();
    fleet.sector = Coords::from_array(coords);
    fleet.order = order;
    fleet.target = Coords::from_array(target);
    fleet.rules_of_engagement = 10;
    fleet.ships.scouts = force.scouts;
    fleet.ships.battleships = force.battleships;
    fleet.ships.cruisers = force.cruisers;
    fleet.ships.destroyers = force.destroyers;
    set_troop_transports(fleet, force.transports);
    set_armies_aboard(fleet, force.loaded_armies);
    fleet.ships.etacs = force.etacs;
    recompute_max_speed(fleet);
}

fn sample_bombard_force(rng: &mut GameRng) -> ForceSpec {
    ForceSpec {
        battleships: range_u16(rng, 0, 4),
        cruisers: range_u16(rng, 1, 5),
        destroyers: range_u16(rng, 1, 6),
        transports: range_u16(rng, 0, 3),
        loaded_armies: range_u16(rng, 0, 6),
        ..ForceSpec::default()
    }
}

fn sample_assault_force(rng: &mut GameRng) -> ForceSpec {
    ForceSpec {
        battleships: range_u16(rng, 0, 3),
        cruisers: range_u16(rng, 1, 4),
        destroyers: range_u16(rng, 1, 5),
        transports: range_u16(rng, 2, 6),
        loaded_armies: range_u16(rng, 4, 10),
        ..ForceSpec::default()
    }
}

fn sample_battle_force(rng: &mut GameRng) -> ForceSpec {
    ForceSpec {
        battleships: range_u16(rng, 0, 2),
        cruisers: range_u16(rng, 1, 4),
        destroyers: range_u16(rng, 1, 6),
        transports: range_u16(rng, 0, 2),
        loaded_armies: range_u16(rng, 0, 4),
        etacs: range_u16(rng, 0, 2),
        ..ForceSpec::default()
    }
}

fn sample_destroyed_fleet_force(rng: &mut GameRng) -> ForceSpec {
    ForceSpec {
        cruisers: range_u16(rng, 1, 2),
        destroyers: range_u16(rng, 1, 4),
        transports: range_u16(rng, 0, 1),
        loaded_armies: range_u16(rng, 0, 3),
        ..ForceSpec::default()
    }
}

fn sample_contact_force(rng: &mut GameRng) -> ForceSpec {
    ForceSpec {
        scouts: u16::from(rng.range_u8(0, 1)),
        cruisers: range_u16(rng, 0, 2),
        destroyers: range_u16(rng, 0, 2),
        transports: range_u16(rng, 0, 2),
        loaded_armies: range_u16(rng, 0, 3),
        ..ForceSpec::default()
    }
}

fn describe_force(force: ForceSpec) -> String {
    let mut parts = Vec::new();
    if force.battleships > 0 {
        parts.push(unit_text(
            force.battleships.into(),
            "battleship",
            "battleships",
        ));
    }
    if force.cruisers > 0 {
        parts.push(unit_text(force.cruisers.into(), "cruiser", "cruisers"));
    }
    if force.destroyers > 0 {
        parts.push(unit_text(
            force.destroyers.into(),
            "destroyer",
            "destroyers",
        ));
    }
    if force.scouts > 0 {
        parts.push(unit_text(force.scouts.into(), "scout ship", "scout ships"));
    }
    if force.transports > 0 {
        parts.push(unit_text(
            force.transports.into(),
            "troop transport ship",
            "troop transport ships",
        ));
    }
    if force.etacs > 0 {
        parts.push(unit_text(force.etacs.into(), "ETAC ship", "ETAC ships"));
    }
    if force.loaded_armies > 0 {
        parts.push(unit_text(force.loaded_armies.into(), "army", "armies"));
    }
    if parts.is_empty() {
        "no ships".to_string()
    } else {
        join_parts(&parts)
    }
}

fn describe_ground_forces(batteries: u8, armies: u8) -> String {
    let mut parts = Vec::new();
    if batteries > 0 {
        parts.push(unit_text(
            batteries.into(),
            "ground battery",
            "ground batteries",
        ));
    }
    if armies > 0 {
        parts.push(unit_text(armies.into(), "army", "armies"));
    }
    if parts.is_empty() {
        "none".to_string()
    } else {
        join_parts(&parts)
    }
}

fn retreat_coords(coords: [u8; 2]) -> [u8; 2] {
    [
        coords[0].saturating_sub(1).max(1),
        coords[1].saturating_sub(1).max(1),
    ]
}

fn mission_label(mission: Mission) -> &'static str {
    match mission {
        Mission::BombardWorld => "bombard",
        Mission::InvadeWorld => "invade",
        Mission::BlitzWorld => "blitz",
        Mission::GuardBlockadeWorld => "guard-blockade",
        Mission::ScoutSector => "scout-sector",
        _ => "mission",
    }
}

fn mission_to_order(mission: Mission) -> Order {
    match mission {
        Mission::BombardWorld => Order::BombardWorld,
        Mission::InvadeWorld => Order::InvadeWorld,
        Mission::BlitzWorld => Order::BlitzWorld,
        Mission::ScoutSector => Order::ScoutSector,
        _ => Order::HoldPosition,
    }
}

fn sample_week(sample_index: usize) -> u8 {
    ((sample_index % 52) + 1) as u8
}

fn range_u16(rng: &mut GameRng, min: u16, max: u16) -> u16 {
    if min >= max {
        return min;
    }
    let span = u32::from(max - min) + 1;
    min + (rng.next_u32() % span) as u16
}

fn unit_text(count: u32, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("1 {singular}")
    } else {
        format!("{count} {plural}")
    }
}

fn join_parts(parts: &[String]) -> String {
    match parts {
        [] => String::new(),
        [one] => one.clone(),
        [left, right] => format!("{left} and {right}"),
        _ => {
            let mut out = parts[..parts.len() - 1].join(", ");
            out.push_str(" and ");
            out.push_str(parts.last().unwrap());
            out
        }
    }
}
