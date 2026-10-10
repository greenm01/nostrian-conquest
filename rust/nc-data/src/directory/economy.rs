use super::*;
use crate::{
    PlayerRecord, ProductionItemKind, build_capacity, yearly_growth_delta, yearly_tax_revenue,
};

impl CoreGameData {
    /// The bases player 1's PLAYER chain reaches.
    pub fn player1_starbase_count_current_known(&self) -> usize {
        self.base_chain(1).map_or(0, |chain| chain.len())
    }

    pub fn player1_owned_base_record_count_current_known(&self) -> usize {
        self.player_owned_base_record_count_current_known(1)
    }

    pub fn player_owned_planet_count_current_known(
        &self,
        player_record_index_1_based: usize,
    ) -> usize {
        self.planets
            .records
            .iter()
            .filter(|record| record.owner_empire_slot_raw() as usize == player_record_index_1_based)
            .count()
    }

    pub fn player_owned_base_record_count_current_known(
        &self,
        player_record_index_1_based: usize,
    ) -> usize {
        self.bases
            .records
            .iter()
            .filter(|record| record.owner_empire_raw() as usize == player_record_index_1_based)
            .count()
    }

    pub fn player_owned_fleet_count_current_known(
        &self,
        player_record_index_1_based: usize,
    ) -> usize {
        self.fleets
            .records
            .iter()
            .filter(|record| record.owner_empire_raw() as usize == player_record_index_1_based)
            .count()
    }

    pub fn player_ipbm_count_current_known(&self, player_record_index_1_based: usize) -> usize {
        self.player
            .records
            .get(player_record_index_1_based - 1)
            .map(|record| record.ipbm_count_raw() as usize)
            .unwrap_or(0)
    }

    pub fn player1_ipbm_count_current_known(&self) -> usize {
        self.player_ipbm_count_current_known(1)
    }

    pub fn empire_present_production(&self, player_record_index_1_based: usize) -> u16 {
        self.planets
            .records
            .iter()
            .filter(|record| record.owner_empire_slot_raw() as usize == player_record_index_1_based)
            .filter_map(|record| record.present_production_points())
            .sum()
    }

    pub fn empire_potential_production(&self, player_record_index_1_based: usize) -> u16 {
        self.planets
            .records
            .iter()
            .filter(|record| record.owner_empire_slot_raw() as usize == player_record_index_1_based)
            .map(|record| record.potential_production_points())
            .sum()
    }

    pub fn empire_available_production_points(&self, player_record_index_1_based: usize) -> u32 {
        let tax_rate = self
            .player
            .records
            .get(player_record_index_1_based - 1)
            .map(|record| record.tax_rate())
            .unwrap_or(0);

        self.planets
            .records
            .iter()
            .filter(|record| record.owner_empire_slot_raw() as usize == player_record_index_1_based)
            .filter_map(|record| {
                record
                    .present_production_points()
                    .map(|points| yearly_tax_revenue(points, tax_rate))
            })
            .sum()
    }

    pub fn empire_efficiency_percent(&self, player_record_index_1_based: usize) -> f64 {
        let potential = self.empire_potential_production(player_record_index_1_based);
        if potential == 0 {
            return 0.0;
        }
        let present = self.empire_present_production(player_record_index_1_based);
        (present as f64 / potential as f64) * 100.0
    }

    pub fn empire_current_fleets_and_bases_count_current_known(
        &self,
        player_record_index_1_based: usize,
    ) -> usize {
        self.player_owned_fleet_count_current_known(player_record_index_1_based)
            + self.player_owned_base_record_count_current_known(player_record_index_1_based)
    }

    pub fn empire_rank_by_planets_current_known(
        &self,
        player_record_index_1_based: usize,
    ) -> usize {
        let own = self.player_owned_planet_count_current_known(player_record_index_1_based);
        1 + (1..=self.player.records.len())
            .filter(|&idx| idx != player_record_index_1_based)
            .filter(|&idx| self.player_owned_planet_count_current_known(idx) > own)
            .count()
    }

