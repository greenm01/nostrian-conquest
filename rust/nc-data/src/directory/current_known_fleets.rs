use super::*;
use crate::Real48;

impl CoreGameData {
    pub fn player_fleet_chain_heads_current_known(&self) -> Vec<usize> {
        self.player
            .records
            .iter()
            .map(|record| record.fleet_chain_head_raw() as usize)
            .collect()
    }

    pub fn looks_like_initialized_fleet_blocks_current_known(&self) -> bool {
        let player_count = self.conquest.player_count() as usize;
        let expected_fleet_count = player_count.saturating_mul(4);
        player_count > 0
            && self.fleets.records.len() == expected_fleet_count
            && self
                .fleets
                .records
                .chunks_exact(4)
                .enumerate()
                .all(|(block_idx, group)| {
                    group.iter().enumerate().all(|(slot_idx, record)| {
                        let expected_fleet_id = (block_idx * 4 + slot_idx + 1) as u16;
                        let expected_local_slot = (slot_idx + 1) as u8;
                        let expected_prev = if slot_idx == 0 {
                            0
                        } else {
                            expected_fleet_id - 1
                        };
                        let expected_next = if slot_idx == 3 {
                            0
                        } else {
                            expected_fleet_id + 1
                        };
                        record.fleet_id() == expected_fleet_id
                            && record.local_slot() == expected_local_slot
                            && record.previous_fleet_id() == expected_prev
                            && record.next_fleet_id() == expected_next
                    })
                })
    }

    pub fn current_known_initialized_fleet_block_head_ids(&self) -> Vec<usize> {
        self.fleets
            .records
            .chunks(4)
            .filter_map(|group| group.first())
            .map(|record| record.fleet_id() as usize)
            .collect()
    }

    pub fn current_known_initialized_fleet_block_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let player_count = self.conquest.player_count() as usize;
        let expected_fleet_count = player_count.saturating_mul(4);

        if self.fleets.records.len() != expected_fleet_count {
            errors.push(format!(
                "FLEETS.DAT record count expected {}, got {}",
                expected_fleet_count,
                self.fleets.records.len()
            ));
            return errors;
        }

