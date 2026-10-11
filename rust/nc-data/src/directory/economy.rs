use super::*;
use crate::{
    Planet, Player, ProductionItemKind, Real48, build_capacity, is_homeworld_seed_ignoring_name,
    next_pass, present_production_points, stardock_slot_kind, stored_production_points,
    troop_transports, whole_points,
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
            .iter()
            .filter(|record| record.owner.0 as usize == player_record_index_1_based)
            .count()
    }

    pub fn player_owned_base_record_count_current_known(
        &self,
        player_record_index_1_based: usize,
    ) -> usize {
        self.bases
            .iter()
            .filter(|record| record.owner.0 as usize == player_record_index_1_based)
            .count()
    }

    pub fn player_owned_fleet_count_current_known(
        &self,
        player_record_index_1_based: usize,
    ) -> usize {
        self.fleets
            .iter()
            .filter(|record| record.owner.0 as usize == player_record_index_1_based)
            .count()
    }

    pub fn player_ipbm_count_current_known(&self, player_record_index_1_based: usize) -> usize {
        self.players
            .get(player_record_index_1_based - 1)
            .map(|record| record.ipbms_first.0 as usize)
            .unwrap_or(0)
    }

    pub fn player1_ipbm_count_current_known(&self) -> usize {
        self.player_ipbm_count_current_known(1)
    }

    pub fn empire_present_production(&self, player_record_index_1_based: usize) -> u16 {
        self.planets
            .iter()
            .filter(|record| record.owner.0 as usize == player_record_index_1_based)
            .map(present_production_points)
            .sum()
    }

    pub fn empire_potential_production(&self, player_record_index_1_based: usize) -> u16 {
        self.planets
            .iter()
            .filter(|record| record.owner.0 as usize == player_record_index_1_based)
            .map(|record| u16::from(record.potential_production))
            .sum()
    }

    /// Empire Revenue: next pass's tax revenue over the empire's worlds,
    /// each as the whole number its screens show.
    pub fn empire_available_production_points(&self, player_record_index_1_based: usize) -> u32 {
        self.planets
            .iter()
            .filter(|record| record.owner.0 as usize == player_record_index_1_based)
            .map(|record| self.planet_forecast_revenue(record))
            .sum()
    }

    /// Next pass's tax revenue at `planet`, assuming its owner's tax rate and
    /// starbase stay as they are, to the nearest whole point.
    pub fn planet_forecast_revenue(&self, planet: &Planet) -> u32 {
        let (tax_rate, has_starbase) = self.planet_forecast_inputs(planet);
        whole_points(next_pass(planet, tax_rate, has_starbase).revenue)
    }

    fn planet_forecast_inputs(&self, planet: &Planet) -> (u8, bool) {
        let owner = planet.owner.0;
        let tax_rate = self
            .players
            .get(usize::from(owner).saturating_sub(1))
            .map_or(0, |player| player.tax_rate);
        (
            tax_rate,
            self.planet_has_friendly_starbase(owner, planet.sector.to_array()),
        )
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
        1 + (1..=self.players.len())
            .filter(|&idx| idx != player_record_index_1_based)
            .filter(|&idx| self.player_owned_planet_count_current_known(idx) > own)
            .count()
    }

    pub fn empire_rank_by_present_production(&self, player_record_index_1_based: usize) -> usize {
        let own = self.empire_present_production(player_record_index_1_based);
        1 + (1..=self.players.len())
            .filter(|&idx| idx != player_record_index_1_based)
            .filter(|&idx| self.empire_present_production(idx) > own)
            .count()
    }

    pub fn empire_economy_summary(
        &self,
        player_record_index_1_based: usize,
    ) -> EmpireEconomySummary {
        let tax_rate = self
            .players
            .get(player_record_index_1_based - 1)
            .map(|record| record.tax_rate)
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
        let empire = player_record_index_1_based as u8;
        let mut summary = EmpireUnitSummary::default();

        for fleet in &self.fleets {
            if fleet.owner.0 != empire {
                continue;
            }
            summary.destroyers += u32::from(fleet.ships.destroyers);
            summary.cruisers += u32::from(fleet.ships.cruisers);
            summary.battleships += u32::from(fleet.ships.battleships);
            summary.scouts += u32::from(fleet.ships.scouts);
            summary.transports += u32::from(troop_transports(fleet));
            summary.etacs += u32::from(fleet.ships.etacs);
        }

        summary.starbases =
            self.player_owned_base_record_count_current_known(player_record_index_1_based) as u32;

        for planet in &self.planets {
            if planet.owner.0 != empire {
                continue;
            }
            summary.armies += u32::from(u8::try_from(planet.armies).unwrap_or(u8::MAX));
            summary.ground_batteries +=
                u32::from(u8::try_from(planet.ground_batteries).unwrap_or(u8::MAX));
        }

        summary
    }

    pub fn empire_stardock_summary(&self, player_record_index_1_based: usize) -> EmpireUnitSummary {
        let empire = player_record_index_1_based as u8;
        let mut summary = EmpireUnitSummary::default();

        for planet in &self.planets {
            if planet.owner.0 != empire {
                continue;
            }
            for slot in 0..crate::STARDOCK_SLOT_COUNT {
                let count = u32::from(u16::try_from(planet.stardock[slot]).unwrap_or(u16::MAX));
                if count == 0 {
                    continue;
                }
                match stardock_slot_kind(planet, slot) {
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
            .players
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
            .iter()
            .enumerate()
            .filter(|(_, record)| record.owner.0 as usize == player_record_index_1_based)
            .map(|(idx, record)| {
                let present_production = present_production_points(record);
                let (tax_rate, has_friendly_starbase) = self.planet_forecast_inputs(record);
                let forecast = next_pass(record, tax_rate, has_friendly_starbase);
                EmpirePlanetEconomyRow {
                    planet_record_index_1_based: idx + 1,
                    coords: record.sector.to_array(),
                    planet_name: record.name.trim().to_string(),
                    present_production,
                    potential_production: u16::from(record.potential_production),
                    stored_production_points: stored_production_points(record),
                    yearly_tax_revenue: whole_points(forecast.revenue),
                    yearly_growth_delta: whole_points(forecast.growth).min(u32::from(u16::MAX))
                        as u16,
                    build_capacity: build_capacity(present_production, has_friendly_starbase),
                    has_friendly_starbase,
                    armies: u8::try_from(record.armies).unwrap_or(u8::MAX),
                    ground_batteries: u8::try_from(record.ground_batteries).unwrap_or(u8::MAX),
                    is_homeworld_seed: is_homeworld_seed_ignoring_name(record),
                }
            })
            .collect()
    }

    /// Whether one of the empire's bases orbits the world at `coords` and is
    /// bound for it, as ECGAME's build screen tests for the 5x budget: in
    /// the world's sector at depth 0.5 or more, with that sector as its
    /// target and orbit as its target layer.
    pub fn planet_has_friendly_starbase(&self, owner_empire: u8, coords: [u8; 2]) -> bool {
        self.bases.iter().any(|base| {
            base.owner.0 == owner_empire
                && base.sector.to_array() == coords
                && base.base_id.0 != 0
                && base.depth >= Real48::HALF
                && base.target.to_array() == coords
                && base.target_depth >= 1
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
            .iter()
            .position(|planet| planet.sector.to_array() == coords)
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

    pub fn empire_campaign_state(&self, empire: u8) -> Option<CampaignState> {
        let player_idx = empire.checked_sub(1)? as usize;
        let player = self.players.get(player_idx)?;

        match player.mode.code() {
            0x00 => return Some(CampaignState::CivilDisorder),
            0xff => return Some(CampaignState::Rogue),
            0x01 => {}
            _ => {}
        }

        let owned_planets = self
            .planets
            .iter()
            .filter(|planet| planet.owner.0 == empire)
            .count();
        if owned_planets > 0 {
            return Some(CampaignState::Stable);
        }

        let mut has_any_fleet_presence = false;
        let mut can_recover_planet = false;

        for fleet in &self.fleets {
            if fleet.owner.0 != empire {
                continue;
            }

            let has_presence = fleet.ships.scouts > 0
                || fleet.ships.battleships > 0
                || fleet.ships.cruisers > 0
                || fleet.ships.destroyers > 0
                || troop_transports(fleet) > 0
                || fleet.ships.loaded_transports > 0
                || fleet.ships.etacs > 0;
            if !has_presence {
                continue;
            }
            has_any_fleet_presence = true;

            if fleet.ships.etacs > 0
                || (troop_transports(fleet) > 0 && fleet.ships.loaded_transports > 0)
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
        (1..=self.players.len() as u8)
            .filter(|&empire| crate::is_campaign_contender(self, empire, &[]))
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
            Some(empire) => CampaignOutlook::SoleContender(empire),
            None => CampaignOutlook::Contested,
        }
    }

    pub fn campaign_outcome(&self) -> CampaignOutcome {
        match self.sole_contender() {
            Some(empire)
                if matches!(
                    self.empire_campaign_state(empire),
                    Some(CampaignState::Stable)
                ) =>
            {
                CampaignOutcome::RecognizedEmperor(empire)
            }
            _ => CampaignOutcome::Ongoing,
        }
    }
}

fn empire_name_for_rankings(record: &Player) -> String {
    let empire = record.empire_name.trim().to_string();
    if !empire.is_empty() {
        return empire;
    }

    let legacy = record.empire_name.trim().to_string();
    if !legacy.is_empty() {
        legacy
    } else {
        "Unknown".to_string()
    }
}