    pub fn empire_rank_by_present_production(&self, player_record_index_1_based: usize) -> usize {
        let own = self.empire_present_production(player_record_index_1_based);
        1 + (1..=self.player.records.len())
            .filter(|&idx| idx != player_record_index_1_based)
            .filter(|&idx| self.empire_present_production(idx) > own)
            .count()
    }

    pub fn empire_economy_summary(
        &self,
        player_record_index_1_based: usize,
    ) -> EmpireEconomySummary {
        let tax_rate = self
            .player
            .records
            .get(player_record_index_1_based - 1)
            .map(|record| record.tax_rate())
            .unwrap_or(0);
        EmpireEconomySummary {
            owned_planets: self
                .player_owned_planet_count_current_known(player_record_index_1_based),
            present_production: self.empire_present_production(player_record_index_1_based),
            potential_production: self.empire_potential_production(player_record_index_1_based),
            total_available_points: self
                .empire_available_production_points(player_record_index_1_based),
            efficiency_percent: self.empire_efficiency_percent(player_record_index_1_based),
            rank_by_planets: self.empire_rank_by_planets_current_known(player_record_index_1_based),
            rank_by_present_production: self
                .empire_rank_by_present_production(player_record_index_1_based),
            tax_rate,
            max_fleets_and_bases: 500,
            current_fleets_and_bases: self
                .empire_current_fleets_and_bases_count_current_known(player_record_index_1_based),
        }
    }

    pub fn empire_active_duty_summary(
        &self,
        player_record_index_1_based: usize,
    ) -> EmpireUnitSummary {
        let empire_raw = player_record_index_1_based as u8;
        let mut summary = EmpireUnitSummary::default();

        for fleet in &self.fleets.records {
            if fleet.owner_empire_raw() != empire_raw {
                continue;
            }
            summary.destroyers += u32::from(fleet.destroyer_count());
            summary.cruisers += u32::from(fleet.cruiser_count());
            summary.battleships += u32::from(fleet.battleship_count());
            summary.scouts += u32::from(fleet.scout_count());
            summary.transports += u32::from(fleet.troop_transport_count());
            summary.etacs += u32::from(fleet.etac_count());
        }

        summary.starbases =
            self.player_owned_base_record_count_current_known(player_record_index_1_based) as u32;

        for planet in &self.planets.records {
            if planet.owner_empire_slot_raw() != empire_raw {
                continue;
            }
            summary.armies += u32::from(planet.army_count_raw());
            summary.ground_batteries += u32::from(planet.ground_batteries_raw());
        }

        summary
    }

    pub fn empire_stardock_summary(&self, player_record_index_1_based: usize) -> EmpireUnitSummary {
        let empire_raw = player_record_index_1_based as u8;
        let mut summary = EmpireUnitSummary::default();

        for planet in &self.planets.records {
            if planet.owner_empire_slot_raw() != empire_raw {
                continue;
            }
            for slot in 0..crate::STARDOCK_SLOT_COUNT {
                let count = u32::from(planet.stardock_count_raw(slot));
                if count == 0 {
                    continue;
                }
                match planet.stardock_item_kind_current_known(slot) {
                    ProductionItemKind::Destroyer => summary.destroyers += count,
                    ProductionItemKind::Cruiser => summary.cruisers += count,
                    ProductionItemKind::Battleship => summary.battleships += count,
                    ProductionItemKind::Scout => summary.scouts += count,
                    ProductionItemKind::Transport => summary.transports += count,
                    ProductionItemKind::Etac => summary.etacs += count,
                    ProductionItemKind::Starbase => summary.starbases += count,
                    ProductionItemKind::GroundBattery => summary.ground_batteries += count,
                    ProductionItemKind::Army => summary.armies += count,
                    ProductionItemKind::Unknown(_) => {}
                }
            }
        }

        summary
    }

