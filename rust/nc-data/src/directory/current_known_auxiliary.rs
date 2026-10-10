use super::*;

impl CoreGameData {
    pub fn planet_build_errors_current_known(
        &self,
        record_index_1_based: usize,
        slot_raw: u8,
        kind_raw: u8,
    ) -> Vec<String> {
        let mut errors = Vec::new();
        match self.planets.records.get(record_index_1_based - 1) {
            Some(record) => {
                if record.build_count_raw(0) != slot_raw {
                    errors.push(format!(
                        "PLANET[{}].build_slot expected {:#04x}, got {:#04x}",
                        record_index_1_based,
                        slot_raw,
                        record.build_count_raw(0)
                    ));
                }
                if record.build_kind_raw(0) != kind_raw {
                    errors.push(format!(
                        "PLANET[{}].build_kind expected {:#04x}, got {:#04x}",
                        record_index_1_based,
                        kind_raw,
                        record.build_kind_raw(0)
                    ));
                }
            }
            None => errors.push(format!("PLANETS.DAT missing record {record_index_1_based}")),
        }
        errors
    }

    pub fn guard_starbase_onebase_errors_current_known(&self) -> Vec<String> {
        let mut errors = Vec::new();

        match self.player.records.first() {
            Some(record) if record.starbase_count_raw() == 1 => {}
            Some(record) => errors.push(format!(
                "PLAYER[1].starbase_count_raw expected 1, got {}",
                record.starbase_count_raw()
            )),
            None => errors.push("PLAYER.DAT missing record 1".to_string()),
        }

        match self.fleets.records.first() {
            Some(record) => {
                if record.standing_order_code_raw() != 0x04 {
                    errors.push(format!(
                        "FLEET[1].order expected 0x04, got {:#04x}",
                        record.standing_order_code_raw()
                    ));
                }
                if record.guard_starbase_index_raw() == 0 {
                    errors.push("FLEET[1].guard starbase index expected non-zero".to_string());
                }
            }
            None => errors.push("FLEETS.DAT missing record 1".to_string()),
        }

        let Some(fleet) = self.fleets.records.first() else {
            return errors;
        };
        let Some(player1) = self.player.records.first() else {
            return errors;
        };

        if self.bases.records.len() != 1 {
            errors.push(format!(
                "BASES.DAT expected 1 record, got {}",
                self.bases.records.len()
            ));
        } else {
            let base = &self.bases.records[0];
            if base.local_slot_raw() == 0 {
                errors.push("BASES[1].local_slot expected non-zero".to_string());
            }
            if base.summary_word_raw() != fleet.local_slot_word_raw() {
                errors.push(format!(
                    "BASES[1].summary_word expected FLEET[1].local_slot_word {}, got {}",
                    fleet.local_slot_word_raw(),
                    base.summary_word_raw()
                ));
            }
            if u16::from(base.base_id_raw()) != fleet.guard_starbase_index_raw() {
                errors.push(format!(
                    "BASES[1].base_id expected FLEET[1].guard index {}, got {}",
                    fleet.guard_starbase_index_raw(),
                    base.base_id_raw()
                ));
            }
            if base.coords_raw() != fleet.standing_order_target_coords_raw() {
                errors.push(format!(
                    "BASES[1].coords expected {:?}, got {:?}",
                    fleet.standing_order_target_coords_raw(),
                    base.coords_raw()
                ));
            }
            if base.trailing_coords_raw() != base.coords_raw() {
                errors.push(format!(
                    "BASES[1].trailing coords expected {:?}, got {:?}",
                    base.coords_raw(),
                    base.trailing_coords_raw()
                ));
            }
            if base.chain_word_raw() != player1.starbase_count_raw() {
                errors.push(format!(
                    "BASES[1].chain_word expected PLAYER[1].starbase_count_raw {}, got {}",
                    player1.starbase_count_raw(),
                    base.chain_word_raw()
                ));
            }
            if fleet.local_slot_word_raw() != player1.starbase_count_raw() {
                errors.push(format!(
                    "FLEET[1].local slot word expected PLAYER[1].starbase_count_raw {}, got {}",
                    player1.starbase_count_raw(),
                    fleet.local_slot_word_raw()
                ));
            }
            if fleet.fleet_id_word_raw() != base.chain_word_raw() {
                errors.push(format!(
                    "FLEET[1].fleet ID word expected BASES[1].chain_word {}, got {}",
                    base.chain_word_raw(),
                    fleet.fleet_id_word_raw()
                ));
            }
            if base.tuple_a_payload_raw() != fleet.tuple_a_payload_raw() {
                errors.push(format!(
                    "BASES[1].tuple_a_payload expected FLEET[1].tuple_a_payload {:?}, got {:?}",
                    fleet.tuple_a_payload_raw(),
                    base.tuple_a_payload_raw()
                ));
            }
            if base.tuple_b_payload_raw() != fleet.tuple_b_payload_raw() {
                errors.push(format!(
                    "BASES[1].tuple_b_payload expected FLEET[1].tuple_b_payload {:?}, got {:?}",
                    fleet.tuple_b_payload_raw(),
                    base.tuple_b_payload_raw()
                ));
            }
            if base.tuple_c_payload_raw() != fleet.tuple_c_payload_raw() {
                errors.push(format!(
                    "BASES[1].tuple_c_payload expected FLEET[1].tuple_c_payload {:?}, got {:?}",
                    fleet.tuple_c_payload_raw(),
                    base.tuple_c_payload_raw()
                ));
            }
        }

        errors
    }

