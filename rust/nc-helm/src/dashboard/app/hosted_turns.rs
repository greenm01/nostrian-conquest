use nc_data::{
    DiplomaticRelation, FleetDetachSelection, FleetTurnAction, FleetTurnBlock, PlanetTurnAction,
    PlanetTurnBlock, TurnDiplomacyDirective, TurnMessage, TurnSubmission,
};

use super::state::DashApp;

impl DashApp {
    pub(crate) fn initialize_hosted_turn_draft(&mut self) {
        self.hosted_turn_draft = Some(TurnSubmission {
            player_record_index_1_based: self.player_record_index_1_based,
            year: self.game_data.conquest.game_year(),
            tax_rate: None,
            diplomacy: Vec::new(),
            planets: Vec::new(),
            fleets: Vec::new(),
            messages: Vec::new(),
        });
    }

    pub(crate) fn stage_hosted_planet_build(
        &mut self,
        planet_record_index_1_based: usize,
        points_remaining_raw: u8,
        kind_raw: u8,
    ) {
        let Some(submission) = self.hosted_turn_draft.as_mut() else {
            return;
        };
        let actions = planet_actions_mut(submission, planet_record_index_1_based);
        actions.push(PlanetTurnAction::Build {
            points_remaining_raw,
            kind_raw,
        });
    }

    pub(crate) fn stage_hosted_tax_rate(&mut self, tax_rate: u8) {
        let Some(submission) = self.hosted_turn_draft.as_mut() else {
            return;
        };
        submission.tax_rate = Some(tax_rate);
    }

    pub(crate) fn stage_hosted_diplomacy(
        &mut self,
        to_empire_raw: u8,
        relation: DiplomaticRelation,
    ) {
        let Some(submission) = self.hosted_turn_draft.as_mut() else {
            return;
        };
        if let Some(existing) = submission
            .diplomacy
            .iter_mut()
            .find(|directive| directive.to_empire_raw == to_empire_raw)
        {
            existing.relation = relation;
        } else {
            submission.diplomacy.push(TurnDiplomacyDirective {
                to_empire_raw,
                relation,
            });
        }
    }

    pub(crate) fn stage_hosted_message(
        &mut self,
        recipient_empire_raw: u8,
        subject: String,
        body: String,
    ) {
        let Some(submission) = self.hosted_turn_draft.as_mut() else {
            return;
        };
        submission.messages.push(TurnMessage {
            recipient_empire_raw,
            subject,
            body,
        });
    }

    pub(crate) fn remove_hosted_message(&mut self, index: usize) -> Option<TurnMessage> {
        let submission = self.hosted_turn_draft.as_mut()?;
        (index < submission.messages.len()).then(|| submission.messages.remove(index))
    }

    pub(crate) fn stage_hosted_planet_clear_build_kind(
        &mut self,
        planet_record_index_1_based: usize,
        kind_raw: u8,
    ) {
        let Some(submission) = self.hosted_turn_draft.as_mut() else {
            return;
        };
        let actions = planet_actions_mut(submission, planet_record_index_1_based);
        actions.retain(|action| {
            !matches!(
                action,
                PlanetTurnAction::Build {
                    kind_raw: existing_kind_raw,
                    ..
                } if *existing_kind_raw == kind_raw
            ) && !matches!(
                action,
                PlanetTurnAction::RemoveBuild {
                    kind_raw: existing_kind_raw,
                    ..
                } if *existing_kind_raw == kind_raw
            ) && !matches!(
                action,
                PlanetTurnAction::ClearBuildKind {
                    kind_raw: existing_kind_raw,
                } if *existing_kind_raw == kind_raw
            )
        });
        actions.push(PlanetTurnAction::ClearBuildKind { kind_raw });
    }

    #[allow(dead_code)]
    pub(crate) fn stage_hosted_planet_clear_build_queue(
        &mut self,
        planet_record_index_1_based: usize,
    ) {
        let Some(submission) = self.hosted_turn_draft.as_mut() else {
            return;
        };
        let actions = planet_actions_mut(submission, planet_record_index_1_based);
        actions.retain(|action| {
            !matches!(
                action,
                PlanetTurnAction::ClearBuildQueue
                    | PlanetTurnAction::ClearBuildKind { .. }
                    | PlanetTurnAction::RemoveBuild { .. }
                    | PlanetTurnAction::Build { .. }
                    | PlanetTurnAction::Commission { .. }
            )
        });
        actions.insert(0, PlanetTurnAction::ClearBuildQueue);
    }

    pub(crate) fn stage_hosted_planet_remove_build(
        &mut self,
        planet_record_index_1_based: usize,
        qty: u16,
        kind_raw: u8,
    ) {
        let Some(submission) = self.hosted_turn_draft.as_mut() else {
            return;
        };
        let actions = planet_actions_mut(submission, planet_record_index_1_based);
        actions.push(PlanetTurnAction::RemoveBuild { qty, kind_raw });
    }

    pub(crate) fn stage_hosted_planet_commission(
        &mut self,
        planet_record_index_1_based: usize,
        slot_0_based: usize,
    ) {
        let Some(submission) = self.hosted_turn_draft.as_mut() else {
            return;
        };
        let actions = planet_actions_mut(submission, planet_record_index_1_based);
        actions.push(PlanetTurnAction::Commission { slot_0_based });
    }