    pub fn empire_production_ranking_rows(
        &self,
        sort: EmpireProductionRankingSort,
    ) -> Vec<EmpireProductionRankingRow> {
        let mut rows = self
            .player
            .records
            .iter()
            .enumerate()
            .map(|(idx, record)| EmpireProductionRankingRow {
                empire_id: (idx + 1) as u8,
                empire_name: empire_name_for_rankings(record),
                planets_owned: self.player_owned_planet_count_current_known(idx + 1),
                current_production: self.empire_present_production(idx + 1),
            })
            .collect::<Vec<_>>();

        match sort {
            EmpireProductionRankingSort::Id => rows.sort_by_key(|row| row.empire_id),
            EmpireProductionRankingSort::Production => {
                rows.sort_by(|left, right| {
                    right
                        .current_production
                        .cmp(&left.current_production)
                        .then_with(|| right.planets_owned.cmp(&left.planets_owned))
                        .then_with(|| left.empire_id.cmp(&right.empire_id))
                });
            }
            EmpireProductionRankingSort::NumberOfPlanets => {
                rows.sort_by(|left, right| {
                    right
                        .planets_owned
                        .cmp(&left.planets_owned)
                        .then_with(|| right.current_production.cmp(&left.current_production))
                        .then_with(|| left.empire_id.cmp(&right.empire_id))
                });
            }
        }

        rows
    }

    pub fn empire_planet_economy_rows(
        &self,
        player_record_index_1_based: usize,
    ) -> Vec<EmpirePlanetEconomyRow> {
        self.planets
            .records
            .iter()
            .enumerate()
            .filter(|(_, record)| {
                record.owner_empire_slot_raw() as usize == player_record_index_1_based
            })
            .filter_map(|(idx, record)| {
                let present_production = record.present_production_points()?;
                let tax_rate = self
                    .player
                    .records
                    .get(player_record_index_1_based - 1)
                    .map(|player| player.tax_rate())
                    .unwrap_or(0);
                let has_friendly_starbase = self.planet_has_friendly_starbase(
                    player_record_index_1_based as u8,
                    record.coords_raw(),
                );
                Some(EmpirePlanetEconomyRow {
                    planet_record_index_1_based: idx + 1,
                    coords: record.coords_raw(),
                    planet_name: record.status_or_name_summary(),
                    present_production,
                    potential_production: record.potential_production_points(),
                    stored_production_points: record.stored_production_points(),
                    yearly_tax_revenue: yearly_tax_revenue(present_production, tax_rate),
                    yearly_growth_delta: yearly_growth_delta(
                        present_production,
                        record.potential_production_points(),
                        tax_rate,
                        has_friendly_starbase,
                    ),
                    build_capacity: build_capacity(present_production, has_friendly_starbase),
                    has_friendly_starbase,
                    armies: record.army_count_raw(),
                    ground_batteries: record.ground_batteries_raw(),
                    is_homeworld_seed: record.is_homeworld_seed_ignoring_name(),
                })
            })
            .collect()
    }

    pub fn planet_has_friendly_starbase(&self, owner_empire_raw: u8, coords: [u8; 2]) -> bool {
        self.bases.records.iter().any(|base| {
            base.owner_empire_raw() == owner_empire_raw
                && base.coords_raw() == coords
                && base.base_id_raw() != 0
        })
    }

    pub fn empire_present_production_current_known(
        &self,
        player_record_index_1_based: usize,
    ) -> u16 {
        self.empire_present_production(player_record_index_1_based)
    }

    pub fn planet_record_index_at_coords(&self, coords: [u8; 2]) -> Option<usize> {
        self.planets
            .records
            .iter()
            .position(|planet| planet.coords_raw() == coords)
    }

    pub fn empire_potential_production_current_known(
        &self,
        player_record_index_1_based: usize,
    ) -> u16 {
        self.empire_potential_production(player_record_index_1_based)
    }

    pub fn empire_total_available_points_current_known(
        &self,
        player_record_index_1_based: usize,
    ) -> u32 {
        self.empire_available_production_points(player_record_index_1_based)
    }

    pub fn empire_efficiency_percent_current_known(
        &self,
        player_record_index_1_based: usize,
    ) -> f64 {
        self.empire_efficiency_percent(player_record_index_1_based)
    }

