//! Maintenance logic for ECMAINT.EXE mechanics.

mod campaign;
mod canonicalize;
mod combat;
mod economics;
pub mod encounter;
mod fleet_refs;
pub mod gate;
mod merging;
mod movement;
pub mod recovery;
mod results;
mod retarget;
mod sanitize;
pub mod timing;

pub use nc_data::maintenance_types::*;
pub use results::{
    apply_results_reviewable_flags, build_rankings_text, build_results_dat,
    build_results_report_blocks,
};

use crate::VisibleHazardIntel;
use fleet_refs::{FleetIndexRemap, TurnFleetRefs};
use nc_data::{
    CoreGameData, FleetRecord, Order, PlayerLifecycleState, WinnerState,
    default_player_lifecycle_states,
};
use std::fmt;

/// Event produced when a fleet completes a ColonizeWorld order.
#[derive(Debug)]
struct ColonizationEvent {
    /// Fleet index in the in-memory fleet list that arrived.
    fleet_idx: usize,
    /// Target coordinates where colonization occurred.
    coords: [u8; 2],
    /// Empire that colonized (owner_empire_raw from fleet record).
    owner_empire: u8,
    /// The tick the fleet arrived.
    stardate_week: Option<u8>,
}

#[derive(Debug, Default)]
struct MovementEvents {
    colonization_events: Vec<ColonizationEvent>,
    /// Bombard/Invade/Blitz fleets whose strike test passed this year.
    hostile_strike_fleet_indices: Vec<usize>,
    planet_intel_events: Vec<PlanetIntelEvent>,
    pending_observation_events: Vec<PendingObservationEvent>,
    mission_events: Vec<MissionEvent>,
    salvage_events: Vec<SalvageResolvedEvent>,
    diplomatic_escalation_events: Vec<DiplomaticEscalationEvent>,
    /// Starbase arrival and lost-world reports.
    starbase_reports: Vec<nc_data::StarbaseReportEvent>,
    /// Guards merged into another guard of their base; the flag marks an
    /// arriving guard's merge, which it reports.
    guard_merges: Vec<(FleetMergeEvent, bool)>,
    /// Guards whose base was destroyed during the year.
    guard_retargets: Vec<MissionRetargetEvent>,
    /// Speed each fleet moved at this turn (0 when it did not move), indexed by fleet.
    fleet_travel_speeds: Vec<u8>,
}

#[derive(Debug, Default, Clone)]
pub(super) struct FleetRemovalRemapInfo {
    destroyed_join_host_fleet_numbers: std::collections::HashMap<u16, u8>,
    prior_join_host_ids: std::collections::HashMap<u16, u16>,
    fleet_indices: FleetIndexRemap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingObservationEvent {
    fleet_idx: usize,
    owner_empire_raw: u8,
    kind: Mission,
    outcome: MissionOutcome,
    planet_idx: Option<usize>,
    location_coords: [u8; 2],
    target_coords: [u8; 2],
    intel_event: Option<PlanetIntelEvent>,
    stardate_week: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenancePreflightError {
    issues: Vec<String>,
}

impl MaintenancePreflightError {
    pub fn new(issues: Vec<String>) -> Self {
        Self { issues }
    }

    pub fn issues(&self) -> &[String] {
        &self.issues
    }
}

impl fmt::Display for MaintenancePreflightError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.issues.is_empty() {
            write!(f, "maintenance preflight failed")
        } else {
            write!(
                f,
                "maintenance preflight failed ({} issue(s)): {}",
                self.issues.len(),
                self.issues.join("; ")
            )
        }
    }
}

impl std::error::Error for MaintenancePreflightError {}

pub fn validate_maintenance_state(
    game_data: &CoreGameData,
) -> Result<(), MaintenancePreflightError> {
    let issues = game_data.ecmaint_structural_preflight_errors();
    if issues.is_empty() {
        Ok(())
    } else {
        Err(MaintenancePreflightError::new(issues))
    }
}

/// Run a single turn of maintenance processing.
///
/// This is the Rust yearly maintenance simulation. It validates structural
/// campaign state, advances the yearly simulation core, and returns a
/// canonicalized `MaintenanceEvents` pool for later report/projection
/// builders.
///
/// Compatibility-edge projections such as `DATABASE.DAT` regeneration remain
/// outside the core engine because they are not part of `CoreGameData`.
///
/// # Arguments
/// * `game_data` - Mutable reference to the game state to modify
///
/// # Returns
/// Canonicalized maintenance events on success, or an error if structural
/// validation or yearly maintenance fails.
pub fn run_maintenance_turn(
    game_data: &mut CoreGameData,
) -> Result<MaintenanceEvents, Box<dyn std::error::Error>> {
    let mut player_lifecycle_states =
        default_player_lifecycle_states(game_data.conquest.player_count());
    let mut winner_state = WinnerState::default();
    run_maintenance_turn_with_context_seed_and_lifecycle(
        game_data,
        0,
        &[],
        &[],
        &mut player_lifecycle_states,
        &mut winner_state,
    )
}

