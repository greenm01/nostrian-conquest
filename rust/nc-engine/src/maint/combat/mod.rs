//! Canonical EC combat resolution.
//!
//! The structure here owes an explicit debt to *Empire of the Sun*: both sides
//! compute their blows from the same moment in time, and only then does the
//! board reckon with the cost. That simultaneous exchange fits EC's manuals
//! better than file-order skirmishes, while staying seeded and reproducible
//! for Rust maintenance and classic save compatibility.

mod assault;
#[cfg(test)]
mod assault_tests;
#[cfg(test)]
mod capture_tests;
mod exchange;
mod fleet_battle;
#[cfg(test)]
mod fleet_battle_tests;
mod reporting;
mod retreat;
mod state;

pub(crate) use assault::{AssaultEvents, resolve_strike};
pub(crate) use fleet_battle::{BattleSide, FleetBattlePhaseEvents, fight_encounter};
pub(crate) use reporting::{
    ContactObserver, mission_kind_for_order, push_fleet_contact, push_no_engagement,
};
pub(crate) use retreat::{abort_mission_to_seek_home_or_hold, nearest_friendly_world};
