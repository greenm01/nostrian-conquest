use super::*;

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
            planet.coords_raw() == destination && planet.owner_empire_slot_raw() == owner_empire
        });
        self.owned_base_mut(player_index_1_based, base_record_index_1_based)?;
        self.cancel_builds_relying_on_base(base_record_index_1_based - 1);
        let base = self.owned_base_mut(player_index_1_based, base_record_index_1_based)?;
        base.set_moving_flag_raw(1);
        base.set_target_coords_raw(destination);
        base.set_target_depth_raw(u8::from(own_world_at_destination));
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
        base.set_moving_flag_raw(0);
        base.set_target_coords_raw(base.coords_raw());
        base.set_target_depth_raw(base.depth().round() as u8);
        Ok(())
    }

    /// ECGAME warns that moving a base in orbit cancels the builds of the
    /// world beneath it when they rely on the base: a queued unit, or the
    /// whole queue, costs more than the world's production. Another base at
    /// rest in orbit there keeps them alive. On "yes" ECGAME clears the
    /// world's whole build queue, with no refund.
    fn cancel_builds_relying_on_base(&mut self, base_idx: usize) {
        let base = &self.bases[base_idx];
        if base.depth() < Real48::ONE {
            return;
        }
        let (owner, coords) = (base.owner_empire_raw(), base.coords_raw());
        let Some(planet_idx) = self.planets.iter().position(|planet| {
            planet.coords_raw() == coords && planet.owner_empire_slot_raw() == owner
        }) else {
            return;
        };
        let planet = &self.planets[planet_idx];
        let costs = (0..10).filter_map(|slot| {
            let count = u32::from(planet.build_count_raw(slot));
            let cost = ProductionItemKind::from_raw(planet.build_kind_raw(slot)).build_cost()?;
            (count > 0).then_some((count, cost))
        });
        let total: u32 = costs.clone().map(|(count, cost)| count * cost).sum();
        let dearest = costs.map(|(_, cost)| cost).max().unwrap_or(0);
        let production = planet.present_production();
        if f64::from(dearest) <= production && f64::from(total) <= production {
            return;
        }
        let other_base_in_orbit = self.bases.iter().enumerate().any(|(idx, other)| {
            idx != base_idx
                && other.coords_raw() == coords
                && other.depth() >= Real48::HALF
                && other.moving_flag_raw() == 0
                && other.target_coords_raw() == coords
                && other.target_depth_raw() >= 1
        });
        if other_base_in_orbit {
            return;
        }
        let planet = &mut self.planets[planet_idx];
        for slot in 0..10 {
            planet.set_build_count_raw(slot, 0);
            planet.set_build_kind_raw(slot, 0);
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
        if base.base_id_raw() == 0 || base.owner_empire_raw() as usize != player_index_1_based {
            return Err(GameStateMutationError::BaseOwnershipMismatch {
                player_index_1_based,
                base_record_index_1_based,
            });
        }
        Ok(base)
    }
}
