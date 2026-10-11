use super::*;
use crate::{Coords, Order, ProductionItemKind};

impl CoreGameData {
    /// Starbase Command "Move": what ECGAME writes on a confirmed move. The
    /// base sets out for `destination`; it enters orbit (target depth 1)
    /// only when the player owns a world there, and otherwise stops in deep
    /// space (target depth 0). A world whose builds rely on the base loses
    /// its build queue first (`cancel_builds_relying_on_base`).
    pub fn set_starbase_destination(
        &mut self,
        player_index_1_based: usize,
        base_record_index_1_based: usize,
        destination: [u8; 2],
    ) -> Result<(), GameStateMutationError> {
        let owner_empire = player_index_1_based as u8;
        let own_world_at_destination = self.planets.iter().any(|planet| {
            planet.sector.to_array() == destination && planet.owner.0 == owner_empire
        });
        self.owned_base_mut(player_index_1_based, base_record_index_1_based)?;
        self.cancel_builds_relying_on_base(base_record_index_1_based - 1);
        let base = self.owned_base_mut(player_index_1_based, base_record_index_1_based)?;
        base.moving = true;
        base.target = Coords::from_array(destination);
        base.target_depth = u8::from(own_world_at_destination);
        Ok(())
    }

    /// Starbase Command "Halt": what ECGAME writes when the player halts a
    /// moving base. It stops where it is, in the depth layer it is in.
    pub fn halt_starbase(
        &mut self,
        player_index_1_based: usize,
        base_record_index_1_based: usize,
    ) -> Result<(), GameStateMutationError> {
        let base = self.owned_base_mut(player_index_1_based, base_record_index_1_based)?;
        base.moving = false;
        base.target = base.sector;
        base.target_depth = base.depth.round() as u8;
        Ok(())
    }

    /// ECMAINT's `2895:30f4`, which it runs whenever it re-orders a Guard
    /// Starbase fleet: the fleet lets go of its base. Every base of the
    /// fleet's empire in the fleet's sector and layer whose escort word is
    /// the fleet's number loses it. A fleet on any other order releases
    /// nothing.
    pub fn release_starbase_escort(&mut self, fleet_idx: usize) {
        let fleet = &self.fleets[fleet_idx];
        if fleet.order != Order::GuardStarbase {
            return;
        }
        let (owner, sector, layer, number) = (
            fleet.owner,
            fleet.sector,
            fleet.depth.round() == 1,
            fleet.fleet_number,
        );
        for base in &mut self.bases {
            if base.owner == owner
                && base.sector == sector
                && (base.depth.round() == 1) == layer
                && base.escort == number
            {
                base.escort.0 = 0;
            }
        }
    }

    /// ECGAME warns that moving a base in orbit cancels the builds of the
    /// world beneath it when they rely on the base: a queued unit, or the
    /// whole queue, costs more than the world's production. Another base at
    /// rest in orbit there keeps them alive. On "yes" ECGAME clears the
    /// world's whole build queue, with no refund.
    fn cancel_builds_relying_on_base(&mut self, base_idx: usize) {
        let base = &self.bases[base_idx];
        if base.depth < Real48::ONE {
            return;
        }
        let (owner, coords) = (base.owner.0, base.sector.to_array());
        let Some(planet_idx) = self
            .planets
            .iter()
            .position(|planet| planet.sector.to_array() == coords && planet.owner.0 == owner)
        else {
            return;
        };
        let planet = &self.planets[planet_idx];
        let queued = planet
            .build_queue
            .iter()
            .filter(|slot| slot.points_owed > 0);
        let total: u32 = queued.clone().map(|slot| slot.points_owed).sum();
        let dearest = queued
            .filter_map(|slot| slot.kind.build_cost())
            .max()
            .unwrap_or(0);
        let production = planet.production.to_f64();
        if f64::from(dearest) <= production && f64::from(total) <= production {
            return;
        }
        let other_base_in_orbit = self.bases.iter().enumerate().any(|(idx, other)| {
            idx != base_idx
                && other.sector.to_array() == coords
                && other.depth >= Real48::HALF
                && !other.moving
                && other.target.to_array() == coords
                && other.target_depth >= 1
        });
        if other_base_in_orbit {
            return;
        }
        let planet = &mut self.planets[planet_idx];
        for slot in 0..10 {
            planet.build_queue[slot].points_owed = 0;
            planet.build_queue[slot].kind = ProductionItemKind::Unknown(0);
        }
    }

    fn owned_base_mut(
        &mut self,
        player_index_1_based: usize,
        base_record_index_1_based: usize,
    ) -> Result<&mut Starbase, GameStateMutationError> {
        let base = self.bases.get_mut(base_record_index_1_based - 1).ok_or(
            GameStateMutationError::MissingBaseRecord {
                index_1_based: base_record_index_1_based,
            },
        )?;
        if base.base_id.0 == 0 || base.owner.0 as usize != player_index_1_based {
            return Err(GameStateMutationError::BaseOwnershipMismatch {
                player_index_1_based,
                base_record_index_1_based,
            });
        }
        Ok(base)
    }
}
