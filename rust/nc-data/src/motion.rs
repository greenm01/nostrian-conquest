//! Where a fleet or starbase sits inside its sector.

use crate::{Fleet, Real48, Starbase};

/// At rest in orbit at the sector centre: offsets 0.5, depth 1.0.
pub fn set_fleet_at_rest(fleet: &mut Fleet) {
    fleet.x_fraction = Real48::HALF;
    fleet.y_fraction = Real48::HALF;
    fleet.depth = Real48::ONE;
}

/// At rest in orbit at the sector centre: offsets 0.5, depth 1.0.
pub fn set_starbase_at_rest(base: &mut Starbase) {
    base.x_fraction = Real48::HALF;
    base.y_fraction = Real48::HALF;
    base.depth = Real48::ONE;
}
