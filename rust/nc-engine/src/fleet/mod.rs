pub mod eta;
pub mod selection;
pub mod starbase;
pub mod targeting;

pub use eta::{
    fleet_eta_estimate_sort_key, fleet_eta_label, fleet_list_eta_label,
    fleet_target_eta_confirmation_message, fleet_target_eta_estimate, fleet_target_eta_message,
};
pub use selection::{
    CheckedFleetMergePlan, CheckedFleetTransferPlan, SelectedFleetRef,
    resolve_checked_fleet_merge_plan, resolve_checked_fleet_transfer_plan,
};
pub use starbase::{
    format_guard_fleet_clause, format_starbase_list_guard_label,
    format_starbase_review_guard_label, guard_fleet_numbers_for_starbase, starbase_eta_label,
    starbase_operation_label,
};
pub use targeting::{
    OwnedFleetTarget, OwnedStarbaseTarget, automatic_order_targets, default_host_fleet_target,
    default_starbase_target, fleet_mission_requires_preselected_target,
    fleet_order_target_rejects_owned_planet, fleet_order_target_rejects_owned_scout_target,
    fleet_order_target_requires_owned_planet, fleet_order_target_requires_planet_system,
    fleet_order_target_y_depends_on_entered_x, owned_fleet_targets, owned_starbase_targets,
    recommended_coordinate_target, recommended_coordinate_target_candidates,
    recommended_coordinate_target_y_for_entered_x, target_available_for_mission,
};
