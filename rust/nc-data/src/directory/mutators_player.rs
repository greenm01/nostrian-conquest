use super::*;
use crate::is_classic_text_char;

const DEFAULT_JOIN_TAX_RATE: u8 = 50;

impl CoreGameData {
    pub fn player_slot_is_open_for_first_join(&self, player_record_index_1_based: usize) -> bool {
        let Some(player) = self.player.records.get(player_record_index_1_based - 1) else {
            return false;
        };
        if !player.is_civil_disorder_player() || player.last_run_year_raw() != 0 {
            return false;
        }
        self.planets.records.iter().any(|planet| {
            planet.owner_empire_slot_raw() as usize == player_record_index_1_based
                && planet.is_homeworld_seed_ignoring_name()
        })
    }

    pub fn has_open_first_join_slot(&self) -> bool {
        (1..=self.player.records.len()).any(|idx| self.player_slot_is_open_for_first_join(idx))
    }

    pub fn first_open_first_join_slot(&self) -> Option<usize> {
        (1..=self.player.records.len()).find(|&idx| self.player_slot_is_open_for_first_join(idx))
    }

    pub fn join_player(
        &mut self,
        player_record_index_1_based: usize,
        empire_name: &str,
    ) -> Result<(), GameStateMutationError> {
        let (ipbm_count, homeworld_planet_index_1_based) = {
            let player = self
                .player
                .records
                .get(player_record_index_1_based - 1)
                .ok_or(GameStateMutationError::MissingPlayerRecord {
                    index_1_based: player_record_index_1_based,
                })?;
            let homeworld_planet_index_1_based =
                if player.homeworld_planet_index_1_based_raw() as usize != 0 {
                    player.homeworld_planet_index_1_based_raw() as usize
                } else {
                    self.planets
                        .records
                        .iter()
                        .position(|planet| {
                            planet.owner_empire_slot_raw() as usize == player_record_index_1_based
                                && planet.is_homeworld_seed_ignoring_name()
                        })
                        .map(|idx| idx + 1)
                        .unwrap_or(0)
                };
            (player.ipbm_chain_head_raw(), homeworld_planet_index_1_based)
        };
        let tax_rate = DEFAULT_JOIN_TAX_RATE;
        let already_joined = self
            .player
            .records
            .get(player_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlayerRecord {
                index_1_based: player_record_index_1_based,
            })?
            .is_active_human_player();
        if already_joined {
            return Err(GameStateMutationError::PlayerAlreadyJoined {
                index_1_based: player_record_index_1_based,
            });
        }
        if !self.player_slot_is_open_for_first_join(player_record_index_1_based) {
            return Err(GameStateMutationError::PlayerSlotNotJoinable {
                index_1_based: player_record_index_1_based,
            });
        }
        if !empire_name.chars().all(is_classic_text_char) {
            return Err(GameStateMutationError::InvalidEmpireName {
                player_index_1_based: player_record_index_1_based,
            });
        }
        let player = self
            .player
            .records
            .get_mut(player_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlayerRecord {
                index_1_based: player_record_index_1_based,
            })?;
        player.set_player_mode_raw(0x01);
        player.set_controlled_empire_name_raw(empire_name);
        player.set_tax_rate_raw(tax_rate);
        player.set_ipbm_chain_head_raw(ipbm_count);
        player.set_autopilot_flag(0);
        player.set_last_run_year_raw(self.conquest.game_year());
        if homeworld_planet_index_1_based != 0 {
            player.set_homeworld_planet_index_1_based_raw(homeworld_planet_index_1_based as u8);
            self.seed_first_turn_treasury(homeworld_planet_index_1_based, tax_rate);
        }
        Ok(())
    }

    /// A new empire starts with one year's tax revenue from its homeworld.
    pub(crate) fn seed_first_turn_treasury(
        &mut self,
        homeworld_planet_index_1_based: usize,
        tax_rate: u8,
    ) {
        if let Some(planet) = self
            .planets
            .records
            .get_mut(homeworld_planet_index_1_based - 1)
        {
            let revenue = crate::yearly_tax_revenue(
                planet.present_production_points().unwrap_or(0),
                tax_rate,
            );
            planet.set_stored_production_points(revenue);
        }
    }

    pub fn rename_player_homeworld(
        &mut self,
        player_record_index_1_based: usize,
        homeworld_name: &str,
    ) -> Result<usize, GameStateMutationError> {
        let raw_homeworld_planet_index_1_based =
            self.player
                .records
                .get(player_record_index_1_based - 1)
                .ok_or(GameStateMutationError::MissingPlayerRecord {
                    index_1_based: player_record_index_1_based,
                })?
                .homeworld_planet_index_1_based_raw() as usize;
        let homeworld_planet_index_1_based = if raw_homeworld_planet_index_1_based != 0 {
            raw_homeworld_planet_index_1_based
        } else {
            self.planets
                .records
                .iter()
                .position(|planet| {
                    planet.owner_empire_slot_raw() as usize == player_record_index_1_based
                        && planet.is_homeworld_seed_ignoring_name()
                })
                .map(|idx| idx + 1)
                .ok_or(GameStateMutationError::MissingPlanetRecord { index_1_based: 0 })?
        };
        let planet = self
            .planets
            .records
            .get_mut(homeworld_planet_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlanetRecord {
                index_1_based: homeworld_planet_index_1_based,
            })?;
        if !homeworld_name.chars().all(is_classic_text_char) {
            return Err(GameStateMutationError::InvalidPlanetName {
                planet_index_1_based: homeworld_planet_index_1_based,
            });
        }
        planet.set_planet_name(homeworld_name);
        Ok(homeworld_planet_index_1_based)
    }