pub fn run_maintenance_turn_with_visible_hazards(
    game_data: &mut CoreGameData,
    visible_hazards_by_empire: &[VisibleHazardIntel],
) -> Result<MaintenanceEvents, Box<dyn std::error::Error>> {
    let mut player_lifecycle_states =
        default_player_lifecycle_states(game_data.conquest.player_count());
    let mut winner_state = WinnerState::default();
    run_maintenance_turn_with_context_seed_and_lifecycle(
        game_data,
        0,
        visible_hazards_by_empire,
        &[],
        &mut player_lifecycle_states,
        &mut winner_state,
    )
}

pub fn run_maintenance_turn_with_seed(
    game_data: &mut CoreGameData,
    campaign_seed: u64,
) -> Result<MaintenanceEvents, Box<dyn std::error::Error>> {
    let mut player_lifecycle_states =
        default_player_lifecycle_states(game_data.conquest.player_count());
    let mut winner_state = WinnerState::default();
    run_maintenance_turn_with_context_seed_and_lifecycle(
        game_data,
        campaign_seed,
        &[],
        &[],
        &mut player_lifecycle_states,
        &mut winner_state,
    )
}

pub fn run_maintenance_turn_with_visible_hazards_and_seed(
    game_data: &mut CoreGameData,
    campaign_seed: u64,
    visible_hazards_by_empire: &[VisibleHazardIntel],
) -> Result<MaintenanceEvents, Box<dyn std::error::Error>> {
    let mut player_lifecycle_states =
        default_player_lifecycle_states(game_data.conquest.player_count());
    let mut winner_state = WinnerState::default();
    run_maintenance_turn_with_context_seed_and_lifecycle(
        game_data,
        campaign_seed,
        visible_hazards_by_empire,
        &[],
        &mut player_lifecycle_states,
        &mut winner_state,
    )
}

pub fn run_maintenance_turn_with_context(
    game_data: &mut CoreGameData,
    visible_hazards_by_empire: &[VisibleHazardIntel],
    diplomacy_overrides: &[DiplomacyOverride],
) -> Result<MaintenanceEvents, Box<dyn std::error::Error>> {
    let mut player_lifecycle_states =
        default_player_lifecycle_states(game_data.conquest.player_count());
    let mut winner_state = WinnerState::default();
    run_maintenance_turn_with_context_seed_and_lifecycle(
        game_data,
        0,
        visible_hazards_by_empire,
        diplomacy_overrides,
        &mut player_lifecycle_states,
        &mut winner_state,
    )
}

pub fn run_maintenance_turn_with_context_and_seed(
    game_data: &mut CoreGameData,
    campaign_seed: u64,
    visible_hazards_by_empire: &[VisibleHazardIntel],
    diplomacy_overrides: &[DiplomacyOverride],
) -> Result<MaintenanceEvents, Box<dyn std::error::Error>> {
    let mut player_lifecycle_states =
        default_player_lifecycle_states(game_data.conquest.player_count());
    let mut winner_state = WinnerState::default();
    run_maintenance_turn_with_context_seed_and_lifecycle(
        game_data,
        campaign_seed,
        visible_hazards_by_empire,
        diplomacy_overrides,
        &mut player_lifecycle_states,
        &mut winner_state,
    )
}

