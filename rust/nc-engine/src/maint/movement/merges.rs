//! Join and Rendezvous in the year's ticks: a joiner merges into its host,
//! and the fleets meeting at a rendezvous into one, in the tick they come
//! together, as in classic. The year-end pass catches the rest.

use super::YearMovement;
use crate::maint::merging::{merge_join, merge_rendezvous, on_rendezvous_station};
use crate::{CoreGameData, Order};

impl YearMovement<'_> {
    /// Fleet `fleet_idx`'s merge after its arrival test this tick.
    pub(super) fn dispatch_merges(
        &mut self,
        game_data: &mut CoreGameData,
        fleet_idx: usize,
        order: Order,
        tick: u8,
    ) {
        let merges = match order {
            Order::JoinAnotherFleet => {
                merge_join(game_data, fleet_idx, &mut self.to_remove, Some(tick))
                    .into_iter()
                    .collect()
            }
            Order::RendezvousSector
                if on_rendezvous_station(game_data, fleet_idx, &self.to_remove) =>
            {
                let fleet = &game_data.fleets[fleet_idx];
                let (owner, coords) = (fleet.owner.0, fleet.sector.to_array());
                merge_rendezvous(game_data, owner, coords, &mut self.to_remove, Some(tick))
            }
            _ => Vec::new(),
        };
        for event in merges {
            if !event.survivor_side {
                self.merged_away.push(event.fleet_idx);
            }
            self.events.tick_merges.push((event, true));
        }
    }
}
