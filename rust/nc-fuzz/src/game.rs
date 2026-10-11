//! One fuzz case: draw a game, play it turn by turn through the harness,
//! and check the invariants after every maintenance turn.

use std::panic::{AssertUnwindSafe, catch_unwind};

use nc_data::{CoreGameData, MaintenanceEvents, ReportBlockRow, TurnSubmission};
use nc_engine::{
    apply_results_reviewable_flags, build_results_report_blocks, validate_maintenance_state,
};
use nc_harness::{ScenarioPlay, build_scenario};

use crate::choice::ChoiceSource;
use crate::invariants::{TurnObservation, Violation, check_submitted_spending, check_turn};
use crate::policy::plan_turn;
use crate::setup::{CaseSetup, FuzzConfig};
use crate::storage::check_round_trip;
use crate::turn::is_empty_submission;

#[derive(Debug, Clone)]
pub struct CaseRun {
    pub setup: CaseSetup,
    /// Accepted submissions for each turn played, in submission order.
    pub turns: Vec<Vec<TurnSubmission>>,
    pub violations: Vec<Violation>,
    /// State after the last turn played.
    pub final_state: CoreGameData,
}

impl CaseRun {
    /// Turns played and turn actions submitted: the size of the scenario.
    pub fn size(&self) -> (usize, usize) {
        let actions = self
            .turns
            .iter()
            .flatten()
            .map(|submission| {
                usize::from(submission.tax_rate.is_some())
                    + submission.diplomacy.len()
                    + submission
                        .planets
                        .iter()
                        .map(|block| block.actions.len())
                        .sum::<usize>()
                    + submission
                        .fleets
                        .iter()
                        .map(|block| block.actions.len())
                        .sum::<usize>()
            })
            .sum();
        (self.turns.len(), actions)
    }
}

pub fn run_case(choices: &mut dyn ChoiceSource, config: &FuzzConfig) -> CaseRun {
    let setup = CaseSetup::draw(choices, config);
    choices.note(&setup.summary());
    let built = build_scenario(&setup.scenario_spec()).expect("fuzz setup should build");
    let mut play = ScenarioPlay::new(built);
    let mut run = CaseRun {
        setup,
        turns: Vec::new(),
        violations: Vec::new(),
        final_state: play.built.game_data.clone(),
    };

    for turn in 1..=run.setup.turns {
        let turn_start = play.built.game_data.clone();
        let mut submissions = Vec::new();
        for empire in 1..=run.setup.player_count {
            let planned = plan_turn(
                run.setup.policy(empire),
                &play.built.game_data,
                empire,
                choices,
            );
            run.violations.extend(
                planned
                    .lossy_round_trips
                    .into_iter()
                    .map(|kdl| Violation::TurnKdlRoundTrip { empire, kdl }),
            );
            if is_empty_submission(&planned.submission) {
                continue;
            }
            match catch_unwind(AssertUnwindSafe(|| play.submit_turn(&planned.submission))) {
                Ok(Ok(_)) => {}
                Ok(Err(err)) => run.violations.push(Violation::SubmissionRejected {
                    empire,
                    error: err.to_string(),
                }),
                Err(panic) => run.violations.push(Violation::SubmissionPanic {
                    empire,
                    message: panic_message(&panic),
                }),
            }
            submissions.push(planned.submission);
        }
        run.turns.push(submissions);
        run.violations
            .extend(check_submitted_spending(&turn_start, &play.built.game_data));
        if let Err(err) = validate_maintenance_state(&play.built.game_data) {
            run.violations.push(Violation::InvalidAfterSubmission {
                issues: err.issues().to_vec(),
            });
        }
        if !run.violations.is_empty() {
            break;
        }

        let checked = match maintain_and_check(&mut play) {
            Ok(checked) => checked,
            Err(violation) => {
                run.violations.push(violation);
                break;
            }
        };
        run.violations.extend(checked.violations);
        choices.note(&turn_note(turn, &run, &checked.events));

        if !run.violations.is_empty() {
            break;
        }
        if play.winner_state.winner_empire.is_some() || turn == run.setup.turns {
            run.violations
                .extend(check_round_trip(&play, &checked.report_rows));
            break;
        }
    }
    run.final_state = play.built.game_data;
    run
}

