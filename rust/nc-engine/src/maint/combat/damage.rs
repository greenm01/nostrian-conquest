//! Sharing a side's losses among its fleets, and writing a battle's result
//! back to the records. A side fires and is fired on as one aggregate; hit
//! allocation chooses the class, the damage state, and the recovery cohort
//! it strikes, and within that choice the losses are shared across the
//! side's fleets in proportion to their eligible hulls by largest remainder
//! (combat decision 6). Ties at the cutoff are drawn from the battle's
//! seeded roll stream, without replacement, only when a tied group straddles
//! the remaining losses; a completed allocation is never redrawn. Transport
//! losses fall on loaded transports first across the side, so moving cargo
//! into another fleet does not shelter it. Nothing is restored afterwards:
//! crippled hulls stay crippled until the recovery step.

use std::cmp::Reverse;
use std::collections::BTreeMap;

use nc_data::{
    CoreGameData, CrippledHulls, GameRng, RECOVERY_TURNS, has_any_force, recompute_max_speed,
    troop_transports,
};

use super::state::{FleetCombatState, IDX_SB, IDX_TT, write_fleet_state};

/// One record's hulls of a class: how many it holds, how many of them are
/// crippled, and how many are loaded transports (0 for any other class).
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Holder {
    pub(super) hulls: u32,
    pub(super) crippled: CrippledHulls,
    pub(super) loaded: u32,
}

/// The cohorts of a class in the order finishing fire takes them: this
/// pass's cripples, then the most turns left down to one, then the nominal
/// hulls. A fleet's loaded transports are deemed to be the first hulls in
/// this order, which is how the record's loaded-first losses read.
const NOMINAL: usize = RECOVERY_TURNS + 1;

impl Holder {
    fn nominal(&self) -> u32 {
        self.hulls - self.crippled.total()
    }

    fn stage(&self, stage: usize) -> u32 {
        match stage {
            0 => self.crippled.fresh,
            NOMINAL => self.nominal(),
            s => self.crippled.turns[RECOVERY_TURNS - s],
        }
    }

    fn finish(&mut self, stage: usize, count: u32) {
        match stage {
            0 => self.crippled.fresh -= count,
            s => self.crippled.turns[RECOVERY_TURNS - s] -= count,
        }
        self.lose(count);
    }

    fn lose(&mut self, count: u32) {
        self.hulls -= count;
        self.loaded -= count.min(self.loaded);
    }

    /// The holder's loaded transports in `stage`: its loaded hulls fill the
    /// stages in finishing order.
    fn loaded_in(&self, stage: usize) -> u32 {
        let earlier: u32 = (0..stage).map(|s| self.stage(s)).sum();
        self.loaded.saturating_sub(earlier).min(self.stage(stage))
    }
}

/// `loss` hulls shared among holders of `counts` eligible hulls by largest
/// remainder. Ties at the cutoff are drawn from `rng` without replacement,
/// and only when the tied group straddles the hulls still to allocate.
pub(super) fn apportion(counts: &[u32], loss: u32, rng: &mut GameRng) -> Vec<u32> {
    let total: u64 = counts.iter().map(|&n| u64::from(n)).sum();
    if total == 0 {
        return vec![0; counts.len()];
    }
    let exact = |n: u32| u64::from(loss) * u64::from(n);
    let mut shares: Vec<u32> = counts.iter().map(|&n| (exact(n) / total) as u32).collect();
    let mut left = loss - shares.iter().sum::<u32>();
    let mut tied: BTreeMap<Reverse<u64>, Vec<usize>> = BTreeMap::new();
    for (i, &n) in counts.iter().enumerate() {
        let remainder = exact(n) % total;
        if remainder > 0 {
            tied.entry(Reverse(remainder)).or_default().push(i);
        }
    }
    for (_, mut group) in tied {
        if left == 0 {
            break;
        }
        let chosen = (group.len() as u32).min(left);
        for k in 0..chosen as usize {
            if group.len() as u32 > left {
                let j = k + rng.below((group.len() - k) as u64) as usize;
                group.swap(k, j);
            }
            shares[group[k]] += 1;
        }
        left -= chosen;
    }
    shares
}

/// `loss` hulls shared loaded-first: among the holders' loaded hulls while
/// any remain, then among their empty ones.
fn apportion_cargo(loaded: &[u32], empty: &[u32], loss: u32, rng: &mut GameRng) -> Vec<u32> {
    let loaded_loss = loss.min(loaded.iter().sum());
    let on_loaded = apportion(loaded, loaded_loss, rng);
    let on_empty = apportion(empty, loss - loaded_loss, rng);
    on_loaded
        .into_iter()
        .zip(on_empty)
        .map(|(l, e)| l + e)
        .collect()
}

