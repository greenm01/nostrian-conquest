//! Canonical EC combat resolution.
//!
//! The structure here owes an explicit debt to *Empire of the Sun*: both sides
//! compute their blows from the same moment in time, and only then does the
//! board reckon with the cost. That simultaneous exchange fits EC's manuals
//! better than file-order skirmishes, while staying seeded and reproducible
//! for Rust maintenance and classic save compatibility.

mod assault;
mod exchange;
mod fleet_battle;
mod reporting;
mod retreat;
mod state;

pub(crate) use assault::process_planetary_assaults;
pub(crate) use fleet_battle::{
    FleetBattlePhaseEvents, process_departure_interceptions, process_fleet_battles,
};
pub(crate) use retreat::{abort_mission_to_seek_home_or_hold, nearest_owned_planet};
