use super::*;
use crate::{
    Coords, ProductionItemKind, Real48, is_homeworld_seed_ignoring_name, set_armies_aboard,
    set_fleet_at_rest, set_mission_target, set_planet_name, set_troop_transports,
    snapshot_previous_year,
};

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
        if self.ipbms.len() != expected_ipbm {
            errors.push(format!(
                "IPBM.DAT record count expected {}, got {}",
                expected_ipbm,
                self.ipbms.len()
            ));
        }

        errors
    }

    pub fn current_known_player_input_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        for (fleet_idx, fleet) in self.fleets.iter().enumerate() {
            if let Err(reason) = self.validate_fleet_player_inputs(
                fleet_idx + 1,
                fleet.order.code(),
                fleet.target.to_array(),
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
        for planet_idx in 0..self.planets.len() {
            if let Err(reason) = self.validate_planet_player_inputs(planet_idx + 1) {
                errors.push(format!(
                    "PLANET[{}] invalid player input: {:?}",
                    planet_idx + 1,
                    reason
                ));
            }
        }
        for (player_idx, player) in self.players.iter().enumerate() {
            if player.tax_rate > 100 {
                errors.push(format!(
                    "PLAYER[{}] invalid tax rate {}",
                    player_idx + 1,
                    player.tax_rate
                ));
            }
        }
        for player_idx in 0..self.players.len() {
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
        let player_count = self.conquest.player_count as usize;
        for (idx, record) in self.planets.iter().enumerate() {
            let owner = record.owner.0 as usize;
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
        let player_count = self.conquest.player_count as usize;
        for (idx, record) in self.bases.iter().enumerate() {
            let owner = record.owner.0 as usize;
            let is_active = record.base_id.0 != 0;
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
        let ipbm_count = self.ipbms.len() as u16;
        if self.players.is_empty() {
            return;
        }
        self.relink_base_chain(1);
        self.players[0].ipbms_first.0 = ipbm_count;
    }

    pub fn sync_current_known_baseline_controls_and_counts(&mut self) {
        self.sync_player1_current_known_counts();

        self.setup.version_tag = *b"EC151";
        self.setup.com_irq = [4, 3, 4, 3];
        self.setup.com_hw_flow_control = [true; 4];
        self.setup.snoop = true;
        self.setup.max_key_gap_minutes = 10;
        self.setup.remote_timeout = true;
        self.setup.local_timeout = false;
        self.setup.minimum_time_minutes = 0;
        self.setup.purge_after_turns = 0;
        self.setup.autopilot_after_turns = 0;

        if !matches!(self.conquest.game_year, 3000 | 3001) {
            self.conquest.game_year = 3001;
        }
        self.conquest.player_count = 4;
        self.conquest.maintenance_days.0 = [true; 7];
    }

    pub fn sync_current_known_initialized_fleet_baseline(&mut self) {
        let player_count = self.conquest.player_count as usize;
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
                let mut record = Fleet::default();
                let fleet_id = fleet_record_index_1_based as u16;
                let local_slot = (slot_idx + 1) as u16;
                let owner_empire = (block_idx + 1) as u8;
                let prev = if slot_idx == 0 { 0 } else { fleet_id - 1 };
                let next = if slot_idx == 3 { 0 } else { fleet_id + 1 };

                record.fleet_number.0 = local_slot;
                record.owner.0 = owner_empire;
                record.next.0 = next;
                record.record.0 = fleet_id;
                record.prev.0 = prev;
                record.max_speed = if slot_idx < 2 { 3 } else { 6 };
                record.speed = 0;
                record.sector = Coords::from_array(coords);
                set_fleet_at_rest(&mut record);
                record.order = crate::Order::GuardBlockadeWorld;
                record.target = Coords::from_array(coords);
                set_mission_target(&mut record, 1, 0);
                record.ships.scouts = 0;
                record.rules_of_engagement = 6;
                record.ships.battleships = 0;
                record.ships.cruisers = if slot_idx < 2 { 1 } else { 0 };
                record.ships.destroyers = if slot_idx < 2 { 0 } else { 1 };
                set_troop_transports(&mut record, 0);
                set_armies_aboard(&mut record, 0);
                record.ships.etacs = if slot_idx < 2 { 1 } else { 0 };

                records.push(record);
            }
        }

        self.fleets = records;
    }

    pub fn sync_current_known_initialized_planet_payloads(&mut self) {
        let player_count = self.conquest.player_count as usize;

        for record in &mut self.planets {
            let owner = record.owner.0 as usize;
            if is_homeworld_seed_ignoring_name(record) && (1..=player_count).contains(&owner) {
                record.potential_production = 100;
                record.production = Real48::from_f64(100.0);
                record.stored_goods = Real48::from_f64(35.0);
                set_planet_name(record, "Not Named Yet");
                for slot in 0..10 {
                    record.build_queue[slot].points_owed = 0;
                    record.build_queue[slot].kind = ProductionItemKind::Unknown(0);
                }
                for slot in 0..crate::STARDOCK_SLOT_COUNT {
                    record.stardock[slot] = 0;
                }
                record.armies = 10;
                record.ground_batteries = 4;
                record.years_under_owner = 2;
            } else if owner == 0 {
                set_planet_name(record, "Unowned");
                record.production = Real48::ZERO;
                record.stored_goods = Real48::ZERO;
                for slot in 0..10 {
                    record.build_queue[slot].points_owed = 0;
                    record.build_queue[slot].kind = ProductionItemKind::Unknown(0);
                }
                for slot in 0..crate::STARDOCK_SLOT_COUNT {
                    record.stardock[slot] = 0;
                }
                record.armies = 0;
                record.ground_batteries = 0;
                record.years_under_owner = 0;
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

    /// CONQUEST.DAT as one pass over the initialized baseline leaves it:
    /// the year after 3000, and the rankings ECMAINT snapshots at the start
    /// of the pass.
    pub fn sync_current_known_initialized_conquest_post_maint_header(&mut self) {
        self.conquest.game_year = 3001;
        snapshot_previous_year(&mut self.conquest, &self.players);
    }

    pub fn current_known_empty_auxiliary_state_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if !self.bases.is_empty() {
            errors.push(format!(
                "BASES.DAT expected empty auxiliary baseline, got {} records",
                self.bases.len()
            ));
        }
        if !self.ipbms.is_empty() {
            errors.push(format!(
                "IPBM.DAT expected empty auxiliary baseline, got {} records",
                self.ipbms.len()
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
        self.bases.clear();
        self.ipbms.clear();
    }

    pub fn current_known_setup_baseline_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.setup.version_tag != *b"EC151" {
            errors.push(format!(
                "SETUP.DAT.version_tag expected EC151, got {:?}",
                self.setup.version_tag.as_slice()
            ));
        }
        if self.setup.com_irq != [4, 3, 4, 3] || self.setup.com_hw_flow_control != [true; 4] {
            errors.push(format!(
                "SETUP.DAT COM ports expected IRQs [4, 3, 4, 3] with flow control on, got IRQs {:?} and flow control {:?}",
                self.setup.com_irq, self.setup.com_hw_flow_control
            ));
        }
        if !self.setup.snoop {
            errors.push("SETUP.DAT.snoop expected enabled in baseline".to_string());
        }
        if self.setup.max_key_gap_minutes != 10 {
            errors.push(format!(
                "SETUP.DAT.max_time_between_keys expected 10, got {}",
                u8::try_from(self.setup.max_key_gap_minutes).unwrap_or(u8::MAX)
            ));
        }
        if !self.setup.remote_timeout {
            errors.push("SETUP.DAT.remote_timeout expected enabled in baseline".to_string());
        }
        if self.setup.local_timeout {
            errors.push("SETUP.DAT.local_timeout expected disabled in baseline".to_string());
        }
        if self.setup.minimum_time_minutes != 0 {
            errors.push(format!(
                "SETUP.DAT.minimum_time_granted expected 0, got {}",
                self.setup.minimum_time_minutes
            ));
        }
        if self.setup.purge_after_turns != 0 {
            errors.push(format!(
                "SETUP.DAT.purge_after_turns expected 0, got {}",
                u8::try_from(self.setup.purge_after_turns).unwrap_or(u8::MAX)
            ));
        }
        if self.setup.autopilot_after_turns != 0 {
            errors.push(format!(
                "SETUP.DAT.autopilot_inactive_turns expected 0, got {}",
                u8::try_from(self.setup.autopilot_after_turns).unwrap_or(u8::MAX)
            ));
        }

        errors
    }

    pub fn current_known_conquest_baseline_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if !matches!(self.conquest.game_year, 3000 | 3001) {
            errors.push(format!(
                "CONQUEST.DAT.game_year expected 3000 or 3001 for preserved initialized/post-maint baseline, got {}",
                self.conquest.game_year
            ));
        }
        if self.conquest.player_count != 4 {
            errors.push(format!(
                "CONQUEST.DAT.player_count expected 4, got {}",
                self.conquest.player_count
            ));
        }
        if self.conquest.maintenance_days.0.map(u8::from) != [1; 7] {
            errors.push(format!(
                "CONQUEST.DAT.maintenance_schedule expected [1, 1, 1, 1, 1, 1, 1], got {:?}",
                self.conquest.maintenance_days.0.map(u8::from)
            ));
        }

        errors
    }
}