fn shares(holders: &[Holder], stage: usize, loss: u32, rng: &mut GameRng) -> Vec<u32> {
    let loaded: Vec<u32> = holders.iter().map(|h| h.loaded_in(stage)).collect();
    let empty: Vec<u32> = holders
        .iter()
        .zip(&loaded)
        .map(|(h, l)| h.stage(stage) - l)
        .collect();
    apportion_cargo(&loaded, &empty, loss, rng)
}

/// Spread a side's change in one class, `before` to `after` (hulls and
/// crippled hulls), over `holders`: each cohort's finished hulls among the
/// holders of that cohort, the nominal hulls destroyed among the holders of
/// nominal hulls, and the hulls newly crippled among those left nominal,
/// each in proportion to the holders' eligible hulls.
pub(super) fn apply_class_change(
    holders: &mut [Holder],
    before: (u32, &CrippledHulls),
    after: (u32, &CrippledHulls),
    rng: &mut GameRng,
) {
    // Finishing is the only way a recovering cohort shrinks. This pass's
    // cohort also gains the hulls crippled in the battle, so a hull
    // crippled and finished within it shows as a nominal hull destroyed.
    let finished = |stage: usize| match stage {
        0 => before.1.fresh.saturating_sub(after.1.fresh),
        s => before.1.turns[RECOVERY_TURNS - s] - after.1.turns[RECOVERY_TURNS - s],
    };
    let mut removed = 0;
    for stage in 0..NOMINAL {
        let removal = finished(stage);
        if removal == 0 {
            continue;
        }
        removed += removal;
        let shares = shares(holders, stage, removal, rng);
        for (holder, share) in holders.iter_mut().zip(shares) {
            holder.finish(stage, share);
        }
    }
    let nominal_removed = before.0 - after.0 - removed;
    let shares_destroyed = shares(holders, NOMINAL, nominal_removed, rng);
    for (holder, share) in holders.iter_mut().zip(shares_destroyed) {
        holder.lose(share);
    }
    let newly_crippled = after.1.fresh.saturating_sub(before.1.fresh);
    let shares_crippled = shares(holders, NOMINAL, newly_crippled, rng);
    for (holder, share) in holders.iter_mut().zip(shares_crippled) {
        holder.crippled.cripple(share);
    }
}

/// Put a fleet's ships as a battle or strike left them on its record. A
/// survivor moves at the speed of the ships it has left (classic recomputes
/// it after the battle too, `24a3:036b`); a fleet with none left is culled,
/// not re-rated.
pub(super) fn write_back_fleet(
    game_data: &mut CoreGameData,
    fleet_idx: usize,
    state: &FleetCombatState,
    loaded: u32,
) {
    let lost = write_fleet_state(game_data, fleet_idx, state, loaded);
    let fleet = &mut game_data.fleets[fleet_idx];
    if lost && has_any_force(fleet) {
        recompute_max_speed(fleet);
    }
}

/// Put a strike's result on the striking fleet: its ships as `after` has
/// them, transport losses on loaded transports first, each taking its army
/// with it.
pub(super) fn write_back_strike(
    game_data: &mut CoreGameData,
    fleet_idx: usize,
    after: &FleetCombatState,
) {
    let fleet = &game_data.fleets[fleet_idx];
    let lost = u32::from(troop_transports(fleet)).saturating_sub(after.counts[IDX_TT]);
    let loaded = u32::from(fleet.ships.loaded_transports).saturating_sub(lost);
    write_back_fleet(game_data, fleet_idx, after, loaded);
}

/// Put the result of a battle on a starbase record: a base destroyed is
/// unlinked and zeroed, one that survives keeps or gains its crippled hull.
/// Returns the id of a base destroyed.
pub(super) fn write_back_base(
    game_data: &mut CoreGameData,
    base_idx: usize,
    state: &FleetCombatState,
) -> Option<u16> {
    if state.counts[IDX_SB] == 0 {
        let destroyed = game_data.bases[base_idx].base_id.0;
        game_data.unlink_base(base_idx);
        game_data.bases[base_idx] = nc_data::Starbase::default();
        Some(destroyed)
    } else {
        game_data.bases[base_idx].crippled = state.crippled[IDX_SB];
        None
    }
}
