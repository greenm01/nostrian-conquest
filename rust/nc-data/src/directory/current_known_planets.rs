use super::*;
use crate::{is_homeworld_seed_ignoring_name, stardock_slot_kind};

impl CoreGameData {
    pub fn player_owned_base_record_counts_current_known(&self) -> Vec<usize> {
        (1..=self.players.len())
            .map(|player_record_index_1_based| {
                self.player_owned_base_record_count_current_known(player_record_index_1_based)
            })
            .collect()
    }

    pub fn player_owned_planet_counts_current_known(&self) -> Vec<usize> {
        (1..=self.players.len())
            .map(|player_record_index_1_based| {
                self.player_owned_planet_count_current_known(player_record_index_1_based)
            })
            .collect()
    }

    pub fn player_homeworld_seed_coords_current_known(&self) -> Vec<Option<[u8; 2]>> {
        let player_count = self.conquest.player_count as usize;
        (1..=player_count)
            .map(|player_record_index_1_based| {
                self.planets
                    .iter()
                    .find(|record| {
                        record.owner.0 as usize == player_record_index_1_based
                            && is_homeworld_seed_ignoring_name(record)
                    })
                    .map(|record| record.sector.to_array())
            })
            .collect()
    }

    pub fn current_known_homeworld_seed_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let player_count = self.conquest.player_count as usize;
        for player_record_index_1_based in 1..=player_count {
            let matches = self
                .planets
                .iter()
                .enumerate()
                .filter(|(_, record)| {
                    record.owner.0 as usize == player_record_index_1_based
                        && is_homeworld_seed_ignoring_name(record)
                })
                .map(|(idx, record)| (idx + 1, record.sector.to_array()))
                .collect::<Vec<_>>();
            if matches.len() != 1 {
                errors.push(format!(
                    "PLAYER[{}] homeworld seed expected 1 owned 'Not Named Yet' planet, got {}",
                    player_record_index_1_based,
                    matches.len()
                ));
            }
        }
        errors
    }

    pub fn current_known_initialized_planet_ownership_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let player_count = self.conquest.player_count as usize;

        for (idx, record) in self.planets.iter().enumerate() {
            let planet_index_1_based = idx + 1;
            let owner = record.owner.0 as usize;
            let is_homeworld_seed = is_homeworld_seed_ignoring_name(record);

            if owner != 0 && !is_homeworld_seed {
                errors.push(format!(
                    "PLANET[{}] expected unowned non-homeworld baseline, got owner {}",
                    planet_index_1_based, owner
                ));
            }

            if is_homeworld_seed {
                if owner == 0 || owner > player_count {
                    errors.push(format!(
                        "PLANET[{}] homeworld seed expected owner 1..={}, got {}",
                        planet_index_1_based, player_count, owner
                    ));
                }
                if owner != 0 && record.years_under_owner != 2 {
                    errors.push(format!(
                        "PLANET[{}].ownership_status expected 2 for owned homeworld seed, got {}",
                        planet_index_1_based, record.years_under_owner
                    ));
                }
            }
        }

        for player_record_index_1_based in 1..=player_count {
            let owned_count =
                self.player_owned_planet_count_current_known(player_record_index_1_based);
            if owned_count != 1 {
                errors.push(format!(
                    "PLAYER[{}] owned_planet_count expected 1, got {}",
                    player_record_index_1_based, owned_count
                ));
            }
        }

        errors
    }

    pub fn current_known_homeworld_seed_payload_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        for (idx, record) in self.planets.iter().enumerate() {
            if !is_homeworld_seed_ignoring_name(record) {
                continue;
            }
            let planet_index_1_based = idx + 1;
            if u16::from(record.potential_production) != 100 {
                errors.push(format!(
                    "PLANET[{}].potential_production expected 100 for homeworld seed, got {}",
                    planet_index_1_based,
                    u16::from(record.potential_production)
                ));
            }
            if record.production.to_f64() != 100.0 {
                errors.push(format!(
                    "PLANET[{}].present_production expected 100 for homeworld seed, got {}",
                    planet_index_1_based,
                    record.production.to_f64()
                ));
            }
            if record.years_under_owner != 2 {
                errors.push(format!(
                    "PLANET[{}].ownership_status expected 2 for homeworld seed, got {}",
                    planet_index_1_based, record.years_under_owner
                ));
            }
            if record.armies != 10 {
                errors.push(format!(
                    "PLANET[{}].army_count_raw expected 10 for homeworld seed, got {}",
                    planet_index_1_based,
                    u8::try_from(record.armies).unwrap_or(u8::MAX)
                ));
            }
            if record.ground_batteries != 4 {
                errors.push(format!(
                    "PLANET[{}].ground_batteries_raw expected 4 for homeworld seed, got {}",
                    planet_index_1_based,
                    u8::try_from(record.ground_batteries).unwrap_or(u8::MAX)
                ));
            }
            if record.stored_goods.to_f64() != 35.0 {
                errors.push(format!(
                    "PLANET[{}].stored_goods expected 35 for homeworld seed, got {}",
                    planet_index_1_based,
                    record.stored_goods.to_f64()
                ));
            }
            if (0..10).any(|slot| {
                record.build_queue[slot].points_owed != 0
                    || record.build_queue[slot].kind.to_raw() != 0
            }) {
                errors.push(format!(
                    "PLANET[{}] build queue expected all zeroes for homeworld seed",
                    planet_index_1_based
                ));
            }
            if (0..crate::STARDOCK_SLOT_COUNT).any(|slot| {
                stardock_slot_kind(record, slot).to_raw() != 0 || record.stardock[slot] != 0
            }) {
                errors.push(format!(
                    "PLANET[{}] stardock expected all zeroes for homeworld seed",
                    planet_index_1_based
                ));
            }
        }

        errors
    }

    pub fn current_known_unowned_planet_payload_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        for (idx, record) in self.planets.iter().enumerate() {
            if is_homeworld_seed_ignoring_name(record) {
                continue;
            }
            let planet_index_1_based = idx + 1;
            if record.name.trim() != "Unowned" {
                errors.push(format!(
                    "PLANET[{}].status_or_name expected 'Unowned', got {:?}",
                    planet_index_1_based,
                    record.name.trim().to_string()
                ));
            }
            if record.owner.0 != 0 {
                errors.push(format!(
                    "PLANET[{}].owner_empire_slot expected 0 for unowned baseline, got {}",
                    planet_index_1_based, record.owner.0
                ));
            }
            if record.years_under_owner != 0 {
                errors.push(format!(
                    "PLANET[{}].ownership_status expected 0 for unowned baseline, got {}",
                    planet_index_1_based, record.years_under_owner
                ));
            }
            if record.armies != 0 {
                errors.push(format!(
                    "PLANET[{}].army_count_raw expected 0 for unowned baseline, got {}",
                    planet_index_1_based,
                    u8::try_from(record.armies).unwrap_or(u8::MAX)
                ));
            }
            if record.ground_batteries != 0 {
                errors.push(format!(
                    "PLANET[{}].ground_batteries_raw expected 0 for unowned baseline, got {}",
                    planet_index_1_based,
                    u8::try_from(record.ground_batteries).unwrap_or(u8::MAX)
                ));
            }
            if record.production.to_f64() != 0.0 {
                errors.push(format!(
                    "PLANET[{}].present_production expected 0 for unowned baseline, got {}",
                    planet_index_1_based,
                    record.production.to_f64()
                ));
            }
            if record.stored_goods.to_f64() != 0.0 {
                errors.push(format!(
                    "PLANET[{}].stored_goods expected 0 for unowned baseline, got {}",
                    planet_index_1_based,
                    record.stored_goods.to_f64()
                ));
            }
            if (0..10).any(|slot| {
                record.build_queue[slot].points_owed != 0
                    || record.build_queue[slot].kind.to_raw() != 0
            }) {
                errors.push(format!(
                    "PLANET[{}] build queue expected all zeroes for unowned baseline",
                    planet_index_1_based
                ));
            }
            if (0..crate::STARDOCK_SLOT_COUNT).any(|slot| {
                stardock_slot_kind(record, slot).to_raw() != 0 || record.stardock[slot] != 0
            }) {
                errors.push(format!(
                    "PLANET[{}] stardock expected all zeroes for unowned baseline",
                    planet_index_1_based
                ));
            }
        }

        errors
    }
}
