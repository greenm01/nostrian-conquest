//! Training-side views of a game for learned AI seats.
//!
//! A [`SeatMemory`] holds what one empire knows: its planet intel and the
//! foreign forces its reports named. [`observe`] turns that memory and the
//! empire's own records into fixed-width rows of numbers, and nothing else:
//! an observation never reads another empire's records. [`SeatedGame`]
//! plays a [`nc_harness::ScenarioPlay`] and updates every seat's memory
//! after each maintenance year, as campaign maintenance does.

mod game;
mod memory;
mod observe;

pub use game::SeatedGame;
pub use memory::{CONTACT_MEMORY_YEARS, Contact, ContactSource, SeatMemory};
pub use observe::{
    CONTACT_WIDTH, FLEET_WIDTH, GLOBAL_WIDTH, Observation, PLANET_WIDTH, STARBASE_WIDTH, observe,
};
