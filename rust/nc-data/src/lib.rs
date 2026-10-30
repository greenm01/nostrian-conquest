/// ROE of a fleet that detach or commissioning creates while it has a
/// battleship, cruiser, or destroyer; one without gets 0. ECGAME's default,
/// which its ROE prompt offers.
pub const NEW_FLEET_DEFAULT_ROE: u8 = 6;

pub mod access_code;
mod account;
mod bbs_config;
mod builder;
mod classic_results;
mod config;
mod diplomacy;
mod directory;
mod economy;
mod fleet_orders;
mod fleet_pick;
mod fleet_ships;
mod homeworld_seed;
pub mod hosted;
mod intel;
pub mod maintenance_types;
mod map_dimensions;
mod map_export;
pub mod matrix_login;
pub mod model;
mod motion;
mod names;
mod planet_summary;
mod player_activity;
mod player_lifecycle;
mod player_mail;
mod player_war_stats;
mod rankings;
mod real48;
mod report_blocks;
mod rng;
mod runtime_reports;
mod ship_notation;
pub mod sqlite;
mod stardock;
mod starmap;
mod state_hash;
mod storage;
mod support;
mod turns;

pub use account::AccountIdentity;
pub use bbs_config::{
    BbsGameConfig, BbsGameConfigError, SeatReservation, check_new_ai_seats, sync_game_config,
    write_game_config,
};
pub use builder::{
    FleetOrderSpec, GameStateBuilder, GuardStarbaseSpec, PlanetBuildSpec,
    reset_player_slot_to_baseline,
};
pub use classic_results::{
    CLASSIC_RESULTS_RECORD_SIZE, ClassicResultsChain, ClassicResultsLayout,
    classic_results_header_viewers, has_results_chain, layout_classic_results, results_chain,
    set_login_reviewables_present, set_results_chain,
};
pub use config::{DiplomacyConfig, DiplomacyDirective, SetupConfigError};
pub use diplomacy::{
    diplomatic_relation_toward, set_diplomatic_relation_toward, stored_diplomatic_relation,
};
pub use directory::{
    AutoCommissionEntry, AutoCommissionFleetEntry, AutoCommissionReport,
    AutoCommissionStarbaseEntry, CampaignOutcome, CampaignOutlook, CampaignState,
    CommissionFleetDraft, CommissionResult, CoreGameData, CurrentKnownComplianceStatus,
    CurrentKnownGuardStarbaseLinkageSummary, CurrentKnownKeyWordSummary, EmpireEconomySummary,
    EmpirePlanetEconomyRow, EmpireProductionRankingRow, EmpireProductionRankingSort,
    EmpireUnitSummary, FleetDetachResult, FleetDetachSelection, FleetOrderValidationError,
    FleetPlayerInputValidationError, FleetTransferResult, GameStateMutationError,
    PLANET_BUILD_CURRENT_KNOWN_ORDERS, PlanetPlayerInputValidationError, PlanetSpendLimits,
    PlayerDiplomacyValidationError, SeekHomeTarget,
};
pub use economy::{
    NextPass, build_capacity, next_pass, next_production, present_production_points,
    starbase_growth_bonus, stored_production_points, tax_income, whole_points, yearly_growth,
    yearly_high_tax_penalty,
};
pub use fleet_orders::{order_summary, set_mission_target};
pub use fleet_pick::{
    CrippledShortage, ResolvedSelection, crippled_left_to_stage, resolve_selection,
};
pub use fleet_ships::{
    crippled_hull_count, has_any_force, has_combat_ships, hull_count, is_support_only,
    recompute_max_speed, set_armies_aboard, set_troop_transports, ship_composition_table_summary,
    slowest_speed, starship_count, troop_transports,
};
pub use homeworld_seed::{
    has_homeworld_seed_production, is_homeworld_seed_ignoring_name, is_named_homeworld_seed,
};
pub use intel::{
    active_starbase_count_at, build_runtime_planet_intel_snapshot,
    latest_planet_intel_grants_for_viewer, merge_player_intel_from_runtime,
};
pub use maintenance_types::{
    AssaultReportEvent, BlockingForce, BombardEvent, CampaignOutcomeEvent, CampaignOutlookEvent,
    CivilDisorderEvent, ColonizationResolvedEvent, ContactReportSource, DiplomacyOverride,
    DiplomaticEscalationEvent, EmpireEliminationCause, EmpireEliminationEvent,
    EncounterDispositionEvent, EncounterDispositionReason, FleetBattleEvent, FleetDefectionEvent,
    FleetDestroyedEvent, FleetMergeEvent, GameVictoryNoticeEvent, InheritedAssets,
    InvalidPlayerStateEvent, JoinMissionHostEvent, LostWorldResponse, MaintenanceEvents, Mission,
    MissionAbortReason, MissionEvent, MissionOutcome, MissionRetargetEvent, PlanetIntelEvent,
    PlanetIntelSource, PlanetOwnershipChangeEvent, PlanetScorchEvent, RecoveredUnit,
    SalvageResolvedEvent, ScoutContactEvent, ShipLosses, ShipsRecoveredEvent, StarbaseAlert,
    StarbaseDestroyedEvent, StarbaseReport, StarbaseReportEvent, StarbaseSightingEvent,
};
pub use map_dimensions::{is_valid_player_count, map_size_for_player_count, sector_within_map};
pub use map_export::{
    PlayerMapExportData, PlayerMapExportFile, STARMAP_CSV_FILE_NAME, STARMAP_DETAILS_CSV_FILE_NAME,
    STARMAP_TEXT_FILE_NAME, build_galaxy_map_export_data, build_player_map_export_data,
};
pub use model::{
    BuildSlot, CONTACT_MEMORY_YEARS, Conquest, Contact, Coords, CrippledHulls, CrippledShips, DiplomaticRelation, EmpireId,
    EscapeEpisode, Fleet, FleetId, HullClass, Ipbm, MaintenanceDays, Order, Planet, Player,
    PlayerMode, ProductionItemKind, RECOVERY_TURNS, RecordNumber, Recovered, Setup, ShipCounts,
    Sighting, SightingMemory, Starbase, StarbaseId, Stardock,
};
pub use motion::{set_fleet_at_rest, set_starbase_at_rest};
pub use names::{
    name_unnamed_worlds, ownership_summary, planet_name_in_use, player_handle_matches,
    set_empire_name, set_planet_name, set_player_handle,
};
pub use planet_summary::{
    CompactUnitSummaryStyle, OrbitPresenceSummary, OwnedPlanetStatus, build_queue_unit_counts,
    format_build_queue_summary, format_owned_orbit_summary, format_stardock_summary,
    format_unit_counts, ordered_unit_count_entries, owned_orbit_presence, owned_planet_status,
    planet_derived_summary, stardock_unit_counts,
};
pub use player_activity::{
    DEFAULT_INACTIVITY_AUTOPILOT_AFTER_TURNS, apply_inactivity_autopilot_policy,
    clear_inactivity_autopilot_pending, default_player_activity_states,
    record_interactive_participation, record_submitted_turn_participation,
};
pub use player_lifecycle::{
    PlayerAccessMode, PublicEmpireStatus, default_player_lifecycle_states,
    empire_has_recovery_path, is_campaign_contender, is_contesting_empire, player_access_mode,
    player_public_status, set_civil_disorder, set_rogue,
};
pub use player_mail::{
    MAX_QUEUED_MESSAGES_PER_RECIPIENT_PER_YEAR, QueuedPlayerMail, append_mail_queue,
    clear_mail_queue, load_mail_queue, queued_message_count_for_sender_recipient_year,
    save_mail_queue, validate_queue_message_limit, validate_queued_player_message,
};
pub use player_war_stats::{
    apply_maintenance_events_to_player_war_stats, default_player_war_stats_states,
};
pub use rankings::snapshot_previous_year;
pub use real48::Real48;
pub use report_blocks::{
    ReportBlockRow, carry_unread_report_blocks, delete_report_block_for_viewer,
    delete_report_blocks_for_viewer,
};
pub use rng::{
    GameRng, RNG_TAG_COMBAT, RNG_TAG_ECONOMY, RNG_TAG_ENCOUNTER, RNG_TAG_MAPGEN, RNG_TAG_ORDERS,
    derive_campaign_seed_from_runtime, generate_campaign_seed, mix_seed,
};
pub use runtime_reports::{
    InboxItem, InboxItemSource, InboxItemType, ReportSummaryBucket, ReportsPreview, ReviewBlock,
    has_visible_runtime_messages, has_visible_runtime_reports, runtime_inbox_items,
    runtime_inbox_preview_lines, runtime_outgoing_inbox_items, runtime_outgoing_message_lines,
    runtime_sent_inbox_items, wrap_review_text_preserving_spacing,
};
pub use ship_notation::{
    Composition, NOTATION_LEGEND, STARBASE_CODE, ShipCategory, auto_commission_notation,
    crippled_by_category, fit_notation, fleet_notation, join_tokens, ship_token,
};
pub use stardock::{STARDOCK_SLOT_COUNT, stardock_slot_for_kind, stardock_slot_kind};
pub use starmap::{
    PlayerStarmapProjection, PlayerStarmapWorld, build_player_starmap_projection_from_snapshots,
};
pub use storage::{
    AiDifficulty, AiSeat, CampaignRuntimeState, CampaignSettings, CampaignStore,
    CampaignStoreError, CommittedChange, DEFAULT_AI_PERSONALITY, DEFAULT_CAMPAIGN_DB_NAME,
    DEFAULT_CAMPAIGN_THEME_KEY, DEFAULT_MAINTENANCE_INTERVAL_MINUTES, IntelTier, MaintenanceRun,
    PlanetIntelSnapshot, PlayerActivityState, PlayerLifecycleState, PlayerWarStatsState,
    RuntimeChange, STALE_SESSION_MESSAGE, SeatBindSource, TerminalOutcome, WinnerState,
};
pub use support::{ParseError, is_classic_text_char};
pub use turns::{
    FleetTurnAction, FleetTurnBlock, MAX_MESSAGE_BODY_CHARS, MAX_MESSAGE_SUBJECT_CHARS,
    PlanetTurnAction, PlanetTurnBlock, TurnDiplomacyDirective, TurnMessage, TurnSubmission,
    TurnSubmissionError, TurnSubmissionReport,
};