/// One maintenance turn run and checked.
#[derive(Debug)]
pub(crate) struct CheckedTurn {
    pub events: MaintenanceEvents,
    pub report_rows: Vec<ReportBlockRow>,
    pub violations: Vec<Violation>,
}

/// Run one maintenance turn on `play` and check every per-turn invariant
/// over it; the error is a maintenance failure.
pub(crate) fn maintain_and_check(play: &mut ScenarioPlay) -> Result<CheckedTurn, Violation> {
    let before = play.clone();
    let mut twin = play.clone();
    let events = maintain(play)?;
    let mut violations = Vec::new();
    if let Some(detail) = twin_difference(play, &events, &mut twin) {
        violations.push(Violation::Nondeterministic(detail));
    }

    let after = &play.built.game_data;
    let report_rows = build_results_report_blocks(after, &events);
    let mut flagged = after.clone();
    apply_results_reviewable_flags(&mut flagged, &report_rows);
    violations.extend(check_turn(&TurnObservation {
        before: &before.built.game_data,
        after,
        events: &events,
        report_rows: &report_rows,
        flagged: &flagged,
        lifecycle: &play.player_lifecycle_states,
    }));
    Ok(CheckedTurn {
        events,
        report_rows,
        violations,
    })
}

fn maintain(play: &mut ScenarioPlay) -> Result<MaintenanceEvents, Violation> {
    match catch_unwind(AssertUnwindSafe(|| play.run_maintenance())) {
        Ok(Ok(events)) => Ok(events),
        Ok(Err(err)) => Err(Violation::MaintenanceError(err.to_string())),
        Err(panic) => Err(Violation::MaintenancePanic(panic_message(&panic))),
    }
}

/// Run the same maintenance turn again on `twin` and describe the first
/// difference from `play`, if any.
fn twin_difference(
    play: &ScenarioPlay,
    events: &MaintenanceEvents,
    twin: &mut ScenarioPlay,
) -> Option<String> {
    let twin_events = match maintain(twin) {
        Ok(events) => events,
        Err(violation) => return Some(format!("second run: {violation}")),
    };
    let (state, twin_state) = (&play.built.game_data, &twin.built.game_data);
    if state.fleets != twin_state.fleets {
        return Some("fleet records differ".to_string());
    }
    if state.planets != twin_state.planets {
        return Some("planet records differ".to_string());
    }
    if play != twin {
        return Some("campaign state differs".to_string());
    }
    let (events, twin_events) = (format!("{events:?}"), format!("{twin_events:?}"));
    (events != twin_events).then(|| "maintenance events differ".to_string())
}

fn turn_note(turn: u16, run: &CaseRun, events: &MaintenanceEvents) -> String {
    let actions = run.turns.last().map_or(0, |submissions| {
        submissions
            .iter()
            .map(|submission| {
                submission.planets.len() + submission.fleets.len() + submission.diplomacy.len()
            })
            .sum::<usize>()
    });
    format!(
        "turn {turn}: {actions} actions; {} battles, {} retreats, {} merges, {} assaults, {} colonies, {} captures, {} eliminations, {} outlooks; {} violations",
        events.fleet_battle_events.len(),
        events.encounter_disposition_events.len(),
        events.fleet_merge_events.len(),
        events.bombard_events.len() + events.assault_report_events.len(),
        events.colonization_events.len(),
        events.ownership_change_events.len(),
        events.empire_elimination_events.len(),
        events.campaign_outlook_events.len(),
        run.violations.len()
    )
}

fn panic_message(panic: &Box<dyn std::any::Any + Send>) -> String {
    panic
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| panic.downcast_ref::<&str>().map(|text| text.to_string()))
        .unwrap_or_else(|| "non-string panic".to_string())
}