    pub fn empire_rank_by_present_production_current_known(
        &self,
        player_record_index_1_based: usize,
    ) -> usize {
        self.empire_rank_by_present_production(player_record_index_1_based)
    }

    pub fn empire_economy_summary_current_known(
        &self,
        player_record_index_1_based: usize,
    ) -> EmpireEconomySummary {
        self.empire_economy_summary(player_record_index_1_based)
    }

    pub fn empire_active_duty_summary_current_known(
        &self,
        player_record_index_1_based: usize,
    ) -> EmpireUnitSummary {
        self.empire_active_duty_summary(player_record_index_1_based)
    }

    pub fn empire_stardock_summary_current_known(
        &self,
        player_record_index_1_based: usize,
    ) -> EmpireUnitSummary {
        self.empire_stardock_summary(player_record_index_1_based)
    }

    pub fn empire_production_ranking_rows_current_known(
        &self,
        sort: EmpireProductionRankingSort,
    ) -> Vec<EmpireProductionRankingRow> {
        self.empire_production_ranking_rows(sort)
    }

    pub fn empire_campaign_state(&self, empire_raw: u8) -> Option<CampaignState> {
        let player_idx = empire_raw.checked_sub(1)? as usize;
        let player = self.player.records.get(player_idx)?;

        match player.owner_mode_raw() {
            0x00 => return Some(CampaignState::CivilDisorder),
            0xff => return Some(CampaignState::Rogue),
            0x01 => {}
            _ => {}
        }

        let owned_planets = self
            .planets
            .records
            .iter()
            .filter(|planet| planet.owner_empire_slot_raw() == empire_raw)
            .count();
        if owned_planets > 0 {
            return Some(CampaignState::Stable);
        }

        let mut has_any_fleet_presence = false;
        let mut can_recover_planet = false;

        for fleet in &self.fleets.records {
            if fleet.owner_empire_raw() != empire_raw {
                continue;
            }

            let has_presence = fleet.scout_count() > 0
                || fleet.battleship_count() > 0
                || fleet.cruiser_count() > 0
                || fleet.destroyer_count() > 0
                || fleet.troop_transport_count() > 0
                || fleet.army_count() > 0
                || fleet.etac_count() > 0;
            if !has_presence {
                continue;
            }
            has_any_fleet_presence = true;

            if fleet.etac_count() > 0
                || (fleet.troop_transport_count() > 0 && fleet.army_count() > 0)
            {
                can_recover_planet = true;
                break;
            }
        }

        if can_recover_planet {
            Some(CampaignState::MarginalExistence)
        } else if has_any_fleet_presence {
            Some(CampaignState::DefectionRisk)
        } else {
            Some(CampaignState::Defeated)
        }
    }

    pub fn campaign_contenders(&self) -> Vec<u8> {
        (1..=self.player.records.len() as u8)
            .filter(|&empire_raw| crate::is_campaign_contender(self, empire_raw, &[]))
            .collect()
    }

    pub fn sole_contender(&self) -> Option<u8> {
        let contenders = self.campaign_contenders();
        if contenders.len() == 1 {
            contenders.first().copied()
        } else {
            None
        }
    }

    pub fn campaign_outlook(&self) -> CampaignOutlook {
        match self.sole_contender() {
            Some(empire_raw) => CampaignOutlook::SoleContender(empire_raw),
            None => CampaignOutlook::Contested,
        }
    }

    pub fn campaign_outcome(&self) -> CampaignOutcome {
        match self.sole_contender() {
            Some(empire_raw)
                if matches!(
                    self.empire_campaign_state(empire_raw),
                    Some(CampaignState::Stable)
                ) =>
            {
                CampaignOutcome::RecognizedEmperor(empire_raw)
            }
            _ => CampaignOutcome::Ongoing,
        }
    }
}

fn empire_name_for_rankings(record: &PlayerRecord) -> String {
    let empire = record.controlled_empire_name_summary();
    if !empire.is_empty() {
        return empire;
    }

    let legacy = record.legacy_status_name_summary();
    if !legacy.is_empty() {
        legacy
    } else {
        "Unknown".to_string()
    }
}
