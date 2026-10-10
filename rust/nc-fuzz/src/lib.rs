//! Engine fuzzer: seeded games played with generated orders through the
//! real turn-submission path, with invariants checked after every turn.
//!
//! Drivers (Hegel, proptest) implement [`ChoiceSource`] and call
//! [`check_case`]; everything else is shared.

mod choice;
mod failure;
mod game;
mod invariants;
mod policy;
mod setup;
mod storage;
mod turn;

pub use choice::{ChoiceSource, ChoiceTape, pick_from, pick_index};
pub use failure::{SCENARIO_FILE_NAME, failure_dir, write_scenario};
pub use game::{CaseRun, run_case};
pub use invariants::{TurnObservation, Violation, check_turn};
pub use policy::Policy;
pub use setup::{CaseSetup, FuzzConfig, START_YEAR, case_count_from_env};

/// Play one case and panic on any violation, after writing the case as a
/// harness scenario under [`failure_dir`]`(driver)`.
pub fn check_case(choices: &mut dyn ChoiceSource, driver: &str) {
    let run = run_case(choices, &FuzzConfig::from_env());
    if run.violations.is_empty() {
        return;
    }
    let (turns, actions) = run.size();
    let written = match write_scenario(&run, &failure_dir(driver)) {
        Ok(path) => path.display().to_string(),
        Err(err) => format!("(could not write scenario: {err})"),
    };
    let violations = run
        .violations
        .iter()
        .map(|violation| format!("  - {violation}"))
        .collect::<Vec<_>>()
        .join("\n");
    panic!(
        "fuzz invariant violated ({turns} turns, {actions} actions); scenario: {written}\n{violations}"
    );
}
