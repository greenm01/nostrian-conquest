//! Building one empire's turn through the real submission path.
//!
//! Each proposed action is rendered to turn KDL, parsed back, and applied
//! with everything accepted so far to a copy of the turn-start state, exactly
//! as `submit-turn` would apply the finished file. Rejected actions are
//! dropped, so the finished submission is always one the game accepts. An
//! action that panics the submission path is kept and ends the turn, so the
//! real submission reproduces the panic.

use std::panic::{AssertUnwindSafe, catch_unwind};

use nc_data::{
    CoreGameData, FleetTurnAction, FleetTurnBlock, PlanetTurnAction, PlanetTurnBlock,
    TurnDiplomacyDirective, TurnSubmission,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnAction {
    Tax(u8),
    Diplomacy(TurnDiplomacyDirective),
    /// 1-based planet record and one action on it.
    Planet(usize, PlanetTurnAction),
    /// 1-based fleet record and one action on it.
    Fleet(usize, FleetTurnAction),
}

pub struct TurnBuilder {
    start: CoreGameData,
    /// Turn-start state with every accepted action applied.
    working: CoreGameData,
    accepted: TurnSubmission,
    /// Actions whose KDL did not parse back to the same submission.
    pub lossy_round_trips: Vec<String>,
    /// The last accepted action panicked the submission path.
    pub panicked: bool,
}

impl TurnBuilder {
    pub fn new(start: &CoreGameData, empire_raw: u8) -> Self {
        Self {
            start: start.clone(),
            working: start.clone(),
            accepted: TurnSubmission {
                player_record_index_1_based: usize::from(empire_raw),
                year: start.conquest.game_year(),
                tax_rate: None,
                diplomacy: Vec::new(),
                planets: Vec::new(),
                fleets: Vec::new(),
                messages: Vec::new(),
            },
            lossy_round_trips: Vec::new(),
            panicked: false,
        }
    }

    /// The state the next action should be chosen against.
    pub fn state(&self) -> &CoreGameData {
        &self.working
    }

    /// Submit `action` with everything accepted so far; keep it when the
    /// game accepts the combined turn.
    pub fn try_add(&mut self, action: TurnAction) -> bool {
        let mut trial = self.accepted.clone();
        match action {
            TurnAction::Tax(rate) => trial.tax_rate = Some(rate),
            TurnAction::Diplomacy(directive) => trial.diplomacy.push(directive),
            TurnAction::Planet(record, action) => trial.planets.push(PlanetTurnBlock {
                planet_record_index_1_based: record,
                actions: vec![action],
            }),
            TurnAction::Fleet(record, action) => trial.fleets.push(FleetTurnBlock {
                fleet_record_index_1_based: record,
                actions: vec![action],
            }),
        }
        let kdl = trial.to_kdl_string();
        let Ok(parsed) = TurnSubmission::parse_kdl_str(&kdl) else {
            return false;
        };
        if parsed != trial {
            self.lossy_round_trips.push(kdl);
            return false;
        }
        let mut state = self.start.clone();
        match catch_unwind(AssertUnwindSafe(|| {
            parsed.apply_to(&mut state, &mut Vec::new())
        })) {
            Ok(Ok(_)) => {
                self.accepted = parsed;
                self.working = state;
                true
            }
            Ok(Err(_)) => false,
            Err(_) => {
                self.accepted = parsed;
                self.panicked = true;
                false
            }
        }
    }

    pub fn finish(self) -> (TurnSubmission, Vec<String>) {
        (self.accepted, self.lossy_round_trips)
    }
}

pub fn is_empty_submission(submission: &TurnSubmission) -> bool {
    submission.tax_rate.is_none()
        && submission.diplomacy.is_empty()
        && submission.planets.is_empty()
        && submission.fleets.is_empty()
        && submission.messages.is_empty()
}