pub fn run_maintenance_turn_with_context_seed_and_lifecycle(
    game_data: &mut CoreGameData,
    campaign_seed: u64,
    visible_hazards_by_empire: &[VisibleHazardIntel],
    diplomacy_overrides: &[DiplomacyOverride],
    player_lifecycle_states: &mut [PlayerLifecycleState],
    winner_state: &mut WinnerState,
) -> Result<MaintenanceEvents, Box<dyn std::error::Error>> {
    validate_maintenance_state(game_data)
        .map_err(|err| -> Box<dyn std::error::Error> { Box::new(err) })?;

    // Remove any fleet records that carry no ships at all.  These are stale
    // shells left over from combat that zeroed a fleet's counts without
    // removing the record (e.g. pursuit-fire eliminations, prior-turn
    // attrition).  Culling here — before snapshots or event dispatch — means
    // no later code ever sees a ghost fleet, and no event index remapping is
    // required after the fact.
    let destroyed_join_host_fleet_numbers = cull_empty_fleets(game_data);

    // CONQUEST.DAT 0x0c/0x3d accumulation trigger — snapshot BEFORE processing.
    // ECMAINT increments production total (0x0c += 100) and turn counter (0x3d += 1)
    // only when raw[0x0c] == 0x64 at the start of the turn.
    //
    // State machine for 0x0c:
    //   0x00 (fresh/econ start) → first tick writes non-active prod words → becomes 0x64
    //   0x64 (initialized)      → next tick with should_accumulate=true increments → 0xc8
    //   0xc8+ (accumulated)     → no further changes
    //
    // Confirmed across all scenarios:
    // - fleet/build/move:  0x0c pre=0x64, post=0x64 (no change; stays at 0x64 each tick but
    //   the condition raw[0x0c]==0x64 IS true ... so why don't they accumulate?)
    //
    // Actually the rule is: accumulate when raw[0x0c]==0x64 AND (any fleet in-transit OR
    // any active/rogue player). Without one of those two game-activity signals, the
    // fleet/build/move scenarios don't accumulate even though 0x0c==0x64.
    //
    // Confirmed:
    // - bombard tick 2: 0x0c=0x64, fleet 2 in-transit → accumulates to 0xc8 ✓
    // - invade/econ tick 2: 0x0c=0x64, active player present → accumulates to 0xc8 ✓
    // - fleet/build/move tick N: 0x0c=0x64, no in-transit, no active → no accumulation ✓
    // Snapshot pre-turn state needed for post-movement processing.
    // These must be captured BEFORE any mutations so they reflect start-of-turn conditions.

    // ECMAINT copies every empire's production and planet count into the
    // CONQUEST.DAT rankings before the pass changes them.
    game_data
        .conquest
        .snapshot_previous_year(&game_data.player.records);

    let initial_campaign_outlook = campaign::campaign_outlook(game_data, player_lifecycle_states);
    let initial_campaign_outcome =
        campaign::campaign_outcome(game_data, player_lifecycle_states, *winner_state);
    let empire_turn_start_states =
        campaign::capture_empire_turn_start_states(game_data, player_lifecycle_states);

    // Advance game year by 1
    let current_year = game_data.conquest.game_year();
    let new_year = current_year + 1;
    game_data.conquest.set_game_year(new_year);

    // Every fleet-record compaction below must remap these captured references.
    let mut refs = TurnFleetRefs {
        destroyed_join_host_fleet_numbers,
        ..TurnFleetRefs::default()
    };

    // Merge co-located friendly fleets BEFORE movement.
    // Confirmed from econ fixture: the Bombard fleet (at 16,13 pre-move) is
    // included in the merge even though it moves to (15,13) this turn.
    // The merge runs before movement resolution, absorbing all same-position
    // fleets for flagged players (PLAYER raw[0x00]==0xff).
    let (merge_events, consolidation_remap) = merging::process_fleet_merging(game_data)?;
    refs.merge_events.extend(merge_events.iter().copied());
    refs.remap(&consolidation_remap.fleet_indices);
    refs.join_host_events = merging::process_join_host_updates(
        game_data,
        &merge_events,
        &refs.destroyed_join_host_fleet_numbers,
        &consolidation_remap,
    );

    refs.invalid_player_state_events = sanitize_invalid_player_inputs(game_data);

    refs.mission_retarget_events = retarget::refresh_seek_home_targets(game_data);
    refs.mission_retarget_events
        .extend(retarget::refresh_join_host_targets(game_data));
    refs.mission_retarget_events
        .extend(retarget::refresh_guard_starbase_targets(game_data));

    // Autopilot fleet recall: idle fleets in deep space get SeekHome before
    // movement so they can start heading home this turn.
    economics::process_autopilot_fleet_orders(game_data)?;

    // ECMAINT settles the starbase growth bonus before the year's ticks.
    let starbase_planets = economics::starbase_planets(game_data);

    // The year's 52 ticks: every fleet and starbase, in a random order,
    // moves, meets the units sharing its sector and depth layer, and runs
    // its order's arrival test.
    let year = encounter::run_year(
        game_data,
        campaign_seed,
        visible_hazards_by_empire,
        diplomacy_overrides,
    )?;
    refs.movement_events = year.movement_events;
    refs.encounter_events = year.encounter_events;
    let guard_merges: Vec<(FleetMergeEvent, bool)> =
        std::mem::take(&mut refs.movement_events.guard_merges);
    refs.merge_events.extend(
        guard_merges
            .iter()
            .filter(|(_, reported)| *reported)
            .map(|(event, _)| *event),
    );
    refs.mission_retarget_events
        .append(&mut refs.movement_events.guard_retargets);
    refs.remap(&year.movement_remap.fleet_indices);
    refs.destroyed_join_host_fleet_numbers.extend(
        year.movement_remap
            .destroyed_join_host_fleet_numbers
            .clone(),
    );
    if !guard_merges.is_empty() {
        // Join missions follow a host that merged into another guard.
        let merges: Vec<FleetMergeEvent> = guard_merges.iter().map(|(event, _)| *event).collect();
        let join_host_events = merging::process_join_host_updates(
            game_data,
            &merges,
            &refs.destroyed_join_host_fleet_numbers,
            &year.movement_remap,
        );
        refs.join_host_events.extend(join_host_events);
    }
    loop {
        let (new_merge_events, remap_info) = merging::process_mission_fleet_merging(game_data)?;
        if new_merge_events.is_empty() {
            break;
        }
        refs.merge_events.extend(new_merge_events.iter().copied());
        refs.remap(&remap_info.fleet_indices);
        let new_join_host_events = merging::process_join_host_updates(
            game_data,
            &new_merge_events,
            &refs.destroyed_join_host_fleet_numbers,
            &remap_info,
        );
        let had_retargets = new_join_host_events
            .iter()
            .any(|event| matches!(event, JoinMissionHostEvent::Retargeted { .. }));
        refs.join_host_events.extend(new_join_host_events);
        if !had_retargets {
            break;
        }
    }

    let mut fleet_battle_phase_events = std::mem::take(&mut refs.encounter_events);

    // Apply colonization results to the in-memory game state.
    let mut colonization_events =
        merging::process_colonizations(game_data, &refs.movement_events.colonization_events)?;
    for event in &mut colonization_events {
        let (ColonizationResolvedEvent::Succeeded {
            fleet_idx,
            stardate_week,
            ..
        }
        | ColonizationResolvedEvent::BlockedByOwner {
            fleet_idx,
            stardate_week,
            ..
        }
        | ColonizationResolvedEvent::Aborted {
            fleet_idx,
            stardate_week,
            ..
        }) = event;
        if stardate_week.is_none() {
            *stardate_week = refs
                .movement_events
                .colonization_events
                .iter()
                .find(|arrival| arrival.fleet_idx == *fleet_idx)
                .and_then(|arrival| arrival.stardate_week);
        }
    }
    let newly_colonized_planets = colonization_events
        .iter()
        .filter_map(|event| match *event {
            ColonizationResolvedEvent::Succeeded { planet_idx, .. } => Some(planet_idx),
            _ => None,
        })
        .collect::<Vec<_>>();

    // Process build queues and track which planets had activity
    let planets_with_builds = economics::process_build_completion(game_data)?;

    // Process planet economic updates for planets that had builds
    economics::process_planet_economics(
        game_data,
        &planets_with_builds,
        &newly_colonized_planets,
        &starbase_planets,
    )?;

    // Run autopilot / rogue AI planet economics.
    // Updates factories, armies, and raw[0x0E] for rogue and autopilot-on players.
    economics::process_autopilot_ai(game_data)?;

    // ECMAINT leaves PLAYER 0x46 naming the last base in each chain.
    game_data.sync_last_base_records();

    // Hostile fleets strike in the tick they reach orbit, or in the first
    // tick of the year when they start it in orbit on a hostile order.
    let mut assault_events = combat::process_planetary_assaults(
        game_data,
        campaign_seed,
        &refs.movement_events.hostile_strike_fleet_indices,
    )?;

    // Recompute per-player planet count and production score from the in-memory
    // planet records after hostile action, since assault resolution can change ownership.
    economics::recompute_player_planet_stats(game_data);
    let campaign_transition_events = campaign::apply_player_lifecycle_transitions(
        game_data,
        &empire_turn_start_states,
        &assault_events.ownership_change_events,
        &{
            let mut combined = fleet_battle_phase_events.fleet_destroyed_events.clone();
            combined.extend(assault_events.fleet_destroyed_events.iter().cloned());
            combined
        },
        player_lifecycle_states,
        winner_state,
        initial_campaign_outlook,
        initial_campaign_outcome,
    );
    let fleet_keys_before_defection = fleet_record_keys(game_data);
    let fleet_defection_events = campaign::apply_civil_disorder_fleet_defections(
        game_data,
        &campaign_transition_events.empire_elimination_events,
    )?;
    let defection_remap = FleetIndexRemap::from_survivors(
        &fleet_keys_before_defection,
        &fleet_record_keys(game_data),
    );
    refs.remap(&defection_remap);
    defection_remap.remap_colonization_events(&mut colonization_events);
    defection_remap.remap_fleet_battle_phase_events(&mut fleet_battle_phase_events);
    defection_remap.remap_mission_events(&mut assault_events.mission_events);
    defection_remap.remap_planet_intel_events(&mut assault_events.planet_intel_events);

    finalize_pending_observation_events(
        game_data,
        &mut refs.movement_events,
        &fleet_battle_phase_events.mission_events,
    );

    restore_scout_orders_and_generate_on_station_observations(game_data, &mut refs.movement_events);

    let movement_events = refs.movement_events;
    let mut mission_events = movement_events.mission_events;
    mission_events.extend(fleet_battle_phase_events.mission_events);
    mission_events.extend(assault_events.mission_events);
    for colonization in &colonization_events {
        match *colonization {
            ColonizationResolvedEvent::Succeeded {
                fleet_idx,
                planet_idx,
                colonizer_empire_raw,
                stardate_week,
                ..
            } => mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire_raw: colonizer_empire_raw,
                kind: Mission::ColonizeWorld,
                outcome: MissionOutcome::Succeeded,
                abort_reason: None,
                planet_idx: Some(planet_idx),
                location_coords: Some(game_data.planets.records[planet_idx].coords_raw()),
                target_coords: Some(game_data.planets.records[planet_idx].coords_raw()),
                stardate_week,
            }),
            ColonizationResolvedEvent::BlockedByOwner {
                fleet_idx,
                planet_idx,
                colonizer_empire_raw,
                stardate_week,
                ..
            } => mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire_raw: colonizer_empire_raw,
                kind: Mission::ColonizeWorld,
                outcome: MissionOutcome::Failed,
                abort_reason: None,
                planet_idx: Some(planet_idx),
                location_coords: Some(game_data.planets.records[planet_idx].coords_raw()),
                target_coords: Some(game_data.planets.records[planet_idx].coords_raw()),
                stardate_week,
            }),
            ColonizationResolvedEvent::Aborted {
                fleet_idx,
                colonizer_empire_raw,
                coords,
                stardate_week,
                ..
            } => mission_events.push(MissionEvent {
                fleet_idx,
                owner_empire_raw: colonizer_empire_raw,
                kind: Mission::ColonizeWorld,
                outcome: MissionOutcome::Aborted,
                abort_reason: None,
                planet_idx: game_data
                    .planets
                    .records
                    .iter()
                    .position(|planet| planet.coords_raw() == coords),
                location_coords: Some(coords),
                target_coords: Some(coords),
                stardate_week,
            }),
        }
    }

    let blocked_colonization_intel_events = colonization_events
        .iter()
        .filter_map(|event| match *event {
            ColonizationResolvedEvent::BlockedByOwner {
                fleet_idx,
                planet_idx,
                colonizer_empire_raw,
                stardate_week,
                ..
            } => Some(PlanetIntelEvent {
                planet_idx,
                viewer_empire_raw: colonizer_empire_raw,
                source: nc_data::PlanetIntelSource::ColonizeBlockedByOwner,
                source_fleet_idx: Some(fleet_idx),
                observed_snapshot: None,
                stardate_week,
            }),
            _ => None,
        })
        .collect::<Vec<_>>();

    let events = MaintenanceEvents {
        bombard_events: assault_events.bombard_events,
        planet_intel_events: {
            let mut events = movement_events.planet_intel_events;
            events.extend(fleet_battle_phase_events.planet_intel_events);
            events.extend(assault_events.planet_intel_events);
            events.extend(blocked_colonization_intel_events);
            events
        },
        ownership_change_events: assault_events.ownership_change_events,
        fleet_battle_events: fleet_battle_phase_events.fleet_battle_events,
        fleet_destroyed_events: {
            let mut events = fleet_battle_phase_events.fleet_destroyed_events;
            events.extend(assault_events.fleet_destroyed_events);
            events
        },
        starbase_destroyed_events: fleet_battle_phase_events.starbase_destroyed_events,
        starbase_report_events: movement_events.starbase_reports,
        assault_report_events: assault_events.assault_report_events,
        scout_contact_events: fleet_battle_phase_events.scout_contact_events,
        encounter_disposition_events: fleet_battle_phase_events.encounter_disposition_events,
        invalid_player_state_events: refs.invalid_player_state_events,
        fleet_merge_events: refs.merge_events,
        join_host_events: refs.join_host_events,
        mission_retarget_events: refs.mission_retarget_events,
        colonization_events,
        mission_events,
        salvage_events: movement_events.salvage_events,
        diplomatic_escalation_events: movement_events.diplomatic_escalation_events,
        civil_disorder_events: campaign_transition_events.civil_disorder_events,
        campaign_outlook_events: campaign_transition_events.campaign_outlook_events,
        campaign_outcome_events: campaign_transition_events.campaign_outcome_events,
        game_victory_notice_events: campaign_transition_events.game_victory_notice_events,
        empire_elimination_events: campaign_transition_events.empire_elimination_events,
        fleet_defection_events,
    };

    campaign::apply_stored_diplomatic_escalations(game_data, &events)?;

    // Assign stardate week values and sort event vectors chronologically.
    let mut events = events;
    canonicalize::canonicalize_events(&mut events, &movement_events.fleet_travel_speeds);

    Ok(events)
}

