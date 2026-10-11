use super::*;

impl CoreGameData {
    pub fn ecmaint_structural_preflight_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        // CONQUEST header: year in valid range, player_count consistent
        errors.extend(self.conquest_header_errors());

        // Planet/base owner bounds and base chains ECMAINT cannot load.
        errors.extend(self.current_known_planet_owner_slot_errors());
        errors.extend(self.current_known_base_owner_empire_errors());
        errors.extend(self.base_chain_errors());

        errors
    }

    pub fn ecmaint_preflight_errors(&self) -> Vec<String> {
        let mut errors = self.ecmaint_structural_preflight_errors();

        // SETUP header: version tag check
        errors.extend(self.setup_header_errors());

        // Player/planet table lengths
        errors.extend(self.record_count_errors());

        // Every live base hangs off its owner's PLAYER chain
        errors.extend(self.unchained_base_errors());

        // PLAYER ipbm_count ↔ IPBM.DAT length
        errors.extend(self.ipbm_count_length_errors_current_known());

        // Fleet owner validation
        errors.extend(self.fleet_owner_errors());

        // Fleet block structure (for initialized scenarios)
        errors.extend(self.current_known_initialized_fleet_block_errors());

        // Planet owner bounds
        errors.extend(self.current_known_planet_owner_slot_errors());

        // Base owner bounds
        errors.extend(self.current_known_base_owner_empire_errors());

        errors
    }

    /// Validate CONQUEST.DAT header fields.
    /// ECMAINT checks: year in valid range, player_count plausible
    fn conquest_header_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let year = self.conquest.game_year;

        // Year should be in a reasonable range (3000-3100 based on game context)
        if year < 3000 || year > 3100 {
            errors.push(format!(
                "CONQUEST.DAT.game_year {} out of expected range (3000-3100)",
                year
            ));
        }

        let player_count = self.conquest.player_count;
        if player_count == 0 || player_count > 25 {
            errors.push(format!(
                "CONQUEST.DAT.player_count {} out of range (1-25)",
                player_count
            ));
        }

        errors
    }

    fn record_count_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let player_count = self.conquest.player_count as usize;
        let expected_planets = player_count.saturating_mul(5);

        if self.players.len() != player_count {
            errors.push(format!(
                "PLAYER.DAT record count expected {}, got {}",
                player_count,
                self.players.len()
            ));
        }
        if self.planets.len() != expected_planets {
            errors.push(format!(
                "PLANETS.DAT record count expected {}, got {}",
                expected_planets,
                self.planets.len()
            ));
        }

        errors
    }

    /// Validate SETUP.DAT header.
    /// ECMAINT checks: version tag matches expected
    fn setup_header_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.setup.version_tag != *b"EC151" {
            errors.push(format!(
                "SETUP.DAT.version_tag expected EC151, got {:?}",
                String::from_utf8_lossy(self.setup.version_tag.as_slice())
            ));
        }

        errors
    }

    /// Validate fleet owner bytes match expected player indices.
    /// ECMAINT at 2000:6040..6368: validates fleet owner bytes against player index
    fn fleet_owner_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        // For each fleet block (player_count * 4 records)
        let player_count = self.conquest.player_count as usize;
        let expected_fleets = player_count * 4;

        for (fleet_idx, fleet) in self.fleets.iter().enumerate() {
            let owner = fleet.owner.0 as usize;

            // Determine expected owner from fleet index
            let expected_owner = if fleet_idx < expected_fleets {
                (fleet_idx / 4) + 1
            } else {
                0 // Extra fleets should have owner 0
            };

            if owner != expected_owner && owner != 0 {
                errors.push(format!(
                    "FLEET[{}].owner_empire expected {} or 0, got {}",
                    fleet_idx, expected_owner, owner
                ));
            }
        }

        errors
    }
}
