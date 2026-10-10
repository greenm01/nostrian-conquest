use super::*;

impl CoreGameData {
    pub fn player_owned_base_record_counts_current_known(&self) -> Vec<usize> {
        (1..=self.player.records.len())
            .map(|player_record_index_1_based| {
                self.player_owned_base_record_count_current_known(player_record_index_1_based)
            })
            .collect()
    }

    pub fn player_owned_planet_counts_current_known(&self) -> Vec<usize> {
        (1..=self.player.records.len())
            .map(|player_record_index_1_based| {
                self.player_owned_planet_count_current_known(player_record_index_1_based)
            })
            .collect()
    }

    pub fn player_homeworld_seed_coords_current_known(&self) -> Vec<Option<[u8; 2]>> {
        let player_count = self.conquest.player_count() as usize;
        (1..=player_count)
            .map(|player_record_index_1_based| {
                self.planets
                    .records
                    .iter()
                    .find(|record| {
                        record.owner_empire_slot_raw() as usize == player_record_index_1_based
                            && record.is_homeworld_seed_ignoring_name()
                    })
                    .map(|record| record.coords_raw())
            })
            .collect()
    }

    pub fn current_known_homeworld_seed_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let player_count = self.conquest.player_count() as usize;
        for player_record_index_1_based in 1..=player_count {
            let matches = self
                .planets
                .records
                .iter()
                .enumerate()
                .filter(|(_, record)| {
                    record.owner_empire_slot_raw() as usize == player_record_index_1_based
                        && record.is_homeworld_seed_ignoring_name()
                })
                .map(|(idx, record)| (idx + 1, record.coords_raw()))
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
        let player_count = self.conquest.player_count() as usize;

        for (idx, record) in self.planets.records.iter().enumerate() {
            let planet_index_1_based = idx + 1;
            let owner = record.owner_empire_slot_raw() as usize;
            let is_homeworld_seed = record.is_homeworld_seed_ignoring_name();

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
                if owner != 0 && record.ownership_status_raw() != 2 {
                    errors.push(format!(
                        "PLANET[{}].ownership_status expected 2 for owned homeworld seed, got {}",
                        planet_index_1_based,
                        record.ownership_status_raw()
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

        for (idx, record) in self.planets.records.iter().enumerate() {
            if !record.is_homeworld_seed_ignoring_name() {
                continue;
            }
            let planet_index_1_based = idx + 1;
            if record.potential_production_points() != 100 {
                errors.push(format!(
                    "PLANET[{}].potential_production expected 100 for homeworld seed, got {}",
                    planet_index_1_based,
                    record.potential_production_points()
                ));
            }
            if record.present_production() != 100.0 {
                errors.push(format!(
                    "PLANET[{}].present_production expected 100 for homeworld seed, got {}",
                    planet_index_1_based,
                    record.present_production()
                ));
            }
            if record.ownership_status_raw() != 2 {
                errors.push(format!(
                    "PLANET[{}].ownership_status expected 2 for homeworld seed, got {}",
                    planet_index_1_based,
                    record.ownership_status_raw()
                ));
            }
            if record.army_count_raw() != 10 {
                errors.push(format!(
                    "PLANET[{}].army_count_raw expected 10 for homeworld seed, got {}",
                    planet_index_1_based,
                    record.army_count_raw()
                ));
            }
            if record.ground_batteries_raw() != 4 {
                errors.push(format!(
                    "PLANET[{}].ground_batteries_raw expected 4 for homeworld seed, got {}",
                    planet_index_1_based,
                    record.ground_batteries_raw()
                ));
            }
            if record.stored_goods() != 35.0 {
                errors.push(format!(
                    "PLANET[{}].stored_goods expected 35 for homeworld seed, got {}",
                    planet_index_1_based,
                    record.stored_goods()
                ));
            }
            if (0..10)
                .any(|slot| record.build_count_raw(slot) != 0 || record.build_kind_raw(slot) != 0)
            {
                errors.push(format!(
                    "PLANET[{}] build queue expected all zeroes for homeworld seed",
                    planet_index_1_based
                ));
            }
            if (0..crate::STARDOCK_SLOT_COUNT).any(|slot| {
                record.stardock_kind_raw(slot) != 0 || record.stardock_count_raw(slot) != 0
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

        for (idx, record) in self.planets.records.iter().enumerate() {
            if record.is_homeworld_seed_ignoring_name() {
                continue;
            }
            let planet_index_1_based = idx + 1;
            if record.status_or_name_summary() != "Unowned" {
                errors.push(format!(
                    "PLANET[{}].status_or_name expected 'Unowned', got {:?}",
                    planet_index_1_based,
                    record.status_or_name_summary()
                ));
            }
            if record.owner_empire_slot_raw() != 0 {
                errors.push(format!(
                    "PLANET[{}].owner_empire_slot expected 0 for unowned baseline, got {}",
                    planet_index_1_based,
                    record.owner_empire_slot_raw()
                ));
            }
            if record.ownership_status_raw() != 0 {
                errors.push(format!(
                    "PLANET[{}].ownership_status expected 0 for unowned baseline, got {}",
                    planet_index_1_based,
                    record.ownership_status_raw()
                ));
            }
            if record.army_count_raw() != 0 {
                errors.push(format!(
                    "PLANET[{}].army_count_raw expected 0 for unowned baseline, got {}",
                    planet_index_1_based,
                    record.army_count_raw()
                ));
            }
            if record.ground_batteries_raw() != 0 {
                errors.push(format!(
                    "PLANET[{}].ground_batteries_raw expected 0 for unowned baseline, got {}",
                    planet_index_1_based,
                    record.ground_batteries_raw()
                ));
            }
            if record.present_production() != 0.0 {
                errors.push(format!(
                    "PLANET[{}].present_production expected 0 for unowned baseline, got {}",
                    planet_index_1_based,
                    record.present_production()
                ));
            }
            if record.stored_goods() != 0.0 {
                errors.push(format!(
                    "PLANET[{}].stored_goods expected 0 for unowned baseline, got {}",
                    planet_index_1_based,
                    record.stored_goods()
                ));
            }
            if (0..10)
                .any(|slot| record.build_count_raw(slot) != 0 || record.build_kind_raw(slot) != 0)
            {
                errors.push(format!(
                    "PLANET[{}] build queue expected all zeroes for unowned baseline",
                    planet_index_1_based
                ));
            }
            if (0..crate::STARDOCK_SLOT_COUNT).any(|slot| {
                record.stardock_kind_raw(slot) != 0 || record.stardock_count_raw(slot) != 0
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
