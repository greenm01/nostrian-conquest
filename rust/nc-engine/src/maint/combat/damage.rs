//! Writing a battle's result back to the records. A battle fights on one
//! aggregate state per side; this spreads what changed over the side's
//! fleets and starbases: which of them lose hulls, which hulls are finished
//! (those crippled this pass first, then the ones with the most turns left),
//! and which nominal hulls end the battle crippled. Nothing is restored
//! afterwards: crippled hulls stay crippled until the recovery step.

use nc_data::{
    CoreGameData, CrippledHulls, HullClass, RECOVERY_TURNS, has_any_force, hull_count,
    recompute_max_speed,
};

use super::state::{FleetCombatState, IDX_SB, class_index};

/// One record's hulls of a class: how many it holds and how many of them
/// are crippled.
#[derive(Clone, Copy, Debug)]
pub(super) struct Holder {
    pub(super) hulls: u32,
    pub(super) crippled: CrippledHulls,
}

impl Holder {
    fn nominal(&self) -> u32 {
        self.hulls - self.crippled.total()
    }
}

/// Spread a side's change in one class, `before` to `after` (hulls and
/// crippled hulls), over `holders` in order. Finished hulls leave the
/// holders that have them, cohort by cohort; nominal hulls destroyed leave
/// the first holders that have any; hulls newly crippled are taken from the
/// first nominal hulls left.
pub(super) fn apply_class_change(
    holders: &mut [Holder],
    before: (u32, &CrippledHulls),
    after: (u32, &CrippledHulls),
) {
    // Finishing is the only way a recovering cohort shrinks. This pass's
    // cohort also gains the hulls crippled in the battle, so a hull
    // crippled and finished within it shows as a nominal hull destroyed.
    let mut removals = [0u32; RECOVERY_TURNS + 1];
    for (i, removal) in removals.iter_mut().take(RECOVERY_TURNS).enumerate() {
        *removal = before.1.turns[i] - after.1.turns[i];
    }
    removals[RECOVERY_TURNS] = before.1.fresh.saturating_sub(after.1.fresh);
    let newly_crippled = after.1.fresh.saturating_sub(before.1.fresh);

    for (cohort, &removal) in removals.iter().enumerate() {
        let mut left = removal;
        for holder in holders.iter_mut() {
            let count = match cohort {
                RECOVERY_TURNS => &mut holder.crippled.fresh,
                i => &mut holder.crippled.turns[i],
            };
            let take = left.min(*count);
            *count -= take;
            holder.hulls -= take;
            left -= take;
        }
    }
    let mut nominal_removed = before.0 - after.0 - removals.iter().sum::<u32>();
    for holder in holders.iter_mut() {
        let take = nominal_removed.min(holder.nominal());
        holder.hulls -= take;
        nominal_removed -= take;
    }
    let mut to_cripple = newly_crippled;
    for holder in holders.iter_mut() {
        let take = to_cripple.min(holder.nominal());
        holder.crippled.cripple(take);
        to_cripple -= take;
    }
}

/// Put the result of a battle or strike, `before` to `after`, on the side's
/// fleets: their ships lost and their crippled hulls finished or added. A
/// survivor moves at the speed of the ships it has left (classic recomputes
/// it after the battle too, `24a3:036b`); a fleet with none left is culled,
/// not re-rated.
pub(super) fn distribute_fleet_losses(
    game_data: &mut CoreGameData,
    fleet_indices: &[usize],
    before: &FleetCombatState,
    after: &FleetCombatState,
) {
    let mut lost_ships = vec![false; fleet_indices.len()];
    for class in HullClass::ALL {
        let idx = class_index(class);
        let mut holders: Vec<Holder> = fleet_indices
            .iter()
            .map(|&fleet_idx| {
                let fleet = &game_data.fleets[fleet_idx];
                Holder {
                    hulls: hull_count(fleet, class),
                    crippled: *fleet.crippled.get(class),
                }
            })
            .collect();
        let held: Vec<u32> = holders.iter().map(|holder| holder.hulls).collect();
        apply_class_change(
            &mut holders,
            (before.counts[idx], &before.crippled[idx]),
            (after.counts[idx], &after.crippled[idx]),
        );
        for (n, &fleet_idx) in fleet_indices.iter().enumerate() {
            let fleet = &mut game_data.fleets[fleet_idx];
            *fleet.crippled.get_mut(class) = holders[n].crippled;
            let lost = (held[n] - holders[n].hulls) as u16;
            if lost > 0 {
                remove_ships(fleet, class, lost);
                lost_ships[n] = true;
            }
        }
    }
    for (n, &fleet_idx) in fleet_indices.iter().enumerate() {
        let fleet = &mut game_data.fleets[fleet_idx];
        if lost_ships[n] && has_any_force(fleet) {
            recompute_max_speed(fleet);
        }
    }
}

/// Take `lost` ships of `class` out of the fleet. Transport losses fall on
/// loaded transports first, each taking its army with it.
fn remove_ships(fleet: &mut nc_data::Fleet, class: HullClass, lost: u16) {
    match class {
        HullClass::Destroyer => fleet.ships.destroyers -= lost,
        HullClass::Cruiser => fleet.ships.cruisers -= lost,
        HullClass::Battleship => fleet.ships.battleships -= lost,
        HullClass::Scout => fleet.ships.scouts -= lost,
        HullClass::Etac => fleet.ships.etacs -= lost,
        HullClass::Transport => {
            let loaded = lost.min(fleet.ships.loaded_transports);
            fleet.ships.loaded_transports -= loaded;
            fleet.ships.empty_transports -= lost - loaded;
        }
    }
}

/// Put the result of a battle on the side's starbase records: a base
/// destroyed is unlinked and zeroed, one that survives keeps or gains its
/// crippled hull. Returns the ids of the bases destroyed.
pub(super) fn write_back_bases(
    game_data: &mut CoreGameData,
    bases: &[usize],
    before: &FleetCombatState,
    after: &FleetCombatState,
) -> Vec<u16> {
    let mut holders: Vec<Holder> = bases
        .iter()
        .map(|&idx| Holder {
            hulls: 1,
            crippled: game_data.bases[idx].crippled,
        })
        .collect();
    apply_class_change(
        &mut holders,
        (before.counts[IDX_SB], &before.crippled[IDX_SB]),
        (after.counts[IDX_SB], &after.crippled[IDX_SB]),
    );
    let mut destroyed = Vec::new();
    for (&idx, holder) in bases.iter().zip(&holders) {
        if holder.hulls == 0 {
            destroyed.push(game_data.bases[idx].base_id.0);
            game_data.unlink_base(idx);
            game_data.bases[idx] = nc_data::Starbase::default();
        } else {
            game_data.bases[idx].crippled = holder.crippled;
        }
    }
    destroyed
}
