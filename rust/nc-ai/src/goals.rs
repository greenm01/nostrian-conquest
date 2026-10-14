//! A seat's strategic goals for one year: what a strategist network
//! chooses and the executor carries out ([`crate::execute_goals`]).
//!
//! The executor's own turn is its goals ([`crate::plan_goals`]) carried out,
//! so its goals are exact imitation labels. Goal masks read only the seat's
//! [`SeatMasks`] and [`SeatMemory`].

use nc_data::{CoreGameData, IntelTier, Order};

use crate::action::{DIPLOMACY_CHOICES, SeatMasks};
use crate::executor::Weights;
use crate::memory::SeatMemory;

/// Build postures, each an adjustment of the weights' build targets:
/// balanced (the weights as they are), expand, defend and attack.
pub const POSTURES: usize = 4;

/// One seat's goals for a year.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Goals {
    /// Tax choice, as [`crate::SeatAction::tax`].
    pub tax: usize,
    /// One choice per empire slot, as [`crate::SeatAction::diplomacy`].
    pub diplomacy: Vec<usize>,
    /// Build posture, below [`POSTURES`].
    pub posture: usize,
    /// 0 holds every strike. `k` makes world `k - 1` (0-based) the target of
    /// every home force strong enough to take it; weaker forces pick their
    /// own targets.
    pub strike: usize,
    /// 0 lets every ETAC fleet pick its world. `k` sends the first ETAC
    /// fleet needing orders to world `k - 1`; the rest pick their own.
    pub colonize: usize,
}

/// What a seat may choose as goals. Tax and posture are always free.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalMasks {
    pub diplomacy: Vec<[bool; DIPLOMACY_CHOICES]>,
    /// Hold, then one flag per world: known to be owned by another empire.
    pub strike: Vec<bool>,
    /// Auto, then one flag per world: not known to be owned, and no ETAC
    /// of the seat's already flies there.
    pub colonize: Vec<bool>,
}

/// The goals `memory`'s empire may choose, given its `masks`.
pub fn goal_masks(game_data: &CoreGameData, memory: &SeatMemory, masks: &SeatMasks) -> GoalMasks {
    let empire = memory.empire;
    let claimed = claimed_worlds(game_data, masks);
    let world = |pick: &dyn Fn(usize, &nc_data::PlanetIntelSnapshot) -> bool| {
        std::iter::once(true)
            .chain((0..game_data.planets.len()).map(|idx| {
                memory
                    .planet_intel
                    .get(&(idx + 1))
                    .is_some_and(|intel| pick(idx, intel))
            }))
            .collect()
    };
    GoalMasks {
        diplomacy: masks.diplomacy.clone(),
        strike: world(&|_, intel| {
            intel
                .known_owner_empire_id
                .is_some_and(|owner| owner != 0 && owner != empire)
        }),
        colonize: world(&|idx, intel| colonizable(intel, empire) && !claimed.contains(&idx)),
    }
}

/// `weights` with `posture`'s build targets.
pub fn posture_weights(weights: &Weights, posture: usize) -> Weights {
    let mut adjusted = *weights;
    match posture {
        1 => {
            adjusted.etacs += 3.0;
            adjusted.scouts += 1.0;
            adjusted.home_armies *= 0.75;
            adjusted.colony_armies *= 0.5;
        }
        2 => {
            adjusted.home_armies *= 1.5;
            adjusted.colony_armies *= 1.5;
            adjusted.batteries *= 1.5;
            adjusted.etacs *= 0.5;
        }
        3 => {
            adjusted.transports *= 1.5;
            adjusted.home_armies *= 0.75;
            adjusted.colony_armies *= 0.5;
            adjusted.batteries *= 0.5;
        }
        _ => {}
    }
    adjusted
}

/// A world worth sending an ETAC to: not known to be owned by anyone.
pub(crate) fn colonizable(intel: &nc_data::PlanetIntelSnapshot, empire: u8) -> bool {
    intel.intel_tier != IntelTier::Owned
        && intel.known_owner_empire_id.is_none_or(|owner| owner == 0)
        && intel.known_owner_empire_id != Some(empire)
}

/// Worlds (0-based) an ETAC fleet of the seat already flies to colonize.
pub(crate) fn claimed_worlds(game_data: &CoreGameData, masks: &SeatMasks) -> Vec<usize> {
    masks
        .fleets
        .iter()
        .filter_map(|mask| {
            let fleet = &game_data.fleets[mask.record - 1];
            (fleet.ships.etacs > 0 && fleet.order == Order::ColonizeWorld)
                .then(|| world_at(game_data, fleet.target.to_array()))
                .flatten()
        })
        .collect()
}

pub(crate) fn world_at(game_data: &CoreGameData, coords: [u8; 2]) -> Option<usize> {
    game_data
        .planets
        .iter()
        .position(|planet| planet.sector.to_array() == coords)
}
