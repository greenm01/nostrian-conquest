//! ECUTIL's starting worlds: each joined empire's homeworld starts named
//! "Not Named Yet" with present production 100.

use crate::Planet;

pub fn is_named_homeworld_seed(planet: &Planet) -> bool {
    planet.name.trim() == "Not Named Yet"
}

/// Present production in `64 <= P < 128`, the band ECUTIL's homeworld seed
/// (100) falls in.
pub fn has_homeworld_seed_production(planet: &Planet) -> bool {
    (64.0..128.0).contains(&planet.production.to_f64())
}

/// A fully owned world with homeworld seed production, whatever its name.
pub fn is_homeworld_seed_ignoring_name(planet: &Planet) -> bool {
    planet.years_under_owner == 2 && planet.owner.0 != 0 && has_homeworld_seed_production(planet)
}