    pub(crate) fn stage_hosted_planet_auto_commission(
        &mut self,
        planet_record_index_1_based: usize,
    ) {
        let Some(submission) = self.hosted_turn_draft.as_mut() else {
            return;
        };
        let actions = planet_actions_mut(submission, planet_record_index_1_based);
        if !actions
            .iter()
            .any(|action| matches!(action, PlanetTurnAction::AutoCommission))
        {
            actions.push(PlanetTurnAction::AutoCommission);
        }
    }

    pub(crate) fn stage_hosted_planet_scorch(&mut self, planet_record_index_1_based: usize) {
        let Some(submission) = self.hosted_turn_draft.as_mut() else {
            return;
        };
        let actions = planet_actions_mut(submission, planet_record_index_1_based);
        actions.retain(|action| !matches!(action, PlanetTurnAction::Scorch));
        actions.push(PlanetTurnAction::Scorch);
    }

    pub(crate) fn stage_hosted_fleet_roe(&mut self, fleet_record_index_1_based: usize, value: u8) {
        let Some(submission) = self.hosted_turn_draft.as_mut() else {
            return;
        };
        let actions = fleet_actions_mut(submission, fleet_record_index_1_based);
        upsert_fleet_action(actions, FleetTurnAction::RulesOfEngagement { value });
    }

    pub(crate) fn stage_hosted_fleet_order(
        &mut self,
        fleet_record_index_1_based: usize,
        speed: u8,
        order_code: u8,
        target: [u8; 2],
        aux0: Option<u8>,
        aux1: Option<u8>,
    ) {
        let Some(submission) = self.hosted_turn_draft.as_mut() else {
            return;
        };
        let actions = fleet_actions_mut(submission, fleet_record_index_1_based);
        actions.retain(|action| {
            !matches!(
                action,
                FleetTurnAction::Order { .. } | FleetTurnAction::Join { .. }
            )
        });
        actions.push(FleetTurnAction::Order {
            speed,
            order_code,
            target: Some(target),
            aux0,
            aux1,
        });
    }

    pub(crate) fn stage_hosted_fleet_join(
        &mut self,
        fleet_record_index_1_based: usize,
        host_fleet_record_index_1_based: usize,
    ) {
        let Some(submission) = self.hosted_turn_draft.as_mut() else {
            return;
        };
        let actions = fleet_actions_mut(submission, fleet_record_index_1_based);
        actions.retain(|action| {
            !matches!(
                action,
                FleetTurnAction::Order { .. } | FleetTurnAction::Join { .. }
            )
        });
        actions.push(FleetTurnAction::Join {
            host_fleet_record_index_1_based,
        });
    }

    pub(crate) fn stage_hosted_fleet_transfer(
        &mut self,
        donor_fleet_record_index_1_based: usize,
        host_fleet_record_index_1_based: usize,
        selection: FleetDetachSelection,
    ) {
        let Some(submission) = self.hosted_turn_draft.as_mut() else {
            return;
        };
        let actions = fleet_actions_mut(submission, donor_fleet_record_index_1_based);
        actions.push(FleetTurnAction::Transfer {
            host_fleet_record_index_1_based,
            selection,
        });
    }

    pub(crate) fn stage_hosted_fleet_load_armies(
        &mut self,
        fleet_record_index_1_based: usize,
        planet_record_index_1_based: usize,
        qty: u16,
    ) {
        let Some(submission) = self.hosted_turn_draft.as_mut() else {
            return;
        };
        let actions = fleet_actions_mut(submission, fleet_record_index_1_based);
        actions.push(FleetTurnAction::LoadArmies {
            planet_record_index_1_based,
            qty,
        });
    }

    pub(crate) fn stage_hosted_fleet_unload_armies(
        &mut self,
        fleet_record_index_1_based: usize,
        planet_record_index_1_based: usize,
        qty: u16,
    ) {
        let Some(submission) = self.hosted_turn_draft.as_mut() else {
            return;
        };
        let actions = fleet_actions_mut(submission, fleet_record_index_1_based);
        actions.push(FleetTurnAction::UnloadArmies {
            planet_record_index_1_based,
            qty,
        });
    }
}

fn planet_actions_mut(
    submission: &mut TurnSubmission,
    planet_record_index_1_based: usize,
) -> &mut Vec<PlanetTurnAction> {
    if let Some(index) = submission
        .planets
        .iter()
        .position(|planet| planet.planet_record_index_1_based == planet_record_index_1_based)
    {
        return &mut submission.planets[index].actions;
    }
    submission.planets.push(PlanetTurnBlock {
        planet_record_index_1_based,
        actions: Vec::new(),
    });
    &mut submission.planets.last_mut().expect("planet block").actions
}

fn fleet_actions_mut(
    submission: &mut TurnSubmission,
    fleet_record_index_1_based: usize,
) -> &mut Vec<FleetTurnAction> {
    if let Some(index) = submission
        .fleets
        .iter()
        .position(|fleet| fleet.fleet_record_index_1_based == fleet_record_index_1_based)
    {
        return &mut submission.fleets[index].actions;
    }
    submission.fleets.push(FleetTurnBlock {
        fleet_record_index_1_based,
        actions: Vec::new(),
    });
    &mut submission.fleets.last_mut().expect("fleet block").actions
}

fn upsert_fleet_action(actions: &mut Vec<FleetTurnAction>, replacement: FleetTurnAction) {
    match replacement {
        FleetTurnAction::RulesOfEngagement { .. } => {
            if let Some(index) = actions
                .iter()
                .position(|action| matches!(action, FleetTurnAction::RulesOfEngagement { .. }))
            {
                actions[index] = replacement;
            } else {
                actions.push(replacement);
            }
        }
        other => actions.push(other),
    }
}