        for (block_idx, group) in self.fleets.records.chunks_exact(4).enumerate() {
            for (slot_idx, record) in group.iter().enumerate() {
                let fleet_record_index_1_based = block_idx * 4 + slot_idx + 1;
                let expected_fleet_id = fleet_record_index_1_based as u16;
                let expected_local_slot = (slot_idx + 1) as u8;
                let expected_prev = if slot_idx == 0 {
                    0
                } else {
                    expected_fleet_id - 1
                };
                let expected_next = if slot_idx == 3 {
                    0
                } else {
                    expected_fleet_id + 1
                };

                if record.fleet_id() != expected_fleet_id {
                    errors.push(format!(
                        "FLEET[{}].fleet_id expected {}, got {}",
                        fleet_record_index_1_based,
                        expected_fleet_id,
                        record.fleet_id()
                    ));
                }
                if record.local_slot() != expected_local_slot {
                    errors.push(format!(
                        "FLEET[{}].local_slot expected {}, got {}",
                        fleet_record_index_1_based,
                        expected_local_slot,
                        record.local_slot()
                    ));
                }
                if record.previous_fleet_id() != expected_prev {
                    errors.push(format!(
                        "FLEET[{}].previous_fleet_id expected {}, got {}",
                        fleet_record_index_1_based,
                        expected_prev,
                        record.previous_fleet_id()
                    ));
                }
                if record.next_fleet_id() != expected_next {
                    errors.push(format!(
                        "FLEET[{}].next_fleet_id expected {}, got {}",
                        fleet_record_index_1_based,
                        expected_next,
                        record.next_fleet_id()
                    ));
                }
            }
        }
        errors
    }

    pub fn current_known_initialized_fleet_payload_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let player_count = self.conquest.player_count() as usize;
        let expected_fleet_count = player_count.saturating_mul(4);

        if self.fleets.records.len() != expected_fleet_count {
            return errors;
        }

        for (block_idx, group) in self.fleets.records.chunks_exact(4).enumerate() {
            let expected_loc = group[0].current_location_coords_raw();
            let expected_mission = group[0].mission_param_bytes().to_vec();

            for (slot_idx, record) in group.iter().enumerate() {
                let fleet_record_index_1_based = block_idx * 4 + slot_idx + 1;
                let expected_owner_empire = (block_idx + 1) as u8;
                let expected_max_speed = if slot_idx < 2 { 3 } else { 6 };
                let expected_cur_speed = 0;
                let expected_ca = if slot_idx < 2 { 1 } else { 0 };
                let expected_dd = if slot_idx < 2 { 0 } else { 1 };
                let expected_et = if slot_idx < 2 { 1 } else { 0 };
                let expected_roe = 6;

                if record.current_location_coords_raw() != expected_loc {
                    errors.push(format!(
                        "FLEET[{}].current_location expected {:?}, got {:?}",
                        fleet_record_index_1_based,
                        expected_loc,
                        record.current_location_coords_raw()
                    ));
                }
                if record.mission_param_bytes() != expected_mission.as_slice() {
                    errors.push(format!(
                        "FLEET[{}].mission_param_bytes expected {:?}, got {:?}",
                        fleet_record_index_1_based,
                        expected_mission,
                        record.mission_param_bytes()
                    ));
                }
                if record.owner_empire_raw() != expected_owner_empire {
                    errors.push(format!(
                        "FLEET[{}].owner_empire expected {}, got {}",
                        fleet_record_index_1_based,
                        expected_owner_empire,
                        record.owner_empire_raw()
                    ));
                }
                if record.max_speed() != expected_max_speed {
                    errors.push(format!(
                        "FLEET[{}].max_speed expected {}, got {}",
                        fleet_record_index_1_based,
                        expected_max_speed,
                        record.max_speed()
                    ));
                }
                if record.current_speed() != expected_cur_speed {
                    errors.push(format!(
                        "FLEET[{}].current_speed expected {}, got {}",
                        fleet_record_index_1_based,
                        expected_cur_speed,
                        record.current_speed()
                    ));
                }
                if record.rules_of_engagement() != expected_roe {
                    errors.push(format!(
                        "FLEET[{}].roe expected {}, got {}",
                        fleet_record_index_1_based,
                        expected_roe,
                        record.rules_of_engagement()
                    ));
                }
                if record.cruiser_count() != expected_ca {
                    errors.push(format!(
                        "FLEET[{}].cruiser_count expected {}, got {}",
                        fleet_record_index_1_based,
                        expected_ca,
                        record.cruiser_count()
                    ));
                }
                if record.destroyer_count() != expected_dd {
                    errors.push(format!(
                        "FLEET[{}].destroyer_count expected {}, got {}",
                        fleet_record_index_1_based,
                        expected_dd,
                        record.destroyer_count()
                    ));
                }
                if record.etac_count() != expected_et {
                    errors.push(format!(
                        "FLEET[{}].etac_count expected {}, got {}",
                        fleet_record_index_1_based,
                        expected_et,
                        record.etac_count()
                    ));
                }
                if record.x_fraction() != Real48::HALF {
                    errors.push(format!(
                        "FLEET[{}].x_fraction expected 0.5, got {:?}",
                        fleet_record_index_1_based,
                        record.x_fraction().to_bytes()
                    ));
                }
                if record.y_fraction() != Real48::HALF {
                    errors.push(format!(
                        "FLEET[{}].y_fraction expected 0.5, got {:?}",
                        fleet_record_index_1_based,
                        record.y_fraction().to_bytes()
                    ));
                }
                if record.depth() != Real48::ONE {
                    errors.push(format!(
                        "FLEET[{}].depth expected 1.0, got {:?}",
                        fleet_record_index_1_based,
                        record.depth().to_bytes()
                    ));
                }
            }
        }

        errors
    }

    pub fn current_known_initialized_fleet_mission_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let player_count = self.conquest.player_count() as usize;
        let expected_fleet_count = player_count.saturating_mul(4);

        if self.fleets.records.len() != expected_fleet_count {
            return errors;
        }

        let homeworld_coords = self.player_homeworld_seed_coords_current_known();
        for block_idx in 0..player_count {
            let Some(expected_coords) = homeworld_coords.get(block_idx).and_then(|coords| *coords)
            else {
                continue;
            };

            for slot_idx in 0..4 {
                let fleet_record_index_1_based = block_idx * 4 + slot_idx + 1;
                let record = &self.fleets.records[fleet_record_index_1_based - 1];
                if record.standing_order_code_raw() != 5 {
                    errors.push(format!(
                        "FLEET[{}].standing_order expected 5 for initialized baseline, got {}",
                        fleet_record_index_1_based,
                        record.standing_order_code_raw()
                    ));
                }
                if record.standing_order_target_coords_raw() != expected_coords {
                    errors.push(format!(
                        "FLEET[{}].standing_order_target expected {:?} for initialized baseline, got {:?}",
                        fleet_record_index_1_based,
                        expected_coords,
                        record.standing_order_target_coords_raw()
                    ));
                }
                if record.mission_aux_bytes() != [1, 0] {
                    errors.push(format!(
                        "FLEET[{}].mission_aux expected [1, 0] for initialized baseline, got {:?}",
                        fleet_record_index_1_based,
                        record.mission_aux_bytes()
                    ));
                }
            }
        }

        errors
    }

    pub fn current_known_initialized_homeworld_alignment_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let player_count = self.conquest.player_count() as usize;
        let expected_fleet_count = player_count.saturating_mul(4);

        if self.fleets.records.len() != expected_fleet_count {
            return errors;
        }

        let homeworld_coords = self.player_homeworld_seed_coords_current_known();
        for block_idx in 0..player_count {
            let Some(expected_coords) = homeworld_coords.get(block_idx).and_then(|coords| *coords)
            else {
                continue;
            };
            let fleet = &self.fleets.records[block_idx * 4];
            let actual_loc = fleet.current_location_coords_raw();
            let actual_target = fleet.standing_order_target_coords_raw();
            if actual_loc != expected_coords {
                errors.push(format!(
                    "FLEET block {} location expected homeworld seed {:?}, got {:?}",
                    block_idx + 1,
                    expected_coords,
                    actual_loc
                ));
            }
            if actual_target != expected_coords {
                errors.push(format!(
                    "FLEET block {} target expected homeworld seed {:?}, got {:?}",
                    block_idx + 1,
                    expected_coords,
                    actual_target
                ));
            }
        }

        errors
    }

    pub fn fleet_order_errors_current_known(
        &self,
        record_index_1_based: usize,
        speed: u8,
        order_code: u8,
        target: [u8; 2],
        aux0: Option<u8>,
        aux1: Option<u8>,
    ) -> Vec<String> {
        let mut errors = Vec::new();
        match self.fleets.records.get(record_index_1_based - 1) {
            Some(record) => {
                if record.current_speed() != speed {
                    errors.push(format!(
                        "FLEET[{}].current_speed expected {}, got {}",
                        record_index_1_based,
                        speed,
                        record.current_speed()
                    ));
                }
                if record.standing_order_code_raw() != order_code {
                    errors.push(format!(
                        "FLEET[{}].order expected {:#04x}, got {:#04x}",
                        record_index_1_based,
                        order_code,
                        record.standing_order_code_raw()
                    ));
                }
                if record.standing_order_target_coords_raw() != target {
                    errors.push(format!(
                        "FLEET[{}].target expected ({}, {}), got {:?}",
                        record_index_1_based,
                        target[0],
                        target[1],
                        record.standing_order_target_coords_raw()
                    ));
                }
                let mission_aux = record.mission_aux_bytes();
                if let Some(value) = aux0 {
                    if mission_aux[0] != value {
                        errors.push(format!(
                            "FLEET[{}].aux0 expected {:#04x}, got {:#04x}",
                            record_index_1_based, value, mission_aux[0]
                        ));
                    }
                }
                if let Some(value) = aux1 {
                    if mission_aux[1] != value {
                        errors.push(format!(
                            "FLEET[{}].aux1 expected {:#04x}, got {:#04x}",
                            record_index_1_based, value, mission_aux[1]
                        ));
                    }
                }
            }
            None => errors.push(format!("FLEETS.DAT missing record {record_index_1_based}")),
        }
        errors
    }
}