    pub fn rename_owned_planet(
        &mut self,
        player_record_index_1_based: usize,
        planet_record_index_1_based: usize,
        planet_name: &str,
    ) -> Result<(), GameStateMutationError> {
        let owner_empire = player_record_index_1_based as u8;
        let planet = self
            .planets
            .records
            .get_mut(planet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlanetRecord {
                index_1_based: planet_record_index_1_based,
            })?;
        if planet.owner_empire_slot_raw() != owner_empire {
            return Err(GameStateMutationError::PlanetOwnershipMismatch {
                player_index_1_based: player_record_index_1_based,
                planet_index_1_based: planet_record_index_1_based,
            });
        }
        if !planet_name.chars().all(is_classic_text_char) {
            return Err(GameStateMutationError::InvalidPlanetName {
                planet_index_1_based: planet_record_index_1_based,
            });
        }
        planet.set_planet_name(planet_name);
        Ok(())
    }
    pub fn validate_player_diplomacy_inputs(
        &self,
        player_index_1_based: usize,
    ) -> Vec<PlayerDiplomacyValidationError> {
        let mut errors = Vec::new();
        let Some(player) = self.player.records.get(player_index_1_based - 1) else {
            return errors;
        };
        let empire_raw = player_index_1_based as u8;
        let player_count = self.player.records.len() as u8;
        for target_empire_raw in 1..=player_count {
            let raw = player
                .diplomatic_relation_byte_raw(target_empire_raw)
                .unwrap_or(0);
            if target_empire_raw == empire_raw {
                if raw != 0 {
                    errors.push(PlayerDiplomacyValidationError::SelfTarget { empire_raw });
                }
                continue;
            }
            if raw != 0x00 && raw != 0x01 {
                errors.push(PlayerDiplomacyValidationError::InvalidStoredRelationByte {
                    target_empire_raw,
                    raw,
                });
            }
        }
        errors
    }
    pub fn set_player_tax_rate(
        &mut self,
        player_record_index_1_based: usize,
        tax_rate: u8,
    ) -> Result<(), GameStateMutationError> {
        if tax_rate > 100 {
            return Err(GameStateMutationError::InvalidPlayerTaxRate {
                player_index_1_based: player_record_index_1_based,
                tax_rate,
            });
        }
        let record = self
            .player
            .records
            .get_mut(player_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlayerRecord {
                index_1_based: player_record_index_1_based,
            })?;
        record.set_tax_rate_raw(tax_rate);
        Ok(())
    }

    pub fn stored_diplomatic_relation(
        &self,
        from_empire_raw: u8,
        to_empire_raw: u8,
    ) -> Option<DiplomaticRelation> {
        if from_empire_raw == 0 || to_empire_raw == 0 || from_empire_raw == to_empire_raw {
            return None;
        }
        self.player
            .records
            .get(from_empire_raw.saturating_sub(1) as usize)
            .and_then(|record| record.diplomatic_relation_toward(to_empire_raw))
    }

    pub fn set_stored_diplomatic_relation(
        &mut self,
        from_empire_raw: u8,
        to_empire_raw: u8,
        relation: DiplomaticRelation,
    ) -> Result<bool, GameStateMutationError> {
        let player_count = self.player.records.len() as u8;
        if to_empire_raw == 0 || to_empire_raw > player_count {
            return Err(GameStateMutationError::InvalidDiplomacyInput {
                player_index_1_based: from_empire_raw as usize,
                reason: PlayerDiplomacyValidationError::TargetOutOfRange {
                    target_empire_raw: to_empire_raw,
                },
            });
        }
        if from_empire_raw == 0 || from_empire_raw > player_count {
            return Err(GameStateMutationError::MissingPlayerRecord {
                index_1_based: from_empire_raw as usize,
            });
        }
        if from_empire_raw == to_empire_raw {
            return Err(GameStateMutationError::InvalidDiplomacyInput {
                player_index_1_based: from_empire_raw as usize,
                reason: PlayerDiplomacyValidationError::SelfTarget {
                    empire_raw: from_empire_raw,
                },
            });
        }
        let Some(record) = self
            .player
            .records
            .get_mut(from_empire_raw.saturating_sub(1) as usize)
        else {
            return Err(GameStateMutationError::MissingPlayerRecord {
                index_1_based: from_empire_raw as usize,
            });
        };
        Ok(record.set_diplomatic_relation_toward(to_empire_raw, relation))
    }
}
