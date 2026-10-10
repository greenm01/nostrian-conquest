use super::*;

impl CoreGameData {
    pub fn current_known_core_state_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let expected_ipbm = self.player1_ipbm_count_current_known();

        errors.extend(self.current_known_planet_owner_slot_errors());
        errors.extend(self.current_known_base_owner_empire_errors());
        errors.extend(self.current_known_player1_starbase_count_errors());
        errors.extend(self.current_known_initialized_fleet_block_errors());
        errors.extend(self.current_known_initialized_fleet_payload_errors());
        errors.extend(self.current_known_initialized_fleet_mission_errors());
        errors.extend(self.current_known_homeworld_seed_errors());
        errors.extend(self.current_known_initialized_planet_ownership_errors());
        errors.extend(self.current_known_homeworld_seed_payload_errors());
        errors.extend(self.current_known_unowned_planet_payload_errors());
        errors.extend(self.current_known_empty_auxiliary_state_errors());
        errors.extend(self.current_known_initialized_homeworld_alignment_errors());
        errors.extend(self.current_known_setup_baseline_errors());
        errors.extend(self.current_known_conquest_baseline_errors());
        errors.extend(self.current_known_player_input_errors());
        if self.ipbm.records.len() != expected_ipbm {
            errors.push(format!(
                "IPBM.DAT record count expected {}, got {}",
                expected_ipbm,
                self.ipbm.records.len()
            ));
        }

