use super::*;

impl CoreGameData {
    /// Starbase Command "Move": what ECGAME writes on a confirmed move. The
    /// base sets out for `destination`; it enters orbit (target depth 1)
    /// only when the player owns a world there, and otherwise stops in deep
    /// space (target depth 0).
    pub fn set_starbase_destination(
        &mut self,
        player_index_1_based: usize,
        base_record_index_1_based: usize,
        destination: [u8; 2],
    ) -> Result<(), GameStateMutationError> {
        let owner_empire = player_index_1_based as u8;
        let own_world_at_destination = self.planets.records.iter().any(|planet| {
            planet.coords_raw() == destination && planet.owner_empire_slot_raw() == owner_empire
        });
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

    fn owned_base_mut(
        &mut self,
        player_index_1_based: usize,
        base_record_index_1_based: usize,
    ) -> Result<&mut BaseRecord, GameStateMutationError> {
        let base = self
            .bases
            .records
            .get_mut(base_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingBaseRecord {
                index_1_based: base_record_index_1_based,
            })?;
        if base.base_id_raw() == 0 || base.owner_empire_raw() as usize != player_index_1_based {
            return Err(GameStateMutationError::BaseOwnershipMismatch {
                player_index_1_based,
                base_record_index_1_based,
            });
        }
        Ok(base)
    }
}