/// Per-fleet identity used to detect which records a compaction removed.
fn fleet_record_keys(game_data: &CoreGameData) -> Vec<(u8, u16)> {
    game_data
        .fleets
        .records
        .iter()
        .map(|fleet| (fleet.owner_empire_raw(), fleet.local_slot_word_raw()))
        .collect()
}

fn fleet_has_presence(fleet: &FleetRecord) -> bool {
    fleet.has_any_force()
}

/// Restore scout orders for fleets that arrived this turn (they were set to
/// HoldPosition by the stepper to avoid interfering with combat resolution).
/// Also generate per-turn observation reports for scout fleets already on station
/// from a previous turn.
///
/// This "Revert-then-Restore" mechanism is the engine's implementation of
/// "Persistent Stealth" defined in the player manual: fleets appear stationary
/// to avoid forced combat triggers, but maintain their standing scouting
/// mission across maintenance turns.
fn restore_scout_orders_and_generate_on_station_observations(
    game_data: &mut CoreGameData,
    movement_events: &mut MovementEvents,
) {
    // Collect scout/observer arrivals: fleet_idx → mission kind.
    // Used to skip fleets that already generated a report via the arrival path
    // this turn so they don't also fire the on-station repeat path.
    let scout_arrivals: std::collections::HashMap<usize, Mission> = movement_events
        .mission_events
        .iter()
        .filter(|e| {
            matches!(
                e.kind,
                Mission::ScoutSector | Mission::ScoutSolarSystem | Mission::ViewWorld
            ) && matches!(
                e.outcome,
                MissionOutcome::Succeeded | MissionOutcome::Arrived
            )
        })
        .map(|e| (e.fleet_idx, e.kind))
        .collect();

    // Restore scout orders for fleets that just arrived and survived combat.
    // This completes the persistence cycle for newly-arrived scouts.
    for (&fleet_idx, &kind) in &scout_arrivals {
        let Some(fleet) = game_data.fleets.records.get_mut(fleet_idx) else {
            continue;
        };
        if !fleet.has_any_force() {
            continue;
        }
        let order = match kind {
            Mission::ScoutSector => Order::ScoutSector,
            Mission::ScoutSolarSystem => Order::ScoutSolarSystem,
            _ => continue,
        };
        fleet.set_standing_order_kind(order);
    }

    // Generate per-turn observations for scouts already on station from previous turns.
    for (fleet_idx, fleet) in game_data.fleets.records.iter().enumerate() {
        if !fleet.has_any_force() {
            continue;
        }
        if scout_arrivals.contains_key(&fleet_idx) {
            continue;
        }
        let order = fleet.standing_order_kind();
        let coords = fleet.current_location_coords_raw();
        let target = fleet.standing_order_target_coords_raw();
        // On station means settled at the order's target depth; a fleet
        // still diving into the system has not arrived yet.
        let settled =
            fleet.depth() == crate::navigation::motion::target_depth(fleet.world_target_flag_raw());
        if coords != target || !settled {
            continue;
        }
        let owner_empire_raw = fleet.owner_empire_raw();
        match order {
            Order::ScoutSector => {
                movement_events.mission_events.push(MissionEvent {
                    fleet_idx,
                    owner_empire_raw,
                    kind: Mission::ScoutSector,
                    outcome: MissionOutcome::Succeeded,
                    abort_reason: None,
                    planet_idx: None,
                    location_coords: Some(coords),
                    target_coords: Some(coords),
                    stardate_week: None,
                });
            }
            Order::ScoutSolarSystem => {
                let planet_idx = game_data
                    .planets
                    .records
                    .iter()
                    .position(|planet| planet.coords_raw() == coords);
                if let Some(planet_idx) = planet_idx {
                    movement_events.planet_intel_events.push(PlanetIntelEvent {
                        planet_idx,
                        viewer_empire_raw: owner_empire_raw,
                        source: nc_data::PlanetIntelSource::ScoutSolarSystem,
                        source_fleet_idx: Some(fleet_idx),
                        observed_snapshot: nc_data::build_runtime_planet_intel_snapshot(
                            game_data,
                            owner_empire_raw,
                            game_data.conquest.game_year(),
                            planet_idx,
                            nc_data::PlanetIntelSource::ScoutSolarSystem,
                        ),
                        stardate_week: None,
                    });
                }
                movement_events.mission_events.push(MissionEvent {
                    fleet_idx,
                    owner_empire_raw,
                    kind: Mission::ScoutSolarSystem,
                    outcome: MissionOutcome::Succeeded,
                    abort_reason: None,
                    planet_idx,
                    location_coords: Some(coords),
                    target_coords: Some(coords),
                    stardate_week: None,
                });
            }
            _ => {}
        }
    }
}

