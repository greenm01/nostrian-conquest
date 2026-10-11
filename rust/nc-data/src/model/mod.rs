//! Game state as plain typed structs with named fields. Every field carries
//! the classic file offset it came from; the byte layouts themselves live in
//! the `nc-compat` codecs.

mod conquest;
mod crippled;
mod fleet;
mod ids;
mod ipbm;
mod planet;
mod player;
mod setup;
mod starbase;

pub use conquest::{Conquest, MaintenanceDays};
pub use crippled::{CrippledHulls, CrippledShips, HullClass, RECOVERY_TURNS, Recovered};
pub use fleet::{EscapeEpisode, Fleet, Order, ShipCounts};
pub use ids::{Coords, EmpireId, FleetId, RecordNumber, StarbaseId};
pub use ipbm::Ipbm;
pub use planet::{BuildSlot, Planet, ProductionItemKind, Stardock};
pub use player::{DiplomaticRelation, Player, PlayerMode};
pub use setup::Setup;
pub use starbase::Starbase;
