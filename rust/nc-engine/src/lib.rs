pub mod fleet;
pub mod maint;
pub mod navigation;
pub mod orders;
pub mod planet;
pub mod setup;

pub use fleet::{
    CheckedFleetMergePlan, CheckedFleetTransferPlan, OwnedFleetTarget, OwnedStarbaseTarget,
    SEEK_NEXT_CLOSEST_PROMPT, SeekNextClosestQuestion, SelectedFleetRef, automatic_order_targets,
    default_host_fleet_target, default_starbase_target, fleet_eta_estimate_sort_key,
    fleet_eta_label, fleet_list_eta_label, fleet_mission_requires_preselected_target,
    fleet_order_target_rejects_owned_planet, fleet_order_target_rejects_owned_scout_target,
    fleet_order_target_requires_owned_planet, fleet_order_target_requires_planet_system,
    fleet_order_target_y_depends_on_entered_x, fleet_target_eta_confirmation_message,
    fleet_target_eta_estimate, fleet_target_eta_message, format_guard_fleet_clause,
    format_starbase_list_guard_label, format_starbase_review_guard_label,
    guard_fleet_numbers_for_starbase, owned_fleet_targets, owned_starbase_targets,
    recommended_coordinate_target, recommended_coordinate_target_candidates,
    recommended_coordinate_target_y_for_entered_x, resolve_checked_fleet_merge_plan,
    resolve_checked_fleet_transfer_plan, seek_next_closest_question, starbase_eta_label,
    starbase_operation_label, target_available_for_mission,
};
pub use maint::{
    CampaignMaintenance, MaintenancePreflightError, apply_results_reviewable_flags,
    build_results_dat, build_results_report_blocks, process_autopilot_ai, run_maintenance_turn,
    run_maintenance_turn_with_context, run_maintenance_turn_with_context_and_seed,
    run_maintenance_turn_with_context_seed_and_lifecycle, run_maintenance_turn_with_seed,
    run_maintenance_turn_with_visible_hazards, run_maintenance_turn_with_visible_hazards_and_seed,
    run_maintenance_turns, validate_maintenance_state,
};
pub use navigation::{
    FleetEtaEstimate, PlannedRoute, RouteStep, VisibleHazardIntel, estimate_direct_eta,
    estimate_fleet_eta, estimate_fleet_eta_to_destination, next_path_step, plan_route,
    plan_route_with_intel, visible_hazard_intel_from_snapshots,
};
pub use nc_data::{
    AssaultReportEvent, BombardEvent, CampaignOutcome, CampaignOutcomeEvent, CampaignOutlook,
    CampaignOutlookEvent, CampaignState, CivilDisorderEvent, ColonizationResolvedEvent,
    CommissionResult, Conquest, ContactReportSource, CoreGameData, DiplomacyConfig,
    DiplomacyDirective, DiplomacyOverride, DiplomaticEscalationEvent, DiplomaticRelation,
    EmpireEconomySummary, EmpireEliminationCause, EmpireEliminationEvent, EmpirePlanetEconomyRow,
    EmpireProductionRankingRow, EmpireProductionRankingSort, EncounterDispositionEvent,
    EncounterDispositionReason, FleetBattleEvent, FleetDefectionEvent, FleetDestroyedEvent,
    FleetMergeEvent, FleetOrderSpec, FleetOrderValidationError, FleetPlayerInputValidationError,
    GameRng, GameStateBuilder, GameVictoryNoticeEvent, GuardStarbaseSpec, InvalidPlayerStateEvent,
    JoinMissionHostEvent, MaintenanceEvents, Mission, MissionAbortReason, MissionEvent,
    MissionOutcome, MissionRetargetEvent, Order, Planet, PlanetBuildSpec, PlanetIntelEvent,
    PlanetIntelSource, PlanetOwnershipChangeEvent, PlanetPlayerInputValidationError,
    PlayerDiplomacyValidationError, ProductionItemKind, RNG_TAG_COMBAT, STARDOCK_SLOT_COUNT,
    SalvageResolvedEvent, ScoutContactEvent, SetupConfigError, ShipLosses, Starbase, StarbaseAlert,
    StarbaseDestroyedEvent, StarbaseSightingEvent, build_capacity, map_size_for_player_count,
};
pub use orders::{
    BUILD_UNITS, BuildUnitSpec, FLEET_MISSION_OPTIONS, FleetMissionOption, FleetMissionRequirement,
    FleetTargetInputKind, build_kind_count_label, build_kind_name, build_quantity_from_points,
    build_unit_spec, build_unit_spec_by_kind, fleet_mission_option,
    fleet_record_supports_mission_code, fleet_record_supports_requirement, fleet_target_input_kind,
    fleet_target_status_line, max_quantity,
};
pub use planet::{
    ArmyTransportMode, PlanetBuildListEntry, PlanetBuildOrderLine, PlanetBuildSpecifyEntry,
    PlanetBuildViewStats, PlanetCommissionDraftEntry, PlanetCommissionDraftState,
    PlanetCommissionSlotEntry, PlanetTransportFleetCandidate, PlanetTransportPlanetCandidate,
    PlanetTransportSelectionError, commission_fleet_draft_from_entries,
    default_fleet_transport_fleet_number, planet_build_committed_points, planet_build_list_entries,
    planet_build_max_quantity, planet_build_max_selectable_unit_number, planet_build_orders,
    planet_build_specify_entries, planet_build_unavailable_message, planet_build_view,
    planet_building_unit_count, planet_commission_draft_state, planet_commission_slot_entries,
    planet_docked_unit_count, planet_has_any_buildable_unit, production_item_kind_code,
    resolve_planet_transport_fleet_selection, transport_available_qty,
    transport_fleet_candidates_for_planet, transport_planet_candidates,
};
pub use setup::{
    GeneratedMap, GeneratedWorld, MapMetrics, build_seeded_initialized_game, build_seeded_new_game,
    generate_map,
};
