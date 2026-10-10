use super::*;

impl CoreGameData {
    pub fn ecmaint_structural_preflight_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        // CONQUEST header: year in valid range, player_count consistent
        errors.extend(self.conquest_header_errors());

        // Planet/base owner bounds and fatal base-link words.
        errors.extend(self.current_known_planet_owner_slot_errors());
        errors.extend(self.current_known_base_owner_empire_errors());
        errors.extend(self.base_link_word_errors());

        errors
    }

    pub fn ecmaint_preflight_errors(&self) -> Vec<String> {
        let mut errors = self.ecmaint_structural_preflight_errors();

        // SETUP header: version tag check
        errors.extend(self.setup_header_errors());

        // Player/planet table lengths
        errors.extend(self.record_count_errors());

        // PLAYER starbase_count ↔ BASES.DAT linkage
        errors.extend(self.player_starbase_bases_linkage_errors());

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

        // Base link word validity
        errors.extend(self.base_link_word_errors());

        errors
    }

    /// Validate CONQUEST.DAT header fields.
    /// ECMAINT checks: year in valid range, player_count plausible
    fn conquest_header_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let year = self.conquest.game_year();

        // Year should be in a reasonable range (3000-3100 based on game context)
        if year < 3000 || year > 3100 {
            errors.push(format!(
                "CONQUEST.DAT.game_year {} out of expected range (3000-3100)",
                year
            ));
        }

        let player_count = self.conquest.player_count();
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
        let player_count = self.conquest.player_count() as usize;
        let expected_planets = player_count.saturating_mul(5);

        if self.player.records.len() != player_count {
            errors.push(format!(
                "PLAYER.DAT record count expected {}, got {}",
                player_count,
                self.player.records.len()
            ));
        }
        if self.planets.records.len() != expected_planets {
            errors.push(format!(
                "PLANETS.DAT record count expected {}, got {}",
                expected_planets,
                self.planets.records.len()
            ));
        }

        errors
    }

    /// Validate SETUP.DAT header.
    /// ECMAINT checks: version tag matches expected
    fn setup_header_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.setup.version_tag() != b"EC151" {
            errors.push(format!(
                "SETUP.DAT.version_tag expected EC151, got {:?}",
                String::from_utf8_lossy(self.setup.version_tag())
            ));
        }

        errors
    }

    /// Validate PLAYER starbase_count matches BASES.DAT records.
    /// ECMAINT at 2000:5EE4: PLAYER[0x44] used as base record selector
    fn player_starbase_bases_linkage_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        for (player_idx, player) in self.player.records.iter().enumerate() {
            let expected_count = player.starbase_count_raw() as usize;
            // Count actual bases owned by this player
            let actual_count = self
                .bases
                .records
                .iter()
                .filter(|b| b.owner_empire_raw() == (player_idx + 1) as u8)
                .count();

            if actual_count != expected_count {
                errors.push(format!(
                    "PLAYER[{}].starbase_count ({}) doesn't match owned BASES records ({})",
                    player_idx + 1,
                    expected_count,
                    actual_count
                ));
            }
        }

        errors
    }

    /// Validate fleet owner bytes match expected player indices.
    /// ECMAINT at 2000:6040..6368: validates fleet owner bytes against player index
    fn fleet_owner_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        // For each fleet block (player_count * 4 records)
        let player_count = self.conquest.player_count() as usize;
        let expected_fleets = player_count * 4;

        for (fleet_idx, fleet) in self.fleets.records.iter().enumerate() {
            let owner = fleet.owner_empire_raw() as usize;

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

    /// Validate BASES.DAT link words (offset 0x05..0x06).
    /// ECMAINT: BASES[0x05..0x06] = 0x0001 or 0x0101 triggers abort
    fn base_link_word_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        for (base_idx, base) in self.bases.records.iter().enumerate() {
            let link_word = base.next_base_link_raw();

            // Dangerous patterns that trigger ECMAINT abort
            if link_word == 0x0001 || link_word == 0x0101 {
                errors.push(format!(
                    "BASES[{}].link_word = 0x{:04X} triggers ECMAINT integrity abort",
                    base_idx, link_word
                ));
            }
        }

        errors
    }
}
