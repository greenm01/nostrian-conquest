//! Which ships a detach or transfer actually moves. One rule, used by the
//! mutators and by the door's previews, so a confirmation cannot promise
//! different hulls than the move takes.
//!
//! A selection names a number of ships per category. A plain number takes
//! healthy ships first and crippled ones only when a category has too few
//! healthy ships; `n!` (see [`FleetDetachSelection::required_crippled`]) takes
//! exactly `n` crippled ships and never stands in healthy ones. Among crippled
//! ships, the most recovery turns left go first, this pass's newly crippled
//! before all (see [`CrippledHulls::take`]). Recovery state moves intact:
//! nothing here heals a ship or resets a counter.

use crate::{
    CrippledShips, Fleet, FleetDetachSelection, HullClass, ShipCategory, crippled_by_category,
    join_tokens, ship_token,
};

/// A request for more crippled ships than a category has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrippledShortage {
    pub category: ShipCategory,
    pub requested: u16,
    pub available: u32,
}

/// What a selection takes from a fleet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedSelection {
    /// Per category: the ships taken and how many of them are crippled.
    pub groups: [(u32, u32); ShipCategory::COUNT],
    /// The recovery state that goes with the crippled ships taken.
    pub moved: CrippledShips,
    /// The recovery state the donor keeps.
    pub remaining: CrippledShips,
}

impl ResolvedSelection {
    /// The ships taken, in notation: `CA3!1 TT*2`. `none` when empty.
    pub fn notation(&self) -> String {
        let notation = join_tokens(ShipCategory::ALL.into_iter().map(|category| {
            let (total, crippled) = self.groups[category.index()];
            ship_token(category.code(), total, crippled)
        }));
        if notation.is_empty() {
            "none".to_string()
        } else {
            notation
        }
    }
}

/// Resolves `selection` against `fleet`. The selection must not ask for more
/// ships than a category has (the mutators and the door check that first);
/// it can still ask for more crippled ships than the category has.
pub fn resolve_selection(
    fleet: &Fleet,
    selection: &FleetDetachSelection,
) -> Result<ResolvedSelection, CrippledShortage> {
    let pool = crippled_by_category(fleet);
    let mut groups = [(0, 0); ShipCategory::COUNT];
    for category in ShipCategory::ALL {
        let slot = category.index();
        let taken = u32::from(selection.count(category));
        let required = u32::from(selection.required_crippled[slot]).min(taken);
        let crippled = pool[slot];
        if required > crippled {
            return Err(CrippledShortage {
                category,
                requested: selection.required_crippled[slot],
                available: crippled,
            });
        }
        let healthy = category.ships_in(fleet).saturating_sub(crippled);
        let healthy_taken = (taken - required).min(healthy);
        groups[slot] = (taken, (taken - healthy_taken).min(crippled));
    }

    let mut remaining = fleet.crippled;
    let mut moved = CrippledShips::default();
    for class in HullClass::ALL {
        let crippled_taken = ShipCategory::ALL
            .into_iter()
            .filter(|category| category.hull_class() == class)
            .map(|category| groups[category.index()].1)
            .sum();
        *moved.get_mut(class) = remaining.get_mut(class).take(crippled_taken);
    }
    Ok(ResolvedSelection {
        groups,
        moved,
        remaining,
    })
}

/// How many more crippled ships of `category` the player could stage on top
/// of `selection`: the ones the staged ships do not already take.
pub fn crippled_left_to_stage(
    fleet: &Fleet,
    selection: &FleetDetachSelection,
    category: ShipCategory,
) -> u32 {
    let slot = category.index();
    resolve_selection(fleet, selection).map_or(0, |resolved| {
        crippled_by_category(fleet)[slot].saturating_sub(resolved.groups[slot].1)
    })
}
