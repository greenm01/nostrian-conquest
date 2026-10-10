//! Game state as plain typed structs with named fields. Every field carries
//! the classic file offset it came from; the byte layouts themselves live in
//! [`crate::classic`].

mod conquest;
mod fleet;
mod ids;
mod ipbm;
mod planet;
mod player;
mod setup;
mod starbase;

pub use conquest::{Conquest, MaintenanceDays};
pub use fleet::{Fleet, Order, ShipCounts};
pub use ids::{Coords, EmpireId, FleetId, RecordNumber, StarbaseId};
pub use ipbm::Ipbm;
pub use planet::{BuildQueue, Planet, Stardock};
pub use player::{DiplomaticRelation, Player, PlayerMode};
pub use setup::Setup;
pub use starbase::Starbase;