fn finalize_pending_observation_events(
    game_data: &mut CoreGameData,
    movement_events: &mut MovementEvents,
    combat_mission_events: &[MissionEvent],
) {
    let pending_events = std::mem::take(&mut movement_events.pending_observation_events);
    for pending in pending_events {
        let Some(fleet) = game_data.fleets.records.get_mut(pending.fleet_idx) else {
            continue;
        };
        if !fleet_has_presence(fleet) {
            continue;
        }
        let mission_aborted = combat_mission_events.iter().any(|event| {
            event.fleet_idx == pending.fleet_idx
                && event.owner_empire_raw == pending.owner_empire_raw
                && event.kind == pending.kind
                && event.outcome == MissionOutcome::Aborted
        });
        if mission_aborted {
            continue;
        }
        let moved_out = pending.kind == Mission::ViewWorld && movement::complete_view_world(fleet);
        if let Some(intel_event) = pending.intel_event {
            movement_events.planet_intel_events.push(intel_event);
        }
        movement_events.mission_events.push(MissionEvent {
            fleet_idx: pending.fleet_idx,
            owner_empire_raw: pending.owner_empire_raw,
            kind: pending.kind,
            outcome: pending.outcome,
            abort_reason: None,
            planet_idx: pending.planet_idx,
            location_coords: Some(pending.location_coords),
            target_coords: Some(pending.target_coords),
            stardate_week: pending.stardate_week,
        });
        if moved_out {
            movement_events.mission_events.push(MissionEvent {
                fleet_idx: pending.fleet_idx,
                owner_empire_raw: pending.owner_empire_raw,
                kind: Mission::MoveOnly,
                outcome: MissionOutcome::Succeeded,
                abort_reason: None,
                planet_idx: None,
                location_coords: Some(pending.location_coords),
                target_coords: Some(pending.target_coords),
                stardate_week: pending.stardate_week,
            });
        }
    }
}

