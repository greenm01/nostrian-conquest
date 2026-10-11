use super::*;

impl CoreGameData {
    pub fn current_known_compliance_status(&self) -> CurrentKnownComplianceStatus {
        CurrentKnownComplianceStatus {
            fleet_order: self
                .fleet_order_errors_current_known(1, 0x03, 0x0C, [0x0F, 0x0D], None, None)
                .is_empty(),
            planet_build: self
                .planet_build_errors_current_known(15, 0x03, 0x01)
                .is_empty(),
            guard_starbase: self
                .guard_starbase_linkage_errors_for_guarding_fleets_current_known(1)
                .is_empty(),
            ipbm: self.ipbm_count_length_errors_current_known().is_empty(),
        }
    }

    pub fn current_known_key_word_summary(&self) -> CurrentKnownKeyWordSummary {
        let player1 = self.players.first();
        let fleet1 = self.fleets.first();
        let base1 = self.bases.first();

        CurrentKnownKeyWordSummary {
            player_first_base_record: player1
                .map(|record| record.first_base_record_raw())
                .unwrap_or(0),
            player_ipbm_count: player1
                .map(|record| record.ipbm_chain_head_raw())
                .unwrap_or(0),
            fleet1_local_slot: fleet1.map(|record| record.local_slot_word_raw()),
            fleet1_id: fleet1.map(|record| record.fleet_id_word_raw()),
            fleet1_guard_index: fleet1.map(|record| record.guard_starbase_index_raw()),
            fleet1_target: fleet1.map(|record| record.standing_order_target_coords_raw()),
            base1_escort_fleet_slot: base1.map(|record| record.escort_fleet_slot_raw()),
            base1_id: base1.map(|record| record.base_id_raw()),
            base1_record_number: base1.map(|record| record.record_number_raw()),
            base1_coords: base1.map(|record| record.coords_raw()),
            ipbm_record_count: self.ipbms.len(),
        }
    }
}
