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
        let player1 = self.player.records.first();
        let fleet1 = self.fleets.records.first();
        let base1 = self.bases.records.first();

        CurrentKnownKeyWordSummary {
            player_starbase_count: player1
                .map(|record| record.starbase_count_raw())
                .unwrap_or(0),
            player_ipbm_count: player1.map(|record| record.ipbm_count_raw()).unwrap_or(0),
            fleet1_local_slot: fleet1.map(|record| record.local_slot_word_raw()),
            fleet1_id: fleet1.map(|record| record.fleet_id_word_raw()),
            fleet1_guard_index: fleet1.map(|record| record.guard_starbase_index_raw()),
            fleet1_target: fleet1.map(|record| record.standing_order_target_coords_raw()),
            base1_summary: base1.map(|record| record.summary_word_raw()),
            base1_id: base1.map(|record| record.base_id_raw()),
            base1_chain: base1.map(|record| record.chain_word_raw()),
            base1_coords: base1.map(|record| record.coords_raw()),
            ipbm_record_count: self.ipbm.records.len(),
        }
    }

    pub fn current_known_baseline_diff_counts(&self) -> Vec<CoreFileDiffCount> {
        let mut normalized = self.clone();
        normalized.sync_current_known_initialized_post_maint_baseline();
        self.diff_counts_against(&normalized)
    }

    pub fn current_known_baseline_diff_offsets(&self) -> Vec<CoreFileDiffOffsets> {
        let mut normalized = self.clone();
        normalized.sync_current_known_initialized_post_maint_baseline();
        self.diff_offsets_against(&normalized)
    }

    pub fn diff_counts_against(&self, other: &Self) -> Vec<CoreFileDiffCount> {
        [
            (
                "PLAYER.DAT",
                self.player.to_bytes(),
                other.player.to_bytes(),
            ),
            (
                "PLANETS.DAT",
                self.planets.to_bytes(),
                other.planets.to_bytes(),
            ),
            (
                "FLEETS.DAT",
                self.fleets.to_bytes(),
                other.fleets.to_bytes(),
            ),
            ("BASES.DAT", self.bases.to_bytes(), other.bases.to_bytes()),
            ("IPBM.DAT", self.ipbm.to_bytes(), other.ipbm.to_bytes()),
            ("SETUP.DAT", self.setup.to_bytes(), other.setup.to_bytes()),
            (
                "CONQUEST.DAT",
                self.conquest.to_bytes(),
                other.conquest.to_bytes(),
            ),
        ]
        .into_iter()
        .map(|(name, current, other)| CoreFileDiffCount {
            name,
            differing_bytes: byte_diff_count(&current, &other),
        })
        .collect()
    }

    pub fn diff_offsets_against(&self, other: &Self) -> Vec<CoreFileDiffOffsets> {
        [
            (
                "PLAYER.DAT",
                self.player.to_bytes(),
                other.player.to_bytes(),
            ),
            (
                "PLANETS.DAT",
                self.planets.to_bytes(),
                other.planets.to_bytes(),
            ),
            (
                "FLEETS.DAT",
                self.fleets.to_bytes(),
                other.fleets.to_bytes(),
            ),
            ("BASES.DAT", self.bases.to_bytes(), other.bases.to_bytes()),
            ("IPBM.DAT", self.ipbm.to_bytes(), other.ipbm.to_bytes()),
            ("SETUP.DAT", self.setup.to_bytes(), other.setup.to_bytes()),
            (
                "CONQUEST.DAT",
                self.conquest.to_bytes(),
                other.conquest.to_bytes(),
            ),
        ]
        .into_iter()
        .map(|(name, current, other)| CoreFileDiffOffsets {
            name,
            differing_offsets: byte_diff_offsets(&current, &other),
        })
        .collect()
    }

    pub fn exact_match_errors_against(&self, other: &Self, label: &str) -> Vec<String> {
        self.diff_counts_against(other)
            .into_iter()
            .filter(|diff| diff.differing_bytes != 0)
            .map(|diff| {
                format!(
                    "{} differs by {} bytes from {}",
                    diff.name, diff.differing_bytes, label
                )
            })
            .collect()
    }
}

fn byte_diff_count(left: &[u8], right: &[u8]) -> usize {
    left.iter()
        .zip(right.iter())
        .filter(|(a, b)| a != b)
        .count()
        + left.len().abs_diff(right.len())
}

fn byte_diff_offsets(left: &[u8], right: &[u8]) -> Vec<usize> {
    let shared_len = left.len().min(right.len());
    let mut offsets: Vec<usize> = left[..shared_len]
        .iter()
        .zip(right[..shared_len].iter())
        .enumerate()
        .filter_map(|(idx, (a, b))| (a != b).then_some(idx))
        .collect();

    let extra_len = left.len().max(right.len());
    offsets.extend(shared_len..extra_len);
    offsets
}
