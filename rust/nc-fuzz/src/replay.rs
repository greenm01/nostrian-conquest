//! A written scenario (a fuzz failure, or any harness combat scenario)
//! played turn by turn under the invariants.

use nc_data::TurnSubmission;
use nc_harness::{CombatScenarioSpec, HarnessError, ScenarioPlay, build_scenario};

use crate::game::maintain_and_check;
use crate::invariants::Violation;

/// Play `spec` through its maintenance turns and return the first turn's
/// violations, or none. Turn 1 files apply while the scenario is built;
/// later turn files go through the real submission path.
pub fn replay_scenario(spec: &CombatScenarioSpec) -> Result<Vec<Violation>, HarnessError> {
    let mut play = ScenarioPlay::new(build_scenario(&spec.scenario)?);
    for turn in 1..=spec.maintenance_turns {
        for file in spec
            .scenario
            .turn_files
            .iter()
            .filter(|file| file.turn == turn && turn > 1)
        {
            let submission = TurnSubmission::load_kdl(&file.path)
                .map_err(|err| HarnessError::Validation(err.to_string()))?;
            play.submit_turn(&submission)?;
        }
        let violations = match maintain_and_check(&mut play) {
            Ok(checked) => checked.violations,
            Err(violation) => vec![violation],
        };
        if !violations.is_empty() {
            return Ok(violations);
        }
    }
    Ok(Vec::new())
}
