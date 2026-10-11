use super::*;
use crate::{
    PlayerMode, Real48, diplomatic_relation_toward, is_classic_text_char,
    is_homeworld_seed_ignoring_name, present_production_points, set_diplomatic_relation_toward,
    set_empire_name, set_planet_name, stored_diplomatic_relation,
};

const DEFAULT_JOIN_TAX_RATE: u8 = 50;

impl CoreGameData {
    pub fn player_slot_is_open_for_first_join(&self, player_record_index_1_based: usize) -> bool {
        let Some(player) = self.players.get(player_record_index_1_based - 1) else {
            return false;
        };
        if player.mode != PlayerMode::CivilDisorder || player.last_year_on != 0 {
            return false;
        }
        self.planets.iter().any(|planet| {
            planet.owner.0 as usize == player_record_index_1_based
                && is_homeworld_seed_ignoring_name(planet)
        })
    }

    pub fn has_open_first_join_slot(&self) -> bool {
        (1..=self.players.len()).any(|idx| self.player_slot_is_open_for_first_join(idx))
    }

    pub fn first_open_first_join_slot(&self) -> Option<usize> {
        (1..=self.players.len()).find(|&idx| self.player_slot_is_open_for_first_join(idx))
    }

    pub fn join_player(
        &mut self,
        player_record_index_1_based: usize,
        empire_name: &str,
    ) -> Result<(), GameStateMutationError> {
        let (ipbm_count, homeworld_planet_index_1_based) = {
            let player = self.players.get(player_record_index_1_based - 1).ok_or(
                GameStateMutationError::MissingPlayerRecord {
                    index_1_based: player_record_index_1_based,
                },
            )?;
            let homeworld_planet_index_1_based = if player.homeworld as usize != 0 {
                player.homeworld as usize
            } else {
                self.planets
                    .iter()
                    .position(|planet| {
                        planet.owner.0 as usize == player_record_index_1_based
                            && is_homeworld_seed_ignoring_name(planet)
                    })
                    .map(|idx| idx + 1)
                    .unwrap_or(0)
            };
            (player.ipbms_first.0, homeworld_planet_index_1_based)
        };
        let tax_rate = DEFAULT_JOIN_TAX_RATE;
        let already_joined = self
            .players
            .get(player_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlayerRecord {
                index_1_based: player_record_index_1_based,
            })?
            .mode
            == PlayerMode::Human;
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
            .players
            .get_mut(player_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlayerRecord {
                index_1_based: player_record_index_1_based,
            })?;
        player.mode = PlayerMode::Human;
        set_empire_name(player, empire_name);
        player.tax_rate = tax_rate;
        player.ipbms_first.0 = ipbm_count;
        player.autopilot = false;
        player.last_year_on = self.conquest.game_year;
        if homeworld_planet_index_1_based != 0 {
            player.homeworld = homeworld_planet_index_1_based as u8;
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
        if let Some(planet) = self.planets.get_mut(homeworld_planet_index_1_based - 1) {
            let revenue = crate::yearly_tax_revenue(present_production_points(planet), tax_rate);
            planet.stored_goods = Real48::from_f64(f64::from(revenue));
        }
    }

    pub fn rename_player_homeworld(
        &mut self,
        player_record_index_1_based: usize,
        homeworld_name: &str,
    ) -> Result<usize, GameStateMutationError> {
        let raw_homeworld_planet_index_1_based = self
            .players
            .get(player_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlayerRecord {
                index_1_based: player_record_index_1_based,
            })?
            .homeworld as usize;
        let homeworld_planet_index_1_based = if raw_homeworld_planet_index_1_based != 0 {
            raw_homeworld_planet_index_1_based
        } else {
            self.planets
                .iter()
                .position(|planet| {
                    planet.owner.0 as usize == player_record_index_1_based
                        && is_homeworld_seed_ignoring_name(planet)
                })
                .map(|idx| idx + 1)
                .ok_or(GameStateMutationError::MissingPlanetRecord { index_1_based: 0 })?
        };
        let planet = self
            .planets
            .get_mut(homeworld_planet_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlanetRecord {
                index_1_based: homeworld_planet_index_1_based,
            })?;
        if !homeworld_name.chars().all(is_classic_text_char) {
            return Err(GameStateMutationError::InvalidPlanetName {
                planet_index_1_based: homeworld_planet_index_1_based,
            });
        }
        set_planet_name(planet, homeworld_name);
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
            .get_mut(planet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlanetRecord {
                index_1_based: planet_record_index_1_based,
            })?;
        if planet.owner.0 != owner_empire {
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
        set_planet_name(planet, planet_name);
        Ok(())
    }
    pub fn validate_player_diplomacy_inputs(
        &self,
        player_index_1_based: usize,
    ) -> Vec<PlayerDiplomacyValidationError> {
        let mut errors = Vec::new();
        let Some(player) = self.players.get(player_index_1_based - 1) else {
            return errors;
        };
        let empire = player_index_1_based as u8;
        let player_count = self.players.len() as u8;
        for target_empire in 1..=player_count {
            let raw = stored_diplomatic_relation(player, target_empire)
                .map_or(0, DiplomaticRelation::code);
            if target_empire == empire {
                if raw != 0 {
                    errors.push(PlayerDiplomacyValidationError::SelfTarget { empire });
                }
                continue;
            }
            if raw != 0x00 && raw != 0x01 {
                errors.push(PlayerDiplomacyValidationError::InvalidStoredRelationByte {
                    target_empire,
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
            .players
            .get_mut(player_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlayerRecord {
                index_1_based: player_record_index_1_based,
            })?;
        record.tax_rate = tax_rate;
        Ok(())
    }

    pub fn stored_diplomatic_relation(
        &self,
        from_empire: u8,
        to_empire: u8,
    ) -> Option<DiplomaticRelation> {
        if from_empire == 0 || to_empire == 0 || from_empire == to_empire {
            return None;
        }
        self.players
            .get(from_empire.saturating_sub(1) as usize)
            .and_then(|record| diplomatic_relation_toward(record, to_empire))
    }

    pub fn set_stored_diplomatic_relation(
        &mut self,
        from_empire: u8,
        to_empire: u8,
        relation: DiplomaticRelation,
    ) -> Result<bool, GameStateMutationError> {
        let player_count = self.players.len() as u8;
        if to_empire == 0 || to_empire > player_count {
            return Err(GameStateMutationError::InvalidDiplomacyInput {
                player_index_1_based: from_empire as usize,
                reason: PlayerDiplomacyValidationError::TargetOutOfRange {
                    target_empire: to_empire,
                },
            });
        }
        if from_empire == 0 || from_empire > player_count {
            return Err(GameStateMutationError::MissingPlayerRecord {
                index_1_based: from_empire as usize,
            });
        }
        if from_empire == to_empire {
            return Err(GameStateMutationError::InvalidDiplomacyInput {
                player_index_1_based: from_empire as usize,
                reason: PlayerDiplomacyValidationError::SelfTarget {
                    empire: from_empire,
                },
            });
        }
        let Some(record) = self.players.get_mut(from_empire.saturating_sub(1) as usize) else {
            return Err(GameStateMutationError::MissingPlayerRecord {
                index_1_based: from_empire as usize,
            });
        };
        Ok(set_diplomatic_relation_toward(record, to_empire, relation))
    }
}