    pub fn guard_starbase_linkage_summary_current_known(
        &self,
        player_record_index_1_based: usize,
        fleet_record_index_1_based: usize,
    ) -> Result<CurrentKnownGuardStarbaseLinkageSummary, GameStateMutationError> {
        let player = self
            .player
            .records
            .get(player_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingPlayerRecord {
                index_1_based: player_record_index_1_based,
            })?;
        let fleet = self
            .fleets
            .records
            .get(fleet_record_index_1_based - 1)
            .ok_or(GameStateMutationError::MissingFleetRecord {
                index_1_based: fleet_record_index_1_based,
            })?;

        let selected_base = fleet
            .guard_starbase_index_raw()
            .checked_sub(1)
            .and_then(|idx| self.bases.records.get(idx as usize));

        Ok(CurrentKnownGuardStarbaseLinkageSummary {
            player_record_index_1_based,
            fleet_record_index_1_based,
            player_starbase_count: player.starbase_count_raw(),
            fleet_order: fleet.standing_order_code_raw(),
            fleet_local_slot: fleet.local_slot_word_raw(),
            fleet_id: fleet.fleet_id_word_raw(),
            guard_index: fleet.guard_starbase_index_raw(),
            target_coords: fleet.standing_order_target_coords_raw(),
            selected_base_present: selected_base.is_some(),
            selected_base_summary_word: selected_base.map(|base| base.summary_word_raw()),
            selected_base_id: selected_base.map(|base| base.base_id_raw()),
            selected_base_chain_word: selected_base.map(|base| base.chain_word_raw()),
            selected_base_coords: selected_base.map(|base| base.coords_raw()),
            selected_base_trailing_coords: selected_base.map(|base| base.trailing_coords_raw()),
            selected_base_owner_empire: selected_base.map(|base| base.owner_empire_raw()),
        })
    }

