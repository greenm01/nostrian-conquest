//! Training-side views of a game for learned AI seats.
//!
//! A [`SeatMemory`] holds what one empire knows: its planet intel and the
//! foreign forces its reports named. [`observe`] turns that memory and the
//! empire's own records into fixed-width rows of numbers, and nothing else:
//! an observation never reads another empire's records. [`SeatedGame`]
//! plays a [`nc_harness::ScenarioPlay`] and updates every seat's memory
//! after each maintenance year, as campaign maintenance does.
//! [`seat_masks`] says which orders a seat may choose, from the game's own
//! validators, and [`decode_action`] builds the [`nc_data::TurnSubmission`]
//! for a choice without going through turn KDL.

mod action;
mod game;
mod memory;
mod observe;

pub use action::{
    BUILD_AMOUNT_CHOICES, BUILD_CHOICES, DIPLOMACY_CHOICES, DecodedTurn, FleetChoice, FleetMask,
    MISSION_CHOICES, MISSIONS, PlanetChoice, PlanetMask, ROE_CHOICES, SPEED_CHOICES, SeatAction,
    SeatMasks, TAX_CHOICES, TAX_STEP, commission_submission, decode_action, seat_masks,
};
pub use game::SeatedGame;
pub use memory::{CONTACT_MEMORY_YEARS, Contact, ContactSource, SeatMemory};
pub use observe::{
    CONTACT_WIDTH, FLEET_WIDTH, GLOBAL_WIDTH, Observation, PLANET_WIDTH, STARBASE_WIDTH, observe,
};