        errors
    }

    pub fn current_known_player_input_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        for (fleet_idx, fleet) in self.fleets.records.iter().enumerate() {
            if let Err(reason) = self.validate_fleet_player_inputs(
                fleet_idx + 1,
                fleet.standing_order_code_raw(),
                fleet.standing_order_target_coords_raw(),
                None,
                None,
            ) {
                errors.push(format!(
                    "FLEET[{}] invalid player input: {:?}",
                    fleet_idx + 1,
                    reason
                ));
            }
        }
        for planet_idx in 0..self.planets.records.len() {
            if let Err(reason) = self.validate_planet_player_inputs(planet_idx + 1) {
                errors.push(format!(
                    "PLANET[{}] invalid player input: {:?}",
                    planet_idx + 1,
                    reason
                ));
            }
        }
        for (player_idx, player) in self.player.records.iter().enumerate() {
            if player.tax_rate() > 100 {
                errors.push(format!(
                    "PLAYER[{}] invalid tax rate {}",
                    player_idx + 1,
                    player.tax_rate()
                ));
            }
        }
        for player_idx in 0..self.player.records.len() {
            for reason in self.validate_player_diplomacy_inputs(player_idx + 1) {
                errors.push(format!(
                    "PLAYER[{}] invalid diplomacy input: {:?}",
                    player_idx + 1,
                    reason
                ));
            }
        }
        errors
    }

    pub fn current_known_player1_starbase_count_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if let Err(error) = self.base_chain(1) {
            errors.push(error);
        }
        let expected_bases = self.player1_starbase_count_current_known();
        let owned_bases = self.player1_owned_base_record_count_current_known();
        if owned_bases != expected_bases {
            errors.push(format!(
                "PLAYER[1]-owned BASES.DAT record count expected {}, got {}",
                expected_bases, owned_bases
            ));
        }
        errors
    }

    pub fn current_known_planet_owner_slot_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let player_count = self.conquest.player_count() as usize;
        for (idx, record) in self.planets.records.iter().enumerate() {
            let owner = record.owner_empire_slot_raw() as usize;
            if owner > player_count {
                errors.push(format!(
                    "PLANET[{}].owner_empire_slot expected <= {}, got {}",
                    idx + 1,
                    player_count,
                    owner
                ));
            }
        }
        errors
    }

    pub fn current_known_base_owner_empire_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let player_count = self.conquest.player_count() as usize;
        for (idx, record) in self.bases.records.iter().enumerate() {
            let owner = record.owner_empire_raw() as usize;
            let is_active = record.base_id_raw() != 0;
            if (is_active && owner == 0) || owner > player_count {
                errors.push(format!(
                    "BASES[{}].owner_empire expected 1..={}, got {}",
                    idx + 1,
                    player_count,
                    owner
                ));
            }
        }
        errors
    }

    pub fn sync_player1_current_known_counts(&mut self) {
        let ipbm_count = self.ipbm.records.len() as u16;
        if self.player.records.is_empty() {
            return;
        }
        self.relink_base_chain(1);
        self.player.records[0].set_ipbm_count_raw(ipbm_count);
    }

    pub fn sync_current_known_baseline_controls_and_counts(&mut self) {
        self.sync_player1_current_known_counts();

        self.setup.set_version_tag(b"EC151");
        self.setup.set_option_prefix(&[4, 3, 4, 3, 1, 1, 1, 1]);
        self.setup.set_snoop_enabled(true);
        self.setup.set_max_time_between_keys_minutes_raw(10);
        self.setup.set_remote_timeout_enabled(true);
        self.setup.set_local_timeout_enabled(false);
        self.setup.set_minimum_time_granted_minutes_raw(0);
        self.setup.set_purge_after_turns_raw(0);
        self.setup.set_autopilot_inactive_turns_raw(0);

        if !matches!(self.conquest.game_year(), 3000 | 3001) {
            self.conquest.set_game_year(3001);
        }
        self.conquest.set_player_count(4);
        self.conquest.set_maintenance_schedule_bytes([1; 7]);
    }

    pub fn sync_current_known_initialized_fleet_baseline(&mut self) {
        let player_count = self.conquest.player_count() as usize;
        let expected_fleet_count = player_count.saturating_mul(4);
        let homeworld_coords = self.player_homeworld_seed_coords_current_known();

        let mut records = Vec::with_capacity(expected_fleet_count);
        for block_idx in 0..player_count {
            let coords = homeworld_coords
                .get(block_idx)
                .and_then(|coords| *coords)
                .unwrap_or([0, 0]);

            for slot_idx in 0..4 {
                let fleet_record_index_1_based = block_idx * 4 + slot_idx + 1;
                let mut record = FleetRecord::new_zeroed();
                let fleet_id = fleet_record_index_1_based as u16;
                let local_slot = (slot_idx + 1) as u16;
                let owner_empire = (block_idx + 1) as u8;
                let prev = if slot_idx == 0 { 0 } else { fleet_id - 1 };
                let next = if slot_idx == 3 { 0 } else { fleet_id + 1 };

                record.set_local_slot_word_raw(local_slot);
                record.set_owner_empire_raw(owner_empire);
                record.set_next_fleet_link_word_raw(next);
                record.set_fleet_id_word_raw(fleet_id);
                record.set_previous_fleet_id(prev);
                record.set_max_speed(if slot_idx < 2 { 3 } else { 6 });
                record.set_current_speed(0);
                record.set_current_location_coords_raw(coords);
                record.set_tuple_a_payload_raw([0x80, 0, 0, 0, 0]);
                record.set_tuple_b_payload_raw([0x80, 0, 0, 0, 0]);
                record.set_tuple_c_payload_raw([0x81, 0, 0, 0, 0]);
                record.set_standing_order_kind(crate::Order::GuardBlockadeWorld);
                record.set_standing_order_target_coords_raw(coords);
                record.set_mission_aux_bytes([1, 0]);
                record.set_scout_count(0);
                record.set_rules_of_engagement(6);
                record.set_battleship_count(0);
                record.set_cruiser_count(if slot_idx < 2 { 1 } else { 0 });
                record.set_destroyer_count(if slot_idx < 2 { 0 } else { 1 });
                record.set_troop_transport_count(0);
                record.set_army_count(0);
                record.set_etac_count(if slot_idx < 2 { 1 } else { 0 });

                records.push(record);
            }
        }

        self.fleets.records = records;
    }

    pub fn sync_current_known_initialized_planet_payloads(&mut self) {
        let player_count = self.conquest.player_count() as usize;

        for record in &mut self.planets.records {
            let owner = record.owner_empire_slot_raw() as usize;
            if record.is_homeworld_seed_ignoring_name() && (1..=player_count).contains(&owner) {
                record.set_potential_production_raw([100, 135]);
                record.set_factories_raw([0, 0, 0, 0, 72, 134]);
                record.set_stored_goods_raw(0);
                record.set_economy_marker_raw(12);
                record.set_status_or_name_summary_raw("Not Named Yet");
                for slot in 0..10 {
                    record.set_build_count_raw(slot, 0);
                    record.set_build_kind_raw(slot, 0);
                }
                for slot in 0..crate::STARDOCK_SLOT_COUNT {
                    record.set_stardock_count_raw(slot, 0);
                    record.set_stardock_kind_raw(slot, 0);
                }
                record.set_population_raw([0; 6]);
                record.set_army_count_raw(10);
                record.set_ground_batteries_raw(4);
                record.set_ownership_status_raw(2);
            } else if owner == 0 {
                record.set_status_or_name_prefix_raw("Unowned");
                record.set_economy_marker_raw(0);
                record.set_factories_raw([0; 6]);
                record.set_stored_goods_raw(0);
                for slot in 0..10 {
                    record.set_build_count_raw(slot, 0);
                    record.set_build_kind_raw(slot, 0);
                }
                for slot in 0..crate::STARDOCK_SLOT_COUNT {
                    record.set_stardock_count_raw(slot, 0);
                    record.set_stardock_kind_raw(slot, 0);
                }
                record.set_population_raw([0; 6]);
                record.set_army_count_raw(0);
                record.set_ground_batteries_raw(0);
                record.set_ownership_status_raw(0);
            }
        }
    }

    pub fn sync_current_known_initialized_post_maint_baseline(&mut self) {
        self.sync_current_known_empty_auxiliary_state();
        self.sync_current_known_baseline_controls_and_counts();
        self.sync_current_known_initialized_fleet_baseline();
        self.sync_current_known_initialized_planet_payloads();
        self.sync_current_known_initialized_conquest_post_maint_header();
    }

    pub fn sync_current_known_initialized_conquest_post_maint_header(&mut self) {
        self.conquest
            .set_control_header_bytes(&CURRENT_KNOWN_POST_MAINT_CONQUEST_CONTROL_HEADER);
    }

    pub fn current_known_empty_auxiliary_state_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if !self.bases.records.is_empty() {
            errors.push(format!(
                "BASES.DAT expected empty auxiliary baseline, got {} records",
                self.bases.records.len()
            ));
        }
        if !self.ipbm.records.is_empty() {
            errors.push(format!(
                "IPBM.DAT expected empty auxiliary baseline, got {} records",
                self.ipbm.records.len()
            ));
        }

        let guarding_fleet_count = self.guarding_fleet_record_indexes_current_known().len();
        if guarding_fleet_count != 0 {
            errors.push(format!(
                "guarding fleet count expected 0 in empty auxiliary baseline, got {}",
                guarding_fleet_count
            ));
        }

        errors
    }

    pub fn sync_current_known_empty_auxiliary_state(&mut self) {
        self.bases.records.clear();
        self.ipbm.records.clear();
    }

    pub fn current_known_setup_baseline_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.setup.version_tag() != b"EC151" {
            errors.push(format!(
                "SETUP.DAT.version_tag expected EC151, got {:?}",
                self.setup.version_tag()
            ));
        }
        if self.setup.option_prefix() != [4, 3, 4, 3, 1, 1, 1, 1] {
            errors.push(format!(
                "SETUP.DAT.option_prefix expected [4, 3, 4, 3, 1, 1, 1, 1], got {:?}",
                self.setup.option_prefix()
            ));
        }
        if !self.setup.snoop_enabled() {
            errors.push("SETUP.DAT.snoop expected enabled in baseline".to_string());
        }
        if self.setup.max_time_between_keys_minutes_raw() != 10 {
            errors.push(format!(
                "SETUP.DAT.max_time_between_keys expected 10, got {}",
                self.setup.max_time_between_keys_minutes_raw()
            ));
        }
        if !self.setup.remote_timeout_enabled() {
            errors.push("SETUP.DAT.remote_timeout expected enabled in baseline".to_string());
        }
        if self.setup.local_timeout_enabled() {
            errors.push("SETUP.DAT.local_timeout expected disabled in baseline".to_string());
        }
        if self.setup.minimum_time_granted_minutes_raw() != 0 {
            errors.push(format!(
                "SETUP.DAT.minimum_time_granted expected 0, got {}",
                self.setup.minimum_time_granted_minutes_raw()
            ));
        }
        if self.setup.purge_after_turns_raw() != 0 {
            errors.push(format!(
                "SETUP.DAT.purge_after_turns expected 0, got {}",
                self.setup.purge_after_turns_raw()
            ));
        }
        if self.setup.autopilot_inactive_turns_raw() != 0 {
            errors.push(format!(
                "SETUP.DAT.autopilot_inactive_turns expected 0, got {}",
                self.setup.autopilot_inactive_turns_raw()
            ));
        }

        errors
    }

    pub fn current_known_conquest_baseline_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if !matches!(self.conquest.game_year(), 3000 | 3001) {
            errors.push(format!(
                "CONQUEST.DAT.game_year expected 3000 or 3001 for preserved initialized/post-maint baseline, got {}",
                self.conquest.game_year()
            ));
        }
        if self.conquest.player_count() != 4 {
            errors.push(format!(
                "CONQUEST.DAT.player_count expected 4, got {}",
                self.conquest.player_count()
            ));
        }
        if self.conquest.maintenance_schedule_bytes() != [1; 7] {
            errors.push(format!(
                "CONQUEST.DAT.maintenance_schedule expected [1, 1, 1, 1, 1, 1, 1], got {:?}",
                self.conquest.maintenance_schedule_bytes()
            ));
        }

        errors
    }
}