    pub fn guard_starbase_linkage_errors_current_known(
        &self,
        player_record_index_1_based: usize,
        fleet_record_index_1_based: usize,
    ) -> Vec<String> {
        let mut errors = Vec::new();

        let summary = match self.guard_starbase_linkage_summary_current_known(
            player_record_index_1_based,
            fleet_record_index_1_based,
        ) {
            Ok(summary) => summary,
            Err(GameStateMutationError::MissingPlayerRecord { index_1_based }) => {
                errors.push(format!("PLAYER.DAT missing record {index_1_based}"));
                return errors;
            }
            Err(GameStateMutationError::MissingFleetRecord { index_1_based }) => {
                errors.push(format!("FLEETS.DAT missing record {index_1_based}"));
                return errors;
            }
            Err(other) => {
                errors.push(other.to_string());
                return errors;
            }
        };

        if summary.fleet_order != 0x04 {
            errors.push(format!(
                "FLEET[{}].order expected 0x04, got {:#04x}",
                fleet_record_index_1_based, summary.fleet_order
            ));
        }
        if summary.guard_index == 0 {
            errors.push(format!(
                "FLEET[{}].guard starbase index expected non-zero",
                fleet_record_index_1_based
            ));
            return errors;
        }
        if summary.player_starbase_count == 0 {
            errors.push(format!(
                "PLAYER[{}].starbase_count_raw expected non-zero, got 0",
                player_record_index_1_based
            ));
        }
        if summary.guard_index as u16 > summary.player_starbase_count {
            errors.push(format!(
                "FLEET[{}].guard index {} exceeds PLAYER[{}].starbase_count_raw {}",
                fleet_record_index_1_based,
                summary.guard_index,
                player_record_index_1_based,
                summary.player_starbase_count
            ));
        }
        if !summary.selected_base_present {
            errors.push(format!(
                "BASES.DAT missing selected starbase record {}",
                summary.guard_index
            ));
            return errors;
        }

        if summary.selected_base_id.map(u16::from) != Some(summary.guard_index) {
            errors.push(format!(
                "BASES[{}].base_id expected FLEET[{}].guard index {}, got {:?}",
                summary.guard_index,
                fleet_record_index_1_based,
                summary.guard_index,
                summary.selected_base_id
            ));
        }
        if summary.selected_base_summary_word != Some(summary.fleet_local_slot) {
            errors.push(format!(
                "BASES[{}].summary_word expected FLEET[{}].local_slot_word {}, got {:?}",
                summary.guard_index,
                fleet_record_index_1_based,
                summary.fleet_local_slot,
                summary.selected_base_summary_word
            ));
        }
        if summary.selected_base_coords != Some(summary.target_coords) {
            errors.push(format!(
                "BASES[{}].coords expected FLEET[{}].target {:?}, got {:?}",
                summary.guard_index,
                fleet_record_index_1_based,
                summary.target_coords,
                summary.selected_base_coords
            ));
        }
        if summary.selected_base_trailing_coords != summary.selected_base_coords {
            errors.push(format!(
                "BASES[{}].trailing coords expected {:?}, got {:?}",
                summary.guard_index,
                summary.selected_base_coords.unwrap_or([0, 0]),
                summary.selected_base_trailing_coords
            ));
        }
        let expected_owner_empire = player_record_index_1_based as u8;
        if summary.selected_base_owner_empire != Some(expected_owner_empire) {
            errors.push(format!(
                "BASES[{}].owner_empire expected {}, got {:?}",
                summary.guard_index, expected_owner_empire, summary.selected_base_owner_empire
            ));
        }

        errors
    }

    pub fn guarding_fleet_record_indexes_current_known(&self) -> Vec<usize> {
        self.fleets
            .records
            .iter()
            .enumerate()
            .filter_map(|(idx, fleet)| (fleet.standing_order_code_raw() == 0x04).then_some(idx + 1))
            .collect()
    }

    pub fn guard_starbase_linkage_summaries_for_guarding_fleets_current_known(
        &self,
        player_record_index_1_based: usize,
    ) -> Vec<CurrentKnownGuardStarbaseLinkageSummary> {
        self.guarding_fleet_record_indexes_current_known()
            .into_iter()
            .filter_map(|fleet_record_index_1_based| {
                self.guard_starbase_linkage_summary_current_known(
                    player_record_index_1_based,
                    fleet_record_index_1_based,
                )
                .ok()
            })
            .collect()
    }

    pub fn guard_starbase_linkage_errors_for_guarding_fleets_current_known(
        &self,
        player_record_index_1_based: usize,
    ) -> Vec<String> {
        let guarding_fleets = self.guarding_fleet_record_indexes_current_known();
        if guarding_fleets.is_empty() {
            return vec!["no guarding fleets found".to_string()];
        }

        let mut errors = Vec::new();
        for fleet_record_index_1_based in guarding_fleets {
            errors.extend(self.guard_starbase_linkage_errors_current_known(
                player_record_index_1_based,
                fleet_record_index_1_based,
            ));
        }
        errors
    }

    pub fn ipbm_count_length_errors_current_known(&self) -> Vec<String> {
        let expected_count = self.player1_ipbm_count_current_known();
        let actual_count = self.ipbm.records.len();
        let expected_size = expected_count * crate::IPBM_RECORD_SIZE;
        let actual_size = self.ipbm.to_bytes().len();

        let mut errors = Vec::new();
        if actual_count != expected_count {
            errors.push(format!(
                "IPBM record count expected {}, got {}",
                expected_count, actual_count
            ));
        }
        if actual_size != expected_size {
            errors.push(format!(
                "IPBM.DAT size expected {}, got {}",
                expected_size, actual_size
            ));
        }
        errors
    }
}