fn apply_fleet_removal_remap(
    game_data: &mut CoreGameData,
    to_remove: &[bool],
) -> FleetRemovalRemapInfo {
    let fleet_count = game_data.fleets.records.len();
    if fleet_count == 0 || to_remove.len() != fleet_count {
        return FleetRemovalRemapInfo::default();
    }

    let pre_removal_owner: Vec<u8> = game_data
        .fleets
        .records
        .iter()
        .map(|f| f.owner_empire_raw())
        .collect();
    let pre_removal_fleet_id: Vec<u16> = game_data
        .fleets
        .records
        .iter()
        .map(|f| f.fleet_id_word_raw())
        .collect();
    let removed_fleet_number_by_id: std::collections::HashMap<u16, u8> = game_data
        .fleets
        .records
        .iter()
        .enumerate()
        .filter(|(idx, _)| to_remove[*idx])
        .filter_map(|(_, fleet)| {
            let fleet_number = fleet.local_slot_word_raw() as u8;
            (fleet_number != 0).then_some((fleet.fleet_id_word_raw(), fleet_number))
        })
        .collect();

    let removed_before: Vec<u16> = {
        let mut count = 0u16;
        (0..fleet_count)
            .map(|i| {
                let current = count;
                if to_remove[i] {
                    count = count.saturating_add(1);
                }
                current
            })
            .collect()
    };

    let remap_id = |old_id: u16| -> u16 {
        if old_id == 0 {
            return 0;
        }
        let orig_idx = (old_id as usize).saturating_sub(1);
        if orig_idx >= fleet_count || to_remove[orig_idx] {
            0
        } else {
            old_id.saturating_sub(removed_before[orig_idx])
        }
    };

    // Surviving local fleet numbers stay unchanged. Only global linkage IDs compress.
    let mut remap_info = FleetRemovalRemapInfo {
        fleet_indices: FleetIndexRemap::from_removed(to_remove),
        ..FleetRemovalRemapInfo::default()
    };
    let mut remapped_fleets = Vec::new();
    for (idx, fleet) in game_data.fleets.records.iter().enumerate() {
        if to_remove[idx] {
            continue;
        }
        let mut remapped = fleet.clone();
        let new_fleet_id = remap_id(fleet.fleet_id_word_raw());
        remapped.set_fleet_id_word_raw(new_fleet_id);
        remapped.set_next_fleet_link_word_raw(remap_id(fleet.next_fleet_link_word_raw()));
        remapped.set_previous_fleet_id(remap_id(fleet.previous_fleet_id()));
        if fleet.standing_order_kind() == Order::JoinAnotherFleet {
            let prior_host_id = fleet.join_host_fleet_id_raw();
            let remapped_host_id = remap_id(prior_host_id);
            remapped.set_join_host_fleet_id_raw(remapped_host_id);
            if remapped_host_id == 0 && prior_host_id != 0 {
                remap_info
                    .prior_join_host_ids
                    .insert(new_fleet_id, prior_host_id);
                if let Some(host_fleet_number) =
                    removed_fleet_number_by_id.get(&prior_host_id).copied()
                {
                    remap_info
                        .destroyed_join_host_fleet_numbers
                        .insert(new_fleet_id, host_fleet_number);
                }
            }
        }
        remapped_fleets.push(remapped);
    }
    game_data.fleets.records = remapped_fleets;

    for player_idx in 0..game_data.player.records.len() {
        let owner_raw = (player_idx + 1) as u8;
        let first_id = game_data.player.records[player_idx].fleet_chain_head_raw();
        let last_id = game_data.player.records[player_idx].fleet_chain_tail_raw();
        let new_first = remap_id(first_id);
        let new_last = remap_id(last_id);
        game_data.player.records[player_idx].set_fleet_chain_head_raw(new_first);
        game_data.player.records[player_idx].set_fleet_chain_tail_raw(
            if new_last == 0 && new_first != 0 {
                let mut max_new_id: u16 = new_first;
                for orig_idx in 0..fleet_count {
                    if pre_removal_owner[orig_idx] == owner_raw && !to_remove[orig_idx] {
                        let mapped = remap_id(pre_removal_fleet_id[orig_idx]);
                        if mapped > max_new_id {
                            max_new_id = mapped;
                        }
                    }
                }
                max_new_id
            } else {
                new_last
            },
        );
    }

    remap_info
}

fn remove_selected_fleets(
    game_data: &mut CoreGameData,
    to_remove: &[bool],
) -> std::collections::HashMap<u16, u8> {
    apply_fleet_removal_remap(game_data, to_remove).destroyed_join_host_fleet_numbers
}

fn cull_empty_fleets(game_data: &mut CoreGameData) -> std::collections::HashMap<u16, u8> {
    let to_remove: Vec<bool> = game_data
        .fleets
        .records
        .iter()
        .map(|f| !f.has_any_force())
        .collect();
    if to_remove.iter().any(|&r| r) {
        apply_fleet_removal_remap(game_data, &to_remove).destroyed_join_host_fleet_numbers
    } else {
        std::collections::HashMap::new()
    }
}

fn sanitize_invalid_player_inputs(game_data: &mut CoreGameData) -> Vec<InvalidPlayerStateEvent> {
    sanitize::sanitize_invalid_player_inputs(game_data)
}

pub fn process_autopilot_ai(
    game_data: &mut CoreGameData,
) -> Result<(), Box<dyn std::error::Error>> {
    economics::process_autopilot_ai(game_data)
}

pub fn run_maintenance_turns(
    game_data: &mut CoreGameData,
    turns: u16,
) -> Result<u16, Box<dyn std::error::Error>> {
    for _ in 0..turns {
        run_maintenance_turn_with_seed(game_data, 0)?;
    }
    Ok(game_data.conquest.game_year())
}
